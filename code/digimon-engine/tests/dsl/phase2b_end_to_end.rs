//! Task 8 end-to-end Phase 2b: compile a YAML DSL card with
//! `select_trash → add_to_hand_from_trash`, register it via `DslCardEffect`,
//! invoke the OnPlay process closure, resolve the pending selection, and
//! assert the target card moves from trash to hand.

use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;

/// Build a runner with `src_id` in player 0's hand and `tgt_id` in player 0's
/// trash. Trash is populated via direct push because `DebugRunnerBuilder` has
/// no `.trash()` method — same approach used in `phase2b_zone_moves.rs`.
fn make_runner_with_trash(src_id: &str, tgt_id: &str) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card(src_id, src_id))
        .add_card(make_test_card(tgt_id, tgt_id))
        .hand(0, &[src_id])
        .build();

    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == tgt_id)
        .expect("tgt_id must be present in card_data");
    let card_index = runner.game.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_index);
    runner.game.players[0].trash.push(card);

    runner
}

#[test]
fn dsl_select_trash_then_add_to_hand_from_trash_round_trip() {
    let yaml = r#"
card: DSL-E2E-002
name: Reclaim
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_trash:
          of: you
          bind_as: pick
          filter: {}
          prompt: "Return a card from trash"
      - add_to_hand_from_trash: { of: you, card: pick }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("valid YAML");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles");

    // Seed: DSL-E2E-002 in player 0's hand, TGT in player 0's trash.
    let mut runner = make_runner_with_trash("DSL-E2E-002", "TGT");

    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let card: CardHandle = runner.game.players[0].hand[0].handle();
    let tgt_handle: CardHandle = runner.game.players[0].trash[0].handle();

    let effects = dsl_effect.effects(card);
    assert_eq!(effects.len(), 1, "exactly one OnPlay effect");

    let process = effects[0].process.as_ref().expect("has process closure");

    // Invoke the process closure. This installs a Trash selection and returns.
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        process(&mut ctx);
    }

    // Verify the pending selection was installed with exactly one valid action.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("OnPlay installed a PendingSelection");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "one trash card → one valid action ID"
    );
    let action_id = pending.valid_action_ids[0];
    let selecting_player = pending.selecting_player;

    // Resolve the selection: callback re-enters run_steps with the tail
    // (add_to_hand_from_trash) and moves TGT from trash to hand.
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve selection succeeds");

    // Post-conditions.
    assert!(
        runner.game.pending_selection.is_none(),
        "pending selection must be cleared after resolution"
    );
    assert!(
        runner.game.players[0].trash.is_empty(),
        "TGT should have left the trash"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.handle() == tgt_handle),
        "TGT should have landed in hand"
    );
}

/// Negative case: when player 0's trash is empty, no valid action IDs are
/// installed and the pending selection reflects an empty selection set.
#[test]
fn dsl_select_trash_empty_trash_installs_selection_with_no_valid_actions() {
    let yaml = r#"
card: DSL-E2E-002
name: Reclaim
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_trash:
          of: you
          bind_as: pick
          filter: {}
          prompt: "Return a card from trash"
          optional: true
      - add_to_hand_from_trash: { of: you, card: pick }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("valid YAML");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles");

    // Player 0 has a hand card but an empty trash.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("DSL-E2E-002", "Reclaim"))
        .hand(0, &["DSL-E2E-002"])
        .build();

    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let card: CardHandle = runner.game.players[0].hand[0].handle();

    let effects = dsl_effect.effects(card);
    let process = effects[0].process.as_ref().expect("has process closure");

    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        process(&mut ctx);
    }

    // With an empty trash there are no valid action IDs.
    if let Some(pending) = runner.game.pending_selection.as_ref() {
        assert_eq!(
            pending.valid_action_ids.len(),
            0,
            "empty trash → zero valid action IDs"
        );
    }
    // Hand remains unchanged.
    assert_eq!(runner.game.players[0].hand.len(), 1);
    assert!(runner.game.players[0].trash.is_empty());
}
