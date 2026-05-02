//! GameEvent accumulator is drained per step. TEST-001 ("On Play: Gain 1
//! memory") should emit at minimum a Play event and a MemoryChange event
//! when played.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::events::GameEvent;

#[test]
fn memory_change_event_emitted_on_gain_memory() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();
    r.game.drain_events(); // clear any startup events

    r.game.gain_memory(2);

    let events = r.game.drain_events();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, GameEvent::MemoryChange { delta: 2, .. })),
        "gain_memory(2) should emit MemoryChange {{ delta: 2 }}; got {:?}",
        events,
    );
}

#[test]
fn play_and_memory_events_emitted_on_play() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();
    r.game.drain_events(); // clear startup events
    r.play(0, 0);

    let events = r.game.drain_events();
    let has_play = events.iter().any(|e| matches!(e, GameEvent::Play { .. }));
    let has_memory = events
        .iter()
        .any(|e| matches!(e, GameEvent::MemoryChange { .. }));
    assert!(
        has_play,
        "expected a Play event after r.play(); got {:?}",
        events
    );
    assert!(
        has_memory,
        "expected MemoryChange events (pay cost + OnPlay gain); got {:?}",
        events
    );
}

#[test]
fn events_have_monotonic_seq_numbers() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();
    r.game.drain_events();

    r.game.gain_memory(1);
    r.game.gain_memory(1);
    r.game.gain_memory(1);

    let events = r.game.drain_events();
    let seqs: Vec<u64> = events.iter().map(|e| e.seq()).collect();
    let sorted: Vec<u64> = {
        let mut s = seqs.clone();
        s.sort_unstable();
        s
    };
    assert_eq!(seqs, sorted, "seq must be monotonic; got {:?}", seqs);
    for w in seqs.windows(2) {
        assert!(w[1] > w[0], "seq must be strictly increasing");
    }
}

#[test]
fn drain_events_clears_buffer() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();
    let _ = r.game.drain_events();
    r.game.gain_memory(1);
    let first = r.game.drain_events();
    assert_eq!(first.len(), 1);
    let second = r.game.drain_events();
    assert!(
        second.is_empty(),
        "drain_events must clear the buffer; got {:?}",
        second
    );
}
