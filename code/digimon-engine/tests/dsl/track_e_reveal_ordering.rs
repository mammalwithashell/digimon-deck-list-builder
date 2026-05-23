//! Phase 2 Track E (2026-05-17): DSL flow tests for `choose_from_reveal` +
//! `order_remainder`.
//!
//! Drives the new author-facing verbs through `CompiledStep` directly so the
//! reveal+pick+order flow can be exercised without depending on a specific
//! card's trigger plumbing (those are covered by `cards_behavioral/p/p_167.rs`).
//!
//! ## What this fixture proves
//!
//! - `choose_from_reveal { destination: hand }` installs a single-card
//!   selection over the reveal pool, the pick lands in the chosen player's
//!   hand, and `optional: true` lets the player decline.
//! - `order_remainder { destinations: [deck_top, deck_bottom] }` surfaces an
//!   `effect_choice` for top/bottom AND a follow-up ordered-permutation
//!   selection (no auto-determinism — Working Rule §17).
//! - `order_remainder { destinations: [deck_bottom] }` skips the destination
//!   choice and goes straight to the ordering selection (equivalent to
//!   `place_remainder_on_deck`).
//! - The chained tail (a `gain_memory` sentinel) runs only after both the
//!   destination choice AND the ordering selection resolve.

use digimon_dsl::compiled::{
    CompiledPlayerRef, CompiledPredicate, CompiledRemainderDestination, CompiledRevealDestination,
    CompiledStep,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_deck(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .deck
        .push(CardSource::new(data_idx, player, card_index));
}

fn mineral(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.traits.push("Mineral".to_string());
    c
}

fn rock(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.traits.push("Rock".to_string());
    c
}

fn plain(id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(id, id)
}

fn trait_any_predicate(traits: &[&str]) -> CompiledPredicate {
    let mut alts = Vec::new();
    for t in traits {
        let mut p = CompiledPredicate::default();
        p.trait_has = Some((*t).to_string());
        alts.push(p);
    }
    let mut pred = CompiledPredicate::default();
    pred.any_of = alts;
    pred
}

/// `choose_from_reveal { destination: hand }` over a 3-card reveal pool with
/// a Mineral-or-Rock filter. With one matching card on top of deck, the
/// player picks it and it lands in hand.
#[test]
fn choose_from_reveal_hand_routes_picked_card_to_hand() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(mineral("MIN-1"))
        .add_card(plain("FILL-1"))
        .add_card(plain("FILL-2"))
        .hand(0, &["SRC"])
        .build();

    push_to_deck(&mut runner, 0, "FILL-2");
    push_to_deck(&mut runner, 0, "FILL-1");
    push_to_deck(&mut runner, 0, "MIN-1"); // top
    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::RevealTopDeck {
            of: CompiledPlayerRef::You,
            count: 3,
            zone: None,
            bind_as: Some("revealed".to_string()),
        },
        CompiledStep::ChooseFromReveal {
            of: CompiledPlayerRef::You,
            filter: trait_any_predicate(&["Mineral", "Rock"]),
            destination: CompiledRevealDestination::Hand,
            bind_as: Some("picked".to_string()),
            prompt: "Pick a Mineral or Rock card".to_string(),
            prompt_key: None,
            optional: true,
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut Bindings::new());
    }

    // Exactly one selection (the choose-from-reveal pick) should install.
    assert!(
        runner.game.pending_selection.is_some(),
        "choose_from_reveal must install a pending selection when candidates exist"
    );

    // Player picks the first valid action (the only matching card).
    runner
        .auto_resolve()
        .expect("choose_from_reveal pick resolves");

    // MIN-1 is now in hand alongside SRC.
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "MIN-1"),
        "MIN-1 must be in hand after choose_from_reveal destination: hand"
    );
}

/// `choose_from_reveal { optional: true }` with no matching candidates skips
/// the pick (player auto-declines via PASS) — reveal pool stays intact for
/// the next step.
#[test]
fn choose_from_reveal_optional_no_candidates_declines() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(plain("FILL-1"))
        .add_card(plain("FILL-2"))
        .add_card(plain("FILL-3"))
        .hand(0, &["SRC"])
        .build();

    push_to_deck(&mut runner, 0, "FILL-3");
    push_to_deck(&mut runner, 0, "FILL-2");
    push_to_deck(&mut runner, 0, "FILL-1");
    let src_card = runner.game.players[0].hand[0].handle();
    let hand_before = runner.game.players[0].hand.len();

    let steps = vec![
        CompiledStep::RevealTopDeck {
            of: CompiledPlayerRef::You,
            count: 3,
            zone: None,
            bind_as: Some("revealed".to_string()),
        },
        CompiledStep::ChooseFromReveal {
            of: CompiledPlayerRef::You,
            filter: trait_any_predicate(&["Mineral", "Rock"]),
            destination: CompiledRevealDestination::Hand,
            bind_as: None,
            prompt: "Pick a Mineral or Rock card".to_string(),
            prompt_key: None,
            optional: true,
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut Bindings::new());
    }

    // Selection installs (optional select_reveal); auto_resolve takes PASS.
    runner
        .auto_resolve()
        .expect("optional choose_from_reveal declines cleanly");

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "no matching candidate → hand unchanged after optional decline"
    );
    // Reveal pool still holds the 3 revealed cards — next step would consume them.
    assert_eq!(
        runner.game.revealed_cards.len(),
        3,
        "declined optional pick leaves reveal pool intact"
    );
}

/// `order_remainder { destinations: [deck_bottom] }` with a single
/// destination skips the destination choice and installs only the
/// ordered-permutation selection.
#[test]
fn order_remainder_single_destination_installs_only_ordering() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(plain("A"))
        .add_card(plain("B"))
        .hand(0, &["SRC"])
        .build();

    push_to_deck(&mut runner, 0, "B");
    push_to_deck(&mut runner, 0, "A");
    let src_card = runner.game.players[0].hand[0].handle();
    let deck_before = runner.game.players[0].deck.len();

    let steps = vec![
        CompiledStep::RevealTopDeck {
            of: CompiledPlayerRef::You,
            count: 2,
            zone: None,
            bind_as: Some("revealed".to_string()),
        },
        CompiledStep::OrderRemainder {
            of: CompiledPlayerRef::You,
            destinations: vec![CompiledRemainderDestination::DeckBottom],
            prompt: None,
            prompt_key: None,
        },
        CompiledStep::GainMemory(1),
    ];

    let memory_before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut Bindings::new());
    }

    // A 2-item permutation prompts twice. Just drain.
    runner
        .auto_resolve()
        .expect("order_remainder permutation resolves");

    assert!(
        runner.game.pending_selection.is_none(),
        "all selections resolved"
    );
    // Reveal pool drained; the 2 cards land back at deck bottom.
    assert_eq!(
        runner.game.revealed_cards.len(),
        0,
        "reveal pool emptied after order_remainder"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "remainder placed back on deck — deck size restored"
    );
    // Tail (gain_memory) ran exactly once.
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "tail must run after order_remainder resolves"
    );
}

/// `order_remainder { destinations: [deck_top, deck_bottom] }` with two
/// destinations FIRST installs an `effect_choice` for top vs bottom, THEN
/// the ordered permutation. The chained tail runs only after both resolve.
#[test]
fn order_remainder_two_destinations_chains_choice_then_permutation() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(plain("A"))
        .add_card(plain("B"))
        .hand(0, &["SRC"])
        .build();

    push_to_deck(&mut runner, 0, "B");
    push_to_deck(&mut runner, 0, "A");
    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::RevealTopDeck {
            of: CompiledPlayerRef::You,
            count: 2,
            zone: None,
            bind_as: Some("revealed".to_string()),
        },
        CompiledStep::OrderRemainder {
            of: CompiledPlayerRef::You,
            destinations: vec![
                CompiledRemainderDestination::DeckTop,
                CompiledRemainderDestination::DeckBottom,
            ],
            prompt: None,
            prompt_key: None,
        },
        CompiledStep::GainMemory(1),
    ];

    let memory_before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut Bindings::new());
    }

    // First prompt: effect_choice for top vs bottom (2 entries).
    use digimon_engine::selection::SelectionKind;
    let first_kind = runner.pending_kind();
    assert_eq!(
        first_kind,
        Some(SelectionKind::EffectChoice),
        "two destinations must install an effect_choice first; got {:?}",
        first_kind
    );
    assert_eq!(
        runner.pending_action_count(),
        2,
        "effect_choice must offer exactly 2 destination options"
    );

    // Player picks "Top of deck" (index 0).
    runner
        .execute_branch(0)
        .expect("destination choice resolves");

    // Memory must NOT have advanced yet — tail runs only after permutation.
    assert_eq!(
        runner.game.memory, memory_before,
        "tail must not run before the ordered permutation completes"
    );

    // Next prompt: ordered permutation for the 2 revealed cards.
    assert!(
        runner.game.pending_selection.is_some(),
        "permutation selection must install after destination choice"
    );
    runner
        .auto_resolve()
        .expect("permutation selection resolves");

    assert_eq!(
        runner.game.revealed_cards.len(),
        0,
        "reveal pool drained after permutation"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "tail must run after permutation resolves"
    );
}

/// `order_remainder` with an empty reveal pool is a silent no-op; the tail
/// runs synchronously without any selection.
#[test]
fn order_remainder_empty_reveal_pool_runs_tail_synchronously() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .build();
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::OrderRemainder {
            of: CompiledPlayerRef::You,
            destinations: vec![CompiledRemainderDestination::DeckTop],
            prompt: None,
            prompt_key: None,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut Bindings::new());
    }

    assert!(
        runner.game.pending_selection.is_none(),
        "empty reveal pool installs no selection"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "tail must run synchronously when reveal pool is empty"
    );
}

/// YAML round-trip: both new verbs parse from natural author syntax and
/// compile to the expected `CompiledStep` shapes.
#[test]
fn yaml_round_trip_compiles_new_verbs_to_expected_shapes() {
    use digimon_dsl::compile::compile;
    use digimon_dsl::spec::CardSpec;
    let yaml = r#"
card: TEST-TRACK-E
name: Track E Reveal Ordering
kind: option
color: [black]
cost: 2
effects:
  - when: main
    process:
      - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
      - choose_from_reveal:
          of: you
          filter:
            any_of:
              - trait_has: Mineral
              - trait_has: Rock
          destination: hand
          bind_as: picked
          prompt: "Pick a Mineral or Rock card"
          optional: true
      - order_remainder:
          of: you
          destinations: [deck_top, deck_bottom]
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("track-e yaml parses");
    let compiled = compile(&spec).expect("track-e yaml compiles");
    let digimon_dsl::compiled::CompiledClause::Triggered(effect) = &compiled.effects[0] else {
        panic!("expected triggered effect");
    };
    // Index 1 = choose_from_reveal, index 2 = order_remainder.
    match &effect.process[1] {
        CompiledStep::ChooseFromReveal {
            destination,
            optional,
            ..
        } => {
            assert_eq!(*optional, true);
            assert!(matches!(destination, CompiledRevealDestination::Hand));
        }
        other => panic!("expected ChooseFromReveal at index 1; got {:?}", other),
    }
    match &effect.process[2] {
        CompiledStep::OrderRemainder { destinations, .. } => {
            assert_eq!(destinations.len(), 2);
            assert!(matches!(
                destinations[0],
                CompiledRemainderDestination::DeckTop
            ));
            assert!(matches!(
                destinations[1],
                CompiledRemainderDestination::DeckBottom
            ));
        }
        other => panic!("expected OrderRemainder at index 2; got {:?}", other),
    }
}

/// YAML round-trip: `choose_from_reveal` accepts the `bottom_source_of`
/// destination variant for source-placement search effects.
#[test]
fn yaml_round_trip_choose_from_reveal_bottom_source_of_variant() {
    use digimon_dsl::compile::compile;
    use digimon_dsl::spec::CardSpec;
    let yaml = r#"
card: TEST-TRACK-E-BOTTOM-SOURCE
name: Bottom Source Pick
kind: digimon
level: 4
color: [black]
cost: 4
dp: 4000
traits: [Mineral]
attribute: Virus
effects:
  - when: main_from_hand
    process:
      - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
      - choose_from_reveal:
          of: you
          filter: { trait_has: Mineral }
          destination:
            bottom_source_of: { target: this }
          prompt: "Place as bottom source"
          optional: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let digimon_dsl::compiled::CompiledClause::Triggered(effect) = &compiled.effects[0] else {
        panic!("expected triggered effect");
    };
    match &effect.process[1] {
        CompiledStep::ChooseFromReveal { destination, .. } => {
            assert!(
                matches!(destination, CompiledRevealDestination::BottomSourceOf(_)),
                "destination must be BottomSourceOf; got {:?}",
                destination
            );
        }
        other => panic!("expected ChooseFromReveal; got {:?}", other),
    }
}

/// Full P-167-style flow: reveal top 3, choose 1 to hand, order remainder
/// onto top or bottom. The reveal+choose+order chain operates as a single
/// authored sequence with two distinct player-visible selections.
#[test]
fn p_167_style_reveal_choose_order_full_flow() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(rock("ROCK-A"))
        .add_card(plain("FILL-1"))
        .add_card(plain("FILL-2"))
        .hand(0, &["SRC"])
        .build();

    push_to_deck(&mut runner, 0, "FILL-2");
    push_to_deck(&mut runner, 0, "FILL-1");
    push_to_deck(&mut runner, 0, "ROCK-A"); // top
    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::RevealTopDeck {
            of: CompiledPlayerRef::You,
            count: 3,
            zone: None,
            bind_as: Some("revealed".to_string()),
        },
        CompiledStep::ChooseFromReveal {
            of: CompiledPlayerRef::You,
            filter: trait_any_predicate(&["Mineral", "Rock"]),
            destination: CompiledRevealDestination::Hand,
            bind_as: Some("picked".to_string()),
            prompt: "Pick a Mineral or Rock card".to_string(),
            prompt_key: None,
            optional: true,
        },
        CompiledStep::OrderRemainder {
            of: CompiledPlayerRef::You,
            destinations: vec![
                CompiledRemainderDestination::DeckTop,
                CompiledRemainderDestination::DeckBottom,
            ],
            prompt: None,
            prompt_key: None,
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut Bindings::new());
    }

    // Phase 1: choose_from_reveal selection over Mineral/Rock matches.
    assert!(
        runner.game.pending_selection.is_some(),
        "choose_from_reveal selection must install"
    );
    runner
        .execute_branch(0) // SelectReveal uses branch-style actions via valid_action_ids
        .or_else(|_| {
            let act = runner
                .game
                .pending_selection
                .as_ref()
                .unwrap()
                .valid_action_ids[0];
            let p = runner
                .game
                .pending_selection
                .as_ref()
                .unwrap()
                .selecting_player;
            runner.execute_action(p, act)
        })
        .expect("pick the only matching Mineral/Rock revealed card");

    // ROCK-A landed in hand.
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "ROCK-A"),
        "ROCK-A must be in hand after choose_from_reveal: hand"
    );

    // Phase 2a: order_remainder installs effect_choice for top vs bottom.
    use digimon_engine::selection::SelectionKind;
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "order_remainder must install destination effect_choice next"
    );
    runner
        .execute_branch(1) // pick "Bottom of deck" (index 1)
        .expect("destination effect_choice resolves");

    // Phase 2b: ordered permutation for the 2 remaining revealed cards.
    assert!(
        runner.game.pending_selection.is_some(),
        "permutation selection must install after destination choice"
    );
    runner.auto_resolve().expect("permutation resolves");

    assert_eq!(
        runner.game.revealed_cards.len(),
        0,
        "reveal pool empty after full flow"
    );
}
