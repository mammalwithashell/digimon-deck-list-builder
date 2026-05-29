//! Combat resolution — attack flow, battle DP comparison, security checks,
//! interrupt state machine.
//!
//! Flow (PR4+):
//!
//! 1. `attack_digimon` / `attack_player` → `begin_attack` — validates,
//!    suspends the attacker, marks `is_attacking`, installs `PendingAttack`
//!    in `AttackState::Declared`, fires OnAttack.
//! 2. `advance_pending_attack` drives the state machine:
//!    `Declared → RaidOpen → AllianceOpen → CounterOpen → BlockOpen → Battle → Cleanup`.
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
//! Alliance, Counter, Raid, and Blocker windows all park through
//! `PendingSelection` when they create player-visible choices.
//!
//! Vortex short-circuits directly from `Declared` to `Battle` after OnAttack
//! — Vortex attacks are uninterruptible per Digimon TCG rules.

use crate::card_source::CardHandle;
use crate::enums::{CardKind, EffectTiming, Expiry, GamePhase, Keyword, ModifierType, PlayerId};
use crate::events::GameEvent;
use crate::game::Game;
use crate::modifiers::ModifierEntry;
use crate::permanent::PermanentHandle;
use crate::selection::{
    AttackState, AttackTarget, PendingAttack, PendingSecurity, PendingSelection, SecurityPhase,
    SecurityResolutionState, SecurityRevealSnapshot, SelectionKind, TriggerSource,
};
use crate::trigger_context::AttackTargetChangeReason;

/// Phase 9 Task 3 — taxonomy of Counter-window candidates. The broadened
/// window unifies three distinct resolution paths behind a single
/// selection:
///
/// - `Blast` — existing blast-digivolve path (hand card + field target).
/// - `BlastDna` — counter-window Blast DNA: result card from hand, one
///   specified field material, then one specified hand material.
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
    /// Blast DNA digivolve from `hand_index`; material selection is a
    /// follow-up pending-selection chain.
    BlastDna { hand_index: u8 },
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

/// Describes why an attack flow was opened. The combat state machine uses a
/// single entry point for natural attacks, effect-created attacks, and
/// keyword/special-rule attacks; the initiator keeps provenance visible for
/// future predicates without changing action IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackInitiator {
    NaturalMainPhase,
    Effect {
        source: Option<CardHandle>,
        optional: bool,
    },
    Overclock,
    Vortex,
}

/// Constraint applied while locking the attack target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetConstraint {
    PlayerOnly,
    DigimonOnly,
    Any,
    Forced(AttackTarget),
}

/// Temporary modifiers attached while an effect-created attack is opened after
/// paying an optional printed upgrade cost. The normal attack cleanup removes
/// these through `Expiry::EndOfAttack`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AttackCostUpgrade {
    pub dp: i32,
    pub security_attack: i32,
}

/// Central attack-flow open request. All attack entry helpers should build one
/// of these and call [`Game::begin_attack_open`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackOpen {
    pub attacker: PermanentHandle,
    pub initiator: AttackInitiator,
    pub suspend_attacker: bool,
    pub target_constraint: TargetConstraint,
    pub allow_cancel: bool,
    pub cost_upgrade: Option<AttackCostUpgrade>,
}

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
    /// The helper was called in an attack phase where that operation is
    /// no longer legal.
    InvalidPhase,
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
    pub fn permanent_is_digimon_for_rules(&self, handle: PermanentHandle) -> bool {
        self.player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| perm.is_digimon_for_rules(&self.card_data, &self.modifiers, handle))
            .unwrap_or(false)
    }

    /// Compute a permanent's effective DP (base + modifier sum).
    ///
    /// Track C / D consult site (2026-05-08): if the target carries any
    /// `ModifierType::ImmuneFromDPMinus` entry, negative `ChangeDp`
    /// modifiers are filtered out before summing — the protection
    /// narrows to DP-minus only, leaving positive `ChangeDp` and the
    /// dynamic aura bonus untouched.
    ///
    /// PUPPETS-G024 (2026-05-20): each `ImmuneFromDPMinus` entry's
    /// `effect_immunity_filter.controller` now scopes which negative
    /// `ChangeDp` deltas it suppresses:
    ///   - `OpponentOnly` — suppress only deltas whose `source_player`
    ///     is the protected permanent's opponent (printed text "can't
    ///     have its DP reduced **by your opponent's effects**"). The
    ///     controller's own DP-reduction still applies.
    ///   - `Any` / no filter — suppress every negative delta (the broad
    ///     variant; back-compat for entries installed without a filter).
    /// A negative `ChangeDp` delta is suppressed if ANY `ImmuneFromDPMinus`
    /// entry's scope covers it.
    pub fn effective_dp(&self, handle: PermanentHandle) -> Option<i32> {
        use crate::modifiers::EffectControllerFilter;
        let perm = self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)?;
        let base = perm.base_dp_for_rules(&self.card_data, &self.modifiers, handle)?;
        let immunity_scopes: Vec<EffectControllerFilter> = self
            .modifiers
            .get(handle, ModifierType::ImmuneFromDPMinus)
            .iter()
            .map(|entry| {
                entry
                    .effect_immunity_filter
                    .map(|f| f.controller)
                    .unwrap_or(EffectControllerFilter::Any)
            })
            .collect();
        let change_dp_sum: i32 = self
            .modifiers
            .get(handle, ModifierType::ChangeDp)
            .iter()
            .filter(|entry| {
                if entry.value >= 0 {
                    return true;
                }
                // Negative delta — suppress if any ImmuneFromDPMinus
                // entry's scope covers this delta's source.
                let from_opponent = entry.source_player != handle.player;
                let suppressed = immunity_scopes.iter().any(|scope| match scope {
                    EffectControllerFilter::Any => true,
                    EffectControllerFilter::OpponentOnly => from_opponent,
                    EffectControllerFilter::OwnOnly => !from_opponent,
                });
                !suppressed
            })
            .map(|entry| entry.value)
            .sum();
        let bonus = change_dp_sum
            + self.static_self_dp_aura_bonus(handle)
            + self.dynamic_dp_aura_bonus(handle);
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
        if !perm.is_digimon_for_rules(&self.card_data, &self.modifiers, handle) {
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

    pub(crate) fn can_attack_without_suspending(
        &self,
        handle: PermanentHandle,
        vortex: bool,
    ) -> bool {
        let perm = match self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
        {
            Some(p) => p,
            None => return false,
        };
        if !perm.is_digimon_for_rules(&self.card_data, &self.modifiers, handle) {
            return false;
        }
        // "Without suspending" bypasses only the suspend cost/unsuspended
        // requirement. Summoning sickness still requires Rush or Vortex.
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

    /// Resolve a Digimon-vs-Digimon battle caused by an effect.
    ///
    /// This is not an attack declaration: it does not suspend the source,
    /// install `PendingAttack`, fire attack timings, open interrupt windows,
    /// or continue into Piercing security checks.
    pub fn battle_digimon(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        if attacker.player == defender.player {
            return AttackResult::Invalid;
        }
        if !self.handle_valid(attacker) || !self.handle_valid(defender) {
            return AttackResult::Invalid;
        }

        self.resolve_battle(attacker, defender)
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
        self.begin_attack_open(AttackOpen {
            attacker,
            initiator: if vortex {
                AttackInitiator::Vortex
            } else {
                AttackInitiator::NaturalMainPhase
            },
            suspend_attacker: true,
            target_constraint: TargetConstraint::Forced(target),
            allow_cancel: false,
            cost_upgrade: None,
        })
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
        self.begin_attack_open(AttackOpen {
            attacker,
            initiator: AttackInitiator::Overclock,
            suspend_attacker: false,
            target_constraint: TargetConstraint::Forced(target),
            allow_cancel: false,
            cost_upgrade: None,
        })
    }

    pub fn begin_attack_open(&mut self, open: AttackOpen) -> AttackResult {
        let AttackOpen {
            attacker,
            initiator,
            suspend_attacker,
            target_constraint,
            allow_cancel: _,
            cost_upgrade,
        } = open;
        let TargetConstraint::Forced(target) = target_constraint else {
            return AttackResult::Invalid;
        };
        let vortex = matches!(initiator, AttackInitiator::Vortex);
        let skips_suspend_cost = !suspend_attacker;

        let attacker_can_attack = if skips_suspend_cost {
            self.can_attack_without_suspending(attacker, vortex)
        } else {
            self.can_attack(attacker, vortex)
        };
        if !attacker_can_attack {
            return AttackResult::Invalid;
        }
        // Target validation (Digimon must be a Digimon on field; Player target
        // is legal unless the attacker is under a CannotAttackPlayer gate;
        // Digimon target is illegal when the attacker's controller is under a
        // player-scoped MayAttackPlayerOnly gate).
        match target {
            AttackTarget::Digimon(d) => {
                if !self.handle_valid(d) {
                    return AttackResult::Invalid;
                }
                // Track C: `MayAttackPlayerOnly` (player-scoped) restricts the
                // attacker's controller to attacking the opposing player only.
                // Companion to permanent-scoped `CannotAttackPlayer`; reject
                // Digimon targets when the modifier is active.
                if self
                    .modifiers
                    .player_has(attacker.player, ModifierType::MayAttackPlayerOnly)
                {
                    return AttackResult::Invalid;
                }
                if self.attack_target_blocked_by_modifier(attacker, d) {
                    return AttackResult::Invalid;
                }
            }
            AttackTarget::Player(_) => {
                if self
                    .modifiers
                    .has(attacker, ModifierType::CannotAttackPlayer)
                {
                    return AttackResult::Invalid;
                }
            }
        }

        if let Some(upgrade) = cost_upgrade {
            if upgrade.dp != 0 {
                self.modifiers.add(
                    attacker,
                    ModifierEntry::simple(
                        ModifierType::ChangeDp,
                        upgrade.dp,
                        Expiry::EndOfAttack,
                        attacker.player,
                    ),
                );
            }
            if upgrade.security_attack != 0 {
                self.modifiers.add(
                    attacker,
                    ModifierEntry::simple(
                        ModifierType::SecurityAttackChange,
                        upgrade.security_attack,
                        Expiry::EndOfAttack,
                        attacker.player,
                    ),
                );
            }
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
            is_overclock: skips_suspend_cost,
            declaration_committed: false,
            cancelled: false,
            battle_occurred: false,
            return_phase,
            state: AttackState::Declared,
            counter_depth: 0,
        });

        // `GameEvent::Attack` emission — spec
        // `engine-event-emission` requires emission at declaration time,
        // before any block resolution or interrupt window opens (which
        // begins at the RaidOpen transition further down). Emitting here
        // (post-PendingAttack-install, pre-WhenWouldAttack-dispatch)
        // ensures consumers see every declared attack — including those
        // later cancelled by a `WhenWouldAttack` replacement — matching
        // the unconditional-emission contract.
        let (attack_target_field_index, attack_target_player) = match target {
            AttackTarget::Digimon(d) => (Some(d.index), Some(d.player)),
            AttackTarget::Player(p) => (None, Some(p)),
        };
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Attack {
            seq,
            player: attacker.player,
            attacker_field_index: attacker.index,
            target_field_index: attack_target_field_index,
            target_player: attack_target_player,
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
            self.transition_attack_state(AttackState::RaidOpen);
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
                    self.transition_attack_state(AttackState::RaidOpen);
                }
                AttackState::RaidOpen => {
                    if !self.try_enter_raid_switch() {
                        self.transition_attack_state(AttackState::AllianceOpen);
                    }
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
    pub(crate) fn cancel_pending_attack_from_effect_checked(&mut self) -> Result<(), AttackError> {
        let Some(pa) = self.pending_attack.as_ref() else {
            return Err(AttackError::NoActiveAttack);
        };
        if !Self::attack_state_allows_effect_cancel(pa.state, pa.counter_depth) {
            return Err(AttackError::InvalidPhase);
        }

        let Some(pending) = self.pending_attack.as_mut() else {
            return Err(AttackError::NoActiveAttack);
        };
        pending.cancelled = true;

        if self.pending_selection.is_none() && self.effect_chain_depth == 0 {
            let _ = self.cleanup_attack(AttackResult::Cancelled);
        }
        Ok(())
    }

    pub fn cancel_pending_attack_from_effect(&mut self) {
        let _ = self.cancel_pending_attack_from_effect_checked();
    }

    /// Explicitly open the Counter window from a card-effect body.
    ///
    /// Normal attacks reach `AttackState::CounterOpen` through
    /// `advance_pending_attack`; this helper exists for DSL/card text that
    /// needs to publish the same window from an interrupt continuation without
    /// constructing private Counter selections. It reuses `try_enter_counter`,
    /// so all candidates still surface through `pending_selection`.
    pub(crate) fn open_counter_window_from_effect_checked(&mut self) -> Result<bool, AttackError> {
        let Some(pa) = self.pending_attack.as_ref() else {
            return Err(AttackError::NoActiveAttack);
        };
        if pa.counter_depth >= MAX_COUNTER_DEPTH
            || self.in_counter_window
            || self.pending_selection.is_some()
        {
            return Err(AttackError::InvalidPhase);
        }

        self.transition_attack_state(AttackState::CounterOpen);
        Ok(self.try_enter_counter())
    }

    fn attack_state_allows_effect_cancel(state: AttackState, counter_depth: u8) -> bool {
        counter_depth == 0
            && matches!(
                state,
                AttackState::Declared
                    | AttackState::RaidOpen
                    | AttackState::AllianceOpen
                    | AttackState::BlockOpen
                    | AttackState::PostBlock
            )
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
                self.apply_attack_target_substitution_with_reason(
                    new_target,
                    AttackTargetChangeReason::EffectRedirect(None),
                );
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
        self.apply_attack_target_substitution_with_reason(
            new_target,
            AttackTargetChangeReason::EffectRedirect(None),
        );
    }

    pub(crate) fn apply_attack_target_substitution_with_reason(
        &mut self,
        new_target: AttackTarget,
        reason: AttackTargetChangeReason,
    ) {
        // No active attack — silent no-op (callers should have checked).
        let Some(pa) = self.pending_attack.as_mut() else {
            return;
        };
        let old_target = pa.effective_target;
        // Firing-guard: a redirect to the current effective target is a
        // no-op — don't re-fire OnAttackTargetChange.
        if old_target == new_target {
            return;
        }
        let attacker = pa.attacker;

        // Track C: `CannotSwitchAttackTarget` on the attacker locks the
        // target — silently drop the substitution. Covers both the
        // dispatcher path (replacement-driven `Substituted` outcome) and
        // the script-facing `EffectContext::redirect_attack` helper.
        if self
            .modifiers
            .has(attacker, ModifierType::CannotSwitchAttackTarget)
        {
            return;
        }
        // Track C: `CannotBeRedirectedAsAttackTarget` on the candidate new
        // target prevents being chosen as the redirected target via this
        // unified path. Player targets bypass — the modifier is permanent-
        // scoped and only applies to Digimon-as-target.
        if let AttackTarget::Digimon(h) = new_target {
            if self
                .modifiers
                .has(h, ModifierType::CannotBeRedirectedAsAttackTarget)
            {
                return;
            }
        }

        pa.effective_target = new_target;

        self.fire_attack_target_change_observers(attacker, old_target, new_target, reason);
    }

    fn fire_attack_target_change_observers(
        &mut self,
        attacker: PermanentHandle,
        old_target: AttackTarget,
        new_target: AttackTarget,
        reason: AttackTargetChangeReason,
    ) {
        let Some(card) = self
            .players
            .get(attacker.player as usize)
            .and_then(|p| p.battle_area.get(attacker.index as usize))
            .map(|perm| perm.top_card().handle())
        else {
            return;
        };
        self.enqueue_triggered(
            EffectTiming::OnAttackTargetChange,
            TriggerSource::AttackTargetChanged {
                player: attacker.player,
                attacker,
                card,
                old_target,
                new_target,
                reason,
            },
        );
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

    pub(crate) fn validate_attack_redirect_target(
        &self,
        attacker: PermanentHandle,
        target: AttackTarget,
    ) -> Result<(), AttackError> {
        if self
            .modifiers
            .has(attacker, ModifierType::CanNotSwitchAttackTarget)
        {
            return Err(AttackError::InvalidTarget);
        }
        self.validate_attack_target(attacker, target)?;
        if let AttackTarget::Digimon(target_handle) = target {
            if self.attack_target_blocked_by_modifier(attacker, target_handle) {
                return Err(AttackError::InvalidTarget);
            }
            if self.modifiers.has(
                target_handle,
                ModifierType::CannotBeRedirectedAsAttackTarget,
            ) {
                return Err(AttackError::InvalidTarget);
            }
        }
        Ok(())
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
            if !self.permanent_is_digimon_for_rules(h) {
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
            source_kind: crate::enums::EffectSourceKind::Digimon,
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
        use crate::action::space::{
            encode_attack, encode_digivolve, DNA_DIGIVOLVE_START, PLAY_HAND_START,
        };

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

            // Blast digivolve / Blast DNA digivolve. DCGO models both at
            // OnCounterTiming; Blast DNA first selects a field material,
            // then a matching hand material.
            let has_blast = effects.iter().any(|e| e.blast_digivolve);
            if has_blast {
                let has_registered_blast_dna =
                    self.hand_card_has_registered_blast_dna_paths(defender_player, h_idx);
                if self.has_valid_blast_dna_route_for_hand_card(defender_player, h_idx) {
                    candidates.push(CounterCandidate::BlastDna {
                        hand_index: h_idx as u8,
                    });
                } else if !has_registered_blast_dna {
                    // Re-borrow hand with fresh index; effects_for_card only
                    // saw a local view.
                    for f_idx in 0..field_len {
                        let handle = PermanentHandle {
                            player: defender_player,
                            index: f_idx as u8,
                        };
                        if !self.permanent_is_digimon_for_rules(handle) {
                            continue;
                        }
                        let card = &self.player(defender_player).hand[h_idx];
                        if self.normal_digivolve_route_for_card(card, handle).is_none() {
                            continue;
                        }
                        candidates.push(CounterCandidate::Blast {
                            hand_index: h_idx as u8,
                            field_index: f_idx as u8,
                        });
                    }
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
                    // Legality parity with Phase 8 Option play: the
                    // candidate surface must match `play_option_from_hand`.
                    let card = &self.player(defender_player).hand[h_idx];
                    if crate::action::mask::option_use_requirement_or_color_available(
                        card,
                        self,
                        defender_player,
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
            let handle = PermanentHandle {
                player: defender_player,
                index: f_idx as u8,
            };
            if !self.permanent_is_digimon_for_rules(handle) {
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
                CounterCandidate::BlastDna { hand_index } => {
                    DNA_DIGIVOLVE_START + *hand_index as u16
                }
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
            source_kind: crate::enums::EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                // Match the resolved action_id back to a candidate.
                let picked = valid_action_ids
                    .iter()
                    .position(|&a| a == action_id)
                    .and_then(|pos| candidate_snapshot.get(pos).copied());
                if let Some(cand) = picked {
                    game.resolve_counter_selection(defender_player, cand);
                }

                if game.pending_selection.is_some() {
                    return;
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
            CounterCandidate::BlastDna { hand_index } => {
                self.initiate_counter_blast_dna(defender, hand_index as usize);
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

    fn initiate_counter_blast_dna(&mut self, defender: PlayerId, result_hand_index: usize) -> bool {
        use crate::action::space::PLAY_HAND_START;

        if result_hand_index >= self.player(defender).hand.len() {
            return false;
        }

        let first_targets: Vec<u16> = self
            .valid_blast_dna_field_targets_for_hand_card(defender, result_hand_index)
            .collect();
        if first_targets.is_empty() {
            return false;
        }

        let previous_phase = self.current_phase;
        let source_card = self.player(defender).hand[result_hand_index].handle();
        self.current_phase = GamePhase::SelectMaterial;
        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Material,
            selecting_player: defender,
            previous_phase,
            valid_action_ids: first_targets,
            is_optional: false,
            prompt: "Select Blast DNA field material".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind: crate::enums::EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let field_idx = action_id as usize;
                let hand_targets: Vec<u16> = game
                    .valid_blast_dna_hand_materials_for_hand_card(
                        defender,
                        result_hand_index,
                        field_idx,
                    )
                    .map(|idx| PLAY_HAND_START + idx)
                    .collect();
                if hand_targets.is_empty() {
                    return;
                }
                game.current_phase = GamePhase::SelectHand;
                game.pending_selection = Some(PendingSelection {
                    kind: SelectionKind::Hand,
                    selecting_player: defender,
                    previous_phase,
                    valid_action_ids: hand_targets,
                    is_optional: false,
                    prompt: "Select Blast DNA hand material".to_string(),
                    effect_choices: None,
                    source_card,
                    source_permanent: None,
                    source_kind: crate::enums::EffectSourceKind::Digimon,
                    callback: Box::new(move |game: &mut Game, action_id: u16| {
                        let material_idx = (action_id - PLAY_HAND_START) as usize;
                        game.execute_blast_dna_digivolve(
                            defender,
                            result_hand_index,
                            field_idx,
                            material_idx,
                        );
                    }),
                    on_decline: None,
                });
            }),
            on_decline: None,
        });
        true
    }

    fn execute_blast_dna_digivolve(
        &mut self,
        defender: PlayerId,
        result_hand_index: usize,
        field_idx: usize,
        material_hand_index: usize,
    ) {
        if self
            .blast_dna_route_for_hand_card(
                defender,
                result_hand_index,
                field_idx,
                material_hand_index,
            )
            .is_none()
        {
            return;
        }
        if field_idx >= self.player(defender).battle_area.len()
            || result_hand_index >= self.player(defender).hand.len()
            || material_hand_index >= self.player(defender).hand.len()
            || result_hand_index == material_hand_index
        {
            return;
        }

        let (first_idx, second_idx, first_is_result) = if result_hand_index > material_hand_index {
            (result_hand_index, material_hand_index, true)
        } else {
            (material_hand_index, result_hand_index, false)
        };
        let first = self.player_mut(defender).hand.remove(first_idx);
        let second = self.player_mut(defender).hand.remove(second_idx);
        let (result_card, material_card) = if first_is_result {
            (first, second)
        } else {
            (second, first)
        };

        let turn = self.turn_count;
        {
            let perm = &mut self.player_mut(defender).battle_area[field_idx];
            perm.card_sources.push(material_card);
            perm.card_sources.push(result_card);
            perm.turn_digivolved = turn;
        }

        let handle = PermanentHandle {
            player: defender,
            index: field_idx as u8,
        };
        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue_with_dna_origin(true);

        self.enqueue_triggered(
            EffectTiming::OnDnaDigivolve,
            TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue_with_dna_origin(true);

        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnDigivolve,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
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

        // Track C: `CannotSwitchAttackTarget` on the attacker locks the
        // declared target — Block (and any redirect path that routes
        // through here) is suppressed. Mirrors BT24-062 MasterBlimpmon's
        // "[Your Turn] This Digimon's attack target can't change."
        if self
            .modifiers
            .has(attacker, ModifierType::CannotSwitchAttackTarget)
        {
            return false;
        }

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
            if !self.permanent_is_digimon_for_rules(h) {
                continue;
            }
            // `CannotBlock` (Phase 6 restriction) short-circuits
            // candidacy regardless of Collision — Collision promotes
            // every opponent Digimon to "has Blocker" but does NOT
            // override a printed/modifier `CannotBlock` gate.
            if self.modifiers.has(h, ModifierType::CannotBlock) {
                continue;
            }
            // Track C: `CannotBeRedirectedAsAttackTarget` on a candidate
            // blocker prevents it from becoming the new attack target via
            // the Block redirect path. Distinct from `CannotBlock`
            // (which forbids declaring a block at all) — this modifier
            // only protects the candidate from being chosen AS the new
            // target, leaving its blocker semantics untouched.
            if self
                .modifiers
                .has(h, ModifierType::CannotBeRedirectedAsAttackTarget)
            {
                continue;
            }
            // Blocker required UNLESS the attacker has Collision, which
            // grants Blocker to every opponent Digimon for this attack.
            if !attacker_has_collision && !self.has_keyword(h, Keyword::Blocker) {
                continue;
            }
            // Blocker declaration rewrites the attack target, so it must
            // honor the same retarget restrictions as effect redirects.
            if self
                .validate_attack_redirect_target(attacker, AttackTarget::Digimon(h))
                .is_err()
            {
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
            source_kind: crate::enums::EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
                let offset = action_id.saturating_sub(ATTACK_START);
                let blocker_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let blocker = PermanentHandle {
                    player: defender_player,
                    index: blocker_index,
                };

                if game
                    .validate_attack_redirect_target(attacker, AttackTarget::Digimon(blocker))
                    .is_err()
                {
                    if let Some(pa) = game.pending_attack.as_mut() {
                        pa.state = AttackState::PostBlock;
                    }
                    game.advance_pending_attack();
                    return;
                }

                if let Some(pa) = game.pending_attack.as_mut() {
                    pa.is_blocked = true;
                    pa.blocker = Some(blocker);
                    pa.state = AttackState::PostBlock;
                }
                // OnAttackTargetChange: fires in all players' battle areas
                // when Block rewrites effective_target. The payload carries
                // attacker, old/new targets, reason, and controller.
                game.apply_attack_target_substitution_with_reason(
                    AttackTarget::Digimon(blocker),
                    AttackTargetChangeReason::Blocker,
                );

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

    /// Open Raid's printed optional attack-target switch immediately after
    /// attack declaration. This is distinct from the post-Block Raid retarget
    /// rider below, which only rescues an attack whose Digimon target became
    /// invalid before battle.
    fn try_enter_raid_switch(&mut self) -> bool {
        let Some(pa) = self.pending_attack.as_ref() else {
            return false;
        };
        let attacker = pa.attacker;
        if !self.has_keyword(attacker, Keyword::Raid) {
            return false;
        }
        // Track C: `CannotSwitchAttackTarget` locks the attack onto its
        // declared target. Raid's printed switch window would rewrite that
        // target, so suppress the optional selection entirely.
        if self
            .modifiers
            .has(attacker, ModifierType::CannotSwitchAttackTarget)
        {
            return false;
        }

        let candidates = self.raid_switch_candidates(attacker, pa.effective_target);
        if candidates.is_empty() {
            return false;
        }

        self.install_raid_switch_selection(attacker, candidates);
        true
    }

    /// Enumerate Raid's printed candidates: opponent unsuspended Digimon tied
    /// for highest DP, excluding the current target because choosing it would
    /// not switch the attack target or fire the observer.
    fn raid_switch_candidates(
        &self,
        attacker: PermanentHandle,
        current_target: AttackTarget,
    ) -> Vec<u8> {
        let opp_id = 1 - attacker.player;
        if (opp_id as usize) >= self.players.len() {
            return Vec::new();
        }

        let opp = self.player(opp_id);
        let mut unsuspended: Vec<(u8, i32)> = Vec::new();
        for (index, target) in opp.battle_area.iter().enumerate() {
            let handle = PermanentHandle {
                player: opp_id,
                index: index as u8,
            };
            if matches!(current_target, AttackTarget::Digimon(current) if current == handle) {
                continue;
            }
            if !matches!(target.option_state, crate::permanent::OptionState::Standard) {
                continue;
            }
            if !self.permanent_is_digimon_for_rules(handle) || target.is_suspended {
                continue;
            }
            if self
                .validate_attack_redirect_target(attacker, AttackTarget::Digimon(handle))
                .is_err()
            {
                continue;
            }
            unsuspended.push((index as u8, self.effective_dp(handle).unwrap_or(0)));
        }

        let Some(max_dp) = unsuspended.iter().map(|&(_, dp)| dp).max() else {
            return Vec::new();
        };
        unsuspended
            .into_iter()
            .filter(|&(_, dp)| dp == max_dp)
            .map(|(index, _)| index)
            .collect()
    }

    fn install_raid_switch_selection(&mut self, attacker: PermanentHandle, candidates: Vec<u8>) {
        use crate::action::space::{encode_attack, ATTACK_START, TARGETS_PER_ATTACKER};

        let attacker_player = attacker.player;
        let opp_id = 1 - attacker_player;
        let valid_action_ids: Vec<u16> = candidates
            .iter()
            .map(|&index| encode_attack(0, index as u16))
            .collect();

        let source_card = self.player(attacker_player).battle_area[attacker.index as usize]
            .top_card()
            .handle();
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectTarget;

        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::OppField,
            selecting_player: attacker_player,
            previous_phase,
            valid_action_ids,
            is_optional: true,
            prompt: "Raid - switch attack target".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(attacker),
            source_kind: crate::enums::EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let offset = action_id.saturating_sub(ATTACK_START);
                let new_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let new_target = PermanentHandle {
                    player: opp_id,
                    index: new_index,
                };
                if game
                    .validate_attack_redirect_target(attacker, AttackTarget::Digimon(new_target))
                    .is_ok()
                {
                    game.apply_attack_target_substitution_with_reason(
                        AttackTarget::Digimon(new_target),
                        AttackTargetChangeReason::Raid,
                    );
                }
                game.transition_attack_state(AttackState::AllianceOpen);
                game.advance_pending_attack();
            }),
            on_decline: Some(Box::new(move |game: &mut Game| {
                if let Some(pa) = game.pending_attack.as_mut() {
                    pa.state = AttackState::AllianceOpen;
                }
                game.advance_pending_attack();
            })),
        });
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
        // Track C: `CannotSwitchAttackTarget` on the attacker suppresses
        // Raid retarget. The attack proceeds with its (now-invalid)
        // effective_target — `resolve_pending_battle` treats a deleted
        // defender as a clean attacker-wins, matching the no-Raid path.
        // Without this gate the engine would still install the mandatory
        // retarget selection, then `apply_attack_target_substitution`
        // would silently no-op when the user picks — confusing UX.
        if self
            .modifiers
            .has(attacker, ModifierType::CannotSwitchAttackTarget)
        {
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
    /// - Must pass shared attack-retarget validation, including
    ///   `CannotAttackTarget`, `CannotBeRedirectedAsAttackTarget` on
    ///   the candidate, and `CanNotSwitchAttackTarget` on the attacker.
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
            let t_handle = PermanentHandle {
                player: opp_id,
                index: j as u8,
            };
            if !self.permanent_is_digimon_for_rules(t_handle) {
                continue;
            }
            if self
                .validate_attack_redirect_target(attacker, AttackTarget::Digimon(t_handle))
                .is_err()
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
            let t_handle = PermanentHandle {
                player: opp_id,
                index: j as u8,
            };
            if !self.permanent_is_digimon_for_rules(t_handle) {
                continue;
            }
            if self
                .validate_attack_redirect_target(attacker, AttackTarget::Digimon(t_handle))
                .is_err()
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
            source_kind: crate::enums::EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let offset = action_id.saturating_sub(ATTACK_START);
                let new_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let new_target = PermanentHandle {
                    player: opp_id,
                    index: new_index,
                };
                if game
                    .validate_attack_redirect_target(attacker, AttackTarget::Digimon(new_target))
                    .is_err()
                {
                    game.cleanup_attack(AttackResult::Cancelled);
                    return;
                }
                // Reuse the shared entry point — rewrites
                // `effective_target` and fires OnAttackTargetChange.
                game.apply_attack_target_substitution_with_reason(
                    AttackTarget::Digimon(new_target),
                    AttackTargetChangeReason::Raid,
                );
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
                // count_as_digivolve_driven_attack=true: this is the
                // primary Player-target attack arm (after blocks have
                // already redirected to a Digimon target if any). Piercing
                // follow-ups enter via `enter_piercing_security_check`
                // with `false` and therefore do NOT bump the counter.
                self.resolve_player_security_loop(attacker, defender_player, true)
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
        // `<Security A.>` recompute: piercing follow-ups delegate to
        // `resolve_player_security_loop`, so they automatically inherit
        // the per-iteration recompute from
        // `fix-security-check-recompute-mid-attack`. Mid-piercing-attack
        // digivolves / modifier grants land via the same path.
        //
        // count_as_digivolve_driven_attack=false: the originating attack
        // was on a Digimon target; per the gameplay-reward-config spec
        // the counter only increments for primary Player-target attacks.
        self.resolve_player_security_loop(attacker, defender_player, false)
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
    ///
    /// `<Security A.>` recomputation: matches DCGO's
    /// `Permanent.Strike` getter ([`CardController.cs:3956-3987`] +
    /// [`Permanent.cs:1818-1951`]). The effective strike is re-read at
    /// each iteration boundary by [`Self::current_security_strike`],
    /// so mid-attack digivolves / modifier gains / `ChangeSAttack`
    /// effects that land between checks extend (or shorten) the loop
    /// as DCGO would. See change `fix-security-check-recompute-mid-attack`.
    fn resolve_player_security_loop(
        &mut self,
        attacker: PermanentHandle,
        defender_player: PlayerId,
        count_as_digivolve_driven_attack: bool,
    ) -> AttackResult {
        // Start with `checks_performed = 0`. The first iteration's
        // pre-pop guard below re-reads the live strike; if it is 0 we
        // skip the loop entirely (handles `<Jamming>` and `0 base_checks`
        // attackers without popping any security cards).
        let initial_strike = self.current_security_strike(attacker);
        if initial_strike == 0 {
            return AttackResult::SecurityCheckSurvived;
        }

        // Gameplay-reward-config: count this as a "digivolve-driven
        // attack" iff (a) we're the primary Player-target arm (not a
        // piercing follow-up), (b) the attacker's effective level is
        // ≥ 5 — the engine-side lower bound. The Python component
        // re-checks against its own (configurable) `attacker_min_level`.
        // Per-attack semantics: one increment regardless of how many
        // security cards Security Attack +N reveals. Incremented before
        // the first pop so deck-out (returns GameWon) still counts.
        if count_as_digivolve_driven_attack {
            let attacker_level = self
                .player(attacker.player)
                .battle_area
                .get(attacker.index as usize)
                .and_then(|perm| perm.level_for_rules(&self.card_data, &self.modifiers, attacker))
                .unwrap_or(0);
            if attacker_level >= 5 {
                self.n_digivolve_driven_attacks[attacker.player as usize] += 1;
            }
        }

        // Pop the first card and install the resolution state. Deck-out
        // declares the attacker's controller the winner immediately.
        if !self.pop_and_start_security_check(attacker, defender_player, 0) {
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

    /// Effective `<Security A.>` total for `attacker` right now, in DCGO
    /// `Permanent.Strike` shape: `base_checks (aura bonus or 1) +
    /// SecurityAttackChange modifier sum + ChangeSAttack payload deltas
    /// (with invert honored) + native/printed Security-Attack keyword
    /// bonus`, clamped to `[0, MAX_SECURITY_CHECKS]`.
    ///
    /// Returns `0` when the attacker handle is no longer valid (e.g.
    /// deleted or returned to hand mid-resolution). The `DisposeFinalize`
    /// arm of [`Self::drive_security_resolution`] treats `0` as
    /// "loop done" and routes through the existing
    /// `AttackerDeletedBySecurity` / `SecurityCheckSurvived` decision.
    ///
    /// Bounded by [`Self::MAX_SECURITY_CHECKS`] as a belt-and-braces
    /// safety cap: any future modifier interaction that produces a
    /// runaway strike value is clamped here and an `EffectFizzled`
    /// event is emitted on first overflow.
    fn current_security_strike(&mut self, attacker: PermanentHandle) -> u8 {
        if !self.handle_valid(attacker) {
            return 0;
        }
        let sa_modifier = self
            .modifiers
            .sum(attacker, ModifierType::SecurityAttackChange);
        let change_s_attack: i32 = self
            .modifiers
            .get(attacker, ModifierType::ChangeSAttack)
            .into_iter()
            .map(|entry| match &entry.payload {
                crate::modifiers::ModifierPayload::SecurityAttack { delta, invert } => {
                    if *invert {
                        -*delta
                    } else {
                        *delta
                    }
                }
                crate::modifiers::ModifierPayload::None => entry.value,
                _ => 0,
            })
            .sum();
        let sa_keyword = self.security_attack_keyword_bonus(attacker);
        let base_checks = self
            .dynamic_security_attack_aura_bonus(attacker)
            .unwrap_or(1);
        let raw = base_checks + sa_modifier + change_s_attack + sa_keyword;
        let clamped = raw.max(0) as u32;
        if clamped > Self::MAX_SECURITY_CHECKS as u32 {
            // Safety cap. Log + emit a single fizzle so test runners and
            // recordings can surface the abnormal state. Cap the return
            // value so the caller terminates cleanly.
            self.logger.log(&format!(
                "[Safety] current_security_strike clamped {} -> {} for attacker P{} slot {}",
                clamped,
                Self::MAX_SECURITY_CHECKS,
                attacker.player,
                attacker.index,
            ));
            let seq = self.next_event_seq();
            self.events.push(GameEvent::EffectFizzled {
                seq,
                source_permanent: Some(attacker),
                reason: "security strike exceeds safety cap".to_string(),
            });
            return Self::MAX_SECURITY_CHECKS;
        }
        clamped as u8
    }

    /// Belt-and-braces upper bound on the `<Security A.>` recompute. No
    /// printed effect today produces anywhere near this much — checks
    /// rarely exceed 2 — but the loop is bound to a `u8` and we want a
    /// deterministic break if a future modifier interaction ever produces
    /// a runaway value.
    const MAX_SECURITY_CHECKS: u8 = 16;

    /// Pop the top security card from `defender`, snapshot its face-up
    /// state on the defender, and install `Game::security_resolution` for
    /// the new check. Returns `false` iff the defender's security stack
    /// was empty — the caller is responsible for `declare_winner` + the
    /// `GameWon` / `Invalid` distinction.
    ///
    /// `checks_performed` is the cumulative iteration counter prior to
    /// this pop — the post-pop counter is `checks_performed + 1`. Stored
    /// on the installed state so pause-and-resume survives across
    /// `drive_security_resolution` re-entries; the next iteration's
    /// `current_security_strike` comparison reads it back.
    fn pop_and_start_security_check(
        &mut self,
        attacker: PermanentHandle,
        defender: PlayerId,
        checks_performed: u8,
    ) -> bool {
        // Opaque-aware: if the about-to-pop security card is a
        // placeholder (opaque opponent's security whose identity hasn't
        // yet been revealed), materialize it via the reveal source
        // BEFORE popping. The top of security in Vec convention is the
        // last element; that's the index to check.
        if let Some(top_idx) = self.player(defender).security.len().checked_sub(1) {
            let needs = self.player(defender).security[top_idx].is_opaque_placeholder;
            if needs {
                if let Err(e) =
                    self.materialize_opaque_security_placeholder(defender, top_idx)
                {
                    eprintln!(
                        "[opaque-deck] security flip materialization error for player {} \
                         at idx {}: {}",
                        defender, top_idx, e
                    );
                    // Continue with the placeholder — it will pop with
                    // data_index=0 (probably the first card in the data
                    // store, semantically garbage but won't crash).
                    // The replay harness will surface a parity failure
                    // when the engine's behavior diverges from the
                    // recording, which is the right diagnostic.
                }
            }
        }
        let sec_card = match self.player_mut(defender).security.pop() {
            Some(c) => c,
            None => {
                let winner = attacker.player;
                self.declare_winner_with_reason(
                    winner,
                    crate::game::TerminalOutcomeReason::SecurityAttack,
                );
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

        // `GameEvent::SecurityReveal` — spec `engine-event-emission`
        // requires emission at reveal time, before any security-effect
        // resolution. Capture card_id from CardData (via the
        // pre-move `sec_card`) before the card moves into
        // `pending_security`. Emission is independent of the subsequent
        // `GameEvent::Trash` that fires when the dispose phase trashes
        // the revealed card.
        let revealed_card_id = sec_card.card_id(&self.card_data).to_string();
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::SecurityReveal {
            seq,
            defender,
            card_id: revealed_card_id,
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
            phase_enqueue_done: false,
            checks_performed,
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
                    let enqueue_done = state.phase_enqueue_done;
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
                    //
                    // Collect the revealed card's `[Security]` effects exactly
                    // once. If one parks on a `pending_selection`, this phase
                    // is re-entered on resume; `phase_enqueue_done` keeps the
                    // re-entry from re-collecting (and re-firing) the same
                    // `[Security]` clause — an infinite loop when the player
                    // declines an optional "you may" clause.
                    if !enqueue_done {
                        if let Some(st) = self.security_resolution.as_mut() {
                            st.phase_enqueue_done = true;
                        }
                        self.enqueue_triggered(
                            EffectTiming::SecuritySkill,
                            TriggerSource::SecurityRevealed {
                                defender,
                                card: card_handle,
                            },
                        );
                    }
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
                                // RULES_CONTEXT 14-2-1-3: "Same DP = both lose."
                                // The attacker is deleted when it has STRICTLY LESS
                                // OR EQUAL DP to the security Digimon. Per 14-2-3 the
                                // security Digimon itself isn't deleted by the
                                // battle (its trashing happens later via the
                                // check-disposal flow); only the attacker's
                                // deletion is the battle's responsibility here.
                                // Jamming preserves the attacker even on a losing
                                // or tied compare (RULES_CONTEXT 16-Jamming: "this
                                // Digimon can't be deleted in battles").
                                if attacker_dp <= sec_dp
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
                    let enqueue_done = state.phase_enqueue_done;
                    let attacker_opt = state.attacker;
                    let defender = state.defender;
                    let revealed_card = state.revealed_card;
                    let was_face_up = state.was_face_up;
                    if let Some(attacker) = attacker_opt {
                        // Collect observers once; a parked selection re-enters
                        // this phase, and `phase_enqueue_done` stops the
                        // re-entry from double-firing the observers.
                        if !enqueue_done {
                            if let Some(st) = self.security_resolution.as_mut() {
                                st.phase_enqueue_done = true;
                            }
                            let trigger = TriggerSource::OnSecurityCheck {
                                attacker,
                                defender,
                                revealed_card,
                                was_face_up,
                            };
                            self.enqueue_triggered(EffectTiming::OnSecurityCheck, trigger);
                            if was_face_up {
                                let trigger = TriggerSource::OnCheckFaceUpSecurity {
                                    attacker,
                                    defender,
                                    revealed_card,
                                };
                                self.enqueue_triggered(
                                    EffectTiming::OnCheckFaceUpSecurity,
                                    trigger,
                                );
                            }
                        }
                        self.drain_effect_queue();
                        if self.pending_selection.is_some() {
                            return None;
                        }
                    }
                    self.set_security_phase(SecurityPhase::WhenWouldLoseSecurity);
                }
                SecurityPhase::WhenWouldLoseSecurity => {
                    let Some(state) = self.security_resolution.as_ref() else {
                        break;
                    };
                    let subject = crate::replacement::ReplacementSubject::Card(
                        state.revealed_card,
                        crate::enums::Zone::Security,
                    );
                    let outcome = self.try_replace(
                        EffectTiming::WhenWouldLoseSecurity,
                        subject,
                        crate::replacement::ReplacementCause::SecurityCheck,
                        Some(crate::enums::Zone::Trash),
                    );
                    if self.pending_selection.is_some() {
                        return None;
                    }
                    self.commit_pending_security_loss_replacement(outcome);
                }
                SecurityPhase::OnLoseSecurityDrain => {
                    let Some(state) = self.security_resolution.as_ref() else {
                        break;
                    };
                    let defender = state.defender;
                    let card_handle = state.revealed_card;
                    let enqueue_done = state.phase_enqueue_done;
                    // Collect `OnLoseSecurity` effects once; a parked selection
                    // re-enters this phase, and `phase_enqueue_done` keeps the
                    // re-entry from re-firing them.
                    if !enqueue_done {
                        if let Some(st) = self.security_resolution.as_mut() {
                            st.phase_enqueue_done = true;
                        }
                        self.enqueue_triggered(
                            EffectTiming::OnLoseSecurity,
                            TriggerSource::SecurityRevealed {
                                defender,
                                card: card_handle,
                            },
                        );
                    }
                    self.drain_effect_queue();
                    if self.pending_selection.is_some() {
                        return None;
                    }
                    self.set_security_phase(SecurityPhase::Dispose);
                }
                SecurityPhase::Dispose => {
                    // Trash the revealed card unless an effect raised the
                    // `played` bit via `EffectContext::play_pending_security`.
                    // `security_resolution` stays alive across the observer
                    // drain so a selection installed by an observer can
                    // resume through `DisposeFinalize` (§2.5j residual).
                    let defender = state.defender;
                    let attacker_opt = state.attacker;
                    let revealed_card = state.revealed_card;
                    if let Some(pending) = self.pending_security.take() {
                        if !pending.played {
                            self.player_mut(defender).trash.push(pending.card);
                        }
                    }

                    // Security-removed observers fire after a security card
                    // leaves the defender's stack (trashed or played from
                    // security). Own-side observers scan the defender's
                    // battle area; opponent-side observers scan the attacker's.
                    if let Some(atk) = attacker_opt {
                        self.enqueue_triggered(
                            crate::enums::EffectTiming::OnOwnSecurityRemoved,
                            crate::selection::TriggerSource::SecurityRemoved {
                                affected_player: defender,
                                observer_player: defender,
                                source_player: atk.player,
                                card: revealed_card,
                                cause: crate::trigger_context::EventCause::SecurityRemoval,
                            },
                        );
                        self.drain_effect_queue();
                        if self.pending_selection.is_some() {
                            self.set_security_phase(SecurityPhase::DisposeFinalize);
                            return None;
                        }

                        self.enqueue_triggered(
                            crate::enums::EffectTiming::OnOpponentSecurityRemoved,
                            crate::selection::TriggerSource::SecurityRemoved {
                                affected_player: defender,
                                observer_player: atk.player,
                                source_player: atk.player,
                                card: revealed_card,
                                cause: crate::trigger_context::EventCause::SecurityRemoval,
                            },
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
                    //
                    // DCGO parity: the security-attack count is NOT a
                    // declaration-time snapshot. We recompute the active
                    // attacker's effective `<Security A.>` every iteration
                    // (see `Self::current_security_strike`) so that a
                    // post-effect digivolve / modifier grant / ChangeSAttack
                    // that lands during this card's drain extends (or
                    // shortens) the loop on the next iteration. Mirrors
                    // `Permanent.Strike` re-read in DCGO
                    // `CardController.cs:3956-3987`. See
                    // `fix-security-check-recompute-mid-attack`.
                    let state = self.security_resolution.take().expect("checked Some above");
                    let defender = state.defender;
                    let attacker_opt = state.attacker;
                    // `checks_performed` was the counter BEFORE this card
                    // was popped; we just resolved that card, so the new
                    // cumulative count is +1.
                    let checks_performed = state.checks_performed.saturating_add(1);
                    let outcome = state.outcome_so_far;

                    // Hard terminal: attacker was deleted mid-check.
                    if matches!(outcome, AttackResult::AttackerDeletedBySecurity) {
                        return Some(outcome);
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

                    // Recompute the live strike. Terminate when we've
                    // already popped enough cards for the current
                    // attacker; otherwise pop another.
                    let current_strike = self.current_security_strike(attacker);
                    if checks_performed >= current_strike {
                        return Some(AttackResult::SecurityCheckSurvived);
                    }
                    if !self.pop_and_start_security_check(attacker, defender, checks_performed) {
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

    pub(crate) fn commit_pending_security_loss_replacement(
        &mut self,
        outcome: crate::replacement::ReplacementOutcome,
    ) {
        use crate::replacement::ReplacementOutcome;

        match outcome {
            ReplacementOutcome::None => {
                self.set_security_phase(SecurityPhase::OnLoseSecurityDrain);
            }
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                self.restore_pending_security_to_stack();
                self.set_security_phase(SecurityPhase::DisposeFinalize);
            }
            ReplacementOutcome::Redirected(crate::enums::Zone::Hand) => {
                if let Some(pending) = self.pending_security.take() {
                    self.player_mut(pending.defender).hand.push(pending.card);
                }
                self.set_security_phase(SecurityPhase::DisposeFinalize);
            }
            ReplacementOutcome::Redirected(crate::enums::Zone::Deck) => {
                if let Some(pending) = self.pending_security.take() {
                    self.player_mut(pending.defender)
                        .deck
                        .insert(0, pending.card);
                }
                self.set_security_phase(SecurityPhase::DisposeFinalize);
            }
            ReplacementOutcome::Redirected(_) | ReplacementOutcome::Substituted(_) => {
                self.set_security_phase(SecurityPhase::OnLoseSecurityDrain);
            }
        }
    }

    fn restore_pending_security_to_stack(&mut self) {
        let Some(state) = self.security_resolution.as_ref() else {
            return;
        };
        let defender = state.defender;
        let was_face_up = state.was_face_up;
        let Some(pending) = self.pending_security.take() else {
            return;
        };
        let card_index = pending.card.card_index;
        self.player_mut(defender).security.push(pending.card);
        if was_face_up {
            self.player_mut(defender)
                .face_up_security
                .insert(card_index);
        }
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
    ///
    /// Clears `phase_enqueue_done` so the phase being entered gets a fresh
    /// enqueue budget — each `*Drain` phase collects its triggered effects
    /// exactly once, then re-entries on resume skip the re-collection.
    fn set_security_phase(&mut self, phase: SecurityPhase) {
        if let Some(st) = self.security_resolution.as_mut() {
            st.phase = phase;
            st.phase_enqueue_done = false;
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
        self.players
            .get(handle.player as usize)
            .map(|player| &player.battle_area)
            .and_then(|battle_area| battle_area.get(handle.index as usize))
            .map(|p| {
                // Must be a Digimon (Tamers, DigiEggs, Options aren't attack
                // targets). Phase 8 Task 3 reinforces this for Delayed /
                // Training Options: they live on battle_area but are not
                // attackable. Linked (Task 4) is attached sideways to its
                // host and doesn't occupy a standalone permanent slot.
                self.permanent_is_digimon_for_rules(handle)
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
                    && self.permanent_is_digimon_for_rules(handle)
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
        // G-DSL-OUTER-TAIL-NESTED-PARK: defer when inside select/outer-tail scope.
        self.maybe_drain_effect_queue();
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
        self.maybe_drain_effect_queue();
        if self.pending_selection.is_some() || !self.handle_still_attacking(handle) {
            return;
        }

        // Phase 9 Task 8 — OnAllyAttack fan-out. Observer timing firing on
        // every permanent in the attacker-controller's battle area EXCEPT
        // the attacker itself. We piggyback on the PlayerBattleArea scan
        // and filter the attacker's own slot from the queue after enqueue.
        let attacker_queue_start = self.effect_queue.len();
        let attacker_card = self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| perm.top_card().handle());
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnAllyAttack,
            attacker_card.map_or(
                crate::selection::TriggerSource::PlayerBattleArea(handle.player),
                |card| crate::selection::TriggerSource::PlayerBattleAreaAttack {
                    player: handle.player,
                    attacker: handle,
                    card,
                },
            ),
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
                attacker_card.map_or(
                    crate::selection::TriggerSource::PlayerBattleArea(opp),
                    |card| crate::selection::TriggerSource::PlayerBattleAreaAttack {
                        player: opp,
                        attacker: handle,
                        card,
                    },
                ),
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
            // DCGO convention, with both bodies leaving as one batch before
            // global OnAnyDeletion observers check survivor state.
            self.delete_permanents_batch(
                vec![defender, attacker],
                crate::replacement::ReplacementCause::Battle,
            );
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
    /// Phase 7: this entry point infers the `ReplacementCause` from live game
    /// state (security-resolution / pending-attack / effect_source_player) and
    /// delegates to `delete_permanent_with_cause`. Callers that already know
    /// the cause (e.g. `resolve_battle` → `Battle`) should invoke
    /// `delete_permanent_with_cause` directly.
    ///
    /// **Post-batched-refactor (2026-05-23):** both this entrypoint and
    /// `delete_permanent_with_cause` are shims over `delete_permanents_batch`
    /// — the unified deletion API that runs the DCGO-modeled batched flow
    /// (replacement window → snapshot → trash → OnDeletion → OnAnyDeletion).
    pub fn delete_permanent_with_effects(&mut self, handle: PermanentHandle) {
        let cause = self.infer_deletion_cause(handle);
        self.delete_permanent_with_cause(handle, cause);
    }

    /// Batched deletion entrypoint — DCGO `DestroyPermanentsClass.Destroy()`
    /// equivalent. Accepts a list of permanent handles and processes them as
    /// a single batched unit:
    ///
    /// 1. **Filter** — drop handles whose battle-area slot is empty.
    /// 2. **Per-handle replacement window** — fire `WhenWouldLeaveBattleArea`
    ///    then `WhenWouldBeDeleted` per handle. (Phase 3 batches these into
    ///    a single two-stage cut-in across the kill list.) Cancelled,
    ///    redirected, and substituted outcomes mutate the surviving list.
    /// 3. **Snapshot** — capture each surviving permanent's pre-removal
    ///    state (`DeletedObjectSnapshot` with `dp_just_before`,
    ///    `level_just_before`, etc.) while the carrier is still on field.
    /// 4. **Trash** — linked-card cascade, ACE overflow, `delete_permanent`,
    ///    modifier cleanup. After this step the carrier is gone from
    ///    `battle_area` and its top card is in trash.
    /// 5. **OnDeletion drain** — enqueue per-survivor OnDeletion triggers
    ///    carrying the snapshot in the trigger context. Handlers that park
    ///    selections (printed `<Save>`) unwind through `pending_selection`;
    ///    the resume hook (`Game::resume_pending_deletion`) continues the
    ///    drain via the active batch after each selection resolves.
    /// 6. **OnAnyDeletion / OnLeaveField** — global broadcast with each
    ///    snapshot. Drain.
    ///
    /// Callers:
    /// - Single-target callers (`delete_permanent_with_effects`,
    ///   `delete_permanent_with_cause`) pass a one-element kill list.
    /// - Battle resolution passes `[defender]` (winner) or
    ///   `[defender, attacker]` (mutual destruction).
    /// - DSL `DeleteBoundPermanents` passes the resolved binding list.
    ///
    /// Returns a `DeletionBatchOutcome` describing which handles trashed,
    /// were cancelled, or were substituted in. Most callers ignore this.
    ///
    /// See `openspec/changes/align-deletion-with-dcgo-model/design.md` D2
    /// for the rationale; `specs/permanent-deletion-semantics/spec.md` for
    /// the requirement contracts this implements.
    pub fn delete_permanents_batch(
        &mut self,
        handles: Vec<PermanentHandle>,
        cause: crate::replacement::ReplacementCause,
    ) -> crate::deletion_batch::DeletionBatchOutcome {
        use crate::deletion_batch::{DeletionBatch, DeletionBatchOutcome};

        // Stage: Filtering — drop handles whose battle_area slot is empty.
        let kill_list: Vec<PermanentHandle> = handles
            .into_iter()
            .filter(|h| {
                self.player(h.player)
                    .battle_area
                    .get(h.index as usize)
                    .is_some()
            })
            .collect();
        if kill_list.is_empty() {
            return DeletionBatchOutcome::default();
        }

        // Track that a batch is in flight. Save+restore the outer batch
        // so nested `delete_permanents_batch` calls inside an OnDeletion
        // handler don't clobber the outer batch's state.
        let prior_batch = self.active_deletion_batch.take();
        self.active_deletion_batch = Some(DeletionBatch::new(kill_list.clone(), cause));

        // Carry the cause across the OnDeletion drain via the existing
        // `current_deletion_cause` slot, matching the pre-batched panic-safe
        // save/restore at the old single-handle entrypoint.
        let prior_cause = self.current_deletion_cause;
        self.current_deletion_cause = Some(cause);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.run_deletion_batch_stages()
        }));

        self.current_deletion_cause = prior_cause;
        let outcome = match self.active_deletion_batch.take() {
            Some(batch) => DeletionBatchOutcome {
                completed: batch.completed,
                cancelled: batch.cancelled,
                substituted_in: batch.substituted_in,
            },
            None => DeletionBatchOutcome::default(),
        };
        self.active_deletion_batch = prior_batch;

        if let Err(payload) = result {
            std::panic::resume_unwind(payload);
        }
        outcome
    }

    /// Run the batched deletion stages against `self.active_deletion_batch`.
    /// Called by `delete_permanents_batch` inside its panic-safe scope.
    ///
    /// **Phase 2 implementation note.** The replacement window stages run
    /// per-handle here using the existing `try_replace` machinery. Phase 3
    /// will batch them into a single two-stage cut-in across the kill list.
    /// The snapshot + trash + OnDeletion stages already run as a batched
    /// unit so trash-before-drain semantics hold for single-target callers
    /// (the dominant case) and the per-handle replacement loop preserves
    /// today's substitute/redirect/cancel outcomes.
    fn run_deletion_batch_stages(&mut self) {
        use crate::deletion_batch::BatchStage;
        use crate::enums::{EffectTiming, Zone};

        // Stage 1: WhenWouldLeaveBattleArea (per-handle for Phase 2).
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("run_deletion_batch_stages called without active batch");
            batch.stage = BatchStage::Stage1ReplacementDrain;
        }
        if self.run_replacement_stage(EffectTiming::WhenWouldLeaveBattleArea, Zone::Trash) {
            return;
        }

        // Stage 2: WhenWouldBeDeleted.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist across stages");
            batch.stage = BatchStage::Stage2ReplacementDrain;
        }
        if self.run_replacement_stage(EffectTiming::WhenWouldBeDeleted, Zone::Trash) {
            return;
        }

        // Stage 3: Snapshotting. Capture each survivor's pre-removal state
        // while the carrier is still on field.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist into snapshot stage");
            batch.stage = BatchStage::Snapshotting;
        }
        self.capture_batch_snapshots();

        // DCGO-faithful enqueue-before-trash ordering. `enqueue_from_permanent`
        // reads the permanent's effects from the live `battle_area` slot,
        // which means OnDeletion must be enqueued while the carrier is
        // still on field. The drain then runs post-trash so handlers see
        // the carrier in trash. We use the deferred-drain scope to hold
        // the drain across the trash mutation.
        //
        // DCGO equivalent: `DestroyPermanentsClass.Destroy()` step 8 stacks
        // OnDestroyedAnyone via `autoProcessing.StackSkillInfos` BEFORE the
        // trash loop at step 10; the outer `TriggeredSkillProcess` drains
        // after step 10 returns.
        self.enter_deferred_drain();

        // Stage 4a: Enqueue OnDeletion for each survivor with its snapshot,
        // while the carrier is still on field (so its effects can be read).
        self.enqueue_batch_on_deletion();

        // Stage 4b: Trashing. Move carriers to trash; linked-card cascade;
        // modifier cleanup. Highest-index-first within each player so
        // removals don't shift later handles.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist into trash stage");
            batch.stage = BatchStage::Trashing;
        }
        self.trash_batch_survivors();

        // Stage 5: OnDeletion drain. Exit the deferred-drain scope to flush
        // the queued OnDeletion handlers. Handlers run post-trash and read
        // pre-removal state via the snapshot.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist into OnDeletion stage");
            batch.stage = BatchStage::OnDeletionDrain;
        }
        self.exit_deferred_drain_and_flush();

        // If an OnDeletion handler parked a selection, control unwinds
        // here with `pending_selection.is_some()`. The resume hook
        // (`resume_pending_deletion`) continues into the OnAnyDeletion
        // stage when the selection resolves.
        if self.pending_selection.is_some() {
            return;
        }

        // Stage 6: OnAnyDeletion / OnLeaveField global broadcasts.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist into OnAnyDeletion stage");
            batch.stage = BatchStage::OnAnyDeletionDrain;
        }
        self.drain_batch_on_any_deletion();
    }

    /// Post-replacement batched commit — skip stages 1+2 (the replacement
    /// window has already been offered and declined) and run snapshot +
    /// trash + OnDeletion + OnAnyDeletion against a single handle.
    ///
    /// Called from `replacement::commit_permanent_deletion_no_replace` —
    /// the deferred-decline path when an optional `WhenWouldBeDeleted`
    /// replacement is offered, the user PASSes, and the deletion needs to
    /// commit without re-running the replacement window.
    ///
    /// Re-uses the active-batch state machine for trash-before-drain
    /// semantics; restores the prior batch (if any) on exit.
    pub(crate) fn commit_post_replacement_single(
        &mut self,
        handle: PermanentHandle,
        cause: crate::replacement::ReplacementCause,
    ) {
        use crate::deletion_batch::{BatchStage, DeletionBatch};

        // Skip if the handle is already gone.
        if self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .is_none()
        {
            return;
        }

        let prior_batch = self.active_deletion_batch.take();
        let mut batch = DeletionBatch::new(vec![handle], cause);
        batch.stage = BatchStage::Snapshotting;
        self.active_deletion_batch = Some(batch);

        let prior_cause = self.current_deletion_cause;
        self.current_deletion_cause = Some(cause);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.capture_batch_snapshots();
            self.enter_deferred_drain();
            self.enqueue_batch_on_deletion();
            self.trash_batch_survivors();
            self.exit_deferred_drain_and_flush();
            if self.pending_selection.is_some() {
                return;
            }
            self.drain_batch_on_any_deletion();
        }));

        self.current_deletion_cause = prior_cause;
        self.active_deletion_batch = prior_batch;

        if let Err(payload) = result {
            std::panic::resume_unwind(payload);
        }
    }

    /// Run one replacement stage (`WhenWouldLeaveBattleArea` or
    /// `WhenWouldBeDeleted`) over the active batch's kill list. Returns
    /// `true` if a handler parked a selection — caller unwinds.
    ///
    /// Phase 2: per-handle dispatch using the existing `try_replace`
    /// machinery. Outcomes mutate the active batch's kill list / cancelled /
    /// substituted_in vectors.
    fn run_replacement_stage(
        &mut self,
        timing: crate::enums::EffectTiming,
        destination: crate::enums::Zone,
    ) -> bool {
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        let cause = self
            .active_deletion_batch
            .as_ref()
            .expect("active batch in replacement stage")
            .cause;

        // Process kill_list in a copy so we can mutate the batch's list as
        // we go (substitutes append, cancels mark).
        let mut i = 0;
        loop {
            let handle = {
                let batch = self
                    .active_deletion_batch
                    .as_ref()
                    .expect("active batch persists through replacement loop");
                if i >= batch.kill_list.len() {
                    return false;
                }
                batch.kill_list[i]
            };

            // Skip handles already trashed via OnDeletion side-effects
            // (defensive — shouldn't happen this early but guarded).
            if self
                .player(handle.player)
                .battle_area
                .get(handle.index as usize)
                .is_none()
            {
                if let Some(batch) = self.active_deletion_batch.as_mut() {
                    batch.cancelled.push(handle);
                    batch.kill_list.remove(i);
                }
                continue;
            }

            let subject = ReplacementSubject::Permanent(handle);
            let outcome = self.try_replace(timing, subject, cause, Some(destination));
            if self.pending_selection.is_some() {
                // Parked — caller unwinds and resumes via parked_replacement.
                return true;
            }

            match outcome {
                ReplacementOutcome::None => {
                    i += 1;
                }
                ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                    if let Some(batch) = self.active_deletion_batch.as_mut() {
                        batch.cancelled.push(handle);
                        batch.kill_list.remove(i);
                    }
                }
                ReplacementOutcome::Redirected(crate::enums::Zone::Deck) => {
                    self.return_to_deck(handle, crate::enums::StackPosition::Bottom);
                    if let Some(batch) = self.active_deletion_batch.as_mut() {
                        batch.cancelled.push(handle);
                        batch.kill_list.remove(i);
                    }
                }
                ReplacementOutcome::Redirected(crate::enums::Zone::Hand) => {
                    self.return_to_hand(handle);
                    if let Some(batch) = self.active_deletion_batch.as_mut() {
                        batch.cancelled.push(handle);
                        batch.kill_list.remove(i);
                    }
                }
                ReplacementOutcome::Redirected(other) => {
                    debug_assert!(
                        false,
                        "unexpected redirect destination for {:?}: {:?}",
                        timing, other
                    );
                    if let Some(batch) = self.active_deletion_batch.as_mut() {
                        batch.cancelled.push(handle);
                        batch.kill_list.remove(i);
                    }
                }
                ReplacementOutcome::Substituted(ReplacementSubject::Permanent(source_h)) => {
                    // Substitute: drop original from kill list, append
                    // substitute. Bound recursion via batch.depth.
                    if let Some(batch) = self.active_deletion_batch.as_mut() {
                        batch.cancelled.push(handle);
                        batch.kill_list.remove(i);
                        batch.depth = batch.depth.saturating_add(1);
                        if batch.depth >= 16 {
                            debug_assert!(
                                false,
                                "deletion batch substitute depth exceeded — pathological loop"
                            );
                            return false;
                        }
                        // Only add if substitute is on field and not already
                        // in the kill list.
                        let already_present = batch.kill_list.contains(&source_h)
                            || batch.substituted_in.contains(&source_h);
                        if !already_present {
                            batch.kill_list.push(source_h);
                            batch.substituted_in.push(source_h);
                        }
                    }
                    // Don't increment i — the slot we removed shifts later
                    // entries down by one. Next iter checks the new entry
                    // at index i.
                }
                ReplacementOutcome::Substituted(_) => {
                    debug_assert!(false, "non-Permanent substitute subject for {:?}", timing);
                    i += 1;
                }
            }
        }
    }

    /// Capture `DeletedObjectSnapshot` for each survivor in the active
    /// batch. Populates `batch.snapshots` and `batch.top_cards`.
    fn capture_batch_snapshots(&mut self) {
        let kill_list = {
            let batch = self
                .active_deletion_batch
                .as_ref()
                .expect("active batch in snapshot stage");
            batch.kill_list.clone()
        };
        let mut snapshots: Vec<crate::trigger_context::DeletedObjectSnapshot> =
            Vec::with_capacity(kill_list.len());
        let mut top_cards: Vec<Option<crate::card_source::CardHandle>> =
            Vec::with_capacity(kill_list.len());
        for handle in &kill_list {
            let snapshot_opt = self.build_snapshot_for_handle(*handle);
            let top = snapshot_opt.as_ref().map(|s| s.top_card);
            top_cards.push(top);
            if let Some(snap) = snapshot_opt {
                snapshots.push(snap);
            } else {
                // Carrier vanished between filter and snapshot — defensive.
                // Build a placeholder snapshot using just the cause so the
                // batch arrays stay aligned with kill_list indices.
                snapshots.push(crate::trigger_context::DeletedObjectSnapshot {
                    former_controller: handle.player,
                    top_card: crate::card_source::CardHandle(0),
                    card_kind: crate::enums::CardKind::Digimon,
                    traits: Vec::new(),
                    level: None,
                    dp: None,
                    cause: self
                        .observed_deletion_event_cause()
                        .unwrap_or(crate::trigger_context::EventCause::Rule),
                    dp_just_before: None,
                    level_just_before: None,
                    cost_just_before: None,
                    names_just_before: Vec::new(),
                    traits_just_before: Vec::new(),
                    source_count_just_before: 0,
                    digisources_just_before: Vec::new(),
                });
            }
        }
        let batch = self
            .active_deletion_batch
            .as_mut()
            .expect("active batch persists through snapshot capture");
        batch.snapshots = snapshots;
        batch.top_cards = top_cards;
    }

    /// Build a `DeletedObjectSnapshot` for a live battle-area handle.
    /// Returns `None` if the slot is empty.
    fn build_snapshot_for_handle(
        &self,
        handle: PermanentHandle,
    ) -> Option<crate::trigger_context::DeletedObjectSnapshot> {
        let perm = self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)?;
        let top_handle = perm.top_card().handle();
        let data = self.card_data_for_handle(top_handle)?;
        let mut digisources: Vec<crate::card_source::CardHandle> = Vec::new();
        for src in perm.card_sources.iter() {
            let h = src.handle();
            if h != top_handle {
                digisources.push(h);
            }
        }
        let source_count = digisources.len();
        let dp_now = self.effective_dp(handle);
        Some(crate::trigger_context::DeletedObjectSnapshot {
            former_controller: handle.player,
            top_card: top_handle,
            card_kind: data.card_kind,
            traits: data.traits.clone(),
            level: data.level,
            dp: dp_now,
            cause: self
                .observed_deletion_event_cause()
                .unwrap_or(crate::trigger_context::EventCause::Rule),
            dp_just_before: dp_now,
            level_just_before: data.level,
            cost_just_before: Some(data.play_cost),
            names_just_before: vec![data.card_name.clone()],
            traits_just_before: data.traits.clone(),
            source_count_just_before: source_count,
            digisources_just_before: digisources,
        })
    }

    /// Trash every survivor in the active batch. Processes within each
    /// player's `battle_area` in highest-index-first order so removals
    /// don't shift later handles' indices.
    fn trash_batch_survivors(&mut self) {
        // Group kill_list by player and sort high-to-low.
        let kill_list = {
            let batch = self
                .active_deletion_batch
                .as_ref()
                .expect("active batch in trash stage");
            batch.kill_list.clone()
        };
        // Sort: descending player, descending index. Stable iter so the
        // batch's snapshot/top_cards arrays don't have to be reordered —
        // we look up by handle, not by position.
        let mut sorted = kill_list.clone();
        sorted.sort_by(|a, b| b.player.cmp(&a.player).then(b.index.cmp(&a.index)));
        for handle in sorted {
            // Skip if already gone (defensive — substitute targets that
            // were already cancelled, etc.).
            if self
                .player(handle.player)
                .battle_area
                .get(handle.index as usize)
                .is_none()
            {
                continue;
            }
            self.trash_single_for_batch(handle);
        }
        // Record what completed (everything that's now gone from battle_area
        // among the kill_list).
        let batch = self
            .active_deletion_batch
            .as_mut()
            .expect("active batch persists through trash stage");
        let mut completed = Vec::new();
        for h in &batch.kill_list {
            // Use a heuristic: if the handle's slot is now empty, it
            // trashed. Index-shift across permanents in the same player
            // makes this approximate; the snapshot is the authoritative
            // record of what died.
            completed.push(*h);
        }
        batch.completed = completed;
    }

    /// Trash one permanent: linked-card cascade, ACE overflow, delete,
    /// modifier cleanup. No OnDeletion enqueue here — that's stage 5.
    fn trash_single_for_batch(&mut self, handle: PermanentHandle) {
        // Linked-card cascade — drain BEFORE removing the permanent so
        // OnLinkedCardTrashed observers see the host still in place.
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
                // Route through emission helper so each linked card surfaces
                // a `GameEvent::Trash` (capability `engine-event-emission`).
                for card in taken {
                    self.trash_card(handle.player, card);
                }
                true
            } else {
                false
            }
        };
        if had_linked {
            // Keep the already-stacked OnDeletion drain separate from this
            // immediate linked-card trash event; they are different trigger
            // windows and should not collapse into one TriggerOrder prompt.
            let mut deferred_queue = std::mem::take(&mut self.effect_queue);
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnLinkedCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(pid as PlayerId),
                );
            }
            self.drain_effect_queue();
            let mut immediate_queue = std::mem::take(&mut self.effect_queue);
            immediate_queue.append(&mut deferred_queue);
            self.effect_queue = immediate_queue;
        }

        // Now actually trash.
        if self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .is_some()
        {
            let sources = self.player(handle.player).battle_area[handle.index as usize]
                .card_sources
                .clone();
            if !sources.first().is_some_and(|source| source.is_token) {
                self.apply_ace_overflow_for_sources(&sources);
            }
            self.clear_permanent_full(handle);
            self.modifiers.expire_player_on_permanent_leave(handle);
            // Route stack-to-trash through emission helper so every
            // card surfaces a `GameEvent::Trash` (capability
            // `engine-event-emission`). Token-skip + empty-stack
            // semantics match `Player::delete_permanent`.
            self.trash_permanent_stack(handle.player, handle.index as usize);
            self.modifiers
                .shift_after_battle_area_remove(handle.player, handle.index);
        } else {
            self.clear_permanent_full(handle);
            self.modifiers.expire_player_on_permanent_leave(handle);
        }
        self.mark_until_condition_dirty();
    }

    /// Enqueue OnDeletion for each survivor in the active batch with its
    /// snapshot threaded into each entry's trigger context.
    ///
    /// Called BEFORE the trash stage so `enqueue_from_permanent` can read
    /// the carriers' effects from their live `battle_area` slots. The
    /// surrounding `enter_deferred_drain` scope holds the actual drain
    /// until `exit_deferred_drain_and_flush` runs after trash — matching
    /// DCGO `DestroyPermanentsClass.Destroy()`'s step-8 stack-before-trash
    /// ordering.
    fn enqueue_batch_on_deletion(&mut self) {
        let (kill_list, snapshots) = {
            let batch = self
                .active_deletion_batch
                .as_ref()
                .expect("active batch in OnDeletion-enqueue stage");
            (batch.kill_list.clone(), batch.snapshots.clone())
        };
        for (handle, snapshot) in kill_list.iter().zip(snapshots.iter()) {
            let queue_start = self.effect_queue.len();
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnDeletion,
                crate::selection::TriggerSource::Permanent(*handle),
            );
            // Thread the snapshot into the just-enqueued entries so OnDeletion
            // handlers can read `ctx.deleted_self_*()` accessors.
            for queued in self.effect_queue.iter_mut().skip(queue_start) {
                if queued.timing != crate::enums::EffectTiming::OnDeletion {
                    continue;
                }
                if let Some(trigger) = queued.trigger_context.as_mut() {
                    trigger.deleted_object = Some(snapshot.clone());
                    trigger.cause = Some(snapshot.cause);
                    trigger.affected_player = Some(snapshot.former_controller);
                    trigger.subject =
                        Some(crate::trigger_context::EventSubject::Permanent(*handle));
                }
            }
        }
    }

    /// Enqueue global OnAnyDeletion and OnLeaveField per survivor with
    /// snapshots, drain. Phase 5 (2026-05-23) retired the legacy
    /// `pending_post_deletion_replays` slot — Fortitude/Partition now play
    /// from trash inline during their OnDeletion handlers, so no
    /// post-finalize drain hook is needed here.
    fn drain_batch_on_any_deletion(&mut self) {
        let (kill_list, snapshots, top_cards) = {
            let batch = self
                .active_deletion_batch
                .as_ref()
                .expect("active batch in OnAnyDeletion stage");
            (
                batch.kill_list.clone(),
                batch.snapshots.clone(),
                batch.top_cards.clone(),
            )
        };

        // OnAnyDeletion + OnLeaveField per survivor.
        for ((handle, snapshot), top_card_opt) in
            kill_list.iter().zip(snapshots.iter()).zip(top_cards.iter())
        {
            if let Some(card) = top_card_opt {
                let queue_start = self.effect_queue.len();
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnAnyDeletion,
                    crate::selection::TriggerSource::EventObserved {
                        player: handle.player,
                        permanent: *handle,
                        card: *card,
                    },
                );
                for queued in self.effect_queue.iter_mut().skip(queue_start) {
                    if queued.timing != crate::enums::EffectTiming::OnAnyDeletion {
                        continue;
                    }
                    if let Some(trigger) = queued.trigger_context.as_mut() {
                        trigger.deleted_object = Some(snapshot.clone());
                        trigger.cause = Some(snapshot.cause);
                        trigger.affected_player = Some(snapshot.former_controller);
                        trigger.subject =
                            Some(crate::trigger_context::EventSubject::Permanent(*handle));
                    }
                }

                let queue_start_lf = self.effect_queue.len();
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnLeaveField,
                    crate::selection::TriggerSource::EventObserved {
                        player: handle.player,
                        permanent: *handle,
                        card: *card,
                    },
                );
                for queued in self.effect_queue.iter_mut().skip(queue_start_lf) {
                    if queued.timing != crate::enums::EffectTiming::OnLeaveField {
                        continue;
                    }
                    if let Some(trigger) = queued.trigger_context.as_mut() {
                        trigger.deleted_object = Some(snapshot.clone());
                        trigger.cause = Some(snapshot.cause);
                        trigger.affected_player = Some(snapshot.former_controller);
                        trigger.subject =
                            Some(crate::trigger_context::EventSubject::Permanent(*handle));
                    }
                }
            }
        }
        self.drain_effect_queue();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Cause-aware deletion entry point. **Post-batched-refactor (2026-05-23):**
    /// shimmed through `delete_permanents_batch(vec![handle], cause)`. The
    /// batched flow runs replacement window → snapshot → trash → OnDeletion →
    /// OnAnyDeletion as a unit, so a single-target deletion exhibits the
    /// DCGO-modeled trash-before-drain semantics: OnDeletion handlers fire
    /// AFTER the carrier's top card has moved to trash.
    ///
    /// Callers that need to know whether the deletion completed (vs. was
    /// cancelled/redirected/substituted) should call `delete_permanents_batch`
    /// directly and inspect the returned `DeletionBatchOutcome`.
    pub fn delete_permanent_with_cause(
        &mut self,
        handle: PermanentHandle,
        cause: crate::replacement::ReplacementCause,
    ) {
        let _ = self.delete_permanents_batch(vec![handle], cause);
    }

    /// Resume any deferred deletion work after a `pending_selection`
    /// resolves. Called by `effect_queue::resolve_generic_selection` after
    /// the parked selection's callback runs and the post-callback drain
    /// returns without re-parking.
    ///
    /// **Post-batched-refactor (2026-05-23):** when an OnDeletion handler
    /// parked a selection during the OnDeletion drain of
    /// `delete_permanents_batch`, `active_deletion_batch.is_some()` and
    /// the batch is mid-stage. Resume by continuing the OnDeletion drain
    /// until either the queue is empty or another handler parks. If
    /// drained cleanly, advance to the OnAnyDeletion stage.
    pub(crate) fn resume_pending_deletion(&mut self) {
        use crate::deletion_batch::BatchStage;

        // When an OnDeletion handler installs a `pending_selection` during
        // `exit_deferred_drain_and_flush`, the deferred-drain scope is
        // already closed (counter back to 0). `drain_effect_queue` is the
        // right primitive to continue the drain; further parks unwind
        // again until the queue is empty.
        let in_batch = self
            .active_deletion_batch
            .as_ref()
            .is_some_and(|b| matches!(b.stage, BatchStage::OnDeletionDrain));
        if in_batch {
            self.drain_effect_queue();
            if self.pending_selection.is_some() {
                // Another handler parked; the next `resume_pending_deletion`
                // call (after this selection resolves) continues the drain.
                return;
            }
            // OnDeletion drain settled — advance to OnAnyDeletion stage.
            if let Some(batch) = self.active_deletion_batch.as_mut() {
                batch.stage = BatchStage::OnAnyDeletionDrain;
            }
            self.drain_batch_on_any_deletion();
            // After OnAnyDeletion stage, clear the batch.
            self.active_deletion_batch = None;
        }
    }
}
