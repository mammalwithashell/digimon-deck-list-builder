//! BT23-079 Eri Karan — Tamer, Blue, Play Cost 3.
//! Traits: App Driver / Appmon
//!
//! # Printed card text (card image — authoritative)
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [Your Turn] When any of your Digimon get linked, by suspending this Tamer,
//!   those Digimon get +3000 DP until your opponent's turn ends.
//!   Then, 1 of your Digimon may app fuse into a Digimon card in the hand.
//! [Security] Play this card without paying the cost.
//!
//! # Gaps and omissions
//! The "Then, 1 of your Digimon may app fuse into a Digimon card in the hand."
//! rider is OMITTED — no engine primitive for effect-initiated App Fusion
//! (`EffectContext::effect_initiated_app_fuse` / DSL `app_fuse` step).
//! Gap: effect-initiated app fuse — see docs/RUST_ENGINE_GAPS.md (App Fuse entry).
//! Same gap keeps BT21-084, BT24-087, and BT25-089 PARTIAL.
//!
//! The "+3000 DP until your opponent's turn ends" IS implemented via
//! `add_dp_modifier { target: event_target, value: 3000, expiry: end_of_opponents_turn }`.
//! `CompiledBindingRef::EventTarget` resolves to `event_permanent` (the link host)
//! in the `OnLink` trigger context — confirmed in `dsl_cards/binding_ref.rs`.
//!
//! # Verdict: PARTIAL
//! Gap kind: engine (effect-initiated app fuse)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Blue/BT23_079.cs

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT23-079";

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
        .expect("BT23-079 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("MY-DIGIMON", 4, 4000, 4))
        .add_card(make_digimon("OPP-DIGIMON", 4, 4000, 4))
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
fn bt23_079_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Eri Karan");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
}

#[test]
fn bt23_079_security_clause_has_play_from_security_step() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT23-079 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist on BT23-079");

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
fn bt23_079_start_of_main_gains_memory_when_opponent_has_digimon() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(3).start();
    let eri = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-DIGIMON", Some(0));
    let mem_before = r.game.memory;

    let handle = r.perm_handle(0, eri.index as usize);
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
fn bt23_079_start_of_main_no_gain_when_opponent_has_no_digimon() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(3).start();
    let eri = r.place_on_field(0, CARD_ID, Some(0));
    // No Digimon placed on opponent's field.
    let mem_before = r.game.memory;

    let handle = r.perm_handle(0, eri.index as usize);
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

/// When our Digimon gets linked on our turn, and Eri is unsuspended,
/// the optional activation prompt appears.
#[test]
fn bt23_079_own_digimon_linked_on_your_turn_installs_optional_prompt() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let _eri = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(
        r.pending_selection().is_some(),
        "OnLink on own Digimon during your turn → optional activation prompt must appear"
    );
}

/// Accepting the prompt: Eri is suspended, and the linked host gets +3000 DP.
#[test]
fn bt23_079_accept_suspends_eri_and_boosts_dp() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let eri = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    let dp_before = r
        .effective_dp(host_handle)
        .expect("host must have effective DP");

    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    // Accept the outer optional prompt.
    {
        let sel = r.game.pending_selection.as_ref().unwrap();
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }

    assert!(
        r.game.players[0].battle_area[eri.index as usize].is_suspended,
        "Eri must be suspended after paying activation cost"
    );
    assert_eq!(
        r.effective_dp(host_handle),
        Some(dp_before + 3000),
        "linked host must gain +3000 DP"
    );
}

/// The +3000 DP boost expires at the end of the opponent's turn.
#[test]
fn bt23_079_dp_boost_expires_end_of_opponents_turn() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let eri = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    let dp_before = r
        .effective_dp(host_handle)
        .expect("host must have effective DP");

    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    // Accept the prompt.
    {
        let sel = r.game.pending_selection.as_ref().unwrap();
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }

    assert_eq!(
        r.effective_dp(host_handle),
        Some(dp_before + 3000),
        "boost must be active after accepting"
    );

    // Advance to opponent's turn end.
    r.game.end_turn(); // P0 ends → P1's turn
    r.game.enter_main_phase();
    r.game.end_turn(); // P1 ends → P0's turn (end_of_opponents_turn drains here)

    assert_eq!(
        r.effective_dp(host_handle),
        Some(dp_before),
        "+3000 DP modifier must have expired after opponent's turn end"
    );
}

/// Declining the outer optional prompt: nothing changes.
#[test]
fn bt23_079_decline_prompt_no_effect() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let eri = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    let dp_before = r.effective_dp(host_handle).expect("host has effective DP");

    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(r.pending_selection().is_some(), "optional prompt appears");

    // Decline via PASS.
    use digimon_engine::action::space::PASS;
    let _ = r.game.resolve_selection(0, PASS);
    r.game.drain_effect_queue();

    assert!(
        !r.game.players[0].battle_area[eri.index as usize].is_suspended,
        "declining must leave Eri unsuspended"
    );
    assert_eq!(
        r.effective_dp(host_handle),
        Some(dp_before),
        "declining must not boost DP"
    );
}

/// Already-suspended Eri cannot pay the activation cost → no prompt.
#[test]
fn bt23_079_already_suspended_eri_no_prompt() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let eri = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    // Manually suspend Eri.
    r.game.players[0].battle_area[eri.index as usize].is_suspended = true;

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(
        r.pending_selection().is_none(),
        "already-suspended Eri cannot pay the activation cost — no prompt"
    );
}

/// Opponent's turn: even if our Digimon gets linked, no prompt fires
/// (active_when: your_turn: true gate).
#[test]
fn bt23_079_opponents_turn_link_no_prompt() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let _eri = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "MY-DIGIMON", Some(0));
    r.game.enter_main_phase();

    // End player 0's turn → it becomes player 1's turn.
    r.game.end_turn();
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(
        r.pending_selection().is_none(),
        "on opponent's turn, on_any_link should not fire for Eri"
    );
}

/// Opponent's Digimon gets linked: no prompt fires
/// (event_target_owner: you gate — host is opponent, not us).
#[test]
fn bt23_079_opponent_digimon_linked_no_prompt() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let _eri = r.place_on_field(0, CARD_ID, Some(0));
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
// (Already covered by bt23_079_security_clause_has_play_from_security_step above.)

// ─── Clause 2 tail: App Fuse from hand (effect-initiated, 2026-06-13) ─────────

const APP_FUSE_RESULT: &str = r#"
card: TEST-APPFUSE
name: Test App Fusion Digimon
kind: digimon
level: 5
color: [blue]
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

/// Full chain: link Gomimon onto a Kabemon host → accept → suspend + the host
/// gets +3000 DP → app fuse a hand card onto that host.
#[test]
fn bt23_079_app_fuse_from_hand_after_dp_buff() {
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
    let _eri = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "TEST-KABEMON", Some(0));
    r.game.enter_main_phase();
    let dp_before = r.effective_dp(host).expect("host on field");

    let host_handle = r.perm_handle(0, host.index as usize);
    // Links Gomimon onto the host → fires the observer AND completes the
    // 2-distinct-materials app-fuse condition.
    fire_link_event(&mut r, 0, host_handle, "TEST-GOMIMON");

    // 1. Accept the outer optional prompt.
    {
        let sel = r.game.pending_selection.as_ref().expect("optional prompt");
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }

    // The linked host got +3000 DP.
    assert_eq!(
        r.effective_dp(host),
        Some(dp_before + 3000),
        "linked host gains +3000 DP"
    );

    // 2. App-fuse selection #1: pick the host.
    let host_action = encode_attack(0, host.index as u16);
    {
        let view = r
            .pending_selection_view()
            .expect("app-fuse perm selection installs after the DP buff");
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
        "hand result app-fused onto the host after the DP buff"
    );
}
