//! Resumable-effect-VM frame stack — `make-engine-cloneable` Phase 2 (task 2.1).
//!
//! Plain-data replacement for the `Box<dyn FnOnce>` callbacks on
//! [`crate::selection::PendingSelection`]. The legacy executor pauses a
//! mid-effect selection by parking a `SelectionCallback = Box<dyn FnOnce(&mut
//! Game, u16)>` (and an `on_decline`), and *composes* nested continuations by
//! wrapping one closure inside another. Neither is `Clone`/serializable, which
//! is the sole remaining blocker for `Game: Clone` (see
//! `make-engine-cloneable/design.md` — "Verified defunctionalization
//! inventory": after Phase 1, `pending_selection` is the only hard blocker).
//!
//! This module models that suspended continuation as **data**: a stack of
//! [`ResumeFrame`]s run inner→outer when a selection resolves. "Wrapping" a
//! callback becomes `Vec::push` (a frame), not a nested closure. Every capture
//! across the ~50 callback sites + the 7 recursive trampolines was verified to
//! be `Copy`/`Clone` (filters are `CompiledPredicate` data, the
//! `Arc<Mutex<Box<dyn FnOnce>>>` trampoline plumbing collapses into the
//! `MultiPickStep::then` frame), so the whole stack is `Clone`.
//!
//! Wiring this into `PendingSelection` (a coexistence `resume: Option<...>`
//! field run by `resolve_generic_selection` ahead of the legacy closure) and
//! porting `count_capped` onto it is the next step — the task-0.2 spike.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::effect_context::selections::{CountCappedZone, DistinctByMode, RevealBucketSelection};
use crate::enums::{EffectSourceKind, GamePhase, PlayerId};
use crate::permanent::PermanentHandle;
use crate::selection::{SourceSelectionRef, UnionZoneOrigin};
use crate::trigger_context::TriggerContext;

/// Coexistence-only resume-side continuation hooks (make-engine-cloneable
/// Phase 2). Several callback-wrappers — the play-cost / digivolve-reducer /
/// option-reducer continuations, the DigiXros leave-window resume, and
/// `run_after_selections_drain` — compose their continuation onto a selection's
/// CLOSURE `callback`/`on_decline`. When the selection is resume-driven the
/// closure is BYPASSED by [`run_resume`](crate::dsl_cards::step::selections::run_resume),
/// so those wrappers instead defer their continuation here, and
/// `resolve_generic_selection` drains it right after the resume resolution (for
/// both accept and decline — every current wrapper runs the same continuation
/// either way).
///
/// Like the closure callbacks they stand in for, these are **not**
/// clone-faithful: a clone yields an EMPTY list (a forked search node must not
/// inherit live closure continuations). Faithful clone of a mid-continuation
/// state arrives only once the wrappers themselves are ported to data.
#[derive(Default)]
pub struct ResumeContinuationHooks(pub Vec<Box<dyn FnOnce(&mut crate::game::Game) + Send + Sync>>);

impl Clone for ResumeContinuationHooks {
    fn clone(&self) -> Self {
        Self(Vec::new())
    }
}

impl std::fmt::Debug for ResumeContinuationHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResumeContinuationHooks({} pending)", self.0.len())
    }
}

/// A paused continuation, as data. Frames run inner→outer on selection
/// resolution; this replaces the closure-nesting performed today by
/// `wrap_pending_selection_with_tail` and the `effect_queue` / `game_actions`
/// composition sites.
#[derive(Debug, Clone, Default)]
pub struct ResumeStack {
    /// Inner-to-outer: the last pushed frame runs first.
    pub frames: Vec<ResumeFrame>,
}

impl ResumeStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a continuation frame (the data analog of wrapping a callback).
    pub fn push(&mut self, frame: ResumeFrame) {
        self.frames.push(frame);
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

/// Provenance carried by every frame (former closure captures of the `Copy`
/// handles the no-lifetimes selection callbacks already close over).
#[derive(Debug, Clone, Copy)]
pub struct ResumeProvenance {
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub source_kind: EffectSourceKind,
    pub controller: PlayerId,
    /// `override_selecting_player` at install time (mirrors
    /// `EffectContext::new_with_source_kind_and_override`).
    pub override_pin: Option<PlayerId>,
}

/// Which zone/kind a `RunTail` frame decodes the resolving `action_id` against
/// in order to bind the chosen value. Mirrors the `SelectionKind`-specific
/// decode each legacy install closure performed inline. Spike scope: `Hand`.
// Not `Copy`: `AnyPermanent` carries a `Vec` of candidates.
#[derive(Debug, Clone)]
pub enum ResumeSelectKind {
    /// `action_id - PLAY_HAND_START` → hand index of `of_player`.
    Hand { of_player: PlayerId },
    /// `action_id - TRASH_EFFECT_START` → trash index of `of_player`.
    Trash { of_player: PlayerId },
    /// `(action_id - ATTACK_START) % TARGETS_PER_ATTACKER` → battle-area index
    /// of `of_player`. Covers own- and opponent-field permanent selects (the
    /// `install_field_selection` decode); the resume arm mirrors that wrapper's
    /// `effect_source_player` scoping.
    FieldPermanent { of_player: PlayerId },
    /// `action_id - (SEL_MY_SECURITY_START | SEL_OPP_SECURITY_START)` → security
    /// index of `of_player` (base chosen by whether `of_player` is the
    /// controller). Binds the resolved security `CardHandle`.
    Security { of_player: PlayerId },
    /// Own breeding-area permanent select. The single breeding permanent is
    /// reconstructed from game state (not decoded from `action_id`); binds a
    /// `BreedingPermanentSelectionRef`.
    BreedingPermanent { of_player: PlayerId },
    /// Both-battle-area permanent select (`select_any_permanent`). The
    /// candidate `(action_id, handle)` pairs are captured at install time
    /// (heterogeneous player domain), so the resume arm resolves by linear
    /// search rather than arithmetic decode. Binds the matched `PermanentHandle`.
    AnyPermanent {
        candidates: Vec<(u16, PermanentHandle)>,
    },
    /// `action_id - SEL_REVEAL_START` → index into the shared
    /// `Game.revealed_cards`. Ownership is enforced at install time (the filter
    /// builds `valid_action_ids`), so the arm just binds the revealed
    /// `CardHandle`; a stale index skips the bind.
    Reveal,
    /// `action_id - material_zone_geometry(perm).range_start` → digivolution
    /// source index under carrier `perm` (battle- or breeding-area). Binds the
    /// resolved source `CardHandle` via `material_carrier_permanent`.
    Material { perm: PermanentHandle },
    /// `action_id - HAND_EFFECT_START` → 0-based label index of a "choose one"
    /// prompt (`select_effect_choice`). Binds the index as a literal
    /// (`insert_literal`). Not optional (a branch must be picked).
    EffectChoice,
    /// Union-zone select spanning hand ∪ trash ∪ material. The tri-range decode
    /// is captured at install as `(action_id, handle, origin)` candidates (the
    /// decode runs once), so the arm linear-searches and binds via
    /// `insert_union_card` (recording the origin zone for downstream replay).
    /// Dual-tail: success uses `inner_tail`; decline uses the frame's `decline`.
    UnionZone {
        of_player: PlayerId,
        candidates: Vec<(u16, CardHandle, UnionZoneOrigin)>,
    },
}

/// What an optional selection does when declined (PASS). Mandatory selects never
/// reach this — PASS is rejected by `resolve_generic_selection`.
#[derive(Debug, Clone)]
pub enum ResumeDecline {
    /// No `on_decline` was installed (e.g. `select_security`): PASS resolves to
    /// nothing.
    None,
    /// Run a decline tail — may equal the success tail (optional `select_trash`)
    /// or differ (`select_own_breeding_permanent`'s `decline_tail`).
    /// `aborts_clause` first sets `dsl_clause_aborted`: the optional-cost
    /// "By trashing X, do Y" path where the cost was declined.
    RunTail {
        tail: Arc<Vec<CompiledStep>>,
        aborts_clause: bool,
    },
}

/// An outer-tail continuation composed onto a resume-driven selection by
/// `wrap_pending_selection_with_tail` when a flipped (resumable-VM) select is
/// nested inside another clause (e.g. an interrupt trigger's `select_trash`
/// firing mid-clause, while the outer clause still has steps after the point it
/// parked). In the closure world this was a `move` closure wrapping the
/// selection's callback; as data it is the tail plus the snapshot needed to run
/// it via `drain_or_rewrap_pending_tail` (which re-composes onto any further
/// nested select, so deep chains thread correctly). Runs AFTER the frame's
/// `inner_tail` (success) or decline tail; multiple conts run in push order.
#[derive(Debug, Clone)]
pub struct OuterContinuation {
    pub tail: Arc<Vec<CompiledStep>>,
    pub bindings: Bindings,
    pub runtime: StepRuntime,
    pub trigger_context: Option<TriggerContext>,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
    pub player: PlayerId,
}

/// One resumable continuation frame. Each variant corresponds to a former
/// `Box::new(move |game, action_id| …)` closure body; all captured fields are
/// `Copy`/`Clone` data.
#[derive(Debug, Clone)]
pub enum ResumeFrame {
    /// ~95% of DSL selection sites (Shapes A+B): bind the chosen value into a
    /// slot, run the inner step tail, then drain the outer tail. `decline`
    /// carries the optional-decline behavior (none / run-a-tail / cost-abort).
    RunTail {
        prov: ResumeProvenance,
        /// How to decode the resolving `action_id` into the bound value.
        select_kind: ResumeSelectKind,
        /// Binding slot the chosen value is written into (`bind_as`), if any.
        bind_as: Option<String>,
        /// Steps after the select within the same effect body.
        inner_tail: Arc<Vec<CompiledStep>>,
        /// Outer-tail continuations composed by `wrap_pending_selection_with_tail`
        /// when this resume-driven select is nested inside another clause. Run
        /// (in push order) after `inner_tail` on success / after the decline tail
        /// on decline. Empty for a top-level (un-nested) select.
        outer_conts: Vec<OuterContinuation>,
        bindings: Bindings,
        runtime: StepRuntime,
        trigger_context: Option<TriggerContext>,
        decline: ResumeDecline,
    },
    /// Recursive multi-pick accumulator (the `count_capped` family). Carries the
    /// full pick state plus the data terminal (bind the accumulated list, then
    /// run the tail) — see [`MultiPickState`]. The former
    /// `Arc<Mutex<Option<Box<dyn FnOnce>>>>` final-callback threading collapses
    /// into re-parking this frame until the pick count is satisfied.
    MultiPickStep(MultiPickState),
    /// Ordered-permutation accumulator (`select_ordered_permutation`): the player
    /// orders a fixed item list via sequential single picks (`SEL_REVEAL` range
    /// into the REMAINING items). Mandatory until the list is exhausted; the data
    /// terminal binds the ordered list and runs the tail. See [`PermutationState`].
    PermutationStep(PermutationState),
    /// Cost-budget accumulator over opponent permanents (`..._by_dp_budget` /
    /// `..._by_play_cost_budget`, unified). Each pick subtracts the target's cost
    /// (DP or play cost per [`BudgetKind`]) from `remaining`; candidates are
    /// re-derived each step from the carried `CompiledPredicate` (data-pure).
    /// PASS commits at/above `min_picks`; terminal binds the permanent list. See
    /// [`BudgetState`].
    BudgetStep(BudgetState),
    /// Cross-permanent digivolution-source multi-pick (`select_own_sources` /
    /// `select_opponent_sources`). Picks `min..max` sources across `of_player`'s
    /// battle-area stacks; candidates re-derived each step from the carried
    /// `CompiledPredicate` (data-pure) with live revalidation (a picked source's
    /// card may vanish mid-selection). PASS commits at/above `min`; terminal binds
    /// the source-ref list. See [`SourceMultiState`].
    SourceMultiStep(SourceMultiState),
    /// Battle-area permanent multi-pick (`select_count_capped` over `BattleArea` →
    /// `install_count_capped_permanent_step`). Picks `min..max` permanents from a
    /// captured candidate snapshot (snapshot-minus-picked, NOT live-recomputed —
    /// mirrors the closure). PASS commits at/above the floor; terminal binds the
    /// permanent list. See [`CountCappedPermanentsState`].
    CountCappedPermanentsStep(CountCappedPermanentsState),
    /// Multi-bucket reveal selection (`select_reveal_buckets`): a sequence of
    /// buckets, each picking `min..max` from its pre-resolved candidate handles
    /// (∩ the live reveal pile, minus already-picked + cross-bucket dedup). A
    /// completed/empty/`max==0` bucket advances `bucket_index`; the terminal (all
    /// buckets done) binds each bucket's list by its `bind_as` and runs the tail.
    /// See [`RevealBucketState`].
    RevealBucketStep(RevealBucketState),
}

/// In-flight state of a multi-bucket reveal selection, as data. The buckets'
/// candidates are concrete `CardHandle`s pre-resolved at install (the per-bucket
/// `CompiledPredicate` is evaluated then), so no closure/predicate is parked.
#[derive(Debug, Clone)]
pub struct RevealBucketState {
    pub prov: ResumeProvenance,
    pub selecting_player: PlayerId,
    pub previous_phase: GamePhase,
    pub buckets: Vec<RevealBucketSelection>,
    pub bucket_index: usize,
    /// Completed buckets: `(bind_as, picked cards)`.
    pub picked_buckets: Vec<(String, Vec<CardHandle>)>,
    /// Picks accumulated for the current (in-progress) bucket.
    pub current_bucket_picks: Vec<CardHandle>,
    pub no_duplicate_cards: bool,
    pub prompt: String,
    // ── data terminal (bind each bucket's list by bind_as, then run the tail) ──
    pub inner_tail: Arc<Vec<CompiledStep>>,
    pub bindings: Bindings,
    pub runtime: StepRuntime,
    pub trigger_context: Option<TriggerContext>,
    pub outer_conts: Vec<OuterContinuation>,
}

/// In-flight state of a battle-area permanent multi-pick, as data. The candidate
/// snapshot is captured at install and shrunk by removing the picked action each
/// step (the closure's `candidates.filter(!= action)`), not re-derived — so no
/// predicate/filter is carried.
#[derive(Debug, Clone)]
pub struct CountCappedPermanentsState {
    pub prov: ResumeProvenance,
    pub selecting_player: PlayerId,
    pub previous_phase: GamePhase,
    /// Drives the `OppField`/`OwnField` selection kind for the frontend router.
    pub target_is_opponent: bool,
    /// `min` is already clamped (`max.min(candidates)` when `clamp_to_available`).
    pub min: u8,
    pub max: u8,
    pub optional_zero: bool,
    pub candidates: Vec<(u16, PermanentHandle)>,
    pub accum: Vec<PermanentHandle>,
    pub prompt: String,
    pub bind_as: Option<String>,
    pub inner_tail: Arc<Vec<CompiledStep>>,
    pub bindings: Bindings,
    pub runtime: StepRuntime,
    pub trigger_context: Option<TriggerContext>,
    pub outer_conts: Vec<OuterContinuation>,
}

/// In-flight state of a cross-permanent source multi-pick, as data. Re-derives
/// candidates from `filter` each step (the closure trampoline's `SourceFilter`
/// is a `CompiledPredicate` here), so no runtime closure is parked.
#[derive(Debug, Clone)]
pub struct SourceMultiState {
    pub prov: ResumeProvenance,
    pub of_player: PlayerId,
    pub selecting_player: PlayerId,
    pub previous_phase: GamePhase,
    pub min: u8,
    pub max: u8,
    /// Sources picked so far (de-duplicated by card).
    pub picked: Vec<SourceSelectionRef>,
    /// This step's candidate snapshot `(action_id, source_ref)`. Carried (not
    /// recomputed on resolve) so a pick decodes to the SAME `source_ref` the
    /// install offered — the live-revalidation check then asks whether that
    /// card is still present (mirrors the closure's `action_to_source` snapshot).
    pub candidates: Vec<(u16, SourceSelectionRef)>,
    pub filter: CompiledPredicate,
    pub filter_bindings: Bindings,
    /// `target` binding restriction: if `Some`, only sources under this permanent
    /// are candidates. `target_resolution_failed` rejects all (binding missing).
    pub target_permanent: Option<PermanentHandle>,
    pub target_resolution_failed: bool,
    /// `select_own_sources` evaluates the predicate on `PredicateSubject::Source`;
    /// `select_opponent_sources` on `PredicateSubject::Card(source.card)`.
    pub eval_on_card: bool,
    pub prompt: String,
    // ── data terminal (bind the picked source-ref list, then run the tail) ──
    pub bind_as: Option<String>,
    pub inner_tail: Arc<Vec<CompiledStep>>,
    pub bindings: Bindings,
    pub runtime: StepRuntime,
    pub trigger_context: Option<TriggerContext>,
    /// Outer-tail continuations composed by `wrap_pending_selection_with_tail`
    /// when this multi-pick select is nested inside another clause. Run at the
    /// TERMINAL (after the accumulated list is bound + inner_tail runs), in push
    /// order — NOT on intermediate re-parks. Empty for a top-level select.
    pub outer_conts: Vec<OuterContinuation>,
}

/// Which cost a [`BudgetState`] accumulator spends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetKind {
    /// `game.effective_dp(handle)` — `select_opponent_permanents_by_dp_budget`.
    Dp,
    /// `top_card().play_cost(card_data)` — `..._by_play_cost_budget`.
    PlayCost,
}

/// In-flight state of a cost-budget multi-pick over opponent permanents, as data.
/// Re-derives candidates from `filter` each step (the closure trampoline's filter
/// is a `CompiledPredicate` here), so no runtime closure is parked.
#[derive(Debug, Clone)]
pub struct BudgetState {
    pub prov: ResumeProvenance,
    pub kind: BudgetKind,
    pub opponent: PlayerId,
    pub selecting_player: PlayerId,
    pub previous_phase: GamePhase,
    /// Cost budget left to spend.
    pub remaining: i32,
    pub min_picks: u8,
    /// Permanents picked so far (subtracted from `remaining`).
    pub picked: Vec<PermanentHandle>,
    /// Candidate predicate, re-evaluated each step against a read context.
    pub filter: CompiledPredicate,
    pub filter_bindings: Bindings,
    pub prompt: String,
    // ── data terminal (bind the picked permanent list, then run the tail) ──
    pub bind_as: Option<String>,
    pub inner_tail: Arc<Vec<CompiledStep>>,
    pub bindings: Bindings,
    pub runtime: StepRuntime,
    pub trigger_context: Option<TriggerContext>,
    /// Outer-tail continuations composed by `wrap_pending_selection_with_tail`
    /// when this multi-pick select is nested inside another clause. Run at the
    /// TERMINAL (after the accumulated list is bound + inner_tail runs), in push
    /// order — NOT on intermediate re-parks. Empty for a top-level select.
    pub outer_conts: Vec<OuterContinuation>,
}

/// In-flight state of an ordered-permutation selection, as data. Re-parked once
/// per pick; the terminal fires when `remaining` is empty (every item placed).
#[derive(Debug, Clone)]
pub struct PermutationState {
    pub prov: ResumeProvenance,
    pub selecting_player: PlayerId,
    pub previous_phase: GamePhase,
    /// Items not yet placed; `action_id - SEL_REVEAL_START` indexes into this.
    pub remaining: Vec<CardHandle>,
    /// Items placed so far, in chosen order.
    pub accum: Vec<CardHandle>,
    pub prompt: String,
    // ── data terminal (bind the ordered list, then run the tail) ──
    pub bind_as: Option<String>,
    pub inner_tail: Arc<Vec<CompiledStep>>,
    pub bindings: Bindings,
    pub runtime: StepRuntime,
    pub trigger_context: Option<TriggerContext>,
    /// Outer-tail continuations composed by `wrap_pending_selection_with_tail`
    /// when this multi-pick select is nested inside another clause. Run at the
    /// TERMINAL (after the accumulated list is bound + inner_tail runs), in push
    /// order — NOT on intermediate re-parks. Empty for a top-level select.
    pub outer_conts: Vec<OuterContinuation>,
}

/// In-flight state of a `count_capped`-family multi-pick selection, as data.
/// Re-parked once per pick; the data terminal fires when the count is satisfied
/// (or candidates exhausted, or the player passes at/above the floor).
#[derive(Debug, Clone)]
pub struct MultiPickState {
    pub prov: ResumeProvenance,
    pub of_player: PlayerId,
    pub selecting_player: PlayerId,
    pub previous_phase: GamePhase,
    /// Zone the picks index into (decode base + carrier branch).
    pub zone: CountCappedZone,
    pub range_start: u16,
    pub min: u8,
    pub max: u8,
    pub is_optional_zero: bool,
    pub distinct_by: Option<DistinctByMode>,
    /// Zone indices still eligible to pick.
    pub candidate_indices: Vec<usize>,
    /// Handles picked so far, in order.
    pub accum: Vec<CardHandle>,
    // ── data terminal (count_capped binds the accumulated list, then runs the tail) ──
    pub bind_as: Option<String>,
    pub inner_tail: Arc<Vec<CompiledStep>>,
    pub bindings: Bindings,
    pub runtime: StepRuntime,
    pub trigger_context: Option<TriggerContext>,
    /// Outer-tail continuations composed by `wrap_pending_selection_with_tail`
    /// when this multi-pick select is nested inside another clause. Run at the
    /// TERMINAL (after the accumulated list is bound + inner_tail runs), in push
    /// order — NOT on intermediate re-parks. Empty for a top-level select.
    pub outer_conts: Vec<OuterContinuation>,
}

#[cfg(test)]
mod tests {
    //! Spike (make-engine-cloneable task 0.2): prove a paused selection whose
    //! continuation is carried as plain DATA resumes faithfully — the data
    //! path runs the inner tail exactly as the legacy closure callback would.

    use super::*;
    use crate::debug_runner::{make_test_card, DebugRunner};
    use crate::dsl_cards::step::StepRuntime;
    use crate::enums::EffectSourceKind;
    use crate::selection::{PendingSelection, SelectionKind};
    use digimon_dsl::compiled::CompiledStep;

    #[test]
    fn runtail_hand_runs_inner_tail_as_data() {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();

        let source_card;
        let prev_phase;
        {
            let game = runner.game_mut();
            source_card = game.player(0).hand[0].handle();
            prev_phase = game.current_phase;

            // A Hand selection whose CONTINUATION is data: gain 5 memory. The
            // legacy closure is rigged to panic, so if the test passes it can
            // only be because `run_resume` (the data path) executed.
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::Hand,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![crate::action::space::PLAY_HAND_START],
                is_optional: false,
                prompt: "spike".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| {
                    panic!("closure path must NOT run when pending_selection_resume is set")
                }),
                on_decline: None,
                zone_owner: Some(0),
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::Hand { of_player: 0 },
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }

        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::PLAY_HAND_START)
            .expect("resolve_selection should succeed");
        let after = runner.memory();

        assert_eq!(
            (after - before).abs(),
            5,
            "RunTail inner tail (GainMemory 5) must execute via the data path"
        );
        assert!(
            runner.game_mut().pending_selection.is_none(),
            "the selection must be consumed on resolve"
        );
        assert!(
            runner.game_mut().pending_selection_resume.is_none(),
            "the resume stack must be consumed on resolve"
        );
    }

    #[test]
    fn runtail_trash_resolves_via_data_path() {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();
        let source_card;
        {
            let game = runner.game_mut();
            source_card = game.player(0).hand[0].handle();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::Trash,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![crate::action::space::TRASH_EFFECT_START],
                is_optional: false,
                prompt: "spike-trash".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| panic!("closure path must NOT run")),
                on_decline: None,
                zone_owner: Some(0),
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::Trash { of_player: 0 },
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::TRASH_EFFECT_START)
            .expect("resolve_selection should succeed");
        assert_eq!(
            (runner.memory() - before).abs(),
            5,
            "Trash RunTail inner tail (GainMemory 5) must execute via the data path"
        );
        assert!(runner.game_mut().pending_selection.is_none());
    }

    #[test]
    fn runtail_field_permanent_resolves_via_data_path() {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();
        let source_card;
        {
            let game = runner.game_mut();
            source_card = game.player(0).hand[0].handle();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::OwnField,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![crate::action::space::ATTACK_START],
                is_optional: false,
                prompt: "spike-field".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| panic!("closure path must NOT run")),
                on_decline: None,
                zone_owner: None,
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::FieldPermanent { of_player: 0 },
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::ATTACK_START)
            .expect("resolve_selection should succeed");
        assert_eq!(
            (runner.memory() - before).abs(),
            5,
            "FieldPermanent RunTail inner tail (GainMemory 5) must execute via the data path"
        );
        assert!(runner.game_mut().pending_selection.is_none());
    }

    #[test]
    fn runtail_security_resolves_via_data_path() {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();
        let source_card;
        {
            let game = runner.game_mut();
            source_card = game.player(0).hand[0].handle();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::Security,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![crate::action::space::SEL_MY_SECURITY_START],
                is_optional: false,
                prompt: "spike-security".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| panic!("closure path must NOT run")),
                on_decline: None,
                zone_owner: Some(0),
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::Security { of_player: 0 },
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::SEL_MY_SECURITY_START)
            .expect("resolve_selection should succeed");
        assert_eq!(
            (runner.memory() - before).abs(),
            5,
            "Security RunTail inner tail (GainMemory 5) must execute via the data path"
        );
        assert!(runner.game_mut().pending_selection.is_none());
    }

    #[test]
    fn runtail_decline_none_runs_no_tail() {
        // An optional select with ResumeDecline::None (e.g. select_security)
        // resolves PASS to NOTHING — the inner tail must NOT run.
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();
        let source_card;
        {
            let game = runner.game_mut();
            source_card = game.player(0).hand[0].handle();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::Security,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![crate::action::space::SEL_MY_SECURITY_START],
                is_optional: true,
                prompt: "spike-decline".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| panic!("closure path must NOT run")),
                on_decline: None,
                zone_owner: Some(0),
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::Security { of_player: 0 },
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::PASS)
            .expect("PASS on an optional selection should succeed");
        assert_eq!(
            runner.memory(),
            before,
            "ResumeDecline::None must run NO tail on PASS"
        );
        assert!(runner.game_mut().pending_selection.is_none());
    }

    #[test]
    fn runtail_breeding_permanent_resolves_via_data_path() {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();
        runner.place_in_breeding(0, "TEST-RESUME");
        let action_id =
            crate::action::space::encode_breeding_select(0).expect("breeding select action id");
        let source_card;
        {
            let game = runner.game_mut();
            source_card = game.player(0).hand[0].handle();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::BreedingPermanent,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![action_id],
                is_optional: false,
                prompt: "spike-breeding".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| panic!("closure path must NOT run")),
                on_decline: None,
                zone_owner: None,
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::BreedingPermanent { of_player: 0 },
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, action_id)
            .expect("resolve_selection should succeed");
        assert_eq!(
            (runner.memory() - before).abs(),
            5,
            "BreedingPermanent RunTail inner tail (GainMemory 5) must execute via the data path"
        );
        assert!(runner.game_mut().pending_selection.is_none());
    }

    #[test]
    fn runtail_decline_runs_decline_tail_not_inner() {
        // ResumeDecline::RunTail runs the DECLINE tail (3), never inner_tail (5),
        // on PASS — the dual-tail behavior breeding/union_zone rely on.
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();
        let source_card;
        {
            let game = runner.game_mut();
            source_card = game.player(0).hand[0].handle();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::Trash,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![crate::action::space::TRASH_EFFECT_START],
                is_optional: true,
                prompt: "spike-dual-tail".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| panic!("closure path must NOT run")),
                on_decline: None,
                zone_owner: Some(0),
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::Trash { of_player: 0 },
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::RunTail {
                        tail: Arc::new(vec![CompiledStep::GainMemory(3)]),
                        aborts_clause: false,
                    },
                }],
            });
        }
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::PASS)
            .expect("PASS should succeed");
        assert_eq!(
            (runner.memory() - before).abs(),
            3,
            "decline must run the DECLINE tail (GainMemory 3), not inner_tail (5)"
        );
    }

    #[test]
    fn runtail_any_permanent_resolves_via_data_path() {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();
        let action = crate::action::space::ATTACK_START;
        let source_card;
        {
            let game = runner.game_mut();
            source_card = game.player(0).hand[0].handle();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::AnyField,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![action],
                is_optional: false,
                prompt: "spike-any".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| panic!("closure path must NOT run")),
                on_decline: None,
                zone_owner: None,
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::AnyPermanent {
                        candidates: vec![(action, PermanentHandle { player: 0, index: 0 })],
                    },
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, action)
            .expect("resolve_selection should succeed");
        assert_eq!(
            (runner.memory() - before).abs(),
            5,
            "AnyPermanent RunTail inner tail (GainMemory 5) must execute via the data path"
        );
        assert!(runner.game_mut().pending_selection.is_none());
    }

    #[test]
    fn runtail_reveal_resolves_via_data_path() {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();
        let action = crate::action::space::SEL_REVEAL_START;
        let source_card;
        {
            let game = runner.game_mut();
            source_card = game.player(0).hand[0].handle();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::Reveal,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![action],
                is_optional: false,
                prompt: "spike-reveal".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| panic!("closure path must NOT run")),
                on_decline: None,
                zone_owner: None,
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::Reveal,
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, action)
            .expect("resolve_selection should succeed");
        assert_eq!(
            (runner.memory() - before).abs(),
            5,
            "Reveal RunTail inner tail (GainMemory 5) must execute via the data path"
        );
        assert!(runner.game_mut().pending_selection.is_none());
    }

    #[test]
    fn runtail_material_resolves_via_data_path() {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();
        // A 2-card stack: top card + 1 digivolution source (the selectable material).
        let carrier = runner.place_stack(0, &["TEST-RESUME", "TEST-RESUME"]);
        let range_start =
            crate::effect_context::selections::material_zone_geometry(runner.game_mut(), carrier)
                .expect("carrier has selectable material")
                .1;
        let source_card;
        {
            let game = runner.game_mut();
            source_card = game.player(0).hand[0].handle();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::Material,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![range_start],
                is_optional: false,
                prompt: "spike-material".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| panic!("closure path must NOT run")),
                on_decline: None,
                zone_owner: None,
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::Material { perm: carrier },
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, range_start)
            .expect("resolve_selection should succeed");
        assert_eq!(
            (runner.memory() - before).abs(),
            5,
            "Material RunTail inner tail (GainMemory 5) must execute via the data path"
        );
        assert!(runner.game_mut().pending_selection.is_none());
    }

    #[test]
    fn runtail_union_zone_resolves_via_data_path() {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();
        let action = crate::action::space::PLAY_HAND_START;
        let hand_card;
        {
            let game = runner.game_mut();
            hand_card = game.player(0).hand[0].handle();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::Hand,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![action],
                is_optional: false,
                prompt: "spike-union".to_string(),
                effect_choices: None,
                source_card: hand_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| panic!("closure path must NOT run")),
                on_decline: None,
                zone_owner: Some(0),
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card: hand_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::UnionZone {
                        of_player: 0,
                        candidates: vec![(action, hand_card, UnionZoneOrigin::Hand)],
                    },
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, action)
            .expect("resolve_selection should succeed");
        assert_eq!(
            (runner.memory() - before).abs(),
            5,
            "UnionZone RunTail inner tail (GainMemory 5) must execute via the data path"
        );
        assert!(runner.game_mut().pending_selection.is_none());
    }

    #[test]
    fn multipick_step_accumulates_then_terminates() {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME", "TEST-RESUME", "TEST-RESUME"])
            .memory(0)
            .start();
        let source_card = runner.game_mut().player(0).hand[0].handle();
        let prev_phase = runner.game_mut().current_phase;
        // Install the initial count_capped step: min 0, max 2, Hand zone, 3 candidates.
        let state = MultiPickState {
            prov: ResumeProvenance {
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                controller: 0,
                override_pin: None,
            },
            of_player: 0,
            selecting_player: 0,
            previous_phase: prev_phase,
            zone: CountCappedZone::Hand,
            range_start: crate::action::space::PLAY_HAND_START,
            min: 0,
            max: 2,
            is_optional_zero: true,
            distinct_by: None,
            candidate_indices: vec![0, 1, 2],
            accum: vec![],
            bind_as: Some("picks".to_string()),
            inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
            bindings: Bindings::new(),
            runtime: StepRuntime::default(),
            trigger_context: None,
            outer_conts: Vec::new(),
        };
        crate::dsl_cards::step::selections::install_multipick_step(runner.game_mut(), state);

        // Pick 1 (hand index 0): accumulates one, re-parks for pick 2.
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::PLAY_HAND_START)
            .expect("pick 1 resolves");
        {
            let g = runner.game_mut();
            assert!(g.pending_selection.is_some(), "must re-park for pick 2");
            match g.pending_selection_resume.as_ref().expect("re-parked frame") {
                ResumeStack { frames } => match &frames[0] {
                    ResumeFrame::MultiPickStep(s) => {
                        assert_eq!(s.accum.len(), 1, "exactly one pick accumulated");
                        assert_eq!(s.candidate_indices, vec![1, 2], "picked index removed");
                    }
                    _ => panic!("expected a MultiPickStep frame"),
                },
            }
        }

        // Pick 2 (hand index 1): reaches max → terminal binds the list + runs the tail.
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::PLAY_HAND_START + 1)
            .expect("pick 2 resolves");
        assert_eq!(
            (runner.memory() - before).abs(),
            5,
            "reaching max must run the terminal inner tail (GainMemory 5)"
        );
        assert!(
            runner.game_mut().pending_selection.is_none(),
            "multi-pick complete"
        );
    }

    #[test]
    fn runtail_runs_outer_continuation_after_inner_tail() {
        // Nested-resume composition (the EX11-044 shape): a flipped (resume-
        // driven) select carries an OuterContinuation wrapped on by an outer
        // clause. On resolution, BOTH the inner tail (5) and the outer cont (3)
        // must run — total 8 — exactly as the closure wrapper would have.
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME"])
            .memory(0)
            .start();
        let source_card;
        {
            let game = runner.game_mut();
            source_card = game.player(0).hand[0].handle();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::Hand,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![crate::action::space::PLAY_HAND_START],
                is_optional: false,
                prompt: "nested".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| panic!("closure path must NOT run")),
                on_decline: None,
                zone_owner: Some(0),
            });
            game.pending_selection_resume = Some(ResumeStack {
                frames: vec![ResumeFrame::RunTail {
                    prov: ResumeProvenance {
                        source_card,
                        source_permanent: None,
                        source_kind: EffectSourceKind::Digimon,
                        controller: 0,
                        override_pin: None,
                    },
                    select_kind: ResumeSelectKind::Hand { of_player: 0 },
                    bind_as: None,
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: vec![OuterContinuation {
                        tail: Arc::new(vec![CompiledStep::GainMemory(3)]),
                        bindings: Bindings::new(),
                        runtime: StepRuntime::default(),
                        trigger_context: None,
                        source_card,
                        source_permanent: None,
                        player: 0,
                    }],
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::PLAY_HAND_START)
            .expect("resolve_selection should succeed");
        assert_eq!(
            (runner.memory() - before).abs(),
            8,
            "inner tail (5) AND the outer continuation (3) must both run"
        );
        assert!(runner.game_mut().pending_selection.is_none());
        assert!(runner.game_mut().pending_selection_resume.is_none());
    }

    #[test]
    fn permutation_step_accumulates_in_order_then_terminates() {
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
            .hand(0, &["TEST-RESUME", "TEST-RESUME"])
            .memory(0)
            .start();
        let (c0, c1, prev_phase) = {
            let g = runner.game_mut();
            (
                g.player(0).hand[0].handle(),
                g.player(0).hand[1].handle(),
                g.current_phase,
            )
        };
        let source_card = c0;
        let state = PermutationState {
            prov: ResumeProvenance {
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                controller: 0,
                override_pin: None,
            },
            selecting_player: 0,
            previous_phase: prev_phase,
            remaining: vec![c0, c1],
            accum: vec![],
            prompt: "order".to_string(),
            bind_as: Some("ordered".to_string()),
            inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
            bindings: Bindings::new(),
            runtime: StepRuntime::default(),
            trigger_context: None,
            outer_conts: Vec::new(),
        };
        crate::dsl_cards::step::selections::install_permutation_resume_step(runner.game_mut(), state);

        // Pick item 0: re-parks for the last item.
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::SEL_REVEAL_START)
            .expect("pick 1 resolves");
        {
            let g = runner.game_mut();
            assert!(g.pending_selection.is_some(), "must re-park for the 2nd pick");
            match &g.pending_selection_resume.as_ref().expect("re-parked").frames[0] {
                ResumeFrame::PermutationStep(s) => {
                    assert_eq!(s.accum, vec![c0], "first pick accumulated in order");
                    assert_eq!(s.remaining, vec![c1], "picked item removed");
                }
                _ => panic!("expected a PermutationStep frame"),
            }
        }

        // Pick the last item (index 0 of the remaining 1): terminal runs the tail.
        let before = runner.memory();
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::SEL_REVEAL_START)
            .expect("pick 2 resolves");
        assert_eq!(
            (runner.memory() - before).abs(),
            5,
            "exhausting the list must run the terminal inner tail (GainMemory 5)"
        );
        assert!(runner.game_mut().pending_selection.is_none());
        assert!(runner.game_mut().pending_selection_resume.is_none());
    }

    #[test]
    fn resume_stack_is_clone() {
        // The whole point: a paused continuation is Clone data.
        let stack = ResumeStack {
            frames: vec![ResumeFrame::RunTail {
                prov: ResumeProvenance {
                    source_card: crate::card_source::CardHandle(0),
                    source_permanent: None,
                    source_kind: EffectSourceKind::Digimon,
                    controller: 0,
                    override_pin: None,
                },
                select_kind: ResumeSelectKind::Hand { of_player: 0 },
                bind_as: Some("x".to_string()),
                inner_tail: Arc::new(vec![CompiledStep::GainMemory(1)]),
                outer_conts: Vec::new(),
                bindings: Bindings::new(),
                runtime: StepRuntime::default(),
                trigger_context: None,
                decline: ResumeDecline::None,
            }],
        };
        let cloned = stack.clone();
        assert_eq!(cloned.frames.len(), stack.frames.len());
    }

    /// The capstone: `Game` is `Clone`, and a clone taken at a (resume-path)
    /// decision point is INDEPENDENT of the original and REPLAYS IDENTICALLY.
    /// This is make-engine-cloneable task 4.2's guard — the property MCTS needs.
    #[test]
    fn game_clone_is_independent_and_replays_identically() {
        fn setup() -> DebugRunner {
            let mut runner = DebugRunner::builder()
                .add_card(make_test_card("TEST-RESUME", "Resume Tester"))
                .hand(0, &["TEST-RESUME"])
                .memory(0)
                .start();
            let source_card;
            {
                let game = runner.game_mut();
                source_card = game.player(0).hand[0].handle();
                let prev_phase = game.current_phase;
                game.pending_selection = Some(PendingSelection {
                    kind: SelectionKind::Hand,
                    selecting_player: 0,
                    previous_phase: prev_phase,
                    valid_action_ids: vec![crate::action::space::PLAY_HAND_START],
                    is_optional: false,
                    prompt: "clone".to_string(),
                    effect_choices: None,
                    source_card,
                    source_permanent: None,
                    source_kind: EffectSourceKind::Digimon,
                    callback: Box::new(|_g, _a| unreachable!("resume path drives this")),
                    on_decline: None,
                    zone_owner: Some(0),
                });
                game.pending_selection_resume = Some(ResumeStack {
                    frames: vec![ResumeFrame::RunTail {
                        prov: ResumeProvenance {
                            source_card,
                            source_permanent: None,
                            source_kind: EffectSourceKind::Digimon,
                            controller: 0,
                            override_pin: None,
                        },
                        select_kind: ResumeSelectKind::Hand { of_player: 0 },
                        bind_as: None,
                        inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                        outer_conts: Vec::new(),
                        bindings: Bindings::new(),
                        runtime: StepRuntime::default(),
                        trigger_context: None,
                        decline: ResumeDecline::None,
                    }],
                });
            }
            runner
        }

        let mut runner = setup();
        let original_memory_before = runner.game_mut().memory;

        // Clone at the decision point (the operation MCTS performs).
        let mut clone = runner.game_mut().clone();

        // Resolve on the CLONE only.
        clone
            .resolve_selection(0, crate::action::space::PLAY_HAND_START)
            .expect("clone resolves");

        // INDEPENDENCE: the original is untouched by mutating the clone.
        assert!(
            runner.game_mut().pending_selection.is_some(),
            "original's pending selection must survive cloning + resolving the clone"
        );
        assert_eq!(
            runner.game_mut().memory,
            original_memory_before,
            "original memory must be unchanged by the clone's resolution"
        );

        // The clone advanced (GainMemory(5) ran via the cloned resume frame).
        assert_eq!((clone.memory - original_memory_before).abs(), 5);

        // REPLAYS IDENTICALLY: driving the original with the same input reaches
        // the same state the clone did.
        runner
            .game_mut()
            .resolve_selection(0, crate::action::space::PLAY_HAND_START)
            .expect("original resolves");
        assert_eq!(
            runner.game_mut().memory,
            clone.memory,
            "original and clone must reach identical state from identical input"
        );
    }
}
