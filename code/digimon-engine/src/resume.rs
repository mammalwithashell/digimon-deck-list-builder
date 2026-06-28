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

use digimon_dsl::compiled::{CompiledPredicate, CompiledRevealDestination, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::effect_context::selections::{CountCappedZone, DistinctByMode, RevealBucketSelection};
use crate::enums::{EffectSourceKind, GamePhase, PlayerId, StackPosition};
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
    /// `effect_source_player` scoping. `post` is `None` for a plain
    /// `select_own/opponent_permanent` (bind then run `inner_tail`); `Some` runs
    /// a cost post-action on the picked permanent before the tail (e.g.
    /// `install_trash_bottom_face_down_source_under_tamer`), with the tail
    /// cost-gated on the action succeeding.
    FieldPermanent {
        of_player: PlayerId,
        post: Option<FieldPermanentPostAction>,
    },
    /// `action_id - (SEL_MY_SECURITY_START | SEL_OPP_SECURITY_START)` → security
    /// index of `of_player` (base chosen by whether `of_player` is the
    /// controller). `post: None` is a plain `select_security` (bind the resolved
    /// security `CardHandle` then run `inner_tail`); `Some` runs a post-action
    /// before the tail (e.g. `install_may_add_top_security_to_hand` adds the top
    /// security to hand). Mirrors `FieldPermanent`'s `post`.
    Security {
        of_player: PlayerId,
        post: Option<SecurityPostAction>,
    },
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
    /// `CardHandle`; a stale index skips the bind. `route: None` is a plain
    /// `select_reveal` (bind only); `route: Some(..)` is `choose_from_reveal` —
    /// after binding, the picked card is routed to its destination
    /// (`route_chosen_reveal`) before `inner_tail` runs.
    Reveal { route: Option<RevealRoute> },
    /// `action_id - material_zone_geometry(perm).range_start` → digivolution
    /// source index under carrier `perm` (battle- or breeding-area). Binds the
    /// resolved source `CardHandle` via `material_carrier_permanent`.
    Material { perm: PermanentHandle },
    /// `action_id - HAND_EFFECT_START` → 0-based label index of a "choose one"
    /// prompt (`select_effect_choice`). Not optional (a branch must be picked).
    /// `post: None` is a plain choice: bind the index as a literal
    /// (`insert_literal`) then run `inner_tail`. `post: Some(..)` runs a data
    /// post-action keyed on the chosen index INSTEAD of binding — e.g.
    /// `OrderRemainder` (multi-destination `order_remainder`) maps the index to
    /// a deck position and chains into the already-flipped
    /// `install_remainder_permutation_with_tail` (frame-installs-frame, like
    /// `DnaPairLeft`); `inner_tail` is NOT run here — it becomes the
    /// permutation's tail. `outer_conts` transfer to the permutation frame via
    /// the shared post-match `run_outer_conts`.
    EffectChoice {
        post: Option<EffectChoicePostAction>,
    },
    /// LEFT pick of a `select_dna_pair` (heterogeneous two-pick). The
    /// `candidates` are the left-filter matches (`encode_attack`-encoded, both
    /// battle areas); the arm resolves the picked `left`, binds it via the
    /// frame's `bind_as` (`bind_left_as`), then chains into the already-flipped
    /// `install_select_any_permanent` for the RIGHT pick (excluding `left`,
    /// re-deriving right candidates from `right_filter`), which parks its own
    /// `AnyPermanent` frame. The frame's `inner_tail` is NOT run here — it
    /// becomes the RIGHT pick's tail. `outer_conts` transfer to the right frame
    /// via the shared post-match `run_outer_conts`.
    DnaPairLeft {
        candidates: Vec<(u16, PermanentHandle)>,
        right_filter: CompiledPredicate,
        bind_right_as: String,
        right_prompt: String,
        optional: bool,
    },
    /// Union-zone select spanning hand ∪ trash ∪ material. The tri-range decode
    /// is captured at install as `(action_id, handle, origin)` candidates (the
    /// decode runs once), so the arm linear-searches and binds via
    /// `insert_union_card` (recording the origin zone for downstream replay).
    /// Dual-tail: success uses `inner_tail`; decline uses the frame's `decline`.
    UnionZone {
        of_player: PlayerId,
        candidates: Vec<(u16, CardHandle, UnionZoneOrigin)>,
    },
    /// Attack-target redirect (`select_redirect_attack_target`). `attacker` is
    /// captured at install (from the live `pending_attack`); the arm decodes the
    /// chosen target via `decode_attack` (a `SECURITY_TARGET` index → the player,
    /// else an opponent battle-area Digimon), validates it via
    /// `validate_attack_redirect_target`, then substitutes it with reason
    /// `EffectRedirect(Some(prov.source_card))`. The substitution fires
    /// `OnAttackTargetChange` + drains (NOT atomic), so a nested park threads via
    /// the empty `inner_tail` + `outer_conts` (`run_outer_conts`). No `bind_as`.
    AttackTarget { attacker: PermanentHandle },
    /// Attack-open prompt (`may_attack_now_*` / `force_opponent_attack`). Carries
    /// the `begin_attack_open` parameters captured at install; the target is
    /// decoded from the resolving `action_id` like `AttackTarget`. The arm runs
    /// `begin_attack_open` with `initiator: Effect{ source: prov.source_card,
    /// optional }`, `suspend_attacker: !without_suspending`, `allow_cancel:
    /// optional`. That STARTS the attack sub-machine (counter/block/alliance),
    /// which may park a nested interrupt selection — threaded via the empty
    /// `inner_tail` + `outer_conts` (`run_outer_conts` → `drain_or_rewrap`, which
    /// composes onto a closure-based interrupt window too). No `bind_as`.
    BeginAttack {
        attacker: PermanentHandle,
        without_suspending: bool,
        ignore_summoning_sickness: bool,
        optional: bool,
        cost_upgrade: Option<crate::combat::AttackCostUpgrade>,
    },
}

/// Cost post-action run on a picked field permanent after a `FieldPermanent`
/// resolve, before the (cost-gated) `inner_tail`. Plain data — no closure.
#[derive(Debug, Clone)]
pub enum FieldPermanentPostAction {
    /// `install_trash_bottom_face_down_source_under_tamer`: trash the picked
    /// Tamer's bottom face-down digivolution source (`trash_bottom_face_down_source`);
    /// the tail runs ONLY if the trash succeeded (no-approximations cost gate).
    TrashBottomFaceDownSource,
    /// `try_run_relink` (`relink_self_to_own_digimon`, EX11-027): the picked
    /// permanent is the host; absorb the effect's own standing `source`
    /// permanent onto it as a link card (`absorb_standing_digimon_as_link`).
    /// No bind. The absorb fires `OnDigivolutionCardTrashed` / `OnLinkedCardTrashed`
    /// / `OnLink` + drains (NOT atomic), so a nested park threads via the empty
    /// `inner_tail` + `outer_conts` (`run_outer_conts` → `drain_or_rewrap`),
    /// exactly like `AddTopToHand` / the redirect arm. Host pick is mandatory
    /// (`select_own_permanent(.., false, ..)`) → decline `None`.
    AbsorbStandingAsLink { source: PermanentHandle },
}

/// Post-action run on an `EffectChoice` resolve, keyed on the chosen label
/// index, INSTEAD of binding the index. Plain data — no closure.
#[derive(Debug, Clone)]
pub enum EffectChoicePostAction {
    /// Multi-destination `order_remainder`: the choice picks deck top vs bottom
    /// (`positions[index]`), then the picked position drives
    /// `install_remainder_permutation_with_tail` over the reveal pool, which
    /// parks its own `PermutationStep`. Frame-installs-frame: the `EffectChoice`
    /// frame's `inner_tail` becomes the permutation's tail (not run in the
    /// choice arm). `positions.len()` matches the prompt's label count.
    OrderRemainder {
        positions: Vec<crate::enums::StackPosition>,
        player: PlayerId,
    },
}

/// Post-action run on a `Security` resolve before the `inner_tail`. Plain data —
/// no closure. Mirrors `FieldPermanentPostAction`.
#[derive(Debug, Clone)]
pub enum SecurityPostAction {
    /// `install_may_add_top_security_to_hand`: add the target player's TOP
    /// security card to their hand (`add_top_security_to_hand`). The selection
    /// pins the top slot but the action always adds the top regardless of the
    /// decoded index; no `bind_as`. The "may" (optional) decline is encoded on
    /// the frame's `ResumeDecline` (continue, no add). `add_top_security_to_hand`
    /// drains a security-removed observer, so a nested park threads via the
    /// frame's `outer_conts` (`run_outer_conts` → `drain_or_rewrap_pending_tail`).
    AddTopToHand,
}

/// Data for the `choose_from_reveal` routing performed after a `Reveal` pick
/// resolves (mirrors `route_chosen_reveal`). `target_permanent` is resolved at
/// install time for `BottomSourceOf` (and `None` otherwise), so the resume arm
/// carries no closures — `destination` is plain compiled data.
#[derive(Debug, Clone)]
pub struct RevealRoute {
    pub destination: CompiledRevealDestination,
    pub target_player: PlayerId,
    pub target_permanent: Option<PermanentHandle>,
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
    /// `use_option_from_hand` (play an Option/Dual from hand without paying its
    /// cost, then run the effect tail). Unlike a `RunTail`, the post-pick action
    /// (`use_option_from_hand_without_paying_cost`) can itself install a NESTED
    /// selection (the option's own effect), so the tail is composed via
    /// `drain_or_rewrap_pending_tail` (which threads it onto the nested frame)
    /// rather than run inline. See [`UseOptionFromHandState`].
    UseOptionFromHandStep(UseOptionFromHandState),
    /// One pick stage of the recursive `link_cards` loop (`link 1..N cards from
    /// hand/trash/digivolution-sources to a Digimon`). The state carries the
    /// loop's spec (Arc) + `pick_index` + a per-stage enum; `run_link_pick_step`
    /// decodes the resolving action for that stage and advances the loop (re-park
    /// the next stage, attach + recurse, or run the captured tail), mirroring the
    /// closure callbacks. The post-attach `install_pick(K+1)` runs inline exactly
    /// as the closure path does. See [`crate::dsl_cards::step::link_cards::LinkPickState`].
    LinkPickStep(crate::dsl_cards::step::link_cards::LinkPickState),
    /// Digivolve cost-choice (rule 17 — a base satisfies >1 of a hand card's
    /// digivolution requirements at different costs). A top-level player action,
    /// so it carries no `outer_conts` and is never nested in a DSL clause;
    /// `Game::run_digivolve_cost_choice_step` decodes the chosen route index,
    /// pins it, and re-enters the digivolve. See [`crate::game_actions::digivolve::DigivolveCostChoiceState`].
    DigivolveCostChoice(crate::game_actions::digivolve::DigivolveCostChoiceState),
    /// Player-scoped digivolve cost-reducer accept/decline prompt
    /// (`G-COST-REDUCE-ALLY-DIGIVOLVE`). PASS declines (re-enter at full cost),
    /// accept runs `player_digivolve_reducer_accept`. A player-digivolve action,
    /// never nested in a DSL clause. See [`crate::game_actions::digivolve::DigivolveReducerState`].
    DigivolveReducerPrompt(crate::game_actions::digivolve::DigivolveReducerState),
    /// The suspend-cost pick that the reducer accept path chains into: decode the
    /// suspend target, suspend it, consume the reducer, credit the reduction,
    /// re-enter the digivolve. Mandatory.
    DigivolveReducerSuspend(crate::game_actions::digivolve::DigivolveReducerState),
}

/// In-flight state of a `use_option_from_hand` selection, as data. The pick
/// decodes a hand index (`PLAY_HAND_START` range); the accept arm plays the
/// option (a parking post-action) then `drain_or_rewrap`s the tail; the decline
/// arm (optional only) runs the SAME tail (continue-tail, no clause-abort).
#[derive(Debug, Clone)]
pub struct UseOptionFromHandState {
    pub prov: ResumeProvenance,
    /// Hand owner the option is played from (the installer's `target_player`).
    pub of_player: PlayerId,
    /// The effect tail (shared accept/decline — the closure used identical tails).
    pub tail: Arc<Vec<CompiledStep>>,
    pub bindings: Bindings,
    pub runtime: StepRuntime,
    pub trigger_context: Option<TriggerContext>,
    /// Outer-tail continuations composed when this select is nested in another
    /// clause. Run after the tail (success or decline), via the shared
    /// `run_outer_conts`. Empty for a top-level select.
    pub outer_conts: Vec<OuterContinuation>,
    /// Whether the select was optional (PASS reaches the decline arm only then;
    /// `resolve_generic_selection` rejects PASS on a non-optional select).
    pub optional: bool,
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
    /// `order_remainder` / `remainder_permutation`: instead of binding the
    /// ordered list, place it back on `player`'s deck at `position` (mirrors
    /// `place_remainder_in_order`). `None` for a plain `select_ordered_permutation`
    /// (which binds via `bind_as`). When `Some`, `bind_as` is `None`.
    pub placement: Option<RemainderPlacement>,
    /// Outer-tail continuations composed by `wrap_pending_selection_with_tail`
    /// when this multi-pick select is nested inside another clause. Run at the
    /// TERMINAL (after the accumulated list is bound + inner_tail runs), in push
    /// order — NOT on intermediate re-parks. Empty for a top-level select.
    pub outer_conts: Vec<OuterContinuation>,
}

/// Placement action for a permutation terminal that returns the ordered reveal
/// pool to the deck (`order_remainder` / `remainder_permutation_with_tail`),
/// mirroring `place_remainder_in_order`. Plain data — no closure.
#[derive(Debug, Clone)]
pub struct RemainderPlacement {
    pub player: PlayerId,
    pub position: StackPosition,
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
                    select_kind: ResumeSelectKind::FieldPermanent {
                        of_player: 0,
                        post: None,
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
                    select_kind: ResumeSelectKind::Security { of_player: 0, post: None },
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
                    select_kind: ResumeSelectKind::Security { of_player: 0, post: None },
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
                    select_kind: ResumeSelectKind::Reveal { route: None },
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
            placement: None,
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

    /// DnaPairLeft (frame-installs-frame): resolving the LEFT pick must install
    /// the RIGHT pick (AnyField, excluding the left handle) WITHOUT running the
    /// tail; the tail runs only after the right pick resolves. A clone taken
    /// BETWEEN the two picks (the MCTS operation) must be independent and replay
    /// identically — the cloneability property for the two-pick chain.
    #[test]
    fn dna_pair_left_chains_to_right_and_clones_between_picks() {
        use crate::action::space::encode_attack;
        // Build a game with two Digimon on player 0's field (slots 0 + 1) and a
        // parked DnaPairLeft frame whose right filter matches anything.
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("TEST-DNA-A", "Dna A"))
            .add_card(make_test_card("TEST-DNA-B", "Dna B"))
            .memory(0)
            .start();
        let h0 = runner.place_stack(0, &["TEST-DNA-A"]);
        let h1 = runner.place_stack(0, &["TEST-DNA-B"]);
        let a0 = encode_attack(h0.player as u16, h0.index as u16);
        let a1 = encode_attack(h1.player as u16, h1.index as u16);
        let source_card = runner.top_card(h0);
        {
            let game = runner.game_mut();
            let prev_phase = game.current_phase;
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::AnyField,
                selecting_player: 0,
                previous_phase: prev_phase,
                valid_action_ids: vec![a0, a1],
                is_optional: false,
                prompt: "dna-left".to_string(),
                effect_choices: None,
                source_card,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                callback: Box::new(|_g, _a| unreachable!("resume path drives this")),
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
                    select_kind: ResumeSelectKind::DnaPairLeft {
                        candidates: vec![(a0, h0), (a1, h1)],
                        right_filter: CompiledPredicate::default(),
                        bind_right_as: "right".to_string(),
                        right_prompt: "dna-right".to_string(),
                        optional: false,
                    },
                    bind_as: Some("left".to_string()),
                    inner_tail: Arc::new(vec![CompiledStep::GainMemory(5)]),
                    outer_conts: Vec::new(),
                    bindings: Bindings::new(),
                    runtime: StepRuntime::default(),
                    trigger_context: None,
                    decline: ResumeDecline::None,
                }],
            });
        }

        // Resolve the LEFT pick → installs the RIGHT pick (excluding left), tail
        // NOT yet run.
        runner
            .game_mut()
            .resolve_selection(0, a0)
            .expect("left pick resolves");
        {
            let g = runner.game_mut();
            let pend = g
                .pending_selection
                .as_ref()
                .expect("right pick must be installed after the left pick");
            assert_eq!(
                pend.kind,
                SelectionKind::AnyField,
                "right pick is an AnyField select"
            );
            assert_eq!(
                pend.valid_action_ids,
                vec![a1],
                "right pick must exclude the left handle (only slot 1 remains)"
            );
            assert_eq!(g.memory, 0, "tail must NOT run until the right pick resolves");
            assert!(
                g.pending_selection_resume.is_some(),
                "right pick must be resume-driven (data VM), not closure-only"
            );
        }

        // Clone BETWEEN the two picks (the operation MCTS performs) and resolve
        // the right pick on the clone only.
        let mut clone = runner.game_mut().clone();
        clone
            .resolve_selection(0, a1)
            .expect("clone's right pick resolves");
        assert_eq!(
            clone.memory.abs(),
            5,
            "clone: the DNA-pair tail (GainMemory 5) runs after the right pick"
        );
        // INDEPENDENCE: the original is untouched by resolving the clone.
        assert!(
            runner.game_mut().pending_selection.is_some(),
            "original's right pick must survive cloning + resolving the clone"
        );
        assert_eq!(
            runner.game_mut().memory,
            0,
            "original memory unchanged by the clone's resolution"
        );
        // REPLAYS IDENTICALLY.
        runner
            .game_mut()
            .resolve_selection(0, a1)
            .expect("original's right pick resolves");
        assert_eq!(
            runner.game_mut().memory,
            clone.memory,
            "original and clone reach identical state from the same right pick"
        );
    }
}
