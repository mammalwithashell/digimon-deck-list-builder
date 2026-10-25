//! Action mask building for the RL action space.
//!
//! Produces a `Vec<f32>` of size `ACTION_SPACE_SIZE` where 1.0 means legal,
//! 0.0 means illegal. Mask is phase-aware.
//!
//! Note: Phase 2 only implements basic Main/Breeding/Mulligan masking using
//! engine state available so far. Combat phases (BlockTiming, CounterTiming,
//! AllianceTiming) and effect-driven actions (effect activations, [Hand][Main],
//! DNA Digivolve, blast, etc.) are filled in later phases.

use crate::action::main_effect_select;
use crate::action::space::*;
use crate::card_data::CardData;
use crate::effect_context::{AttackTargetRestriction, EffectReadContext};
use crate::enums::{
    CardColor, CardKind, EffectSourceKind, EffectTiming, GamePhase, Keyword, ModifierType, PlayerId,
};
use crate::game::Game;
use crate::permanent::PermanentHandle;
// The mask is bounded by the action space, not the observation tensor —
// `action::space::MAX_FIELD_SLOTS` is the legal-action ceiling. Using it
// here decouples the mask from any specific tensor profile.
use crate::action::space::MAX_FIELD_SLOTS as FIELD_SLOTS_RAW;
const FIELD_SLOTS: usize = FIELD_SLOTS_RAW as usize;

pub(crate) fn evo_color(raw: u8) -> Option<CardColor> {
    // Mirrors `card_data::parse_card_color` — the raw ints come from
    // `cards.json::evo_costs[*].card_color`, which follows Python's
    // `CardColor` enum (see `digimon_gym/engine/data/enums.py` and
    // `tools/ingest_cards.py::COLOR_MAP`).
    match raw {
        0 => Some(CardColor::Red),
        1 => Some(CardColor::Blue),
        2 => Some(CardColor::Yellow),
        3 => Some(CardColor::Green),
        4 => Some(CardColor::White),
        5 => Some(CardColor::Black),
        6 => Some(CardColor::Purple),
        _ => None,
    }
}

/// Build an action mask of size `ACTION_SPACE_SIZE`.
/// Returns 1.0 for legal actions, 0.0 for illegal.
pub fn build_action_mask(game: &Game, player_id: PlayerId) -> Vec<f32> {
    let mut mask = vec![0.0f32; ACTION_SPACE_SIZE];
    let me = game.player(player_id);
    let opp_id = game.next_clockwise(player_id);
    let opp = game.player(opp_id);

    // A live PendingSelection is always the current decision, even when the
    // prompt's previous_phase is Main/Breeding and that phase has already been
    // restored. The mask must expose only the prompt's valid action ids.
    if let Some(sel) = &game.pending_selection {
        if sel.selecting_player == player_id {
            let valid = sel.card_choice.as_ref().map(|owner| owner.valid_action_ids(game));
            for &aid in valid.as_ref().unwrap_or(&sel.valid_action_ids) {
                if (aid as usize) < ACTION_SPACE_SIZE {
                    mask[aid as usize] = 1.0;
                }
            }
            if sel.is_optional && sel.card_choice.is_none() {
                mask[PASS as usize] = 1.0;
            }
            // CONCEDE_GAME (93) is intentionally NOT exposed in the action
            // mask (disabled in all match formats, 2026-06-19). Conceding is
            // a strictly-dominated "give up" with no strategic value — in
            // single-game format it just forfeits the episode, and even in
            // BO3 the agent can always play a game to its natural end. RL
            // policies degenerately picked it (premature surrender of even/
            // winnable boards), so it's removed from the learnable action
            // space. The `Game::concede` PRIMITIVE is unaffected — human PvP
            // surrender routes through it directly via the server's
            // `/surrender` endpoint + WS "surrender" message, not via the mask.
        }
        return mask;
    }

    match game.current_phase {
        GamePhase::Mulligan => {
            // Mulligan is sequential: only the currently-deciding player has
            // a non-empty mask. Everyone else sees all zeros.
            if game.mulligan_current_player() != Some(player_id) {
                return mask;
            }
            // Bit 0 = keep (always available for the decider).
            mask[0] = 1.0;
            // Bit 1 = mulligan (one per player). Suppress if already used.
            if !game.mulligan_used[player_id as usize] {
                mask[1] = 1.0;
            }
        }

        GamePhase::Main => {
            // --- Play cards (0-29) ---
            let max_hand = (me.hand.len() as u16).min(PLAY_HAND_END);
            // §4.7c CANNOT_PLAY_FROM_HAND — any active modifier of this
            // type (anywhere in the registry) suppresses every hand-play
            // bit. Python's context discriminant (the specific `card`
            // argument) isn't carried in Rust — tracked as §4.7x.
            let play_blocked = game
                .modifiers
                .player_has(player_id, ModifierType::CannotPlayFromHand)
                || game
                    .modifiers
                    .any_with_type(ModifierType::CannotPlayFromHand);
            for i in 0..max_hand as usize {
                if play_blocked {
                    continue;
                }
                let card = &me.hand[i];
                // Digi-Eggs cannot be played. Even a malformed imported state
                // must not advertise a hand-play action execution will reject.
                if card.card_kind(&game.card_data) == CardKind::DigiEgg {
                    continue;
                }
                let is_option_use = card.card_kind(&game.card_data) == CardKind::Option
                    || card.card_kind(&game.card_data) == CardKind::Dual;
                if is_option_use {
                    // §4.2 Option color requirement: an Option is playable
                    // when the player has a matching-color Digimon/Tamer,
                    // or when a printed Use Req. predicate satisfies it.
                    if !option_has_active_main_effect(card, game, player_id) {
                        continue;
                    }
                    if !option_use_requirement_or_color_available(card, game, player_id) {
                        continue;
                    }
                    // Affordability is folded into the legal-mode set: a
                    // dual-mode Plug-In Option is playable when EITHER its
                    // Standard `[Main]` mode or its Link mode fits the
                    // memory budget. An empty set means no mode is
                    // affordable right now.
                    if game
                        .option_legal_play_modes(
                            card,
                            player_id,
                            crate::game_actions::OptionSource::Hand(i),
                        )
                        .is_empty()
                    {
                        continue;
                    }
                } else {
                    if game.card_play_is_prohibited(player_id, card) {
                        continue;
                    }
                    // Memory check: a non-Option card is affordable if
                    // memory - cost >= memory_min. An eligible `[Assembly]`
                    // alt-path (materials present in trash) reduces the cost,
                    // so legality is computed against the REDUCED cost
                    // (declare-then-pay — G-ASSEMBLY-PLAY-EXECUTION / judge
                    // quiz Q5). 0 when the card has no eligible assembly path.
                    let printed = card.play_cost(&game.card_data) as i16;
                    let assigned = game
                        .available_play_assignment_base(
                            player_id,
                            card.handle(),
                            true,
                            printed as u16,
                        )
                        .map(i32::from)
                        .unwrap_or(printed as i32);
                    let assembly_reduction =
                        game.assembly_play_reduction_for_hand_card(player_id, i) as i16;
                    // Cast-time assembly (BT15-102): like `[Assembly]`, the
                    // potential place-under reduction participates in
                    // declare-then-pay legality (DCGO's hidden availability
                    // ChangeCostClass). 0 for every other card.
                    let cast_time_reduction =
                        game.cast_time_assembly_play_reduction_for_hand_card(player_id, i) as i16;
                    let cost =
                        (assigned - assembly_reduction as i32 - cast_time_reduction as i32).max(0);
                    if (game.memory as i32 - cost) < game.rules.memory_range.0 as i32 {
                        continue;
                    }
                    if me.battle_area.len() >= game.rules.field_slots as usize {
                        continue;
                    }
                }
                mask[i] = 1.0;
            }

            // --- Attack (100-399) ---
            // §4.7e CannotAttack (player-scoped) — zero ALL attack bits for
            // this player. Check before entering the per-attacker loop.
            // Memory gate: a Main-phase attack needs the memory on this
            // player's side. There is no <Blitz> carve-out: <Blitz> offers its
            // attack as its own decision inside the effect that executes it
            // (§16-16-2/-3, `keyword_effects::blitz_process` →
            // `ResumeFrame::AttackDeclarationDecision`), never through the Main-phase
            // mask, so a Digimon that merely has <Blitz> cannot attack with the
            // memory on the opponent's side.
            let attack_blocked = game
                .modifiers
                .player_has(player_id, ModifierType::CannotAttack);
            let max_field = me.battle_area.len().min(FIELD_SLOTS);
            for i in 0..max_field {
                if attack_blocked {
                    continue;
                }
                let attacker = &me.battle_area[i];
                let handle = PermanentHandle {
                    player: player_id,
                    index: i as u8,
                };
                if !can_basic_attack(attacker, handle, game.turn_count, &game.card_data, game) {
                    continue;
                }
                if game.memory < 0 {
                    continue;
                }

                if !game.modifiers.has(handle, ModifierType::CannotAttackPlayer) {
                    let sec_action = encode_attack(i as u16, SECURITY_TARGET);
                    mask[sec_action as usize] = 1.0;
                }

                // Digimon attacks: the declaration targets the opponent or a
                // suspended Digimon (§11-2-7-1), or an unsuspended one when
                // the attacker carries a CAN_ATTACK_UNSUSPENDED grant
                // (§16-38-1 <Execute>; DCGO `Permanent.CanAttackTargetDigimon`).
                // <Raid> is NOT a declaration-time permission: it switches the
                // target after the declaration (§16-23-1 / §16-23-2), as a
                // [When Attacking] trigger in the attack's pool
                // (`Game::raid_switch_options`).
                let can_attack_unsuspended = game
                    .modifiers
                    .has(handle, ModifierType::CanAttackUnsuspended);
                let max_opp = opp.battle_area.len().min(FIELD_SLOTS);

                for j in 0..max_opp {
                    let t_handle = PermanentHandle {
                        player: opp_id,
                        index: j as u8,
                    };
                    let target = &opp.battle_area[j];
                    if !game.permanent_is_digimon_for_rules(t_handle) {
                        continue;
                    }
                    // §4.7a CANNOT_ATTACK_TARGET — suppress this target if
                    // it carries the modifier. Per-attacker discriminant
                    // from Python is §4.7x. Track C / D (2026-05-08):
                    // `CanAttackTargetDefendingPermanent` overrides this
                    // gate so the affirmative form remains visible to
                    // both mask emission and the action decode path.
                    if game.attack_target_blocked_by_modifier(handle, t_handle) {
                        continue;
                    }
                    let action_bit = encode_attack(i as u16, j as u16) as usize;
                    if target.is_suspended {
                        mask[action_bit] = 1.0;
                        continue;
                    }
                    if can_attack_unsuspended {
                        mask[action_bit] = 1.0;
                    }
                }
            }

            // --- Digivolve (400-999) ---
            // Basic check: card in hand is Digimon and matching evo_costs.
            // Full digivolve validation (alt-digi, modifiers) deferred.
            for h in 0..max_hand as usize {
                let card = &me.hand[h];
                if !card.is_digimon_card_for_search(&game.card_data) {
                    continue;
                }
                let max_field = me.battle_area.len().min(FIELD_SLOTS);
                for f in 0..max_field {
                    let base_handle = PermanentHandle {
                        player: player_id,
                        index: f as u8,
                    };
                    // §4.7b CANNOT_DIGIVOLVE — suppress the bit if the
                    // base permanent carries an active CannotDigivolve
                    // modifier. Python's `{'digivolving_card': card}`
                    // discriminant is not carried in Rust (§4.7x).
                    if game
                        .modifiers
                        .has(base_handle, ModifierType::CannotDigivolve)
                    {
                        continue;
                    }
                    if game
                        .normal_digivolve_route_for_hand_card(player_id, h, base_handle)
                        .is_some()
                    {
                        mask[encode_digivolve(h as u16, f as u16) as usize] = 1.0;
                    }
                }
                // Breeding-area digivolve — consult the SAME route enumeration
                // as the battle area (printed circles + DSL alt-digivolve
                // paths) via the `BREEDING_TARGET` sentinel handle, so printed
                // "[Digivolve] [Name]: Cost N" alt evo boxes (e.g. EX11-014
                // Penguinmon over a breeding Hiyarimon) are honored.
                if me.breeding_area.is_some() {
                    let breeding_handle = PermanentHandle {
                        player: player_id,
                        index: BREEDING_TARGET as u8,
                    };
                    if game
                        .normal_digivolve_route_for_hand_card(player_id, h, breeding_handle)
                        .is_some()
                    {
                        mask[encode_digivolve(h as u16, BREEDING_TARGET) as usize] = 1.0;
                    }
                }
            }

            // --- DNA Digivolve (63-92) --- §4.5 slice.
            // A hand Digimon with non-empty dna_costs is playable if some
            // pair of permanents in battle_area satisfies any of its DnaCost
            // entries (either ordering). Python's mask does NOT gate on
            // memory (action_mask.py:161-166) — the memory check runs at
            // action-execution time, not mask generation. Mirror that here.
            // Data population (cards.json ingest of dna_costs) is §4.5b.
            for h in 0..max_hand as usize {
                if game.has_valid_dna_route_for_hand_card(player_id, h) {
                    mask[(DNA_DIGIVOLVE_START + h as u16) as usize] = 1.0;
                }
            }

            // --- [Main] activated effects (§4.5c) ---
            //
            // Python iterates each card's effect list and filters by the
            // `_is_{hand,field,trash}_main` bool flag. Rust promotes the
            // zone distinction into the timing enum itself, so filtering
            // is a single `effect.timing ==` check.
            //
            // OPT enforcement:
            //   * Field — use the existing per-permanent
            //     `activation_count((handle, slot))` map (already populated
            //     by effect firing at runtime).
            //   * Hand / Trash — Python's mask does NOT check
            //     `_turn_activate_count` either; the effect's
            //     `can_use_condition` closure is responsible. Mirror that
            //     here. Adding explicit hand/trash activation maps is a
            //     follow-up tracked in §4.5c residuals.

            // Hand [Main] (bits 30-59). One bit per hand slot, mirroring
            // Python's `action_mask.py:176-185`. First-match-wins per slot.
            // Selection is shared with the decoder via `main_effect_select` so
            // the emitted bit and `explain_action`'s label never disagree on
            // which effect this activation is.
            let hand_limit = me.hand.len().min(HAND_MAIN_LIMIT);
            for h in 0..hand_limit {
                if main_effect_select::hand_main_match(game, player_id, h).is_some() {
                    mask[(HAND_EFFECT_START + h as u16) as usize] = 1.0;
                    continue;
                }
                // DigiLink Shape-B, hand origin: an Appmon Link Digimon in hand
                // may declare its `<Link>` ("Plug this card from the hand ...")
                // onto one of the controller's standing Digimon. Shares the
                // slot's single `HAND_EFFECT` bit with a `[Hand] [Main]`
                // effect, first-match-wins in the decoder's order ([Main] then
                // link) — the same one-declarable-per-hand-slot limit DCGO's
                // recorder has (`ActionEncoder.EncodeActivateCard`, skill 0).
                // Host selection follows via the pending-selection mask.
                // Either half of the §6-5-1-4 declaration: a Link Digimon, or
                // a Plug-In Option being plugged in from hand (which is NOT
                // the §6-5-1-3 "use an Option card from the hand" on the PLAY
                // bit — two different main-phase actions).
                if game.hand_link_available(player_id, h) {
                    mask[(HAND_EFFECT_START + h as u16) as usize] = 1.0;
                }
            }

            // Field [Main] (bits 1000-1149). Python emits one bit per
            // permanent at sub-slot `+2` (FIELD_EFFECT_SLOT_FOR_MAIN),
            // first-match-wins across the entire digivolution stack.
            // Inherited-vs-top filter matches `source_dp_contribution`.
            //
            // §4.7f CannotActivateMainEffects (player-scoped) — zero ALL
            // FIELD_EFFECT bits for this player when the modifier is active.
            let main_effects_blocked = game
                .modifiers
                .player_has(player_id, ModifierType::CannotActivateMainEffects);
            let field_limit = me.battle_area.len().min(FIELD_SLOTS);
            for i in 0..field_limit {
                if main_effects_blocked {
                    continue;
                }
                // The standard field [Main] selection now lives in
                // `main_effect_select::field_main_match`; only `perm_handle` is
                // needed here for the DigiLink / delayed-Option emitters.
                let perm_handle = PermanentHandle {
                    player: player_id,
                    index: i as u8,
                };

                // DigiLink Shape-B: an un-linked standing Appmon Link Digimon
                // may activate its [Main] Link ability (sub-slot 3) to attach
                // itself onto one of the controller's other Digimon. Coexists
                // with a [Main] (sub-slot 2) bit on the same permanent. No
                // once-per-turn map is needed — a linked source is absorbed and
                // can never re-offer the ability.
                if let Some((link_cost, link_hosts)) =
                    game.digimon_link_condition_targets(perm_handle)
                {
                    let affordable =
                        (game.memory as i32 - link_cost as i32) >= game.rules.memory_range.0 as i32;
                    if !link_hosts.is_empty() && affordable {
                        let bit = FIELD_EFFECT_START
                            + i as u16 * EFFECTS_PER_PERMANENT
                            + FIELD_EFFECT_SLOT_FOR_LINK;
                        mask[bit as usize] = 1.0;
                    }
                }

                // PUPPETS-G009 — standard `<Delay>` `[Main]`-phase activation.
                // A parked `DelayTrigger::MainPhaseActivated` Option whose
                // placing turn has passed exposes a player-visible
                // FIELD_EFFECT activation (slot `+2`) — "By trashing this
                // card after the placing turn, activate the effect below"
                // (RULES_CONTEXT 16-17). The choice is optional, so PASS
                // remains legal alongside it (emitted below).
                if game.delayed_option_main_activation_available(perm_handle) {
                    let bit = FIELD_EFFECT_START
                        + i as u16 * EFFECTS_PER_PERMANENT
                        + FIELD_EFFECT_SLOT_FOR_MAIN;
                    mask[bit as usize] = 1.0;
                    continue;
                }

                // Standard field [Main] selection is shared with the decoder
                // (first-match-wins across the stack, inherited/OPT/condition
                // gates) so the emitted bit and `explain_action`'s label always
                // refer to the same effect.
                if main_effect_select::field_main_match(game, player_id, i).is_some() {
                    let bit = FIELD_EFFECT_START
                        + i as u16 * EFFECTS_PER_PERMANENT
                        + FIELD_EFFECT_SLOT_FOR_MAIN;
                    mask[bit as usize] = 1.0;
                }
            }

            // Phase F Task 6 — Training-only breeding-area `[Main]` emitter.
            //
            // Surfaced at field index `BREEDING_TARGET (=14)`, sub-slot
            // `FIELD_EFFECT_SLOT_FOR_MAIN (=2)` → action_id 1142. This bit
            // is reserved for the breeding-area Training activation; no
            // other `MainOnField` effect surfaces from breeding (RULES_CONTEXT
            // 16-41 specifies that ONLY `<Training>` activates from breeding
            // — surfacing all `MainOnField` effects from breeding would
            // inadvertently expose MindLink from breeding too, which is
            // wrong).
            //
            // Gate: same `CannotActivateMainEffects` short-circuit as the
            // battle-area emitter, then check that the breeding-area
            // permanent's top card carries `Keyword::Training` AND its
            // `MainOnField` `<Training>` auto-effect's condition passes
            // (`!perm.is_suspended`). Inherited Training (a Training source
            // under a non-Training top) is intentionally not surfaced —
            // matches DCGO `Training.cs:21` `card.PermanentOfThisCard()`
            // which scopes to the top card's effect.
            // Gate is the printed `Keyword::Training` + non-suspension of the
            // breeding-area top card (see `main_effect_select::breeding_training_match`).
            // Shared with the decoder so action 1142's label names the
            // `<Training>` effect. We deliberately don't run the effect's own
            // condition closure (it resolves via `battle_area` and would
            // short-circuit for a breeding-area carrier); the on-field
            // condition (`!is_suspended`) is represented by the gate.
            if !main_effects_blocked
                && main_effect_select::breeding_training_match(game, player_id).is_some()
            {
                let bit = FIELD_EFFECT_START
                    + BREEDING_TARGET * EFFECTS_PER_PERMANENT
                    + FIELD_EFFECT_SLOT_FOR_MAIN;
                mask[bit as usize] = 1.0;
            }

            // Trash [Main] (bits 1150-1194). One bit per trash slot,
            // first-match-wins, mirroring `action_mask.py:216-225`. Selection
            // shared with the decoder via `main_effect_select`.
            let trash_limit = me.trash.len().min(TRASH_MAIN_LIMIT);
            for t in 0..trash_limit {
                if main_effect_select::trash_main_match(game, player_id, t).is_some() {
                    mask[(TRASH_EFFECT_START + t as u16) as usize] = 1.0;
                }
            }

            // --- Pass (62) ---
            mask[PASS as usize] = 1.0;

            // §4.7d FORCE_ATTACK global mask replacement. If any friendly
            // Digimon has the ForceAttack modifier AND at least one legal
            // attack is available, zero every other bit and retain only
            // the forced attacker(s)' attack bits. Mirrors Python
            // action_mask.py:227-280. Falls through to the normal mask
            // above when no forced attacker can act (e.g. all suspended).
            apply_force_attack_mask_replacement(&mut mask, game, player_id);
        }

        GamePhase::Breeding => {
            // Hatch (60)
            if me.breeding_area.is_none() && !me.digitama_deck.is_empty() {
                mask[HATCH as usize] = 1.0;
            }
            // Rule 4-17-2: a breeding Digimon may move if it has DP.
            // Level-less Digi-Eggs such as Mother D-Reaper qualify (Q3276).
            if let Some(ref perm) = me.breeding_area {
                if perm.top_card().digimon_dp(&game.card_data).is_some()
                    && me.battle_area.len() < game.rules.field_slots as usize
                {
                    mask[MOVE_FROM_BREEDING as usize] = 1.0;
                }
            }
            // Pass (62)
            mask[PASS as usize] = 1.0;
        }

        GamePhase::EndOfTurnAction => {
            // Decline end-of-turn action — always legal. Matches Python's
            // `action_mask.py` EndOfTurnAction branch where PASS (62) is the
            // sole exit even when FORCE_ATTACK is present (execution-side
            // enforcement; the mask doesn't hide PASS).
            mask[PASS as usize] = 1.0;

            // §4.7e CannotAttack (player-scoped) — zero ALL attack bits for
            // this player, including the end-of-turn Vortex / MayAttack /
            // ForceAttack window. Rules judgment: CannotAttack overrides
            // ForceAttack; a "cannot attack" effect always wins.
            let attack_blocked = game
                .modifiers
                .player_has(player_id, ModifierType::CannotAttack);

            let max_field = me.battle_area.len().min(FIELD_SLOTS);
            let max_opp = opp.battle_area.len().min(FIELD_SLOTS);
            let max_hand = (me.hand.len() as u16).min(DNA_DIGIVOLVE_END - DNA_DIGIVOLVE_START);

            for h in 0..max_hand as usize {
                if game.has_valid_dna_route_for_hand_card(player_id, h) {
                    mask[(DNA_DIGIVOLVE_START + h as u16) as usize] = 1.0;
                }
            }

            for i in 0..max_field {
                let handle = PermanentHandle {
                    player: player_id,
                    index: i as u8,
                };

                // No `<Overclock>` field-effect bit and no `<Vortex>` /
                // `<Engage>` / `<Execute>` attack bits here: those keywords are
                // trigger-type effects whose decision is offered inside the
                // `[End of Your Turn]` pool, as a pending selection (§16-33-2,
                // §16-34-2, §16-38-2, §16-44-2).

                // §4.7e CannotAttack gate — skip all attack-bit emission
                // for this permanent when the player-scoped modifier is set.
                if attack_blocked {
                    continue;
                }
                // Permanent-scoped CannotAttack: a declaration would be
                // Cancelled by the `WhenWouldAttack` replacement, so never
                // offer it (mirror of `can_basic_attack`'s Main-phase gate).
                // Without this the end-of-turn (Vortex / MayAttack /
                // ForceAttack) range could surface a guaranteed-cancel no-op.
                if game.modifiers.has(handle, ModifierType::CannotAttack) {
                    continue;
                }

                // §4.6 attack bits: MayAttack / ForceAttack share the 100-399
                // attack range and use the same suspended /
                // CanAttackUnsuspended target legality as Main-phase attacks
                // (§11-2-7-1); <Raid> switches the target after the
                // declaration (§16-23-1), not before.
                let may_attack = game.modifiers.has(handle, ModifierType::MayAttack);
                let force_attack = game.modifiers.has(handle, ModifierType::ForceAttack);
                if !may_attack && !force_attack {
                    continue;
                }
                if !game.can_attack(handle, /* vortex = */ false) {
                    continue;
                }

                if !game.modifiers.has(handle, ModifierType::CannotAttackPlayer) {
                    mask[encode_attack(i as u16, SECURITY_TARGET) as usize] = 1.0;
                }

                let can_attack_unsuspended = game
                    .modifiers
                    .has(handle, ModifierType::CanAttackUnsuspended);

                for j in 0..max_opp {
                    let t_handle = PermanentHandle {
                        player: opp_id,
                        index: j as u8,
                    };
                    if !game.permanent_is_digimon_for_rules(t_handle) {
                        continue;
                    }
                    // §4.7a CANNOT_ATTACK_TARGET — suppress attacks against
                    // a target carrying the modifier, regardless of which
                    // keyword granted the attack. Track C / D (2026-05-08):
                    // `CanAttackTargetDefendingPermanent` is the
                    // affirmative override; when set, the mask must
                    // continue emitting the bit so the granted attack
                    // remains usable.
                    if game.attack_target_blocked_by_modifier(handle, t_handle) {
                        continue;
                    }
                    // Either grant (checked above) attacks only a legal
                    // target: suspended, or one this attacker may attack
                    // unsuspended (§11-2-7-1).
                    let target_legal =
                        game.player(opp_id).battle_area[j].is_suspended || can_attack_unsuspended;
                    if !target_legal {
                        continue;
                    }
                    mask[encode_attack(i as u16, j as u16) as usize] = 1.0;
                }
            }
        }

        // Selection phases: SelectTarget, SelectHand, SelectTrash, ...,
        // and TriggerOrder (which parks in EffectChoice). Emit only the
        // exact action IDs the pending selection considers valid — plus
        // PASS when the selection is optional.
        //
        // Non-selecting players see an empty mask (modulo the soft-lock
        // safety PASS below) — they have no legal action while another
        // player is answering. Combat sub-phases (BlockTiming,
        // CounterTiming, AllianceTiming) work the same way: a
        // PendingSelection is installed during those phases too (PR4+),
        // and this branch renders it.
        phase
            if phase.is_selection_phase()
                || phase == GamePhase::BlockTiming
                || phase == GamePhase::CounterTiming
                || phase == GamePhase::AllianceTiming =>
        {
            if let Some(sel) = &game.pending_selection {
                if sel.selecting_player == player_id {
                    for &aid in &sel.valid_action_ids {
                        if (aid as usize) < ACTION_SPACE_SIZE {
                            mask[aid as usize] = 1.0;
                        }
                    }
                    if sel.is_optional {
                        mask[PASS as usize] = 1.0;
                    }
                }
                // Non-selecting players see zeros — nothing to do.
            } else {
                // In a selection phase with no pending_selection installed
                // is a transient state we shouldn't normally hit; allow
                // PASS as a soft-lock safety rail rather than returning
                // an all-zero mask.
                mask[PASS as usize] = 1.0;
            }
        }

        _ => {
            // Phases we haven't wired a mask for (EndOfTurnAction is
            // handled above). PASS-only keeps the engine from soft-locking.
            mask[PASS as usize] = 1.0;
        }
    }

    // CONCEDE_GAME (93) is intentionally NOT exposed in the action mask
    // (disabled in all match formats, 2026-06-19 — see the matching note in
    // the pending-selection branch above). It is a strictly-dominated "give
    // up" action with no strategic value (single: forfeits the episode; BO3:
    // a game can always be played to its natural end), and RL policies abused
    // it as premature surrender. Removed from the learnable action space. The
    // `Game::concede` primitive still backs human PvP surrender via the
    // server, which does not go through this mask.

    mask
}

// ─── Helpers ──────────────────────────────────────────────────────────

/// 4-22 — can this player meet an Option card's COLOR REQUIREMENTS?
///
/// 4-22-2: "To meet color requirements, you must have a Digimon or Tamer on
/// your field that's the same color as the Option card you want to use."
/// 4-22-3: "An Option card with multiple colors can't be used unless the color
/// requirements are met for ALL of its colors."
/// 4-22-4: "A multicolor Digimon or multicolor Tamer can meet the color
/// requirements for multiple colors."
///
/// So the test is: for EVERY color the Option prints, SOME qualifying permanent
/// carries that color. Different colors may be covered by different permanents,
/// and one multicolor permanent may cover several (4-22-4) -- hence a per-color
/// existence check rather than any per-permanent test.
///
/// This used to ask whether any SINGLE permanent shared any ONE of the Option's
/// colors, which let a mono-color board pay for a multicolor Option -- 4-22-3
/// says it cannot. 63 Option cards in data/cards.json print more than one
/// color, so this was reachable, not theoretical. DCGO agrees with the manual:
/// CardSource.cs:307-310 uses `colorsToCheck.Every(...)`.
///
/// BREEDING AREA COUNTS, and this is SETTLED -- all three sources agree.
/// 3-4-6: "The field is divided into the breeding area and the battle area",
/// so a breeding Digimon is literally "on your field" as 4-22-2 words it. The
/// restrictions on breeding cards (3-4-7-3 "can't be affected by effects",
/// 3-4-7-5 "can't be chosen for effects") are scoped to EFFECTS; a color
/// requirement is a rules gate on USING a card, not an effect targeting the
/// breeding Digimon.
///
/// DCGO agrees, contrary to a review claim that it checked the battle area
/// only: `CardSource.cs:308-310` iterates `Owner.GetFieldPermanents()`, and
/// `Player.cs:669-685` returns the WHOLE `FieldPermanents` array unfiltered --
/// it is `GetBattleAreaPermanents` (`Player.cs:621-637`) that narrows with
/// `FieldCardFrame.isBattleAreaFrameID(i)`. Confirmed independently by the
/// repo owner. Pinned by
/// `a_breeding_area_digimon_satisfies_the_color_requirement`; do NOT "align to
/// DCGO" by dropping the breeding branch -- that would break all three.
///
/// Player-scoped `IgnoreColorRequirement` is consumed by
/// `option_use_requirement_or_color_available` before this helper runs.
pub(crate) fn option_color_match_available(
    card: &crate::card_source::CardSource,
    me: &crate::player::Player,
    card_data: &[crate::card_data::CardData],
) -> bool {
    let option_colors = card.option_colors(card_data);

    // A colorless Option has nothing to satisfy.
    if option_colors.is_empty() {
        return true;
    }

    // Every color a qualifying permanent contributes. Collected once, because
    // 4-22-4 lets a single multicolor permanent cover several of the Option's
    // colors and lets different colors come from different permanents.
    let mut covered: Vec<crate::enums::CardColor> = Vec::new();
    for perm in &me.battle_area {
        if !perm.is_digimon(card_data) && !perm.is_tamer(card_data) {
            continue;
        }
        let perm_colors = if perm.is_digimon(card_data) {
            perm.top_card().digimon_colors(card_data)
        } else {
            perm.top_card().colors(card_data)
        };
        covered.extend(perm_colors);
    }
    // Breeding area: only Digimon can be there (Tamers cannot). See the
    // breeding note above -- 3-4-6 puts the breeding area inside the field.
    if let Some(ref breeding) = me.breeding_area {
        // A hatched Digi-Egg is a Digimon on the field (rules 4-3-1/2),
        // and therefore supplies its color for an Option under 4-22-2.
        if breeding.is_digimon(card_data) || breeding.is_digi_egg(card_data) {
            covered.extend(breeding.top_card().digimon_colors(card_data));
        }
    }

    // 4-22-3: ALL of the Option's colors, not any.
    option_colors.iter().all(|c| covered.contains(c))
}

/// Option-use legality for the color requirement. A true printed Use Req.
/// predicate or player-scoped `IgnoreColorRequirement` satisfies the
/// requirement; otherwise normal color matching still applies.
pub(crate) fn option_use_requirement_or_color_available(
    card: &crate::card_source::CardSource,
    game: &Game,
    player_id: PlayerId,
) -> bool {
    if game
        .modifiers
        .player_has(player_id, ModifierType::IgnoreColorRequirement)
    {
        return true;
    }

    if option_color_match_available(card, game.player(player_id), &game.card_data) {
        return true;
    }

    let card_id = card.card_id(&game.card_data);
    let Some(effects) = game.effects_for_card(card_id, card.handle()) else {
        return false;
    };
    if effects.is_empty() {
        return false;
    }
    let ctx = EffectReadContext::new(game, card.handle(), None, player_id);
    effects.iter().any(|effect| {
        if !matches!(
            effect.timing,
            EffectTiming::MainFromHand | EffectTiming::OptionMain
        ) {
            return false;
        }
        effect
            .option_color_requirement_bypass
            .as_ref()
            .is_some_and(|condition| condition(&ctx))
    })
}

/// An Option hand/trash use must have an authored `OptionMain` body. This
/// prevents partial Security-only YAML from becoming a legal no-effect hand
/// play (an implementation-completeness guard, not a rules gate).
///
/// Deliberately does NOT evaluate the effect's `condition`: Option use
/// legality is phase + cost payable (rule 1-3-11-1) + the color requirement —
/// never the availability of effect targets. Effects are performed "whenever
/// possible" (15-1-5); with no eligible target the `[Main]` body simply does
/// nothing and the card is still used and trashed. DCGO agrees: its generic
/// `CanTriggerOptionMainEffect` gate is identity plumbing only, and each
/// card's target check lives INSIDE `ActivateCoroutine`
/// (`HasMatchConditionPermanent`). Card-level `condition:` clauses on
/// `main_from_hand` remain honored at RESOLUTION time — the effect-queue
/// drainer (`run_queued_effect_inner`) skips a false-condition body silently
/// — so they still guard mandatory selects against zero candidates without
/// over-restricting playability (the "can't use options if there aren't any
/// targets" regression, observed on ST1-15).
pub(crate) fn option_has_active_main_effect(
    card: &crate::card_source::CardSource,
    game: &Game,
    _player_id: PlayerId,
) -> bool {
    let card_id = card.card_id(&game.card_data);
    let Some(effects) = game.effects_for_card(card_id, card.handle()) else {
        return true;
    };
    if effects.is_empty() {
        return true;
    }
    effects.iter().any(|effect| {
        effect.delay_trigger.is_some()
            || matches!(
                effect.timing,
                EffectTiming::OptionMain | EffectTiming::MainFromHand
            )
    })
}

/// Counter-window Option uses are legal through a `CounterEffect` body even
/// when the card has no ordinary `OptionMain` body.
pub(crate) fn option_has_active_counter_effect(
    card: &crate::card_source::CardSource,
    game: &Game,
    player_id: PlayerId,
) -> bool {
    // G-ENGINE-CANNOT-USE-OPTION-CARDS: a hand Counter-timing Option is a use.
    if game.option_use_blocked(player_id) {
        return false;
    }
    let card_id = card.card_id(&game.card_data);
    let Some(effects) = game.effects_for_card(card_id, card.handle()) else {
        return false;
    };
    let ctx = EffectReadContext::new_with_source_kind(
        game,
        card.handle(),
        None,
        EffectSourceKind::Option,
        player_id,
    );
    effects.iter().any(|effect| {
        if effect.timing != EffectTiming::CounterEffect || !effect.counter {
            return false;
        }
        effect
            .condition
            .as_ref()
            .is_none_or(|condition| condition(&ctx))
    })
}

/// Basic attack eligibility: unsuspended Digimon not played this turn,
/// unless Rush (native printed OR modifier-granted) exempts summoning sickness.
///
/// Vortex is not checked here — Vortex attacks belong to `EndOfTurnAction`
/// phase mask generation (§4.6), not the Main-phase attack range.
fn can_basic_attack(
    perm: &crate::permanent::Permanent,
    handle: PermanentHandle,
    turn: u16,
    card_data: &[CardData],
    game: &Game,
) -> bool {
    if perm.is_suspended {
        return false;
    }
    if !perm.is_digimon_for_rules(card_data, &game.modifiers, handle) {
        return false;
    }
    // 11-2-5: "An attack declaration can't be made using a Digimon that
    // can't suspend." Mask-side mirror of the `Game::can_attack*` consult
    // (mask and engine API must agree).
    if game.modifiers.has(handle, ModifierType::CannotSuspend) {
        return false;
    }
    // Permanent-scoped CannotAttack: a Digimon carrying the CannotAttack
    // modifier can't declare an attack. The player-scoped form is zeroed
    // before the per-attacker loop (the `attack_blocked` short-circuit);
    // this covers the permanent-scoped form that the loop would otherwise
    // miss. Mask-side mirror of the `Game::can_attack*` consult — without
    // it the mask offers an attack the engine refuses, and a deterministic
    // policy re-picks it forever (the no-op attack loop).
    if game.modifiers.has(handle, ModifierType::CannotAttack) {
        return false;
    }
    // Summoning sickness: can't attack the turn it was played unless Rush
    // is present (native printed OR modifier-granted) — §2.1b.
    let is_fresh = perm.turn_played == turn;
    if is_fresh && !game.has_keyword(handle, Keyword::Rush) {
        return false;
    }
    true
}

/// Legal target action IDs for an effect-installed immediate attack prompt.
///
/// This mirrors Main-phase attack target filtering while intentionally omitting
/// the Main-phase memory gate: the enclosing effect grants the immediate
/// attack, but target legality still uses the normal action IDs.
pub(crate) fn effect_attack_target_action_ids(
    game: &Game,
    attacker: PermanentHandle,
    restriction: AttackTargetRestriction,
    without_suspending: bool,
) -> Vec<u16> {
    effect_attack_target_action_ids_with_options(
        game,
        attacker,
        restriction,
        without_suspending,
        false,
    )
}

/// The attack declarations an end-of-turn attack keyword's trigger body
/// offers for `attacker` (`Game::offer_end_of_turn_keyword_attack`), as
/// `encode_attack` ids; empty when it cannot attack at all.
///
/// - `<Engage>` (§16-44-1): an ordinary attack: the opponent, or a suspended
///   Digimon (§11-2-7-1).
/// - `<Execute>` (§16-38-1): also an opponent's UNSUSPENDED Digimon.
/// - `<Vortex>` (§16-33-1): an opponent's Digimon it may attack (suspended,
///   §11-2-7-1; W2 reading (a)), the player only under
///   `VortexCanAttackPlayer` (DCGO `PermanentHasVortexCanAttackPlayers`);
///   it may attack the turn it was played (`can_attack(.., vortex = true)`).
///
/// The same player- and permanent-scoped gates as every other offered attack
/// (`attack_declaration_action_ids`: `CannotAttack`, `CannotAttackPlayer`,
/// `MayAttackPlayerOnly` and the per-target bans in
/// `attack_target_blocked_by_modifier`); no Main-phase memory gate.
pub(crate) fn end_of_turn_keyword_attack_action_ids(
    game: &Game,
    attacker: PermanentHandle,
    keyword: Keyword,
) -> Vec<u16> {
    if game.pending_attack.is_some() {
        return Vec::new();
    }
    end_of_turn_keyword_potential_attack_action_ids(game, attacker, keyword)
}

/// Intrinsic targets for a keyword activation that may defer its offer until
/// the current attack ends. This is not a legal-now declaration list.
pub(crate) fn end_of_turn_keyword_potential_attack_action_ids(
    game: &Game,
    attacker: PermanentHandle,
    keyword: Keyword,
) -> Vec<u16> {
    let (restriction, vortex, unsuspended_targets) = match keyword {
        Keyword::Engage => (AttackTargetRestriction::Any, false, false),
        Keyword::Execute => (AttackTargetRestriction::Any, false, true),
        Keyword::Vortex => (
            if game
                .modifiers
                .has(attacker, ModifierType::VortexCanAttackPlayer)
            {
                AttackTargetRestriction::Any
            } else {
                AttackTargetRestriction::DigimonOnly
            },
            true,
            false,
        ),
        _ => return Vec::new(),
    };
    attack_declaration_action_ids(game, attacker, restriction, false, false, vortex, unsuspended_targets)
}

pub(crate) fn effect_attack_target_action_ids_with_options(
    game: &Game,
    attacker: PermanentHandle,
    restriction: AttackTargetRestriction,
    without_suspending: bool,
    ignore_summoning_sickness: bool,
) -> Vec<u16> {
    // CR 11-2-4 / BT18-088 Q3046: activation can occur during an attack,
    // but a new declaration cannot, including in EndOfAttackDrain.
    if game.pending_attack.is_some() {
        return Vec::new();
    }
    attack_declaration_action_ids(
        game,
        attacker,
        restriction,
        without_suspending,
        ignore_summoning_sickness,
        false,
        false,
    )
}

/// Intrinsic targets behind immediate attack offers and deferred keyword
/// activation. Legal-now wrappers separately enforce the declaration window;
/// keyword activation must survive while its offer waits for an earlier attack. `vortex` exempts the played-this-turn check (§16-33-1);
/// `unsuspended_targets` adds the opponent's unsuspended Digimon
/// (`<Execute>`, §16-38-1) on top of a `CanAttackUnsuspended` grant.
fn attack_declaration_action_ids(
    game: &Game,
    attacker: PermanentHandle,
    restriction: AttackTargetRestriction,
    without_suspending: bool,
    ignore_summoning_sickness: bool,
    vortex: bool,
    unsuspended_targets: bool,
) -> Vec<u16> {
    if game
        .modifiers
        .player_has(attacker.player, ModifierType::CannotAttack)
    {
        return Vec::new();
    }
    // Permanent-scoped CannotAttack: a declaration would be Cancelled by the
    // `WhenWouldAttack` replacement, so a granted attack must offer no target
    // bits (mirror of the Main-phase / end-of-turn mask gates).
    if game.modifiers.has(attacker, ModifierType::CannotAttack) {
        return Vec::new();
    }
    let attacker_can_attack = if ignore_summoning_sickness && without_suspending {
        game.can_attack_without_suspending_ignoring_summoning_sickness(attacker)
    } else if ignore_summoning_sickness {
        game.can_attack_ignoring_summoning_sickness(attacker)
    } else if without_suspending {
        game.can_attack_without_suspending(attacker, vortex)
    } else {
        game.can_attack(attacker, vortex)
    };
    if !attacker_can_attack {
        return Vec::new();
    }

    let mut action_ids = Vec::new();
    let opponent = game.next_clockwise(attacker.player);
    let attacker_index = attacker.index as u16;

    if matches!(
        restriction,
        AttackTargetRestriction::Any | AttackTargetRestriction::PlayerOnly
    ) && !game
        .modifiers
        .has(attacker, ModifierType::CannotAttackPlayer)
    {
        action_ids.push(encode_attack(attacker_index, SECURITY_TARGET));
    }

    if matches!(
        restriction,
        AttackTargetRestriction::Any | AttackTargetRestriction::DigimonOnly
    ) {
        let can_attack_unsuspended = unsuspended_targets
            || game
                .modifiers
                .has(attacker, ModifierType::CanAttackUnsuspended);
        let max_opp = game.player(opponent).battle_area.len().min(FIELD_SLOTS);

        for j in 0..max_opp {
            let target_handle = PermanentHandle {
                player: opponent,
                index: j as u8,
            };
            let target = &game.player(opponent).battle_area[j];
            if !game.permanent_is_digimon_for_rules(target_handle) {
                continue;
            }
            // Track C / D consult site (2026-05-08):
            // `CanAttackTargetDefendingPermanent` is the affirmative
            // override of `CannotAttackTarget` at the granted-attack
            // mask emission site too.
            if game.attack_target_blocked_by_modifier(attacker, target_handle) {
                continue;
            }
            // §11-2-7-1: a suspended Digimon, or any Digimon under a
            // CAN_ATTACK_UNSUSPENDED grant. <Raid> switches after the
            // declaration (§16-23-1), so it adds no declaration target.
            let legal = target.is_suspended || can_attack_unsuspended;
            if legal {
                action_ids.push(encode_attack(attacker_index, j as u16));
            }
        }
    }

    action_ids
}

/// §4.7d FORCE_ATTACK global mask replacement. If any friendly Digimon
/// carries `ModifierType::ForceAttack` AND has at least one legal attack
/// available (respecting summoning sickness, CanAttackUnsuspended
/// targeting, and CannotAttackTarget per-target filtering), replace the
/// passed-in `mask` with a fresh all-zero mask populated only with those
/// attackers' attack bits.
///
/// Falls through (leaves `mask` untouched) when no forced Digimon has a legal
/// Main-phase attack. This restriction cannot grant an otherwise illegal attack.
fn apply_force_attack_mask_replacement(
    mask: &mut [f32],
    game: &Game,
    player_id: PlayerId,
) {
    // A forced attack restricts the choices that are already legal in Main.
    // Reuse the normal mask so memory, player prohibitions, effective
    // kind, freshness, suspension and target restrictions cannot drift apart.
    let mut replacement = vec![0.0f32; ACTION_SPACE_SIZE];
    for i in 0..game.player(player_id).battle_area.len().min(FIELD_SLOTS) {
        let handle = PermanentHandle { player: player_id, index: i as u8 };
        if !game.modifiers.has(handle, ModifierType::ForceAttack) {
            continue;
        }
        for target in 0..=SECURITY_TARGET {
            let action = encode_attack(i as u16, target) as usize;
            replacement[action] = mask[action];
        }
    }
    if replacement.iter().any(|bit| *bit > 0.0) {
        mask.copy_from_slice(&replacement);
    }
}
