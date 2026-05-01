//! Combat resolution — attack flow, battle DP comparison, security checks,
//! interrupt state machine.
//!
//! Flow (PR4+):
//!
//! 1. `attack_digimon` / `attack_player` → `begin_attack` — validates,
//!    suspends the attacker, marks `is_attacking`, installs `PendingAttack`
//!    in `AttackState::Declared`, fires OnAttack.
//! 2. `advance_pending_attack` drives the state machine:
//!    `Declared → AllianceOpen → CounterOpen → BlockOpen → Battle → Cleanup`.
//!    At each open-window state the attempt helper (`try_enter_*`) either
//!    installs a `PendingSelection` and pauses, or auto-advances when no
//!    candidates exist.
//! 3. When a selection resolves, its callback mutates `PendingAttack.state`
//!    and re-enters `advance_pending_attack` to continue.
//! 4. `resolve_pending_battle` reads `effective_target` (which Counter or
//!    Block may have rewritten) and runs the DP comparison / security loop.
//! 5. `cleanup_attack` clears `is_attacking`, expires end-of-attack
//!    modifiers, and drops `pending_attack`.
//!
//! PR4 wires the Block path end-to-end. Alliance and Counter are stubbed as
//! no-op pass-throughs — the state transitions exist so cards can reason
//! about `AttackState`, but no interrupt is offered yet. Full Alliance /
//! Counter implementation lands in PR5.
//!
//! Vortex short-circuits directly from `Declared` to `Battle` after OnAttack
//! — Vortex attacks are uninterruptible per Digimon TCG rules.

use crate::enums::{CardKind, EffectTiming, GamePhase, Keyword, ModifierType, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::selection::{
    AttackState, AttackTarget, PendingAttack, PendingSecurity, PendingSelection, SecurityPhase,
    SecurityResolutionState, SecurityRevealSnapshot, SelectionKind, TriggerSource,
};

/// Phase 9 Task 3 — taxonomy of Counter-window candidates. The broadened
/// window unifies three distinct resolution paths behind a single
/// selection:
///
/// - `Blast` — existing blast-digivolve path (hand card + field target).
/// - `HandOption` — an Option card in the defender's hand with a
///   `.counter()` + `EffectTiming::CounterEffect` effect. Routed through
///   Phase 8's `play_option_from_hand` pipeline with an overlay that
///   fires CounterEffect BEFORE OptionMain.
/// - `FieldAbility` — a defender's battle-area permanent whose top card
///   exposes a `.counter()` + `EffectTiming::CounterEffect` ability.
///   Fired directly via `fire_counter_ability` — no card play, no cost.
///
/// Stored inside the Counter selection callback closure so the chosen
/// action ID resolves to the correct path. See spec
/// `docs/superpowers/specs/2026-04-21-combat-interrupt-completion-design.md`
/// §5 and the plan file for the design rationale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CounterCandidate {
    /// Blast-digivolve from `hand_index` onto `field_index`.
    Blast { hand_index: u8, field_index: u8 },
    /// Play the Option at `hand_index` as a Counter Option (normal cost).
    HandOption { hand_index: u8 },
    /// Activate the Counter ability on battle-area permanent at
    /// `perm_index`.
    FieldAbility { perm_index: u8 },
}

/// Maximum Counter-window depth per attack. DCGO (and the Digimon TCG
/// rules) do not allow nested Counter windows — a counter effect that
/// itself triggers another attack does not open a fresh Counter window
/// for the secondary attack. Spec §5.4.
const MAX_COUNTER_DEPTH: u8 = 1;

/// Phase 9 Task 4 — result of the PostBlock Raid retarget rider.
/// Crate-private; the state machine arm routes on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RaidRetargetOutcome {
    /// Rider didn't apply (target still valid, no Raid, or Player target).
    /// Caller transitions to `Battle`.
    Proceed,
    /// A retarget `PendingSelection` was installed. Caller yields —
    /// `advance_pending_attack` exits on `pending_selection.is_some()`.
    SelectionInstalled,
    /// Rider applied (Raid attacker + invalid target) but no legal
    /// retarget candidate exists. `cleanup_attack` has already run;
    /// caller returns the terminal result directly.
    Fizzled,
}

/// Phase 9 internal dispatcher result for the pair of attack-declaration
/// replacements (`WhenWouldAttack` + `WhenWouldBeAttackTarget`). Kept
/// crate-private; callers route back into the public `AttackResult` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WouldAttackOutcome {
    /// No replacement cancelled or parked; caller continues normally.
    Proceed,
    /// A mandatory cancel fired, or an optional replacement resolved to
    /// cancel. Caller must route through `cleanup_attack(Cancelled)`.
    Cancelled,
    /// An optional replacement installed a `PendingSelection`; caller must
    /// return `InProgress`. Re-entry lands via the selection callback.
    Pending,
}

/// Error returned by script-facing combat helpers
/// (`ctx.redirect_attack`, `ctx.cancel_attack`) when the precondition
/// is not met. Spec §6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackError {
    /// No `pending_attack` is installed — the helper was called outside
    /// an active attack (e.g. from an `OnPlay` observer).
    NoActiveAttack,
    /// The proposed redirect target is not a legal attack target —
    /// wrong controller, not a Digimon (`option_state != Standard`, or
    /// a Tamer/Option/DigiEgg), or an out-of-range handle. For Player
    /// targets: rejecting the attacker's own controller.
    InvalidTarget,
}

/// Result of an attack resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackResult {
    /// Attack couldn't be declared (invalid attacker or target).
    Invalid,
    /// Attacker won the battle (defender was deleted or security absorbed).
    AttackerWins,
    /// Defender won (attacker was deleted).
    DefenderWins,
    /// Both attacker and defender were deleted in battle.
    MutualDestruction,
    /// Security check(s) ended with the attacker still alive; game continues.
    SecurityCheckSurvived,
    /// Security check resulted in the attacker being deleted.
    AttackerDeletedBySecurity,
    /// Attack connected on a defenseless player — game over.
    GameWon,
    /// Phase 9: a `WhenWouldAttack` / `WhenWouldBeAttackTarget` replacement
    /// cancelled the attack before any battle state was observable. No
    /// memory swing, no battle, no security check — but `EndOfAttack` still
    /// fires for symmetry with interrupt-aborted attacks.
    Cancelled,
    /// The attack has paused on an interrupt (Alliance / Counter / Block).
    /// `game.pending_attack` is set with the in-flight state, and a
    /// `pending_selection` is installed for the player who owes a decision.
    /// The final result becomes available after the selection resolves and
    /// the state machine finishes draining.
    InProgress,
}

impl Game {
    /// Compute a permanent's effective DP (base + modifier sum).
    pub fn effective_dp(&self, handle: PermanentHandle) -> Option<i32> {
        let perm = self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)?;
        let base = perm.base_dp(&self.card_data)?;
        let bonus = self.modifiers.sum(handle, ModifierType::ChangeDp);
        Some(base + bonus)
    }

    /// Sum the attacker's digivolution-stack DP adjustments that apply to
    /// the opposing security Digimon during a security DP battle (§2.5e).
    /// Walks every `CardSource` in the stack — any effect flagged with
    /// `applies_to_opponent_security_dp` contributes its `dp_modifier` to
    /// the security Digimon's effective DP, including effects on the top
    /// card. Returns 0 when the attacker has no such effects.
    pub fn attacker_security_dp_adjustment(&self, attacker: PermanentHandle) -> i32 {
        let Some(perm) = self
            .player(attacker.player)
            .battle_area
            .get(attacker.index as usize)
        else {
            return 0;
        };
        let mut total: i32 = 0;
        for source in &perm.card_sources {
            let card_id = source.card_id(&self.card_data);
            let Some(effects) = self.effects_for_card(card_id, source.handle()) else {
                continue;
            };
            for effect in &effects {
                if effect.applies_to_opponent_security_dp {
                    total = total.saturating_add(effect.dp_modifier);
                }
            }
        }
        total
    }

    /// Check whether a permanent can attack right now (atomic — ignores interrupts).
    ///
    /// `vortex` — pass `true` when the attack is invoked via the <Vortex>
    /// end-of-turn mechanic. Vortex exempts summoning sickness at the call
    /// site without adding a persistent keyword (mirrors Python's
    /// `Permanent.can_attack(is_vortex=True)` — see RUST_PYTHON_PARITY §2.1).
    pub fn can_attack(&self, handle: PermanentHandle, vortex: bool) -> bool {
        let perm = match self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
        {
            Some(p) => p,
            None => return false,
        };
        if !perm.is_digimon(&self.card_data) {
            return false;
        }
        if perm.is_suspended {
            return false;
        }
        // Summoning sickness: can't attack on the turn it was played unless
        // Rush is present (native printed OR modifier-granted) or this is a
        // Vortex end-of-turn attack (§2.1b parity fix).
        let is_fresh = perm.turn_played == self.turn_count && perm.turn_digivolved == 0;
        if is_fresh && !vortex && !self.has_keyword(handle, Keyword::Rush) {
            return false;
        }
        true
    }

    /// Attack another Digimon on the opponent's field.
    ///
    /// Returns the terminal battle outcome — **unless** an interrupt
    /// (Block, Alliance, Counter) parks the attack on a `PendingSelection`,
    /// in which case `AttackResult::InProgress` is returned and the caller
    /// continues resolving via `Game::resolve_selection`. The attack
    /// resumes automatically inside that callback and will eventually
    /// return a terminal outcome — inspect it via `last_attack_result`
    /// after the selection chain clears.
    ///
    /// `vortex` — see [`Game::can_attack`]. Vortex attacks bypass all
    /// interrupt windows per Digimon TCG rules.
    pub fn attack_digimon(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
        vortex: bool,
    ) -> AttackResult {
        self.begin_attack(attacker, AttackTarget::Digimon(defender), vortex)
    }

    /// Attack the defending player (security check sequence).
    ///
    /// Same interrupt semantics as `attack_digimon`: a blocker declaration
    /// redirects the attack into a Digimon battle via
    /// `effective_target`, and the call returns `InProgress` if a
    /// selection pauses the flow.
    pub fn attack_player(
        &mut self,
        attacker: PermanentHandle,
        defender_player: PlayerId,
        vortex: bool,
    ) -> AttackResult {
        self.begin_attack(attacker, AttackTarget::Player(defender_player), vortex)
    }

    /// Declare an attack. Validates, installs `PendingAttack`, fires
    /// OnAttack, then hands off to `advance_pending_attack` to drive the
    /// interrupt state machine.
    pub fn begin_attack(
        &mut self,
        attacker: PermanentHandle,
        target: AttackTarget,
        vortex: bool,
    ) -> AttackResult {
        self.begin_attack_impl(attacker, target, vortex, /* is_overclock = */ false)
    }

    /// Declare an `<Overclock>` attack. The sacrifice must already have been
    /// paid by the caller ([`Game::activate_overclock`] drives this via a
    /// `PendingSelection`). Identical to [`Game::begin_attack`] except:
    ///
    /// - The attacker is **not** suspended on declaration (§4.6c-residual
    ///   parity — matches Python `resolve_attack(..., without_suspend=True)`).
    /// - Interrupts (Alliance / Counter / Block) still fire normally; per
    ///   DCGO only Vortex is uninterruptible.
    pub fn begin_attack_overclock(
        &mut self,
        attacker: PermanentHandle,
        target: AttackTarget,
    ) -> AttackResult {
        self.begin_attack_impl(
            attacker, target, /* vortex = */ false, /* is_overclock = */ true,
        )
    }

    fn begin_attack_impl(
        &mut self,
        attacker: PermanentHandle,
        target: AttackTarget,
        vortex: bool,
        is_overclock: bool,
    ) -> AttackResult {
        if !self.can_attack(attacker, vortex) {
            return AttackResult::Invalid;
        }
        // Target validation (Digimon must be a Digimon on field; Player target
        // is always valid — player existence is guaranteed by turn_order).
        match target {
            AttackTarget::Digimon(d) => {
                if !self.handle_valid(d) {
                    return AttackResult::Invalid;
                }
            }
            AttackTarget::Player(_) => {}
        }

        // Install PendingAttack.
        let return_phase = self.current_phase;
        self.pending_attack = Some(PendingAttack {
            attacker,
            original_target: target,
            effective_target: target,
            is_blocked: false,
            blocker: None,
            is_vortex: vortex,
            is_overclock,
            declaration_committed: false,
            cancelled: false,
            battle_occurred: false,
            return_phase,
            state: AttackState::Declared,
            counter_depth: 0,
        });

        // Phase 9: WhenWouldAttack fires on the attacker BEFORE any
        // observable state transitions (suspend / OnAttack / memory). A
        // mandatory cancel at this point rolls the attack back cleanly —
        // attacker not suspended, no Rush turn-count bump, no OnAttack
        // triggers.
        //
        // Optional replacements install a `PendingSelection::Replacement`
        // and we return `InProgress` — the selection callback re-enters by
        // resolving directly to the committed outcome. Spec §4.1 + §6.1.
        match self.fire_would_attack_dispatch(attacker, target) {
            WouldAttackOutcome::Proceed => {}
            WouldAttackOutcome::Cancelled => {
                return self.cleanup_attack(AttackResult::Cancelled);
            }
            WouldAttackOutcome::Pending => {
                return AttackResult::InProgress;
            }
        }

        if let Some(result) = self.commit_attack_declaration(attacker) {
            return result;
        }

        // Vortex short-circuits interrupts.
        if vortex {
            self.transition_attack_state(AttackState::Battle);
        } else {
            self.transition_attack_state(AttackState::AllianceOpen);
        }

        self.advance_pending_attack()
    }

    /// Drive the attack state machine forward. Returns the terminal
    /// `AttackResult` once the flow completes, or `InProgress` if a
    /// selection / queued effect parks the flow. Safe to call when no
    /// attack is in flight — returns `Invalid`.
    ///
    /// Re-entered from selection-resolution callbacks: the callback mutates
    /// `pending_attack.state` (e.g. to `Battle` after a block declaration)
    /// and re-invokes this method to continue.
    pub fn advance_pending_attack(&mut self) -> AttackResult {
        loop {
            // If a selection or effect batch is still pending, yield —
            // advance_pending_attack will be re-entered when it clears.
            if self.pending_selection.is_some() || !self.effect_queue.is_empty() {
                return AttackResult::InProgress;
            }

            let Some(pa) = self.pending_attack.as_ref() else {
                return AttackResult::Invalid;
            };
            let state = pa.state;
            let attacker = pa.attacker;
            let cancelled = pa.cancelled;
            let declaration_committed = pa.declaration_committed;

            // Phase 9: a `WhenWouldAttack` / `WhenWouldBeAttackTarget`
            // replacement (or a `ctx.cancel_attack()` from a later phase)
            // may have flagged the attack as cancelled while an optional
            // replacement selection was resolving. Short-circuit straight
            // to cleanup — EndOfAttack still fires for symmetry; EndOfBattle
            // does NOT (no DP comparison ran).
            if cancelled && state != AttackState::Cleanup {
                return self.cleanup_attack(AttackResult::Cancelled);
            }

            if state != AttackState::Cleanup {
                if declaration_committed {
                    // Attacker liveness check: OnAttack / interrupt effects
                    // may have deleted or moved it. Handles are slot-based, so
                    // require the slot to still be the in-flight attacker
                    // rather than merely any valid Digimon.
                    if !self.handle_still_attacking(attacker) {
                        return self.cleanup_attack(AttackResult::Invalid);
                    }
                } else if !self.handle_valid(attacker) {
                    // Pre-observable replacement resume: the attacker has not
                    // yet been marked attacking, so the normal validity gate is
                    // the only committed state available.
                    return self.cleanup_attack(AttackResult::Invalid);
                }
            }

            match state {
                AttackState::Declared => {
                    if !declaration_committed {
                        if let Some(result) = self.commit_attack_declaration(attacker) {
                            return result;
                        }
                    }
                    // Declared is transient — begin_attack transitions it
                    // immediately. If we're still here, fall through.
                    self.transition_attack_state(AttackState::AllianceOpen);
                }
                AttackState::AllianceOpen => {
                    if !self.try_enter_alliance() {
                        self.transition_attack_state(AttackState::CounterOpen);
                    }
                }
                AttackState::CounterOpen => {
                    if !self.try_enter_counter() {
                        self.transition_attack_state(AttackState::BlockOpen);
                    }
                }
                AttackState::BlockOpen => {
                    // Install a blocker-selection if any candidate exists.
                    // If one installs, next loop iteration sees
                    // pending_selection and yields.
                    if !self.try_enter_block() {
                        self.transition_attack_state(AttackState::PostBlock);
                    }
                }
                AttackState::PostBlock => {
                    // Phase 9 Task 4 — post-Block Raid retarget rider. If
                    // `effective_target` became invalid during the Block
                    // window (OnAttack side-effects, Counter bodies, block
                    // redirects, etc.) AND the attacker has `<Raid>` AND at
                    // least one legal retarget exists, install a retarget
                    // selection. Otherwise fall through to Battle (which
                    // handles its own trivial-connect for invalid targets).
                    match self.try_enter_raid_retarget() {
                        RaidRetargetOutcome::Proceed => {
                            self.transition_attack_state(AttackState::Battle);
                        }
                        RaidRetargetOutcome::SelectionInstalled => {
                            // Selection parked — next loop iteration yields
                            // on `pending_selection.is_some()`.
                        }
                        RaidRetargetOutcome::Fizzled => {
                            // Raid applies, no retarget candidates —
                            // cleanup_attack already ran; exit the state
                            // machine with a terminal Cancelled result.
                            return AttackResult::Cancelled;
                        }
                    }
                }
                AttackState::Battle => {
                    let outcome = self.resolve_pending_battle();
                    if matches!(outcome, AttackResult::InProgress) {
                        // Paused mid-security (a SecuritySkill effect
                        // installed a pending_selection). Keep
                        // `pending_attack` alive in `Battle` so
                        // `advance_security_resolution` can finalize combat
                        // via `cleanup_attack` once the selection resolves
                        // (§2.5j).
                        return AttackResult::InProgress;
                    }

                    // Phase 9 Task 6 — `<Piercing>` post-battle security
                    // check. Fires iff the just-resolved battle was a
                    // Digimon-vs-Digimon match in which the attacker
                    // survived, the defender was wiped, AND the attacker
                    // has `<Piercing>`. We re-check survival here because
                    // OnDeletion / EndOfBattle triggers that ran inside
                    // `resolve_battle` may have deleted the attacker.
                    //
                    // The security pipeline may park on a
                    // `PendingSelection`; in that case we return
                    // `InProgress` and `advance_security_resolution`
                    // finalizes via `cleanup_attack` when the chain clears.
                    if outcome == AttackResult::AttackerWins {
                        if let Some(pa) = self.pending_attack.as_ref() {
                            let attacker_h = pa.attacker;
                            let defender_handle = match pa.effective_target {
                                AttackTarget::Digimon(h) => Some(h),
                                AttackTarget::Player(_) => None,
                            };
                            if let Some(defender_h) = defender_handle {
                                let defender_wiped = !self.handle_valid(defender_h);
                                let attacker_alive = self.handle_valid(attacker_h);
                                if defender_wiped
                                    && attacker_alive
                                    && self.has_keyword(attacker_h, Keyword::Piercing)
                                {
                                    self.transition_attack_state(AttackState::PostBattle);
                                    let piercing_outcome =
                                        self.enter_piercing_security_check(attacker_h);
                                    match piercing_outcome {
                                        AttackResult::InProgress => {
                                            return AttackResult::InProgress;
                                        }
                                        terminal => {
                                            return self.cleanup_attack(terminal);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    return self.cleanup_attack(outcome);
                }
                AttackState::PostBattle => {
                    // Defensive: the Battle arm handles PostBattle inline
                    // (transition + fire + return). If we land here it's
                    // because a selection callback re-entered
                    // `advance_pending_attack` while the security pipeline
                    // is still in flight — but in that case
                    // `advance_security_resolution` is responsible for
                    // finalizing via `cleanup_attack`. Just yield so the
                    // in-flight security resolution can continue.
                    return AttackResult::InProgress;
                }
                AttackState::Cleanup => {
                    // Shouldn't normally reach here — cleanup_attack is
                    // called on transition into Cleanup. Defensive exit.
                    return self.cleanup_attack(AttackResult::Invalid);
                }
            }
        }
    }

    /// Cancel the active attack from an effect body.
    ///
    /// If this is called while the effect queue is draining, cleanup is
    /// deferred to the normal attack resume hook so `EndOfAttack` fires once.
    pub fn cancel_pending_attack_from_effect(&mut self) {
        let Some(pending) = self.pending_attack.as_mut() else {
            return;
        };
        pending.cancelled = true;

        if self.pending_selection.is_none() && self.effect_chain_depth == 0 {
            let _ = self.cleanup_attack(AttackResult::Cancelled);
        }
    }

    // ─── State-machine helpers ────────────────────────────────────────

    fn transition_attack_state(&mut self, new_state: AttackState) {
        if let Some(pa) = self.pending_attack.as_mut() {
            pa.state = new_state;
        }
    }

    /// Phase 9 helper. Drives the two attack-declaration replacement
    /// dispatches back-to-back: first `WhenWouldAttack` on the attacker,
    /// then `WhenWouldBeAttackTarget` on the declared target.
    ///
    /// Returns one of:
    /// - `Proceed` — no cancel, no substitution parked. Caller continues
    ///   into suspend → OnAttack → state machine.
    /// - `Cancelled` — a mandatory cancel fired (or an optional cancel
    ///   installed a selection and its accept path committed cancel).
    ///   Caller routes through `cleanup_attack(Cancelled)`.
    /// - `Pending` — an optional replacement parked a
    ///   `PendingSelection::Replacement`. The caller returns
    ///   `AttackResult::InProgress`. When the selection resolves via the
    ///   generic replacement-accept commit path in `replacement.rs`,
    ///   `pa.cancelled` is set (or `effective_target` is substituted) and
    ///   the next `advance_pending_attack` tick picks up the mutated state.
    ///
    /// Subject mapping:
    /// - `WhenWouldAttack` → `ReplacementSubject::Permanent(attacker)`.
    /// - `WhenWouldBeAttackTarget` → `Permanent(h)` for Digimon targets,
    ///   `Player(pid)` for direct player attacks.
    ///
    /// `Substituted` outcomes:
    /// - On `WhenWouldAttack`: attacker-side substitution is v1-unsupported
    ///   (debug_assert). Spec §4.1 — no meaningful shape for "a different
    ///   attacker takes over"; reserved for future cards.
    /// - On `WhenWouldBeAttackTarget`: rewrites `pending_attack.effective_target`
    ///   and fires the global `OnAttackTargetChange` observer.
    fn fire_would_attack_dispatch(
        &mut self,
        attacker: PermanentHandle,
        target: AttackTarget,
    ) -> WouldAttackOutcome {
        use crate::replacement::{ReplacementCause, ReplacementOutcome, ReplacementSubject};

        // 1. WhenWouldAttack on attacker.
        let outcome = self.try_replace(
            EffectTiming::WhenWouldAttack,
            ReplacementSubject::Permanent(attacker),
            ReplacementCause::Battle,
            None,
        );
        match outcome {
            ReplacementOutcome::Cancelled => {
                return WouldAttackOutcome::Cancelled;
            }
            ReplacementOutcome::Substituted(_) => {
                // Attacker-side substitution isn't supported in v1.
                debug_assert!(
                    false,
                    "WhenWouldAttack Substituted outcome not supported in v1 — \
                     attacker-side replacement only supports Cancelled / None / \
                     CustomHandled. Use WhenWouldBeAttackTarget for redirects."
                );
            }
            ReplacementOutcome::Redirected(_) => {
                // Redirected is not meaningful for attack-shape (no zone move).
                debug_assert!(
                    false,
                    "WhenWouldAttack Redirected outcome not meaningful — \
                     attack replacements do not produce zone moves."
                );
            }
            ReplacementOutcome::None | ReplacementOutcome::CustomHandled => {}
        }

        // Optional replacement may have parked a selection. Yield to the
        // callback to drive re-entry.
        if self.pending_selection.is_some() {
            return WouldAttackOutcome::Pending;
        }
        // The optional-accept path may have committed `cancel()` already —
        // pick that up here before firing the target-side replacement.
        if self.pending_attack.as_ref().is_some_and(|pa| pa.cancelled) {
            return WouldAttackOutcome::Cancelled;
        }

        // 2. WhenWouldBeAttackTarget on declared target.
        let target_subject = match target {
            AttackTarget::Digimon(h) => ReplacementSubject::Permanent(h),
            AttackTarget::Player(pid) => ReplacementSubject::Player(pid),
        };
        let outcome = self.try_replace(
            EffectTiming::WhenWouldBeAttackTarget,
            target_subject,
            ReplacementCause::Battle,
            None,
        );
        match outcome {
            ReplacementOutcome::Cancelled => {
                return WouldAttackOutcome::Cancelled;
            }
            ReplacementOutcome::Substituted(new_subject) => {
                use crate::replacement::ReplacementSubject;
                let new_target = match new_subject {
                    ReplacementSubject::Permanent(h) => AttackTarget::Digimon(h),
                    ReplacementSubject::Player(pid) => AttackTarget::Player(pid),
                    ReplacementSubject::Card(_, _) => {
                        debug_assert!(
                            false,
                            "Attack target substitution does not accept Card subjects"
                        );
                        return WouldAttackOutcome::Proceed;
                    }
                };
                self.apply_attack_target_substitution(new_target);
            }
            ReplacementOutcome::Redirected(_) => {
                debug_assert!(
                    false,
                    "WhenWouldBeAttackTarget Redirected outcome not meaningful — \
                     use Substituted for target rewrites."
                );
            }
            ReplacementOutcome::None | ReplacementOutcome::CustomHandled => {}
        }

        if self.pending_selection.is_some() {
            return WouldAttackOutcome::Pending;
        }
        if self.pending_attack.as_ref().is_some_and(|pa| pa.cancelled) {
            return WouldAttackOutcome::Cancelled;
        }

        WouldAttackOutcome::Proceed
    }

    /// Apply a `Substituted` outcome from `WhenWouldBeAttackTarget`:
    /// rewrite `pending_attack.effective_target` and fire the global
    /// `OnAttackTargetChange` observer. Mirrors the block-redirect
    /// fan-out at `try_enter_block`.
    ///
    /// Shared entry point used by both:
    /// - the dispatcher path (`fire_would_attack_dispatch`) — converts a
    ///   `ReplacementSubject` from an `rctx.substitute(...)` call; and
    /// - the script-facing `EffectContext::redirect_attack` helper (§6.1).
    ///
    /// Callers must have already validated the target; this method does
    /// not re-check target legality. No-op if `pending_attack` is `None`
    /// or `new_target` equals the current `effective_target` (suppress
    /// redundant `OnAttackTargetChange` fan-out).
    pub(crate) fn apply_attack_target_substitution(&mut self, new_target: AttackTarget) {
        // No active attack — silent no-op (callers should have checked).
        let Some(pa) = self.pending_attack.as_mut() else {
            return;
        };
        // Firing-guard: a redirect to the current effective target is a
        // no-op — don't re-fire OnAttackTargetChange.
        if pa.effective_target == new_target {
            return;
        }
        pa.effective_target = new_target;

        // OnAttackTargetChange: global observer fan-out across both players'
        // battle areas. Matches the Block-redirect fire site.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnAttackTargetChange,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();
    }

    /// Validate whether `target` is a legal attack target for `attacker`.
    ///
    /// Rules:
    /// - `AttackTarget::Digimon(h)`: handle must resolve to an on-field
    ///   permanent that is a Digimon with `OptionState::Standard`, and
    ///   must not be the attacker itself (self-attack is never legal).
    ///   Controller is NOT restricted here — redirect_attack on the
    ///   attacker's own side is conceivable for future cards; the existing
    ///   `can_attack` / mask layer handles the "opponent-only" rule for
    ///   attack DECLARATION. Replacement-driven redirects honor whatever
    ///   the replacement author encodes.
    /// - `AttackTarget::Player(pid)`: legal only if `pid != attacker.player`.
    ///
    /// Delayed / Training option permanents are rejected by the
    /// `handle_valid` inner check (`OptionState::Standard`), matching the
    /// existing target-legality guard in `begin_attack_impl`.
    pub(crate) fn validate_attack_target(
        &self,
        attacker: PermanentHandle,
        target: AttackTarget,
    ) -> Result<(), AttackError> {
        match target {
            AttackTarget::Digimon(h) => {
                if h == attacker {
                    return Err(AttackError::InvalidTarget);
                }
                if !self.handle_valid(h) {
                    return Err(AttackError::InvalidTarget);
                }
                Ok(())
            }
            AttackTarget::Player(pid) => {
                if pid == attacker.player {
                    return Err(AttackError::InvalidTarget);
                }
                if (pid as usize) >= self.players.len() {
                    return Err(AttackError::InvalidTarget);
                }
                Ok(())
            }
        }
    }

    /// Scan the attacker's side for unsuspended allies with the Alliance
    /// keyword. Install a `PendingSelection` in `AllianceTiming` if any
    /// candidate exists. Returns `true` if a selection was installed.
    ///
    /// PR5 implementation: any unsuspended ally with modifier-granted
    /// Alliance is a candidate. The declared ally is suspended and the
    /// attacker gains `ALLIANCE_DP_BONUS` DP until end of attack, plus a
    /// +1 Security Attack modifier. Trait-matching (Alliance only fires
    /// when the ally shares a trait with the attacker) is residual work
    /// — blocked on the trait-parsing infrastructure noted in parity
    /// doc §2.1b; for now any Alliance-keyword ally qualifies.
    fn try_enter_alliance(&mut self) -> bool {
        use crate::action::space::encode_attack;

        let Some(pa) = self.pending_attack.as_ref() else {
            return false;
        };
        let attacker = pa.attacker;
        let attacker_player = attacker.player;

        let battle_area_len = self.player(attacker_player).battle_area.len();
        let mut candidates: Vec<u8> = Vec::new();
        for i in 0..battle_area_len {
            // Attacker can't ally with itself.
            if (i as u8) == attacker.index {
                continue;
            }
            let h = PermanentHandle {
                player: attacker_player,
                index: i as u8,
            };
            let perm = &self.player(attacker_player).battle_area[i];
            if perm.is_suspended {
                continue;
            }
            if !perm.is_digimon(&self.card_data) {
                continue;
            }
            if !self.has_keyword(h, Keyword::Alliance) {
                continue;
            }
            candidates.push(i as u8);
        }

        if candidates.is_empty() {
            return false;
        }

        let valid_action_ids: Vec<u16> = candidates
            .iter()
            .map(|&i| encode_attack(0, i as u16))
            .collect();
        let source_card = self.player(attacker_player).battle_area[attacker.index as usize]
            .top_card()
            .handle();

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::AllianceTiming;

        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::OwnField,
            selecting_player: attacker_player,
            previous_phase,
            valid_action_ids,
            is_optional: true, // Alliance is always a "may" declaration.
            prompt: "Declare an ally (Alliance)".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(attacker),
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
                use crate::enums::{Expiry, ModifierType};
                use crate::modifiers::ModifierEntry;

                let offset = action_id.saturating_sub(ATTACK_START);
                let ally_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let ally = PermanentHandle {
                    player: attacker_player,
                    index: ally_index,
                };

                // Grant attacker +ally_dp DP for the duration of the attack,
                // plus +1 security attack. Matches DCGO's Alliance effect.
                let ally_dp = game.effective_dp(ally).unwrap_or(0);
                game.modifiers.add(
                    attacker,
                    ModifierEntry::simple(
                        ModifierType::ChangeDp,
                        ally_dp,
                        Expiry::EndOfAttack,
                        attacker_player,
                    ),
                );
                game.modifiers.add(
                    attacker,
                    ModifierEntry::simple(
                        ModifierType::SecurityAttackChange,
                        1,
                        Expiry::EndOfAttack,
                        attacker_player,
                    ),
                );
                // Suspend the ally.
                if let Some(perm) = game
                    .player_mut(attacker_player)
                    .battle_area
                    .get_mut(ally_index as usize)
                {
                    perm.is_suspended = true;
                }

                if let Some(pa) = game.pending_attack.as_mut() {
                    pa.state = AttackState::CounterOpen;
                }
                game.advance_pending_attack();
            }),
            on_decline: Some(Box::new(move |game: &mut Game| {
                if let Some(pa) = game.pending_attack.as_mut() {
                    pa.state = AttackState::CounterOpen;
                }
                game.advance_pending_attack();
            })),
        });

        true
    }

    /// Scan the defender's hand + battle area for Counter-window
    /// candidates; install a `PendingSelection` in `CounterTiming` if
    /// any candidate exists. Returns `true` on install, `false` if the
    /// defender has no viable counter and the caller should auto-advance
    /// to BlockOpen.
    ///
    /// Phase 9 Task 3 broadens the scan from blast-digivolve-only to a
    /// union over three candidate shapes (see `CounterCandidate`):
    ///   - Blast digivolve candidates (hand card + field target).
    ///   - Hand Counter Options — Option cards in the defender's hand
    ///     with a `.counter()` + `CounterEffect`-timing effect, routed
    ///     through Phase 8's `play_option_from_hand` pipeline.
    ///   - Field Counter abilities — defender's battle-area Digimon with
    ///     a `.counter()` + `CounterEffect`-timing triggered ability.
    ///
    /// Scope: **Digimon-target attacks only**. Player-target attacks
    /// skip Counter to match Python (`combat.py:139`).
    ///
    /// Depth guard: if `pa.counter_depth >= MAX_COUNTER_DEPTH`, the scan
    /// returns `false` immediately — a counter body that launches a
    /// nested attack does NOT open a fresh Counter window (spec §5.4).
    fn try_enter_counter(&mut self) -> bool {
        use crate::action::space::{encode_attack, encode_digivolve, PLAY_HAND_START};

        let Some(pa) = self.pending_attack.as_ref() else {
            return false;
        };

        let defender_player = match pa.effective_target {
            AttackTarget::Digimon(h) => h.player,
            AttackTarget::Player(_) => return false,
        };
        let attacker = pa.attacker;

        // Depth guard (spec §5.4): nested Counter windows are not allowed.
        // The guard triggers on either signal:
        //   - `pa.counter_depth >= MAX_COUNTER_DEPTH` — the *current*
        //     attack has already opened a Counter window once.
        //   - `self.in_counter_window` — we are inside a counter body
        //     that launched this attack (the pending_attack was just
        //     replaced by the nested begin_attack; its counter_depth is
        //     a fresh 0, but `in_counter_window` carries the context).
        if pa.counter_depth >= MAX_COUNTER_DEPTH || self.in_counter_window {
            return false;
        }

        let mut candidates: Vec<CounterCandidate> = Vec::new();

        // 1. Scan the defender's hand.
        let hand_len = self.player(defender_player).hand.len();
        let field_len = self.player(defender_player).battle_area.len();
        for h_idx in 0..hand_len {
            let card = &self.player(defender_player).hand[h_idx];
            let card_id = card.card_id(&self.card_data).to_string();
            let card_handle = card.handle();
            let card_kind = card.card_kind(&self.card_data);
            let Some(effects) = self.effects_for_card(&card_id, card_handle) else {
                continue;
            };

            // Blast digivolve (existing path).
            let has_blast = effects.iter().any(|e| e.blast_digivolve);
            if has_blast {
                // Re-borrow hand with fresh index; effects_for_card only
                // saw a local view.
                let card = &self.player(defender_player).hand[h_idx];
                for f_idx in 0..field_len {
                    let perm = &self.player(defender_player).battle_area[f_idx];
                    if !perm.is_digimon(&self.card_data) {
                        continue;
                    }
                    if !self.can_digivolve(card, perm) {
                        continue;
                    }
                    candidates.push(CounterCandidate::Blast {
                        hand_index: h_idx as u8,
                        field_index: f_idx as u8,
                    });
                }
            }

            // Hand Counter Option (new): Option card with a `.counter()` +
            // CounterEffect-timing effect, and NOT a blast card (blast
            // overlaps both flags; it's scored as a Blast candidate
            // above and must not double-emit as a HandOption).
            if card_kind == CardKind::Option {
                let has_counter_option = effects.iter().any(|e| {
                    e.counter && !e.blast_digivolve && e.timing == EffectTiming::CounterEffect
                });
                if has_counter_option {
                    // Color-match gate parity with Phase 8 Option play
                    // (if the Option's color cannot be satisfied, the
                    // play would fail anyway — don't offer the candidate).
                    let player_ref = self.player(defender_player);
                    let card = &player_ref.hand[h_idx];
                    if crate::action::mask::option_color_match_available(
                        card,
                        player_ref,
                        &self.card_data,
                    ) {
                        candidates.push(CounterCandidate::HandOption {
                            hand_index: h_idx as u8,
                        });
                    }
                }
            }
        }

        // 2. Scan the defender's battle area for field Counter abilities.
        for f_idx in 0..field_len {
            let perm = &self.player(defender_player).battle_area[f_idx];
            // Only Standard-state permanents offer triggered abilities
            // (Delayed / Training / Linked are structural).
            if !matches!(perm.option_state, crate::permanent::OptionState::Standard) {
                continue;
            }
            if !perm.is_digimon(&self.card_data) {
                continue;
            }
            let top = perm.top_card();
            let card_id = top.card_id(&self.card_data).to_string();
            let source_card = top.handle();
            let Some(effects) = self.effects_for_card(&card_id, source_card) else {
                continue;
            };
            let has_field_counter = effects
                .iter()
                .any(|e| e.counter && e.timing == EffectTiming::CounterEffect);
            if has_field_counter {
                candidates.push(CounterCandidate::FieldAbility {
                    perm_index: f_idx as u8,
                });
            }
        }

        if candidates.is_empty() {
            return false;
        }

        // Encode each candidate as a distinct action ID. The mapping is
        // phase-gated (`GamePhase::CounterTiming`) so collisions with
        // other phase's action-ID ranges are not possible at the mask or
        // decoder layer.
        let valid_action_ids: Vec<u16> = candidates
            .iter()
            .map(|c| match c {
                CounterCandidate::Blast {
                    hand_index,
                    field_index,
                } => encode_digivolve(*hand_index as u16, *field_index as u16),
                CounterCandidate::HandOption { hand_index } => PLAY_HAND_START + *hand_index as u16,
                CounterCandidate::FieldAbility { perm_index } => {
                    encode_attack(0, *perm_index as u16)
                }
            })
            .collect();

        // Snapshot the candidate list into the closure for decoding.
        let candidate_snapshot = candidates.clone();

        // Increment the counter-depth guard — subsequent nested attacks
        // launched by the counter body will see `counter_depth >= 1` and
        // skip their own Counter windows.
        if let Some(pa) = self.pending_attack.as_mut() {
            pa.counter_depth = pa.counter_depth.saturating_add(1);
        }

        let source_card = self.player(attacker.player).battle_area[attacker.index as usize]
            .top_card()
            .handle();
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::CounterTiming;

        self.pending_selection = Some(PendingSelection {
            // SelectionKind::Hand is a defensible umbrella — the primary
            // resource is the defender's hand, and the mask renderer is
            // phase-gated (`CounterTiming`) and reads `valid_action_ids`
            // directly, so the kind is not load-bearing for dispatch.
            kind: SelectionKind::Hand,
            selecting_player: defender_player,
            previous_phase,
            valid_action_ids: valid_action_ids.clone(),
            is_optional: true,
            prompt: "Counter (blast / option / field ability)".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(attacker),
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                // Match the resolved action_id back to a candidate.
                let picked = valid_action_ids
                    .iter()
                    .position(|&a| a == action_id)
                    .and_then(|pos| candidate_snapshot.get(pos).copied());
                if let Some(cand) = picked {
                    game.resolve_counter_selection(defender_player, cand);
                }

                // WhenDigivolving / field-counter bodies may have deleted
                // the attacker. If so skip Block + Battle; jump to
                // Cleanup (mirrors DCGO AttackProcess.cs:301).
                let next_state = match game.pending_attack.as_ref() {
                    Some(pa) if game.handle_valid(pa.attacker) => AttackState::BlockOpen,
                    Some(_) => AttackState::Cleanup,
                    None => return,
                };
                game.transition_attack_state(next_state);
                game.advance_pending_attack();
            }),
            on_decline: Some(Box::new(move |game: &mut Game| {
                if let Some(pa) = game.pending_attack.as_mut() {
                    pa.state = AttackState::BlockOpen;
                }
                game.advance_pending_attack();
            })),
        });

        true
    }

    /// Route a resolved Counter candidate to its execution path. Called
    /// from the Counter-selection callback after decoding the action ID
    /// back into a `CounterCandidate`.
    pub(crate) fn resolve_counter_selection(
        &mut self,
        defender: PlayerId,
        candidate: CounterCandidate,
    ) {
        match candidate {
            CounterCandidate::Blast {
                hand_index,
                field_index,
            } => {
                self.execute_blast_digivolve(defender, hand_index as usize, field_index as usize);
            }
            CounterCandidate::HandOption { hand_index } => {
                // Route through Phase 8's Option pipeline with the
                // CounterEffect overlay active. `play_option_core` reads
                // `in_counter_window` to fire CounterEffect BEFORE
                // OptionMain. Clear the flag afterward even if the pipeline
                // pauses (an OptionMain-side selection); a nested re-entry
                // during that pause would still see `in_counter_window =
                // false`, preserving correct CounterEffect scoping.
                self.in_counter_window = true;
                let _result = self.play_option_from_hand(defender, hand_index as usize);
                self.in_counter_window = false;
            }
            CounterCandidate::FieldAbility { perm_index } => {
                let handle = PermanentHandle {
                    player: defender,
                    index: perm_index,
                };
                self.fire_counter_ability(handle);
            }
        }
    }

    /// Fire a field Counter ability: enqueue the permanent's
    /// `CounterEffect`-timing effect(s) and drain. No card play, no cost.
    /// Phase 9 Task 3.
    fn fire_counter_ability(&mut self, handle: PermanentHandle) {
        self.enqueue_triggered(
            EffectTiming::CounterEffect,
            TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue();
    }

    /// Perform the blast-digivolve card movement and fire the
    /// post-digivolve triggers. Called by the `try_enter_counter`
    /// selection callback once the defender picks a (hand, field) pair.
    ///
    /// Effects: move card from defender's hand to the target permanent's
    /// digivolution stack (zero memory), fire `WhenDigivolving` via the
    /// effect queue. No memory payment. `OnCounterTiming` (distinct from
    /// WhenDigivolving; fires before it in Python) is deferred — no pilot
    /// card needs it yet.
    fn execute_blast_digivolve(&mut self, defender: PlayerId, h_idx: usize, f_idx: usize) {
        if h_idx >= self.player(defender).hand.len() {
            return;
        }
        if f_idx >= self.player(defender).battle_area.len() {
            return;
        }
        let card = self.player_mut(defender).hand.remove(h_idx);
        let turn = self.turn_count;
        self.player_mut(defender).battle_area[f_idx].digivolve(card, turn);

        let handle = PermanentHandle {
            player: defender,
            index: f_idx as u8,
        };
        self.enqueue_triggered(
            crate::enums::EffectTiming::WhenDigivolving,
            crate::selection::TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue();
    }

    /// Scan the defender's battle area for unsuspended Digimon with the
    /// Blocker keyword (modifier-granted or native — native/static parsing
    /// still pending §4.3b/§2.1b; only granted Blocker keywords are
    /// honored today). Install a `PendingSelection` in `BlockTiming` if
    /// any candidate exists. Returns `true` if a selection was installed,
    /// `false` if no candidates and the caller should auto-advance to
    /// Battle.
    fn try_enter_block(&mut self) -> bool {
        use crate::action::space::encode_attack;

        let Some(pa) = self.pending_attack.as_ref() else {
            return false;
        };

        // Defender player is derived from effective_target. After a prior
        // Counter redirect this may not equal original_target.
        let defender_player = match pa.effective_target {
            AttackTarget::Digimon(h) => h.player,
            AttackTarget::Player(pid) => pid,
        };
        let attacker = pa.attacker;

        // Self-block is not allowed — attacker cannot block their own
        // attack. Also rules out the edge case where attacker and blocker
        // would be the same permanent.
        let attacker_is_defender = attacker.player == defender_player;

        // §Collision: when the attacker has `Keyword::Collision`, every
        // opponent Digimon is treated as having Blocker for this attack.
        // Mirrors Python's `_is_collision` check in
        // `permanent.py::can_be_blocker`.
        let attacker_has_collision = self.has_keyword(attacker, Keyword::Collision);

        let battle_area_len = self.player(defender_player).battle_area.len();
        let mut candidates: Vec<u8> = Vec::new();
        for i in 0..battle_area_len {
            // Skip the attacker itself (only relevant if attacker side
            // == defender side, i.e. multiplayer self-attacks — impossible
            // today but cheap to guard).
            if attacker_is_defender && (i as u8) == attacker.index {
                continue;
            }
            let h = PermanentHandle {
                player: defender_player,
                index: i as u8,
            };
            let perm = &self.player(defender_player).battle_area[i];
            if perm.is_suspended {
                continue;
            }
            if !perm.is_digimon(&self.card_data) {
                continue;
            }
            // `CannotBlock` (Phase 6 restriction) short-circuits
            // candidacy regardless of Collision — Collision promotes
            // every opponent Digimon to "has Blocker" but does NOT
            // override a printed/modifier `CannotBlock` gate.
            if self.modifiers.has(h, ModifierType::CannotBlock) {
                continue;
            }
            // Blocker required UNLESS the attacker has Collision, which
            // grants Blocker to every opponent Digimon for this attack.
            if !attacker_has_collision && !self.has_keyword(h, Keyword::Blocker) {
                continue;
            }
            candidates.push(i as u8);
        }

        if candidates.is_empty() {
            return false;
        }

        // §8: when the attacker has `<Collision>` AND the candidate list
        // is non-empty, the Block selection is MANDATORY — the defender
        // MUST declare a blocker. Without Collision (or with an empty
        // pool — handled by the early-return above) the selection
        // remains optional.
        let is_optional = !attacker_has_collision;

        let valid_action_ids: Vec<u16> = candidates
            .iter()
            .map(|&i| encode_attack(0, i as u16))
            .collect();

        // Provenance for the selection — attacker's top card. Decorative,
        // not load-bearing; the callback uses captured state, not the
        // source_card field.
        let source_card = self.player(attacker.player).battle_area[attacker.index as usize]
            .top_card()
            .handle();

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::BlockTiming;

        self.pending_selection = Some(PendingSelection {
            // The selecting player is picking from their *own* field; kind
            // reflects that. Block is the *window*, signalled by the phase.
            kind: SelectionKind::OwnField,
            selecting_player: defender_player,
            previous_phase,
            valid_action_ids,
            is_optional,
            prompt: "Declare a blocker".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(attacker),
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
                let offset = action_id.saturating_sub(ATTACK_START);
                let blocker_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let blocker = PermanentHandle {
                    player: defender_player,
                    index: blocker_index,
                };

                if let Some(pa) = game.pending_attack.as_mut() {
                    pa.is_blocked = true;
                    pa.blocker = Some(blocker);
                    pa.effective_target = AttackTarget::Digimon(blocker);
                    pa.state = AttackState::PostBlock;
                }
                // OnAttackTargetChange: fires in all players' battle areas
                // when Block rewrites effective_target. Observer timing for
                // "when an attack is redirected" effects (e.g. Medusamon).
                for pid in 0..game.players.len() {
                    game.enqueue_triggered(
                        crate::enums::EffectTiming::OnAttackTargetChange,
                        crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                    );
                }
                game.drain_effect_queue();

                // Phase 9 Task 8 — OnBlock fires globally after the blocker
                // is declared. Both players' battle areas are scanned;
                // observers can read `game.pending_attack.{attacker,
                // effective_target}` (effective_target now points at the
                // blocker) from within their process closures.
                for pid in 0..game.players.len() {
                    game.enqueue_triggered(
                        crate::enums::EffectTiming::OnBlock,
                        crate::selection::TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
                    );
                }
                game.drain_effect_queue();

                game.advance_pending_attack();
            }),
            on_decline: Some(Box::new(move |game: &mut Game| {
                // Block declined — advance to PostBlock (Raid retarget
                // rider, then Battle). Attack proceeds against its
                // original target unless the target became invalid
                // during the Block window.
                if let Some(pa) = game.pending_attack.as_mut() {
                    pa.state = AttackState::PostBlock;
                }
                game.advance_pending_attack();
            })),
        });

        true
    }

    /// Phase 9 Task 4 — Raid target-switch rider. Evaluates the
    /// post-Block checkpoint:
    ///
    /// 1. Only fires for Digimon-shaped targets. Direct player attacks
    ///    do not retarget via Raid (spec §4.2 — the attack was already
    ///    aimed at a player, Raid's "pick a new Digimon" semantics
    ///    don't apply).
    /// 2. Only fires when `effective_target` is INVALID — i.e. the
    ///    handle no longer resolves to a legal attack target.
    /// 3. Only fires when the attacker has `<Raid>` (modifier-granted
    ///    or native). Mirrors the mask-layer Raid check (see
    ///    `action/mask.rs:137`).
    /// 4. Candidate list uses the Raid-mask prioritization:
    ///    unsuspended opposing Digimon first (tied on max DP),
    ///    falling back to any legal opposing Digimon if no unsuspended
    ///    candidates exist. Legality excludes `CannotAttackTarget`
    ///    (Phase 6 restriction) and requires `OptionState::Standard`.
    ///
    /// Returns `true` iff a selection was installed (and the state
    /// machine must yield). Returns `false` when the rider doesn't
    /// apply OR when no candidates exist; the `false` path leaves the
    /// state machine to transition to Battle, whose existing
    /// "invalid defender → AttackerWins" fallback preserves Phase 4
    /// semantics for the no-Raid / no-Raid-candidate case.
    ///
    /// When Raid candidates exist but the legal set is empty (all
    /// opposing Digimon carry `CannotAttackTarget`, for instance), this
    /// helper runs `cleanup_attack` directly and returns `Fizzled` —
    /// the attack terminates with no battle resolution.
    fn try_enter_raid_retarget(&mut self) -> RaidRetargetOutcome {
        let Some(pa) = self.pending_attack.as_ref() else {
            return RaidRetargetOutcome::Proceed;
        };
        // Raid retarget only applies to Digimon targets. Player attacks
        // don't gain a new target via Raid.
        let target_handle = match pa.effective_target {
            AttackTarget::Digimon(h) => h,
            AttackTarget::Player(_) => return RaidRetargetOutcome::Proceed,
        };
        // If the target is still valid, fall through to Battle — no
        // retarget needed.
        if self.handle_valid(target_handle) {
            return RaidRetargetOutcome::Proceed;
        }
        let attacker = pa.attacker;
        if !self.has_keyword(attacker, Keyword::Raid) {
            return RaidRetargetOutcome::Proceed;
        }

        let candidates = self.raid_retarget_candidates(attacker);
        if candidates.is_empty() {
            // Raid applies but no legal retarget — fizzle the attack.
            self.cleanup_attack(AttackResult::Cancelled);
            return RaidRetargetOutcome::Fizzled;
        }

        self.install_raid_retarget_selection(attacker, candidates);
        RaidRetargetOutcome::SelectionInstalled
    }

    /// Enumerate legal Raid retarget candidates on the opposing side.
    /// Priority mirrors the mask-layer Raid selection (see
    /// `action/mask.rs:137`): unsuspended Digimon tied at max effective
    /// DP, falling back to any legal opposing Digimon when no
    /// unsuspended candidate exists.
    ///
    /// Legality filters:
    /// - Must be a Digimon (not Tamer/Option/DigiEgg).
    /// - Must be in `OptionState::Standard` (excludes Delayed/Training/
    ///   Linked option permanents).
    /// - Must NOT carry `ModifierType::CannotAttackTarget` (Phase 6).
    ///
    /// Returns permanent indices on the opposing side (same ordering
    /// as battle_area).
    fn raid_retarget_candidates(&self, attacker: PermanentHandle) -> Vec<u8> {
        let opp_id = 1 - attacker.player;
        if (opp_id as usize) >= self.players.len() {
            return Vec::new();
        }
        let opp = self.player(opp_id);
        let max_opp = opp.battle_area.len();

        // First pass: unsuspended candidates + track max DP.
        let mut unsuspended: Vec<(u8, i32)> = Vec::new();
        for j in 0..max_opp {
            let t = &opp.battle_area[j];
            if !matches!(t.option_state, crate::permanent::OptionState::Standard) {
                continue;
            }
            if !t.is_digimon(&self.card_data) {
                continue;
            }
            let t_handle = PermanentHandle {
                player: opp_id,
                index: j as u8,
            };
            if self
                .modifiers
                .has(t_handle, ModifierType::CannotAttackTarget)
            {
                continue;
            }
            if t.is_suspended {
                continue;
            }
            let dp = self.effective_dp(t_handle).unwrap_or(0);
            unsuspended.push((j as u8, dp));
        }

        if !unsuspended.is_empty() {
            let max_dp = unsuspended.iter().map(|&(_, dp)| dp).max().unwrap_or(0);
            return unsuspended
                .into_iter()
                .filter(|&(_, dp)| dp == max_dp)
                .map(|(j, _)| j)
                .collect();
        }

        // Fallback: any legal opposing Digimon (suspended or otherwise)
        // when no unsuspended candidates exist. Raid rules permit this
        // — "highest DP if no unsuspended available".
        let mut fallback: Vec<u8> = Vec::new();
        for j in 0..max_opp {
            let t = &opp.battle_area[j];
            if !matches!(t.option_state, crate::permanent::OptionState::Standard) {
                continue;
            }
            if !t.is_digimon(&self.card_data) {
                continue;
            }
            let t_handle = PermanentHandle {
                player: opp_id,
                index: j as u8,
            };
            if self
                .modifiers
                .has(t_handle, ModifierType::CannotAttackTarget)
            {
                continue;
            }
            fallback.push(j as u8);
        }
        fallback
    }

    /// Install the Raid retarget `PendingSelection`. The selecting
    /// player is the attacker's controller. On resolution the selection
    /// routes through `apply_attack_target_substitution` — reusing the
    /// Task 2 entry point that fires `OnAttackTargetChange` globally —
    /// then transitions to `AttackState::Battle` and re-enters the state
    /// machine.
    fn install_raid_retarget_selection(&mut self, attacker: PermanentHandle, candidates: Vec<u8>) {
        use crate::action::space::{encode_attack, ATTACK_START, TARGETS_PER_ATTACKER};

        let attacker_player = attacker.player;
        let opp_id = 1 - attacker_player;
        let valid_action_ids: Vec<u16> = candidates
            .iter()
            .map(|&j| encode_attack(0, j as u16))
            .collect();

        let source_card = self.player(attacker_player).battle_area[attacker.index as usize]
            .top_card()
            .handle();
        let previous_phase = self.current_phase;

        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::OppField,
            selecting_player: attacker_player,
            previous_phase,
            valid_action_ids,
            // Raid retarget is mandatory: once a legal retarget exists,
            // the attacker must pick one (the alternative — fizzling —
            // is reserved for the no-candidates path, which never
            // installs a selection).
            is_optional: false,
            prompt: "Raid — pick a new target".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(attacker),
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let offset = action_id.saturating_sub(ATTACK_START);
                let new_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let new_target = PermanentHandle {
                    player: opp_id,
                    index: new_index,
                };
                // Reuse the shared entry point — rewrites
                // `effective_target` and fires OnAttackTargetChange.
                game.apply_attack_target_substitution(AttackTarget::Digimon(new_target));
                game.transition_attack_state(AttackState::Battle);
                game.advance_pending_attack();
            }),
            on_decline: None,
        });
    }

    /// Resolve the battle once all interrupt windows have cleared.
    /// Reads `effective_target` (which block / counter may have
    /// rewritten) and dispatches to the Digimon-vs-Digimon or
    /// security-loop path accordingly.
    fn resolve_pending_battle(&mut self) -> AttackResult {
        let Some(pa) = self.pending_attack.as_ref() else {
            return AttackResult::Invalid;
        };
        let attacker = pa.attacker;
        let target = pa.effective_target;

        if !self.handle_valid(attacker) {
            return AttackResult::Invalid;
        }

        match target {
            AttackTarget::Digimon(defender) => {
                if !self.handle_valid(defender) {
                    // Defender was deleted between declaration and battle —
                    // attack connects trivially. Matches Python behavior.
                    return AttackResult::AttackerWins;
                }
                self.resolve_battle(attacker, defender)
            }
            AttackTarget::Player(defender_player) => {
                self.resolve_player_security_loop(attacker, defender_player)
            }
        }
    }

    /// Phase 9 Task 6 — fire the `<Piercing>` follow-up security check.
    /// Reuses the standard security-resolution pipeline: counts +
    /// `SecurityAttackChange` modifier sum, honors `Jamming` on the
    /// attacker during the Digimon-battle phase, and drains
    /// `SecuritySkill` / `OnSecurityCheck` / `OnLoseSecurity` normally.
    /// The defending player is inferred from the attack's effective
    /// target, which has just been wiped — we read `attacker.player`'s
    /// opponent for the defender id.
    ///
    /// Returns `AttackResult::InProgress` if the security pipeline
    /// installed a `PendingSelection`; in that case the caller returns
    /// `InProgress` and `advance_security_resolution` finalizes via
    /// `cleanup_attack` when the chain clears. Otherwise returns a
    /// terminal outcome that the caller routes through
    /// `cleanup_attack`.
    fn enter_piercing_security_check(&mut self, attacker: PermanentHandle) -> AttackResult {
        let defender_player: PlayerId = 1 - attacker.player;
        self.resolve_player_security_loop(attacker, defender_player)
    }

    /// Security-check loop for a `Player` attack. Installs the first
    /// `SecurityResolutionState` and drives the phase state machine.
    /// On a pause (a `SecuritySkill` effect installed a
    /// `pending_selection`) returns `AttackResult::InProgress` — the
    /// caller in `advance_pending_attack` leaves `pending_attack` alive in
    /// `AttackState::Battle` so `advance_security_resolution` can finish
    /// combat when the selection resolves.
    ///
    /// See RUST_PYTHON_PARITY §2.5j for the park-and-resume motivation.
    fn resolve_player_security_loop(
        &mut self,
        attacker: PermanentHandle,
        defender_player: PlayerId,
    ) -> AttackResult {
        let sa_modifier = self
            .modifiers
            .sum(attacker, ModifierType::SecurityAttackChange);
        let sa_keyword = self.security_attack_keyword_bonus(attacker);
        let checks = (1 + sa_modifier + sa_keyword).max(0) as u8;
        if checks == 0 {
            return AttackResult::SecurityCheckSurvived;
        }

        // Pop the first card and install the resolution state. Deck-out
        // declares the attacker's controller the winner immediately.
        if !self.pop_and_start_security_check(attacker, defender_player, checks - 1) {
            return AttackResult::GameWon;
        }

        // Drive the state machine. `None` means a `SecuritySkill` (or any
        // subsequent phase) installed a `pending_selection`; resumption
        // happens via `advance_security_resolution` after the selection
        // resolves. `Some(outcome)` means the whole security loop finished.
        match self.drive_security_resolution() {
            None => AttackResult::InProgress,
            Some(outcome) => outcome,
        }
    }

    /// Pop the top security card from `defender`, snapshot its face-up
    /// state on the defender, and install `Game::security_resolution` for
    /// the new check. Returns `false` iff the defender's security stack
    /// was empty — the caller is responsible for `declare_winner` + the
    /// `GameWon` / `Invalid` distinction.
    ///
    /// `checks_remaining` is the number of *additional* checks queued
    /// behind this one; the installed state records it so pause-and-resume
    /// survives across `drive_security_resolution` re-entries.
    fn pop_and_start_security_check(
        &mut self,
        attacker: PermanentHandle,
        defender: PlayerId,
        checks_remaining: u8,
    ) -> bool {
        let sec_card = match self.player_mut(defender).security.pop() {
            Some(c) => c,
            None => {
                let winner = attacker.player;
                self.declare_winner(winner);
                return false;
            }
        };

        // §2.5k: remove from face_up_security on actual reveal so a future
        // return-to-security effect doesn't resurrect a stale face-up entry.
        let was_face_up = self
            .player_mut(defender)
            .face_up_security
            .remove(&sec_card.card_index);

        let card_handle = sec_card.handle();
        let kind = sec_card.card_kind(&self.card_data);

        // §2.5l: snapshot on the defender so `OnSecurityCheck` observer
        // effects can inspect the revealed card even after
        // `pending_security` has been cleared.
        self.player_mut(defender).last_security_reveal = Some(SecurityRevealSnapshot {
            card: card_handle,
            was_face_up,
        });

        // Park the revealed card for the duration of the check.
        self.pending_security = Some(PendingSecurity {
            defender,
            card: sec_card,
            played: false,
        });

        // Install the resolution state in its starting phase.
        let turn_player = self.turn_player();
        self.security_resolution = Some(SecurityResolutionState {
            attacker: Some(attacker),
            defender,
            turn_player,
            revealed_card: card_handle,
            card_kind: kind,
            was_face_up,
            phase: SecurityPhase::SecuritySkillDrain,
            checks_remaining,
            outcome_so_far: AttackResult::SecurityCheckSurvived,
        });

        true
    }

    /// Run the security-resolution phase machine until either a terminal
    /// outcome is reached or a phase's drain pauses on a
    /// `pending_selection`.
    ///
    /// - `Some(outcome)` — every queued security check finished (possibly
    ///   terminating early on `AttackerDeletedBySecurity` / `GameWon`).
    ///   `security_resolution` and `pending_security` have been cleared.
    /// - `None` — paused inside a phase. `security_resolution` is still
    ///   `Some(...)` with its current `phase`. Resume via
    ///   `advance_security_resolution`.
    ///
    /// Phase ordering matches Python's `_execute_security_checks`
    /// (`combat.py:188-221`): SecuritySkill → battle → OnSecurityCheck →
    /// OnLoseSecurity → dispose. See RUST_PYTHON_PARITY §2.5b / §2.5j.
    fn drive_security_resolution(&mut self) -> Option<AttackResult> {
        loop {
            // Safety rail: if `declare_winner` fired mid-drain, unwind.
            if self.game_over {
                // Clean up the transient state so we don't leak it.
                self.pending_security = None;
                self.security_resolution = None;
                return Some(AttackResult::GameWon);
            }

            let Some(state) = self.security_resolution.as_ref() else {
                // Already terminal — shouldn't normally happen.
                return Some(AttackResult::SecurityCheckSurvived);
            };
            let phase = state.phase;

            match phase {
                SecurityPhase::SecuritySkillDrain => {
                    let defender = state.defender;
                    let card_handle = state.revealed_card;
                    // NOTE: Progress / ImmunityToOpponentEffects is NOT
                    // gated here. Per the printed rules (and DCGO's
                    // `ProgressProcess`), Progress makes the attacking
                    // Digimon immune to opponent effects during the attack,
                    // but it does not suppress the defender's SecuritySkill
                    // phase from firing — effects that don't target the
                    // attacker (e.g. Digital Gate Open: play ≤3-cost
                    // Digimon from hand/trash) still resolve normally.
                    // The correct consumption site for Progress is at the
                    // opponent-effect mutation points (selection filters,
                    // delete_permanent with opponent-source attribution,
                    // negative DP modifiers) — tracked in
                    // docs/DCGO_KEYWORD_PARITY.md under "Progress".
                    self.enqueue_triggered(
                        EffectTiming::SecuritySkill,
                        TriggerSource::SecurityRevealed {
                            defender,
                            card: card_handle,
                        },
                    );
                    self.drain_effect_queue();
                    if self.pending_selection.is_some() {
                        return None;
                    }
                    self.set_security_phase(SecurityPhase::BattleResolved);
                }
                SecurityPhase::BattleResolved => {
                    // Only Digimon security runs the DP battle; Option /
                    // Tamer / DigiEgg skip straight through.
                    let Some(state) = self.security_resolution.as_ref() else {
                        break;
                    };
                    let kind = state.card_kind;
                    let attacker_opt = state.attacker;
                    if kind == CardKind::Digimon {
                        if let Some(attacker) = attacker_opt {
                            if self.handle_valid(attacker)
                                && !self
                                    .modifiers
                                    .has(attacker, ModifierType::DontBattleSecurityDigimon)
                            {
                                let attacker_dp = self.effective_dp(attacker).unwrap_or(0);
                                let raw_sec_dp = self
                                    .pending_security
                                    .as_ref()
                                    .and_then(|p| p.card.dp(&self.card_data))
                                    .unwrap_or(0);
                                // §2.5e: attacker's inherited stack may carry
                                // "+N DP when attacking security" modifiers.
                                let sec_dp = raw_sec_dp
                                    .saturating_add(self.attacker_security_dp_adjustment(attacker));
                                if attacker_dp < sec_dp
                                    && !self.has_keyword(attacker, Keyword::Jamming)
                                {
                                    self.delete_permanent_with_effects(attacker);
                                    if let Some(st) = self.security_resolution.as_mut() {
                                        st.outcome_so_far = AttackResult::AttackerDeletedBySecurity;
                                    }
                                }
                            }
                        }
                    }
                    self.set_security_phase(SecurityPhase::OnSecurityCheckDrain);
                }
                SecurityPhase::OnSecurityCheckDrain => {
                    // Scan the defender's battle area for `OnSecurityCheck`
                    // observer effects. Requires an attacker — non-combat
                    // security reveals skip this phase.
                    let Some(state) = self.security_resolution.as_ref() else {
                        break;
                    };
                    if let Some(attacker) = state.attacker {
                        let trigger = TriggerSource::OnSecurityCheck {
                            attacker,
                            defender: state.defender,
                            revealed_card: state.revealed_card,
                            was_face_up: state.was_face_up,
                        };
                        self.enqueue_triggered(EffectTiming::OnSecurityCheck, trigger);
                        self.drain_effect_queue();
                        if self.pending_selection.is_some() {
                            return None;
                        }
                    }
                    self.set_security_phase(SecurityPhase::OnLoseSecurityDrain);
                }
                SecurityPhase::OnLoseSecurityDrain => {
                    let Some(state) = self.security_resolution.as_ref() else {
                        break;
                    };
                    let defender = state.defender;
                    let card_handle = state.revealed_card;
                    self.enqueue_triggered(
                        EffectTiming::OnLoseSecurity,
                        TriggerSource::SecurityRevealed {
                            defender,
                            card: card_handle,
                        },
                    );
                    self.drain_effect_queue();
                    if self.pending_selection.is_some() {
                        return None;
                    }
                    self.set_security_phase(SecurityPhase::Dispose);
                }
                SecurityPhase::Dispose => {
                    // TODO(phase-7-task-6): fire `WhenWouldLoseSecurity` here
                    // (subject: Card(revealed_card, Zone::Security), cause:
                    // SecurityCheck, original_destination: Some(Zone::Trash))
                    // before the `pending_security` trash below. Deferred in
                    // Task 4 because cancelling the security-loss requires
                    // re-inserting the revealed card back into
                    // `player.security` — non-trivial plumbing that's best
                    // addressed alongside native keyword wiring (Task 6).

                    // Trash the revealed card unless an effect raised the
                    // `played` bit via `EffectContext::play_pending_security`.
                    // `security_resolution` stays alive across the observer
                    // drain so a selection installed by an observer can
                    // resume through `DisposeFinalize` (§2.5j residual).
                    let defender = state.defender;
                    let attacker_opt = state.attacker;
                    if let Some(pending) = self.pending_security.take() {
                        if !pending.played {
                            self.player_mut(defender).trash.push(pending.card);
                        }
                    }

                    // OnOpponentSecurityRemoved: fires in the attacker's
                    // battle area after a security card leaves the defender's
                    // stack (trashed or played from security). Medusamon core
                    // archetype observer.
                    if let Some(atk) = attacker_opt {
                        self.enqueue_triggered(
                            crate::enums::EffectTiming::OnOpponentSecurityRemoved,
                            crate::selection::TriggerSource::PlayerBattleArea(atk.player),
                        );
                        self.drain_effect_queue();
                        if self.pending_selection.is_some() {
                            // Park: on resume, `DisposeFinalize` skips the
                            // re-enqueue and proceeds straight to terminal
                            // finalization.
                            self.set_security_phase(SecurityPhase::DisposeFinalize);
                            return None;
                        }
                    }
                    self.set_security_phase(SecurityPhase::DisposeFinalize);
                }
                SecurityPhase::DisposeFinalize => {
                    // Post-observer finalization. `security_resolution` is
                    // still live; drain it now and decide the terminal
                    // outcome (or loop to the next card).
                    let state = self.security_resolution.take().expect("checked Some above");
                    let defender = state.defender;
                    let attacker_opt = state.attacker;
                    let remaining = state.checks_remaining;
                    let outcome = state.outcome_so_far;

                    // Hard terminal: attacker was deleted mid-check.
                    if matches!(outcome, AttackResult::AttackerDeletedBySecurity) {
                        return Some(outcome);
                    }

                    // Continue the outer loop if more checks remain and
                    // the attacker is still on the field.
                    if remaining == 0 {
                        return Some(AttackResult::SecurityCheckSurvived);
                    }
                    let attacker = match attacker_opt {
                        Some(a) => a,
                        None => {
                            // Non-combat reveal path; no iteration.
                            return Some(AttackResult::SecurityCheckSurvived);
                        }
                    };
                    if !self.handle_valid(attacker) {
                        return Some(AttackResult::AttackerDeletedBySecurity);
                    }
                    if !self.pop_and_start_security_check(attacker, defender, remaining - 1) {
                        // Deck out on the next card: attacker wins the game.
                        return Some(AttackResult::GameWon);
                    }
                    // Loop continues with a fresh state in SecuritySkillDrain.
                }
            }
        }

        // Defensive fallback: some branch break'd out (state unexpectedly
        // cleared). Treat as survived.
        Some(AttackResult::SecurityCheckSurvived)
    }

    /// Resume the security state machine after a `pending_selection`
    /// installed inside a `SecuritySkill` / `OnSecurityCheck` /
    /// `OnLoseSecurity` drain has resolved. Called from
    /// `resolve_generic_selection` in `effect_queue.rs` after the
    /// post-callback drain runs. Idempotent.
    ///
    /// If the resumed run finishes the security loop and a combat-side
    /// `pending_attack` is still alive (installed by the attack that
    /// kicked off the security check), finalize it via `cleanup_attack` —
    /// matching the tail of `AttackState::Battle` in
    /// `advance_pending_attack`. See RUST_PYTHON_PARITY §2.5j.
    pub(crate) fn advance_security_resolution(&mut self) {
        // A nested selection was installed by the callback (e.g. the
        // reveal callback immediately installed a select_hand). Re-pause
        // until that one resolves too.
        if self.pending_selection.is_some() {
            return;
        }
        if self.security_resolution.is_none() {
            return;
        }
        let outcome = match self.drive_security_resolution() {
            None => return,
            Some(o) => o,
        };
        if self.pending_attack.is_some() {
            self.cleanup_attack(outcome);
        }
    }

    /// In-place phase mutation. Kept as a small helper so the state-machine
    /// arms don't re-borrow `security_resolution` mutably inline.
    fn set_security_phase(&mut self, phase: SecurityPhase) {
        if let Some(st) = self.security_resolution.as_mut() {
            st.phase = phase;
        }
    }

    /// End-of-attack cleanup: clear `is_attacking`, expire EndOfAttack
    /// modifiers, drop `pending_attack`, and pass `outcome` through to the
    /// caller. Called from both `begin_attack` (early-exit paths) and
    /// `advance_pending_attack` (normal terminal paths).
    fn cleanup_attack(&mut self, outcome: AttackResult) -> AttackResult {
        // Invariant: a cancelled attack never reached battle resolution.
        // `battle_occurred` is set only by `resolve_battle`, which is
        // unreachable once a WhenWouldAttack / WhenWouldBeAttackTarget
        // replacement commits `pa.cancelled = true`.
        debug_assert!(
            !(outcome == AttackResult::Cancelled
                && self
                    .pending_attack
                    .as_ref()
                    .map(|pa| pa.battle_occurred)
                    .unwrap_or(false)),
            "cancelled attack should not have battle_occurred=true"
        );
        if let Some(pa) = self.pending_attack.as_ref() {
            let h = pa.attacker;
            if let Some(perm) = self
                .player_mut(h.player)
                .battle_area
                .get_mut(h.index as usize)
            {
                perm.is_attacking = false;
            }
        }

        // EndOfAttack: observer timing, fires in every player's battle area.
        // Used by "at the end of an attack" effects. Fires before modifiers
        // are expired and pending_attack is cleared, so effects can still
        // inspect the attack context.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                crate::enums::EffectTiming::EndOfAttack,
                crate::selection::TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();

        // Phase 2f4 Task 2: drain ScheduledEffect entries scheduled for
        // EndOfAttack after the printed-observer fan-out. Fires before
        // modifier expiry / pending_attack clear so scheduled bodies see
        // the same attack context as printed observers.
        crate::scheduled_effects::fire_scheduled_for_timing(
            self,
            crate::enums::EffectTiming::EndOfAttack,
        );

        self.modifiers.expire_end_of_attack();
        self.pending_attack = None;
        outcome
    }

    // ─── Private helpers ──────────────────────────────────────────────

    fn suspend_and_count_attack(&mut self, handle: PermanentHandle) {
        let perm = &mut self.players[handle.player as usize].battle_area[handle.index as usize];
        perm.is_suspended = true;
        perm.attacks_this_turn = perm.attacks_this_turn.saturating_add(1);
    }

    fn handle_valid(&self, handle: PermanentHandle) -> bool {
        self.player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|p| {
                // Must be a Digimon (Tamers, DigiEggs, Options aren't attack
                // targets). Phase 8 Task 3 reinforces this for Delayed /
                // Training Options: they live on battle_area but are not
                // attackable. Linked (Task 4) is attached sideways to its
                // host and doesn't occupy a standalone permanent slot.
                p.is_digimon(&self.card_data)
                    && matches!(p.option_state, crate::permanent::OptionState::Standard)
            })
            .unwrap_or(false)
    }

    fn handle_still_attacking(&self, handle: PermanentHandle) -> bool {
        self.player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|p| {
                p.is_attacking
                    && p.is_digimon(&self.card_data)
                    && matches!(p.option_state, crate::permanent::OptionState::Standard)
            })
            .unwrap_or(false)
    }

    fn commit_attack_declaration(&mut self, attacker: PermanentHandle) -> Option<AttackResult> {
        let is_overclock = match self.pending_attack.as_ref() {
            Some(pa) if pa.declaration_committed => return None,
            Some(pa) => pa.is_overclock,
            None => return Some(AttackResult::Invalid),
        };

        // Mark attacker as attacking (§2.2 parity).
        if let Some(perm) = self
            .player_mut(attacker.player)
            .battle_area
            .get_mut(attacker.index as usize)
        {
            perm.is_attacking = true;
        }
        if let Some(pa) = self.pending_attack.as_mut() {
            pa.declaration_committed = true;
        }

        // Suspend + record attack — skipped for Overclock, which attacks
        // without suspending.
        if !is_overclock {
            self.suspend_and_count_attack(attacker);
        }

        // Fire OnAttack (may install a PendingSelection via a triggered
        // effect; the drainer returns and callers check below).
        self.fire_on_attack(attacker);

        if self.pending_selection.is_some() {
            return Some(AttackResult::InProgress);
        }

        // OnAttack may have deleted or moved the attacker, and field indices
        // can shift into the old handle. Bail unless the handle still points
        // at a live attacking permanent. This intentionally allows legal
        // same-permanent stack changes such as digivolution/de-digivolution.
        if !self.handle_still_attacking(attacker) {
            return Some(self.cleanup_attack(AttackResult::Invalid));
        }

        None
    }

    /// Fire OnAttack effects for the attacker, then WhenAttacking for every
    /// permanent in the attacker's battle area (observer timing).
    ///
    /// Thin wrapper over the effect-queue drainer. Single-trigger cases
    /// fire in one step; multi-trigger cases park on a `TriggerOrder`
    /// selection for the attacker's controller to order.
    fn fire_on_attack(&mut self, handle: PermanentHandle) {
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnAttack,
            crate::selection::TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue();
        if self.pending_selection.is_some() || !self.handle_still_attacking(handle) {
            return;
        }

        // WhenAttacking: observer timing — fires for every permanent in the
        // attacker's battle area right after OnAttack. Distinct from OnAttack
        // (which is scoped to the single attacker). Both fire before the
        // Alliance window opens.
        self.enqueue_triggered(
            crate::enums::EffectTiming::WhenAttacking,
            crate::selection::TriggerSource::PlayerBattleArea(handle.player),
        );
        self.drain_effect_queue();
        if self.pending_selection.is_some() || !self.handle_still_attacking(handle) {
            return;
        }

        // Phase 9 Task 8 — OnAllyAttack fan-out. Observer timing firing on
        // every permanent in the attacker-controller's battle area EXCEPT
        // the attacker itself. We piggyback on the PlayerBattleArea scan
        // and filter the attacker's own slot from the queue after enqueue.
        let attacker_queue_start = self.effect_queue.len();
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnAllyAttack,
            crate::selection::TriggerSource::PlayerBattleArea(handle.player),
        );
        // Drop any entries whose source_permanent is the attacker itself
        // — the attacker does not fire its own OnAllyAttack observer.
        let mut i = attacker_queue_start;
        while i < self.effect_queue.len() {
            if self.effect_queue[i].source_permanent == Some(handle) {
                self.effect_queue.remove(i);
            } else {
                i += 1;
            }
        }
        self.drain_effect_queue();
        if self.pending_selection.is_some() || !self.handle_still_attacking(handle) {
            return;
        }

        // Phase 9 Task 8 — OnOpponentAttack fan-out. Observer timing on
        // every permanent in the non-attacker controller's battle area.
        let opp = 1 - handle.player;
        if (opp as usize) < self.players.len() {
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnOpponentAttack,
                crate::selection::TriggerSource::PlayerBattleArea(opp),
            );
            self.drain_effect_queue();
        }
    }

    /// Resolve battle between two permanents by DP comparison.
    ///
    /// Fires `EndOfBattle` for all players' battle areas after the DP
    /// comparison completes (before `EndOfAttack`). This timing only fires
    /// for Digimon-vs-Digimon battles — direct player attacks go through
    /// `resolve_player_security_loop` instead.
    fn resolve_battle(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        if let Some(pa) = self.pending_attack.as_mut() {
            pa.battle_occurred = true;
        }
        let a_dp = self.effective_dp(attacker).unwrap_or(0);
        let d_dp = self.effective_dp(defender).unwrap_or(0);

        // Phase F §F2 — Iceclad (RULES_CONTEXT 16-34): when EITHER
        // combatant has Iceclad in a Digimon-vs-Digimon battle, compare
        // digivolution-card stack lengths (`card_sources.len()`) instead
        // of DP. The security-battle exception is naturally honored:
        // direct player attacks route through `resolve_player_security_loop`
        // which compares DP at `SecurityPhase::BattleResolved` and never
        // calls back into `resolve_battle`. Tie path is unchanged
        // (mutual destruction).
        //
        // DCGO `Iceclad.cs` registers an `IcecladStaticEffect` consulted by
        // the combat resolver — we collapse that registration into a
        // direct `has_keyword` query at the resolver site since the swap
        // is binary and the registry indirection adds nothing.
        let iceclad_active = self.has_keyword(attacker, crate::enums::Keyword::Iceclad)
            || self.has_keyword(defender, crate::enums::Keyword::Iceclad);

        let (a_value, d_value) = if iceclad_active {
            // `card_sources.len()` includes the top card itself. DCGO's
            // `DigivolutionCards` excludes the top, but for comparison
            // the offset cancels (both sides include the +1 top), so
            // length is the correct compare metric.
            let a_count = self.players[attacker.player as usize]
                .battle_area
                .get(attacker.index as usize)
                .map(|p| p.card_sources.len() as i32)
                .unwrap_or(0);
            let d_count = self.players[defender.player as usize]
                .battle_area
                .get(defender.index as usize)
                .map(|p| p.card_sources.len() as i32)
                .unwrap_or(0);
            (a_count, d_count)
        } else {
            (a_dp, d_dp)
        };

        let outcome = if a_value > d_value {
            // Attacker wins — defender is deleted.
            self.delete_permanent_with_cause(
                defender,
                crate::replacement::ReplacementCause::Battle,
            );
            AttackResult::AttackerWins
        } else if a_value < d_value {
            // Defender wins — attacker is deleted.
            self.delete_permanent_with_cause(
                attacker,
                crate::replacement::ReplacementCause::Battle,
            );
            AttackResult::DefenderWins
        } else {
            // Tie — both are deleted. Delete in order: defender first to match
            // DCGO convention, but both need OnDeletion to fire.
            // Since the second deletion can shift indices, re-resolve via card_index
            // to be safe: we use the handles directly since delete_permanent_with_effects
            // reads the top card's card_index before deletion.
            self.delete_permanent_with_cause(
                defender,
                crate::replacement::ReplacementCause::Battle,
            );
            // After deleting defender, attacker's own handle index is unchanged
            // (different player's battle_area), so the attacker handle is still valid.
            if self.handle_valid(attacker) {
                self.delete_permanent_with_cause(
                    attacker,
                    crate::replacement::ReplacementCause::Battle,
                );
            }
            AttackResult::MutualDestruction
        };

        // EndOfBattle: fires only when a Digimon-vs-Digimon battle resolves.
        // Direct player attacks with security loops skip this timing.
        // Fire in every player's battle area, before EndOfAttack.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                crate::enums::EffectTiming::EndOfBattle,
                crate::selection::TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();

        // Phase 2f4 Task 2: drain ScheduledEffect entries scheduled for
        // EndOfBattle after the printed-observer fan-out. EndOfAttack
        // (which always fires next, in `cleanup_attack`) gets its own
        // drain at that site.
        crate::scheduled_effects::fire_scheduled_for_timing(
            self,
            crate::enums::EffectTiming::EndOfBattle,
        );

        outcome
    }

    /// Delete a permanent, firing its OnDeletion effects first.
    /// Also clears any modifiers attached to the handle.
    ///
    /// OnDeletion effects are enqueued + drained before the actual deletion,
    /// so the effect closures can still observe the permanent on the field
    /// (matches the pre-drainer legacy ordering).
    ///
    /// Phase 7: this entry point infers the `ReplacementCause` from live game
    /// state (security-resolution / pending-attack / effect_source_player) and
    /// delegates to `delete_permanent_with_cause`. Callers that already know
    /// the cause (e.g. `resolve_battle` → `Battle`) should invoke
    /// `delete_permanent_with_cause` directly.
    pub fn delete_permanent_with_effects(&mut self, handle: PermanentHandle) {
        let cause = self.infer_deletion_cause(handle);
        self.delete_permanent_with_cause(handle, cause);
    }

    /// Cause-aware deletion entry point. Fires
    /// `WhenWouldLeaveBattleArea` / `WhenWouldBeDeleted` replacement windows
    /// before committing the deletion. See design spec §5, §7.3.
    ///
    /// Nested-select-in-replacement is supported via Phase C's
    /// `Game.parked_replacement` slot — see
    /// `docs/superpowers/specs/2026-04-25-keyword-parity-phase-c-design.md`.
    pub fn delete_permanent_with_cause(
        &mut self,
        handle: PermanentHandle,
        cause: crate::replacement::ReplacementCause,
    ) {
        use crate::enums::{EffectTiming, Zone};
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        let subject = ReplacementSubject::Permanent(handle);

        // Phase 7 Task 3: replacement window — fire the super-timing first,
        // then the route-specific would.
        let leave_outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            subject,
            cause,
            Some(Zone::Trash),
        );
        if self.pending_selection.is_some() {
            // Optional replacement parked a selection. Task 3 stops here;
            // caller re-drives. See doc comment above.
            return;
        }

        let outcome = match leave_outcome {
            ReplacementOutcome::None => self.try_replace(
                EffectTiming::WhenWouldBeDeleted,
                subject,
                cause,
                Some(Zone::Trash),
            ),
            other => other,
        };
        if self.pending_selection.is_some() {
            return;
        }

        match outcome {
            ReplacementOutcome::None => {
                // Phase B §B5: expose the cause to OnDeletion observers via
                // `current_deletion_cause`. Set before the enqueue; cleared
                // after the drain via panic-safe guard.
                let prior = self.current_deletion_cause;
                self.current_deletion_cause = Some(cause);
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.commit_permanent_deletion(handle)
                }));
                self.current_deletion_cause = prior;
                if let Err(payload) = result {
                    std::panic::resume_unwind(payload);
                }
            }
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                // Skip deletion — no OnDeletion / OnAnyDeletion fires.
            }
            ReplacementOutcome::Redirected(Zone::Deck) => {
                // Route-specific return_to_deck does not fire
                // WhenWouldLeaveBattleArea / OnReturn observers at the Task-3
                // fire-site; Task 4 wires those.
                self.return_to_deck(handle, crate::enums::StackPosition::Bottom);
            }
            ReplacementOutcome::Redirected(Zone::Hand) => {
                self.return_to_hand(handle);
            }
            ReplacementOutcome::Redirected(other) => {
                debug_assert!(
                    false,
                    "unexpected redirect destination for WhenWouldBeDeleted: {:?}",
                    other
                );
                self.commit_permanent_deletion(handle);
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(source_h)) => {
                // Partition-like: operate on the substituted permanent. The
                // substituted subject gets its own replacement pass via
                // recursion; depth guard protects against pathological chains.
                self.delete_permanent_with_cause(source_h, cause);
            }
            ReplacementOutcome::Substituted(_) => {
                debug_assert!(
                    false,
                    "non-Permanent substitute subject for WhenWouldBeDeleted"
                );
                self.commit_permanent_deletion(handle);
            }
        }
    }

    /// Core deletion body (OnDeletion → remove → modifier cleanup →
    /// OnAnyDeletion) — shared between the direct fallthrough and the
    /// defensive branches of `delete_permanent_with_cause`.
    ///
    /// **Phase D Task 6 — deferred completion.** If an `OnDeletion`-timed
    /// effect parks a `pending_selection` (e.g. printed `<Save>` asks
    /// the controller to pick a Tamer), the rest of the deletion sequence
    /// (linked-card cascade, `Player::delete_permanent`, modifier cleanup,
    /// `OnAnyDeletion`) MUST defer until that selection resolves —
    /// otherwise the synchronous `delete_permanent` would shift later
    /// permanents' indices, invalidating the parked selection's
    /// `valid_action_ids`. Detected via `pending_selection.is_some()`
    /// after the OnDeletion drain; the deferred state is parked in
    /// `Game.pending_deletion_resume` and resumed by
    /// `resolve_generic_selection`'s post-callback hook (calls
    /// `resume_pending_deletion`).
    fn commit_permanent_deletion(&mut self, handle: PermanentHandle) {
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnDeletion,
            crate::selection::TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue();

        // Phase D Task 6: an OnDeletion handler installed a player choice
        // (e.g. printed Save). Park the rest of the deletion sequence to be
        // finalized after the selection resolves; running synchronously
        // here would shift later permanents' indices and invalidate the
        // parked selection.
        if self.pending_selection.is_some() {
            debug_assert!(
                self.pending_deletion_resume.is_none(),
                "nested deferred deletion not supported (single-outstanding invariant)"
            );
            self.pending_deletion_resume = Some(handle);
            return;
        }

        self.finalize_permanent_deletion(handle);
    }

    /// Run the post-OnDeletion-drain portion of a permanent's deletion:
    /// linked-card cascade, `Player::delete_permanent`, modifier cleanup,
    /// `OnAnyDeletion` enqueue + drain.
    ///
    /// Split out from `commit_permanent_deletion` so the deferred-completion
    /// path (Phase D Task 6) can resume from the same point after a parked
    /// selection resolves. See `commit_permanent_deletion` for the rationale
    /// behind the split.
    ///
    /// Visible to `crate::replacement` so the no-replace path
    /// (`commit_permanent_deletion_no_replace`) can share the same finalize
    /// step (linked-card cascade, post-deletion replays drain, OnAnyDeletion).
    pub(crate) fn finalize_permanent_deletion(&mut self, handle: PermanentHandle) {
        // Phase 8 Task 4: drain any linked cards BEFORE removing the
        // permanent so the OnLinkedCardTrashed observer sees the host still
        // in place (as required by the Appmon-trait card text). Each linked
        // card trashes to the host owner's trash; fire OnLinkedCardTrashed
        // globally if anything was linked.
        //
        // TODO(phase-8-followup): this host-cascade trashes linked cards
        // via a direct `trash.push` — it does NOT route through Phase 7's
        // `WhenWouldBeTrashed` replacement window. v1 constraint (spec
        // §8.2): firing WWBT here while the host is being deleted risks
        // recursive dispatch (a linked-card replacement could redirect or
        // substitute into another leave-the-field event on the same host,
        // spiralling through the deletion chain). Revisit once the
        // replacement recursion model is proven out on simpler fire-sites.
        let had_linked = {
            let linked = self
                .player(handle.player)
                .battle_area
                .get(handle.index as usize)
                .map(|p| !p.linked_cards.is_empty())
                .unwrap_or(false);
            if linked {
                let taken = std::mem::take(
                    &mut self.player_mut(handle.player).battle_area[handle.index as usize]
                        .linked_cards,
                );
                for card in taken {
                    self.player_mut(handle.player).trash.push(card);
                }
                true
            } else {
                false
            }
        };
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnLinkedCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(pid as PlayerId),
                );
            }
            self.drain_effect_queue();
        }

        // Permanent may already have been removed by an OnDeletion effect
        // (self-sacrifice patterns). Check before deleting.
        if self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .is_some()
        {
            self.player_mut(handle.player)
                .delete_permanent(handle.index as usize);
        }
        // Clear any modifiers on the handle (by index), even if the permanent
        // was already gone — modifiers live in a separate registry.
        self.modifiers.clear_permanent(handle);
        // Phase 6: expire any player-scoped modifiers sourced from this permanent.
        self.modifiers.expire_player_on_permanent_leave(handle);

        // Phase D Task 8 — drain post-deletion replays (e.g. printed
        // `<Fortitude>` keyword auto-install). The carrier's OnDeletion
        // handler stashed `(controller, self_card)` here while the carrier
        // was still on field; now that `delete_permanent` has moved the
        // carrier's top + sources to trash, we can safely retrieve the
        // self_card via `play_from_trash_free_unsuspended`. Drained BEFORE
        // the OnAnyDeletion broadcast so observers see the replayed
        // permanent on field.
        //
        // Re-entrancy: if a replayed permanent's OnPlay triggers a second
        // deletion whose OnDeletion handler pushes onto
        // `pending_post_deletion_replays`, that push lands on the
        // freshly-re-created Vec (`std::mem::take` swapped in an empty Vec).
        // The second deletion's own `finalize_permanent_deletion` call will
        // drain it. Order is depth-first, matching DCGO's synchronous
        // resolution model.
        if !self.pending_post_deletion_replays.is_empty() {
            let replays = std::mem::take(&mut self.pending_post_deletion_replays);
            for (controller, card) in replays {
                // Save + set + restore `effect_source_player` around the
                // replay so the replayer's controller drives any
                // downstream effect resolution (especially Progress
                // filtering on the replayed card's ETB), instead of
                // leaking stale attribution from the surrounding
                // deletion chain.
                let prev_source = self.effect_source_player;
                self.effect_source_player = Some(controller);
                {
                    let mut ctx =
                        crate::effect_context::EffectContext::new(self, card, None, controller);
                    // `play_from_trash_free_unsuspended` returns None if the
                    // battle area is full or a CannotPlayDigimonByEffect
                    // modifier is active (DCGO `CanPlayAsNewPermanent` gate).
                    // Silent skip is the correct behavior — the card remains
                    // in trash.
                    let _ = ctx.play_from_trash_free_unsuspended(card);
                }
                self.effect_source_player = prev_source;
            }
        }

        // OnAnyDeletion: global observer — fires in every player's battle area
        // after a permanent is deleted (battle-driven, effect-driven, or
        // security-check). The deleted permanent is already gone from
        // battle_area at this point, so handle-based listeners won't encounter
        // a stale entry.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnAnyDeletion,
                crate::selection::TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();
    }

    /// Resume a deletion that was deferred by `commit_permanent_deletion`
    /// when an `OnDeletion` handler parked a `pending_selection`. Called by
    /// `effect_queue::resolve_generic_selection` after the parked selection
    /// resolves and the post-callback drain returns without re-parking.
    ///
    /// Reads + clears `Game.pending_deletion_resume` and runs
    /// `finalize_permanent_deletion` against the saved handle. If a fresh
    /// selection is parked during finalization (e.g. an `OnAnyDeletion`
    /// observer asks for a target), `pending_deletion_resume` stays cleared
    /// — the new selection drives the resume of its OWN drain through the
    /// effect queue, not through this hook.
    pub(crate) fn resume_pending_deletion(&mut self) {
        let Some(handle) = self.pending_deletion_resume.take() else {
            return;
        };
        self.finalize_permanent_deletion(handle);
    }
}
