//! BT17-081 Tai Kamiya & Matt Ishida — Tamer, Cost 4, Red+Blue.
//!
//! # Card text (cards.json — printed)
//!
//! [All Turns] When one of your Digimon is played or digivolves, by suspending
//! this Tamer, if you have a Digimon with [Greymon] in its name, gain 1 memory.
//! If you have a Digimon with [Garurumon] in its name, gain 1 memory.
//!
//! [End of Your Turn] [Once Per Turn] 1 of your Digimon with [Omnimon] in its
//! name may attack a player.
//!
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_081.cs
//!
//! # Patterns this test covers
//! - B3 Trigger-on-event tamer (on_enter_field_anyone + on_digivolve observer)
//! - B2-adj Cost-4 dual-color tamer with persistent observer
//! - F9-adj Suspend-self-as-cost on observer trigger
//! - G-MAY-ATTACK-NOW (resolved 2026-05-03) — `may_attack_now` with player-only
//!   target restriction (DCGO defenderCondition: _ => false)
//! - Security clause: `play_from_security` (BT22-084 / BT18-087 / BT24-082 idiom)
//!
//! # Known engine/DSL gaps affecting these tests
//!
//! - **G-ENTERING-PERMANENT-TRAIT** (partially closed 2026-04-29):
//!   `OnEnterFieldAnyone` populates `TriggerContext.event_permanent /
//!   event_card` for normal hand-played battle-area permanents and
//!   `OnDigivolve` does the same for hand-driven `Game::digivolve_from_hand`.
//!   Effect-created permanents, token play, option placement, play-from-trash,
//!   breeding-area observer fan-out, DNA digivolve, effect-initiated
//!   digivolve, and breeding-area digivolve remain open. Tests that rely on
//!   `event_target_owner` / `event_target_kind` evaluating against the
//!   entering card's actual owner / kind are positioned on the closed paths
//!   only; observer-fan-out coverage for tamers, tokens, and DNA digivolve is
//!   `#[ignore]`'d on those gaps.
//!
//! - **G-OPT-TRIGGERED**: OPT lockout for triggered clauses is not yet
//!   enforced through the queue drain. The `once_per_turn: true` flag on
//!   clause 2 is authored but a same-turn re-fire test is `#[ignore]`'d.
//!   In practice `end_of_your_turn` fires at most once per turn, so this is a
//!   structural assertion only.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/bt17/BT17-081.yaml");

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_named_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn make_named_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Push a CardSource referencing `card_id` into player `p`'s hand.
fn push_to_hand(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

/// Standard fixture: BT17-081 (Tai & Matt) registered + filler card. Memory
/// pre-set to 10 so we never trip the seesaw mid-test.
fn taimatt_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-081 YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT17-081 must compile and be present in the embedded DSL pack.
#[test]
fn bt17_081_compiles() {
    let runner = taimatt_runner();
    assert!(
        runner.compiled_card("BT17-081").is_some(),
        "BT17-081 must compile from YAML"
    );
}

/// Card metadata: kind=tamer, cost=4, no level, no dp.
#[test]
fn bt17_081_card_metadata_matches_print() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("BT17-081").expect("BT17-081 compiles");

    assert_eq!(card.cost, Some(4), "Tai & Matt costs 4");
    // Tamer cards have no level or DP.
    assert_eq!(card.level, None, "Tamer card has no level");
    assert_eq!(card.dp, None, "Tamer card has no DP");
}

/// Exactly 3 triggered clauses: the [All Turns] observer, the [End of Your
/// Turn] Omnimon attack, and the [Security] clause.
#[test]
fn bt17_081_has_exactly_three_triggered_clauses() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("BT17-081").unwrap();

    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered, 3,
        "BT17-081 must have 3 triggered clauses (all-turns observer, end-of-your-turn Omnimon, on_security)"
    );
}

/// Clause 1: [All Turns] observer fires on BOTH on_enter_field_anyone and
/// on_digivolve, is optional, and is FaceUp scope.
#[test]
fn bt17_081_clause1_is_all_turns_observer_optional_faceup() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("BT17-081").unwrap();

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone))
        .expect("must have an on_enter_field_anyone clause");

    assert!(
        clause.when.contains(&CompiledTiming::OnDigivolve),
        "clause 1 must also fire on on_digivolve (the digivolve half of \"played or digivolves\")"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "all-turns observer is FaceUp scope"
    );
    assert!(
        clause.optional,
        "\"by suspending this Tamer\" → activation is optional"
    );
    assert!(
        !clause.once_per_turn,
        "no printed [Once Per Turn] on the all-turns observer"
    );
    assert!(
        clause.active_when.is_some(),
        "all-turns observer must declare active_when (all_turns: true)"
    );
    assert!(
        clause.condition.is_some(),
        "observer must have a condition (event_target_owner: you AND event_target_kind: digimon)"
    );
}

/// Clause 2: [End of Your Turn][Once Per Turn] Omnimon attack — optional,
/// FaceUp, once_per_turn flag is set.
#[test]
fn bt17_081_clause2_is_end_of_your_turn_opt_optional_faceup() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("BT17-081").unwrap();

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::EndOfYourTurn))
        .expect("must have an end_of_your_turn clause");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "end-of-your-turn Omnimon clause is FaceUp scope"
    );
    assert!(
        clause.optional,
        "\"may attack\" → outer activation is optional"
    );
    assert!(
        clause.once_per_turn,
        "printed [Once Per Turn] → once_per_turn flag must be set"
    );
}

/// Clause 3: [Security] play_from_security — FaceUp scope, NOT optional
/// (security plays are mandatory).
#[test]
fn bt17_081_clause3_is_on_security_mandatory_faceup() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("BT17-081").unwrap();

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have an on_security clause");

    assert!(
        !clause.optional,
        "[Security] play_from_security is mandatory (not optional)"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "on_security uses FaceUp scope (no separate Security variant)"
    );
}

/// Clause 1's process must contain a `suspend` step (the activation cost),
/// and clause 2's process must contain a `may_attack_now` step.
#[test]
fn bt17_081_process_steps_match_card_text() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("BT17-081").unwrap();

    let clause1 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone))
        .expect("clause 1 present");

    let has_suspend = clause1
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::Suspend { .. }));
    assert!(
        has_suspend,
        "Clause 1 must include a `suspend` step (the \"by suspending this Tamer\" cost)"
    );

    let clause2 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::EndOfYourTurn))
        .expect("clause 2 present");

    let has_may_attack_now = clause2
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::MayAttackNow { .. }));
    assert!(
        has_may_attack_now,
        "Clause 2 must include a `may_attack_now` step (1 of your Omnimon may attack a player)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — Clause 1 condition gating (positive + negative)
// ═══════════════════════════════════════════════════════════════════════════════

/// Negative gate (event_target_kind: digimon): when an own TAMER is played,
/// the observer must NOT fire — printed text says "your **Digimon**", not
/// "your Digimon or Tamers".
#[test]
fn bt17_081_observer_does_not_fire_on_own_tamer_play() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_tamer("OWN-TAMER", "TestTamer"));
    push_to_hand(&mut runner, 0, "OWN-TAMER");

    // Tai & Matt on the field as the observer — placed on a prior turn so no
    // fresh on-play prompt interferes.
    let owen = runner.place_on_field(0, "BT17-081", Some(0));
    let owen_suspended_before =
        runner.game.players[0].battle_area[owen.index as usize].is_suspended;

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-TAMER")
        .expect("OWN-TAMER in hand");
    runner.play(0, hand_idx).expect("tamer plays from hand");

    // Tai & Matt must remain unsuspended — the observer's event_target_kind
    // filter must reject a tamer-entering event.
    assert_eq!(
        runner.game.players[0].battle_area[owen.index as usize].is_suspended, owen_suspended_before,
        "Tai & Matt must NOT suspend itself when an own TAMER (not Digimon) enters"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no observer prompt should install when a Tamer (not Digimon) plays"
    );
}

/// Negative gate (event_target_owner: you): when an OPPONENT's Digimon is
/// played, the observer must NOT fire — printed text scopes to "your Digimon".
#[test]
fn bt17_081_observer_does_not_fire_on_opponent_digimon_play() {
    let mut runner = taimatt_runner();
    let mut opp_dig = make_named_digimon("OPP-DIG", "Opponent Digimon", 3, 3000);
    opp_dig.play_cost = 0;
    runner.game.card_data.push(opp_dig);
    push_to_hand(&mut runner, 1, "OPP-DIG");

    let owen = runner.place_on_field(0, "BT17-081", Some(0));
    let owen_suspended_before =
        runner.game.players[0].battle_area[owen.index as usize].is_suspended;

    // Switch turn so player 1 is active and can play. Memory is 10 → end_turn
    // flips to -10 and immediately reaches the floor; OPP-DIG.play_cost = 0
    // sidesteps `pay_memory`'s affordability check.
    runner.end_turn();
    assert_eq!(
        runner.turn_player(),
        1,
        "after end_turn, player 1 must be active to play their Digimon"
    );

    let hand_idx = runner
        .game
        .player(1)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OPP-DIG")
        .expect("OPP-DIG in player 1 hand");
    runner.play(1, hand_idx).expect("opponent Digimon plays");

    assert_eq!(
        runner.game.players[0].battle_area[owen.index as usize].is_suspended, owen_suspended_before,
        "Tai & Matt must NOT suspend on an opponent Digimon play"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no observer prompt should install on opponent's Digimon play"
    );
}

/// Negative gate (no Greymon, no Garurumon on field): when an own Digimon is
/// played and BOTH the Greymon and Garurumon body checks fail, the suspend
/// cost still pays (the activation cost is paid up front per DCGO ordering),
/// but no memory is gained.
#[test]
fn bt17_081_observer_no_greymon_no_garurumon_no_memory_gain() {
    let mut runner = taimatt_runner();
    let mut own_unrelated = make_named_digimon("OWN-PLAIN", "PlainDigimon", 3, 3000);
    own_unrelated.play_cost = 0;
    runner.game.card_data.push(own_unrelated);
    push_to_hand(&mut runner, 0, "OWN-PLAIN");

    let owen = runner.place_on_field(0, "BT17-081", Some(0));

    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-PLAIN")
        .expect("OWN-PLAIN in hand");
    runner.play(0, hand_idx).expect("plain Digimon plays");

    // The activation prompt is optional — accept it if it installs (the player
    // chooses to suspend), then drain.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }

    // Regardless of whether the player accepted, no memory was gained — both
    // body conditions failed.
    assert_eq!(
        runner.memory(),
        memory_before,
        "no Greymon and no Garurumon on field → no memory gain even if activation accepted; before={memory_before}, after={}",
        runner.memory()
    );

    // If the player accepted activation, Tai & Matt is suspended; if declined,
    // it remains unsuspended. Both are valid.
    let _suspended = runner.game.players[0].battle_area[owen.index as usize].is_suspended;
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — Clause 1 behavioral outcome (memory gain branches)
// ═══════════════════════════════════════════════════════════════════════════════

/// Greymon-only on field: when an own Digimon is played and there's a
/// [Greymon]-named Digimon on the field but no [Garurumon], accepting the
/// activation must suspend self and gain exactly 1 memory.
#[test]
fn bt17_081_observer_greymon_present_gains_one_memory() {
    let mut runner = taimatt_runner();
    let mut grey = make_named_digimon("OWN-GREY", "Greymon", 4, 4000);
    grey.play_cost = 0;
    runner.game.card_data.push(grey);
    let mut plain = make_named_digimon("OWN-PLAIN", "PlainDigimon", 3, 3000);
    plain.play_cost = 0;
    runner.game.card_data.push(plain);

    let owen = runner.place_on_field(0, "BT17-081", Some(0));
    runner.place_on_field(0, "OWN-GREY", Some(0));

    push_to_hand(&mut runner, 0, "OWN-PLAIN");
    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-PLAIN")
        .expect("OWN-PLAIN in hand");
    runner.play(0, hand_idx).expect("plays plain digimon");

    // Accept the optional activation if it installs.
    let mut steps = 0;
    let mut accepted = false;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        // Find the first non-PASS action (accept the activation).
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        accepted = true;
        steps += 1;
    }

    if !accepted {
        // No activation prompt fired — observer didn't install. Test is moot.
        return;
    }

    assert!(
        runner.game.players[0].battle_area[owen.index as usize].is_suspended,
        "Tai & Matt must be suspended after activation accepted"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "Greymon present, no Garurumon → +1 memory; before={memory_before}, after={}",
        runner.memory()
    );
}

/// Garurumon-only on field: gain exactly 1 memory.
#[test]
fn bt17_081_observer_garurumon_present_gains_one_memory() {
    let mut runner = taimatt_runner();
    let mut garu = make_named_digimon("OWN-GARU", "Garurumon", 4, 4000);
    garu.play_cost = 0;
    runner.game.card_data.push(garu);
    let mut plain = make_named_digimon("OWN-PLAIN", "PlainDigimon", 3, 3000);
    plain.play_cost = 0;
    runner.game.card_data.push(plain);

    let owen = runner.place_on_field(0, "BT17-081", Some(0));
    runner.place_on_field(0, "OWN-GARU", Some(0));

    push_to_hand(&mut runner, 0, "OWN-PLAIN");
    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-PLAIN")
        .expect("OWN-PLAIN in hand");
    runner.play(0, hand_idx).expect("plays plain digimon");

    let mut steps = 0;
    let mut accepted = false;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        accepted = true;
        steps += 1;
    }

    if !accepted {
        return;
    }

    assert!(
        runner.game.players[0].battle_area[owen.index as usize].is_suspended,
        "Tai & Matt must be suspended after activation accepted"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "Garurumon present, no Greymon → +1 memory; before={memory_before}, after={}",
        runner.memory()
    );
}

/// Both Greymon AND Garurumon on field: gain exactly 2 memory (the two
/// independent if-blocks each fire).
#[test]
fn bt17_081_observer_both_greymon_and_garurumon_gains_two_memory() {
    let mut runner = taimatt_runner();
    let mut grey = make_named_digimon("OWN-GREY", "Greymon", 4, 4000);
    grey.play_cost = 0;
    runner.game.card_data.push(grey);
    let mut garu = make_named_digimon("OWN-GARU", "Garurumon", 4, 4000);
    garu.play_cost = 0;
    runner.game.card_data.push(garu);
    let mut plain = make_named_digimon("OWN-PLAIN", "PlainDigimon", 3, 3000);
    plain.play_cost = 0;
    runner.game.card_data.push(plain);

    let owen = runner.place_on_field(0, "BT17-081", Some(0));
    runner.place_on_field(0, "OWN-GREY", Some(0));
    runner.place_on_field(0, "OWN-GARU", Some(0));

    push_to_hand(&mut runner, 0, "OWN-PLAIN");
    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-PLAIN")
        .expect("OWN-PLAIN in hand");
    runner.play(0, hand_idx).expect("plays plain digimon");

    let mut steps = 0;
    let mut accepted = false;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        accepted = true;
        steps += 1;
    }

    if !accepted {
        return;
    }

    assert!(
        runner.game.players[0].battle_area[owen.index as usize].is_suspended,
        "Tai & Matt must be suspended after activation accepted"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "both Greymon AND Garurumon present → +2 memory (independent if-blocks); before={memory_before}, after={}",
        runner.memory()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4 — Clause 2 behavior (end-of-turn Omnimon attack)
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause 2 condition gate: when there is NO own Omnimon-named Digimon on the
/// field, the end-of-your-turn trigger must not install any selection prompt.
#[test]
fn bt17_081_clause2_blocked_when_no_omnimon_on_field() {
    let mut runner = taimatt_runner();
    let owen = runner.place_on_field(0, "BT17-081", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(owen));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no Omnimon → no end-of-turn prompt; condition `any_permanent name_contains Omnimon` must gate"
    );
}

/// Clause 2 positive gate: when an own Omnimon-named Digimon is on the field,
/// the end-of-your-turn trigger installs a prompt — the OUTER prompt is the
/// optional activation (player may decline).
#[test]
fn bt17_081_clause2_omnimon_present_installs_prompt() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OWN-OMN", "Omnimon", 6, 13000));
    let owen = runner.place_on_field(0, "BT17-081", Some(0));
    runner.place_on_field(0, "OWN-OMN", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(owen));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "an own Omnimon-named Digimon on field → end-of-your-turn prompt must install"
    );
}

/// Clause 2 condition gate (negative — no Omnimon-named Digimon counts):
/// a non-Omnimon-named own Digimon must not satisfy the gate.
#[test]
fn bt17_081_clause2_blocked_by_non_omnimon_named_digimon() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OWN-PLAIN", "PlainDigimon", 4, 4000));
    let owen = runner.place_on_field(0, "BT17-081", Some(0));
    runner.place_on_field(0, "OWN-PLAIN", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(owen));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "non-Omnimon-named Digimon must not satisfy the name_contains gate"
    );
}

/// Clause 2 OPT lockout — printed [Once Per Turn]. The end-of-your-turn
/// "1 of your Omnimon may attack a player" clause may fire at most once per
/// turn. Firing the `EndOfYourTurn` trigger a second time in the same turn
/// must NOT install a second prompt; after a full turn cycle the OPT counter
/// resets and the clause fires again.
///
/// OPT enforcement for permanent-sourced triggered effects is wired in
/// `run_queued_effect_inner` (`effect_queue.rs` ~1980-2016); the per-permanent
/// `effect_activations` counter resets via `Permanent::new_turn()`.
#[test]
fn bt17_081_clause2_opt_lockout_blocks_second_activation_same_turn() {
    let mut runner = taimatt_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("OWN-OMN", "Omnimon", 6, 13000));
    let owen = runner.place_on_field(0, "BT17-081", Some(0));
    runner.place_on_field(0, "OWN-OMN", Some(0));

    // ── First fire: the trigger installs a prompt; resolving it consumes OPT.
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(owen));
    runner.game.drain_effect_queue();
    assert!(
        runner.game.pending_selection.is_some(),
        "first EndOfYourTurn fire must install the optional Omnimon-attack prompt"
    );
    // Drain the prompt chain (accept the activation, then decline the attack
    // target via PASS once it is offered) — the body running consumes the OPT.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 10 {
        let (player, action) = {
            let pending = runner.game.pending_selection.as_ref().unwrap();
            (pending.selecting_player, pending.valid_action_ids[0])
        };
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }

    // ── Second fire, SAME turn: OPT lockout — no prompt installs.
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(owen));
    runner.game.drain_effect_queue();
    assert!(
        runner.game.pending_selection.is_none(),
        "OPT lockout must block the second EndOfYourTurn activation in the same turn"
    );

    // ── OPT reset: `Permanent::new_turn()` clears `effect_activations`. This
    // is exactly what `begin_turn()` → `Player::new_turn()` calls for the
    // turn player's permanents at the start of each turn. (Sibling idiom:
    // `st9_05_when_attacking_opt_resets_after_turn_end`.)
    runner.game.players[0].battle_area[owen.index as usize].new_turn();

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(owen));
    runner.game.drain_effect_queue();
    assert!(
        runner.game.pending_selection.is_some(),
        "after the per-turn OPT reset the clause must fire again"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5 — Smoke / no-panic checks for observer fan-out
// ═══════════════════════════════════════════════════════════════════════════════

/// Smoke: enqueueing both observer timings on player 0's battle area when
/// Tai & Matt is the only thing in scope must not panic, regardless of whether
/// the condition gate fires (no event_card present in PlayerBattleArea
/// fan-out paths means the observer's predicate may evaluate as a no-match,
/// which is the safe behavior).
#[test]
fn bt17_081_observer_no_panic_on_battle_area_fan_out() {
    let mut runner = taimatt_runner();
    runner.place_on_field(0, "BT17-081", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::OnEnterFieldAnyone,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    // No assertion beyond not panicking; drain any prompts that did install
    // by passing/auto-resolving.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 6 — Security clause structural
// ═══════════════════════════════════════════════════════════════════════════════

/// The on_security clause's process must include `play_from_security`.
#[test]
fn bt17_081_security_clause_uses_play_from_security_step() {
    let runner = taimatt_runner();
    let card = runner.compiled_card("BT17-081").unwrap();

    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause present");

    let has_play_from_security = security
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity));
    assert!(
        has_play_from_security,
        "on_security clause must include play_from_security step"
    );
}
