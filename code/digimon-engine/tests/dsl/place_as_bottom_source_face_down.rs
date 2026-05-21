//! Task A1.3 — DSL `place_as_bottom_source` step gains a `face_down` flag.
//!
//! End-to-end coverage: YAML `[On Play]` effect that picks a card from trash
//! (`select_trash`) and tucks it under the source permanent
//! (`place_as_bottom_source`). Two cases:
//!   1. `face_down: true`  → the placed bottom source must be face-down.
//!   2. `face_down` omitted → the placed bottom source defaults to face-up.
//!
//! Pattern mirrors `phase2f1_end_to_end.rs` / `phase2b_end_to_end.rs`: parse
//! YAML → `compile` → `DslCardEffect` → invoke the OnPlay process → resolve
//! the parked `select_trash` selection. The effect card is placed on the
//! field as a permanent so the `target: source` binding resolves.

use std::sync::Arc;

use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;

/// Run the `select_trash → place_as_bottom_source` OnPlay effect described by
/// `yaml` and return whether the tucked bottom source ended up face-down.
fn run_and_get_bottom_face_down(yaml: &str) -> bool {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles cleanly");

    // Seed: TST-PABS-FD as a permanent on P0's battle area (the effect
    // source — `target: source` must resolve to a permanent), and
    // TST-TUCK in P0's trash (the card to tuck under it).
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-PABS-FD", "PlaceBottomFaceDown"))
        .add_card(make_test_card("TST-TUCK", "TuckTarget"))
        .memory(5)
        .start();

    let source_perm = runner.place_on_field(0, "TST-PABS-FD", None);
    let src_card: CardHandle = runner.game.players[0].battle_area[source_perm.index as usize]
        .top_card()
        .handle();

    // Push TST-TUCK into P0's trash.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TST-TUCK")
        .unwrap();
    let card_index = runner.game.next_card_index();
    let tuck_card = CardSource::new(data_idx, 0, card_index);
    let tuck_handle = tuck_card.handle();
    runner.game.players[0].trash.push(tuck_card);

    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl_effect.effects(src_card);
    assert_eq!(effects.len(), 1, "exactly one OnPlay effect");
    let process = effects[0].process.as_ref().expect("OnPlay has a process");

    // Invoke the process — installs the `select_trash` selection and parks.
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, Some(source_perm), 0);
        process(&mut ctx);
    }

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

    // Resolve: tail (place_as_bottom_source) tucks the trash card under the
    // source permanent.
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve_selection succeeds");

    // The source permanent's stack grew by 1; the tucked card sits at the
    // bottom (card_sources[0]).
    let perm = &runner.game.players[0].battle_area[source_perm.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        2,
        "source permanent's stack should have grown by 1"
    );
    let bottom = &perm.card_sources[0];
    assert_eq!(
        bottom.handle(),
        tuck_handle,
        "the tucked card must be at the BOTTOM of the stack (card_sources[0])"
    );
    bottom.face_down
}

#[test]
fn place_as_bottom_source_face_down_true_marks_bottom_source_face_down() {
    let yaml = r#"
card: TST-PABS-FD
name: PlaceBottomFaceDown
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
          bind_as: tuck
          filter: {}
          prompt: "Pick a card to tuck face-down"
      - place_as_bottom_source:
          source: tuck
          target: source
          face_down: true
"#;
    assert!(
        run_and_get_bottom_face_down(yaml),
        "face_down: true must place the bottom source face-down"
    );
}

#[test]
fn place_as_bottom_source_face_down_omitted_defaults_face_up() {
    let yaml = r#"
card: TST-PABS-FD
name: PlaceBottomFaceDown
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
          bind_as: tuck
          filter: {}
          prompt: "Pick a card to tuck"
      - place_as_bottom_source:
          source: tuck
          target: source
"#;
    assert!(
        !run_and_get_bottom_face_down(yaml),
        "omitting face_down must default the bottom source to face-up"
    );
}
