//! Physical source pins for the new owned chooser; no legacy action decoding.
use digimon_engine::card_choice_page::SourceChoicePin;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("TEST-PIN-SOURCE", "Source"))
        .add_card(make_test_card("TEST-PIN-TOP", "Top"))
        .deck(0, &["TEST-PIN-SOURCE"; 20])
        .deck(1, &["TEST-PIN-SOURCE"; 20])
        .start()
}

#[test]
fn a_deep_physical_source_survives_clone_and_battle_compaction() {
    let mut r = runner();
    r.place_stack(0, &["TEST-PIN-TOP"]);
    let mut stack = vec!["TEST-PIN-SOURCE"; 31];
    stack.push("TEST-PIN-TOP");
    let host = r.place_stack(0, &stack);
    let card = r.game.players[0].battle_area[1].card_sources[30].handle();
    let pin = SourceChoicePin::capture(&mut r.game, host, card).unwrap();
    let mut fork = r.game.clone();
    assert_eq!(pin.resolve(&fork), Some((host, 30)));
    fork.players[0].battle_area.remove(0);
    let moved_host = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(pin.resolve(&fork), Some((moved_host, 30)));
    assert_eq!(pin.resolve(&r.game), Some((host, 30)));
    fork.players[0].battle_area[0].card_sources.remove(0);
    assert_eq!(pin.resolve(&fork), Some((moved_host, 29)));
}

#[test]
fn pin_cannot_bind_a_replacement_at_an_old_source_position() {
    let mut r = runner();
    let host = r.place_stack(0, &["TEST-PIN-SOURCE", "TEST-PIN-SOURCE", "TEST-PIN-TOP"]);
    let card = r.game.players[0].battle_area[0].card_sources[0].handle();
    let pin = SourceChoicePin::capture(&mut r.game, host, card).unwrap();
    r.game.players[0].battle_area[0].card_sources.remove(0);
    assert_eq!(pin.resolve(&r.game), None);
}

#[test]
fn pin_rejects_a_replaced_carrier_or_a_source_moved_to_another_carrier() {
    let mut r = runner();
    let host = r.place_stack(0, &["TEST-PIN-SOURCE", "TEST-PIN-TOP"]);
    r.place_stack(0, &["TEST-PIN-TOP"]);
    let card = r.game.players[0].battle_area[0].card_sources[0].handle();
    let pin = SourceChoicePin::capture(&mut r.game, host, card).unwrap();
    let source = r.game.players[0].battle_area[0].card_sources.remove(0);
    r.game.players[0].battle_area[1]
        .card_sources
        .insert(0, source);
    assert_eq!(pin.resolve(&r.game), None);
    r.game.players[0].battle_area.remove(0);
    assert_eq!(pin.resolve(&r.game), None);
}

#[test]
fn breeding_pins_resolve_deep_sources_and_reject_source_becoming_top() {
    let mut r = runner();
    let mut stack = vec!["TEST-PIN-SOURCE"; 40];
    stack.push("TEST-PIN-TOP");
    r.place_stack(0, &stack);
    r.game.players[0].breeding_area = Some(r.game.players[0].battle_area.remove(0));
    let host = PermanentHandle {
        player: 0,
        index: digimon_engine::action::space::BREEDING_TARGET as u8,
    };
    let card = r.game.players[0]
        .breeding_area
        .as_ref()
        .unwrap()
        .card_sources[39]
        .handle();
    let pin = SourceChoicePin::capture(&mut r.game, host, card).unwrap();
    assert_eq!(pin.resolve(&r.game.clone()), Some((host, 39)));
    r.game.players[0]
        .breeding_area
        .as_mut()
        .unwrap()
        .card_sources
        .pop();
    assert_eq!(pin.resolve(&r.game), None);
}

#[test]
fn top_cards_and_wrong_carriers_are_never_source_pins() {
    let mut r = runner();
    let host = r.place_stack(0, &["TEST-PIN-SOURCE", "TEST-PIN-TOP"]);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    assert!(SourceChoicePin::capture(&mut r.game, host, top).is_none());
    assert!(SourceChoicePin::capture(
        &mut r.game,
        PermanentHandle {
            player: 1,
            index: 0
        },
        top
    )
    .is_none());
}

#[test]
fn same_printing_replacement_at_the_old_slot_is_a_different_carrier() {
    let mut r = runner();
    let host = r.place_stack(0, &["TEST-PIN-SOURCE", "TEST-PIN-TOP"]);
    let card = r.game.players[0].battle_area[0].card_sources[0].handle();
    let pin = SourceChoicePin::capture(&mut r.game, host, card).unwrap();
    let mut old = r.game.players[0].battle_area.remove(0);
    let source = old.card_sources.remove(0);
    let replacement = r.place_stack(0, &["TEST-PIN-TOP"]);
    assert_eq!(replacement, host);
    r.game.players[0].battle_area[0]
        .card_sources
        .insert(0, source);
    assert_eq!(pin.resolve(&r.game), None);
}
