//! ST12-12 Sistermon Blanc
//!
//! Implemented slice:
//! - [On Play] By trashing 1 card in your hand, Draw 2.
//!
//! Gap-routed slice:
//! - Conditional Decoy (Red/Black) while you have a Huckmon/Royal Knight.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST12-12")
        .expect("ST12-12 must load from embedded DSL pack")
        .memory(5)
        .start()
}

#[test]
fn st12_12_has_on_play_discard_draw_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("ST12-12")
        .expect("ST12-12 must be compiled");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )),
        "ST12-12 must have an OnPlay discard/draw clause"
    );
}

#[test]
fn st12_12_on_play_trashes_one_card_and_draws_two() {
    let discard = make_test_card("ST12-12-DISCARD", "Discard");
    let filler = make_test_card("ST12-12-FILLER", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("ST12-12")
        .expect("ST12-12 must load")
        .add_card(discard)
        .add_card(filler)
        .hand(0, &["ST12-12", "ST12-12-DISCARD"])
        .deck(0, &["ST12-12-FILLER", "ST12-12-FILLER"])
        .memory(5)
        .start();

    runner.play(0, 0);
    runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].trash.len(),
        1,
        "one card should be trashed"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        2,
        "Draw 2 should replace the discarded card"
    );
}

#[test]
fn st12_12_decoy_red_black_substitutes_self_for_ally_deletion() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST12-12")
        .expect("ST12-12 must load")
        .add_card(digimon("HUCKMON", "Huckmon", CardColor::Red, &[]))
        .add_card(digimon("RED-ALLY", "Red Ally", CardColor::Red, &[]))
        .memory(5)
        .start();
    let _decoy = runner.place_on_field(0, "ST12-12", Some(0));
    let _huckmon = runner.place_on_field(0, "HUCKMON", Some(0));
    let red_ally = runner.place_on_field(0, "RED-ALLY", Some(0));

    runner
        .game
        .delete_permanent_with_cause(red_ally, ReplacementCause::OpponentEffect);
    let view = runner
        .pending_selection_view()
        .expect("Decoy accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional, "Decoy can be declined");
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Decoy");
    runner.auto_resolve().expect("finish Decoy");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "RED-ALLY"),
        "red ally's deletion is prevented"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ST12-12"),
        "the Decoy source should leave the battle area"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "ST12-12"),
        "Decoy deletes Sistermon Blanc as the substitute"
    );
}

#[test]
fn st12_12_decoy_ignores_non_red_black_or_missing_enabler() {
    let mut no_enabler = DebugRunner::builder()
        .dsl_card("ST12-12")
        .expect("ST12-12 must load")
        .add_card(digimon("RED-ALLY", "Red Ally", CardColor::Red, &[]))
        .memory(5)
        .start();
    let _decoy = no_enabler.place_on_field(0, "ST12-12", Some(0));
    let red_ally = no_enabler.place_on_field(0, "RED-ALLY", Some(0));

    no_enabler
        .game
        .delete_permanent_with_cause(red_ally, ReplacementCause::OpponentEffect);
    assert!(
        no_enabler.pending_selection_view().is_none(),
        "Decoy requires a Huckmon/Royal Knight enabler"
    );
    assert_eq!(no_enabler.battle_area_size(0), 1);

    let mut blue = DebugRunner::builder()
        .dsl_card("ST12-12")
        .expect("ST12-12 must load")
        .add_card(digimon("HUCKMON", "Huckmon", CardColor::Red, &[]))
        .add_card(digimon("BLUE-ALLY", "Blue Ally", CardColor::Blue, &[]))
        .memory(5)
        .start();
    let _decoy = blue.place_on_field(0, "ST12-12", Some(0));
    let _huckmon = blue.place_on_field(0, "HUCKMON", Some(0));
    let blue_ally = blue.place_on_field(0, "BLUE-ALLY", Some(0));

    blue.game
        .delete_permanent_with_cause(blue_ally, ReplacementCause::OpponentEffect);
    assert!(
        blue.pending_selection_view().is_none(),
        "Decoy (Red/Black) ignores blue Digimon"
    );
    assert_eq!(blue.battle_area_size(0), 2);
}

fn digimon(id: &str, name: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.traits = traits.iter().map(|t| (*t).to_string()).collect();
    card
}
