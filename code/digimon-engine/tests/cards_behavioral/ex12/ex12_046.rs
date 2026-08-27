use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::enums::{CardColor, ModifierType};

use super::support::{
    field_contains, fire_end_of_attack, fire_opponent_security_removed, hand_index, plain_digimon,
    select_first_non_pass, select_hand_card, tb_digimon, top_card_id, with_evo_cost, DebugRunner,
};

const CARD_ID: &str = "EX12-046";

#[test]
fn ex12_046_security_removed_effect_is_not_once_per_turn() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-046");
    let effect = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered
                    .when
                    .contains(&CompiledTiming::OnOpponentSecurityRemoved) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("printed opponent-security-removed effect");

    assert!(
        !effect.once_per_turn,
        "printed effect has no [Once Per Turn] tag"
    );
}

#[test]
fn ex12_046_on_play_gives_security_attack_minus_and_dp_minus_until_opponent_turn_end() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .add_card(plain_digimon("OPP", CardColor::Yellow, 4, 5000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP", Some(0));

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-046");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish debuff");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "target gains Security Attack -1"
    );
    assert_eq!(runner.effective_dp(opp), Some(2000));
}

#[test]
fn ex12_046_security_removed_may_digivolve_into_tb_from_hand_cost_reduced_by_two() {
    let tb_lv6 = with_evo_cost(
        tb_digimon("TB-LV6", CardColor::Yellow, 6, 11000),
        CardColor::Yellow,
        5,
        4,
    );
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .add_card(tb_lv6)
        .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
        .hand(0, &["TB-LV6"])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));

    fire_opponent_security_removed(&mut runner, 1);
    select_hand_card(&mut runner, 0, "TB-LV6");
    runner.auto_resolve().expect("finish effect digivolve");

    assert_eq!(top_card_id(&runner, 0, 0), "TB-LV6");
    assert_eq!(
        runner.memory(),
        3,
        "printed cost 4 is reduced by 2 and paid"
    );
}

#[test]
fn ex12_046_inherited_end_of_attack_plays_low_dp_tb_from_hand_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Yellow, 3, 3000))
        .add_card(tb_digimon("TB-LOW", CardColor::Yellow, 4, 5000))
        .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
        .hand(0, &["TB-LOW"])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    fire_end_of_attack(&mut runner, carrier);
    select_hand_card(&mut runner, 0, "TB-LOW");
    runner.auto_resolve().expect("finish inherited play");

    assert!(field_contains(&runner, 0, "TB-LOW"));
    assert_eq!(runner.memory(), 10, "the inherited play is free");
}

/// §8-1-3-3: "The player places the Digimon for the digivolution on top of the
/// chosen card, **draws 1 card**, and the digivolution process is resolved."
///
/// The draw is part of the digivolution PROCEDURE, so it applies to an
/// effect-initiated digivolve exactly as it does to the main-phase action --
/// the engine used to skip it on the effect path, reasoning that the draw was
/// "a player-action benefit, not an effect mechanic". §8-1-2-10 settles it the
/// other way: digivolution is possible when a draw isn't, and only then "is
/// performed without drawing a card", which presumes the draw otherwise.
///
/// Found by the DCGO card-clause exam (`qa/dcgo-exams/EX12/EX12-046-effect2.yaml`):
/// DCGO's hand held one more card than ours from this digivolve onward.
#[test]
fn ex12_046_effect_initiated_digivolve_still_draws_the_rule_8_1_3_3_card() {
    let tb_lv6 = with_evo_cost(
        tb_digimon("TB-LV6", CardColor::Yellow, 6, 11000),
        CardColor::Yellow,
        5,
        4,
    );
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-046 YAML loads")
        .add_card(tb_lv6)
        .add_card(plain_digimon("SEC", CardColor::Yellow, 3, 3000))
        .add_card(plain_digimon("DECKTOP", CardColor::Yellow, 3, 3000))
        .hand(0, &["TB-LV6"])
        .deck(0, &["DECKTOP"])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));

    let deck_before = runner.deck_size(0);

    fire_opponent_security_removed(&mut runner, 1);
    select_hand_card(&mut runner, 0, "TB-LV6");
    runner.auto_resolve().expect("finish effect digivolve");

    assert_eq!(top_card_id(&runner, 0, 0), "TB-LV6", "the digivolve resolved");
    // Hand: TB-LV6 left it (-1) and the §8-1-3-3 draw replaced it (+1).
    assert_eq!(
        runner.hand_size(0),
        1,
        "digivolving from hand draws 1, so the hand returns to its prior size"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "the drawn card came off the deck"
    );
}
