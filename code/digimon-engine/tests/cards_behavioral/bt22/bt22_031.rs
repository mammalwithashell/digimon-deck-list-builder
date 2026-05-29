//! BT22-031 GoldNumemon.
//!
//! [On Play][When Digivolving] 1 opponent Digimon gains Security A. -2
//! until the end of their turn. Then, if this Digimon's stack has 2 or more
//! same-level cards, this Digimon may digivolve into [PlatinumNumemon] in hand
//! for cost 4, ignoring requirements.
//! Inherited: [Your Turn][OPT] When this Digimon would digivolve into a
//! Digimon card with the [CS] trait, reduce the digivolution cost by 1.

use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn bt22_031_on_play_gives_one_opponent_security_attack_minus_two() {
    let mut runner = goldnumemon_runner().start();
    let goldnumemon = runner.place_on_field(0, "BT22-031", Some(0));
    let opponent = runner.place_on_field(1, "OPPONENT", Some(0));

    fire_on_play(&mut runner, goldnumemon);
    choose_permanent(&mut runner, opponent);
    runner.auto_resolve().expect("finish Security A. debuff");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opponent, ModifierType::SecurityAttackChange),
        -2
    );
}

#[test]
fn bt22_031_same_level_pair_may_digivolve_into_platinum_numemon_from_hand() {
    let mut runner = goldnumemon_runner()
        .add_card(digimon_named("PLATINUM", "PlatinumNumemon", 5, 8000))
        .hand(0, &["PLATINUM"])
        .memory(10)
        .start();
    let goldnumemon = runner.place_stack(0, &["SRC-A", "SRC-B", "BT22-031"]);
    let opponent = runner.place_on_field(1, "OPPONENT", Some(0));

    fire_when_digivolving(&mut runner, goldnumemon);
    choose_permanent(&mut runner, opponent);

    let hand = runner
        .pending_selection_view()
        .expect("same-level pair should open optional PlatinumNumemon hand selection");
    assert_eq!(hand.kind, SelectionKind::Hand);
    assert!(hand.is_optional);

    let pick = hand
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .expect("PlatinumNumemon should be selectable");
    runner
        .execute_action(hand.selecting_player, pick)
        .expect("choose PlatinumNumemon");
    runner.auto_resolve().expect("finish effect digivolve");

    let stack = &runner.game.players[0].battle_area[goldnumemon.index as usize];
    assert_eq!(stack.top_card().card_id(&runner.game.card_data), "PLATINUM");
}

#[test]
fn bt22_031_without_same_level_pair_does_not_offer_platinum_numemon_selection() {
    let mut runner = goldnumemon_runner()
        .add_card(digimon_named("PLATINUM", "PlatinumNumemon", 5, 8000))
        .hand(0, &["PLATINUM"])
        .memory(10)
        .start();
    let goldnumemon = runner.place_stack(0, &["SRC-A", "SRC-C", "BT22-031"]);
    let opponent = runner.place_on_field(1, "OPPONENT", Some(0));

    fire_when_digivolving(&mut runner, goldnumemon);
    choose_permanent(&mut runner, opponent);
    runner.auto_resolve().expect("finish gated effect");

    assert!(
        runner.pending_selection_view().is_none(),
        "without a same-level source pair, no PlatinumNumemon selection should install"
    );
    let stack = &runner.game.players[0].battle_area[goldnumemon.index as usize];
    assert_eq!(stack.top_card().card_id(&runner.game.card_data), "BT22-031");
}

fn goldnumemon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT22-031")
        .expect("BT22-031 YAML loads")
        .add_card(digimon("SRC-A", 3, 1000))
        .add_card(digimon("SRC-B", 3, 1000))
        .add_card(digimon("SRC-C", 4, 1000))
        .add_card(digimon("OPPONENT", 4, 5000))
}

fn fire_on_play(runner: &mut DebugRunner, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn fire_when_digivolving(runner: &mut DebugRunner, source: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn choose_permanent(runner: &mut DebugRunner, target: PermanentHandle) {
    let view = runner
        .pending_selection_view()
        .expect("permanent target selection");
    runner
        .execute_action(view.selecting_player, encode_permanent(target))
        .expect("select permanent");
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    digimon_named(id, id, level, dp)
}

fn digimon_named(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}
