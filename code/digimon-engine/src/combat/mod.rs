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
//!    `RaidOpen` / `AllianceOpen` are pass-through: `<Raid>` and `<Alliance>`
//!    are trigger-type effects resolved in the declaration's trigger pool
//!    (`fire_on_attack`, §16-23-2 / §16-24-2). At each remaining open-window
//!    state the attempt helper (`try_enter_*`) either installs a
//!    `PendingSelection` and pauses, or auto-advances when no candidates
//!    exist.
//! 3. When a selection resolves, its callback mutates `PendingAttack.state`
//!    and re-enters `advance_pending_attack` to continue.
//! 4. `resolve_pending_battle` reads `effective_target` (which Counter or
//!    Block may have rewritten) and runs the DP comparison / security loop.
//! 5. `cleanup_attack` clears `is_attacking`, expires end-of-attack
//!    modifiers, and drops `pending_attack`.
//!
//! Counter and Blocker windows, and the `<Alliance>` / `<Raid>` trigger
//! bodies, all park through `PendingSelection` when they create
//! player-visible choices.
//!
//! A `<Vortex>` attack runs this same state machine: §11-1-5 "Once an attack
//! declaration has been made, all of the timings that follow will occur", and
//! nothing in §16-33 exempts it (Q3869, EX7-064: after the <Vortex> attack
//! declaration "the counter timing will occur"). `AttackInitiator::Vortex`
//! only relaxes the played-this-turn check (§16-33-1).

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

#[cfg(test)]
mod queue_handle_rekey_tests {
    use crate::card_source::CardHandle;
    use crate::debug_runner::DebugRunner;
    use crate::enums::{EffectSourceKind, EffectTiming};
    use crate::permanent::PermanentHandle;
    use crate::selection::QueuedEffect;
    use crate::trigger_context::TriggerContext;

    /// A helper contract test, not a legal gameplay fixture: non-field
    /// handle domains must be opaque to battlefield Vec compaction.
    #[test]
    fn security_compaction_rekeys_only_battlefield_handle_domains() {
        for player in 0..=1 {
            for index in [0, 1, 13, 14, 15, 199, 200, 201, 254, 255] {
                for owner in 0..=1 {
                    let handle = PermanentHandle {
                        player: owner,
                        index,
                    };
                    let context = TriggerContext {
                        captured_security_check: true,
                        target_permanent: Some(handle),
                        event_permanent: Some(handle),
                        event_host_permanent: Some(handle),
                        ..Default::default()
                    };
                    let mut game = DebugRunner::builder().build().game;
                    game.current_trigger_context = Some(context.clone());
                    game.effect_queue.push_back(QueuedEffect {
                        source_card: CardHandle(0),
                        source_permanent: Some(handle),
                        source_kind: EffectSourceKind::Digimon,
                        attribution_source_card: None,
                        attribution_source_kind: None,
                        bypass_once_per_turn: false,
                        controller: owner,
                        timing: EffectTiming::OnOwnSecurityRemoved,
                        trigger_context: Some(context),
                        effect_slot: 0,
                        is_optional: false,
                        is_turn_player: false,
                        card_id: String::new(),
                        allow_below_top_liveness: false,
                        dna_origin_context: None,
                        granted_effect_id: None,
                        keyword_effect: None,
                        trigger_batch: 0,
                        pending_processing: None,
                    });
                    game.shift_effect_queue_after_battle_area_remove(player, 0);
                    let expected_source = Some(PermanentHandle {
                        index: if owner == player && index > 0 && index < 14 {
                            index - 1
                        } else {
                            index
                        },
                        ..handle
                    });
                    let expected_context = if owner == player && index == 0 {
                        None
                    } else {
                        expected_source
                    };
                    let queued = &game.effect_queue[0];
                    assert_eq!(
                        queued.source_permanent, expected_source,
                        "source owner={owner} index={index} removed_owner={player}"
                    );
                    assert_eq!(
                        queued
                            .trigger_context
                            .as_ref()
                            .unwrap()
                            .security_check_source_invalidated,
                        owner == player && index == 0
                    );
                    for ctx in [
                        queued.trigger_context.as_ref().unwrap(),
                        game.current_trigger_context.as_ref().unwrap(),
                    ] {
                        assert_eq!(
                            (
                                ctx.target_permanent,
                                ctx.event_permanent,
                                ctx.event_host_permanent
                            ),
                            (expected_context, expected_context, expected_context),
                            "captured/current owner={owner} index={index} removed_owner={player}"
                        );
                    }
                }
            }
        }
    }
}

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
    pub ignore_summoning_sickness: bool,
    pub target_constraint: TargetConstraint,
    pub allow_cancel: bool,
    pub cost_upgrade: Option<AttackCostUpgrade>,
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

/// What a Digimon-vs-Digimon battle compares to decide its winner.
/// `Dp` is the standard rule; `DigivolutionCards` is the battle-scoped
/// "compare the number of digivolution cards instead of DP" override
/// (EX13-076; G-ENGINE-BATTLE-COMPARE-SOURCE-COUNT). `<Iceclad>` on either
/// combatant forces the digivolution-card comparison regardless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BattleComparison {
    #[default]
    Dp,
    DigivolutionCards,
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
    /// §11-5-1-4 / §11-5-1-4-1 (Ver. 4.3): the attack is not successful --
    /// the attack target was removed before the battle (§11-2-6), or the
    /// attacking Digimon left the battle area after its declaration
    /// (§11-1-5). No battle and no security check; the end of attack timing
    /// still occurs.
    Unsuccessful,
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

/// One comparison and its actual deletion outcomes. Entries remain while
/// replacement or deletion-effect choices are parked; no outcome is inferred
/// from whether a former battlefield slot is occupied.
#[derive(Debug, Clone)]
pub(crate) struct PendingBattleResolution {
    attacker: crate::trigger_context::BattleCombatantSnapshot,
    defender: crate::trigger_context::BattleCombatantSnapshot,
    outcome: AttackResult,
    attacker_deleted: bool,
    defender_deleted: bool,
    winner_identity: Option<(PlayerId, u64)>,
    /// The winner's last-known information at the comparison (§15-8-3-8:
    /// the battle is the state the "wins a battle" trigger references),
    /// read by the `event_winner_*` qualifiers once the winner has left.
    winner_snapshot: Option<std::sync::Arc<crate::trigger_context::DeletedObjectSnapshot>>,
    is_attack: bool,
    in_call: bool,
    /// The effects triggered by this battle (`EffectTiming::BattleResult`:
    /// "wins a battle" / "deletes in battle" / <Piercing>) have been enqueued
    /// -- into the battle deletion's trigger pool, or, when no loser was
    /// deleted, by `resume_pending_battle_resolutions` (§14-2-4).
    result_enqueued: bool,
}

// Deletion lifecycle + DP split out of the attack state machine (rule 25).
mod deletion;
mod dp;

impl Game {
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
        // 11-2-5: "An attack declaration can't be made using a Digimon that
        // can't suspend." (DCGO `Permanent.CanAttackTargetDigimon` →
        // `!CanSuspend`, Permanent.cs:2230.)
        if self.modifiers.has(handle, ModifierType::CannotSuspend) {
            return false;
        }
        // NOTE: a permanent-scoped `CannotAttack` modifier is intentionally NOT
        // consulted here. CannotAttack is modeled as a `WhenWouldAttack` cancel
        // replacement (`passive_modifier_to_would`, replacement.rs) so a forced
        // declaration is gracefully *Cancelled*, not rejected `Invalid`. The
        // action mask suppresses the offer separately (`can_basic_attack` +
        // the end-of-turn / granted-attack gates) so a deterministic policy is
        // never handed the guaranteed-cancel no-op.
        // Summoning sickness: can't attack on the turn it was played unless
        // Rush is present (native printed OR modifier-granted) or this is a
        // Vortex end-of-turn attack (§2.1b parity fix).
        let is_fresh = perm.turn_played == self.turn_count;
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
        self.can_attack_without_suspending_inner(handle, vortex, false)
    }

    pub(crate) fn can_attack_ignoring_summoning_sickness(&self, handle: PermanentHandle) -> bool {
        self.can_attack_inner(handle, false, true)
    }

    pub(crate) fn can_attack_without_suspending_ignoring_summoning_sickness(
        &self,
        handle: PermanentHandle,
    ) -> bool {
        self.can_attack_without_suspending_inner(handle, false, true)
    }

    fn can_attack_inner(
        &self,
        handle: PermanentHandle,
        vortex: bool,
        ignore_summoning_sickness: bool,
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
        if perm.is_suspended {
            return false;
        }
        // 11-2-5 — see `can_attack`.
        if self.modifiers.has(handle, ModifierType::CannotSuspend) {
            return false;
        }
        // CannotAttack is a `WhenWouldAttack` cancel replacement, not a
        // can_attack rejection — see `can_attack`.
        let is_fresh = perm.turn_played == self.turn_count;
        if is_fresh
            && !ignore_summoning_sickness
            && !vortex
            && !self.has_keyword(handle, Keyword::Rush)
        {
            return false;
        }
        true
    }

    fn can_attack_without_suspending_inner(
        &self,
        handle: PermanentHandle,
        vortex: bool,
        ignore_summoning_sickness: bool,
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
        // Official Q3185 (BT19-101) permits a granted attack without
        // suspending even when the attacker cannot suspend. DCGO likewise
        // skips CanSuspend when withoutTap is true. Ordinary attacks retain
        // the CannotSuspend gate in can_attack_inner.
        // CannotAttack is a `WhenWouldAttack` cancel replacement, not a
        // can_attack rejection — see `can_attack`.
        // "Without suspending" bypasses only the suspend cost/unsuspended
        // requirement. Summoning sickness still requires Rush or Vortex.
        let is_fresh = perm.turn_played == self.turn_count;
        if is_fresh
            && !ignore_summoning_sickness
            && !vortex
            && !self.has_keyword(handle, Keyword::Rush)
        {
            return false;
        }
        true
    }

    /// Attack another Digimon on the opponent's field.
    ///
    /// Returns the terminal battle outcome — **unless** a decision parks the
    /// attack on a `PendingSelection` (an interrupt such as Block or
    /// Counter, an effect's pick, or a decision in the End of Attack pool,
    /// §11-6-2, which keeps the attack in `AttackState::EndOfAttackDrain`),
    /// in which case `AttackResult::InProgress` is returned and the caller
    /// continues resolving via `Game::resolve_selection`. The attack resumes
    /// automatically in the post-selection tail; once the chain clears,
    /// `pending_attack` is `None` and the result shows in the game state
    /// (no result value is stored).
    ///
    /// `vortex` — see [`Game::can_attack`]. It only exempts the
    /// played-this-turn check (§16-33-1); a Vortex attack still runs the
    /// counter and block timings (§11-1-5; Q3869).
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
    /// or immediately perform security checks. If the active attacking Digimon
    /// deletes its opponent here, Piercing may become pending for that attack.
    pub fn battle_digimon(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        self.battle_digimon_with(attacker, defender, BattleComparison::Dp)
    }

    /// [`Self::battle_digimon`] with an explicit battle comparison.
    ///
    /// `BattleComparison::DigivolutionCards` makes THIS battle compare the
    /// two Digimon's digivolution-card counts instead of DP ("Compare the
    /// number of digivolution cards instead of DP in this battle." — EX13-076
    /// Imperialdramon: Paladin Mode; G-ENGINE-BATTLE-COMPARE-SOURCE-COUNT).
    /// It is the battle-scoped twin of `<Iceclad>` (16-35): DCGO routes both
    /// through the same `IBattle.CompareStats` branch
    /// (`HasIceclad || CompareDigivolutionCards`). Everything else is a
    /// standard battle — ties delete both, battle deletions run the normal
    /// replacement windows (<Evade>, would-leave), and `EndOfBattle` /
    /// win-battle observers fire.
    pub fn battle_digimon_with(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
        comparison: BattleComparison,
    ) -> AttackResult {
        if attacker.player == defender.player {
            return AttackResult::Invalid;
        }
        if !self.handle_valid(attacker) || !self.handle_valid(defender) {
            return AttackResult::Invalid;
        }

        self.resolve_battle_with(attacker, defender, comparison)
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
            ignore_summoning_sickness: false,
            target_constraint: TargetConstraint::Forced(target),
            allow_cancel: false,
            cost_upgrade: None,
        })
    }

    /// Declare an `<Overclock>` attack. The sacrifice must already have been
    /// paid by the caller ([`Game::install_overclock_selection`] drives this via a
    /// `PendingSelection`). Identical to [`Game::begin_attack`] except:
    ///
    /// - The attacker is **not** suspended on declaration (§4.6c-residual
    ///   parity — matches Python `resolve_attack(..., without_suspend=True)`).
    /// - Interrupts (Alliance / Counter / Block) still fire normally, as
    ///   for every attack (§11-1-5).
    pub fn begin_attack_overclock(
        &mut self,
        attacker: PermanentHandle,
        target: AttackTarget,
    ) -> AttackResult {
        self.begin_attack_open(AttackOpen {
            attacker,
            initiator: AttackInitiator::Overclock,
            suspend_attacker: false,
            ignore_summoning_sickness: false,
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
            ignore_summoning_sickness,
            target_constraint,
            allow_cancel: _,
            cost_upgrade,
        } = open;
        // A finished game declares no attack.
        if self.game_over {
            return AttackResult::Invalid;
        }
        // §18-3: a player-declared attack is a decision (an effect-forced
        // attack is not); the loop watch starts over (see
        // `drain_effect_queue`).
        if matches!(
            initiator,
            AttackInitiator::NaturalMainPhase | AttackInitiator::Vortex | AttackInitiator::Overclock
        ) {
            self.reset_loop_watch();
        }
        let TargetConstraint::Forced(target) = target_constraint else {
            return AttackResult::Invalid;
        };
        // §11-2-4 (Ver. 4.3) "A new attack declaration can't be made during
        // an attack." An attack is in progress from its declaration until it
        // has ended -- including while its End of Attack timing's pool is
        // resolving (§11-6-2, `AttackState::EndOfAttackDrain`; G-ENGINE-ATTACK-ENDS-BEFORE-END-OF-ATTACK-EFFECTS-RESOLVE).
        // A new declaration would overwrite its `pending_attack`, so its end
        // (is_attacking clear, end-of-attack expiry, End of Attack timing)
        // would never run. Refuse it; an end-of-turn attack keyword's offer or
        // a paid `<Overclock>` attack waits for the attack to end instead
        // (`deferred_end_of_turn_attack_offers`, `parked_overclock_attacks`).
        if self.pending_attack.is_some() {
            return AttackResult::Invalid;
        }
        let vortex = matches!(initiator, AttackInitiator::Vortex);
        let skips_suspend_cost = !suspend_attacker;

        let attacker_can_attack = if ignore_summoning_sickness && skips_suspend_cost {
            self.can_attack_without_suspending_ignoring_summoning_sickness(attacker)
        } else if ignore_summoning_sickness {
            self.can_attack_ignoring_summoning_sickness(attacker)
        } else if skips_suspend_cost {
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
            effective_target_removed: false,
            is_blocked: false,
            blocker: None,
            is_vortex: vortex,
            is_overclock: skips_suspend_cost,
            declaration_committed: false,
            cancelled: false,
            battle_occurred: false,
            battle_defender_deleted: false,
            effect_battle_piercing_pending: false,
            security_check_started: false,
            return_phase,
            state: AttackState::Declared,
            counter_depth: 0,
        });
        // A new attack: the previous attack's stack identity no longer names
        // its attacker (`fire_on_attack` captures this one's). §16-39-3:
        // `<Progress>` applies from the declaration.
        self.attack_attacker_identity = None;
        self.attack_attacker_card = None;
        // §11-2-6: the declared target is tracked by its stack identity, so
        // a Digimon that compacts into its slot after it leaves is never read
        // as the target.
        self.capture_attack_target_identity();
        self.refresh_progress_shield();

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
        let (attacker_card_id, attacker_card_name) = self
            .player(attacker.player)
            .battle_area
            .get(attacker.index as usize)
            .map(|p| {
                let top = p.top_card();
                (
                    top.card_id(&self.card_data).to_string(),
                    top.card_name(&self.card_data).to_string(),
                )
            })
            .unwrap_or_default();
        let (target_card_id, target_card_name) = match target {
            AttackTarget::Digimon(d) => self
                .player(d.player)
                .battle_area
                .get(d.index as usize)
                .map(|p| {
                    let top = p.top_card();
                    (
                        Some(top.card_id(&self.card_data).to_string()),
                        Some(top.card_name(&self.card_data).to_string()),
                    )
                })
                .unwrap_or((None, None)),
            AttackTarget::Player(_) => (None, None),
        };
        let attacker_dp = self.effective_dp(attacker);
        let target_dp = match target {
            AttackTarget::Digimon(d) => self.effective_dp(d),
            AttackTarget::Player(_) => None,
        };
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Attack {
            seq,
            player: attacker.player,
            attacker_field_index: attacker.index,
            target_field_index: attack_target_field_index,
            target_player: attack_target_player,
            attacker_card_id,
            attacker_card_name,
            target_card_id,
            target_card_name,
            attacker_dp,
            target_dp,
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

        // Every attack, a <Vortex> attack included, runs the counter and
        // block timings (§11-1-5, §11-4-1; Q3869). `vortex` only exempted
        // the played-this-turn check above (§16-33-1).
        self.transition_attack_state(AttackState::RaidOpen);

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
        // §1-2-2: once the game has ended -- a drain inside this attack
        // (e.g. its [When Attacking] effects) ended it in the §18-3-2 draw or
        // by a §1-2-6 win/lose effect, or a security check won it -- no later
        // attack timing, battle or security check runs and no prompt is
        // installed (it could not be answered: `SelectionError::GameOver`).
        // `GameWon` is the engine's "game over" result, as in the
        // security-check guard; the terminal outcome itself is `Game::winner`
        // / `terminal_outcome_reason`, and every terminal outcome is final
        // (`finish_game` keeps the first one).
        if self.game_over {
            return AttackResult::GameWon;
        }
        // A declaration made while resolving an effect is only declared here.
        // Finish that effect's remaining steps before attack interrupts/battle.
        if self.draining_deferred > 0 {
            return AttackResult::InProgress;
        }
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
            let security_check_started = pa.security_check_started;

            // §11-6-2: the End of Attack timing's pool parked on a decision
            // and has now resolved (the loop-top guard yields while anything
            // is pending): only now does the attack end.
            if let AttackState::EndOfAttackDrain(outcome) = state {
                return self.finish_attack_after_end_of_attack_timing(outcome);
            }

            // Phase 9: a `WhenWouldAttack` / `WhenWouldBeAttackTarget`
            // replacement (or a `ctx.cancel_attack()` from a later phase)
            // may have flagged the attack as cancelled while an optional
            // replacement selection was resolving. Short-circuit straight
            // to cleanup — EndOfAttack still fires for symmetry; EndOfBattle
            // does NOT (no DP comparison ran).
            if cancelled && state != AttackState::Cleanup {
                return self.cleanup_attack(AttackResult::Cancelled);
            }

            if state != AttackState::Cleanup
                && !(state == AttackState::Battle && self.pending_attack_battle_result.is_some())
            {
                if declaration_committed {
                    // Attacker liveness check: OnAttack / interrupt effects
                    // may have deleted or moved it. Handles are slot-based, so
                    // require the slot to still be the in-flight attacker
                    // rather than merely any valid Digimon.
                    if !self.handle_still_attacking(attacker) {
                        match state {
                            // §11-1-5 (Ver. 4.3): "Once an attack declaration
                            // has been made, all of the timings that follow
                            // will occur. (Example: Even if an attacking
                            // Digimon leaves the battle area, the non-turn
                            // player's counter timing and block timing will
                            // still occur.)" The counter timing runs on the
                            // stored attack context (`try_enter_counter`);
                            // the block timing offers no block (§12-1-6,
                            // `try_enter_block`). DCGO `AttackProcess.cs`
                            // `CounterTiming()` checks only `IsEndAttack`.
                            AttackState::Declared
                            | AttackState::RaidOpen
                            | AttackState::AllianceOpen
                            | AttackState::CounterOpen
                            | AttackState::BlockOpen => {}
                            // After the block timing with no attacking
                            // Digimon: the attack is not successful
                            // (§11-5-1-4-1), no
                            // other action is taken and the end of attack
                            // timing occurs. A battle already resolved or a
                            // security check in flight keeps its own path.
                            AttackState::PostBlock | AttackState::Battle
                                if self.security_resolution.is_none()
                                    && !security_check_started =>
                            {
                                return self.end_attack_unsuccessful();
                            }
                            _ => return self.cleanup_attack(AttackResult::Invalid),
                        }
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
                // Pass-through: `<Raid>` and `<Alliance>` are trigger-type
                // effects resolved in the attack declaration's trigger pool
                // (`fire_on_attack`; §16-23-2, §16-24-2), not windows. The
                // states stay in the enum for serialization / tensor
                // compatibility.
                AttackState::RaidOpen => {
                    self.transition_attack_state(AttackState::AllianceOpen);
                }
                AttackState::AllianceOpen => {
                    self.transition_attack_state(AttackState::CounterOpen);
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
                    // §11-2-6 (Ver. 4.3): a removed attack target remains the
                    // target and the attack fails; nothing retargets it.
                    // `<Raid>` is a trigger-type effect that triggers once,
                    // when the Digimon attacks, and is optional (§16-23-2 /
                    // §16-23-3) -- it is resolved in the declaration's
                    // trigger pool (`install_raid_switch_selection`), never
                    // here. DCGO `AttackProcess.cs` goes from the block
                    // timing straight to `DetermineAttackOutcome`.
                    self.transition_attack_state(AttackState::Battle);
                }
                AttackState::Battle => {
                    let outcome = self.resolve_pending_battle();
                    if outcome == AttackResult::Unsuccessful {
                        // §11-2-6 / §11-5-1-4-1: the target was removed; the
                        // same not-successful end as an attacker that left.
                        return self.end_attack_unsuccessful();
                    }
                    if matches!(outcome, AttackResult::InProgress) {
                        // Paused mid-security (a SecuritySkill effect
                        // installed a pending_selection). Keep
                        // `pending_attack` alive in `Battle` so
                        // `advance_security_resolution` can finalize combat
                        // via `cleanup_attack` once the selection resolves
                        // (§2.5j).
                        return AttackResult::InProgress;
                    }

                    // `<Piercing>` triggered in the battle's pool with the
                    // loser's `[On Deletion]` (§16-7-4; `EffectTiming::
                    // BattleResult`, keyword body in `keyword_effects.rs`);
                    // its activation left the check pending
                    // (`effect_battle_piercing_pending`). It applies to both
                    // attack shapes -- declared on a Digimon, or declared on
                    // the player and blocked (glossary: "This effect also
                    // works if an attack is blocked"; §16-7-2) -- and to an
                    // effect battle the attacking Digimon wins during the
                    // attack. `cleanup_attack` processes it immediately
                    // before the End of Attack timing, after every effect
                    // triggered by the battle has resolved (the
                    // post-selection resume re-enters through the
                    // `PostBattle` arm below); `fire_piercing_or_finish`
                    // re-checks that the attacker is still attacking
                    // (§16-7-6), and the check is performed even if
                    // <Piercing> was lost after it activated (official Q&A
                    // Q3931). The security pipeline may park on a
                    // `PendingSelection`; `advance_security_resolution` then
                    // finalizes via `cleanup_attack` when the chain clears.
                    return self.cleanup_attack(outcome);
                }
                AttackState::PostBattle => {
                    // Two ways to land here:
                    //
                    // 1. The security pipeline (Piercing follow-up) is in
                    //    flight — `advance_security_resolution` owns
                    //    completion and finalizes via `cleanup_attack`.
                    //    Yield.
                    // 2. `cleanup_attack` deferred the check an activated
                    //    `<Piercing>` left pending, because effects were
                    //    still resolving (§16-7-4). Those have now cleared
                    //    (the loop-top guard yields while a selection or
                    //    queued effect is pending), so fire the check now.
                    if self.security_resolution.is_some() {
                        return AttackResult::InProgress;
                    }
                    return self.fire_piercing_or_finish(attacker);
                }
                AttackState::Cleanup => {
                    // Shouldn't normally reach here — cleanup_attack is
                    // called on transition into Cleanup. Defensive exit.
                    return self.cleanup_attack(AttackResult::Invalid);
                }
                AttackState::EndOfAttackDrain(outcome) => {
                    // Unreachable: handled by the early arm above, which must
                    // precede the cancel / liveness checks (an attacker
                    // deleted in the pool, e.g. by <Execute>, must not divert
                    // to `cleanup_attack` and run the End of Attack again).
                    // Kept only for exhaustiveness; same single finish.
                    return self.finish_attack_after_end_of_attack_timing(outcome);
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
        // A [Counter] body may end the attack it is countering ("Then, you
        // may end this attack" — BT25-103). DCGO `AttackProcess.CounterTiming`
        // re-checks `IsEndAttack` right after the [Counter] effects resolve, so
        // the cancel is honoured while the attack sits in `CounterOpen` (its
        // `counter_depth` was bumped to 1 when the window opened). A nested
        // attack launched by a Counter body is a fresh `pending_attack`.
        if state == AttackState::CounterOpen && counter_depth <= MAX_COUNTER_DEPTH {
            return true;
        }
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
        if self.pending_attack.is_none() {
            return;
        }
        // The attacking Digimon by identity: once it has left the battle area
        // its slot may hold another Digimon, and there is no attacking
        // Digimon whose target could switch (DCGO `AttackProcess.cs`
        // `SwitchDefender` yields when `AttackingPermanent.TopCard == null`).
        let Some(attacker) = self.live_attack_attacker() else {
            return;
        };
        let target_removed = self.attack_target_removed();
        let old_target = self.pending_attack.as_ref().map_or(new_target, |pa| pa.effective_target);
        // Firing-guard: a redirect to the current effective target is a
        // no-op — don't re-fire OnAttackTargetChange. A removed target
        // (§11-2-6) is never the current one.
        if old_target == new_target && !target_removed {
            return;
        }

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

        let Some(pa) = self.pending_attack.as_mut() else {
            return;
        };
        // A block (§12-1-7), redirect or `<Raid>` switch installs a NEW
        // attack target: the removed one (§11-2-6) no longer matters.
        pa.effective_target = new_target;
        pa.effective_target_removed = false;
        self.capture_attack_target_identity();

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
        // Deferred while an effect is resolving (e.g. the `<Raid>` body's
        // switch pick, resolved inside the declaration's trigger pool): the
        // derived OnAttackTargetChange observers wait for that effect to
        // finish (§15-8-3-2) and then resolve before the effects still
        // pending from the pool (§15-4-5-2), instead of draining those
        // pending siblings mid-body.
        self.maybe_drain_effect_queue();
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

    /// Resolve the `<Alliance>` ally pick (resume frame
    /// `ResumeFrame::AllianceSelection`). PASS = the optional processing
    /// condition is not executed (§16-24-3); nothing else happens. The attack
    /// state machine is resumed by the selection resolver once the trigger
    /// pool has drained.
    pub(crate) fn run_alliance_selection_step(
        &mut self,
        attacker: PermanentHandle,
        action_id: u16,
        is_pass: bool,
    ) {
        if is_pass {
            return;
        }
        use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
        let ally = PermanentHandle {
            player: attacker.player,
            index: (action_id.saturating_sub(ATTACK_START) % TARGETS_PER_ATTACKER) as u8,
        };
        self.apply_alliance_ally(attacker, ally);
    }

    /// `<Alliance>` after its ally is chosen (§16-24-1), in DCGO
    /// `AllianceProcess` order (judge-quiz Q24):
    ///   1. Suspend the ally THROUGH the canonical chokepoint — a keyword
    ///      effect's "by suspending" condition is effect-initiated, so OnSuspend
    ///      observers ("when an EFFECT suspends…", EX6-004) see the event. The
    ///      deferred-drain scope parks those observers (and the ≤0-DP rules
    ///      check) until the grant below completes.
    ///   2. Read the ally's DP AFTER suspension (§16-24-4: the value upon the
    ///      suspension) — a suspended-Digimon debuff (BT16-101's −4000)
    ///      applies first and `effective_dp` floors at 0 (judge Q24).
    ///   3. Grant the attacker +DP and <Security A. +1> until end of attack
    ///      (the <Security A. +1> lands even when the DP added is 0).
    ///      §16-24-5: both stay if the ally later leaves.
    /// Flushing afterwards lets the rules check delete a ≤0-DP ally BEFORE its
    /// parked OnSuspend observers activate (judge Q24).
    pub(crate) fn apply_alliance_ally(&mut self, attacker: PermanentHandle, ally: PermanentHandle) {
        let attacker_player = attacker.player;
        self.enter_deferred_drain();
        self.suspend_with_cause(ally, true);
        let ally_dp = self.effective_dp(ally).unwrap_or(0);
        self.modifiers.add(
            attacker,
            ModifierEntry::simple(
                ModifierType::ChangeDp,
                ally_dp,
                Expiry::EndOfAttack,
                attacker_player,
            ),
        );
        self.modifiers.add(
            attacker,
            ModifierEntry::simple(
                ModifierType::SecurityAttackChange,
                1,
                Expiry::EndOfAttack,
                attacker_player,
            ),
        );
        self.exit_deferred_drain_and_flush();
    }

    /// Field indices of the `<Alliance>` ally candidates for `attacker`: empty
    /// unless `attacker` is the Digimon currently attacking (the grant is "for
    /// the attack"; DCGO `CanTriggerOnPermanentAttack`); otherwise any OTHER
    /// unsuspended Digimon of its controller that can be suspended (DCGO
    /// `CanActivateAlliance` / `CanSelectPermanentCondition`: own battle-area
    /// Digimon ≠ carrier + `CanActivateSuspendCostEffect`). The ally itself
    /// does NOT need the keyword (judge-quiz Q24). A `CannotSuspend` ally
    /// can't pay the suspension (prohibition precedence 15-1-3).
    pub(crate) fn alliance_ally_candidates(&self, attacker: PermanentHandle) -> Vec<u8> {
        // The attacker by stack identity (as `raid_switch_options`), never a
        // Digimon that compacted into the attacker's slot.
        if self.live_attack_attacker() != Some(attacker) {
            return Vec::new();
        }
        let Some(player) = self.players.get(attacker.player as usize) else {
            return Vec::new();
        };
        (0..player.battle_area.len() as u8)
            .filter(|&i| i != attacker.index)
            .filter(|&i| {
                let h = PermanentHandle {
                    player: attacker.player,
                    index: i,
                };
                !player.battle_area[i as usize].is_suspended
                    && self.permanent_is_digimon_for_rules(h)
                    && !self.modifiers.has(h, ModifierType::CannotSuspend)
            })
            .collect()
    }

    /// `<Alliance>`'s trigger body (`keyword_to_auto_effect`): park the
    /// optional ally pick for the attacker's controller. PASS = the optional
    /// processing condition is not executed (§16-24-3). No candidates → no
    /// prompt (the trigger's condition normally keeps it out of the pool).
    pub(crate) fn install_alliance_ally_selection(&mut self, attacker: PermanentHandle) {
        use crate::action::space::encode_attack;

        let candidates = self.alliance_ally_candidates(attacker);
        if candidates.is_empty() {
            return;
        }
        let Some(source_card) = self
            .player(attacker.player)
            .battle_area
            .get(attacker.index as usize)
            .map(|p| p.top_card().handle())
        else {
            return;
        };
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::AllianceTiming;
        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::OwnField,
            selecting_player: attacker.player,
            previous_phase,
            valid_action_ids: candidates
                .iter()
                .map(|&i| encode_attack(0, i as u16))
                .collect(),
            is_optional: true,
            prompt: "Declare an ally (Alliance)".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(attacker),
            source_kind: crate::enums::EffectSourceKind::Digimon,
            // Resume-driven (frame below); the closures mirror it.
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                game.run_alliance_selection_step(attacker, action_id, false);
            }),
            on_decline: Some(Box::new(|_game: &mut Game| {})),
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::AllianceSelection(
                crate::resume::CombatAllianceState {
                    attacker,
                    outer_conts: Vec::new(),
                },
            )],
        });
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
    /// Scope: **every attack** — Digimon-target AND player-target.
    /// `general_rule.pdf` 11-1-3 lists Counter timing as a step of every
    /// attack (declaration -> Counter -> Block -> confirming success). The
    /// former "Digimon-target only" scope mirrored the retired Python engine
    /// (`combat.py:139`) and had no rules basis
    /// (G-ENGINE-COUNTER-NO-WINDOW-ON-PLAYER-ATTACK, 2026-10-04).
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

        // Counter timing opens on EVERY attack (rule 11-1-3: declaration →
        // Counter → Block → …), including attacks on the player — DCGO
        // `AttackProcess.CounterTiming` has no target check, and its
        // `CounterClass` fires whenever an opponent's permanent attacks.
        let defender_player = match pa.effective_target {
            AttackTarget::Digimon(h) => h.player,
            AttackTarget::Player(p) => p,
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
                        // Q18 (G-BLAST-DIGIVOLVE-IMMUNITY): a Digimon immune to
                        // its own controller's Digimon effects cannot use
                        // <Blast Digivolve> — Blast Digivolve is itself a
                        // Digimon effect. Quantumon (LM-020) is "unaffected by
                        // ALL Digimon effects, including its own" (immunity
                        // filter `Any`), so it is not a valid Blast target even
                        // for its own controller's blast.
                        if self.permanent_is_unaffected_by_effect(
                            handle,
                            defender_player,
                            crate::enums::EffectSourceKind::Digimon,
                        ) {
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
                if has_counter_option && !self.option_use_blocked(defender_player) {
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
            // A spent [Once Per Turn] / max-per-turn Counter ability, or one
            // whose activation condition fails, would resolve to nothing (the
            // effect queue suppresses it) — so it is not a real candidate and
            // offering it would be a phantom choice (rule 17).
            let has_field_counter = effects.iter().enumerate().any(|(slot, e)| {
                if !(e.counter && e.timing == EffectTiming::CounterEffect) {
                    return false;
                }
                if e.max_per_turn > 0 {
                    let opt_key = e.shared_opt_group.unwrap_or(slot as u8);
                    if perm.activation_count(source_card, opt_key) >= e.max_per_turn {
                        return false;
                    }
                }
                e.condition.as_ref().is_none_or(|cond| {
                    let ctx = crate::effect_context::EffectReadContext::new_with_source_kind(
                        self,
                        source_card,
                        Some(handle),
                        crate::enums::EffectSourceKind::Digimon,
                        defender_player,
                    );
                    cond(&ctx)
                })
            });
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
        let resume_valid_action_ids = valid_action_ids.clone();
        let resume_candidates = candidate_snapshot.clone();

        // §11-1-5: the counter timing occurs even after the attacker left the
        // battle area. Its slot may be empty or reused by then, so read the
        // attacking card from the declaration context, never from the slot.
        // Before the declaration commits (an effect-opened window) the slot
        // handle is still the attacker, as in `live_attack_attacker`.
        let declaration_committed = self
            .pending_attack
            .as_ref()
            .is_some_and(|pa| pa.declaration_committed);
        let attacker_present =
            !declaration_committed || self.handle_still_attacking(attacker);
        let source_card = if attacker_present {
            match self
                .player(attacker.player)
                .battle_area
                .get(attacker.index as usize)
            {
                Some(perm) => perm.top_card().handle(),
                None => return false,
            }
        } else {
            match self.attack_attacker_card {
                Some(card) => card,
                // No declaration context: nothing names the attacking card.
                // Returned before the depth bump, so no window is consumed.
                None => return false,
            }
        };
        let source_permanent = attacker_present.then_some(attacker);

        // Increment the counter-depth guard — subsequent nested attacks
        // launched by the counter body will see `counter_depth >= 1` and
        // skip their own Counter windows.
        if let Some(pa) = self.pending_attack.as_mut() {
            pa.counter_depth = pa.counter_depth.saturating_add(1);
        }

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::CounterTiming;

        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
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
            source_permanent,
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
                // the attacker. §11-1-5: the block timing still occurs; it
                // offers no block without an attacking Digimon (§12-1-6)
                // and the attack then ends as not successful (§11-5-1-4-1).
                if game.pending_attack.is_none() {
                    return;
                }
                game.transition_attack_state(AttackState::BlockOpen);
                game.advance_pending_attack();
            }),
            on_decline: Some(Box::new(move |game: &mut Game| {
                if let Some(pa) = game.pending_attack.as_mut() {
                    pa.state = AttackState::BlockOpen;
                }
                game.advance_pending_attack();
            })),
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::CounterSelection(
                crate::resume::CombatCounterState {
                    defender_player,
                    valid_action_ids: resume_valid_action_ids,
                    candidates: resume_candidates,
                    outer_conts: Vec::new(),
                },
            )],
        });

        true
    }

    pub(crate) fn run_counter_selection_step(
        &mut self,
        defender_player: PlayerId,
        valid_action_ids: &[u16],
        candidates: &[CounterCandidate],
        action_id: u16,
        is_pass: bool,
    ) {
        if is_pass {
            if let Some(pa) = self.pending_attack.as_mut() {
                pa.state = AttackState::BlockOpen;
            }
            self.advance_pending_attack();
            return;
        }

        let picked = valid_action_ids
            .iter()
            .position(|&a| a == action_id)
            .and_then(|pos| candidates.get(pos).copied());
        if let Some(cand) = picked {
            self.resolve_counter_selection(defender_player, cand);
        }

        if self.pending_selection.is_some() {
            return;
        }

        // §11-1-5: the block timing follows even if a counter body removed
        // the attacker; `try_enter_block` then offers no block (§12-1-6).
        if self.pending_attack.is_none() {
            return;
        }
        self.transition_attack_state(AttackState::BlockOpen);
        self.advance_pending_attack();
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
        // Q18 (G-BLAST-DIGIVOLVE-IMMUNITY): defensive abort if the target is
        // immune to its controller's Digimon effects (candidate collection
        // already filters these out; this guards effect-driven blast paths).
        let target = PermanentHandle {
            player: defender,
            index: f_idx as u8,
        };
        if self.permanent_is_unaffected_by_effect(
            target,
            defender,
            crate::enums::EffectSourceKind::Digimon,
        ) {
            return;
        }
        let card = self.player_mut(defender).hand.remove(h_idx);
        let turn = self.turn_count;
        self.player_mut(defender).battle_area[f_idx].digivolve(card, turn);

        // §8-1-3-3: the digivolution procedure "places the Digimon ... on top
        // of the chosen card, draws 1 card"; <Blast Digivolve> "is an effect
        // that digivolves the chosen Digimon" (16-26-4), so it draws like any
        // other digivolution (DCGO `PlayCardClass`: `if (isEvolution) ...
        // DrawClass(card.Owner, 1)`, CardController.cs ~1762). Lands after the
        // stack mutation and BEFORE the WhenDigivolving fan-out, mirroring
        // `effect_initiated_digivolve`. Empty deck: 8-1-2-10, modelled by
        // `draw()`. Exam EX7-059#effect#1.
        self.player_mut(defender).draw();

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
            zone_owner: None,
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
                    zone_owner: None,
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
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::CounterBlastDnaFieldMaterial(
                crate::resume::CombatBlastDnaFieldMaterialState {
                    defender,
                    result_hand_index,
                    source_card,
                    previous_phase,
                    outer_conts: Vec::new(),
                },
            )],
        });
        true
    }

    pub(crate) fn run_counter_blast_dna_field_material_step(
        &mut self,
        defender: PlayerId,
        result_hand_index: usize,
        source_card: CardHandle,
        previous_phase: GamePhase,
        action_id: u16,
    ) {
        use crate::action::space::PLAY_HAND_START;

        let field_idx = action_id as usize;
        let hand_targets: Vec<u16> = self
            .valid_blast_dna_hand_materials_for_hand_card(defender, result_hand_index, field_idx)
            .map(|idx| PLAY_HAND_START + idx)
            .collect();
        if hand_targets.is_empty() {
            return;
        }

        self.current_phase = GamePhase::SelectHand;
        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
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
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::CounterBlastDnaHandMaterial(
                crate::resume::CombatBlastDnaHandMaterialState {
                    defender,
                    result_hand_index,
                    field_idx,
                    outer_conts: Vec::new(),
                },
            )],
        });
    }

    pub(crate) fn run_counter_blast_dna_hand_material_step(
        &mut self,
        defender: PlayerId,
        result_hand_index: usize,
        field_idx: usize,
        action_id: u16,
    ) {
        use crate::action::space::PLAY_HAND_START;

        if action_id < PLAY_HAND_START {
            return;
        }
        let material_idx = (action_id - PLAY_HAND_START) as usize;
        self.execute_blast_dna_digivolve(defender, result_hand_index, field_idx, material_idx);
    }

    fn execute_blast_dna_digivolve(
        &mut self,
        defender: PlayerId,
        result_hand_index: usize,
        field_idx: usize,
        material_hand_index: usize,
    ) {
        let Some(route) = self.blast_dna_route_for_hand_card(
            defender,
            result_hand_index,
            field_idx,
            material_hand_index,
        ) else {
            return;
        };
        if field_idx >= self.player(defender).battle_area.len()
            || result_hand_index >= self.player(defender).hand.len()
            || material_hand_index >= self.player(defender).hand.len()
            || result_hand_index == material_hand_index
        {
            return;
        }

        self.reset_dna_material_objects(&[PermanentHandle {
            player: defender,
            index: field_idx as u8,
        }]);

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
            // DNA rule 8-2-2-2: the first requirement is the upper material.
            // Sources are stored bottom-to-top; keep the entire field block
            // intact and use the already validated Blast route orientation.
            if route.first_is_top {
                perm.card_sources.insert(0, material_card);
            } else {
                perm.card_sources.push(material_card);
            }
            perm.push_top(result_card);
            perm.turn_digivolved = turn;
        }

        self.clear_dna_card_overlays(PermanentHandle {
            player: defender, index: field_idx as u8,
        });

        // Official 8-2-3-3: Blast DNA still performs the DNA procedure's
        // bonus draw after stacking and before digivolution effects activate.
        if let Err(e) = self.draw_one_for_player(defender) {
            eprintln!(
                "[opaque-deck] Blast DNA digivolve draw error for player {}: {}",
                defender, e
            );
        }

        let handle = PermanentHandle {
            player: defender,
            index: field_idx as u8,
        };
        let event_card = self.player(defender).battle_area[field_idx]
            .top_card()
            .handle();
        self.fire_digivolve_event_triggers(handle, event_card, true, true);
    }

    /// Scan the defender's battle area for unsuspended Digimon with the
    /// Blocker keyword (`Game::has_keyword`: printed, inherited, modifier-
    /// granted, or the live `<Collision>` grant of §16-30-1). Install a
    /// `PendingSelection` in `BlockTiming` if any candidate exists. Returns `true` if a selection was installed,
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

        // §12-1-6 (Ver. 4.3): "A block can only be performed if an attacking
        // Digimon is in the battle area." Once declared, the attacker is
        // judged by its live attacking state (its slot may be reused after it
        // left); the block timing itself still occurs (§11-1-5) but offers no
        // block. DCGO `AttackProcess.cs` `BlockTiming()` ends the attack when
        // `AttackingPermanent.TopCard == null || !IsDigimon`.
        if pa.declaration_committed && !self.handle_still_attacking(attacker) {
            return false;
        }

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

        // "This Digimon can't be blocked": an attacker carrying
        // `CannotBeBlocked` opens no block (DCGO `CanNotBeBlockedClass`,
        // `defenderCondition: null` — no Digimon may block it). Checked
        // before the candidate scan so even a <Collision>-forced block
        // window does not open. G-ENGINE-CANNOT-BE-BLOCKED (EX2-068).
        if self.modifiers.has(attacker, ModifierType::CannotBeBlocked) {
            return false;
        }

        // Self-block is not allowed — attacker cannot block their own
        // attack. Also rules out the edge case where attacker and blocker
        // would be the same permanent.
        let attacker_is_defender = attacker.player == defender_player;

        // §16-30-1 / §16-30-4: <Collision> makes the block mandatory (the
        // forcing affects the opponent PLAYER); the <Blocker> every affected
        // opponent Digimon gains is granted declaratively in
        // `Game::has_keyword` (`collision_grants_blocker`), not here.
        let attacker_has_collision = self.has_keyword(attacker, Keyword::Collision);
        // §12-1-5: the attack target can't block. Found by identity: a
        // removed target (§11-2-6) is not on the field, and the Digimon that
        // compacted into its slot is not the attack target.
        let live_target = self.live_attack_target_digimon();

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
            // Official rule 12-1-5: the current attack target cannot block.
            // Use the effective target after Raid/Counter redirects, not the
            // originally declared target. Other unsuspended blockers remain legal.
            if live_target == Some(h) {
                continue;
            }
            let perm = &self.player(defender_player).battle_area[i];
            if perm.is_suspended {
                continue;
            }
            if !self.permanent_is_digimon_for_rules(h) {
                continue;
            }
            // `CannotBlock` (Phase 6 restriction) short-circuits
            // candidacy regardless of Collision -- the <Collision> grant
            // gives the Digimon <Blocker> but does NOT override a
            // printed/modifier `CannotBlock` gate.
            if self.modifiers.has(h, ModifierType::CannotBlock) {
                continue;
            }
            // 12-1-4: "A block can't be performed using a Digimon that
            // can't suspend" — blocking suspends the blocker. (DCGO
            // `Permanent.CanBlock` → `!CanSuspend`, Permanent.cs:2140.)
            if self.modifiers.has(h, ModifierType::CannotSuspend) {
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
            // §16-30-1 / §16-30-4: <Collision> grants <Blocker> to every
            // opponent Digimon its effects can affect, declaratively inside
            // `has_keyword` (`collision_grants_blocker`) -- so candidacy is
            // just "has <Blocker>" and a Digimon unaffected by the attacker's
            // effects is a candidate only through a Blocker it already has.
            if !self.has_keyword(h, Keyword::Blocker) {
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
            zone_owner: None,
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
                // Declaring Blocker suspends the chosen Digimon before the
                // attack target is rewritten.
                game.suspend(blocker);
                // OnAttackTargetChange: fires in all players' battle areas
                // when Block rewrites effective_target. The payload carries
                // attacker, old/new targets, reason, and controller.
                game.apply_attack_target_substitution_with_reason(
                    AttackTarget::Digimon(blocker),
                    AttackTargetChangeReason::Blocker,
                );

                // OnBlock fires globally after the blocker is declared.
                // The payload carries the blocked attacker as the event
                // permanent so inherited "when this Digimon is blocked"
                // predicates can test the carrier directly.
                if let Some(card) = game
                    .player(attacker.player)
                    .battle_area
                    .get(attacker.index as usize)
                    .map(|perm| perm.top_card().handle())
                {
                    game.enqueue_triggered(
                        crate::enums::EffectTiming::OnBlock,
                        crate::selection::TriggerSource::BlockDeclared {
                            attacker,
                            blocker,
                            card,
                        },
                    );
                }
                game.drain_effect_queue();

                game.advance_pending_attack();
            }),
            on_decline: Some(Box::new(move |game: &mut Game| {
                // Block declined — advance to PostBlock, then Battle. The
                // attack proceeds against its target; a target removed
                // meanwhile makes it fail (§11-2-6).
                if let Some(pa) = game.pending_attack.as_mut() {
                    pa.state = AttackState::PostBlock;
                }
                game.advance_pending_attack();
            })),
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::BlockSelection(
                crate::resume::CombatBlockState {
                    attacker,
                    defender_player,
                    outer_conts: Vec::new(),
                },
            )],
        });

        true
    }

    pub(crate) fn run_block_selection_step(
        &mut self,
        attacker: PermanentHandle,
        defender_player: PlayerId,
        action_id: u16,
        is_pass: bool,
    ) {
        if is_pass {
            if let Some(pa) = self.pending_attack.as_mut() {
                pa.state = AttackState::PostBlock;
            }
            self.advance_pending_attack();
            return;
        }

        use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
        let offset = action_id.saturating_sub(ATTACK_START);
        let blocker_index = (offset % TARGETS_PER_ATTACKER) as u8;
        let blocker = PermanentHandle {
            player: defender_player,
            index: blocker_index,
        };

        if self
            .validate_attack_redirect_target(attacker, AttackTarget::Digimon(blocker))
            .is_err()
        {
            if let Some(pa) = self.pending_attack.as_mut() {
                pa.state = AttackState::PostBlock;
            }
            self.advance_pending_attack();
            return;
        }

        if let Some(pa) = self.pending_attack.as_mut() {
            pa.is_blocked = true;
            pa.blocker = Some(blocker);
            pa.state = AttackState::PostBlock;
        }
        self.suspend(blocker);
        self.apply_attack_target_substitution_with_reason(
            AttackTarget::Digimon(blocker),
            AttackTargetChangeReason::Blocker,
        );

        if let Some(card) = self
            .player(attacker.player)
            .battle_area
            .get(attacker.index as usize)
            .map(|perm| perm.top_card().handle())
        {
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnBlock,
                crate::selection::TriggerSource::BlockDeclared {
                    attacker,
                    blocker,
                    card,
                },
            );
        }
        self.drain_effect_queue();

        self.advance_pending_attack();
    }

    /// `<Raid>`'s switch options for `attacker` (§16-23-1 / §16-23-4): empty
    /// unless `attacker` is the Digimon currently attacking and its target can
    /// be switched. Gates the `<Raid>` trigger (DCGO `CanActivateRaid`) and
    /// feeds its body. There is no other `<Raid>` retarget: a removed attack
    /// target makes the attack fail (§11-2-6).
    pub(crate) fn raid_switch_options(&self, attacker: PermanentHandle) -> Vec<u8> {
        // `attacker` must be the attacking Digimon, found by identity.
        if self.live_attack_attacker() != Some(attacker) {
            return Vec::new();
        }
        // Track C: `CannotSwitchAttackTarget` locks the attack onto its
        // declared target, so <Raid> has nothing to switch to.
        if self
            .modifiers
            .has(attacker, ModifierType::CannotSwitchAttackTarget)
        {
            return Vec::new();
        }
        let (current_target, current_removed) = self.attack_target_for_switch();
        self.raid_switch_candidates(attacker, current_target, current_removed)
    }

    /// Enumerate Raid's printed candidates: opponent unsuspended Digimon tied
    /// for highest DP, excluding the current target because choosing it would
    /// not switch the attack target or fire the observer.
    fn raid_switch_candidates(
        &self,
        attacker: PermanentHandle,
        current_target: AttackTarget,
        current_target_removed: bool,
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
            if !current_target_removed
                && matches!(current_target, AttackTarget::Digimon(current) if current == handle) {
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

    /// `<Raid>`'s trigger body (`keyword_to_auto_effect`): park the optional
    /// switch pick for the attacker's controller (§16-23-3: optional; PASS
    /// keeps the target). No options → no prompt (the trigger's condition
    /// normally keeps it out of the pool).
    pub(crate) fn install_raid_switch_selection(&mut self, attacker: PermanentHandle) {
        use crate::action::space::encode_attack;

        let candidates = self.raid_switch_options(attacker);
        if candidates.is_empty() {
            return;
        }
        let attacker_player = attacker.player;
        let opp_id = 1 - attacker_player;
        let Some(source_card) = self
            .player(attacker_player)
            .battle_area
            .get(attacker.index as usize)
            .map(|p| p.top_card().handle())
        else {
            return;
        };
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectTarget;

        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::OppField,
            selecting_player: attacker_player,
            previous_phase,
            valid_action_ids: candidates
                .iter()
                .map(|&index| encode_attack(0, index as u16))
                .collect(),
            is_optional: true,
            prompt: "Raid - switch attack target".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(attacker),
            source_kind: crate::enums::EffectSourceKind::Digimon,
            // Resume-driven (frame below); the closures mirror it.
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                game.run_raid_selection_step(
                    attacker,
                    opp_id,
                    crate::resume::CombatRaidSelectionMode::Switch,
                    action_id,
                    false,
                );
            }),
            on_decline: Some(Box::new(|_game: &mut Game| {})),
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::RaidSelection(
                crate::resume::CombatRaidSelectionState {
                    attacker,
                    opponent_player: opp_id,
                    mode: crate::resume::CombatRaidSelectionMode::Switch,
                    outer_conts: Vec::new(),
                },
            )],
        });
    }

    pub(crate) fn run_raid_selection_step(
        &mut self,
        attacker: PermanentHandle,
        opponent_player: PlayerId,
        // Single-variant since the post-Block retarget was removed; kept so
        // the `ResumeFrame::RaidSelection` shape is unchanged.
        _mode: crate::resume::CombatRaidSelectionMode,
        action_id: u16,
        is_pass: bool,
    ) {
        // Switch (the `<Raid>` trigger body): PASS keeps the target; the
        // attack state machine resumes once the trigger pool has drained.
        if is_pass {
            return;
        }

        use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
        let offset = action_id.saturating_sub(ATTACK_START);
        let new_index = (offset % TARGETS_PER_ATTACKER) as u8;
        let new_target = PermanentHandle {
            player: opponent_player,
            index: new_index,
        };

        let valid = self
            .validate_attack_redirect_target(attacker, AttackTarget::Digimon(new_target))
            .is_ok();
        if valid {
            self.apply_attack_target_substitution_with_reason(
                AttackTarget::Digimon(new_target),
                AttackTargetChangeReason::Raid,
            );
        }
    }

    /// Park an effect-offered attack declaration for `state.attacker`'s
    /// controller: one optional prompt whose choices are
    /// `state.valid_action_ids` (the carrier's legal `encode_attack`
    /// declarations) and PASS. Resume-driven
    /// (`ResumeFrame::AttackDeclarationDecision`); the closures only mirror the
    /// frame. A declinable decision, so it marks the §18-3 loop watch
    /// (`mark_pending_selection_as_optional_decision`). The selecting player
    /// is always the attacker's controller (the Digimon "may attack" for its
    /// controller), whatever selecting-player override the enclosing effect
    /// runs under. Returns whether a prompt was installed (none on a finished
    /// game, when nothing is offered, or when a decision is already parked).
    ///
    /// Offered by `<Blitz>` (§16-16-3, inside the effect that executes it)
    /// and by the `[End of Your Turn]` trigger bodies of `<Engage>`,
    /// `<Execute>` and `<Vortex>` (§16-44-2, §16-38-2, §16-33-2), inside the
    /// end-of-turn pool. While parked, `check_turn_end` is a no-op (a
    /// selection is pending) and the mask exposes only this prompt's choices.
    pub(crate) fn install_attack_declaration_decision(
        &mut self,
        state: crate::resume::AttackDeclarationDecisionState,
        prompt: &str,
    ) -> bool {
        // A finished game offers no decisions (§1-2-2; G-ENGINE-GAME-OVER-NOT-TERMINAL):
        // the drainer stops on `game_over`, and so does this installer.
        if self.game_over || state.valid_action_ids.is_empty() || self.pending_selection.is_some()
        {
            return false;
        }
        let attacker = state.attacker;
        let source_card = state.source_card;
        let source_kind = state.source_kind;
        let valid_action_ids = state.valid_action_ids.clone();
        let callback_state = state.clone();
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectTarget;
        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::Target,
            selecting_player: attacker.player,
            previous_phase,
            valid_action_ids,
            is_optional: true,
            prompt: prompt.to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(attacker),
            source_kind,
            // Resume-driven (frame below); the closures mirror it.
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                game.run_attack_declaration_decision_step(&callback_state, action_id, false);
            }),
            on_decline: Some(Box::new(|_game: &mut Game| {})),
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::AttackDeclarationDecision(state)],
        });
        self.mark_pending_selection_as_optional_decision();
        true
    }

    /// The `AttackTarget` an effect-offered attack declaration `action_id`
    /// (`encode_attack(attacker, target)`) names for `attacker`, or `None`
    /// when the id is another Digimon's declaration. `SECURITY_TARGET` is the
    /// opposing player; any other target index is that opponent's Digimon.
    /// Shared by every effect-offered declaration decode
    /// (`run_attack_declaration_decision_step`, `ResumeSelectKind::BeginAttack`).
    ///
    /// TODO: three decode-to-attack copies still bypass this helper -- the
    /// redirect and may-attack closures in `effect_context/action/combat.rs`
    /// (~L250, ~L467) and the `ResumeSelectKind::AttackTarget` redirect in
    /// `dsl_cards/step/selections.rs` (~L1081) -- and `ResumeSelectKind::BeginAttack`
    /// is still its own DSL frame rather than `ResumeFrame::AttackDeclarationDecision`.
    pub(crate) fn decode_effect_attack_target(
        &self,
        attacker: PermanentHandle,
        action_id: u16,
    ) -> Option<AttackTarget> {
        use crate::action::space::{decode_attack, SECURITY_TARGET};
        let (decoded_attacker, decoded_target) = decode_attack(action_id);
        if decoded_attacker as u8 != attacker.index {
            return None;
        }
        let opponent = self.next_clockwise(attacker.player);
        Some(if decoded_target == SECURITY_TARGET {
            AttackTarget::Player(opponent)
        } else {
            AttackTarget::Digimon(PermanentHandle {
                player: opponent,
                index: decoded_target as u8,
            })
        })
    }

    /// Resolve a parked effect-offered attack declaration: PASS declares
    /// nothing; a pick among the offered declarations declares the attack
    /// through the normal procedure (§11-1-5: every following timing occurs),
    /// opened as `state` says (initiator, suspension, upgrade). The attack
    /// does not re-check the memory gate that let the effect offer it
    /// (§16-16-4 for `<Blitz>`). The caller runs the frame's `outer_conts`.
    ///
    /// An end-of-turn keyword's declaration (`state.end_of_turn_keyword`) is
    /// made inside the `[End of Your Turn]` pool: the declaration's own
    /// triggers and the pool's remaining entries resolve before the counter
    /// timing (§11-1-4; Q3869), because `advance_pending_attack` yields while
    /// the queue is non-empty and the `resolve_selection` tail resumes it once
    /// the pool has drained. The keyword is recorded for the attack
    /// (`end_of_turn_attack_keyword`, cleared when the attack ends).
    pub(crate) fn run_attack_declaration_decision_step(
        &mut self,
        state: &crate::resume::AttackDeclarationDecisionState,
        action_id: u16,
        is_pass: bool,
    ) {
        if is_pass || !state.valid_action_ids.contains(&action_id) {
            return;
        }
        let Some(target) = self.decode_effect_attack_target(state.attacker, action_id) else {
            return;
        };
        let previous_keyword = self.end_of_turn_attack_keyword;
        self.end_of_turn_attack_keyword = state.end_of_turn_keyword;
        let result = self.begin_attack_open(AttackOpen {
            attacker: state.attacker,
            initiator: state.initiator,
            suspend_attacker: state.suspend_attacker,
            ignore_summoning_sickness: state.ignore_summoning_sickness,
            target_constraint: TargetConstraint::Forced(target),
            allow_cancel: true,
            cost_upgrade: state.cost_upgrade,
        });
        // No attack was installed (an illegal declaration): nothing to tag.
        if result == AttackResult::Invalid && self.pending_attack.is_none() {
            self.end_of_turn_attack_keyword = previous_keyword;
        }
    }

    /// Resolve the battle once all interrupt windows have cleared.
    /// Reads `effective_target` (which block / counter may have
    /// rewritten) and dispatches to the Digimon-vs-Digimon or
    /// security-loop path accordingly.
    fn resolve_pending_battle(&mut self) -> AttackResult {
        if let Some(outcome) = self.pending_attack_battle_result.take() {
            return outcome;
        }
        // Re-key both handles by identity first (a DNA / DigiXros field
        // removal skips the slot shift), and record a removed target.
        self.sync_attack_target();
        let Some(pa) = self.pending_attack.as_ref() else {
            return AttackResult::Invalid;
        };
        let attacker = pa.attacker;
        let target = pa.effective_target;

        if !self.handle_valid(attacker) {
            return AttackResult::Invalid;
        }

        match target {
            AttackTarget::Digimon(_) => {
                // §11-2-6 (Ver. 4.3): "Even if the attack target Digimon is
                // removed during an attack, that Digimon remains the attack
                // target, but the attack fails." Keyed on the recorded
                // removal, never on whatever now occupies the target's slot
                // (DCGO `DetermineAttackOutcome` battles only when
                // `IsPermanentExistsOnBattleAreaDigimon(DefendingPermanent)`).
                let defender = match self.live_attack_target_digimon() {
                    // A target still on the field that is no longer a
                    // Digimon in the standard state can't be battled either.
                    Some(defender) if self.handle_valid(defender) => defender,
                    // The attack is not successful (§11-5-1-4-1).
                    _ => return AttackResult::Unsuccessful,
                };
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

    /// Fire the owed `<Piercing>` follow-up security check, or finish the
    /// attack when the check can no longer be performed.
    ///
    /// The check is owed once a `<Piercing>` ACTIVATED in the battle's
    /// trigger pool (`effect_battle_piercing_pending`, §16-7-4); losing the
    /// keyword afterwards does not cancel it (official Q&A Q3931).
    /// Preconditions re-validated here (they may have changed while
    /// battle-triggered effects resolved, §16-7-4):
    /// - the attacker is still the in-flight attacking Digimon (§16-7-6);
    /// - the defending player has ≥ 1 security card — §16-6-6 [3.6 numbering; deleted in 4.3 — see changelog-3.6-to-4.3.md]: with 0
    ///   security the check "can't be performed" at all, and in particular
    ///   Piercing must NOT win the game (the 0-security game-win arm is
    ///   exclusive to unblocked player attacks, §11-5-1-2; DCGO parity:
    ///   `DetermineAttackOutcome` gates the post-battle check on
    ///   `SecurityCards.Count >= 1` and only ends the game on the
    ///   no-defender arm).
    fn fire_piercing_or_finish(&mut self, attacker: PermanentHandle) -> AttackResult {
        // Keyword state may have changed while battle-triggered effects
        // resolved — refresh the materialized aura grants before the
        // security-check pipeline reads them (same rationale as
        // `current_security_strike`).
        self.tick_declarative_effects();
        let piercing_activated = self.pending_attack.as_mut().is_some_and(|pa| {
            let pending = pa.effect_battle_piercing_pending;
            pa.effect_battle_piercing_pending = false;
            pending
        });
        let defender_player: PlayerId = 1 - attacker.player;
        if self.handle_still_attacking(attacker)
            && piercing_activated
            && self.pending_attack.as_ref().is_some_and(|pa| !pa.security_check_started)
            && !self.player(defender_player).security.is_empty()
        {
            match self.enter_piercing_security_check(attacker) {
                AttackResult::InProgress => AttackResult::InProgress,
                terminal => self.cleanup_attack(terminal),
            }
        } else {
            self.cleanup_attack(AttackResult::AttackerWins)
        }
    }

    /// `<Piercing>`'s trigger condition and activation gate (§16-7-1 /
    /// §16-7-2; DCGO `CanTriggerPierce` + `CanActivatePierce`), read with
    /// the battle's `BattleResolved` trigger context installed: `carrier`
    /// won the battle (found by identity, §15-8-3-8) and deleted its battle
    /// opponent; it is still the in-flight attacking Digimon and still has
    /// <Piercing>; the attack has checked no security yet and owes no
    /// <Piercing> check already (§16-7-5: one check per attack, so a second
    /// instance does not activate); the opponent has security to check.
    /// Covers the attack's own battle and an effect battle the attacking
    /// Digimon fights during the attack (Bandai CRM4 "May battle" notes).
    pub(crate) fn piercing_trigger_applies(&self, carrier: PermanentHandle) -> bool {
        let Some(trigger) = self.current_trigger_context.as_ref() else {
            return false;
        };
        let Some((winner_controller, winner_identity)) = trigger.battle_winner_identity else {
            return false;
        };
        if trigger.resolve_battle_winner(self) != Some(carrier) {
            return false;
        }
        let deleted_its_opponent = trigger.battle_deleters.iter().any(|deleter| {
            deleter.controller == winner_controller && deleter.battle_identity == winner_identity
        });
        let Some(pa) = self.pending_attack.as_ref() else {
            return false;
        };
        deleted_its_opponent
            && pa.attacker == carrier
            && !pa.cancelled
            && !pa.security_check_started
            && !pa.effect_battle_piercing_pending
            && self.handle_still_attacking(carrier)
            && self.has_keyword(carrier, Keyword::Piercing)
            && !self.player(1 - carrier.player).security.is_empty()
    }

    /// `<Piercing>`'s activation (§16-7-4; DCGO `PierceProcess`
    /// `DoSecurityCheck = true`): the security check becomes pending
    /// processing, performed immediately before the End of Attack timing by
    /// `fire_piercing_or_finish`, whether or not the attacker keeps the
    /// keyword (official Q&A Q3931).
    pub(crate) fn activate_piercing(&mut self, carrier: PermanentHandle) {
        if let Some(pa) = self.pending_attack.as_mut() {
            if pa.attacker == carrier && !pa.security_check_started {
                pa.effect_battle_piercing_pending = true;
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
        if let Some(pa) = self.pending_attack.as_mut() {
            pa.security_check_started = true;
        }
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
    /// Read-only `<Security A.>` strike components (unclamped), in the DCGO
    /// `Permanent.Strike` shape. Shared by the mutable combat-time
    /// [`Self::current_security_strike`] (which ticks declarative state first)
    /// and the read-only [`Self::effective_security_strike`] display path so
    /// the two never drift.
    ///
    /// A `security_attack_fn` formula aura is BASE-INCLUSIVE: it returns the
    /// total base check count (the default 1, replaced by the formula's value
    /// when active), and multiple formula auras take the MAX rather than
    /// summing. Flat `<Security A.>` keyword/modifier deltas (`sa_keyword`,
    /// `sa_modifier`, `change_s_attack`) are then added on top. A card whose
    /// `<Security A.>` is itself variable (e.g. WarGreymon's `1 + floor(n/2)`)
    /// must author its formula base-inclusive — `base: 2` over `material_count`
    /// yields `floor((2 + n)/2) = 1 + floor(n/2)`, NOT a bare delta `base: 0`.
    /// When no formula aura is active the default base of 1 applies.
    pub(crate) fn raw_security_strike(&self, target: PermanentHandle) -> i32 {
        let sa_modifier = self
            .modifiers
            .sum(target, ModifierType::SecurityAttackChange);
        let change_s_attack: i32 = self
            .modifiers
            .get(target, ModifierType::ChangeSAttack)
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
        let sa_keyword = self.security_attack_keyword_bonus(target);
        let base_checks = self.dynamic_security_attack_aura_bonus(target).unwrap_or(1);
        base_checks + sa_modifier + change_s_attack + sa_keyword
    }

    /// Read-only effective `<Security A.>` strike TOTAL for display / queries,
    /// clamped to `[0, MAX_SECURITY_CHECKS]`. This is exactly what the attacker
    /// would check if it attacked the player right now — the value the UI must
    /// render. Unlike [`Self::current_security_strike`] it does NOT tick
    /// declarative effects or emit fizzle events (callers mid-attack that need a
    /// guaranteed-fresh value use the combat-time variant). Returns 0 for an
    /// invalid handle.
    pub fn effective_security_strike(&self, target: PermanentHandle) -> i32 {
        if !self.handle_valid(target) {
            return 0;
        }
        self.raw_security_strike(target)
            .clamp(0, Self::MAX_SECURITY_CHECKS as i32)
    }

    fn current_security_strike(&mut self, attacker: PermanentHandle) -> u8 {
        if !self.handle_valid(attacker) {
            return 0;
        }
        // Refresh declarative state so the `<Security A.>` keyword term in
        // `raw_security_strike` reads the CURRENT materialized grants.
        // Inherited/aura declarative `grant_keyword` grants (e.g. ST1-07
        // Greymon's inherited `<Security A. +1>`) live in the keyword registry
        // only after a tick; the strike must not read a stale cache. The
        // `decode_action` path ticks before resolving an attack, but direct
        // callers (and mid-attack board changes between the per-iteration
        // recomputes) would otherwise miss it. `tick_declarative_effects` is
        // idempotent and de-dups own grants already printed in `card_data`, so
        // this does not double-count.
        self.tick_declarative_effects();
        let raw = self.raw_security_strike(attacker);
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
                if let Err(e) = self.materialize_opaque_security_placeholder(defender, top_idx) {
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
        let revealed_card_name = sec_card.card_name(&self.card_data).to_string();
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::SecurityReveal {
            seq,
            defender,
            card_id: revealed_card_id,
            card_name: revealed_card_name,
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

        // One reveal/removal occurrence: capture eligible entries now, before
        // Security can play new observers or change the attacking source.
        self.stamp_unstamped_trigger_batch();
        let start = self.effect_queue.len();
        self.enqueue_triggered(
            EffectTiming::OnSecurityCheck,
            TriggerSource::OnSecurityCheck {
                attacker,
                defender,
                revealed_card: card_handle,
                was_face_up,
            },
        );
        if was_face_up {
            self.enqueue_triggered(
                EffectTiming::OnCheckFaceUpSecurity,
                TriggerSource::OnCheckFaceUpSecurity {
                    attacker,
                    defender,
                    revealed_card: card_handle,
                },
            );
        }
        self.enqueue_triggered(
            EffectTiming::OnLoseSecurity,
            TriggerSource::SecurityRevealed {
                defender,
                card: card_handle,
            },
        );
        for (timing, observer_player) in [
            (EffectTiming::OnOwnSecurityRemoved, defender),
            (EffectTiming::OnOpponentSecurityRemoved, attacker.player),
        ] {
            self.enqueue_triggered(
                timing,
                TriggerSource::SecurityRemoved {
                    affected_player: defender,
                    observer_player,
                    source_player: attacker.player,
                    card: card_handle,
                    cause: crate::trigger_context::EventCause::SecurityRemoval,
                },
            );
        }
        self.freeze_security_check_entries(start);
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
    /// Rule 13-1-8 (Q6813): immediate Security, captured simultaneous observers,
    /// security battle, then disposal. Security choices pause the barrier.
    /// A security battle's triggered and scheduled effects finish before
    /// disposal or another check (general_rule.pdf 13-1-8 / 14-2).
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
                    // §13-1-8-2: every effect triggered by the security
                    // check resolves -- the [Security] effect first
                    // (§15-16-10-2, just drained: it activates immediately
                    // without pending activation, Q6813), then the check /
                    // removal observers captured at the reveal -- BEFORE the
                    // §13-1-8-3 battle against a Security Digimon. DCGO:
                    // `ISecurityCheck.SecurityCheck` drains
                    // `triggeredSkillInfos` (OnSecurityCheck) through
                    // `AutoProcessCheck` and only then enters
                    // `#region battle with security Digimon`.
                    self.set_security_phase(SecurityPhase::OnSecurityCheckDrain);
                }
                SecurityPhase::BattleResolved => {
                    // §16-39-1: the security battle compares the attacker's DP
                    // as it is now, as `resolve_battle_with` does.
                    self.refresh_progress_shield();
                    // Only Digimon security runs the DP battle; Option /
                    // Tamer / DigiEgg skip straight through.
                    let Some(state) = self.security_resolution.as_ref() else {
                        break;
                    };
                    let kind = state.card_kind;
                    let attacker_opt = state.attacker;
                    let attacker_top_before = attacker_opt.and_then(|h| {
                        self.player(h.player)
                            .battle_area
                            .get(h.index as usize)
                            .map(|p| p.top_card().handle())
                    });
                    if kind == CardKind::Digimon
                        && self
                            .pending_security
                            .as_ref()
                            .is_some_and(|pending| !pending.played)
                    {
                        if let Some(attacker) = attacker_opt {
                            if self.handle_still_attacking(attacker)
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
                                    .saturating_add(self.attacker_security_dp_adjustment(attacker))
                                    .saturating_add(
                                        self.defender_security_dp_adjustment(state.defender),
                                    );
                                // Rules 13-1-8-3 / 14-2: this is a battle even
                                // though the security Digimon cannot be deleted.
                                // Save the comparison before deletion can park on
                                // a replacement or an On Deletion choice.
                                self.set_security_phase(SecurityPhase::EndOfSecurityBattleDrain {
                                    attacker,
                                    attacker_top: attacker_top_before,
                                    winner: (attacker_dp > sec_dp).then_some(attacker),
                                });
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
                                    // Cause must be Battle, NOT the inferred
                                    // SecurityCheck. `infer_deletion_cause`
                                    // tests `security_resolution.is_some()`
                                    // before `pending_attack.is_some()`, and
                                    // during a security battle BOTH are set —
                                    // so the attacker's deletion was attributed
                                    // to the check rather than to the battle.
                                    // <Barrier>/<Evade> gate their replacement
                                    // on ReplacementCause::Battle and were
                                    // therefore filtered out before ever being
                                    // offered. Losing to a Security Digimon is
                                    // a battle deletion: rule 16-9 <Jamming>
                                    // names it outright ("not deleted as a
                                    // result of a battle with a Security
                                    // Digimon"), and the Jamming check directly
                                    // above relies on exactly that reading.
                                    self.delete_permanent_with_cause(
                                        attacker,
                                        crate::replacement::ReplacementCause::Battle,
                                    );
                                }
                                if self.pending_selection.is_some() {
                                    return None;
                                }
                                continue;
                            }
                        }
                    }
                    // Record the outcome from what actually HAPPENED rather than
                    // assuming a losing compare was lethal. Now that the cause is
                    // Battle, <Barrier>/<Evade> can prevent the deletion, so the
                    // old unconditional `AttackerDeletedBySecurity` would report a
                    // survivor as deleted. Compared by top-card identity, not by
                    // index: deleting a permanent shifts every later
                    // `battle_area` index down.
                    if let Some(attacker) = attacker_opt {
                        if !self.attacker_still_on_field(attacker, attacker_top_before) {
                            if let Some(st) = self.security_resolution.as_mut() {
                                st.outcome_so_far = AttackResult::AttackerDeletedBySecurity;
                            }
                        }
                    }
                    self.set_security_phase(SecurityPhase::WhenWouldLoseSecurity);
                }
                SecurityPhase::EndOfSecurityBattleDrain {
                    attacker,
                    attacker_top,
                    winner,
                } => {
                    // Deletion replacements and On Deletion effects finish
                    // before this phase is resumed. A Jamming/Barrier survivor
                    // is not a winner unless its DP was strictly higher.
                    let enqueue_done = state.phase_enqueue_done;
                    if !enqueue_done {
                        if !self.attacker_still_on_field(attacker, attacker_top) {
                            if let Some(st) = self.security_resolution.as_mut() {
                                st.outcome_so_far = AttackResult::AttackerDeletedBySecurity;
                            }
                        }
                        if let Some(st) = self.security_resolution.as_mut() {
                            st.phase_enqueue_done = true;
                        }
                        let (winner_identity, winner_snapshot) =
                            self.battle_winner_referent(winner);
                        // §14-2-4: the effects triggered by the battle
                        // ("wins a battle") resolve first; the end of the
                        // battle timing (§14-2-5) follows in
                        // `EndOfSecurityBattleTimingDrain`, which reads the
                        // same winner referent.
                        self.security_battle_end_referent =
                            Some((winner_identity, winner_snapshot.clone()));
                        // §15-8-3-1 / §15-8-3-4: only the effects whose trigger
                        // qualifier this battle met trigger
                        // (`enqueue_triggered` -> `discard_entries_not_triggered`).
                        self.enqueue_triggered(
                            EffectTiming::BattleResult,
                            TriggerSource::BattleResolved {
                                winner,
                                // Rule 14-2-3: beating a Security Digimon does
                                // not delete it, so no permanent is a deleter.
                                deleters: Vec::new(),
                                winner_identity,
                                winner_snapshot,
                            },
                        );
                    }
                    self.drain_effect_queue();
                    if self.pending_selection.is_some() {
                        return None;
                    }
                    self.set_security_phase(SecurityPhase::EndOfSecurityBattleTimingDrain);
                }
                SecurityPhase::EndOfSecurityBattleTimingDrain => {
                    // §14-2-5: the end of the battle timing, once the effects
                    // triggered by the battle have resolved.
                    if !state.phase_enqueue_done {
                        if let Some(st) = self.security_resolution.as_mut() {
                            st.phase_enqueue_done = true;
                        }
                        let (winner_identity, winner_snapshot) =
                            self.security_battle_end_referent.take().unwrap_or((None, None));
                        let winner = winner_identity
                            .and_then(|identity| self.permanent_by_battle_identity(identity));
                        self.enqueue_triggered(
                            EffectTiming::EndOfBattle,
                            TriggerSource::BattleResolved {
                                winner,
                                deleters: Vec::new(),
                                winner_identity,
                                winner_snapshot,
                            },
                        );
                        // §18-1-2: pending processing due at the end of the
                        // battle pools with the end-of-battle effects
                        // (enqueued once: the guard above survives a park).
                        self.enqueue_scheduled_for_timing(EffectTiming::EndOfBattle);
                    }
                    self.drain_effect_queue();
                    if self.pending_selection.is_some() {
                        return None;
                    }
                    self.set_security_phase(SecurityPhase::WhenWouldLoseSecurity);
                }
                SecurityPhase::OnSecurityCheckDrain => {
                    // §13-1-8-2 / Q6813: the check and removal observers
                    // (OnSecurityCheck, OnCheckFaceUpSecurity, OnLoseSecurity,
                    // On{Own,Opponent}SecurityRemoved) triggered
                    // simultaneously at the reveal and were captured there
                    // as one pool (`freeze_security_check_entries`); the
                    // turn player's bundle activates first (§15-4-3-2). No
                    // rescan here: a Digimon the [Security] effect played did
                    // not exist at the check. Effects derived from the
                    // [Security] body carry newer batch ids and activate
                    // first (G-ENGINE-TRIGGER-ORDER-DERIVED-FIRST).
                    self.drain_effect_queue();
                    if self.pending_selection.is_some() {
                        return None;
                    }
                    self.set_security_phase(SecurityPhase::BattleResolved);
                }
                SecurityPhase::WhenWouldLoseSecurity => {
                    // A battle-deletion replacement may still own the prompt.
                    // Probing a different replacement here would reset its
                    // offered-stage keys before the player accepts or declines.
                    if self.pending_selection.is_some() {
                        return None;
                    }
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
                SecurityPhase::OnLoseSecurityDrain | SecurityPhase::Dispose => {
                    // Check-driven removal effects (§13-1-6: the checked card
                    // left the security stack at the check) already activated
                    // in `OnSecurityCheckDrain`, before the battle (Q6813). Do
                    // not rescan or duplicate them at disposal; the trash
                    // push stays in `DisposeFinalize` (§13-1-8-4).
                    self.set_security_phase(SecurityPhase::DisposeFinalize);
                }
                SecurityPhase::DisposeFinalize => {
                    // §13-1-8-4, the last step of the check: the revealed card
                    // is placed in the trash "unless it belongs to an area" --
                    // i.e. unless an effect raised the `played` bit via
                    // `EffectContext::play_pending_security` (or otherwise
                    // moved it), in which case it already belongs somewhere
                    // and must not be trashed on top of that.
                    //
                    // Reached from BOTH the straight-through path and the
                    // park-and-resume path, so the `take()` runs exactly once
                    // per checked card either way.
                    // Read the defender BEFORE taking, so a missing
                    // resolution can never silently swallow the card.
                    if let Some(defender) = self.security_resolution.as_ref().map(|s| s.defender) {
                        if let Some(pending) = self.pending_security.take() {
                            if !pending.played {
                                self.player_mut(defender).put_in_trash(pending.card);
                            }
                        }
                    }

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
                    // A cancelled/exhausted carrier can finish this already
                    // revealed occurrence but cannot reveal another card.
                    if !self.handle_still_attacking(attacker)
                        || self
                            .pending_attack
                            .as_ref()
                            .is_some_and(|attack| attack.cancelled)
                    {
                        return Some(AttackResult::SecurityCheckSurvived);
                    }

                    // Recompute the live strike. Terminate when we've
                    // already popped enough cards for the current
                    // attacker; otherwise pop another.
                    let current_strike = self.current_security_strike(attacker);
                    if checks_performed >= current_strike {
                        return Some(AttackResult::SecurityCheckSurvived);
                    }
                    // `<Security A. +N>` overflow: if the defender already ran
                    // out of security during this same attack, the remaining
                    // extra checks simply do not happen — and crucially do NOT
                    // win the game. The 0-security knockout is decided exactly
                    // once, at the FIRST check (the primary attack connecting),
                    // in `resolve_player_security_loop`'s opening pop. An empty
                    // stack reached here is always a *subsequent* check
                    // (`checks_performed >= 1`), so it must end as a survived
                    // check, never `GameWon`.
                    //
                    // DCGO authority: `CardController.cs` `ISecurityCheck`'s
                    // multi-check loop `break`s on an empty stack (the
                    // `else { break; }` at the `SecurityCards.Count >= 1`
                    // guard) and never ends the game; the win is decided in
                    // `AttackProcess.DetermineAttackOutcome` BEFORE the loop.
                    // Rule 11-5-1-2 + the `wargreymon_security_attack` tests.
                    if self.player(defender).security.is_empty() {
                        return Some(AttackResult::SecurityCheckSurvived);
                    }
                    if !self.pop_and_start_security_check(attacker, defender, checks_performed) {
                        // Unreachable given the emptiness guard above, but keep
                        // the terminal branch defensive: an empty stack on a
                        // non-first check is a survived check, not a win.
                        return Some(AttackResult::SecurityCheckSurvived);
                    }
                    // Loop continues with a fresh state in SecuritySkillDrain.
                }
            }
        }

        // Defensive fallback: some branch break'd out (state unexpectedly
        // cleared). Treat as survived.
        Some(AttackResult::SecurityCheckSurvived)
    }

    /// True when `handle`'s permanent is still on the field carrying the same
    /// top card it had before a battle deletion was attempted.
    ///
    /// Index-independent on purpose: deleting a permanent removes it from
    /// `battle_area` and shifts every later index down, so a bare
    /// `handle_valid` can report "still there" while pointing at a different
    /// Digimon that slid into the slot.
    fn attacker_still_on_field(
        &self,
        handle: PermanentHandle,
        top_before: Option<crate::card_source::CardHandle>,
    ) -> bool {
        let Some(card) = top_before else {
            // Nothing was there to begin with; treat as "not deleted here" so
            // the caller does not report a deletion that never happened.
            return true;
        };
        self.player(handle.player)
            .battle_area
            .iter()
            .any(|p| p.top_card().handle() == card)
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
                    self.player_mut(pending.defender).put_in_hand(pending.card);
                }
                self.set_security_phase(SecurityPhase::DisposeFinalize);
            }
            ReplacementOutcome::Redirected(crate::enums::Zone::Deck) => {
                if let Some(pending) = self.pending_security.take() {
                    self.player_mut(pending.defender)
                        .insert_in_deck(0, pending.card);
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
        self.player_mut(defender).put_on_security_top(pending.card);
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

    /// The End of Attack timing, then the end of the attack. Called from
    /// both `begin_attack` (early-exit paths) and `advance_pending_attack`
    /// (normal terminal paths).
    ///
    /// §11-6-2 (Ver. 4.3): "For the end of the attack timing, the attack
    /// won't end until all processing has been resolved." The timing's pool
    /// is drained while the attack is still in progress; if it parks on a
    /// decision, `pending_attack` stays set in
    /// `AttackState::EndOfAttackDrain(outcome)` and this returns
    /// `InProgress`. The post-selection resume (`advance_pending_attack`)
    /// then calls `finish_attack_after_end_of_attack_timing`, which clears
    /// `is_attacking`, expires the end-of-attack modifiers, drops
    /// `pending_attack` and returns `outcome`. DCGO `AttackProcess.cs`
    /// `EndAttack()` likewise yields on the `OnEndAttack` stack before its
    /// clean-up step.
    fn cleanup_attack(&mut self, outcome: AttackResult) -> AttackResult {
        // A `<Piercing>` that activated in a battle's trigger pool -- the
        // attack's own battle, or an effect battle the attacking Digimon won
        // during the attack (which can also remove the declared target before
        // the attack's normal battle) -- left its check pending (§16-7-4). It
        // is processed here, immediately before the End of Attack timing,
        // after the battle's triggered effects, but never after an ordinary
        // security transition (§16-7-5).
        if let Some(pa) = self.pending_attack.as_ref() {
            if pa.effect_battle_piercing_pending && !pa.security_check_started && !pa.cancelled
                && self.handle_still_attacking(pa.attacker)
            {
                let attacker = pa.attacker;
                self.transition_attack_state(AttackState::PostBattle);
                if self.pending_selection.is_some() || !self.effect_queue.is_empty() {
                    return AttackResult::InProgress;
                }
                return self.fire_piercing_or_finish(attacker);
            }
        }
        self.pending_attack_battle_result = None;
        // Pre-declaration prevention cannot have reached battle resolution.
        // A later end-attack effect can follow an effect battle (15-15-1-1),
        // and jumps directly to EndOfAttack without pending Piercing.
        debug_assert!(
            !(outcome == AttackResult::Cancelled
                && self
                    .pending_attack
                    .as_ref()
                    .map(|pa| !pa.declaration_committed && pa.battle_occurred)
                    .unwrap_or(false)),
            "pre-declaration cancellation cannot have reached battle"
        );
        // EndOfAttack: observer timing, fires in every player's battle area.
        // Used by "at the end of an attack" effects. The attack is still in
        // progress (§11-6-2): the attacker is still attacking, the modifiers
        // "for the attack" still apply and `pending_attack` is still set, so
        // every effect in the pool sees the attack context.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                crate::enums::EffectTiming::EndOfAttack,
                crate::selection::TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        // §18-1-2: pending processing due at the end of the attack joins the
        // printed `EndOfAttack` effects in one simultaneous pool (ordered by
        // the turn player), before modifier expiry / the `pending_attack`
        // clear, so both see the same attack context.
        self.enqueue_scheduled_for_timing(crate::enums::EffectTiming::EndOfAttack);
        self.transition_attack_state(AttackState::EndOfAttackDrain(outcome));
        self.drain_effect_queue();
        // §11-6-2: a decision in the pool (a `TriggerOrder`, an optional
        // effect, a body's own pick) holds the attack open; the resume
        // re-enters `advance_pending_attack`, whose `EndOfAttackDrain` arm
        // finishes it. (No `pending_attack` means no attack to hold open:
        // finish at once, as before.)
        //
        // Park on a selection ONLY, not on a non-empty queue: the resume is
        // the `resolve_selection` tail, so only a selection re-enters
        // `advance_pending_attack`; parking on a queue with no selection would
        // strand the attack. `drain_effect_queue` returns with a non-empty
        // queue and no selection only when the game halted (the queue is
        // cleared then), so the two predicates agree in practice. Do not
        // "align" this with the resume's loop-top guard.
        if self.pending_selection.is_some() && self.pending_attack.is_some() {
            return AttackResult::InProgress;
        }
        self.finish_attack_after_end_of_attack_timing(outcome)
    }

    /// The end of the attack, once the End of Attack timing's processing has
    /// all resolved (§11-6-2): the attacker stops attacking, the
    /// end-of-attack modifiers expire, `pending_attack` clears, the rule
    /// check and the turn-end check run, and an end-of-turn attack lets the
    /// end of turn continue. Returns `outcome`.
    pub(crate) fn finish_attack_after_end_of_attack_timing(
        &mut self,
        outcome: AttackResult,
    ) -> AttackResult {
        // An end-of-turn attack: declared inside the `[End of Your Turn]`
        // pool (`EndTurn`: `<Engage>` / `<Execute>` / `<Vortex>` /
        // `<Overclock>`, §16-44-2 / §16-38-2 / §16-33-2 / §16-34-2, or any
        // other end-of-turn effect's attack) or in the `EndOfTurnAction`
        // window (a `MayAttack` grant).
        let end_of_turn_attack = self.pending_attack.as_ref().is_some_and(|pa| {
            matches!(
                pa.return_phase,
                crate::enums::GamePhase::EndOfTurnAction | crate::enums::GamePhase::EndTurn
            )
        });
        // The attacker by identity: a DNA / DigiXros field removal does not
        // re-key `pa.attacker`, and a Digimon that compacted into its slot is
        // not the attacker. Nothing to clear once it has left (e.g. deleted
        // by `<Execute>` in the End of Attack pool).
        if let Some(h) = self.live_attack_attacker() {
            if let Some(perm) = self
                .player_mut(h.player)
                .battle_area
                .get_mut(h.index as usize)
            {
                perm.is_attacking = false;
            }
        }

        self.modifiers.expire_end_of_attack();
        self.pending_attack = None;
        self.end_of_turn_attack_keyword = None;
        self.attack_attacker_identity = None;
        self.attack_attacker_card = None;
        self.attack_target_identity = None;
        // `expire_end_of_attack` dropped the `<Progress>` shield; keep the
        // registry in step with `progress_shield_target` (now `None`).
        self.refresh_progress_shield();

        // §17-1-3-1-1 / §15-15-5-2: ending the attack can change DP -- an
        // opponent's effect hidden by a `<Progress>` attacker's shield starts
        // applying (`expire_end_of_attack` dropped it), and "until the end of the
        // attack" buffs expire. Run the rule check now (the outermost drain
        // does it; a nested drain defers to its parent) instead of leaving a
        // 0-DP Digimon on the field until some later drain.
        if self.pending_selection.is_none() {
            self.drain_effect_queue();
        }

        // An attack-time effect — e.g. ST1-06 Coredramon's [When Attacking]
        // "Lose 2 memory" — can push memory across to the opponent's side. The
        // attack state machine is the only action that resolves entirely below
        // the action decoder (and `DebugRunner`/`LiveGame` call the combat
        // entry points directly), so nothing else re-checks turn end once the
        // attack completes. Do it here, at the single attack terminal, now that
        // `pending_attack` is cleared. Guard on Main phase: end-of-turn
        // Vortex/MayAttack attacks resolve their own rotation via
        // `pass_end_of_turn_action`, and `check_turn_end` internally no-ops
        // unless memory < 0 with no pending selection.
        if self.current_phase == crate::enums::GamePhase::Main {
            self.check_turn_end();
        }
        // §16-44-1 / §16-38-1 / §16-33-1 / §16-34-1: an end-of-turn attack is
        // over; the end of turn continues (a waiting keyword offer, then the
        // rest of the end of turn; no PASS-only step). Settled once nothing is
        // resolving (`settle_end_of_turn_attack`).
        if end_of_turn_attack {
            self.end_of_turn_attack_settle_pending = true;
            self.settle_end_of_turn_attack();
        }
        outcome
    }

    // ─── Private helpers ──────────────────────────────────────────────

    /// Suspend the attacker at declaration and count the attack. Returns the
    /// `OnSuspend` event card when the attacker actually transitioned to
    /// suspended; the caller enqueues the event in the declaration's trigger
    /// batch (general_rule.pdf 11-2-1: the attack declaration suspends the
    /// Digimon; DCGO `AttackProcess.cs:166` taps it via
    /// `SuspendPermanentsClass.Tap()`, which fires `OnTappedAnyone`) --
    /// G-ENGINE-ATTACK-SUSPENSION-NO-ONSUSPEND. The state flip goes through
    /// the canonical `suspend_state_only` so suspension-keyed auras re-tick
    /// as for any other suspension.
    fn suspend_and_count_attack(
        &mut self,
        handle: PermanentHandle,
    ) -> Option<crate::card_source::CardHandle> {
        let event_card = self.suspend_state_only(handle);
        let perm = &mut self.players[handle.player as usize].battle_area[handle.index as usize];
        // Backstop: attack legality already rejects a `CannotSuspend`
        // attacker (11-2-5); keep the declaration's suspended state even if
        // the chokepoint no-op'd.
        perm.is_suspended = true;
        self.count_attack(handle);
        event_card
    }

    fn count_attack(&mut self, handle: PermanentHandle) {
        let perm = &mut self.players[handle.player as usize].battle_area[handle.index as usize];
        perm.attacks_this_turn = perm.attacks_this_turn.saturating_add(1);
        if let Some(count) = self
            .digimon_attacks_this_turn
            .get_mut(handle.player as usize)
        {
            *count = count.saturating_add(1);
        }
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

    /// Re-key the in-flight attack's slot-based handles after one battle-area
    /// permanent was removed (companion to
    /// `ModifierRegistry::shift_after_battle_area_remove`). Without this, a
    /// removal below the attacker (e.g. EX8-028 paying a lower-indexed ally
    /// into security mid-attack) leaves `pending_attack.attacker` pointing at
    /// the WRONG slot — `handle_still_attacking` then fails and the attack
    /// silently fizzles before its security check.
    ///
    /// A handle that pointed AT the removed slot is left UNCHANGED: it acts
    /// as an identity token for in-flight handlers (Retaliation's
    /// `battle_opponent_of` compares the deleted carrier's pre-removal
    /// handle against `pending_attack.attacker` mid-OnDeletion), and the
    /// battle outcome no longer relies on re-probing it (see
    /// `PendingAttack::battle_defender_deleted`).
    /// Re-key QUEUED-EFFECT permanent handles after removing one battle-area
    /// permanent, the sibling of
    /// [`Self::shift_pending_attack_after_battle_area_remove`] and
    /// `ModifierRegistry::shift_after_battle_area_remove`.
    ///
    /// `PermanentHandle.index` is a POSITION into `Player::battle_area`, a Vec
    /// that `battle_area.remove(field_index)` compacts, so every handle at a
    /// later slot shifts down by one. Modifier state and the pending attack
    /// were already re-keyed on removal; `Game::effect_queue` was not, and a
    /// queued effect holds its carrier in `QueuedEffect::source_permanent`.
    /// After any mid-board removal it therefore addressed the WRONG permanent,
    /// or none once the Vec shrank -- and `queued_effect_source_is_live` read
    /// the resulting mismatch as "source gone", silently dropping a MANDATORY
    /// trigger belonging to a permanent that never moved.
    ///
    /// Ordinary pending effects from the removed carrier lose activation
    /// (15-4-4-3), even if that same physical card is played again before the
    /// queue drains. Preserve snapshot-backed OnDeletion entries: those activate
    /// from the deleted object and use the separate trash/token liveness gate.
    /// Zone-scoped entries with no source permanent are unaffected. OnLeaveField
    /// observers on surviving carriers shift normally; their event snapshot is
    /// not their own source identity.
    ///
    /// DCGO has no equivalent hazard: `FieldPermanents` is a sparse frame array
    /// and `GetFieldPermanents()` compacts a COPY, so a permanent's identity
    /// never moves.
    pub(crate) fn shift_effect_queue_after_battle_area_remove(
        &mut self,
        player: PlayerId,
        removed_index: u8,
    ) {
        let shift = |handle: &mut Option<PermanentHandle>| {
            if let Some(h) = handle.as_mut() {
                // Only actual battle-area slots compact. Breeding, DigiXros
                // limbo and absent/view sentinel handles retain their domain.
                if h.player == player && (h.index as u16) < crate::action::space::MAX_FIELD_SLOTS {
                    if h.index == removed_index {
                        *handle = None;
                    } else if h.index > removed_index {
                        h.index -= 1;
                    }
                }
            }
        };
        // The currently executing captured observer is no longer queued.
        // Its remaining steps must see the same rekeyed attacker as queued
        // siblings after a source deletes itself and compacts the board.
        if let Some(ctx) = self.current_trigger_context.as_mut() {
            if ctx.captured_security_check {
                shift(&mut ctx.target_permanent);
                shift(&mut ctx.event_permanent);
                shift(&mut ctx.event_host_permanent);
            }
        }
        self.invalidate_pending_field_effects(&[PermanentHandle { player, index: removed_index }]);
        for qe in self.effect_queue.iter_mut() {
            if let Some(ctx) = qe.trigger_context.as_mut() {
                if ctx.captured_security_check {
                    if qe.source_permanent
                        == Some(PermanentHandle {
                            player,
                            index: removed_index,
                        })
                    {
                        ctx.security_check_source_invalidated = true;
                    }
                    shift(&mut ctx.target_permanent);
                    shift(&mut ctx.event_permanent);
                    shift(&mut ctx.event_host_permanent);
                }
            }
            if let Some(h) = qe.source_permanent.as_mut() {
                if h.player == player
                    && h.index > removed_index
                    && (h.index as u16) < crate::action::space::MAX_FIELD_SLOTS
                {
                    h.index -= 1;
                }
            }
        }
        // Every field removal passes through this queue fixup, including moves
        // whose older callers did not fix PendingAttack. Keep the security
        // carrier alive across harmless compaction, but never alias a new slot.
        if let Some(state) = self.security_resolution.as_mut() {
            if state.attacker
                == Some(PermanentHandle {
                    player,
                    index: removed_index,
                })
            {
                state.outcome_so_far = AttackResult::AttackerDeletedBySecurity;
            }
            shift(&mut state.attacker);
        }
    }

    pub(crate) fn shift_pending_attack_after_battle_area_remove(
        &mut self,
        player: PlayerId,
        removed_index: u8,
    ) {
        if let Some(pa) = self.pending_attack.as_mut() {
            let shift = |h: &mut PermanentHandle| {
                if h.player == player && h.index > removed_index {
                    h.index -= 1;
                }
            };
            shift(&mut pa.attacker);
            if let AttackTarget::Digimon(h) = &mut pa.original_target {
                shift(h);
            }
            if let AttackTarget::Digimon(h) = &mut pa.effective_target {
                // §11-2-6 (Ver. 4.3): the removed Digimon remains the attack
                // target and the attack fails. Record the removal instead of
                // leaving a handle that the next permanent shifts into. A
                // blocker is the effective target once it blocks (§12-1-7),
                // so a removed blocker is covered here too.
                if h.player == player && h.index == removed_index {
                    pa.effective_target_removed = true;
                } else {
                    shift(h);
                }
            }
            if let Some(h) = pa.blocker.as_mut() {
                shift(h);
            }
        }
        // Every caller re-keyed the modifier registry first (which carried
        // the `<Progress>` shield along); re-derive it from the attacker's
        // identity so a removed attacker is never replaced by the Digimon
        // that slid into its slot.
        self.refresh_progress_shield();
    }

    /// The in-flight attack's attacker, or `None` once it has left the
    /// battle area. `PendingAttack::attacker` is a slot handle: after the
    /// attacker leaves, field compaction can put another permanent in that
    /// slot. Observers that the declaration triggered stay pending and still
    /// activate (§15-4-3-2; the attack happened), but must not read a later
    /// occupant of the slot as the attacker.
    ///
    /// Judged by the attacker's stack identity captured at declaration, not
    /// by `is_attacking` (which `finish_attack_after_end_of_attack_timing`
    /// clears once the End of Attack timing has resolved, §11-6-2).
    pub(crate) fn live_attack_attacker(&self) -> Option<PermanentHandle> {
        let pa = self.pending_attack.as_ref()?;
        if !pa.declaration_committed {
            return Some(pa.attacker);
        }
        let Some((player, identity)) = self.attack_attacker_identity else {
            return self.handle_still_attacking(pa.attacker).then_some(pa.attacker);
        };
        self.player(player)
            .battle_area
            .iter()
            .position(|perm| perm.battle_identity == Some(identity))
            .map(|index| PermanentHandle {
                player,
                index: index as u8,
            })
    }

    /// §11-2-6 (Ver. 4.3): whether the in-flight attack's Digimon target (the
    /// declared one, or the Digimon a block, redirect or `<Raid>` switched it
    /// to) has been removed from the battle area. It remains the attack
    /// target and the attack fails. Recorded by
    /// `shift_pending_attack_after_battle_area_remove` and by DNA
    /// (`PendingAttack::effective_target_removed`); the target's stack
    /// identity (`Game::attack_target_identity`) also catches a removal that
    /// did not re-key the pending attack's handles. Never true for a player
    /// target.
    pub(crate) fn attack_target_removed(&self) -> bool {
        self.pending_attack
            .as_ref()
            .is_some_and(|pa| matches!(pa.effective_target, AttackTarget::Digimon(_)))
            && self.live_attack_target_digimon().is_none()
    }

    /// The in-flight attack's Digimon target on the current board, found by
    /// its stack identity when one was captured. `None` with no attack, on a
    /// player target, or once the target was removed (§11-2-6): a Digimon
    /// that compacted into the target's slot is never returned.
    pub(crate) fn live_attack_target_digimon(&self) -> Option<PermanentHandle> {
        let pa = self.pending_attack.as_ref()?;
        let AttackTarget::Digimon(handle) = pa.effective_target else {
            return None;
        };
        if pa.effective_target_removed {
            return None;
        }
        match self.attack_target_identity {
            Some((player, identity)) => self.resolve_digivolve_host_identity(player, identity),
            None => Some(handle),
        }
    }

    /// The in-flight attack's target as effects see it: the player, or the
    /// live Digimon target; `None` with no attack or once the Digimon target
    /// was removed (§11-2-6).
    pub(crate) fn live_attack_target(&self) -> Option<AttackTarget> {
        match self.pending_attack.as_ref()?.effective_target {
            AttackTarget::Player(player) => Some(AttackTarget::Player(player)),
            AttackTarget::Digimon(_) => self.live_attack_target_digimon().map(AttackTarget::Digimon),
        }
    }

    /// The current target a switch compares against, and whether it was
    /// removed (§11-2-6: a removed target is never "the current target", so
    /// any candidate is a switch). Callers must have a pending attack.
    pub(crate) fn attack_target_for_switch(&self) -> (AttackTarget, bool) {
        let fallback = self
            .pending_attack
            .as_ref()
            .map_or(AttackTarget::Player(0), |pa| pa.effective_target);
        let current = self
            .live_attack_target_digimon()
            .map_or(fallback, AttackTarget::Digimon);
        (current, self.attack_target_removed())
    }

    /// Record the effective Digimon target's stack identity (allocating one if
    /// the permanent has none yet) for `live_attack_target_digimon`; `None`
    /// for a player target.
    fn capture_attack_target_identity(&mut self) {
        let target = self.pending_attack.as_ref().and_then(|pa| match pa.effective_target {
            AttackTarget::Digimon(h) if !pa.effective_target_removed => Some(h),
            _ => None,
        });
        let target = target.filter(|h| {
            self.players
                .get(h.player as usize)
                .is_some_and(|p| p.battle_area.get(h.index as usize).is_some())
        });
        let identity = target.map(|h| (h.player, self.capture_digivolve_host_identity(h)));
        self.attack_target_identity = identity;
    }

    /// Bring the pending attack's slot handles in line with the stack
    /// identities: re-key `attacker` to where the attacking Digimon is now
    /// (left alone once it has left, §11-1-5), and for a Digimon target
    /// record its removal (§11-2-6) or re-key `effective_target`.
    fn sync_attack_target(&mut self) {
        let attacker = self.live_attack_attacker();
        let removed = self.attack_target_removed();
        let live = self.live_attack_target_digimon();
        let Some(pa) = self.pending_attack.as_mut() else {
            return;
        };
        if let Some(attacker) = attacker {
            pa.attacker = attacker;
        }
        if !matches!(pa.effective_target, AttackTarget::Digimon(_)) {
            return;
        }
        if removed {
            pa.effective_target_removed = true;
        } else if let Some(handle) = live {
            pa.effective_target = AttackTarget::Digimon(handle);
        }
    }

    /// Record the declared attacker's stack identity (allocating one if the
    /// permanent has none yet) for `live_attack_attacker`.
    fn capture_attack_attacker_identity(&mut self, attacker: PermanentHandle) {
        let Some(perm) = self
            .players
            .get(attacker.player as usize)
            .and_then(|p| p.battle_area.get(attacker.index as usize))
        else {
            self.attack_attacker_identity = None;
            self.attack_attacker_card = None;
            return;
        };
        // §11-1-5: the declaration context the counter timing reads once the
        // attacker has left the battle area.
        self.attack_attacker_card = Some(perm.top_card().handle());
        let identity = match perm.battle_identity {
            Some(identity) => identity,
            None => {
                let identity = self.next_battle_identity;
                self.next_battle_identity =
                    identity.checked_add(1).expect("stack identity exhausted");
                self.player_mut(attacker.player).battle_area[attacker.index as usize]
                    .battle_identity = Some(identity);
                identity
            }
        };
        self.attack_attacker_identity = Some((attacker.player, identity));
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

        // Every committed declaration counts, including Overclock and other
        // attacks without suspending; only their suspension is skipped.
        let suspend_event = if !is_overclock {
            self.suspend_and_count_attack(attacker)
        } else {
            self.count_attack(attacker);
            None
        };
        self.reevaluate_until_condition_modifiers_if_dirty();

        // Fire OnAttack (may install a PendingSelection via a triggered
        // effect; the drainer returns and callers check below).
        self.fire_on_attack(attacker, suspend_event);

        if self.pending_selection.is_some() {
            return Some(AttackResult::InProgress);
        }

        // OnAttack may have deleted or moved the attacker. §11-1-5 (Ver. 4.3):
        // the declaration has been made, so the timings that follow still
        // occur -- `advance_pending_attack` runs the counter timing, offers no
        // block (§12-1-6) and ends the attack as not successful
        // (§11-5-1-4-1) when the attacker is gone.
        None
    }

    /// §11-5-1-4-1 (Ver. 4.3): the attack is not successful -- "no other
    /// actions are taken and the end of attack timing occurs." Reached when
    /// the attacking Digimon left the battle area after its declaration
    /// (§11-1-5: the counter and block timings already ran). Ends through
    /// `cleanup_attack` exactly like the removed-target unsuccessful attack
    /// (`resolve_pending_battle`'s defender-gone arm), with the same terminal
    /// result, `AttackResult::Unsuccessful` (`Invalid` means the attack could
    /// not be declared at all).
    fn end_attack_unsuccessful(&mut self) -> AttackResult {
        self.cleanup_attack(AttackResult::Unsuccessful)
    }

    /// Enqueue every effect the attack declaration triggers, then drain once.
    ///
    /// §15-4-3-2 (Ver. 4.3): effects triggered before a single rule or effect
    /// is resolved trigger simultaneously and pend together; §15-4-3-5-1/-2:
    /// the turn player orders all of theirs, then the non-turn player orders
    /// theirs. So the attacker's `OnAttack` and `[When Attacking]` effects
    /// (incl. the synthesized `<Alliance>` / `<Raid>` bodies, §16-24-2 /
    /// §16-23-2), the declaration suspension's `OnSuspend` observers, the
    /// attacker's allies' "when one of your Digimon attacks" observers and the
    /// opponent's "when an opponent's Digimon attacks" observers form ONE
    /// bundle drained by a single `maybe_drain_effect_queue` — DCGO
    /// `AttackProcess.cs` stacks them in one `StackSkillInfos(...,
    /// EffectTiming.OnAllyAttack)` batch. Draining between fan-outs used to
    /// lose every later observer as soon as an earlier one parked a prompt
    /// (G-ENGINE-WHEN-ATTACKING-PARK-DROPS-ALLY-ATTACK-OBSERVERS) and kept
    /// `<Alliance>` / `<Raid>` out of the pool
    /// (G-ENGINE-ALLIANCE-OUTSIDE-TRIGGER-POOL).
    ///
    /// `suspend_event` is the attacker's declaration-suspension `OnSuspend`
    /// event card (None for Overclock / an already-suspended attacker).
    fn fire_on_attack(
        &mut self,
        handle: PermanentHandle,
        suspend_event: Option<crate::card_source::CardHandle>,
    ) {
        use crate::selection::TriggerSource;

        self.capture_attack_attacker_identity(handle);
        // The shield now follows the attacker by identity.
        self.refresh_progress_shield();
        self.enqueue_triggered(EffectTiming::OnAttack, TriggerSource::Permanent(handle));
        if let Some(card) = suspend_event {
            self.enqueue_on_suspend(handle, card, false);
        }
        // `[When Attacking]` is carrier-scoped ("when THIS Digimon attacks"):
        // only the attacker's own stack (top card + inherited sources), never
        // a battle-area-wide scan — other Digimon observe via OnAllyAttack.
        self.enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(handle));

        let attacker_card = self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| perm.top_card().handle());
        let attack_source = |player: PlayerId| {
            attacker_card.map_or(TriggerSource::PlayerBattleArea(player), |card| {
                TriggerSource::PlayerBattleAreaAttack {
                    player,
                    attacker: handle,
                    card,
                }
            })
        };

        // OnAllyAttack: every permanent of the attacker's controller EXCEPT
        // the attacker itself — drop the attacker's own entries from this
        // fan-out only (its own OnAttack / [When Attacking] above stay).
        let ally_fanout_start = self.effect_queue.len();
        self.enqueue_triggered(EffectTiming::OnAllyAttack, attack_source(handle.player));
        let mut i = ally_fanout_start;
        while i < self.effect_queue.len() {
            if self.effect_queue[i].source_permanent == Some(handle) {
                self.effect_queue.remove(i);
            } else {
                i += 1;
            }
        }

        let opp = 1 - handle.player;
        if (opp as usize) < self.players.len() {
            self.enqueue_triggered(EffectTiming::OnOpponentAttack, attack_source(opp));
        }

        // G-DSL-OUTER-TAIL-NESTED-PARK: defer when inside select/outer-tail scope.
        self.maybe_drain_effect_queue();
    }

    /// Resolve battle between two permanents by DP comparison.
    ///
    /// Fires `EndOfBattle` for all players' battle areas after the DP
    /// comparison completes (before `EndOfAttack`). This resolver handles
    /// battle-area Digimon; security battles emit the same timing from
    /// their separate security-resolution phase machine.
    fn resolve_battle(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        self.resolve_battle_with(attacker, defender, BattleComparison::Dp)
    }

    fn resolve_battle_with(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
        comparison: BattleComparison,
    ) -> AttackResult {
        if let Some(pa) = self.pending_attack.as_mut() {
            pa.battle_occurred = true;
        }
        // §16-39-1: the battle compares DP as it is now -- `<Progress>` may
        // have been gained or lost since the last refresh.
        self.refresh_progress_shield();
        let a_dp = self.effective_dp(attacker).unwrap_or(0);
        let d_dp = self.effective_dp(defender).unwrap_or(0);

        // Phase F §F2 — Iceclad (RULES_CONTEXT 16-35): when EITHER
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
        //
        // A battle-scoped `BattleComparison::DigivolutionCards` (EX13-076,
        // G-ENGINE-BATTLE-COMPARE-SOURCE-COUNT) takes the same branch — DCGO
        // `IBattle.CompareStats`: `HasIceclad || CompareDigivolutionCards`.
        let iceclad_active = comparison == BattleComparison::DigivolutionCards
            || self.has_keyword(attacker, crate::enums::Keyword::Iceclad)
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

        self.capture_digivolve_host_identity(attacker);
        self.capture_digivolve_host_identity(defender);
        let snapshot = |handle: PermanentHandle| {
            let perm = &self.player(handle.player).battle_area[handle.index as usize];
            crate::trigger_context::BattleCombatantSnapshot {
                battle_identity: perm.battle_identity.expect("battle identity captured"),
                survived: false,
                controller: handle.player,
                top_card: perm.top_card().handle(),
                data_index: perm.top_card().data_index,
                identity: perm.synth_identity(&self.card_data, &self.modifiers, handle),
                is_suspended: perm.is_suspended,
            }
        };
        let outcome = if a_value > d_value {
            AttackResult::AttackerWins
        } else if a_value < d_value {
            AttackResult::DefenderWins
        } else {
            AttackResult::MutualDestruction
        };
        let is_attack = self.pending_attack.as_ref().is_some_and(|pa| {
            pa.state == AttackState::Battle
                && !pa.effective_target_removed
                && pa.attacker == attacker
                && pa.effective_target == AttackTarget::Digimon(defender)
        });
        let attacker_snapshot = snapshot(attacker);
        let defender_snapshot = snapshot(defender);
        let (winner_identity, winner_snapshot) = match outcome {
            AttackResult::AttackerWins => self.battle_winner_referent(Some(attacker)),
            AttackResult::DefenderWins => self.battle_winner_referent(Some(defender)),
            _ => (None, None),
        };
        let state = PendingBattleResolution {
            attacker: attacker_snapshot,
            defender: defender_snapshot,
            outcome,
            attacker_deleted: false,
            defender_deleted: false,
            winner_identity,
            winner_snapshot,
            is_attack,
            in_call: true,
            result_enqueued: false,
        };
        let index = self.pending_battle_resolutions.len();
        self.pending_battle_resolutions.push(state);
        let victims = match outcome {
            AttackResult::AttackerWins => vec![defender],
            AttackResult::DefenderWins => vec![attacker],
            // Both bodies leave as one batch before observer eligibility.
            _ => vec![defender, attacker],
        };
        self.delete_permanents_batch(victims, crate::replacement::ReplacementCause::Battle);
        self.pending_battle_resolutions[index].in_call = false;
        self.resume_pending_battle_resolutions();
        if self.pending_selection.is_some() {
            return AttackResult::InProgress;
        }
        if is_attack {
            self.pending_attack_battle_result.take().unwrap_or(outcome)
        } else {
            outcome
        }
    }

    /// The battle winner's identity (allocated if it has none yet) and its
    /// last-known information, taken at the comparison. Every `EndOfBattle`
    /// firing carries both, so a "wins a battle" qualifier resolves the
    /// winner by identity and, once it has left the field, still reads what
    /// it was when the battle was won (§15-8-3-1, §15-8-3-8, §15-4-4-5;
    /// DCGO keeps a copy of each winner in the battle hashtable's
    /// `WinnerPermanents`). `(None, None)` when there is no permanent winner.
    /// The Digimon-vs-Digimon battle calls this at the comparison; the
    /// security-battle path calls it when it enqueues `EndOfBattle`, which is
    /// equivalent: it has a permanent winner only when the attacker's DP was
    /// strictly higher, and nothing is deleted between (§14-2-3).
    pub(crate) fn battle_winner_referent(
        &mut self,
        winner: Option<PermanentHandle>,
    ) -> (
        Option<(PlayerId, u64)>,
        Option<std::sync::Arc<crate::trigger_context::DeletedObjectSnapshot>>,
    ) {
        let Some(winner) = winner else {
            return (None, None);
        };
        let Some(permanent) = self
            .player(winner.player)
            .battle_area
            .get(winner.index as usize)
        else {
            return (None, None);
        };
        let identity = match permanent.battle_identity {
            Some(identity) => identity,
            None => {
                let identity = self.next_battle_identity;
                self.next_battle_identity =
                    identity.checked_add(1).expect("battle identity exhausted");
                self.player_mut(winner.player).battle_area[winner.index as usize]
                    .battle_identity = Some(identity);
                identity
            }
        };
        let snapshot = self.build_snapshot_for_handle(winner).map(std::sync::Arc::new);
        (Some((winner.player, identity)), snapshot)
    }

    /// Record only a committed battle-cause deletion of an original combatant.
    /// Substitutes, redirects, and deletions by triggered effects do not count.
    pub(crate) fn record_battle_deletion(&mut self, player: PlayerId, top: CardHandle) {
        let Some(state) = self.pending_battle_resolutions.last_mut() else {
            return;
        };
        if state.attacker.controller == player && state.attacker.top_card == top {
            state.attacker_deleted = true;
        }
        if state.defender.controller == player && state.defender.top_card == top {
            state.defender_deleted = true;
        }
    }

    /// Freeze battle survival after the battle's physical deletions, before
    /// OnDeletion bodies can independently remove a combatant. Strict winner
    /// queries still resolve live identity separately when the event is emitted.
    pub(crate) fn capture_pending_battle_survival(&mut self) {
        if !self.active_deletion_batch.as_ref().is_some_and(|b|
            b.cause == crate::replacement::ReplacementCause::Battle) { return; }
        let Some(state) = self.pending_battle_resolutions.last() else { return; };
        let attacker = (state.attacker.controller, state.attacker.battle_identity);
        let defender = (state.defender.controller, state.defender.battle_identity);
        let attacker_survived = self.resolve_digivolve_host_identity(attacker.0, attacker.1).is_some();
        let defender_survived = self.resolve_digivolve_host_identity(defender.0, defender.1).is_some();
        let state = self.pending_battle_resolutions.last_mut().unwrap();
        state.attacker.survived = attacker_survived;
        state.defender.survived = defender_survived;
    }

    /// DNA creates a new Digimon; its materials cannot retain a pending battle win.
    /// Ordinary evolution keeps the original card in the same surviving Digimon.
    pub(crate) fn invalidate_battle_winners_for_dna(&mut self, materials: &[PermanentHandle]) {
        for material in materials {
            if let Some(perm) = self
                .player_mut(material.player)
                .battle_area
                .get_mut(material.index as usize)
            {
                perm.battle_identity = None;
                perm.native_play_incarnation = None;
            }
        }
    }

    /// Whether the Digimon whose top card was `deleted_top` was `carrier`'s
    /// opponent in the battle being resolved, read from the battle's own
    /// record (combatant identities + top cards at the comparison). `None`
    /// when no battle is being resolved or `carrier` is not one of its
    /// combatants. A deleted combatant has no battle-area handle, and since
    /// §11-2-6 the attack no longer names the Digimon that compacted into
    /// its slot, so "this Digimon deleted its battle opponent" is answered
    /// here rather than through a slot comparison.
    pub(crate) fn battle_record_opponent_was(
        &self,
        carrier: PermanentHandle,
        deleted_top: CardHandle,
    ) -> Option<bool> {
        let state = self.pending_battle_resolutions.last()?;
        let identity = self
            .player(carrier.player)
            .battle_area
            .get(carrier.index as usize)?
            .battle_identity?;
        let is = |c: &crate::trigger_context::BattleCombatantSnapshot| {
            c.controller == carrier.player && c.battle_identity == identity
        };
        if is(&state.attacker) {
            Some(state.defender.top_card == deleted_top)
        } else if is(&state.defender) {
            Some(state.attacker.top_card == deleted_top)
        } else {
            None
        }
    }

    /// The battle-area Digimon whose stack carries battle identity
    /// `(controller, identity)` on the current board, if it is still there.
    fn permanent_by_battle_identity(
        &self,
        (controller, identity): (PlayerId, u64),
    ) -> Option<PermanentHandle> {
        self.player(controller)
            .battle_area
            .iter()
            .position(|perm| perm.battle_identity == Some(identity))
            .map(|index| PermanentHandle {
                player: controller,
                index: index as u8,
            })
    }

    /// The `TriggerSource::BattleResolved` payload of `state` on the current
    /// board: the winner found by identity (`None` on a tie or once it has
    /// left the field), every combatant that deleted its battle opponent, and
    /// the winner's identity + last-known information from the comparison --
    /// carried even when `winner` no longer resolves, so a "wins a battle"
    /// qualifier reads what the winner was (§15-8-3-8;
    /// G-ENGINE-BATTLE-WINNER-QUALIFIER-STALE-INDEX).
    fn battle_resolved_source(&self, state: &PendingBattleResolution) -> TriggerSource {
        let winner = state
            .winner_identity
            .and_then(|identity| self.permanent_by_battle_identity(identity));
        let mut deleters = Vec::new();
        if state.defender_deleted {
            deleters.push(state.attacker.clone());
        }
        if state.attacker_deleted {
            deleters.push(state.defender.clone());
        }
        TriggerSource::BattleResolved {
            winner,
            deleters,
            winner_identity: state.winner_identity,
            winner_snapshot: state.winner_snapshot.clone(),
        }
    }

    /// Enqueue the effects triggered BY the innermost pending battle (§14-2-4)
    /// -- the "wins a battle" / "deletes in battle" observers and <Piercing>
    /// (`EffectTiming::BattleResult`) -- once.
    ///
    /// The battle's deletion batch calls this after its trash stage and before
    /// its single trigger flush (`complete_deletion_batch_after_replacements`
    /// stage 4d), so these effects pend in ONE pool with the loser's
    /// `[On Deletion]` and the deletion observers: they trigger simultaneously
    /// (§15-4-3-2, §16-7-4), the turn player activates theirs first and a
    /// player with 2+ orders them in a `TriggerOrder` (§15-4-3-5-1; official
    /// Q&A Q3931, EX8-045 Callismon). A battle in which no Digimon was
    /// deleted (a replacement cancelled every deletion) never reaches that
    /// flush; `resume_pending_battle_resolutions` enqueues it then.
    /// G-ENGINE-BATTLE-WIN-OBSERVERS-SEQUENCED-AFTER-ON-DELETION.
    pub(crate) fn enqueue_pending_battle_result_triggers(&mut self) {
        let Some(state) = self.pending_battle_resolutions.last() else {
            return;
        };
        if state.result_enqueued {
            return;
        }
        // Declarative state reflects the deletion before the trigger scan: a
        // keyword gained at this exact timing triggers (EX11-016's inherited
        // "[Your Turn] while your opponent has no Digimon with digivolution
        // cards" <Piercing> turns on when the battle deletes that last
        // sourced Digimon -- official Q&A: the freshly gained keyword DOES
        // trigger).
        self.tick_declarative_effects();
        let Some(state) = self.pending_battle_resolutions.last_mut() else {
            return;
        };
        state.result_enqueued = true;
        let state = state.clone();
        let source = self.battle_resolved_source(&state);
        // §15-8-3-1 / §15-8-3-4: an effect whose trigger qualifier is not met
        // by this battle never triggers, even if a sibling in the pool (the
        // loser's [On Deletion]) makes it true before it would activate
        // (`enqueue_triggered` -> `discard_entries_not_triggered`).
        let first = self.effect_queue.len();
        self.enqueue_triggered(EffectTiming::BattleResult, source);
        // §16-7-5: one <Piercing> check per attack however many instances the
        // attacker has; DCGO stacks exactly one (`CardController.cs`
        // `PutStackedSkill(skillInfos_Pierce[0])`). Keep one <Piercing> entry
        // per Digimon so identical instances are not offered as distinct
        // ordering branches.
        self.collapse_duplicate_keyword_triggers(first, Keyword::Piercing);
    }

    /// Finish the event after every replacement/rest-of-batch choice has settled.
    /// Pop before fanout: nested battles get their own payload and one fanout.
    ///
    /// §14-2-4 then §14-2-5: the effects triggered by the battle -- the
    /// loser's `[On Deletion]` and the `BattleResult` effects, one pool (see
    /// `enqueue_pending_battle_result_triggers`) -- all resolve before the
    /// end of the battle timing (`EndOfBattle`), which pools with the pending
    /// processing due at the end of the battle (§18-1-1 / §18-1-2). That
    /// sequencing holds for an attack's battle. A battle an effect performs is
    /// part of that effect's processing (§15-8-3-2): its battle-triggered
    /// effects, its end-of-battle effects and the pending processing all wait
    /// for the effect to finish and then pend in ONE pool with whatever else
    /// it triggered (§15-4-3-2; DCGO `IBattle(..., IsWithoutAttack: true)`
    /// stacks them and skips `AutoProcessCheck`).
    pub(crate) fn resume_pending_battle_resolutions(&mut self) {
        while self.pending_selection.is_none() && self.pending_deletion_batch_rest.is_empty() {
            if !self
                .pending_battle_resolutions
                .last()
                .is_some_and(|state| !state.in_call)
            {
                return;
            }
            // Normally enqueued (and drained) by the battle deletion's flush;
            // a no-op then.
            self.enqueue_pending_battle_result_triggers();
            // A battle an effect performs (EX13-077 "This Digimon may battle
            // 1 of your opponent's Digimon") is part of that effect's
            // processing: its triggers -- and the end-of-battle ones below --
            // wait for the effect to finish (§15-8-3-2) and pend with the rest
            // of what it triggered (§15-4-3-2). DCGO agrees: `IBattle` with
            // `IsWithoutAttack` stacks them and skips `AutoProcessCheck`. An
            // attack's battle resolves them before the attack continues
            // (§14-2-4), so it keeps the direct drain.
            // G-ENGINE-EFFECT-BATTLE-DRAIN-RUNS-TRIGGERS-MID-EFFECT.
            let is_attack = self
                .pending_battle_resolutions
                .last()
                .is_some_and(|state| state.is_attack);
            if is_attack {
                self.drain_effect_queue();
            } else {
                self.maybe_drain_effect_queue();
            }
            if self.pending_selection.is_some() {
                return;
            }
            let state = self.pending_battle_resolutions.pop().unwrap();
            let source = self.battle_resolved_source(&state);
            if state.is_attack && self.pending_attack.is_some() {
                self.pending_attack_battle_result = Some(state.outcome);
                if let Some(pa) = self.pending_attack.as_mut() {
                    pa.battle_defender_deleted = state.defender_deleted;
                }
            }
            self.enqueue_triggered(EffectTiming::EndOfBattle, source);
            // §18-1-2: pending processing due at the end of the battle pools
            // with the end-of-battle effects.
            self.enqueue_scheduled_for_timing(EffectTiming::EndOfBattle);
            if state.is_attack {
                self.drain_effect_queue();
            } else {
                self.maybe_drain_effect_queue();
            }
        }
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

        // Snapshot → trash → one flush pooling `[On Deletion]` with the
        // deletion observers (§16-13-2), the same tail as an ordinary batch.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.complete_deletion_batch_after_replacements();
        }));

        self.current_deletion_cause = prior_cause;
        self.active_deletion_batch = prior_batch;

        if let Err(payload) = result {
            std::panic::resume_unwind(payload);
        }
    }
}

#[cfg(test)]
mod pre_declaration_counter_window_tests {
    //! `open_counter_window_from_effect_checked` before the declaration
    //! commits: the slot handle is still the attacker (as in
    //! `live_attack_attacker`), so the window opens naming the attacker's card
    //! -- never a stale `attack_attacker_card` from an earlier attack -- and a
    //! missing declaration context never consumes the window.

    use crate::debug_runner::DebugRunner;
    use crate::enums::GamePhase;
    use crate::selection::{AttackState, AttackTarget, PendingAttack};

    const ATK: &str = r#"
card: PRE-ATK
name: Pre Attacker
kind: digimon
level: 4
color: [red]
cost: 4
dp: 5000
"#;
    const COUNTER: &str = r#"
card: PRE-COUNTER
name: Pre Counter
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: counter
    summary: "[Counter] Gain 1 memory."
    process:
      - gain_memory: 1
"#;

    #[test]
    fn effect_opened_counter_window_before_declaration_names_the_attacker() {
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(ATK)
            .expect("attacker loads")
            .from_dsl_yaml(COUNTER)
            .expect("counter card loads")
            .start();
        let earlier = r.place_on_field(0, "PRE-ATK", Some(0));
        let atk = r.place_on_field(0, "PRE-ATK", Some(0));
        let _counter = r.place_on_field(1, "PRE-COUNTER", Some(0));
        let atk_card = r.game.player(0).battle_area[atk.index as usize]
            .top_card()
            .handle();
        // A stale declaration context from an earlier attack must not name
        // this window's source.
        let stale = r.game.player(0).battle_area[earlier.index as usize]
            .top_card()
            .handle();
        r.game.attack_attacker_card = Some(stale);
        r.game.pending_attack = Some(PendingAttack {
            attacker: atk,
            original_target: AttackTarget::Player(1),
            effective_target: AttackTarget::Player(1),
            effective_target_removed: false,
            is_blocked: false,
            blocker: None,
            is_vortex: false,
            is_overclock: false,
            declaration_committed: false,
            cancelled: false,
            battle_occurred: false,
            battle_defender_deleted: false,
            effect_battle_piercing_pending: false,
            security_check_started: false,
            return_phase: GamePhase::Main,
            state: AttackState::Declared,
            counter_depth: 0,
        });

        assert_eq!(r.game.open_counter_window_from_effect_checked(), Ok(true));
        let sel = r.game.pending_selection.as_ref().expect("the counter window");
        assert_eq!(r.game.current_phase, GamePhase::CounterTiming);
        assert_eq!(sel.selecting_player, 1);
        assert!(sel.is_optional, "the counter timing is declinable");
        assert_eq!(sel.source_card, atk_card, "names the attacker's card");
        assert_eq!(sel.source_permanent, Some(atk));
        assert_eq!(r.game.pending_attack.as_ref().map(|pa| pa.counter_depth), Some(1));
    }

    #[test]
    fn committed_attack_without_attacker_or_context_does_not_consume_the_window() {
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(ATK)
            .expect("attacker loads")
            .from_dsl_yaml(COUNTER)
            .expect("counter card loads")
            .start();
        let _counter = r.place_on_field(1, "PRE-COUNTER", Some(0));
        // The declared attacker's slot is empty and no declaration context
        // was captured: no card names the window, so it does not open and the
        // depth guard is untouched.
        r.game.attack_attacker_card = None;
        r.game.pending_attack = Some(PendingAttack {
            attacker: crate::permanent::PermanentHandle { player: 0, index: 0 },
            original_target: AttackTarget::Player(1),
            effective_target: AttackTarget::Player(1),
            effective_target_removed: false,
            is_blocked: false,
            blocker: None,
            is_vortex: false,
            is_overclock: false,
            declaration_committed: true,
            cancelled: false,
            battle_occurred: false,
            battle_defender_deleted: false,
            effect_battle_piercing_pending: false,
            security_check_started: false,
            return_phase: GamePhase::Main,
            state: AttackState::CounterOpen,
            counter_depth: 0,
        });

        assert_eq!(r.game.open_counter_window_from_effect_checked(), Ok(false));
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.pending_attack.as_ref().map(|pa| pa.counter_depth), Some(0));
    }
}
