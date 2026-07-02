//! Effect-initiated App Fuse primitive — behavioral coverage.
//!
//! Drives the `app_fuse` DSL step through minimal `from_dsl_yaml` fixture cards
//! whose `[On Play]` clause carries a single `app_fuse` step. The result card is
//! a synthetic App-Fusion Digimon (`TEST-APPFUSE`, materials Kabemon & Gomimon),
//! mirroring `tests/cards_behavioral/bt25/app_fusion.rs`. Two engine-driven
//! selections: pick the field permanent, then the result card; then the commit
//! stacks the result on the host and folds the linked materials under it.

use digimon_engine::action::space::{encode_attack, HAND_EFFECT_START, PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

/// Result card: App-Fusion Digimon whose materials are Kabemon & Gomimon.
const APP_FUSE_RESULT: &str = r#"
card: TEST-APPFUSE
name: Test App Fusion Digimon
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 9000
traits: [Appmon]
alt_paths:
  - kind: app_fusion
    materials:
      - filter:
          name_in: [Kabemon, Gomimon]
    cost: 0
"#;

/// A second App-Fusion result carrying a distinguishing trait, for the filter
/// test. Same materials (Kabemon & Gomimon) so it is route-eligible.
const APP_FUSE_RESULT_TAGGED: &str = r#"
card: TEST-APPFUSE-TAGGED
name: Test App Fusion Tagged
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 9000
traits: [Appmon, System]
alt_paths:
  - kind: app_fusion
    materials:
      - filter:
          name_in: [Kabemon, Gomimon]
    cost: 0
"#;

/// Fixture: on play, app fuse from hand (no result filter). Clause is
/// non-optional; the `app_fuse` step itself is optional ("may").
const SRC_HAND: &str = r#"
card: DSL-APPFUSE-HAND
name: App Fuse Source Hand
kind: tamer
color: [green]
cost: 0
effects:
  - when: on_play
    process:
      - app_fuse: { from: hand, optional: true }
"#;

/// Fixture: on play, app fuse from trash (no filter).
const SRC_TRASH: &str = r#"
card: DSL-APPFUSE-TRASH
name: App Fuse Source Trash
kind: tamer
color: [green]
cost: 0
effects:
  - when: on_play
    process:
      - app_fuse: { from: trash, optional: true }
"#;

/// Fixture: on play, app fuse from trash with a [System] result filter.
const SRC_FILTER: &str = r#"
card: DSL-APPFUSE-FILTER
name: App Fuse Source Filter
kind: tamer
color: [green]
cost: 0
effects:
  - when: on_play
    process:
      - app_fuse:
          from: trash
          optional: true
          result_filter: { trait_has: System }
"#;

fn named_digimon(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.colors = vec![CardColor::Yellow];
    card.level = Some(4);
    card
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(SRC_HAND)
        .expect("SRC_HAND compiles")
        .from_dsl_yaml(SRC_TRASH)
        .expect("SRC_TRASH compiles")
        .from_dsl_yaml(SRC_FILTER)
        .expect("SRC_FILTER compiles")
        .from_dsl_yaml(APP_FUSE_RESULT)
        .expect("APP_FUSE_RESULT compiles")
        .from_dsl_yaml(APP_FUSE_RESULT_TAGGED)
        .expect("APP_FUSE_RESULT_TAGGED compiles")
        .add_card(named_digimon("TEST-KABEMON", "Kabemon"))
        .add_card(named_digimon("TEST-GOMIMON", "Gomimon"))
        .memory(5)
}

fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let iid = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(idx, player as u8, iid));
}

/// Host with Kabemon top + Gomimon linked = 2 distinct named materials.
fn fused_host(runner: &mut DebugRunner) -> PermanentHandle {
    let host = runner.place_on_field(0, "TEST-KABEMON", Some(0));
    runner.push_linked_owned(host, "TEST-GOMIMON", 0);
    host
}

fn hand_index(runner: &DebugRunner, card_id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .expect("card in hand")
}

fn top_id(runner: &DebugRunner, host: PermanentHandle) -> String {
    runner.game.players[0].battle_area[host.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

// ── 1. Hand happy path ────────────────────────────────────────────────────────

#[test]
fn app_fuse_hand_happy_path_stacks_result_and_consumes_link() {
    let mut r = builder()
        .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
        .start();
    let host = fused_host(&mut r);

    let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
    r.play(0, src_idx);

    // Selection #1: pick the host permanent.
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Target),
        "perm selection installs"
    );
    let host_action = encode_attack(0, host.index as u16);
    assert!(
        r.pending_selection_view()
            .unwrap()
            .valid_action_ids
            .contains(&host_action),
        "host is an eligible app-fuse permanent"
    );
    r.execute_action(0, host_action).expect("pick host");

    // Selection #2: pick the result card in hand.
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Target),
        "result-card selection installs"
    );
    let card_action = HAND_EFFECT_START + hand_index(&r, "TEST-APPFUSE") as u16;
    r.execute_action(0, card_action).expect("pick result card");

    assert_eq!(
        top_id(&r, host),
        "TEST-APPFUSE",
        "result card stacked on top"
    );
    let perm = &r.game.players[0].battle_area[host.index as usize];
    assert!(perm.linked_cards.is_empty(), "linked materials consumed");
    assert!(
        perm.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "TEST-GOMIMON"),
        "consumed link folded under the new top as a digivolution source"
    );
}

#[test]
fn app_fuse_prompts_clone_faithfully() {
    let mut r = builder()
        .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
        .start();
    let host = fused_host(&mut r);

    let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
    r.play(0, src_idx);

    let host_action = encode_attack(0, host.index as u16);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Target));
    assert!(
        r.game.pending_selection_resume.is_some(),
        "App Fuse host prompt must park a data frame"
    );

    let mut clone_at_host = r.game.clone();
    clone_at_host
        .resolve_selection(0, host_action)
        .expect("clone resolves host prompt");
    assert!(
        clone_at_host.pending_selection.is_some(),
        "clone advances to result-card prompt"
    );
    assert!(
        clone_at_host.pending_selection_resume.is_some(),
        "App Fuse result prompt must park a data frame"
    );
    assert!(
        r.game.pending_selection.is_some(),
        "resolving the clone must leave the original at the host prompt"
    );

    r.execute_action(0, host_action)
        .expect("original picks host");
    let card_action = HAND_EFFECT_START + hand_index(&r, "TEST-APPFUSE") as u16;
    assert!(r.game.pending_selection_resume.is_some());

    let mut clone_at_result = r.game.clone();
    clone_at_result
        .resolve_selection(0, card_action)
        .expect("clone resolves result-card prompt");
    assert!(clone_at_result.pending_selection.is_none());
    assert_eq!(
        clone_at_result.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&clone_at_result.card_data),
        "TEST-APPFUSE"
    );
    assert!(
        r.game.pending_selection.is_some(),
        "resolving the result clone must leave the original at the result prompt"
    );

    r.execute_action(0, card_action)
        .expect("original picks result card");
    assert_eq!(
        top_id(&r, host),
        clone_at_result.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&clone_at_result.card_data)
    );
}

// ── 2. Trash happy path ─────────────────────────────────────────────────────

#[test]
fn app_fuse_trash_happy_path() {
    let mut r = builder().hand(0, &["DSL-APPFUSE-TRASH"]).start();
    let host = fused_host(&mut r);
    seed_trash(&mut r, 0, "TEST-APPFUSE");
    let trash_before = r.trash_size(0);

    let src_idx = hand_index(&r, "DSL-APPFUSE-TRASH");
    r.play(0, src_idx);

    r.execute_action(0, encode_attack(0, host.index as u16))
        .expect("pick host");
    let trash_pos = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "TEST-APPFUSE")
        .unwrap();
    r.execute_action(0, TRASH_EFFECT_START + trash_pos as u16)
        .expect("pick result from trash");

    assert_eq!(
        top_id(&r, host),
        "TEST-APPFUSE",
        "result card stacked from trash"
    );
    assert_eq!(r.trash_size(0), trash_before - 1, "result card left trash");
}

// ── 3. Result filter ────────────────────────────────────────────────────────

#[test]
fn app_fuse_result_filter_excludes_nonmatching_trash_card() {
    let mut r = builder().hand(0, &["DSL-APPFUSE-FILTER"]).start();
    let host = fused_host(&mut r);
    // Both results are route-eligible; only the tagged one has [System].
    seed_trash(&mut r, 0, "TEST-APPFUSE");
    seed_trash(&mut r, 0, "TEST-APPFUSE-TAGGED");

    let src_idx = hand_index(&r, "DSL-APPFUSE-FILTER");
    r.play(0, src_idx);
    r.execute_action(0, encode_attack(0, host.index as u16))
        .expect("pick host");

    // Selection #2 should offer ONLY the [System] result.
    let view = r
        .pending_selection_view()
        .expect("result selection installed");
    let tagged_pos = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "TEST-APPFUSE-TAGGED")
        .unwrap();
    let plain_pos = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "TEST-APPFUSE")
        .unwrap();
    assert!(
        view.valid_action_ids
            .contains(&(TRASH_EFFECT_START + tagged_pos as u16)),
        "the [System] result is offered"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&(TRASH_EFFECT_START + plain_pos as u16)),
        "the non-[System] result is filtered out"
    );
}

// ── 4. Ineligible permanent not offered ─────────────────────────────────────

#[test]
fn app_fuse_ineligible_permanent_not_offered() {
    let mut r = builder()
        .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
        .start();
    let good = fused_host(&mut r);
    // A bare Kabemon (no linked second material) is NOT app-fuse-eligible.
    let bare = r.place_on_field(0, "TEST-KABEMON", Some(0));

    let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
    r.play(0, src_idx);

    let view = r.pending_selection_view().expect("perm selection installs");
    assert!(
        view.valid_action_ids
            .contains(&encode_attack(0, good.index as u16)),
        "the fully-materialed host is eligible"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, bare.index as u16)),
        "the bare host (one material only) is NOT eligible"
    );
}

// ── 5. No eligible permanent → silent no-op ─────────────────────────────────

#[test]
fn app_fuse_no_eligible_permanent_is_silent_noop() {
    let mut r = builder()
        .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
        .start();
    // No host with materials on the field at all.
    let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
    r.play(0, src_idx);
    assert!(
        r.pending_selection_view().is_none(),
        "no selection installs with no eligible host"
    );
}

// ── 6. Decline at permanent pick ────────────────────────────────────────────

#[test]
fn app_fuse_decline_at_permanent_pick_does_nothing() {
    let mut r = builder()
        .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
        .start();
    let host = fused_host(&mut r);
    let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
    r.play(0, src_idx);

    assert!(r.pending_is_optional(), "perm selection is optional (may)");
    r.execute_action(0, PASS).expect("decline");
    assert_eq!(
        top_id(&r, host),
        "TEST-KABEMON",
        "host unchanged after decline"
    );
    assert!(r.pending_selection_view().is_none(), "no further selection");
}

// ── 7. Decline at card pick ─────────────────────────────────────────────────

#[test]
fn app_fuse_decline_at_card_pick_does_nothing() {
    let mut r = builder()
        .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
        .start();
    let host = fused_host(&mut r);
    let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
    r.play(0, src_idx);

    r.execute_action(0, encode_attack(0, host.index as u16))
        .expect("pick host");
    assert!(r.pending_is_optional(), "card selection is optional (may)");
    r.execute_action(0, PASS).expect("decline card");
    assert_eq!(
        top_id(&r, host),
        "TEST-KABEMON",
        "host unchanged after declining the card pick"
    );
}

// ── 8. Requires two distinct named materials ────────────────────────────────

#[test]
fn app_fuse_requires_two_distinct_named_materials() {
    let mut r = builder()
        .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
        .start();
    // Host: Kabemon top + Kabemon linked = SAME name twice → not eligible
    // (App Fusion needs two DISTINCT named conditions).
    let host = r.place_on_field(0, "TEST-KABEMON", Some(0));
    r.push_linked_owned(host, "TEST-KABEMON", 0);

    let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
    r.play(0, src_idx);
    assert!(
        r.pending_selection_view().is_none(),
        "duplicate-name materials do not satisfy App Fusion"
    );
}
