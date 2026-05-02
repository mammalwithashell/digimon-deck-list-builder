use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::StackPosition;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SourceSelectionRef;

fn ace_card(card_id: &str, card_name: &str) -> digimon_engine::CardData {
    let mut ace = make_test_card(card_id, card_name);
    ace.ace_overflow = Some(-4);
    ace
}

#[test]
fn ace_overflow_loses_memory_when_top_card_leaves_battle_area() {
    let ace = ace_card("ACE-RUNTIME", "Ace Runtime");

    let mut runner = DebugRunner::builder().add_card(ace).memory(3).start();

    let handle = runner.place_on_field(0, "ACE-RUNTIME", Some(0));
    runner.game.delete_permanent_with_effects(handle);

    assert_eq!(runner.game.memory, -1);
}

#[test]
fn ace_overflow_loses_memory_when_source_leaves_under_stack() {
    let ace = ace_card("ACE-SOURCE", "Ace Source");
    let top = make_test_card("TOP", "Top");

    let mut runner = DebugRunner::builder()
        .add_card(ace)
        .add_card(top)
        .memory(3)
        .start();

    let perm = runner.place_on_field(0, "ACE-SOURCE", Some(0));
    let top_data = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TOP")
        .expect("TOP card data");
    let top_card = CardSource::new(top_data, 0, runner.game.next_card_index());
    runner.game.players[0].battle_area[perm.index as usize]
        .card_sources
        .push(top_card);

    let source_card =
        runner.game.players[0].battle_area[perm.index as usize].card_sources[0].handle();
    let source_ref = SourceSelectionRef {
        permanent: PermanentHandle {
            player: 0,
            index: perm.index,
        },
        field_index: perm.index,
        source_index: 0,
        card: source_card,
    };

    assert!(runner.game.trash_source_ref(source_ref));
    assert_eq!(runner.game.memory, -1);
}

#[test]
fn ace_overflow_loses_memory_when_top_card_returns_to_hand() {
    let mut runner = DebugRunner::builder()
        .add_card(ace_card("ACE-HAND", "Ace Hand"))
        .memory(3)
        .start();

    let handle = runner.place_on_field(0, "ACE-HAND", Some(0));

    assert!(
        runner.game.return_to_hand(handle).is_some(),
        "ACE top card should return to hand"
    );
    assert_eq!(runner.game.memory, -1);
}

#[test]
fn ace_overflow_loses_memory_when_top_card_returns_to_deck() {
    let mut runner = DebugRunner::builder()
        .add_card(ace_card("ACE-DECK", "Ace Deck"))
        .memory(3)
        .start();

    let handle = runner.place_on_field(0, "ACE-DECK", Some(0));

    assert!(
        runner.game.return_to_deck(handle, StackPosition::Bottom),
        "ACE top card should return to deck"
    );
    assert_eq!(runner.game.memory, -1);
}

#[test]
fn ace_overflow_loses_memory_when_source_leaves_via_return_to_hand_stack_cleanup() {
    let mut runner = DebugRunner::builder()
        .add_card(ace_card("ACE-SOURCE-HAND", "Ace Source Hand"))
        .add_card(make_test_card("TOP-HAND", "Top Hand"))
        .memory(3)
        .start();

    let stack = runner.place_stack(0, &["ACE-SOURCE-HAND", "TOP-HAND"]);

    assert!(
        runner.game.return_to_hand(stack).is_some(),
        "top card should return to hand and sources should leave the stack"
    );
    assert_eq!(runner.game.memory, -1);
}

#[test]
fn ace_overflow_loses_memory_when_source_leaves_via_return_to_deck_stack_cleanup() {
    let mut runner = DebugRunner::builder()
        .add_card(ace_card("ACE-SOURCE-DECK", "Ace Source Deck"))
        .add_card(make_test_card("TOP-DECK", "Top Deck"))
        .memory(3)
        .start();

    let stack = runner.place_stack(0, &["ACE-SOURCE-DECK", "TOP-DECK"]);

    assert!(
        runner.game.return_to_deck(stack, StackPosition::Bottom),
        "top card should return to deck and sources should leave the stack"
    );
    assert_eq!(runner.game.memory, -1);
}
