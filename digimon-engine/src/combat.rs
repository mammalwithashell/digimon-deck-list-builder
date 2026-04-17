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

use crate::card_source::CardSource;
use crate::enums::{CardKind, GamePhase, Keyword, ModifierType, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::selection::{
    AttackState, AttackTarget, PendingAttack, PendingSelection, SelectionKind,
};

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

    /// Check whether a permanent can attack right now (atomic — ignores interrupts).
    ///
    /// `vortex` — pass `true` when the attack is invoked via the <Vortex>
    /// end-of-turn mechanic. Vortex exempts summoning sickness at the call
    /// site without adding a persistent keyword (mirrors Python's
    /// `Permanent.can_attack(is_vortex=True)` — see RUST_PYTHON_PARITY §2.1).
    pub fn can_attack(&self, handle: PermanentHandle, vortex: bool) -> bool {
        let perm = match self.player(handle.player).battle_area.get(handle.index as usize) {
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
        // Rush has been granted or this is a Vortex end-of-turn attack
        // (§2.1 parity fix). Native Rush from a card's static keyword list
        // is not yet checked — that requires the effect-listing
        // infrastructure (§2.1b / §4.5). For now, only modifier-granted
        // Rush exempts a permanent.
        let is_fresh = perm.turn_played == self.turn_count && perm.turn_digivolved == 0;
        if is_fresh && !vortex && !self.modifiers.has_keyword(handle, Keyword::Rush) {
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
        self.begin_attack_impl(attacker, target, /* vortex = */ false, /* is_overclock = */ true)
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
            return_phase,
            state: AttackState::Declared,
        });

        // Mark attacker as attacking (§2.2 parity).
        if let Some(perm) = self
            .player_mut(attacker.player)
            .battle_area
            .get_mut(attacker.index as usize)
        {
            perm.is_attacking = true;
        }

        // Suspend + record attack — skipped for Overclock, which attacks
        // without suspending.
        if !is_overclock {
            self.suspend_and_count_attack(attacker);
        }

        // Fire OnAttack (may install a PendingSelection via a triggered
        // effect; the drainer returns and we check below).
        self.fire_on_attack(attacker);

        // If OnAttack parked a selection, pause — advance_pending_attack
        // will fire again when that selection resolves and drain re-enters
        // this path.
        if self.pending_selection.is_some() {
            return AttackResult::InProgress;
        }

        // OnAttack may have deleted the attacker. Bail early.
        if !self.handle_valid(attacker) {
            return self.cleanup_attack(AttackResult::Invalid);
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

            // Attacker validity check: OnAttack / interrupt effects may
            // have deleted it. Any state past Declared requires a live
            // attacker (except Cleanup, which runs regardless).
            if state != AttackState::Cleanup && !self.handle_valid(attacker) {
                return self.cleanup_attack(AttackResult::Invalid);
            }

            match state {
                AttackState::Declared => {
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
                        self.transition_attack_state(AttackState::Battle);
                    }
                }
                AttackState::Battle => {
                    let outcome = self.resolve_pending_battle();
                    return self.cleanup_attack(outcome);
                }
                AttackState::Cleanup => {
                    // Shouldn't normally reach here — cleanup_attack is
                    // called on transition into Cleanup. Defensive exit.
                    return self.cleanup_attack(AttackResult::Invalid);
                }
            }
        }
    }

    // ─── State-machine helpers ────────────────────────────────────────

    fn transition_attack_state(&mut self, new_state: AttackState) {
        if let Some(pa) = self.pending_attack.as_mut() {
            pa.state = new_state;
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
            if !self.modifiers.has_keyword(h, Keyword::Alliance) {
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
                    ModifierEntry {
                        modifier: ModifierType::ChangeDp,
                        value: ally_dp,
                        expiry: Expiry::EndOfAttack,
                        source_player: attacker_player,
                    },
                );
                game.modifiers.add(
                    attacker,
                    ModifierEntry {
                        modifier: ModifierType::SecurityAttackChange,
                        value: 1,
                        expiry: Expiry::EndOfAttack,
                        source_player: attacker_player,
                    },
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

    /// Scan the defender's hand for blast-digivolve candidates paired
    /// against valid field targets; install a `PendingSelection` in
    /// `CounterTiming` if any pair exists. Returns `true` on install,
    /// `false` if the defender has no viable counter and the caller should
    /// auto-advance to BlockOpen.
    ///
    /// PR6 scope: **Digimon-target attacks only**. Player-target attacks
    /// skip Counter to match Python (`combat.py:139` scopes Counter to
    /// Digimon targets). Memory cost is not deducted — blast digivolve is
    /// always free during Counter per Python `_decode_counter` and DCGO
    /// `BlastDigivolution.cs:109`.
    ///
    /// A blast candidate is any hand card whose `CardEffect::effects`
    /// include one or more entries with `blast_digivolve = true`. Valid
    /// pairings are those where `Game::can_digivolve(card, perm)` passes
    /// for a field Digimon on the defender's side.
    fn try_enter_counter(&mut self) -> bool {
        use crate::action::space::encode_digivolve;

        let Some(pa) = self.pending_attack.as_ref() else {
            return false;
        };

        let defender_player = match pa.effective_target {
            AttackTarget::Digimon(h) => h.player,
            AttackTarget::Player(_) => return false,
        };
        let attacker = pa.attacker;

        let hand_len = self.player(defender_player).hand.len();
        let field_len = self.player(defender_player).battle_area.len();
        let mut valid_action_ids: Vec<u16> = Vec::new();

        for h_idx in 0..hand_len {
            let card = &self.player(defender_player).hand[h_idx];
            let card_id = card.card_id(&self.card_data).to_string();
            let card_handle = card.handle();
            let Some(effects) = self.effects_for_card(&card_id, card_handle) else {
                continue;
            };
            if !effects.iter().any(|e| e.blast_digivolve) {
                continue;
            }

            // Re-borrow with a fresh index into hand since effects_for_card
            // took only a local view.
            let card = &self.player(defender_player).hand[h_idx];
            for f_idx in 0..field_len {
                let perm = &self.player(defender_player).battle_area[f_idx];
                if !perm.is_digimon(&self.card_data) {
                    continue;
                }
                if !self.can_digivolve(card, perm) {
                    continue;
                }
                valid_action_ids.push(encode_digivolve(h_idx as u16, f_idx as u16));
            }
        }

        if valid_action_ids.is_empty() {
            return false;
        }

        let source_card = self.player(attacker.player).battle_area[attacker.index as usize]
            .top_card()
            .handle();
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::CounterTiming;

        self.pending_selection = Some(PendingSelection {
            // Hand kind is the primary resource (the hand card); the action
            // ID encodes (hand_idx, field_idx) via encode_digivolve.
            kind: SelectionKind::Hand,
            selecting_player: defender_player,
            previous_phase,
            valid_action_ids,
            is_optional: true,
            prompt: "Blast digivolve (counter)".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(attacker),
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                use crate::action::space::decode_digivolve;
                let (h_idx, f_idx) = decode_digivolve(action_id);
                game.execute_blast_digivolve(
                    defender_player,
                    h_idx as usize,
                    f_idx as usize,
                );

                // WhenDigivolving / OnDeletion cascades during the blast may
                // have deleted the attacker. If so, skip BlockOpen and
                // Battle — jump straight to Cleanup, matching DCGO
                // AttackProcess.cs:301's "attacker gone" branch.
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

    /// Perform the blast-digivolve card movement and fire the
    /// post-digivolve triggers. Called by the `try_enter_counter`
    /// selection callback once the defender picks a (hand, field) pair.
    ///
    /// Effects: move card from defender's hand to the target permanent's
    /// digivolution stack (zero memory), fire `WhenDigivolving` via the
    /// effect queue. No memory payment. `OnCounterTiming` (distinct from
    /// WhenDigivolving; fires before it in Python) is deferred — no pilot
    /// card needs it yet.
    fn execute_blast_digivolve(
        &mut self,
        defender: PlayerId,
        h_idx: usize,
        f_idx: usize,
    ) {
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
        let attacker_has_collision =
            self.modifiers.has_keyword(attacker, Keyword::Collision);

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
            // Blocker required UNLESS the attacker has Collision, which
            // grants Blocker to every opponent Digimon for this attack.
            if !attacker_has_collision
                && !self.modifiers.has_keyword(h, Keyword::Blocker)
            {
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
            is_optional: true, // Block is always a "may" — PASS means decline.
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
                    pa.state = AttackState::Battle;
                }
                game.advance_pending_attack();
            }),
            on_decline: Some(Box::new(move |game: &mut Game| {
                // Block declined — advance to Battle. Attack proceeds
                // against its original target.
                if let Some(pa) = game.pending_attack.as_mut() {
                    pa.state = AttackState::Battle;
                }
                game.advance_pending_attack();
            })),
        });

        true
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

    /// Security-check loop for a `Player` attack. Extracted from the old
    /// atomic `attack_player` so `resolve_pending_battle` can call it once
    /// the interrupt windows have cleared.
    fn resolve_player_security_loop(
        &mut self,
        attacker: PermanentHandle,
        defender_player: PlayerId,
    ) -> AttackResult {
        let sa_bonus = self
            .modifiers
            .sum(attacker, ModifierType::SecurityAttackChange);
        let checks = (1 + sa_bonus).max(0) as usize;

        let mut attacker_alive = true;
        for _ in 0..checks {
            if self.game_over {
                break;
            }
            if !self.handle_valid(attacker) {
                attacker_alive = false;
                break;
            }
            let sec_card = match self.player_mut(defender_player).security.pop() {
                Some(c) => c,
                None => {
                    let winner = attacker.player;
                    self.declare_winner(winner);
                    return AttackResult::GameWon;
                }
            };
            let outcome = self.resolve_security_card(attacker, sec_card, defender_player);
            match outcome {
                AttackResult::AttackerDeletedBySecurity => {
                    attacker_alive = false;
                    break;
                }
                AttackResult::GameWon => {
                    return AttackResult::GameWon;
                }
                _ => {}
            }
        }

        if !attacker_alive {
            AttackResult::AttackerDeletedBySecurity
        } else {
            AttackResult::SecurityCheckSurvived
        }
    }

    /// End-of-attack cleanup: clear `is_attacking`, expire EndOfAttack
    /// modifiers, drop `pending_attack`, and pass `outcome` through to the
    /// caller. Called from both `begin_attack` (early-exit paths) and
    /// `advance_pending_attack` (normal terminal paths).
    fn cleanup_attack(&mut self, outcome: AttackResult) -> AttackResult {
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
        self.modifiers.expire_end_of_attack();
        self.pending_attack = None;
        outcome
    }

    // ─── Private helpers ──────────────────────────────────────────────

    fn suspend_and_count_attack(&mut self, handle: PermanentHandle) {
        let perm = &mut self.players[handle.player as usize].battle_area
            [handle.index as usize];
        perm.is_suspended = true;
        perm.attacks_this_turn = perm.attacks_this_turn.saturating_add(1);
    }

    fn handle_valid(&self, handle: PermanentHandle) -> bool {
        self.player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|p| p.is_digimon(&self.card_data))
            .unwrap_or(false)
    }

    /// Fire OnAttack effects for the attacker.
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
    }

    /// Resolve battle between two permanents by DP comparison.
    fn resolve_battle(
        &mut self,
        attacker: PermanentHandle,
        defender: PermanentHandle,
    ) -> AttackResult {
        let a_dp = self.effective_dp(attacker).unwrap_or(0);
        let d_dp = self.effective_dp(defender).unwrap_or(0);

        if a_dp > d_dp {
            // Attacker wins — defender is deleted.
            self.delete_permanent_with_effects(defender);
            AttackResult::AttackerWins
        } else if a_dp < d_dp {
            // Defender wins — attacker is deleted.
            self.delete_permanent_with_effects(attacker);
            AttackResult::DefenderWins
        } else {
            // Tie — both are deleted. Delete in order: defender first to match
            // DCGO convention, but both need OnDeletion to fire.
            // Since the second deletion can shift indices, re-resolve via card_index
            // to be safe: we use the handles directly since delete_permanent_with_effects
            // reads the top card's card_index before deletion.
            self.delete_permanent_with_effects(defender);
            // After deleting defender, attacker's own handle index is unchanged
            // (different player's battle_area), so the attacker handle is still valid.
            if self.handle_valid(attacker) {
                self.delete_permanent_with_effects(attacker);
            }
            AttackResult::MutualDestruction
        }
    }

    /// Resolve a single security card being revealed.
    fn resolve_security_card(
        &mut self,
        attacker: PermanentHandle,
        sec_card: CardSource,
        defender_player: PlayerId,
    ) -> AttackResult {
        let kind = sec_card.card_kind(&self.card_data);
        match kind {
            CardKind::Digimon => {
                // Battle attacker vs the security Digimon.
                // Security Digimon has its base DP (no field modifiers).
                let attacker_dp = self.effective_dp(attacker).unwrap_or(0);
                let sec_dp = sec_card.dp(&self.card_data).unwrap_or(0);
                if attacker_dp >= sec_dp {
                    // Attacker wins or ties: security Digimon is trashed,
                    // attacker survives (ties against security favor the attacker
                    // in Digimon TCG — the security Digimon is trashed).
                    self.player_mut(defender_player).trash.push(sec_card);
                    AttackResult::SecurityCheckSurvived
                } else {
                    // Security Digimon wins: attacker is deleted, security
                    // Digimon is trashed (security is "consumed" either way).
                    self.player_mut(defender_player).trash.push(sec_card);
                    self.delete_permanent_with_effects(attacker);
                    AttackResult::AttackerDeletedBySecurity
                }
            }
            CardKind::Option | CardKind::Tamer => {
                // Security effects would fire here. For Phase 4 MVP, just trash.
                self.player_mut(defender_player).trash.push(sec_card);
                AttackResult::SecurityCheckSurvived
            }
            CardKind::DigiEgg => {
                // Eggs in security are just trashed (shouldn't happen normally).
                self.player_mut(defender_player).trash.push(sec_card);
                AttackResult::SecurityCheckSurvived
            }
        }
    }

    /// Delete a permanent, firing its OnDeletion effects first.
    /// Also clears any modifiers attached to the handle.
    ///
    /// OnDeletion effects are enqueued + drained before the actual deletion,
    /// so the effect closures can still observe the permanent on the field
    /// (matches the pre-drainer legacy ordering).
    pub fn delete_permanent_with_effects(&mut self, handle: PermanentHandle) {
        self.enqueue_triggered(
            crate::enums::EffectTiming::OnDeletion,
            crate::selection::TriggerSource::Permanent(handle),
        );
        self.drain_effect_queue();

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
    }

}
