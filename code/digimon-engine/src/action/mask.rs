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
use crate::effect_context::EffectReadContext;
use crate::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword, ModifierType, PlayerId};
use crate::game::Game;
use crate::permanent::PermanentHandle;
use crate::tensor::FIELD_SLOTS;

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
                let mut cost = if is_option_use {
                    card.option_use_cost(&game.card_data)
                        .unwrap_or_else(|| card.play_cost(&game.card_data))
                } else {
                    card.play_cost(&game.card_data)
                } as i16;
                if is_option_use {
                    let link_cost = game
                        .effects_for_card(&card.card_id(&game.card_data), card.handle())
                        .unwrap_or_default()
                        .iter()
                        .find_map(|effect| effect.link_cost)
                        .unwrap_or(0);
                    cost += link_cost as i16;
                }
                // Memory check: card is affordable if memory - cost >= memory_min
                if (game.memory - cost) < game.rules.memory_range.0 {
                    continue;
                }
                // §4.2 Option color requirement: an Option is playable when
                // the player has a matching-color Digimon/Tamer, or when a
                // printed Use Req. predicate satisfies that requirement.
                if is_option_use {
                    if !option_use_requirement_or_color_available(card, game, player_id) {
                        continue;
                    }
                } else if me.battle_area.len() >= game.rules.field_slots as usize {
                    continue;
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
                        if t.is_suspended || !t.is_digimon(&game.card_data) {
                            continue;
                        }
                        let t_handle = PermanentHandle {
                            player: opp_id,
                            index: j as u8,
                        };
                        if let Some(dp) = game.effective_dp(t_handle) {
                            best = Some(best.map_or(dp, |b| b.max(dp)));
                        }
                    }
                    best
                } else {
                    None
                };

                for j in 0..max_opp {
                    let target = &opp.battle_area[j];
                    if !target.is_digimon(&game.card_data) {
                        continue;
                    }
                    let t_handle = PermanentHandle {
                        player: opp_id,
                        index: j as u8,
                    };
                    // §4.7a CANNOT_ATTACK_TARGET — suppress this target if
                    // it carries the modifier. Per-attacker discriminant
                    // from Python is §4.7x.
                    if game
                        .modifiers
                        .has(t_handle, ModifierType::CannotAttackTarget)
                    {
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
                    let base_perm = &me.battle_area[f];
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
                    if can_basic_digivolve(card, base_perm, &game.card_data) {
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
                        if effect.max_per_turn > 0
                            && perm.activation_count(source.handle(), slot as u8)
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
                if !game.can_attack(handle, /* vortex = */ vortex) {
                    continue;
                }

                if !game.modifiers.has(handle, ModifierType::CannotAttackPlayer) {
                    mask[encode_attack(i as u16, SECURITY_TARGET) as usize] = 1.0;
                }
                for j in 0..max_opp {
                    let target = &opp.battle_area[j];
                    if !target.is_digimon(&game.card_data) {
                        continue;
                    }
                    let t_handle = PermanentHandle {
                        player: opp_id,
                        index: j as u8,
                    };
                    // §4.7a CANNOT_ATTACK_TARGET — suppress attacks against
                    // a target carrying the modifier, regardless of which
                    // keyword granted the attack.
                    if game
                        .modifiers
                        .has(t_handle, ModifierType::CannotAttackTarget)
                    {
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
                if t.is_suspended || !t.is_digimon(&game.card_data) {
                    continue;
                }
                let t_handle = PermanentHandle {
                    player: opp_id,
                    index: j as u8,
                };
                if let Some(dp) = game.effective_dp(t_handle) {
                    best = Some(best.map_or(dp, |b| b.max(dp)));
                }
            }
            best
        } else {
            None
        };

        for j in 0..max_opp {
            let target = &opp.battle_area[j];
            if !target.is_digimon(&game.card_data) {
                continue;
            }
            let t_handle = PermanentHandle {
                player: opp_id,
                index: j as u8,
            };
            if game
                .modifiers
                .has(t_handle, ModifierType::CannotAttackTarget)
            {
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
