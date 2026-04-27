//! Phase 2f1 Task 6 — end-to-end YAML test for the play-step pipeline.
//!
//! Loads a "Revival" card from a YAML literal that uses `select_trash`
//! (Phase 2b) chained with `play_from_trash` (Task 4): loader → compile →
//! `DslCardEffect` → register → trigger OnPlay → resolve the parked
//! `select_trash` → tail's `play_from_trash` lifts the picked card from
//! trash into battle area.
//!
//! Pattern mirrors `phase2b_end_to_end.rs` (the established Phase 2b
//! end-to-end shape).

use std::sync::Arc;

use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;

#[test]
fn revival_card_pulls_from_trash_into_battle_area() {
    let yaml = r#"
card: TST-REV
name: Revival
kind: digimon
level: 3
color: [red]
cost: 4
dp: 3000
effects:
  - when: on_play
    process:
      - select_trash:
          of: you
          bind_as: revived
          filter:
            level_lte: 3
          prompt: "Pick a Digimon to revive"
      - play_from_trash:
          of: you
          hand_index: revived
          cost_delta: free
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles cleanly");

    // Seed: TST-REV in P0's hand (the source of OnPlay), TST-TARGET in P0's
    // trash (the revival target).
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-REV", "Revival"))
        .add_card(make_test_card("TST-TARGET", "Target"))
        .hand(0, &["TST-REV"])
        .build();

    // Push TST-TARGET directly into P0's trash.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TST-TARGET")
        .unwrap();
    let card_index = runner.game.next_card_index();
    let target_card = CardSource::new(data_idx, 0, card_index);
    let target_handle = target_card.handle();
    runner.game.players[0].trash.push(target_card);

    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let src_card: CardHandle = runner.game.players[0].hand[0].handle();

    let effects = dsl_effect.effects(src_card);
    assert_eq!(effects.len(), 1, "exactly one OnPlay effect");

    let process = effects[0].process.as_ref().expect("OnPlay has a process");

    // Snapshot before.
    assert_eq!(
        runner.game.players[0].trash.len(),
        1,
        "precondition: 1 card in trash (the revival target)"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "precondition: P0 battle area empty"
    );

    // Invoke the process — installs a Trash selection and parks.
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        process(&mut ctx);
    }

    // The select_trash step parked. Tail (play_from_trash) has not yet run.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_trash should have installed a PendingSelection");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "one trash card → one valid action ID"
    );
    let action_id = pending.valid_action_ids[0];
    let selecting_player = pending.selecting_player;

    // Resolve the selection: callback re-enters run_steps with the tail
    // (play_from_trash) and lifts the target into battle area.
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve_selection succeeds");

    // Post-conditions: pending selection cleared, target moved from trash
    // to battle area.
    assert!(
        runner.game.pending_selection.is_none(),
        "pending selection must be cleared after resolution"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        0,
        "trash should be empty — target was lifted into battle area"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "battle area should contain the revived target"
    );
    let perm = &runner.game.players[0].battle_area[0];
    assert_eq!(
        perm.top_card().handle(),
        target_handle,
        "the revived permanent's top card must be the trashed target"
    );
}
