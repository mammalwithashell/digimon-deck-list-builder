use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::make_test_card_with_level;
pub(super) use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{AttackTarget, SelectionKind, TriggerSource};
use digimon_engine::trigger_context::AttackTargetChangeReason;
use digimon_engine::trigger_context::EventCause;

pub(super) fn digimon(
    id: &str,
    color: CardColor,
    level: u8,
    play_cost: u16,
    dp: i32,
    traits: &[&str],
) -> CardData {
    let mut card = make_test_card_with_level(id, id, level);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.play_cost = play_cost;
    card.dp = Some(dp);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

pub(super) fn red_sw(id: &str, level: u8) -> CardData {
    digimon(
        id,
        CardColor::Red,
        level,
        if level <= 3 { 3 } else { 4 },
        3000,
        &["SW"],
    )
}

pub(super) fn blue_sw(id: &str, level: u8) -> CardData {
    digimon(
        id,
        CardColor::Blue,
        level,
        if level <= 3 { 3 } else { 5 },
        3000,
        &["SW"],
    )
}

pub(super) fn yellow_sw(id: &str, level: u8) -> CardData {
    digimon(
        id,
        CardColor::Yellow,
        level,
        if level <= 3 { 3 } else { 5 },
        3000,
        &["SW"],
    )
}

pub(super) fn black_sw(id: &str, level: u8) -> CardData {
    digimon(
        id,
        CardColor::Black,
        level,
        if level <= 3 { 3 } else { 5 },
        3000,
        &["SW"],
    )
}

fn color_id(color: CardColor) -> u8 {
    match color {
        CardColor::Red => 0,
        CardColor::Blue => 1,
        CardColor::Yellow => 2,
        CardColor::Green => 3,
        CardColor::White => 4,
        CardColor::Black => 5,
        CardColor::Purple => 6,
    }
}

pub(super) fn plain_digimon(id: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    digimon(id, color, level, if level <= 3 { 3 } else { 5 }, dp, &[])
}

pub(super) fn tamer(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card_with_level(id, id, 0);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.colors = vec![color];
    card.play_cost = 3;
    card.traits = Vec::new();
    card
}

pub(super) fn vb_digimon(id: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    digimon(
        id,
        color,
        level,
        if level <= 3 {
            3
        } else if level == 4 {
            5
        } else {
            7
        },
        dp,
        &["VB"],
    )
}

pub(super) fn vb_text_digimon(id: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut card = plain_digimon(id, color, level, dp);
    card.effect_text = "This card has [Gammamon] in its text.".to_string();
    card
}

pub(super) fn tb_digimon(id: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    digimon(
        id,
        color,
        level,
        if level <= 3 {
            3
        } else if level == 4 {
            5
        } else {
            7
        },
        dp,
        &["Shambala", "TB"],
    )
}

pub(super) fn puppet_tb(id: &str, level: u8) -> CardData {
    digimon(
        id,
        CardColor::Purple,
        level,
        if level <= 3 {
            3
        } else if level == 4 {
            4
        } else {
            7
        },
        if level <= 4 { 5000 } else { 7000 },
        &["Puppet", "Shambala", "TB"],
    )
}

pub(super) fn with_evo_cost(
    mut card: CardData,
    from_color: CardColor,
    from_level: u8,
    memory_cost: u8,
) -> CardData {
    card.evo_costs = vec![EvoCost {
        card_color: color_id(from_color),
        level: from_level,
        memory_cost: memory_cost.into(),
    }];
    card
}

pub(super) fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

pub(super) fn hand_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

pub(super) fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

pub(super) fn trash_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

pub(super) fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must exist in card_data"));
    let card = CardSource::new(data_idx, player, runner.game.next_card_index());
    runner.game.players[player as usize].trash.push(card);
}

pub(super) fn field_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|card| card.top_card().card_id(&runner.game.card_data) == card_id)
}

pub(super) fn source_count(runner: &DebugRunner, player: u8, field_index: usize) -> usize {
    runner.game.players[player as usize].battle_area[field_index]
        .card_sources
        .len()
        .saturating_sub(1)
}

pub(super) fn bottom_source_id(runner: &DebugRunner, player: u8, field_index: usize) -> String {
    runner.game.players[player as usize].battle_area[field_index].card_sources[0]
        .card_id(&runner.game.card_data)
        .to_string()
}

pub(super) fn is_suspended(runner: &DebugRunner, player: u8, field_index: usize) -> bool {
    runner.game.players[player as usize].battle_area[field_index].is_suspended
}

pub(super) fn top_card_id(runner: &DebugRunner, player: u8, field_index: usize) -> String {
    runner.game.players[player as usize].battle_area[field_index]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

pub(super) fn select_hand_card(runner: &mut DebugRunner, player: u8, card_id: &str) {
    for _ in 0..8 {
        let view = runner
            .pending_selection_view()
            .unwrap_or_else(|| panic!("{card_id} should be selectable from hand"));
        if view.kind == SelectionKind::Hand {
            let slot = hand_index(runner, player, card_id) as u16;
            let action = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
                .into_iter()
                .find(|action| view.valid_action_ids.contains(action))
                .unwrap_or_else(|| panic!("{card_id} was not selectable from hand; view={view:?}"));
            runner
                .execute_action(view.selecting_player, action)
                .expect("select hand card");
            return;
        }
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|action| *action != PASS)
            .unwrap_or_else(|| panic!("unexpected prompt with only PASS: {view:?}"));
        runner
            .execute_action(view.selecting_player, action)
            .expect("advance pending prompt");
    }
    panic!("{card_id} never reached a hand selection");
}

pub(super) fn select_first_non_pass(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("expected pending selection");
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action != PASS)
        .unwrap_or_else(|| panic!("pending selection had no non-PASS action: {view:?}"));
    runner
        .execute_action(view.selecting_player, action)
        .expect("select first legal action");
}

pub(super) fn fire_own_security_removed(runner: &mut DebugRunner, affected_player: u8) {
    let removed_card = runner.game.players[affected_player as usize]
        .security
        .first()
        .map(|c| c.handle())
        .unwrap_or(digimon_engine::card_source::CardHandle(0));
    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player,
            observer_player: affected_player,
            source_player: 1 - affected_player,
            card: removed_card,
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();
}

pub(super) fn fire_opponent_security_removed(runner: &mut DebugRunner, affected_player: u8) {
    let removed_card = runner.game.players[affected_player as usize]
        .security
        .first()
        .map(|c| c.handle())
        .unwrap_or(digimon_engine::card_source::CardHandle(0));
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player,
            observer_player: 1 - affected_player,
            source_player: 1 - affected_player,
            card: removed_card,
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();
}

pub(super) fn fire_end_of_attack(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

pub(super) fn fire_when_digivolving(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

pub(super) fn fire_attack_target_changed(
    runner: &mut DebugRunner,
    attacker: digimon_engine::permanent::PermanentHandle,
    old_target: AttackTarget,
    new_target: AttackTarget,
    reason: AttackTargetChangeReason,
) {
    let card = runner.game.players[attacker.player as usize].battle_area[attacker.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
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
    runner.game.drain_effect_queue();
}
