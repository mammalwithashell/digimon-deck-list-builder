//! BT24-087 Rei Katsura — Tamer, Purple, Play Cost 3.
//! Traits: App Driver / Appmon
//!
//! # Printed card text (card image — authoritative)
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [Your Turn] When any of your Digimon get linked, by suspending this Tamer,
//!   <Draw 1> (Draw 1 card from your deck.) and trash 1 card in your hand.
//!   Then, 1 of your Digimon may app fuse into a Digimon card with the [System],
//!   [Life] or [Transmutation (App Name)] trait in the trash.
//! [Security] Play this card without paying the cost.
//!
//! # Gaps and omissions
//! The "Then, 1 of your Digimon may app fuse into a Digimon card with the
//! [System], [Life] or [Transmutation (App Name)] trait in the trash."
//! rider is OMITTED — no engine primitive for effect-initiated App Fusion
//! (`EffectContext::effect_initiated_app_fuse` / DSL `app_fuse` step).
//! Gap: effect-initiated app fuse — see docs/RUST_ENGINE_GAPS.md (App Fuse entry).
//! Same gap keeps BT21-084 and BT25-089 PARTIAL.
//!
//! The "<Draw 1> and trash 1 card in your hand" portion IS implemented.
//!
//! # Verdict: PARTIAL
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Purple/BT24_087.cs

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT24-087";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-087 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("MY-DIGIMON", 4, 4000, 4))
        .add_card(make_digimon("OPP-DIGIMON", 4, 4000, 4))
        .add_card(make_test_card("HAND-CARD", "HandCard"))
        .add_card(make_test_card("LINKED-CARD", "LinkedCard"))
        .deck(1, &["DECK-PAD"; 12])
}

/// Fire a synthetic OnLink event: place a card from the registry into the
/// host's linked_cards directly, then enqueue the OnLink trigger as the
/// real engine does after a link resolves.
fn fire_link_event(
    runner: &mut DebugRunner,
    scanning_player: PlayerId,
    host: PermanentHandle,
    linked_card_id: &str,
) {
    let linked_card = runner.push_linked_owned(host, linked_card_id, host.player);
    runner.game.enqueue_triggered(
        EffectTiming::OnLink,
        TriggerSource::Linked {
            player: scanning_player,
            host,
            card: linked_card,
        },
    );
    runner.game.drain_effect_queue();
}

// ─── Structural tests ─────────────────────────────────────────────────────────

#[test]
fn bt24_087_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Rei Katsura");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
}

#[test]
fn bt24_087_security_clause_has_play_from_security_step() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT24-087 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist on BT24-087");

    let has_play_from_security = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity));
    assert!(
        has_play_from_security,
        "on_security clause must lower to a PlayFromSecurity step"
    );
}

// ─── Clause 1: [Start of Your Main Phase] gain memory if opp has Digimon ─────

/// Positive: when opponent has a Digimon, gain 1 memory.
#[test]
fn bt24_087_start_of_main_gains_memory_when_opponent_has_digimon() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(3).start();
    let rei = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-DIGIMON", Some(0));
    let mem_before = r.game.memory;

    let handle = r.perm_handle(0, rei.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();

    assert_eq!(
        r.game.memory,
        mem_before + 1,
        "opponent has a Digimon → must gain 1 memory"
    );
}

/// Negative: when opponent has NO Digimon, memory does not change.
#[test]
fn bt24_087_start_of_main_no_gain_when_opponent_has_no_digimon() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(3).start();
    let rei = r.place_on_field(0, CARD_ID, Some(0));
    // No Digimon placed on opponent's field.
    let mem_before = r.game.memory;

    let handle = r.perm_handle(0, rei.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();

    assert_eq!(
        r.game.memory, mem_before,
        "opponent has no Digimon → memory must not change"
    );
}

// ─── Clause 2: on_any_link observer ───────────────────────────────────────────

/// When our Digimon gets linked on our turn, and Rei is unsuspended,
/// the optional activation prompt appears.
#[test]
fn bt24_087_own_digimon_linked_on_your_turn_installs_optional_prompt() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &["HAND-CARD"])
        .memory(5)
        .start();
    let rei = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(
        r.pending_selection().is_some(),
        "OnLink on own Digimon during your turn → optional activation prompt must appear"
    );
}

/// Accepting the prompt: Rei is suspended, player draws 1, then mandatory
/// trash-1-hand prompt is installed.
#[test]
fn bt24_087_accept_prompt_suspends_rei_and_draws_then_trash_prompt() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &["HAND-CARD"])
        .memory(5)
        .start();
    let rei = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let deck_before = r.deck_size(0);

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(r.pending_selection().is_some(), "optional prompt must appear");

    // Accept the outer optional prompt (first valid action is the accept).
    let sel = r.game.pending_selection.as_ref().unwrap();
    let action = sel.valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.game.drain_effect_queue();

    assert!(
        r.game.players[0].battle_area[rei.index as usize].is_suspended,
        "Rei must be suspended after paying activation cost"
    );
    assert_eq!(
        r.deck_size(0),
        deck_before - 1,
        "player must draw exactly 1 card"
    );
    // After draw, mandatory trash prompt should appear (hand non-empty).
    assert!(
        r.pending_selection().is_some(),
        "after draw, mandatory trash-1-hand prompt must appear"
    );
}

/// Full happy path: accept → suspend + draw 1 → trash 1 hand card.
/// Verifies that after completing the trash, hand size decreases by net 0
/// (drew 1, trashed 1).
#[test]
fn bt24_087_accept_suspend_draw_then_trash_one_hand_card() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &["HAND-CARD"])
        .memory(5)
        .start();
    let rei = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let hand_before = r.game.players[0].hand.len();
    let trash_before = r.game.players[0].trash.len();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    // Step 1: accept the outer optional prompt.
    {
        let sel = r.game.pending_selection.as_ref().unwrap();
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }

    assert!(
        r.game.players[0].battle_area[rei.index as usize].is_suspended,
        "Rei suspended"
    );

    // Step 2: mandatory trash-1-hand prompt — pick any card.
    assert!(
        r.pending_selection().is_some(),
        "mandatory trash prompt must appear"
    );
    {
        let sel = r.game.pending_selection.as_ref().unwrap();
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }

    // Net hand change: drew 1, trashed 1 → same size as before.
    assert_eq!(
        r.game.players[0].hand.len(),
        hand_before,
        "drew 1 and trashed 1 → hand size must be unchanged"
    );
    // Trash grew by 1.
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before + 1,
        "trash must grow by exactly 1"
    );
}

/// Declining the outer optional prompt: nothing changes.
#[test]
fn bt24_087_decline_prompt_no_effect() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &["HAND-CARD"])
        .memory(5)
        .start();
    let rei = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let deck_before = r.deck_size(0);
    let hand_before = r.game.players[0].hand.len();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(r.pending_selection().is_some(), "optional prompt appears");

    // Decline via PASS.
    use digimon_engine::action::space::PASS;
    let _ = r.game.resolve_selection(0, PASS);
    r.game.drain_effect_queue();

    assert!(
        !r.game.players[0].battle_area[rei.index as usize].is_suspended,
        "declining must leave Rei unsuspended"
    );
    assert_eq!(r.deck_size(0), deck_before, "declining must not draw cards");
    assert_eq!(
        r.game.players[0].hand.len(),
        hand_before,
        "declining must not change hand size"
    );
}

/// Already-suspended Rei cannot pay the activation cost → no prompt.
#[test]
fn bt24_087_already_suspended_rei_no_prompt() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &["HAND-CARD"])
        .memory(5)
        .start();
    let rei = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    // Manually suspend Rei.
    r.game.players[0].battle_area[rei.index as usize].is_suspended = true;

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(
        r.pending_selection().is_none(),
        "already-suspended Rei cannot pay the activation cost — no prompt"
    );
}

/// Opponent's turn: even if our Digimon gets linked, no prompt fires
/// (active_when: your_turn: true gate).
#[test]
fn bt24_087_opponents_turn_link_no_prompt() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &["HAND-CARD"])
        .memory(5)
        .start();
    let rei = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    // End player 0's turn → it becomes player 1's turn.
    r.game.end_turn();
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(
        r.pending_selection().is_none(),
        "on opponent's turn, on_any_link should not fire for Rei"
    );
}

/// Opponent's Digimon gets linked: no prompt fires
/// (event_target_owner: you gate — host is opponent, not us).
#[test]
fn bt24_087_opponent_digimon_linked_no_prompt() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &["HAND-CARD"])
        .memory(5)
        .start();
    let rei = r.place_on_field(0, CARD_ID, Some(0));
    let opp_host = r.place_on_field(1, "OPP-DIGIMON", Some(0));
    r.game.enter_main_phase();

    // Fire the OnLink trigger for player 0 scanning, but HOST is player 1's Digimon.
    let opp_host_handle = r.perm_handle(1, opp_host.index as usize);
    fire_link_event(&mut r, 0, opp_host_handle, "LINKED-CARD");

    assert!(
        r.pending_selection().is_none(),
        "opponent's Digimon as link host → event_target_owner is opponent → no prompt"
    );
}

// ─── Clause 3: security play ──────────────────────────────────────────────────

// Structural test: the on_security clause compiles with PlayFromSecurity.
// (Already covered by bt24_087_security_clause_has_play_from_security_step above.)

// ─── Clause 2 tail: App Fuse from trash with [System]/[Life]/[Transmutation] ──
// (effect-initiated App Fuse, 2026-06-13)

/// App-Fusion result with the [System] trait + materials Kabemon & Gomimon.
const APP_FUSE_RESULT_SYSTEM: &str = r#"
card: TEST-APPFUSE-SYS
name: Test App Fusion System
kind: digimon
level: 5
color: [purple]
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

fn named_material(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = vec!["Appmon".to_string()];
    c
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

/// Full chain: link onto a materialed host → accept → suspend + draw → trash 1
/// → app fuse the [System] result from trash onto the host.
#[test]
fn bt24_087_app_fuse_from_trash_filtered_by_trait() {
    use digimon_engine::action::space::encode_attack;

    let mut r = base()
        .from_dsl_yaml(APP_FUSE_RESULT_SYSTEM)
        .expect("APP_FUSE_RESULT_SYSTEM compiles")
        .add_card(named_material("TEST-KABEMON", "Kabemon"))
        .add_card(named_material("TEST-GOMIMON", "Gomimon"))
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &["HAND-CARD"])
        .memory(5)
        .start();
    let rei = r.place_on_field(0, CARD_ID, Some(0));
    // Host: top = Kabemon. The link event below links Gomimon → 2 distinct
    // named materials (app-fuse eligible) AND fires the on_any_link observer.
    let host = r.place_on_field(0, "TEST-KABEMON", Some(0));
    seed_trash(&mut r, 0, "TEST-APPFUSE-SYS");
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "TEST-GOMIMON");

    // 1. Accept the outer optional prompt.
    {
        let sel = r.game.pending_selection.as_ref().expect("optional prompt");
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }
    // 2. Mandatory trash-1-hand: pick the hand card.
    {
        let sel = r.game.pending_selection.as_ref().expect("trash prompt");
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }
    // 3. App-fuse selection #1: the host permanent.
    let host_action = encode_attack(0, host.index as u16);
    {
        let view = r
            .pending_selection_view()
            .expect("app-fuse perm selection installs after trash");
        assert!(
            view.valid_action_ids.contains(&host_action),
            "host with Kabemon+Gomimon is an eligible app-fuse target"
        );
        r.execute_action(0, host_action).expect("pick host");
    }
    // 4. App-fuse selection #2: the [System] result in trash.
    {
        let trash_pos = r.game.players[0]
            .trash
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == "TEST-APPFUSE-SYS")
            .expect("System result still in trash");
        let action = digimon_engine::action::space::TRASH_EFFECT_START + trash_pos as u16;
        r.execute_action(0, action).expect("pick System result");
    }

    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "TEST-APPFUSE-SYS",
        "the [System] result app-fused from trash onto the host"
    );
}

/// A result card lacking System/Life/Transmutation is NOT offered by the
/// filtered app-fuse selection.
#[test]
fn bt24_087_app_fuse_filters_out_nonmatching_trait() {
    use digimon_engine::action::space::{encode_attack, TRASH_EFFECT_START};

    // A second result with NO System/Life/Transmutation trait (same materials).
    const APP_FUSE_RESULT_PLAIN: &str = r#"
card: TEST-APPFUSE-PLAIN
name: Test App Fusion Plain
kind: digimon
level: 5
color: [purple]
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

    let mut r = base()
        .from_dsl_yaml(APP_FUSE_RESULT_SYSTEM)
        .expect("system result compiles")
        .from_dsl_yaml(APP_FUSE_RESULT_PLAIN)
        .expect("plain result compiles")
        .add_card(named_material("TEST-KABEMON", "Kabemon"))
        .add_card(named_material("TEST-GOMIMON", "Gomimon"))
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &["HAND-CARD"])
        .memory(5)
        .start();
    let _rei = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "TEST-KABEMON", Some(0));
    seed_trash(&mut r, 0, "TEST-APPFUSE-SYS");
    seed_trash(&mut r, 0, "TEST-APPFUSE-PLAIN");
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "TEST-GOMIMON");
    // accept → trash → reach app-fuse perm selection.
    for _ in 0..2 {
        let sel = r.game.pending_selection.as_ref().expect("prompt");
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }
    r.execute_action(0, encode_attack(0, host.index as u16))
        .expect("pick host");

    let view = r.pending_selection_view().expect("result selection");
    let sys_pos = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "TEST-APPFUSE-SYS")
        .unwrap();
    let plain_pos = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "TEST-APPFUSE-PLAIN")
        .unwrap();
    assert!(
        view.valid_action_ids.contains(&(TRASH_EFFECT_START + sys_pos as u16)),
        "the [System] result is offered"
    );
    assert!(
        !view.valid_action_ids.contains(&(TRASH_EFFECT_START + plain_pos as u16)),
        "the no-trait result is filtered out"
    );
}
