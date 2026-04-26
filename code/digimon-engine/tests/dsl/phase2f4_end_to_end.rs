//! Phase 2f4 Task 4 — end-to-end YAML test for `schedule_delayed`.
//!
//! Loads a "DelayedDraw" Option card whose `on_play` `process:` schedules a
//! `draw:` body for `end_of_your_turn`. This exercises the full pipeline:
//!
//!   YAML literal → `serde_yml::from_str` → `digimon_dsl::compile::compile`
//!   → `DslCardEffect::new(Arc::new(compiled))` → invoke OnPlay closure
//!   → assert body queued (not fired) → `runner.end_turn()` → assert body
//!   fired (hand grew by 1, queue drained).
//!
//! Pattern mirrors `phase2f3_end_to_end.rs` and `phase2f1_end_to_end.rs`.
//!
//! Timing notes — Phase 2f3 surfaced a snake_case-vs-PascalCase divergence in
//! the EXPIRY map (validator accepts snake_case, engine `lookup_expiry` keys
//! PascalCase). The TIMING map (`compiled_timing_to_engine`) does NOT have
//! that divergence: the YAML `Timing` enum derives `serde(rename_all = "snake_case")`
//! so `end_of_your_turn` parses to `Timing::EndOfYourTurn`, which `compile()`
//! lowers to `CompiledTiming::EndOfYourTurn`, which `compiled_timing_to_engine`
//! maps to `EffectTiming::EndOfYourTurn`. All three steps line up.

use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;

#[test]
fn delayed_draw_card_queues_draw_at_end_of_your_turn() {
    let yaml = r#"
card: TST-DEL
name: "DelayedDraw"
kind: option
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - schedule_delayed:
          when: end_of_your_turn
          body:
            - draw: { of: you, count: 1 }
"#;
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles cleanly");

    // Seed: TST-DEL in P0's hand (the source of OnPlay), plus two filler cards
    // pushed onto P0's deck so the scheduled `draw` has something to pull.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-DEL", "DelayedDraw"))
        .add_card(make_test_card("FILLER1", "Filler 1"))
        .add_card(make_test_card("FILLER2", "Filler 2"))
        .hand(0, &["TST-DEL"])
        .start();

    // Push fillers directly onto P0's deck (top of deck = end of Vec).
    for cid in &["FILLER1", "FILLER2"] {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *cid)
            .unwrap();
        let card_index = runner.game.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_index);
        runner.game.players[0].deck.push(card);
    }

    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let src_card: CardHandle = runner.game.players[0].hand[0].handle();

    let effects = dsl_effect.effects(src_card);
    assert_eq!(effects.len(), 1, "exactly one OnPlay effect");
    let process = effects[0].process.as_ref().expect("OnPlay has a process");

    // Snapshot before invoking the process.
    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();
    assert_eq!(hand_before, 1, "precondition: only TST-DEL in P0 hand");
    assert_eq!(deck_before, 2, "precondition: 2 fillers in P0 deck");
    assert!(
        runner.game.scheduled_effects.is_empty(),
        "precondition: no scheduled effects queued"
    );

    // Invoke the OnPlay process — `schedule_delayed` queues the draw body.
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        process(&mut ctx);
    }

    // Body is queued, NOT fired — hand + deck unchanged, queue has one entry.
    assert_eq!(
        runner.game.scheduled_effects.len(),
        1,
        "schedule_delayed should have queued exactly one entry"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "draw body should NOT have fired yet — it's queued for end of turn"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "deck untouched — draw body has not run"
    );

    // Drive end-of-turn — Task 2's wiring fires the queue.
    runner.end_turn();

    // After end_turn: queue drained, draw fired (hand grew by 1, deck shrank by 1).
    assert!(
        runner.game.scheduled_effects.is_empty(),
        "queue should be drained after end_of_your_turn"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "scheduled draw fired — P0 hand grew by 1"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before - 1,
        "scheduled draw fired — P0 deck shrank by 1"
    );
}
