use crate::card_data::CardData;
use crate::card_registry::CardRegistry;
use crate::card_source::CardSource;
use crate::enums::{CardKind, EffectSourceKind, EffectTiming, GamePhase, PlayerId};
use crate::game::Game;
use crate::permanent::{Permanent, PermanentHandle};
use crate::player::Player;
use crate::tensor::DP_NORM;
use crate::tensor_profiles::standard::v2_lite as layout;

pub fn build_tensor_standard_lite_v2(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
) -> Vec<f32> {
    let mut tensor = vec![0.0f32; layout::TENSOR_SIZE];
    let opponent_id = game.next_clockwise(player_id);

    write_global_features(&mut tensor, game, player_id);
    write_player_summary(&mut tensor, game, player_id, 0, player_id);
    write_player_summary(&mut tensor, game, player_id, 1, opponent_id);
    write_permanent_table(&mut tensor, game, registry, player_id, 0, player_id);
    write_permanent_table(&mut tensor, game, registry, player_id, 1, opponent_id);
    write_own_hand(&mut tensor, game, registry, player_id);
    write_known_zone_cards(&mut tensor, game, registry, player_id, opponent_id);
    write_decision_context(&mut tensor, game, player_id);
    write_pending_choice_features(&mut tensor, game, registry, player_id);

    tensor
}

fn write_global_features(t: &mut [f32], game: &Game, observer: PlayerId) {
    let base = layout::OFF_GLOBAL_FEATURES;
    t[base] = layout::TENSOR_VERSION as f32;
    t[base + 1] = (game.turn_count as f32 / 30.0).min(1.0);
    t[base + 2] = if game.turn_player() == observer {
        game.memory
    } else {
        -game.memory
    } as f32
        / 10.0;
    write_phase_one_hot(t, base + 8, game.current_phase);
    t[base + 40] = relative_player(game.turn_player(), observer);
    t[base + 41] = if game.game_over { 1.0 } else { 0.0 };
    if let Some(winner) = game.winner {
        t[base + 42] = relative_player(winner, observer);
    }
}

fn write_player_summary(
    t: &mut [f32],
    game: &Game,
    observer: PlayerId,
    row: usize,
    player_id: PlayerId,
) {
    let player = game.player(player_id);
    let base = layout::OFF_PLAYER_SUMMARY + row * layout::PLAYER_SUMMARY_ROW_SIZE;
    t[base] = player.deck.len() as f32 / 60.0;
    t[base + 1] = player.digitama_deck.len() as f32 / 10.0;
    t[base + 2] = player.hand.len() as f32 / 30.0;
    t[base + 3] = player.security.len() as f32 / 10.0;
    t[base + 4] = player.trash.len() as f32 / 60.0;
    t[base + 5] = player.battle_area.len() as f32 / 14.0;
    t[base + 6] = if player.breeding_area.is_some() {
        1.0
    } else {
        0.0
    };
    t[base + 7] = relative_player(player_id, observer);
}

fn write_permanent_table(
    t: &mut [f32],
    game: &Game,
    registry: &CardRegistry,
    observer: PlayerId,
    player_row: usize,
    player_id: PlayerId,
) {
    let player = game.player(player_id);
    let action_mask =
        (player_id == observer).then(|| crate::action::build_action_mask(game, observer));
    let action_mask = action_mask.as_deref().unwrap_or(&[]);

    for (slot, permanent) in player.battle_area.iter().take(14).enumerate() {
        let handle = PermanentHandle {
            player: player_id,
            index: slot as u8,
        };
        write_permanent_row(
            t,
            permanent_base(player_row, slot),
            game,
            registry,
            &action_mask,
            observer,
            player_id,
            slot,
            permanent,
            Some(handle),
            false,
        );
    }

    if let Some(permanent) = player.breeding_area.as_ref() {
        write_permanent_row(
            t,
            permanent_base(player_row, 14),
            game,
            registry,
            &action_mask,
            observer,
            player_id,
            14,
            permanent,
            None,
            true,
        );
    }
}

fn write_permanent_row(
    t: &mut [f32],
    base: usize,
    game: &Game,
    registry: &CardRegistry,
    action_mask: &[f32],
    observer: PlayerId,
    controller: PlayerId,
    slot: usize,
    permanent: &Permanent,
    handle: Option<PermanentHandle>,
    is_breeding: bool,
) {
    let top = permanent.top_card();
    let card = &game.card_data[top.data_index];
    t[base] = 1.0;
    t[base + 1] = relative_player(controller, observer);
    t[base + 2] = slot as f32 / 14.0;
    t[base + 3] = if is_breeding { 0.0 } else { 1.0 };
    t[base + 4] = if is_breeding { 1.0 } else { 0.0 };
    t[base + layout::PERM_TOP_CARD_ID_OFFSET] =
        registry.get_index(top.card_id(&game.card_data)) as f32;
    write_static_card_features(t, base + 9, card);
    t[base + layout::PERM_DP_OFFSET] =
        permanent.base_dp(&game.card_data).unwrap_or(0) as f32 / DP_NORM;
    t[base + layout::PERM_SUSPENDED_OFFSET] = if permanent.is_suspended { 1.0 } else { 0.0 };
    t[base + layout::PERM_SOURCE_COUNT_OFFSET] =
        permanent.card_sources.len() as f32 / layout::PERM_MAX_SOURCES as f32;
    t[base + layout::PERM_LINKED_COUNT_OFFSET] = permanent.linked_cards.len() as f32 / 5.0;

    if let Some(handle) = handle.filter(|_| !permanent.card_sources.iter().any(|s| s.face_down)) {
        t[base + layout::PERM_OPT_TOTAL_OFFSET] = game.opt_total(handle) as f32;
        t[base + layout::PERM_OPT_USED_OFFSET] = game.opt_used(handle) as f32;
    }

    if !is_breeding {
        t[base + 33] = can_attack_from_slot(action_mask, slot);
        t[base + 34] = 0.0;
    }

    for (source_idx, source) in permanent
        .card_sources
        .iter()
        .take(layout::PERM_MAX_SOURCES)
        .enumerate()
    {
        let source_base =
            base + layout::PERM_SOURCE_START_OFFSET + source_idx * layout::PERM_SOURCE_ENTRY_SIZE;
        if !source.face_down {
            t[source_base + layout::PERM_SOURCE_CARD_ID_OFFSET] =
                registry.get_index(source.card_id(&game.card_data)) as f32;
        }
        if let Some(handle) = handle.filter(|_| !source.face_down) {
            t[source_base + layout::PERM_SOURCE_OPT_STATE_OFFSET] =
                game.source_opt_state(handle, source_idx);
            t[source_base + layout::PERM_SOURCE_DP_CONTRIBUTION_OFFSET] =
                game.source_dp_contribution(handle, source_idx) as f32 / DP_NORM;
        }
    }
}

fn write_own_hand(t: &mut [f32], game: &Game, registry: &CardRegistry, player_id: PlayerId) {
    let player = game.player(player_id);
    let mask = crate::action::build_action_mask(game, player_id);
    for (idx, card_source) in player.hand.iter().take(layout::OWN_HAND_ROWS).enumerate() {
        let base = layout::OFF_OWN_HAND + idx * layout::OWN_HAND_ROW_SIZE;
        let card = &game.card_data[card_source.data_index];
        t[base] = 1.0;
        t[base + layout::OWN_HAND_CARD_ID_OFFSET] =
            registry.get_index(card_source.card_id(&game.card_data)) as f32;
        write_static_card_features(t, base + 2, card);
        t[base + 19] = mask
            .get(crate::action::space::PLAY_HAND_START as usize + idx)
            .copied()
            .unwrap_or(0.0);
        t[base + 20] = mask
            .get(crate::action::space::HAND_EFFECT_START as usize + idx)
            .copied()
            .unwrap_or(0.0);
        t[base + 21] = hand_has_digivolve_action(&mask, idx);
        t[base + 22] = mask
            .get(crate::action::space::DNA_DIGIVOLVE_START as usize + idx)
            .copied()
            .unwrap_or(0.0);
    }
}

fn write_known_zone_cards(
    t: &mut [f32],
    game: &Game,
    registry: &CardRegistry,
    observer: PlayerId,
    opponent: PlayerId,
) {
    let mut row = 0usize;
    row = write_card_rows(
        t,
        row,
        game,
        registry,
        &game.player(observer).trash,
        1.0,
        0.0,
        45,
    );
    let _row = write_card_rows(
        t,
        row,
        game,
        registry,
        &game.player(opponent).trash,
        -1.0,
        1.0,
        45,
    );
    write_security_rows(t, 90, game, registry, game.player(observer), 1.0, 2.0);
    write_security_rows(t, 100, game, registry, game.player(opponent), -1.0, 3.0);
    write_card_rows(t, 110, game, registry, &game.revealed_cards, 0.0, 4.0, 10);
}

fn write_decision_context(t: &mut [f32], game: &Game, observer: PlayerId) {
    let base = layout::OFF_DECISION_CONTEXT;
    write_phase_one_hot(t, base, game.current_phase);
    t[base + 24] = relative_player(game.turn_player(), observer);
    if let Some(sel) = game.pending_selection.as_ref() {
        t[base + 25] = 1.0;
        t[base + 26] = relative_player(sel.selecting_player, observer);
        if observer == sel.selecting_player {
            t[base + 27] = if sel.is_optional { 1.0 } else { 0.0 };
            t[base + 28] =
                (sel.valid_action_ids.len() as f32 / layout::PENDING_CHOICE_ROWS as f32).min(1.0);
        }
    }
}

fn write_pending_choice_features(
    t: &mut [f32],
    game: &Game,
    registry: &CardRegistry,
    observer: PlayerId,
) {
    if let Some(sel) = game.pending_selection.as_ref() {
        if observer != sel.selecting_player {
            return;
        }
        let source_index = game
            .card_data_for_handle(sel.source_card)
            .map(|card| registry.get_index(&card.card_id) as f32)
            .unwrap_or(0.0);
        for (row, action_id) in sel
            .valid_action_ids
            .iter()
            .take(layout::PENDING_CHOICE_ROWS)
            .enumerate()
        {
            let base = layout::OFF_PENDING_CHOICE_FEATURES + row * layout::PENDING_CHOICE_ROW_SIZE;
            t[base] = 1.0;
            t[base + 1] = relative_player(sel.selecting_player, observer);
            t[base + 2] = *action_id as f32 / crate::action::space::ACTION_SPACE_SIZE as f32;
            t[base + 3] = row as f32 / layout::PENDING_CHOICE_ROWS as f32;
            t[base + 4] =
                (sel.valid_action_ids.len() as f32 / layout::PENDING_CHOICE_ROWS as f32).min(1.0);
            let choice = sel
                .effect_choices
                .as_ref()
                .and_then(|choices| choices.iter().find(|choice| choice.action_id == *action_id));
            t[base + 18] = if choice
                .map(|choice| choice.is_optional)
                .unwrap_or(sel.is_optional)
            {
                1.0
            } else {
                0.0
            };
            t[base + layout::PENDING_SOURCE_CARD_ID_OFFSET] = source_index;
            if let Some(choice) = choice {
                if let Some(timing) = choice.timing {
                    t[base + 22 + timing_bucket(timing)] = 1.0;
                }
                if let Some(source_kind) = choice.source_kind {
                    t[base + 34 + source_kind_bucket(source_kind)] = 1.0;
                }
                if let Some(source_card) = choice.source_card {
                    t[base + layout::PENDING_SOURCE_CARD_ID_OFFSET] = game
                        .card_data_for_handle(source_card)
                        .map(|card| registry.get_index(&card.card_id) as f32)
                        .unwrap_or(source_index);
                }
                write_effect_category_flags(t, base + 45, choice.observation_metadata.categories);
            }
        }
    }
}

fn timing_bucket(timing: EffectTiming) -> usize {
    use EffectTiming::*;
    match timing {
        OnPlay => 0,
        WhenDigivolving | OnDigivolve | OnDnaDigivolve => 1,
        OnAttack | WhenAttacking => 2,
        SecuritySkill | OnSecurityCheck | OnLoseSecurity | OnDiscardSecurity => 3,
        EndOfYourTurn | EndOfOpponentsTurn => 4,
        StartOfYourTurn | StartOfOpponentsTurn | StartOfYourMainPhase => 5,
        OnDeletion | OnAnyDeletion => 6,
        CounterEffect => 7,
        OptionMain | DelayEffect => 8,
        _ => 11,
    }
}

fn source_kind_bucket(source_kind: EffectSourceKind) -> usize {
    match source_kind {
        EffectSourceKind::Digimon => 0,
        EffectSourceKind::Tamer => 1,
        EffectSourceKind::Option => 2,
        EffectSourceKind::Rule => 3,
    }
}

fn write_effect_category_flags(
    t: &mut [f32],
    start: usize,
    flags: crate::effect::EffectCategoryFlags,
) {
    t[start] = flags.delete as u8 as f32;
    t[start + 1] = flags.suspend as u8 as f32;
    t[start + 2] = flags.unsuspend as u8 as f32;
    t[start + 3] = flags.bounce as u8 as f32;
    t[start + 4] = flags.bottom_deck as u8 as f32;
    t[start + 5] = flags.dp_change as u8 as f32;
    t[start + 6] = flags.draw_search as u8 as f32;
    t[start + 7] = flags.memory as u8 as f32;
    t[start + 8] = flags.play as u8 as f32;
    t[start + 9] = flags.digivolve as u8 as f32;
    t[start + 10] = flags.recover as u8 as f32;
    t[start + 11] = flags.trash_security as u8 as f32;
    t[start + 12] = flags.grant_keyword as u8 as f32;
    t[start + 13] = flags.grant_immunity as u8 as f32;
    t[start + 14] = flags.protection as u8 as f32;
}

fn write_static_card_features(t: &mut [f32], base: usize, card: &CardData) {
    t[base] = card_kind_bucket(card.card_kind);
    t[base + 1] = card.level.unwrap_or(0) as f32 / 7.0;
    t[base + 2] = card.dp.unwrap_or(0) as f32 / DP_NORM;
    t[base + 3] = card.play_cost as f32 / 15.0;
    for color in &card.colors {
        let idx = (*color as usize).min(6);
        t[base + 4 + idx] = 1.0;
    }
}

fn write_card_rows(
    t: &mut [f32],
    start_row: usize,
    game: &Game,
    registry: &CardRegistry,
    cards: &[CardSource],
    owner_relative: f32,
    zone_bucket: f32,
    limit: usize,
) -> usize {
    for (idx, card_source) in cards.iter().take(limit).enumerate() {
        let base = layout::OFF_KNOWN_ZONE_CARDS + (start_row + idx) * layout::KNOWN_ZONE_ROW_SIZE;
        let card = &game.card_data[card_source.data_index];
        t[base] = 1.0;
        t[base + layout::KNOWN_ZONE_CARD_ID_OFFSET] =
            registry.get_index(card_source.card_id(&game.card_data)) as f32;
        t[base + 2] = owner_relative;
        t[base + 3] = zone_bucket;
        t[base + 4] = idx as f32 / limit.max(1) as f32;
        t[base + 5] = card_kind_bucket(card.card_kind);
        t[base + 6] = card.level.unwrap_or(0) as f32 / 7.0;
        t[base + 7] = card.dp.unwrap_or(card.play_cost as i32) as f32 / DP_NORM;
    }
    start_row + limit
}

fn write_security_rows(
    t: &mut [f32],
    start_row: usize,
    game: &Game,
    registry: &CardRegistry,
    owner: &Player,
    owner_relative: f32,
    zone_bucket: f32,
) {
    for (idx, card_source) in owner.security.iter().take(10).enumerate() {
        let base = layout::OFF_KNOWN_ZONE_CARDS + (start_row + idx) * layout::KNOWN_ZONE_ROW_SIZE;
        t[base] = 1.0;
        if owner.face_up_security.contains(&card_source.card_index) {
            let card = &game.card_data[card_source.data_index];
            t[base + layout::KNOWN_ZONE_CARD_ID_OFFSET] =
                registry.get_index(card_source.card_id(&game.card_data)) as f32;
            t[base + 5] = card_kind_bucket(card.card_kind);
            t[base + 6] = card.level.unwrap_or(0) as f32 / 7.0;
            t[base + 7] = card.dp.unwrap_or(card.play_cost as i32) as f32 / DP_NORM;
        }
        t[base + 2] = owner_relative;
        t[base + 3] = zone_bucket;
        t[base + 4] = idx as f32 / 10.0;
    }
}

fn permanent_base(player_row: usize, slot: usize) -> usize {
    layout::OFF_PERMANENT_SLOTS
        + (player_row * layout::PERMANENT_SLOTS_PER_PLAYER + slot) * layout::PERMANENT_SLOT_SIZE
}

fn can_attack_from_slot(action_mask: &[f32], slot: usize) -> f32 {
    if slot >= crate::action::space::MAX_FIELD_SLOTS as usize {
        return 0.0;
    }
    let start = crate::action::space::ATTACK_START as usize
        + slot * crate::action::space::TARGETS_PER_ATTACKER as usize;
    let end = start + crate::action::space::TARGETS_PER_ATTACKER as usize;
    if (start..end).any(|action| action_mask.get(action).copied().unwrap_or(0.0) > 0.5) {
        1.0
    } else {
        0.0
    }
}

fn hand_has_digivolve_action(action_mask: &[f32], hand_index: usize) -> f32 {
    let start = crate::action::space::DIGIVOLVE_START as usize
        + hand_index * crate::action::space::FIELDS_PER_HAND as usize;
    let end = start + crate::action::space::FIELDS_PER_HAND as usize;
    if (start..end).any(|action| action_mask.get(action).copied().unwrap_or(0.0) > 0.5) {
        1.0
    } else {
        0.0
    }
}

fn card_kind_bucket(kind: CardKind) -> f32 {
    match kind {
        CardKind::Digimon => 1.0,
        CardKind::Tamer => 2.0,
        CardKind::Option => 3.0,
        CardKind::DigiEgg => 4.0,
        CardKind::Token => 5.0,
        CardKind::Dual => 6.0,
    }
}

fn relative_player(player: PlayerId, observer: PlayerId) -> f32 {
    if player == observer {
        1.0
    } else {
        -1.0
    }
}

fn write_phase_one_hot(t: &mut [f32], start: usize, phase: GamePhase) {
    let idx = phase.tensor_value() as usize;
    if idx < 20 {
        t[start + idx] = 1.0;
    }
}
