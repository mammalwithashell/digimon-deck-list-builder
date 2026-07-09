use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost, CompiledPredicate};
use digimon_engine::action::space::PASS;
use digimon_engine::enums::CardColor;

use super::support::{field_contains, plain_digimon, select_first_non_pass, DebugRunner};

const CARD_ID: &str = "EX12-035";

fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(pred: &CompiledPredicate, f: F) -> bool {
    f(pred)
        || pred.all_of.iter().any(|child| pred_any(child, f))
        || pred.any_of.iter().any(|child| pred_any(child, f))
        || pred.none_of.iter().any(|child| pred_any(child, f))
        || pred
            .not
            .as_deref()
            .is_some_and(|child| pred_any(child, f))
}

#[test]
fn ex12_035_has_printed_three_material_assembly_route() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-035 YAML loads")
        .start();

    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX12-035 should be compiled");
    let assembly = card
        .alt_paths
        .iter()
        .find(|path| path.kind == CompiledAltPathKind::Assembly)
        .expect("EX12-035 prints Assembly -6 and must compile an assembly route");

    assert_eq!(assembly.cost, Some(CompiledCost::Literal(6)));
    assert_eq!(
        assembly.materials.len(),
        3,
        "Assembly -6 requires Lv.5 x Lv.4 x Lv.3 materials"
    );

    for (material, level) in assembly.materials.iter().zip([5, 4, 3]) {
        assert!(material.stack_under, "assembly material should stack under");
        assert!(
            pred_any(&material.filter, |pred| pred.level_eq == Some(level)),
            "assembly material should require level {level}: {:?}",
            material.filter
        );
        assert!(
            pred_any(&material.filter, |pred| {
                pred.name_contains.as_deref() == Some("Gabumon")
                    || pred.name_contains.as_deref() == Some("Garurumon")
                    || pred.trait_has.as_deref() == Some("ME")
                    || pred.trait_has.as_deref() == Some("VB")
            }),
            "assembly material should require [Gabumon]/[Garurumon]/[ME]/[VB]: {:?}",
            material.filter
        );
    }
}

#[test]
fn ex12_035_on_play_bottom_decks_opponent_with_lte_source_count() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-035 YAML loads")
        .add_card(plain_digimon("OWN-MAT1", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("OWN-MAT2", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("OPP-A1", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("OPP-A2", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("OPP-A3", CardColor::Blue, 5, 7000))
        .add_card(plain_digimon("OPP-HIGH", CardColor::Blue, 6, 10000))
        .add_card(plain_digimon("OPP-B1", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("OPP-LOW", CardColor::Blue, 4, 4000))
        .start();

    let metalgarurumon = runner.place_stack(0, &["OWN-MAT1", "OWN-MAT2", CARD_ID]);
    runner.place_stack(1, &["OPP-A1", "OPP-A2", "OPP-A3", "OPP-HIGH"]);
    runner.place_stack(1, &["OPP-B1", "OPP-LOW"]);

    runner.fire_on_play(0, metalgarurumon.index as usize);
    let view = runner
        .pending_selection_view()
        .expect("source-trash prompt should be optional");
    assert!(view.valid_action_ids.contains(&PASS));
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline source trash");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve EX12-035 on-play");

    assert!(field_contains(&runner, 1, "OPP-HIGH"));
    assert!(!field_contains(&runner, 1, "OPP-LOW"));
}
