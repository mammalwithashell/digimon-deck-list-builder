//! G-DSL-PLACE-AS-TOP-SOURCE (resolved 2026-07-05) — DSL `place_as_top_source`
//! step: place a card as a permanent's TOP digivolution source, i.e. inserted
//! DIRECTLY BENEATH the active top card (DCGO `Permanent.AddDigivolutionCardsTop`,
//! which `Insert(1, …)`s into DCGO's top-first `cardSources`; this engine's
//! `card_sources` is bottom-first with the top card LAST, so top-source =
//! index `len - 1`). The permanent's top card / identity is unchanged — a
//! placed digivolution card never changes what the permanent IS.
//!
//! End-to-end coverage mirroring `place_as_bottom_source_face_down.rs`:
//! YAML `[On Play]` effect that picks a card from trash (`select_trash`) and
//! places it via `place_as_top_source`. The host stack carries TWO
//! pre-existing sources so the TOP-source position (directly beneath the top
//! card) is distinguishable from both the bottom (index 0) and any middle
//! slot:
//!   1. Placement lands at `len - 2` (directly beneath the top card), NOT at
//!      the bottom; pre-existing sources keep their relative order and the
//!      top card is unchanged (ordering/observers otherwise unaffected).
//!   2. On a sourceless (single-card) stack the placed card becomes the sole
//!      source.
//!   3. `face_down: true` marks the placed source face-down; omitted → face-up
//!      (args-shape parity with `place_as_bottom_source`).
//!
//! Consumers: EX9-074 Kimeramon ("as this Digimon's top digivolution card"),
//! BT13-088 Belphemon: Sleep Mode ("on top of this Digimon's digivolution
//! cards").

use std::sync::Arc;

use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::permanent::PermanentHandle;

const HOST_ID: &str = "TST-PATS";

fn top_source_yaml(face_down_line: &str) -> String {
    format!(
        r#"
card: TST-PATS
name: PlaceTopSourceHost
kind: digimon
level: 5
color: [white]
cost: 8
dp: 8000
effects:
  - when: on_play
    process:
      - select_trash:
          of: you
          bind_as: tuck
          filter: {{}}
          prompt: "Pick a card to place as this Digimon's top digivolution card"
      - place_as_top_source:
          source: tuck
          target: source
{face_down_line}
"#
    )
}

/// Build a runner, stage the host permanent (optionally with pre-existing
/// sources via `stack`), put TST-TUCK in P0's trash, run the OnPlay process,
/// and resolve the parked `select_trash` pick. Returns (runner, host handle,
/// tucked card handle).
fn run_placement(yaml: &str, stack: &[&str]) -> (DebugRunner, PermanentHandle, CardHandle) {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles cleanly");

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card(HOST_ID, "PlaceTopSourceHost"))
        .add_card(make_test_card("TST-SRC-A", "SourceA"))
        .add_card(make_test_card("TST-SRC-B", "SourceB"))
        .add_card(make_test_card("TST-TUCK", "TuckTarget"))
        .memory(5)
        .start();

    let host = if stack.is_empty() {
        runner.place_on_field(0, HOST_ID, None)
    } else {
        let mut full_stack: Vec<&str> = stack.to_vec();
        full_stack.push(HOST_ID);
        runner.place_stack(0, &full_stack)
    };
    let src_card: CardHandle = runner.game.players[0].battle_area[host.index as usize]
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
        let mut ctx = EffectContext::new(&mut runner.game, src_card, Some(host), 0);
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

    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve_selection succeeds");

    (runner, host, tuck_handle)
}

/// Core position proof: with TWO pre-existing sources, the placed card lands
/// DIRECTLY BENEATH the top card (index `len - 2`) — the TOP-source slot —
/// not at the bottom, and everything else is undisturbed.
#[test]
fn place_as_top_source_inserts_directly_beneath_top_card() {
    let yaml = top_source_yaml("");
    let (runner, host, tuck_handle) = run_placement(&yaml, &["TST-SRC-A", "TST-SRC-B"]);

    assert!(
        runner.game.players[0].trash.is_empty(),
        "the placed card must leave the trash"
    );

    let perm = &runner.game.players[0].battle_area[host.index as usize];
    let ids: Vec<&str> = perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert_eq!(
        ids,
        vec!["TST-SRC-A", "TST-SRC-B", "TST-TUCK", HOST_ID],
        "the placed card must sit at the TOP-source position (directly \
         beneath the top card), with the pre-existing sources' relative \
         order and the top card unchanged"
    );
    assert_eq!(
        perm.card_sources[perm.card_sources.len() - 2].handle(),
        tuck_handle,
        "top-source slot (len - 2) holds the tucked card"
    );
    assert_ne!(
        perm.card_sources[0].handle(),
        tuck_handle,
        "the tucked card must NOT land at the BOTTOM of the stack \
         (that is place_as_bottom_source's position)"
    );
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        HOST_ID,
        "placing a digivolution card never changes the permanent's top card"
    );
}

/// On a sourceless stack the placed card becomes the sole source — beneath
/// the top card, which stays on top.
#[test]
fn place_as_top_source_on_sourceless_stack_becomes_sole_source() {
    let yaml = top_source_yaml("");
    let (runner, host, tuck_handle) = run_placement(&yaml, &[]);

    let perm = &runner.game.players[0].battle_area[host.index as usize];
    assert_eq!(perm.card_sources.len(), 2, "stack grew by exactly 1");
    assert_eq!(
        perm.card_sources[0].handle(),
        tuck_handle,
        "on a sourceless stack the placed card is the sole (top and bottom) source"
    );
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        HOST_ID,
        "the host stays the top card"
    );
}

/// `face_down: true` marks the placed top source face-down.
#[test]
fn place_as_top_source_face_down_true_marks_source_face_down() {
    let yaml = top_source_yaml("          face_down: true");
    let (runner, host, tuck_handle) = run_placement(&yaml, &["TST-SRC-A", "TST-SRC-B"]);

    let perm = &runner.game.players[0].battle_area[host.index as usize];
    let placed = perm
        .card_sources
        .iter()
        .find(|c| c.handle() == tuck_handle)
        .expect("placed card is in the stack");
    assert!(placed.face_down, "face_down: true → the placed source is face-down");
}

/// Omitting `face_down` defaults the placed top source to face-up.
#[test]
fn place_as_top_source_face_down_omitted_defaults_face_up() {
    let yaml = top_source_yaml("");
    let (runner, host, tuck_handle) = run_placement(&yaml, &["TST-SRC-A", "TST-SRC-B"]);

    let perm = &runner.game.players[0].battle_area[host.index as usize];
    let placed = perm
        .card_sources
        .iter()
        .find(|c| c.handle() == tuck_handle)
        .expect("placed card is in the stack");
    assert!(
        !placed.face_down,
        "omitting face_down → the placed source is face-up"
    );
}
