//! Effect-specified reductions obey play-cost prohibitions; assignments/free do not.
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    enums::{CostDelta, Expiry, ModifierType, PlaySource},
    modifiers::PlayerModifierEntry,
};
fn runner(cost: u16, memory: i16, lock: Option<(u8, ModifierType)>) -> DebugRunner {
    let mut c = make_test_card("TARGET", "Target");
    c.play_cost = cost;
    let mut r = DebugRunner::builder()
        .add_card(c)
        .add_card(make_test_card("F", "Filler"))
        .hand(0, &["TARGET"])
        .deck(0, &["F"; 8])
        .deck(1, &["F"; 8])
        .memory(memory)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    if let Some((p, m)) = lock {
        r.game.modifiers.add_player_modifier(
            p,
            PlayerModifierEntry::simple(m, 0, Expiry::Permanent, None, p),
        );
    }
    r
}
#[test]
fn reduced_effect_play_pays_full_cost_under_either_players_prohibition() {
    for p in [0, 1] {
        let mut r = runner(5, 10, Some((p, ModifierType::CannotReducePlayCost)));
        let _ = r
            .game
            .play_from_hand_with_cost(0, 0, CostDelta::Reduce(2), PlaySource::ByEffect);
        assert_eq!(r.memory(), 5);
        assert_eq!(r.game.players[0].battle_area.len(), 1);
    }
}
#[test]
fn general_cost_prohibition_also_blocks_effect_play_reduction() {
    let mut r = runner(5, 10, Some((0, ModifierType::CannotReduceCost)));
    let _ = r
        .game
        .play_from_hand_with_cost(0, 0, CostDelta::Reduce(2), PlaySource::ByEffect);
    assert_eq!(r.memory(), 5);
}
#[test]
fn unblocked_reduction_free_fixed_and_increase_keep_distinct_semantics() {
    for (delta, lock, pay) in [
        (CostDelta::Reduce(2), false, 3),
        (CostDelta::Free, true, 0),
        (CostDelta::Fixed(2), true, 2),
        (CostDelta::Reduce(-2), true, 7),
    ] {
        let mut r = runner(
            5,
            10,
            lock.then_some((0, ModifierType::CannotReducePlayCost)),
        );
        let _ = r
            .game
            .play_from_hand_with_cost(0, 0, delta, PlaySource::ByEffect);
        assert_eq!(r.memory(), 10 - pay);
        assert_eq!(r.game.players[0].battle_area.len(), 1);
    }
}
#[test]
fn full_cost_unaffordable_is_not_exposed_by_dsl_selection() {
    const PICKER: &str = r#"
card: PICKER
name: Picker
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 1000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          optional: true
          bind_as: chosen
          filter: { name_is: Target, affordable_with_cost: { reduce: 2 } }
          prompt: Choose a payable card
      - play_or_use_from_hand: { of: you, hand_index: chosen, cost_delta: { reduce: 2 } }
"#;
    for blocked in [false, true] {
        let mut c = make_test_card("TARGET", "Target");
        c.play_cost = 11;
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(PICKER)
            .unwrap()
            .add_card(c)
            .add_card(make_test_card("F", "Filler"))
            .hand(0, &["PICKER", "TARGET"])
            .deck(0, &["F"; 8])
            .deck(1, &["F"; 8])
            .memory(0)
            .start();
        r.skip_mulligan();
        r.set_first_player(0);
        if blocked {
            r.game.modifiers.add_player_modifier(
                1,
                PlayerModifierEntry::simple(
                    ModifierType::CannotReducePlayCost,
                    0,
                    Expiry::Permanent,
                    None,
                    1,
                ),
            );
        }
        r.play(0, 0).unwrap();
        assert_eq!(r.pending_selection().is_some(), !blocked);
        assert_eq!(r.hand_size(0), 1);
    }
}
#[test]
fn unpayable_full_cost_cannot_commit_or_consume_card() {
    let mut r = runner(11, 0, Some((1, ModifierType::CannotReducePlayCost)));
    let _ = r
        .game
        .play_from_hand_with_cost(0, 0, CostDelta::Reduce(2), PlaySource::ByEffect);
    assert_eq!(r.memory(), 0);
    assert_eq!(r.hand_size(0), 1);
    assert!(r.game.players[0].battle_area.is_empty());
}
#[test]
fn other_players_general_cost_prohibition_does_not_block_payer() {
    let mut r = runner(5, 10, Some((1, ModifierType::CannotReduceCost)));
    let _ = r
        .game
        .play_from_hand_with_cost(0, 0, CostDelta::Reduce(2), PlaySource::ByEffect);
    assert_eq!(r.memory(), 7);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}
