use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT19-026";

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-026 YAML loads")
        .add_card(digimon("SRC-A", "Source A", 5, 7000))
        .add_card(digimon("SRC-B", "Source B", 5, 7000))
        .add_card(digimon("OPP-TOP", "Opponent Top", 6, 11000))
        .add_card(digimon("LOWER", "Lower", 4, 4000))
        .memory(10)
        .start()
}

#[test]
fn bt19_026_de_digivolves_then_returns_level_four_when_two_opponent_digimon() {
    let mut runner = runner();
    let zeig = runner.place_on_field(0, CARD_ID, Some(0));
    let stacked = runner.place_stack(1, &["SRC-A", "SRC-B", "OPP-TOP"]);
    let _lower = runner.place_on_field(1, "LOWER", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(zeig));
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("De-Digivolve target selection should install");
    assert!(pending
        .valid_action_ids
        .contains(&encode_attack(0, stacked.index as u16)));
    runner
        .execute_action(0, encode_attack(0, stacked.index as u16))
        .expect("choose stacked target");
    runner.auto_resolve().expect("resolve follow-up return");

    assert_eq!(
        runner.game.players[1].battle_area[stacked.index as usize]
            .card_sources
            .len(),
        1,
        "De-Digivolve 2 should peel the two top stacked source cards"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .all(|perm| perm.top_card().card_id(&runner.game.card_data) != "LOWER"),
        "level 4 follower should be returned to hand when opponent has 2+ Digimon"
    );
    assert!(runner.game.players[1]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "LOWER"));
}
