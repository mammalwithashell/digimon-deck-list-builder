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
use crate::enums::{EffectSourceKind, PlayerId};
use crate::permanent::PermanentHandle;
use crate::trigger_context::TriggerContext;

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
        /// The `dsl_outer_tail` continuation (already data on `Game` today).
        outer_tail: Option<Arc<Vec<CompiledStep>>>,
        bindings: Bindings,
        runtime: StepRuntime,
        trigger_context: Option<TriggerContext>,
        decline: ResumeDecline,
    },
    /// Recursive multi-pick accumulator — the `count_capped` / source-multi /
    /// dp-budget / play-cost-budget / reveal-bucket / permutation /
    /// partition trampolines. The former `Arc<Mutex<Option<Box<dyn
    /// FnOnce>>>>` threading collapses into plain `accum`/`candidates` data
    /// plus the `then` continuation frame.
    MultiPickStep {
        prov: ResumeProvenance,
        accum: Vec<CardHandle>,
        candidates: Vec<(u16, CardHandle)>,
        min: u8,
        max: u8,
        filter: Option<CompiledPredicate>,
        /// Runs once the pick count is satisfied (or candidates exhausted).
        then: Box<ResumeFrame>,
    },
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
                    outer_tail: None,
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
                    outer_tail: None,
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
                    outer_tail: None,
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
                    outer_tail: None,
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
                    outer_tail: None,
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
                    outer_tail: None,
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
                    outer_tail: None,
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
                    outer_tail: None,
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
                    outer_tail: None,
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
                outer_tail: None,
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
                        outer_tail: None,
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
