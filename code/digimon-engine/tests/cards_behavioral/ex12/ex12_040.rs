use digimon_engine::enums::{CardColor, Keyword, PlaySource};

use super::support::{
    hand_index, plain_digimon, top_card_id, vb_digimon, with_evo_cost, DebugRunner,
};

const CARD_ID: &str = "EX12-040";

#[test]
fn ex12_040_reduces_digivolve_into_holy_beast_or_vb_by_1() {
    let holy_beast = with_evo_cost(
        super::support::digimon(
            "HOLY-BEAST-LV4",
            CardColor::Yellow,
            4,
            5,
            5000,
            &["Holy Beast"],
        ),
        CardColor::Yellow,
        3,
        1,
    );

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-040 YAML loads")
        .add_card(holy_beast)
        .hand(0, &["HOLY-BEAST-LV4"])
        .memory(5)
        .start();

    let salamon = runner.place_on_field(0, CARD_ID, Some(0));
    let slot = hand_index(&runner, 0, "HOLY-BEAST-LV4");
    let memory_before = runner.memory();
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, slot, salamon.index as usize, PlaySource::ByHand);

    assert!(digivolved, "Salamon should digivolve into the Holy Beast");
    assert_eq!(runner.memory(), memory_before, "cost 1 should reduce to 0");
    assert_eq!(
        top_card_id(&runner, 0, salamon.index as usize),
        "HOLY-BEAST-LV4"
    );
}

#[test]
fn ex12_040_does_not_reduce_non_holy_beast_or_vb_digivolve() {
    let plain_lv4 = with_evo_cost(
        plain_digimon("PLAIN-LV4", CardColor::Yellow, 4, 5000),
        CardColor::Yellow,
        3,
        1,
    );

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-040 YAML loads")
        .add_card(plain_lv4)
        .hand(0, &["PLAIN-LV4"])
        .memory(5)
        .start();

    let salamon = runner.place_on_field(0, CARD_ID, Some(0));
    let slot = hand_index(&runner, 0, "PLAIN-LV4");
    let memory_before = runner.memory();
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, slot, salamon.index as usize, PlaySource::ByHand);

    assert!(
        digivolved,
        "normal yellow Lv4 digivolution should still be legal"
    );
    assert_eq!(memory_before - runner.memory(), 1);
}

#[test]
fn ex12_040_inherited_barrier_is_available_from_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-040 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Yellow, 4, 5000))
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "carrier inherits Barrier from EX12-040"
    );
}
