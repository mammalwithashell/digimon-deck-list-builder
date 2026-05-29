//! Action mask building for the RL action space.
//!
//! Produces a `Vec<f32>` of size `ACTION_SPACE_SIZE` where 1.0 means legal,
//! 0.0 means illegal. Mask is phase-aware.
//!
//! Note: Phase 2 only implements basic Main/Breeding/Mulligan masking using
//! engine state available so far. Combat phases (BlockTiming, CounterTiming,
//! AllianceTiming) and effect-driven actions (effect activations, [Hand][Main],
//! DNA Digivolve, blast, etc.) are filled in later phases.

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
            for &aid in &sel.valid_action_ids {
                if (aid as usize) < ACTION_SPACE_SIZE {
                    mask[aid as usize] = 1.0;
                }
            }
            if sel.is_optional {
                mask[PASS as usize] = 1.0;
            }
            // CONCEDE_GAME (93) is legal at every agent decision point
            // (per the BO3 match-training spec), EXCEPT Mulligan and
            // SelectPlayOrder — both are pre-/inter-game decisions
            // where "forfeit" has no productive meaning, and a
            // random-init policy degenerately picking 93 forfeits the
            // whole match. See add-gameplay-reward-config smoke
            // verification + the SelectPlayOrder concede-mask test in
            // `tests/select_play_order.rs`.
            if !matches!(
                game.current_phase,
                GamePhase::Mulligan | GamePhase::SelectPlayOrder,
            ) {
                mask[CONCEDE_GAME as usize] = 1.0;
            }
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
                    if game.option_legal_play_modes(card, player_id).is_empty() {
                        continue;
                    }
                } else {
                    // Memory check: a non-Option card is affordable if
                    // memory - cost >= memory_min.
                    let cost = card.play_cost(&game.card_data) as i16;
                    if (game.memory - cost) < game.rules.memory_range.0 {
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
            // Memory gate is per-attacker: baseline requires memory >= 0,
            // but §4.3 Blitz carves out "Blitz + digivolved this turn" even
            // when memory < 0. Native/static Blitz parsing remains §4.3b.
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
                let memory_ok = game.memory >= 0 || {
                    let digivolved_this_turn = attacker.turn_digivolved == game.turn_count;
                    digivolved_this_turn && game.has_keyword(handle, Keyword::Blitz)
                };
                if !memory_ok {
                    continue;
                }

                if !game.modifiers.has(handle, ModifierType::CannotAttackPlayer) {
                    let sec_action = encode_attack(i as u16, SECURITY_TARGET);
                    mask[sec_action as usize] = 1.0;
                }

                // Digimon attacks (§4.4). Suspended enemies are always valid.
                // Unsuspended enemies are valid iff attacker has:
                //   - CAN_ATTACK_UNSUSPENDED modifier (any unsuspended), or
                //   - Raid keyword (unsuspended enemies tied for highest
                //     effective DP).
                // Native `_is_can_attack_unsuspended` keyword awaits §4.5.
                let can_attack_unsuspended = game
                    .modifiers
                    .has(handle, ModifierType::CanAttackUnsuspended);
                let has_raid = game.has_keyword(handle, Keyword::Raid);
                let max_opp = opp.battle_area.len().min(FIELD_SLOTS);

                // Precompute max effective DP among unsuspended enemy Digimon
                // only if Raid is relevant (skip when CAN_ATTACK_UNSUSPENDED
                // already broadens targeting).
                let raid_max_dp = if has_raid && !can_attack_unsuspended {
                    let mut best: Option<i32> = None;
                    for j in 0..max_opp {
                        let t = &opp.battle_area[j];
                        let t_handle = PermanentHandle {
                            player: opp_id,
                            index: j as u8,
                        };
                        if t.is_suspended || !game.permanent_is_digimon_for_rules(t_handle) {
                            continue;
                        }
                        if let Some(dp) = game.effective_dp(t_handle) {
                            best = Some(best.map_or(dp, |b| b.max(dp)));
                        }
                    }
                    best
                } else {
                    None
                };

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
                        continue;
                    }
                    if let Some(max_dp) = raid_max_dp {
                        if let Some(dp) = game.effective_dp(t_handle) {
                            if dp == max_dp {
                                mask[action_bit] = 1.0;
                            }
                        }
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
                // Breeding-area digivolve
                if let Some(ref breeding) = me.breeding_area {
                    if can_basic_digivolve(card, breeding, &game.card_data) {
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
            let hand_limit = me.hand.len().min(HAND_MAIN_LIMIT);
            for h in 0..hand_limit {
                let card = &me.hand[h];
                let card_id = card.card_id(&game.card_data);
                let Some(effects) = game.effects_for_card(card_id, card.handle()) else {
                    continue;
                };
                for effect in &effects {
                    if effect.timing != EffectTiming::MainFromHand {
                        continue;
                    }
                    if let Some(cond) = &effect.condition {
                        let ctx = EffectReadContext::new(game, card.handle(), None, player_id);
                        if !cond(&ctx) {
                            continue;
                        }
                    }
                    mask[(HAND_EFFECT_START + h as u16) as usize] = 1.0;
                    break;
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
                let perm = &me.battle_area[i];
                let perm_handle = PermanentHandle {
                    player: player_id,
                    index: i as u8,
                };

                // PUPPETS-G009 — standard `<Delay>` `[Main]`-phase activation.
                // A parked `DelayTrigger::MainPhaseActivated` Option whose
                // placing turn has passed exposes a player-visible
                // FIELD_EFFECT activation (slot `+2`) — "By trashing this
                // card after the placing turn, activate the effect below"
                // (RULES_CONTEXT 16-16). The choice is optional, so PASS
                // remains legal alongside it (emitted below).
                if game.delayed_option_main_activation_available(perm_handle) {
                    let bit = FIELD_EFFECT_START
                        + i as u16 * EFFECTS_PER_PERMANENT
                        + FIELD_EFFECT_SLOT_FOR_MAIN;
                    mask[bit as usize] = 1.0;
                    continue;
                }

                let stack_size = perm.card_sources.len();
                let mut emitted = false;
                for (source_index, source) in perm.card_sources.iter().enumerate() {
                    if emitted {
                        break;
                    }
                    let is_under = source_index + 1 < stack_size;
                    let card_id = source.card_id(&game.card_data);
                    let Some(effects) = game.effects_for_card(card_id, source.handle()) else {
                        continue;
                    };
                    for (slot, effect) in effects.iter().enumerate() {
                        if effect.timing != EffectTiming::MainOnField {
                            continue;
                        }
                        if is_under != effect.inherited {
                            continue;
                        }
                        // G-OPT-MULTI-TIMING-SHARED-LOCKOUT: a multi-timing
                        // OPT cluster shares one counter via `shared_opt_group`.
                        let opt_key = effect.shared_opt_group.unwrap_or(slot as u8);
                        if effect.max_per_turn > 0
                            && perm.activation_count(source.handle(), opt_key)
                                >= effect.max_per_turn
                        {
                            continue;
                        }
                        if let Some(cond) = &effect.condition {
                            let ctx = EffectReadContext::new(
                                game,
                                source.handle(),
                                Some(perm_handle),
                                player_id,
                            );
                            if !cond(&ctx) {
                                continue;
                            }
                        }
                        let bit = FIELD_EFFECT_START
                            + i as u16 * EFFECTS_PER_PERMANENT
                            + FIELD_EFFECT_SLOT_FOR_MAIN;
                        mask[bit as usize] = 1.0;
                        emitted = true;
                        break;
                    }
                }
            }

            // Phase F Task 6 — Training-only breeding-area `[Main]` emitter.
            //
            // Surfaced at field index `BREEDING_TARGET (=14)`, sub-slot
            // `FIELD_EFFECT_SLOT_FOR_MAIN (=2)` → action_id 1142. This bit
            // is reserved for the breeding-area Training activation; no
            // other `MainOnField` effect surfaces from breeding (RULES_CONTEXT
            // 16-40 specifies that ONLY `<Training>` activates from breeding
            // — surfacing all `MainOnField` effects from breeding would
            // inadvertently expose Save/MaterialSave/MindLink from breeding
            // too, which is wrong).
            //
            // Gate: same `CannotActivateMainEffects` short-circuit as the
            // battle-area emitter, then check that the breeding-area
            // permanent's top card carries `Keyword::Training` AND its
            // `MainOnField` `<Training>` auto-effect's condition passes
            // (`!perm.is_suspended`). Inherited Training (a Training source
            // under a non-Training top) is intentionally not surfaced —
            // matches DCGO `Training.cs:21` `card.PermanentOfThisCard()`
            // which scopes to the top card's effect.
            if !main_effects_blocked {
                if let Some(ref breeding) = me.breeding_area {
                    let top = breeding.top_card();
                    // Read printed keyword directly from card_data — note that
                    // `Game::has_keyword` would short-circuit on a `battle_area`
                    // lookup and never reach the breeding-area permanent.
                    let top_data = &game.card_data[top.data_index];
                    if top_data.keywords.contains(&Keyword::Training) && !breeding.is_suspended {
                        // Gate the bit ONLY on the printed keyword + suspension
                        // — we deliberately don't run the `<Training>` effect's
                        // own `condition` closure here because
                        // `EffectReadContext::source_permanent()` resolves
                        // via `battle_area`, which would silently return None
                        // for a breeding-area carrier and short-circuit the
                        // gate to false. The on-field condition
                        // (`!perm.is_suspended`) is faithfully represented by
                        // the inline check above.
                        let bit = FIELD_EFFECT_START
                            + BREEDING_TARGET * EFFECTS_PER_PERMANENT
                            + FIELD_EFFECT_SLOT_FOR_MAIN;
                        mask[bit as usize] = 1.0;
                    }
                }
            }

            // Trash [Main] (bits 1150-1194). One bit per trash slot,
            // first-match-wins, mirroring `action_mask.py:216-225`.
            let trash_limit = me.trash.len().min(TRASH_MAIN_LIMIT);
            for t in 0..trash_limit {
                let card = &me.trash[t];
                let card_id = card.card_id(&game.card_data);
                let Some(effects) = game.effects_for_card(card_id, card.handle()) else {
                    continue;
                };
                for effect in &effects {
                    if effect.timing != EffectTiming::MainFromTrash {
                        continue;
                    }
                    if let Some(cond) = &effect.condition {
                        let ctx = EffectReadContext::new(game, card.handle(), None, player_id);
                        if !cond(&ctx) {
                            continue;
                        }
                    }
                    mask[(TRASH_EFFECT_START + t as u16) as usize] = 1.0;
                    break;
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
            apply_force_attack_mask_replacement(&mut mask, game, player_id, opp_id);
        }

        GamePhase::Breeding => {
            // Hatch (60)
            if me.breeding_area.is_none() && !me.digitama_deck.is_empty() {
                mask[HATCH as usize] = 1.0;
            }
            // Move from breeding (61): requires Digimon at level >= 3
            if let Some(ref perm) = me.breeding_area {
                if perm.level(&game.card_data).unwrap_or(0) >= 3
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

                // §4.6c Overclock (sub-slot 0 of the per-permanent field
                // effect range). Mirrors Python action_mask.py:354-361:
                // emits when the Digimon has Overclock AND at least one
                // other sacrificeable permanent exists on the battle area.
                if game.has_keyword(handle, Keyword::Overclock)
                    && !game.is_overclock_declined_for_action_mask(handle)
                    && game.has_overclock_sacrifice(player_id, i)
                {
                    let bit = FIELD_EFFECT_START
                        + i as u16 * EFFECTS_PER_PERMANENT
                        + FIELD_EFFECT_SLOT_FOR_OVERCLOCK;
                    mask[bit as usize] = 1.0;
                }

                // §4.7e CannotAttack gate — skip all attack-bit emission
                // for this permanent when the player-scoped modifier is set.
                if attack_blocked {
                    continue;
                }

                // §4.6 attack bits: Vortex / MayAttack / ForceAttack all
                // share the 100-399 attack range and the same target loop
                // (any enemy Digimon + security, subject to
                // CannotAttackTarget). Vortex uses the summoning-sickness
                // exemption; the other two use normal `can_attack`.
                let vortex = game.has_keyword(handle, Keyword::Vortex);
                let may_attack = game.modifiers.has(handle, ModifierType::MayAttack);
                let force_attack = game.modifiers.has(handle, ModifierType::ForceAttack);
                if !vortex && !may_attack && !force_attack {
                    continue;
                }
                let vortex_can_attack = vortex && game.can_attack(handle, /* vortex = */ true);
                let normal_eot_can_attack = (may_attack || force_attack)
                    && game.can_attack(handle, /* vortex = */ false);
                if !vortex_can_attack && !normal_eot_can_attack {
                    continue;
                }

                let can_attack_player = normal_eot_can_attack
                    || (vortex_can_attack
                        && game
                            .modifiers
                            .has(handle, ModifierType::VortexCanAttackPlayer));
                if can_attack_player
                    && !game.modifiers.has(handle, ModifierType::CannotAttackPlayer)
                {
                    mask[encode_attack(i as u16, SECURITY_TARGET) as usize] = 1.0;
                }
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

    // CONCEDE_GAME (93) is legal at every agent decision point (per
    // the BO3 match-training spec), EXCEPT Mulligan and SelectPlayOrder.
    // Conceding during the pre-game keep/redraw decision or during the
    // inter-game play-order pick is semantically degenerate (the game
    // hasn't started; or the agent is mid-match, not mid-game) AND was
    // a real degeneracy with random-init policies — they would pick 93
    // via argmax and forfeit the entire BO3 match. See
    // `tests/select_play_order.rs::mask_does_not_expose_concede_during_*`
    // for the contract.
    //
    // We detect "has decision point" as "at least one other action is
    // legal in the mask." Players with no agency (e.g., not their turn
    // during Main) get an all-zero mask, so concede stays zero too.
    // The soft-lock PASS rail does set PASS=1 and so will surface
    // concede=1 alongside — harmless: in those rare states the agent
    // step shouldn't be advancing the game anyway.
    let concede_allowed = !matches!(
        game.current_phase,
        GamePhase::Mulligan | GamePhase::SelectPlayOrder,
    );
    if concede_allowed {
        let has_other_legal = mask
            .iter()
            .enumerate()
            .any(|(i, &v)| v > 0.0 && i != CONCEDE_GAME as usize);
        if has_other_legal {
            mask[CONCEDE_GAME as usize] = 1.0;
        }
    }

    mask
}

// ─── Helpers ──────────────────────────────────────────────────────────

/// §4.2 — Check that the player has at least one Digimon or Tamer of a
/// color matching any of the Option card's colors, either on the battle
/// area or in the breeding area.
///
/// Mirror of Python's `action_mask.py` lines 77-99 for ordinary color
/// matching. Player-scoped `IgnoreColorRequirement` is consumed by
/// `option_use_requirement_or_color_available` before this helper runs.
pub(crate) fn option_color_match_available(
    card: &crate::card_source::CardSource,
    me: &crate::player::Player,
    card_data: &[crate::card_data::CardData],
) -> bool {
    let option_colors = card.option_colors(card_data);

    // Battle area: Digimon or Tamer with an overlapping color.
    for perm in &me.battle_area {
        if !perm.is_digimon(card_data) && !perm.is_tamer(card_data) {
            continue;
        }
        let perm_colors = if perm.is_digimon(card_data) {
            perm.top_card().digimon_colors(card_data)
        } else {
            perm.top_card().colors(card_data)
        };
        if option_colors.iter().any(|c| perm_colors.contains(c)) {
            return true;
        }
    }

    // Breeding area: only Digimon counts (Tamers can't be in breeding).
    if let Some(ref breeding) = me.breeding_area {
        if breeding.is_digimon(card_data) {
            let perm_colors = breeding.top_card().digimon_colors(card_data);
            if option_colors.iter().any(|c| perm_colors.contains(c)) {
                return true;
            }
        }
    }

    false
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

/// An Option hand/trash use must have a currently active `OptionMain` body.
/// This prevents partial Security-only YAML from becoming a legal no-effect
/// hand play, and lets card-level conditions preflight mandatory Main choices.
pub(crate) fn option_has_active_main_effect(
    card: &crate::card_source::CardSource,
    game: &Game,
    player_id: PlayerId,
) -> bool {
    let card_id = card.card_id(&game.card_data);
    let Some(effects) = game.effects_for_card(card_id, card.handle()) else {
        return true;
    };
    if effects.is_empty() {
        return true;
    }
    let ctx = EffectReadContext::new_with_source_kind(
        game,
        card.handle(),
        None,
        EffectSourceKind::Option,
        player_id,
    );
    effects.iter().any(|effect| {
        if effect.delay_trigger.is_some() {
            return true;
        }
        if !matches!(
            effect.timing,
            EffectTiming::OptionMain | EffectTiming::MainFromHand
        ) {
            return false;
        }
        effect
            .condition
            .as_ref()
            .is_none_or(|condition| condition(&ctx))
    })
}

/// Counter-window Option uses are legal through a `CounterEffect` body even
/// when the card has no ordinary `OptionMain` body.
pub(crate) fn option_has_active_counter_effect(
    card: &crate::card_source::CardSource,
    game: &Game,
    player_id: PlayerId,
) -> bool {
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
    if !perm.is_digimon(card_data) {
        return false;
    }
    // Summoning sickness: can't attack the turn it was played unless Rush
    // is present (native printed OR modifier-granted) — §2.1b.
    let is_fresh = perm.turn_played == turn && perm.turn_digivolved != turn;
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

pub(crate) fn effect_attack_target_action_ids_with_options(
    game: &Game,
    attacker: PermanentHandle,
    restriction: AttackTargetRestriction,
    without_suspending: bool,
    ignore_summoning_sickness: bool,
) -> Vec<u16> {
    if game
        .modifiers
        .player_has(attacker.player, ModifierType::CannotAttack)
    {
        return Vec::new();
    }
    let attacker_can_attack = if ignore_summoning_sickness && without_suspending {
        game.can_attack_without_suspending_ignoring_summoning_sickness(attacker)
    } else if ignore_summoning_sickness {
        game.can_attack_ignoring_summoning_sickness(attacker)
    } else if without_suspending {
        game.can_attack_without_suspending(attacker, false)
    } else {
        game.can_attack(attacker, false)
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
        let can_attack_unsuspended = game
            .modifiers
            .has(attacker, ModifierType::CanAttackUnsuspended);
        let has_raid = game.has_keyword(attacker, Keyword::Raid);
        let max_opp = game.player(opponent).battle_area.len().min(FIELD_SLOTS);
        let raid_max_dp = if has_raid && !can_attack_unsuspended {
            let mut best: Option<i32> = None;
            for j in 0..max_opp {
                let target = &game.player(opponent).battle_area[j];
                let target_handle = PermanentHandle {
                    player: opponent,
                    index: j as u8,
                };
                if target.is_suspended || !game.permanent_is_digimon_for_rules(target_handle) {
                    continue;
                }
                if let Some(dp) = game.effective_dp(target_handle) {
                    best = Some(best.map_or(dp, |b| b.max(dp)));
                }
            }
            best
        } else {
            None
        };

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
            let legal = target.is_suspended
                || can_attack_unsuspended
                || raid_max_dp.is_some_and(|max_dp| {
                    game.effective_dp(target_handle)
                        .is_some_and(|dp| dp == max_dp)
                });
            if legal {
                action_ids.push(encode_attack(attacker_index, j as u16));
            }
        }
    }

    action_ids
}

/// Basic digivolve eligibility: hand card has an evo_cost matching the base
/// permanent's color and level. Full validation (alt-digi, color modifiers,
/// CANNOT_DIGIVOLVE) deferred.
fn can_basic_digivolve(
    card: &crate::card_source::CardSource,
    base: &crate::permanent::Permanent,
    card_data: &[CardData],
) -> bool {
    let base_top = base.top_card();
    let base_meta = &card_data[base_top.data_index];

    // Base must be Digimon or DigiEgg
    if !base_top.is_digimon_card_for_search(card_data) && base_meta.card_kind != CardKind::DigiEgg {
        return false;
    }

    let base_level = match base_top.digimon_level(card_data) {
        Some(l) => l,
        None => return false,
    };
    let base_colors = base_top.digimon_colors(card_data);

    // Find a matching evo_cost
    for evo in card.digivolution_costs(card_data) {
        if evo.level != base_level {
            continue;
        }
        let color = match evo_color(evo.card_color) {
            Some(c) => c,
            None => continue,
        };
        if !base_colors.contains(&color) {
            continue;
        }
        return true;
    }
    false
}

/// §4.7d FORCE_ATTACK global mask replacement. If any friendly Digimon
/// carries `ModifierType::ForceAttack` AND has at least one legal attack
/// available (respecting summoning sickness, Raid / CanAttackUnsuspended
/// targeting, and CannotAttackTarget per-target filtering), replace the
/// passed-in `mask` with a fresh all-zero mask populated only with those
/// attackers' attack bits.
///
/// Falls through (leaves `mask` untouched) when no forced Digimon can
/// actually attack — matches Python's behavior at
/// `action_mask.py:279-280`. Intentionally does not gate on memory:
/// Python's forced-attack branch only checks `attacker.can_attack()`
/// (summoning sickness), not memory.
fn apply_force_attack_mask_replacement(
    mask: &mut [f32],
    game: &Game,
    player_id: PlayerId,
    opp_id: PlayerId,
) {
    let me = game.player(player_id);
    let opp = game.player(opp_id);
    let max_field = me.battle_area.len().min(FIELD_SLOTS);

    // First pass: is any friendly Digimon forced to attack?
    let has_any_forced = (0..max_field).any(|i| {
        let handle = PermanentHandle {
            player: player_id,
            index: i as u8,
        };
        game.modifiers.has(handle, ModifierType::ForceAttack)
    });
    if !has_any_forced {
        return;
    }

    // Build the replacement mask into a fresh buffer so we can fall through
    // if no forced attacker can legally act this turn.
    let mut replacement = vec![0.0f32; ACTION_SPACE_SIZE];
    let mut any_attack_emitted = false;
    let max_opp = opp.battle_area.len().min(FIELD_SLOTS);

    for i in 0..max_field {
        let attacker = &me.battle_area[i];
        let handle = PermanentHandle {
            player: player_id,
            index: i as u8,
        };
        if !game.modifiers.has(handle, ModifierType::ForceAttack) {
            continue;
        }
        if !can_basic_attack(attacker, handle, game.turn_count, &game.card_data, game) {
            continue;
        }

        // Security attack.
        replacement[encode_attack(i as u16, SECURITY_TARGET) as usize] = 1.0;
        any_attack_emitted = true;

        // Digimon attacks — same Raid / CanAttackUnsuspended logic as the
        // normal Main-phase attack block above.
        let can_attack_unsuspended = game
            .modifiers
            .has(handle, ModifierType::CanAttackUnsuspended);
        let has_raid = game.has_keyword(handle, Keyword::Raid);
        let raid_max_dp = if has_raid && !can_attack_unsuspended {
            let mut best: Option<i32> = None;
            for j in 0..max_opp {
                let t = &opp.battle_area[j];
                let t_handle = PermanentHandle {
                    player: opp_id,
                    index: j as u8,
                };
                if t.is_suspended || !game.permanent_is_digimon_for_rules(t_handle) {
                    continue;
                }
                if let Some(dp) = game.effective_dp(t_handle) {
                    best = Some(best.map_or(dp, |b| b.max(dp)));
                }
            }
            best
        } else {
            None
        };

        for j in 0..max_opp {
            let t_handle = PermanentHandle {
                player: opp_id,
                index: j as u8,
            };
            let target = &opp.battle_area[j];
            if !game.permanent_is_digimon_for_rules(t_handle) {
                continue;
            }
            if game.attack_target_blocked_by_modifier(handle, t_handle) {
                continue;
            }
            let action_bit = encode_attack(i as u16, j as u16) as usize;
            if target.is_suspended {
                replacement[action_bit] = 1.0;
                continue;
            }
            if can_attack_unsuspended {
                replacement[action_bit] = 1.0;
                continue;
            }
            if let Some(max_dp) = raid_max_dp {
                if let Some(dp) = game.effective_dp(t_handle) {
                    if dp == max_dp {
                        replacement[action_bit] = 1.0;
                    }
                }
            }
        }
    }

    if any_attack_emitted {
        mask.copy_from_slice(&replacement);
    }
}
