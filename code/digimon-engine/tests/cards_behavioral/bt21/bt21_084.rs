//! BT21-084 Haru Shinkai — Tamer, Red, Cost 4.
//! Traits: App Driver / Appmon / Hero
//!
//! # Printed card text (card image — authoritative)
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [Your Turn] When your Digimon get linked, by suspending this Tamer,
//!   <Draw 1>. Then, 1 of your Digimon may app fuse into a Digimon card
//!   in the hand.
//! [Security] Play this card without paying the cost.
//!
//! # Gaps and omissions
//! The "Then, 1 of your Digimon may app fuse into a Digimon card in the hand."
//! rider is OMITTED — no engine primitive for effect-initiated App Fusion exists
//! (`EffectContext::effect_initiated_app_fuse` / DSL `app_fuse` step).
//! Gap: effect-initiated app fuse — see docs/RUST_ENGINE_GAPS.md (App Fuse entry).
//! Same gap keeps BT25-089 PARTIAL.
//!
//! # Verdict: PARTIAL
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_084.cs

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT21-084";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card
}

fn make_appmon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.traits = vec!["Appmon".to_string()];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-084 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("MY-DIGIMON", 4, 4000, 4))
        .add_card(make_digimon("OPP-DIGIMON", 4, 4000, 4))
        .add_card(make_appmon("APPMON-CARD"))
        .deck(1, &["DECK-PAD"; 12])
}

/// Fire a synthetic OnLink event: place a card from the card registry into the
/// host's linked_cards directly (no cost), then enqueue the OnLink trigger as
/// the real engine does after a link resolves.
fn fire_link_event(
    runner: &mut DebugRunner,
    scanning_player: PlayerId,
    host: PermanentHandle,
    linked_card_id: &str,
) {
    // Push a linked card onto the host (bypasses costs/prompts — pure state mutation).
    let linked_card = runner.push_linked_owned(host, linked_card_id, host.player);
    // Enqueue the OnLink observer exactly as game_actions/link.rs does.
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
fn bt21_084_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Haru Shinkai");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
}

// ─── Clause 1: memory ramp ────────────────────────────────────────────────────

#[test]
fn bt21_084_start_of_turn_sets_memory_to_3_when_at_0() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let haru = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, haru.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 0 (<=2) → must be set to 3");
}

#[test]
fn bt21_084_start_of_turn_sets_memory_to_3_when_at_2() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(2).start();
    let haru = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, haru.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 2 (<=2) → must be set to 3");
}

#[test]
fn bt21_084_start_of_turn_does_not_change_memory_when_at_3() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(3).start();
    let haru = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, haru.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 3 (>2) → must remain 3");
}

#[test]
fn bt21_084_start_of_turn_does_not_change_memory_when_above_3() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let haru = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, haru.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 5, "memory was 5 (>2) → must remain unchanged");
}

// ─── Clause 2: on_any_link observer — prompt fires ────────────────────────────

/// When our Digimon gets linked on our turn, the OnLink event fires and the
/// optional activation prompt appears (Haru must be unsuspended so the
/// suspend cost can be paid).
#[test]
fn bt21_084_own_digimon_linked_on_your_turn_installs_optional_prompt() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let haru = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "APPMON-CARD");

    assert!(
        r.pending_selection().is_some(),
        "OnLink on own Digimon during your turn should install optional activation prompt"
    );
}

/// Accepting the prompt: Haru is suspended and player draws a card.
#[test]
fn bt21_084_accept_prompt_suspends_haru_and_draws() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let haru = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let deck_before = r.deck_size(0);

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "APPMON-CARD");

    assert!(r.pending_selection().is_some(), "optional prompt must appear");

    // Accept: pick the first non-pass action (the "yes" action).
    let sel = r.game.pending_selection.as_ref().unwrap();
    let action = sel.valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    // Drain any remaining steps.
    r.game.drain_effect_queue();

    assert!(
        r.game.players[0].battle_area[haru.index as usize].is_suspended,
        "Haru must be suspended after paying activation cost"
    );
    assert_eq!(
        r.deck_size(0),
        deck_before - 1,
        "player must draw exactly 1 card"
    );
}

/// Declining the prompt: nothing changes (Haru stays unsuspended, no draw).
#[test]
fn bt21_084_decline_prompt_no_effect() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let haru = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let deck_before = r.deck_size(0);
    let hand_before = r.game.players[0].hand.len();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "APPMON-CARD");

    assert!(r.pending_selection().is_some(), "optional prompt appears");

    // Decline: use PASS (the decline action in an optional prompt).
    use digimon_engine::action::space::PASS;
    let _ = r.game.resolve_selection(0, PASS);
    r.game.drain_effect_queue();

    assert!(
        !r.game.players[0].battle_area[haru.index as usize].is_suspended,
        "declining must leave Haru unsuspended"
    );
    assert_eq!(
        r.game.players[0].hand.len(),
        hand_before,
        "declining must not draw cards"
    );
}

/// Already-suspended Haru cannot pay the activation cost → no prompt.
#[test]
fn bt21_084_already_suspended_haru_no_prompt() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let haru = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    // Manually suspend Haru.
    r.game.players[0].battle_area[haru.index as usize].is_suspended = true;

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "APPMON-CARD");

    assert!(
        r.pending_selection().is_none(),
        "already-suspended Haru cannot pay the activation cost — no prompt"
    );
}

/// Opponent's turn: even if our Digimon gets linked, no prompt fires
/// (active_when: your_turn: true gate).
#[test]
fn bt21_084_opponents_turn_link_no_prompt() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let haru = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    // Advance to player 1's turn by ending player 0's turn.
    r.game.end_turn();
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "APPMON-CARD");

    assert!(
        r.pending_selection().is_none(),
        "on opponent's turn, on_any_link should not fire for Haru"
    );
}

/// Opponent's Digimon gets linked (host is player 1): no prompt fires
/// (event_target_owner: you gate — host is opponent, not us).
#[test]
fn bt21_084_opponent_digimon_linked_no_prompt() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let haru = r.place_on_field(0, CARD_ID, Some(0));
    let opp_host = r.place_on_field(1, "OPP-DIGIMON", Some(0));
    r.game.enter_main_phase();

    // Fire the OnLink trigger for player 0 scanning, but the HOST is player 1's Digimon.
    let opp_host_handle = r.perm_handle(1, opp_host.index as usize);
    fire_link_event(&mut r, 0, opp_host_handle, "APPMON-CARD");

    assert!(
        r.pending_selection().is_none(),
        "opponent's Digimon as link host → event_target_owner is opponent → no prompt"
    );
}

// ─── Clause 3: security play ──────────────────────────────────────────────────

/// Structural test: the `on_security` clause compiles with a `PlayFromSecurity`
/// step. The behavioral runtime for play-self-free security is well-covered by
/// BT18-087, BT22-084, BT21-015; here we only assert the clause is present so
/// security checks can route to it correctly.
#[test]
fn bt21_084_security_clause_has_play_from_security_step() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT21-084 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist on BT21-084");

    let has_play_from_security = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity));
    assert!(
        has_play_from_security,
        "on_security clause must lower to a PlayFromSecurity step"
    );
}

// ─── Clause 2 tail: App Fuse from hand (effect-initiated, 2026-06-13) ─────────

const APP_FUSE_RESULT: &str = r#"
card: TEST-APPFUSE
name: Test App Fusion Digimon
kind: digimon
level: 5
color: [red]
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

fn named_material(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = vec!["Appmon".to_string()];
    c
}

/// Full chain: link Gomimon onto a Kabemon host → accept → suspend + draw 1 →
/// app fuse a hand card onto that host.
#[test]
fn bt21_084_app_fuse_from_hand_after_draw() {
    use digimon_engine::action::space::{encode_attack, HAND_EFFECT_START};

    let mut r = base()
        .from_dsl_yaml(APP_FUSE_RESULT)
        .expect("APP_FUSE_RESULT compiles")
        .add_card(named_material("TEST-KABEMON", "Kabemon"))
        .add_card(named_material("TEST-GOMIMON", "Gomimon"))
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &["TEST-APPFUSE"])
        .memory(5)
        .start();
    let _haru = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "TEST-KABEMON", Some(0));
    r.game.enter_main_phase();
    let deck_before = r.deck_size(0);

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "TEST-GOMIMON");

    // 1. Accept the outer optional prompt → suspend + draw 1.
    {
        let sel = r.game.pending_selection.as_ref().expect("optional prompt");
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }
    assert_eq!(r.deck_size(0), deck_before - 1, "drew 1 card");

    // 2. App-fuse selection #1: pick the host.
    let host_action = encode_attack(0, host.index as u16);
    {
        let view = r
            .pending_selection_view()
            .expect("app-fuse perm selection installs after the draw");
        assert!(
            view.valid_action_ids.contains(&host_action),
            "host with Kabemon+Gomimon is an eligible app-fuse target"
        );
        r.execute_action(0, host_action).expect("pick host");
    }
    // 3. App-fuse selection #2: the hand result.
    {
        let card_pos = r.game.players[0]
            .hand
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == "TEST-APPFUSE")
            .expect("result still in hand");
        r.execute_action(0, HAND_EFFECT_START + card_pos as u16)
            .expect("pick hand result");
    }

    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "TEST-APPFUSE",
        "hand result app-fused onto the host after the draw"
    );
}
