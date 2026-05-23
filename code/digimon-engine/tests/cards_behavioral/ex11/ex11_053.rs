//! EX11-053 Omekamon.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

fn digimon(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-053")
        .expect("EX11-053 YAML loads")
        .add_card(digimon("KING", "King Drasil_7D6", &["CS"]))
        .add_card(digimon("ROYAL", "Magnamon", &["Royal Knight"]))
        .add_card(digimon("NON-RK", "Plain Digimon", &[]))
        .add_card(make_test_card("DRAW", "Draw Filler"))
        .memory(10)
        .start()
}

#[test]
fn ex11_053_has_printed_metadata_and_on_play_clause() {
    let runner = runner();
    let card = runner.compiled_card("EX11-053").expect("compiled card");

    assert_eq!(card.name, "Omekamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.color, vec![CompiledColor::Black]);
    assert!(card.traits.iter().any(|name| name == "Puppet"));
    assert!(card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => {
            triggered.when.contains(&CompiledTiming::OnPlay)
                && triggered
                    .process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::PlaceAsBottomSource { .. }))
                && triggered
                    .process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::Draw { count: 1, .. }))
        }
        _ => false,
    }));
}

#[test]
fn ex11_053_on_play_places_royal_knight_from_hand_under_fielded_king_drasil() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-053")
        .expect("EX11-053 YAML loads")
        .add_card(digimon("KING", "King Drasil_7D6", &["CS"]))
        .add_card(digimon("ROYAL", "Magnamon", &["Royal Knight"]))
        .add_card(digimon("NON-RK", "Plain Digimon", &[]))
        .add_card(make_test_card("DRAW", "Draw Filler"))
        .hand(0, &["EX11-053", "ROYAL", "NON-RK"])
        .deck(0, &["DRAW"])
        .memory(20)
        .start();
    let king = runner.place_on_field(0, "KING", Some(0));
    let royal_handle = runner.game.players[0].hand[1].handle();

    runner.play(0, 0).expect("play Omekamon");
    let king_prompt = runner
        .pending_selection_view()
        .expect("King Drasil selection opens");
    assert_eq!(king_prompt.kind, SelectionKind::OwnField);
    assert_eq!(
        king_prompt.valid_action_ids,
        vec![encode_attack(0, king.index as u16)]
    );
    runner
        .execute_action(0, encode_attack(0, king.index as u16))
        .expect("choose King Drasil");

    let hand_prompt = runner
        .pending_selection_view()
        .expect("Royal Knight hand selection opens");
    assert_eq!(hand_prompt.kind, SelectionKind::Hand);
    assert_eq!(
        hand_prompt.valid_action_ids,
        vec![PLAY_HAND_START],
        "after Omekamon leaves hand, only Magnamon is a legal Royal Knight"
    );
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("choose Magnamon");
    runner.auto_resolve().expect("place source and draw");

    let king_perm = &runner.game.players[0].battle_area[king.index as usize];
    assert_eq!(king_perm.card_sources[0].handle(), royal_handle);
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "DRAW"),
        "Omekamon draws after the Royal Knight card is placed"
    );
}

#[test]
fn ex11_053_on_play_short_circuits_without_fielded_king_drasil() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-053")
        .expect("EX11-053 YAML loads")
        .add_card(digimon("ROYAL", "Magnamon", &["Royal Knight"]))
        .add_card(make_test_card("DRAW", "Draw Filler"))
        .hand(0, &["EX11-053", "ROYAL"])
        .deck(0, &["DRAW"])
        .memory(20)
        .start();

    runner.play(0, 0).expect("play Omekamon");

    assert!(runner.pending_selection().is_none());
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "ROYAL"),
        "Royal Knight stays in hand when there is no fielded King Drasil"
    );
    assert!(
        runner.game.players[0]
            .deck
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "DRAW"),
        "draw does not occur when the placement cost cannot be paid"
    );
}

#[ignore = "pending: G-UNION-HAND-SOURCE-PLAY — On Deletion low-security Omnimon X from hand/source, then attach this card; tracker: qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md"]
#[test]
fn ex11_053_on_deletion_union_hand_source_play_and_attach_self() {}
