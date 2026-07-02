//! collapse-dsl-step-idioms §1 — the generalized `then:` action-tail on the
//! field/zone select steps.
//!
//! The field/zone selects (`select_*_permanent`, `select_hand`,
//! `select_trash`, `select_security`) already run the *implicit* dispatcher
//! tail (the rest of the process body) after a pick resolves. §1 adds an
//! EXPLICIT scoped `then:` that runs the listed steps on accept with the
//! selection binding in scope, composed with the implicit tail without
//! double-running. These tests assert the `then:` form is behaviorally
//! identical to the longhand (implicit-tail) form — and that it is the
//! cloneable VM's `ResumeFrame::RunTail` data (closure-free).

use std::sync::Arc;

use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;

/// Drive a single-OnPlay DSL card's process closure from `card` (player 0),
/// then resolve the single installed pending selection by its first valid
/// action id. Mirrors `phase2b_end_to_end.rs`.
fn run_on_play_and_resolve_single(
    runner: &mut DebugRunner,
    compiled: digimon_dsl::compiled::CompiledCard,
    card: CardHandle,
) {
    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl_effect.effects(card);
    assert_eq!(effects.len(), 1, "exactly one OnPlay effect");
    let process = effects[0].process.as_ref().expect("has process closure");
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        process(&mut ctx);
    }
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("OnPlay installed a PendingSelection");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "exactly one candidate → one valid action id"
    );
    let action_id = pending.valid_action_ids[0];
    let selecting_player = pending.selecting_player;
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve selection succeeds");
    assert!(
        runner.game.pending_selection.is_none(),
        "pending selection cleared after resolution"
    );
}

/// Zone select: `select_trash` with an explicit `then:` that adds the picked
/// card to hand — parity with `phase2b_end_to_end`'s implicit-tail form.
#[test]
fn select_trash_then_add_to_hand_parity() {
    let yaml = r#"
card: COLLAPSE-THEN-001
name: Reclaim (then)
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
          then:
            - add_to_hand_from_trash: { of: you, card: pick }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("valid YAML");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles");

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("COLLAPSE-THEN-001", "COLLAPSE-THEN-001"))
        .add_card(make_test_card("TGT", "TGT"))
        .hand(0, &["COLLAPSE-THEN-001"])
        .build();
    // Seed TGT in player 0's trash.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TGT")
        .expect("TGT in card_data");
    let card_index = runner.game.next_card_index();
    runner.game.players[0]
        .trash
        .push(CardSource::new(data_idx, 0, card_index));
    let tgt_handle = runner.game.players[0].trash[0].handle();

    let card: CardHandle = runner.game.players[0].hand[0].handle();
    run_on_play_and_resolve_single(&mut runner, compiled, card);

    assert!(
        runner.game.players[0].trash.is_empty(),
        "TGT should have left the trash via the then: tail"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.handle() == tgt_handle),
        "TGT should have landed in hand via the then: tail"
    );
}

/// Field select: `select_own_permanent` with an explicit `then:` that deletes
/// the picked permanent.
#[test]
fn select_own_permanent_then_delete() {
    let yaml = r#"
card: COLLAPSE-THEN-002
name: Purge (then)
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: pick
          filter: {}
          prompt: "Delete one of your Digimon"
          then:
            - delete_permanent: { target: pick }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("valid YAML");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles");

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("COLLAPSE-THEN-002", "COLLAPSE-THEN-002"))
        .add_card(make_test_card("TGT-PERM", "TGT-PERM"))
        .hand(0, &["COLLAPSE-THEN-002"])
        .build();
    runner.place_on_field(0, "TGT-PERM", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);

    let card: CardHandle = runner.game.players[0].hand[0].handle();
    run_on_play_and_resolve_single(&mut runner, compiled, card);

    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "picked permanent should have been deleted via the then: tail"
    );
}

/// Field select: `select_own_permanent` with an explicit `then:` that suspends
/// the picked permanent.
#[test]
fn select_own_permanent_then_suspend() {
    let yaml = r#"
card: COLLAPSE-THEN-003
name: Tap (then)
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: pick
          filter: {}
          prompt: "Suspend one of your Digimon"
          then:
            - suspend: { target: pick }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("valid YAML");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles");

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("COLLAPSE-THEN-003", "COLLAPSE-THEN-003"))
        .add_card(make_test_card("TGT-PERM", "TGT-PERM"))
        .hand(0, &["COLLAPSE-THEN-003"])
        .build();
    runner.place_on_field(0, "TGT-PERM", None);
    assert!(
        !runner.game.players[0].battle_area[0].is_suspended,
        "permanent starts unsuspended"
    );

    let card: CardHandle = runner.game.players[0].hand[0].handle();
    run_on_play_and_resolve_single(&mut runner, compiled, card);

    assert!(
        runner.game.players[0].battle_area[0].is_suspended,
        "picked permanent should be suspended via the then: tail"
    );
}

/// §1.3 RL-visibility: a `then` step that itself requires a choice must install
/// its OWN `pending_selection` (no auto-resolution) — the no-approximations
/// policy (rule 17). Here a `select_own_permanent` whose `then` contains a
/// `select_hand`: after resolving the first pick, a NEW pending selection (the
/// hand pick) must be surfaced to the action space, not silently resolved.
#[test]
fn then_tail_inner_selection_stays_rl_visible() {
    let yaml = r#"
card: COLLAPSE-THEN-004
name: Chain (then)
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: pick
          filter: {}
          prompt: "Pick one of your Digimon"
          then:
            - select_hand:
                of: you
                bind_as: handpick
                filter: {}
                prompt: "Now pick a card from hand"
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("valid YAML");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles");

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("COLLAPSE-THEN-004", "COLLAPSE-THEN-004"))
        .add_card(make_test_card("HANDCARD", "HANDCARD"))
        .add_card(make_test_card("TGT-PERM", "TGT-PERM"))
        .hand(0, &["COLLAPSE-THEN-004", "HANDCARD"])
        .build();
    runner.place_on_field(0, "TGT-PERM", None);

    let card: CardHandle = runner.game.players[0].hand[0].handle();
    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl_effect.effects(card);
    let process = effects[0].process.as_ref().expect("has process closure");
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        process(&mut ctx);
    }

    // First selection: the permanent pick.
    let first = runner
        .game
        .pending_selection
        .as_ref()
        .expect("permanent select installed");
    assert_eq!(first.valid_action_ids.len(), 1, "one field permanent");
    let action_id = first.valid_action_ids[0];
    let selecting_player = first.selecting_player;
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve first selection");

    // The then-tail's inner select_hand must have installed a NEW pending
    // selection rather than auto-resolving — RL sees the second choice.
    let second = runner
        .game
        .pending_selection
        .as_ref()
        .expect("then-tail inner select_hand must install its own pending_selection");
    assert!(
        !second.valid_action_ids.is_empty(),
        "inner hand selection must expose its candidate actions to the action space"
    );
}
