use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SourceSelectionRef;

#[test]
fn ace_overflow_loses_memory_when_top_card_leaves_battle_area() {
    let mut ace = make_test_card("ACE-RUNTIME", "Ace Runtime");
    ace.ace_overflow = Some(-4);

    let mut runner = DebugRunner::builder().add_card(ace).memory(3).start();

    let handle = runner.place_on_field(0, "ACE-RUNTIME", Some(0));
    runner.game.delete_permanent_with_effects(handle);

    assert_eq!(runner.game.memory, -1);
}

#[test]
fn ace_overflow_loses_memory_when_source_leaves_under_stack() {
    let mut ace = make_test_card("ACE-SOURCE", "Ace Source");
    ace.ace_overflow = Some(-4);
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
