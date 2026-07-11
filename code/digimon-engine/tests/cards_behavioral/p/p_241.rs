//! P-241 Yujin Ozora — Tamer, Green, Play Cost 4.
//! Traits: App Driver, Appmon
//!
//! # Printed card text (card image — authoritative)
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [Your Turn] When any of your Digimon get linked, by suspending this Tamer,
//!   1 of your Digimon with the [Appmon] trait gains <Vortex> (At the end of
//!   your turn, this Digimon may attack an opponent's Digimon. With this
//!   effect, it can attack the turn it was played.) and +3000 DP for the turn.
//!   Then, 1 of your Digimon may app fuse into a Digimon card in the hand.
//! [Security] Play this card without paying the cost.
//!
//! # Implementation notes (audited 2026-07-10)
//! All clauses are implemented — the earlier "app fuse OMITTED / PARTIAL"
//! header was stale: the tail "Then, 1 of your Digimon may app fuse into a
//! Digimon card in the hand." ships via the effect-initiated `app_fuse` step
//! (`app_fuse: { from: hand, optional: true }`, 2026-06-13) and is covered by
//! `p_241_app_fuse_from_hand_after_vortex_grant` below.
//!
//! The "<Vortex> + +3000 DP for the turn" portion:
//!   - `grant_keyword: { target: tgt, keyword: Vortex, expiry: end_of_your_turn }`
//!   - `add_dp_modifier: { target: tgt, value: 3000, expiry: end_of_your_turn }`
//! Both expire end_of_your_turn (DCGO EffectDuration.UntilEachTurnEnd; the
//! effect is [Your Turn]-gated so the current turn is always yours). `tgt` is
//! a player-chosen [Appmon] Digimon, MANDATORY once the suspend cost is paid
//! (DCGO buff select canNoSelect: false) and NOT event_target — the card says
//! "1 of your Digimon with the [Appmon] trait".
//!
//! DCGO guarded regions: with no [Appmon] Digimon on the field the buff pick
//! is skipped (`if (HasMatchConditionPermanent(...))`) but the app-fuse region
//! STILL runs — covered by `p_241_no_appmon_buff_skipped_app_fuse_still_offered`.
//! No [Once Per Turn] (DCGO use cap -1); the suspend cost is the natural lock,
//! covered by `p_241_already_suspended_yujin_no_prompt`.
//!
//! # Verdict: IMPLEMENTED
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Green/P_241.cs (base repo — the earlier
//! "ABSENT — promo with no DCGO file" note was stale; the file exists and
//! this suite was cross-checked against it).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "P-241";

// ─── Card factory helpers ────────────────────────────────────────────────────

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = 4;
    card
}

fn make_appmon(id: &str) -> CardData {
    let mut card = make_digimon(id, 4, 4000);
    card.traits = vec!["Appmon".to_string()];
    card
}

fn make_plain_digimon(id: &str) -> CardData {
    make_digimon(id, 4, 4000)
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-241 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_appmon("APPMON-DIG"))
        .add_card(make_plain_digimon("PLAIN-DIG"))
        .add_card(make_plain_digimon("OPP-DIGIMON"))
        .add_card(make_test_card("LINKED-CARD", "LinkedCard"))
        .deck(1, &["DECK-PAD"; 12])
}

/// Fire a synthetic OnLink event: push a linked card onto the host, then
/// enqueue the OnLink trigger exactly as game_actions/link.rs does.
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
fn p_241_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("P-241 in pack");
    assert_eq!(card.name, "Yujin Ozora");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
}

#[test]
fn p_241_security_clause_has_play_from_security_step() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("P-241 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist on P-241");

    let has_play_from_security = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity));
    assert!(
        has_play_from_security,
        "on_security clause must lower to a PlayFromSecurity step"
    );
}

// ─── Clause 1: [Start of Your Turn] memory ramp ──────────────────────────────

/// Positive: memory at 0 (≤2) → set to 3.
#[test]
fn p_241_start_of_turn_sets_memory_to_3_when_at_0() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let yujin = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, yujin.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 0 (≤2) → must be set to 3");
}

/// Positive: memory at 2 (≤2) → set to 3.
#[test]
fn p_241_start_of_turn_sets_memory_to_3_when_at_2() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(2).start();
    let yujin = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, yujin.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 2 (≤2) → must be set to 3");
}

/// Negative: memory at 3 (>2) → no change.
#[test]
fn p_241_start_of_turn_no_change_when_memory_at_3() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(3).start();
    let yujin = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, yujin.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 3 (>2) → must remain 3");
}

/// Negative: memory at 5 (>2) → no change.
#[test]
fn p_241_start_of_turn_no_change_when_memory_above_3() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let yujin = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, yujin.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(
        r.game.memory, 5,
        "memory was 5 (>2) → must remain unchanged"
    );
}

// ─── Clause 2: on_any_link observer ───────────────────────────────────────────

/// Own Digimon linked on own turn, Yujin unsuspended → optional prompt appears.
#[test]
fn p_241_own_digimon_linked_on_your_turn_installs_optional_prompt() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let _yujin = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "PLAIN-DIG", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(
        r.pending_selection().is_some(),
        "OnLink on own Digimon during your turn → optional activation prompt must appear"
    );
}

/// Accepting the outer prompt suspends Yujin, then a target-select prompt for the
/// Appmon Digimon appears, and after selecting, the chosen Digimon gains Vortex
/// and +3000 DP.
#[test]
fn p_241_accept_suspends_yujin_then_prompts_appmon_target_and_grants_vortex_plus_dp() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let yujin = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "PLAIN-DIG", Some(0));
    let appmon = r.place_on_field(0, "APPMON-DIG", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    let appmon_handle = r.perm_handle(0, appmon.index as usize);
    let dp_before = r.effective_dp(appmon_handle).expect("APPMON-DIG has DP");

    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    // Step 1: accept the outer optional activation prompt.
    {
        let sel = r.game.pending_selection.as_ref().unwrap();
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }

    // Yujin must now be suspended (activation cost paid).
    assert!(
        r.game.players[0].battle_area[yujin.index as usize].is_suspended,
        "Yujin must be suspended after paying activation cost"
    );

    // Step 2: there should now be a target-selection prompt for the Appmon Digimon.
    assert!(
        r.pending_selection().is_some(),
        "After accepting, a target selection prompt for the Appmon Digimon must appear"
    );

    // Select the APPMON-DIG permanent.
    {
        let sel = r.game.pending_selection.as_ref().unwrap();
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("there must be a valid selection (the Appmon Digimon)");
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }

    // The selected Appmon Digimon must now have Vortex.
    assert!(
        r.game.has_keyword(appmon_handle, Keyword::Vortex),
        "selected Appmon Digimon must gain <Vortex>"
    );

    // And +3000 DP.
    assert_eq!(
        r.effective_dp(appmon_handle),
        Some(dp_before + 3000),
        "selected Appmon Digimon must gain +3000 DP for the turn"
    );
}

/// The buff target selection is MANDATORY (DCGO buff select canNoSelect: false
/// — no PASS once the suspend cost is paid) and restricted to [Appmon]-trait
/// Digimon (the plain, non-Appmon linked host is NOT offered).
#[test]
fn p_241_buff_selection_is_mandatory_and_appmon_only() {
    use digimon_engine::action::space::encode_attack;

    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let _yujin = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "PLAIN-DIG", Some(0));
    let appmon = r.place_on_field(0, "APPMON-DIG", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    // Accept the outer optional prompt (pays the suspend cost).
    {
        let sel = r.game.pending_selection.as_ref().unwrap();
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("buff target selection installs after the cost is paid");
    let appmon_action = encode_attack(0, appmon.index as u16);
    let host_action = encode_attack(0, host.index as u16);
    assert!(
        sel.valid_action_ids.contains(&appmon_action),
        "the [Appmon]-trait Digimon must be a valid buff target"
    );
    assert!(
        !sel.valid_action_ids.contains(&host_action),
        "the plain (non-Appmon) linked host must NOT be a valid buff target — \
         printed text scopes the buff to 'your Digimon with the [Appmon] trait'"
    );
    assert!(
        !sel.valid_action_ids.contains(&PASS),
        "buff selection is mandatory once the suspend cost is paid \
         (DCGO canNoSelect: false) — PASS must not be offered"
    );
}

/// The Vortex and +3000 DP expire at end of your turn.
#[test]
fn p_241_vortex_and_dp_boost_expire_end_of_turn() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let _yujin = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "PLAIN-DIG", Some(0));
    let appmon = r.place_on_field(0, "APPMON-DIG", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    let appmon_handle = r.perm_handle(0, appmon.index as usize);
    let dp_before = r.effective_dp(appmon_handle).expect("APPMON-DIG has DP");

    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    // Accept outer prompt.
    {
        let sel = r.game.pending_selection.as_ref().unwrap();
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }

    // Select the Appmon target.
    {
        let sel = r.game.pending_selection.as_ref().unwrap();
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("Appmon selection action must exist");
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }

    // Confirm the buffs are active.
    assert!(
        r.game.has_keyword(appmon_handle, Keyword::Vortex),
        "Vortex must be active before turn ends"
    );
    assert_eq!(
        r.effective_dp(appmon_handle),
        Some(dp_before + 3000),
        "+3000 DP must be active before turn ends"
    );

    // End P0's turn → expiry fires.
    // Vortex grants an end-of-turn-keyword window so end_turn() parks at
    // GamePhase::EndOfTurnAction; rotate_turn_player (and expire_end_of_turn)
    // fires only after pass_end_of_turn_action() completes the rotation.
    r.game.end_turn();
    r.game.pass_end_of_turn_action();

    assert!(
        !r.game.has_keyword(appmon_handle, Keyword::Vortex),
        "Vortex must expire at end of P0's turn"
    );
    assert_eq!(
        r.effective_dp(appmon_handle),
        Some(dp_before),
        "+3000 DP modifier must expire at end of P0's turn"
    );
}

/// Declining the outer optional prompt → no suspension, no buff, no target prompt.
#[test]
fn p_241_decline_prompt_no_effect() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let yujin = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "PLAIN-DIG", Some(0));
    let appmon = r.place_on_field(0, "APPMON-DIG", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    let appmon_handle = r.perm_handle(0, appmon.index as usize);
    let dp_before = r.effective_dp(appmon_handle).expect("APPMON-DIG has DP");

    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(r.pending_selection().is_some(), "optional prompt appears");

    // Decline via PASS.
    let _ = r.game.resolve_selection(0, PASS);
    r.game.drain_effect_queue();

    assert!(
        !r.game.players[0].battle_area[yujin.index as usize].is_suspended,
        "declining must leave Yujin unsuspended"
    );
    assert!(
        !r.game.has_keyword(appmon_handle, Keyword::Vortex),
        "declining must not grant Vortex"
    );
    assert_eq!(
        r.effective_dp(appmon_handle),
        Some(dp_before),
        "declining must not boost DP"
    );
}

/// Already-suspended Yujin → condition source_is_unsuspended fails → no prompt.
#[test]
fn p_241_already_suspended_yujin_no_prompt() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let yujin = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "PLAIN-DIG", Some(0));
    r.game.enter_main_phase();

    // Manually suspend Yujin.
    r.game.players[0].battle_area[yujin.index as usize].is_suspended = true;

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(
        r.pending_selection().is_none(),
        "already-suspended Yujin cannot pay the activation cost — no prompt"
    );
}

/// Opponent's turn: active_when: your_turn gate → no prompt.
#[test]
fn p_241_opponents_turn_link_no_prompt() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let _yujin = r.place_on_field(0, CARD_ID, Some(0));
    let host = r.place_on_field(0, "PLAIN-DIG", Some(0));
    r.game.enter_main_phase();

    // Advance to player 1's turn.
    r.game.end_turn();
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    fire_link_event(&mut r, 0, host_handle, "LINKED-CARD");

    assert!(
        r.pending_selection().is_none(),
        "on opponent's turn, on_any_link should not fire for Yujin"
    );
}

/// Opponent's Digimon gets linked: event_target_owner gate → no prompt.
#[test]
fn p_241_opponent_digimon_linked_no_prompt() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let _yujin = r.place_on_field(0, CARD_ID, Some(0));
    let opp_host = r.place_on_field(1, "OPP-DIGIMON", Some(0));
    r.game.enter_main_phase();

    let opp_host_handle = r.perm_handle(1, opp_host.index as usize);
    fire_link_event(&mut r, 0, opp_host_handle, "LINKED-CARD");

    assert!(
        r.pending_selection().is_none(),
        "opponent's Digimon as link host → event_target_owner is opponent → no prompt"
    );
}

// ─── Clause 3: security play (structural) ────────────────────────────────────

// Covered by p_241_security_clause_has_play_from_security_step above.
// Behavioral security-play runtime is well-established (BT21-084, BT23-079).

// ─── Clause 2 tail: App Fuse from hand (effect-initiated, 2026-06-13) ─────────

const APP_FUSE_RESULT: &str = r#"
card: TEST-APPFUSE
name: Test App Fusion Digimon
kind: digimon
level: 5
color: [green]
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

/// Full chain: link Gomimon onto a Kabemon (Appmon) host → accept → suspend →
/// pick the host as the <Vortex>/+3000 target → app fuse a hand card onto it.
#[test]
fn p_241_app_fuse_from_hand_after_vortex_grant() {
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
    let _yujin = r.place_on_field(0, CARD_ID, Some(0));
    // Host is an [Appmon] Kabemon — eligible both as the Vortex target and (once
    // Gomimon is linked) as the app-fuse target.
    let host = r.place_on_field(0, "TEST-KABEMON", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    let host_action = encode_attack(0, host.index as u16);
    fire_link_event(&mut r, 0, host_handle, "TEST-GOMIMON");

    // 1. Accept the outer optional prompt → suspend + Vortex-target selection.
    {
        let sel = r.game.pending_selection.as_ref().expect("optional prompt");
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }
    // 2. Vortex/+3000 target selection: pick the host (the only [Appmon] Digimon).
    {
        let _sel = r.pending_selection_view().expect("Vortex target selection");
        r.execute_action(0, host_action)
            .expect("pick Vortex target");
    }
    assert!(
        r.game.has_keyword(host_handle, Keyword::Vortex),
        "host gains <Vortex> for the turn"
    );

    // 3. App-fuse selection #1: pick the host.
    {
        let view = r
            .pending_selection_view()
            .expect("app-fuse perm selection installs after the Vortex grant");
        assert!(
            view.valid_action_ids.contains(&host_action),
            "host with Kabemon+Gomimon is an eligible app-fuse target"
        );
        r.execute_action(0, host_action).expect("pick host");
    }
    // 4. App-fuse selection #2: the hand result.
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
        "hand result app-fused onto the host after the Vortex/DP grant"
    );
}

/// DCGO guarded regions (P_241.cs): with NO [Appmon] Digimon on the field,
/// accepting still pays the suspend cost, the Vortex/+3000 buff pick is
/// skipped (DCGO wraps it in `if (HasMatchConditionPermanent(...))`), and the
/// app-fuse rider STILL runs — "Then, 1 of your Digimon may app fuse into a
/// Digimon card in the hand." is its own sentence and does not depend on the
/// buff having a target.
#[test]
fn p_241_no_appmon_buff_skipped_app_fuse_still_offered() {
    use digimon_engine::action::space::{encode_attack, HAND_EFFECT_START};

    // Deliberately NO Appmon trait — eligible app-fuse material by NAME only.
    fn plain_named(id: &str, name: &str) -> CardData {
        let mut c = make_test_card(id, name);
        c.card_kind = CardKind::Digimon;
        c.level = Some(4);
        c.dp = Some(4000);
        c.play_cost = 4;
        c
    }

    let mut r = base()
        .from_dsl_yaml(APP_FUSE_RESULT)
        .expect("APP_FUSE_RESULT compiles")
        .add_card(plain_named("PLAIN-KABEMON", "Kabemon"))
        .add_card(plain_named("PLAIN-GOMIMON", "Gomimon"))
        .deck(0, &["DECK-PAD"; 12])
        .hand(0, &["TEST-APPFUSE"])
        .memory(5)
        .start();
    let yujin = r.place_on_field(0, CARD_ID, Some(0));
    // Host is a NON-Appmon Kabemon — an app-fuse target once Gomimon links,
    // but NOT a valid Vortex/+3000 buff target.
    let host = r.place_on_field(0, "PLAIN-KABEMON", Some(0));
    r.game.enter_main_phase();

    let host_handle = r.perm_handle(0, host.index as usize);
    let host_action = encode_attack(0, host.index as u16);
    fire_link_event(&mut r, 0, host_handle, "PLAIN-GOMIMON");

    // Accept the outer optional prompt → suspend cost is paid.
    {
        let sel = r.game.pending_selection.as_ref().expect("optional prompt");
        let action = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(0, action);
        r.game.drain_effect_queue();
    }
    assert!(
        r.game.players[0].battle_area[yujin.index as usize].is_suspended,
        "Yujin suspends even with no [Appmon] Digimon on the field"
    );

    // No [Appmon] Digimon → the buff pick is skipped and the NEXT prompt is
    // the app-fuse permanent selection.
    {
        let view = r
            .pending_selection_view()
            .expect("app-fuse perm selection still installs with no [Appmon] on field");
        assert!(
            view.valid_action_ids.contains(&host_action),
            "host with Kabemon+Gomimon linked is an eligible app-fuse target"
        );
        r.execute_action(0, host_action).expect("pick host");
    }
    // App-fuse selection #2: the hand result.
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
        "app fuse still resolves when the buff pick had no targets"
    );
    // The buff never happened — no Vortex on the fused host.
    assert!(
        !r.game.has_keyword(host_handle, Keyword::Vortex),
        "no [Appmon] Digimon at buff time → the Vortex/+3000 grant is skipped"
    );
}
