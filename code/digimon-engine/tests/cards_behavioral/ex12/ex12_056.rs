use digimon_engine::enums::{CardColor, Keyword};

use super::support::{
    black_sw, hand_index, plain_digimon, play_declining_optional_materials, select_first_non_pass,
    top_card_id, DebugRunner,
};

const CARD_ID: &str = "EX12-056";

#[test]
fn ex12_056_has_guard_keyword() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-056 YAML loads")
        .start();

    let cho_hakkaimon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(
        runner.game.has_keyword(cho_hakkaimon, Keyword::Guard),
        "Cho-Hakkaimon has printed <Guard>"
    );
}

#[test]
fn ex12_056_on_play_de_digivolves_then_grants_alliance_and_attack_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-056 YAML loads")
        .add_card(black_sw("SW-ALLY", 4))
        .add_card(plain_digimon("SRC", CardColor::Black, 3, 3000))
        .add_card(plain_digimon("OPP-LV4", CardColor::Black, 4, 5000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 3000))
        .hand(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let ally = runner.place_on_field(0, "SW-ALLY", Some(0));
    let opp = runner.place_stack(1, &["SRC", "OPP-LV4"]);

    let play_slot = hand_index(&runner, 0, CARD_ID);
    play_declining_optional_materials(&mut runner, 0, play_slot);
    select_first_non_pass(&mut runner);
    assert_eq!(
        top_card_id(&runner, 1, opp.index as usize),
        "SRC",
        "De-Digivolve 1 removes the opponent's top card"
    );

    select_first_non_pass(&mut runner);
    let view = runner
        .pending_selection_view()
        .expect("Alliance rider should expose optional attack targets");
    assert!(view.is_optional, "the immediate attack rider is optional");
    assert!(
        runner.game.has_keyword(ally, Keyword::Alliance),
        "the selected other [SW] Digimon gains Alliance"
    );
}

/// DCGO `SelectDigiXrosClass.Select` walks the recipe's elements and asks only for
/// an element that has candidates; after the last element it just ends. A
/// one-element recipe (this card: ONE [SW]/[Gokuumon] material) must therefore
/// commit the play right after its single material. The engine used to park a
/// second, EMPTY "Select DigiXros material" stop prompt that swallowed the next
/// recorded DCGO row (raw-20261113T025510Z#g8, Hakubamon EX12-043 -> EX12-056).
#[test]
fn ex12_056_single_element_digixros_commits_right_after_its_material() {
    use digimon_engine::selection::SelectionKind;

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-056 YAML loads")
        .add_card(black_sw("SW-MAT", 4))
        .add_card(black_sw("SW-MAT-2", 4))
        .hand(0, &[CARD_ID, "SW-MAT", "SW-MAT-2"])
        .memory(10)
        .start();
    let slot = hand_index(&runner, 0, CARD_ID);
    assert!(runner.play(0, slot).is_none(), "the optional material prompt parks first");
    let view = runner.pending_selection_view().expect("material prompt");
    assert_eq!(view.kind, SelectionKind::Material);
    assert!(view.is_optional, "DigiXros is declinable");
    select_first_non_pass(&mut runner);

    assert!(
        runner.pending_selection_view().map_or(true, |v| v.kind != SelectionKind::Material),
        "the one-element recipe is full: no second (empty) material prompt: {:?}",
        runner.pending_selection_view()
    );
    assert_eq!(runner.battle_area_size(0), 1, "the play committed");
    assert_eq!(top_card_id(&runner, 0, 0), CARD_ID);
    assert_eq!(
        runner.game.players[0].battle_area[0].card_sources.len(),
        2,
        "top card + the one DigiXros material"
    );
}
