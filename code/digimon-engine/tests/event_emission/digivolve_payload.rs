//! `GameEvent::Digivolve` payload tests for `was_dna`, `was_blast_dna`,
//! and `memory_paid`. Covers the scenarios in
//! `openspec/changes/add-reward-profiles/specs/engine-event-emission/spec.md`
//! under "GameEvent::Digivolve carries DNA flags and result identity".

use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card_with_level, make_test_dna_card, DebugRunner};
use digimon_engine::enums::{GamePhase, PlaySource};
use digimon_engine::events::GameEvent;

fn drained_digivolves(r: &mut DebugRunner) -> Vec<GameEvent> {
    r.game
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, GameEvent::Digivolve { .. }))
        .collect()
}

#[test]
fn standard_evo_cost_digivolve_emits_was_dna_false_with_memory_paid() {
    // Scenario: standard evo-cost digivolve. was_dna=false, was_blast_dna=false,
    // memory_paid = the printed evo cost.
    let base = make_test_card_with_level("BASE-LV4", "BaseLv4", 4);
    let mut evo = make_test_card_with_level("EVO-LV5", "EvoLv5", 5);
    evo.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 4,
        memory_cost: 3, // pay 3 memory
    }];

    let mut runner = DebugRunner::builder()
        .add_card(base)
        .add_card(evo)
        .hand(0, &["EVO-LV5"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE-LV4", Some(0));
    runner.game.current_phase = GamePhase::Main;
    runner.game.drain_events();

    let ok = runner
        .game
        .digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    assert!(ok, "standard evo-cost digivolve must succeed");

    let digivolves = drained_digivolves(&mut runner);
    assert_eq!(
        digivolves.len(),
        1,
        "exactly one Digivolve event SHALL fire; got {digivolves:?}"
    );

    let GameEvent::Digivolve {
        top_card_id,
        was_dna,
        was_blast_dna,
        memory_paid,
        ..
    } = digivolves[0].clone()
    else {
        unreachable!()
    };

    assert_eq!(top_card_id, "EVO-LV5");
    assert!(!was_dna, "standard evo-cost path SHALL have was_dna=false");
    assert!(
        !was_blast_dna,
        "standard evo-cost path SHALL have was_blast_dna=false"
    );
    assert_eq!(memory_paid, 3, "memory_paid SHALL equal printed evo cost");
}

#[test]
fn dna_digivolve_emits_was_dna_true_with_memory_paid() {
    // Scenario: standard `dna_costs` path. was_dna=true,
    // was_blast_dna=false, memory_paid = the DNA cost.
    let lv5 = make_test_card_with_level("TST-LV5", "FiveDigi", 5);
    let lv6 = make_test_card_with_level("TST-LV6", "SixDigi", 6);
    let dna = make_test_dna_card("TST-DNA", "DnaDigi", 5, 6, 0); // 0-memory DNA

    let mut runner = DebugRunner::builder()
        .add_card(lv5)
        .add_card(lv6)
        .add_card(dna)
        .hand(0, &["TST-DNA"])
        .memory(10)
        .start();

    let lhs = runner.place_on_field(0, "TST-LV5", Some(0));
    let rhs = runner.place_on_field(0, "TST-LV6", Some(0));
    runner.game.current_phase = GamePhase::Main;
    runner.game.drain_events();

    // Drive the DNA digivolve through its 2-stage selection.
    let ok = runner.game.initiate_dna_digivolve(0, 0);
    assert!(ok, "initiate_dna_digivolve must succeed");
    // Resolve first selection: pick the lhs source.
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .expect("first selection")
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| {
            // any valid id works; we just need to commit it
            true
        })
        .expect("at least one valid action");
    runner
        .game
        .resolve_selection(0, action)
        .expect("first stage resolves");
    // Resolve second selection (pick the other source).
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .expect("second selection")
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("second stage resolves");

    let _ = (lhs, rhs);

    let digivolves = drained_digivolves(&mut runner);
    assert!(
        !digivolves.is_empty(),
        "DNA digivolve SHALL emit at least one Digivolve event"
    );

    let GameEvent::Digivolve {
        was_dna,
        was_blast_dna,
        memory_paid,
        ..
    } = digivolves
        .iter()
        .find(|e| matches!(e, GameEvent::Digivolve { top_card_id, .. } if top_card_id == "TST-DNA"))
        .cloned()
        .expect("DNA result emitted as Digivolve")
    else {
        unreachable!()
    };

    assert!(was_dna, "DNA path SHALL have was_dna=true");
    assert!(
        !was_blast_dna,
        "standard DNA (not Blast) SHALL have was_blast_dna=false"
    );
    assert_eq!(memory_paid, 0, "0-memory DNA card pays 0");
}
