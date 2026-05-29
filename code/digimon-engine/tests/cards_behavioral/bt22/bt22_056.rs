//! BT22-056 Guardromon.
//!
//! [On Play][When Digivolving] 1 opponent Digimon gets -3000 DP for the turn.
//! Then, if this Digimon's stack has 2 or more same-level cards,
//! <De-Digivolve 1> 1 opponent Digimon.
//! Inherited: [Opponent's Turn] This Digimon gets +2000 DP.

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn bt22_056_on_play_gives_one_opponent_digimon_minus_3000_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-056")
        .expect("BT22-056 YAML loads")
        .add_card(digimon("OPP", 4, 5000))
        .start();
    let guardromon = runner.place_on_field(0, "BT22-056", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    fire_on_play(&mut runner, guardromon);
    choose_permanent(&mut runner, opp);
    runner.auto_resolve().expect("finish DP debuff");

    assert_eq!(runner.dp_of(opp), Some(2000));
}

#[test]
fn bt22_056_when_digivolving_same_level_pair_de_digivolves_one_opponent() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-056")
        .expect("BT22-056 YAML loads")
        .add_card(digimon("SRC-A", 3, 1000))
        .add_card(digimon("SRC-B", 3, 1000))
        .add_card(digimon("OPP-BASE", 3, 3000))
        .add_card(digimon("OPP-TOP", 4, 5000))
        .start();
    let guardromon = runner.place_stack(0, &["SRC-A", "SRC-B", "BT22-056"]);
    let opp = runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);

    fire_when_digivolving(&mut runner, guardromon);
    choose_permanent(&mut runner, opp);

    let de_digivolve = runner
        .pending_selection_view()
        .expect("same-level pair should unlock De-Digivolve target");
    assert_eq!(
        de_digivolve.kind,
        SelectionKind::CountCappedMultiSelect { max: 1, picked: 0 }
    );
    assert_eq!(
        de_digivolve.valid_action_ids.len(),
        1,
        "only the opponent stack should be selectable for De-Digivolve"
    );
    runner
        .execute_action(
            de_digivolve.selecting_player,
            de_digivolve.valid_action_ids[0],
        )
        .expect("choose De-Digivolve target");
    runner.auto_resolve().expect("finish De-Digivolve");

    assert_eq!(stack_size(&runner, opp), 1);
}

#[test]
fn bt22_056_without_same_level_pair_only_applies_dp_debuff() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-056")
        .expect("BT22-056 YAML loads")
        .add_card(digimon("SRC-L2", 2, 1000))
        .add_card(digimon("SRC-L3", 3, 1000))
        .add_card(digimon("OPP-BASE", 3, 3000))
        .add_card(digimon("OPP-TOP", 4, 5000))
        .start();
    let guardromon = runner.place_stack(0, &["SRC-L2", "SRC-L3", "BT22-056"]);
    let opp = runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);

    fire_when_digivolving(&mut runner, guardromon);
    choose_permanent(&mut runner, opp);
    runner.auto_resolve().expect("finish effect");

    assert!(runner.pending_selection_view().is_none());
    assert_eq!(stack_size(&runner, opp), 2);
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

fn stack_size(runner: &DebugRunner, handle: PermanentHandle) -> usize {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .len()
}

fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}
