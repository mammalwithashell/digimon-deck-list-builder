use crate::dsl_card_data::compiled;

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledColor, CompiledCost, CompiledPredicate};
use digimon_engine::enums::CardColor;

use super::support::{hand_ids, plain_digimon, vb_digimon, vb_text_digimon, DebugRunner};

const CARD_ID: &str = "EX12-007";

#[test]
fn ex12_007_printed_metadata_and_digivolve_paths() {
    let card = compiled(CARD_ID);

    assert_eq!(card.attribute.as_deref(), Some("Virus"));
    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(1))
                && path.from.as_deref().is_some_and(|pred| {
                    predicate_has_level_eq(pred, 2)
                        && predicate_has_color_is(pred, CompiledColor::Red)
                })
        }),
        "standard printed red Lv.2 digivolve path costs 1"
    );

    let special = card
        .alt_paths
        .iter()
        .find(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_deref().is_some_and(|pred| {
                    predicate_has_name_is(pred, "Gurimon")
                        && predicate_has_level_eq(pred, 2)
                        && predicate_has_trait_has(pred, "VB")
                })
        })
        .expect("special [Gurimon]/Lv.2 [VB] digivolve path costs 0");

    assert!(
        !special
            .from
            .as_deref()
            .is_some_and(|pred| predicate_has_color_is(pred, CompiledColor::Yellow)),
        "yellow Lv.2 is not a standalone cost-0 digivolve path"
    );
}

#[test]
fn ex12_007_on_play_reveals_three_adds_gammamon_text_and_vb_cards_rest_to_bottom() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-007 YAML loads")
        .add_card(vb_text_digimon("GAMMA-TEXT", CardColor::Red, 3, 2000))
        .add_card(vb_digimon("VB-PICK", CardColor::Yellow, 3, 2000))
        .add_card(plain_digimon("FILLER", CardColor::Blue, 3, 2000))
        .add_card(vb_digimon("PAD", CardColor::Red, 3, 2000))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD", "GAMMA-TEXT", "VB-PICK", "FILLER"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.play(0, 0).expect("play EX12-007");
    runner.auto_resolve().expect("resolve EX12-007 reveal");

    let hand = hand_ids(&runner, 0);
    assert!(hand.iter().any(|id| id == "GAMMA-TEXT"), "hand={hand:?}");
    assert!(hand.iter().any(|id| id == "VB-PICK"), "hand={hand:?}");
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "two selected cards leave the deck; one reveal remainder returns to bottom"
    );
}

#[test]
fn ex12_007_inherited_dp_aura_applies_only_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-007 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Yellow, 3, 3000))
        .memory(1)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(runner.effective_dp(carrier), Some(5000));

    runner.end_turn();
    assert_eq!(runner.effective_dp(carrier), Some(3000));
}

fn predicate_has_level_eq(pred: &CompiledPredicate, level: u8) -> bool {
    pred.level_eq == Some(level)
        || pred.all_of.iter().any(|p| predicate_has_level_eq(p, level))
        || pred.any_of.iter().any(|p| predicate_has_level_eq(p, level))
}

fn predicate_has_color_is(pred: &CompiledPredicate, color: CompiledColor) -> bool {
    pred.color_is == Some(color)
        || pred.all_of.iter().any(|p| predicate_has_color_is(p, color))
        || pred.any_of.iter().any(|p| predicate_has_color_is(p, color))
}

fn predicate_has_name_is(pred: &CompiledPredicate, name: &str) -> bool {
    pred.name_is.as_deref() == Some(name)
        || pred.all_of.iter().any(|p| predicate_has_name_is(p, name))
        || pred.any_of.iter().any(|p| predicate_has_name_is(p, name))
}

fn predicate_has_trait_has(pred: &CompiledPredicate, trait_name: &str) -> bool {
    pred.trait_has.as_deref() == Some(trait_name)
        || pred
            .all_of
            .iter()
            .any(|p| predicate_has_trait_has(p, trait_name))
        || pred
            .any_of
            .iter()
            .any(|p| predicate_has_trait_has(p, trait_name))
}
