//! BT3-093 Davis Motomiya — Tamer, Cost 4, Blue.
//! Traits: (none).
//!
//! # Card text (cards.json)
//!
//! ```text
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 blue and 1 green
//! Digimon card among them to your hand. Place the remaining cards at the
//! bottom of your deck in any order.
//!
//! Security Effect [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT3/Blue/BT3_093.cs
//!
//! # Patterns this test covers
//! - B1: Start-of-turn memory swing (memory_lte:2 → set_memory:3)
//! - B3: Tamer [On Play] reveal-3 + two color-bucket selection (blue Digimon,
//!       green Digimon) using select_reveal_buckets with color_is filter
//! - Security free-play (play_from_security)
//! - Condition gating: memory_lte gate on clause 1 (pos: memory=2, neg: memory=5)
//! - Reveal branches: both buckets hit / only blue hit / only green hit /
//!   neither hit

use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::action::space::SEL_REVEAL_START;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_blue_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![digimon_engine::enums::CardColor::Blue];
    c
}

fn make_green_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![digimon_engine::enums::CardColor::Green];
    c
}

fn make_red_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![digimon_engine::enums::CardColor::Red];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id)
                .then_some(SEL_REVEAL_START + idx as u16)
        })
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    runner
        .game
        .players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn deck_ids(runner: &DebugRunner) -> Vec<String> {
    runner
        .game
        .players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT3-093 must parse and compile to exactly 3 triggered clauses:
/// start_of_your_turn, on_play, on_security.
#[test]
fn bt3_093_has_exactly_three_triggered_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card("BT3-093").expect("BT3-093 compiled");
    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        3,
        "BT3-093 must have exactly 3 triggered clauses (start_of_your_turn, on_play, on_security)"
    );
}

/// Clause 1: start_of_your_turn, FaceUp scope, not optional, not OPT.
#[test]
fn bt3_093_clause1_start_of_your_turn_mandatory_face_up() {
    let runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card("BT3-093").expect("BT3-093 compiled");
    let clause1 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourTurn))
        .expect("start_of_your_turn clause must exist");

    assert_eq!(clause1.scope, CompiledScope::FaceUp, "clause 1 must be FaceUp scope");
    assert!(!clause1.optional, "clause 1 is mandatory — no 'you may' in printed text");
    assert!(!clause1.once_per_turn, "clause 1 is not once_per_turn");
}

/// Clause 2: on_play, FaceUp scope, not optional.
#[test]
fn bt3_093_clause2_on_play_mandatory_face_up() {
    let runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .memory(5)
        .start();

    let compiled = runner.compiled_card("BT3-093").expect("BT3-093 compiled");
    let clause2 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("on_play clause must exist");

    assert_eq!(clause2.scope, CompiledScope::FaceUp, "clause 2 must be FaceUp scope");
    assert!(!clause2.optional, "clause 2 is mandatory — no 'you may' gate");
    assert!(!clause2.once_per_turn, "clause 2 is not once_per_turn");
}

/// Clause 3: on_security, FaceUp scope, not optional.
#[test]
fn bt3_093_clause3_on_security_mandatory_face_up() {
    let runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner.compiled_card("BT3-093").expect("BT3-093 compiled");
    let clause3 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist");

    assert_eq!(clause3.scope, CompiledScope::FaceUp);
    assert!(!clause3.optional, "on_security is mandatory — [Security] plays are never optional");
    assert!(!clause3.once_per_turn, "on_security must not be once_per_turn");
}

/// Compiled card must be declared as a blue Tamer with cost 4.
#[test]
fn bt3_093_is_blue_tamer_cost_4() {
    use digimon_dsl::compiled::CompiledCardKind;

    let runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .start();
    let compiled = runner.compiled_card("BT3-093").expect("compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Tamer, "BT3-093 is a Tamer");
    assert_eq!(compiled.cost, Some(4), "BT3-093 cost is 4");
    assert!(
        compiled.color.contains(&CompiledColor::Blue),
        "BT3-093 must be blue"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating: Clause 1 (memory_lte:2)
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: memory is exactly 2 at start of P0's turn → condition passes →
/// memory is set to 3.
///
/// Strategy: place Davis on P0's field, set memory to 2 after placement
/// (avoids start-of-turn-at-memory-2 double-fire), cycle turns once, and
/// assert memory == 3 at P0's next start-of-turn.
#[test]
fn bt3_093_clause1_sets_memory_to_3_when_memory_is_2() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(5) // avoid immediate memory-2 fire on setup
        .start();

    runner.place_on_field(0, "BT3-093", Some(0));
    runner.game.memory = 2;

    runner.end_turn(); // P0 → P1 (memory flips to -2)
    runner.end_turn(); // P1 → P0 (memory flips to +2); StartOfYourTurn fires

    assert_eq!(
        runner.memory(),
        3,
        "memory must be set to 3 when it was 2 at start of P0's turn"
    );
}

/// Positive: memory is 0 → condition memory_lte:2 passes → set to 3.
#[test]
fn bt3_093_clause1_sets_memory_to_3_when_memory_is_0() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(5)
        .start();

    runner.place_on_field(0, "BT3-093", Some(0));
    runner.game.memory = 0;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(runner.memory(), 3, "memory must be set to 3 when it was 0");
}

/// Negative: memory is 5 at start of P0's turn → condition fails → memory
/// stays at 5.
#[test]
fn bt3_093_clause1_does_not_fire_when_memory_is_5() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(5)
        .start();

    runner.place_on_field(0, "BT3-093", Some(0));
    runner.game.memory = 5;

    runner.end_turn(); // memory flips to -5
    runner.end_turn(); // memory flips back to +5; StartOfYourTurn fires

    assert_eq!(
        runner.memory(),
        5,
        "memory must stay 5 when condition memory_lte:2 is false"
    );
}

/// Negative: memory is 3 → condition fails → memory stays at 3.
#[test]
fn bt3_093_clause1_does_not_fire_when_memory_is_3() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(3)
        .start();

    runner.place_on_field(0, "BT3-093", Some(0));
    runner.game.memory = 3;

    runner.end_turn(); // -3
    runner.end_turn(); // +3; StartOfYourTurn fires → condition: 3 > 2 → skip

    assert_eq!(
        runner.memory(),
        3,
        "memory must stay 3 when condition memory_lte:2 is false (3 > 2)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 2 On Play reveal-add buckets
// ═══════════════════════════════════════════════════════════════════════════════

/// Both buckets satisfied: reveal 3 = [BLUE-DG, GREEN-DG, FILLER].
/// Blue Digimon goes to hand, green Digimon goes to hand, filler bottoms.
#[test]
fn bt3_093_on_play_adds_blue_and_green_digimon_when_both_present() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .add_card(make_blue_digimon("BLUE-DG", "Veemon"))
        .add_card(make_green_digimon("GREEN-DG", "Guilmon"))
        .add_card(make_filler("FILL"))
        .deck(0, &["BLUE-DG", "GREEN-DG", "FILL"])
        .hand(0, &["BT3-093"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Davis Motomiya BT3-093");

    // First bucket: blue Digimon
    if let Some(view) = runner.pending_selection_view() {
        let blue_action = revealed_action_for_id(&runner, "BLUE-DG");
        if let Some(action) = blue_action {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick blue Digimon");
            }
        }
    }

    // Second bucket: green Digimon
    if let Some(view) = runner.pending_selection_view() {
        let green_action = revealed_action_for_id(&runner, "GREEN-DG");
        if let Some(action) = green_action {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick green Digimon");
            }
        }
    }

    runner.auto_resolve().expect("bottom remainder");

    let hand = hand_ids(&runner);
    assert!(
        hand.contains(&"BLUE-DG".to_string()),
        "blue Digimon must land in hand"
    );
    assert!(
        hand.contains(&"GREEN-DG".to_string()),
        "green Digimon must land in hand"
    );

    // Filler must be at the bottom of the deck.
    let deck = deck_ids(&runner);
    assert!(
        deck.last() == Some(&"FILL".to_string())
            || deck.iter().rev().take(1).any(|id| id == "FILL"),
        "unchosen filler must return to deck bottom; deck was {deck:?}"
    );
}

/// Only blue bucket satisfied: reveal 3 = [BLUE-DG, FILL1, FILL2].
/// Blue Digimon goes to hand; green bucket is empty (no green Digimon
/// revealed) → auto-resolved; both fillers bottom.
#[test]
fn bt3_093_on_play_only_blue_found_skips_green_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .add_card(make_blue_digimon("BLUE-DG", "Veemon"))
        .add_card(make_filler("FILL1"))
        .add_card(make_filler("FILL2"))
        .deck(0, &["BLUE-DG", "FILL1", "FILL2"])
        .hand(0, &["BT3-093"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Davis Motomiya BT3-093");

    // Pick blue Digimon from first bucket.
    while let Some(view) = runner.pending_selection_view() {
        let blue_action = revealed_action_for_id(&runner, "BLUE-DG");
        if let Some(action) = blue_action {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick blue Digimon");
                continue;
            }
        }
        // Advance with first legal action (auto-resolve empty bucket or bottom).
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand = hand_ids(&runner);
    assert!(
        hand.contains(&"BLUE-DG".to_string()),
        "blue Digimon must land in hand"
    );
    assert!(
        !hand.contains(&"FILL1".to_string()) && !hand.contains(&"FILL2".to_string()),
        "fillers must NOT enter hand"
    );
}

/// Only green bucket satisfied: reveal 3 = [GREEN-DG, FILL1, FILL2].
/// Green Digimon goes to hand; blue bucket is empty → skipped; fillers bottom.
#[test]
fn bt3_093_on_play_only_green_found_skips_blue_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .add_card(make_green_digimon("GREEN-DG", "Guilmon"))
        .add_card(make_filler("FILL1"))
        .add_card(make_filler("FILL2"))
        .deck(0, &["GREEN-DG", "FILL1", "FILL2"])
        .hand(0, &["BT3-093"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Davis Motomiya BT3-093");

    while let Some(view) = runner.pending_selection_view() {
        let green_action = revealed_action_for_id(&runner, "GREEN-DG");
        if let Some(action) = green_action {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick green Digimon");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand = hand_ids(&runner);
    assert!(
        hand.contains(&"GREEN-DG".to_string()),
        "green Digimon must land in hand"
    );
    assert!(
        !hand.contains(&"FILL1".to_string()) && !hand.contains(&"FILL2".to_string()),
        "fillers must NOT enter hand"
    );
}

/// Neither bucket has a candidate: reveal 3 = [RED-DG, FILL1, FILL2].
/// Red Digimon is neither blue nor green → both buckets empty → all 3 cards
/// return to deck bottom.
#[test]
fn bt3_093_on_play_no_matching_digimon_bottoms_all_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .add_card(make_red_digimon("RED-DG", "Agumon"))
        .add_card(make_filler("FILL1"))
        .add_card(make_filler("FILL2"))
        .deck(0, &["RED-DG", "FILL1", "FILL2"])
        .hand(0, &["BT3-093"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Davis Motomiya BT3-093");
    runner.auto_resolve().expect("resolve through empty buckets and bottom placement");

    let hand = hand_ids(&runner);
    for id in ["RED-DG", "FILL1", "FILL2"] {
        assert!(
            !hand.contains(&id.to_string()),
            "{id} must NOT enter hand when no bucket matches"
        );
    }
    let deck = deck_ids(&runner);
    assert_eq!(deck.len(), 3, "all 3 revealed cards must return to deck");
}

/// Color bucket filter: a Tamer card colored blue must NOT satisfy the
/// blue Digimon bucket (kind:digimon is required).
#[test]
fn bt3_093_on_play_blue_tamer_rejected_by_blue_digimon_bucket() {
    let mut blue_tamer = make_test_card("BLUE-TAMER", "Ken Ichijoji");
    blue_tamer.card_kind = CardKind::Tamer;
    blue_tamer.colors = vec![digimon_engine::enums::CardColor::Blue];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .add_card(blue_tamer)
        .add_card(make_filler("FILL1"))
        .add_card(make_filler("FILL2"))
        .deck(0, &["BLUE-TAMER", "FILL1", "FILL2"])
        .hand(0, &["BT3-093"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play BT3-093");
    runner.auto_resolve().expect("both buckets empty — no Digimon present");

    let hand = hand_ids(&runner);
    assert!(
        !hand.contains(&"BLUE-TAMER".to_string()),
        "blue Tamer must be rejected by the blue-Digimon bucket (kind:digimon required)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Behavioral: Clause 3 Security free-play
// ═══════════════════════════════════════════════════════════════════════════════

/// [Security] clause: when Davis Motomiya is in P1's security stack and P0
/// attacks P1's player, Davis is played onto P1's battle area without cost.
#[test]
fn bt3_093_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTKR", "Attacker");
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT3-093")
        .expect("BT3-093 in embedded DSL pack")
        .add_card(attacker)
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .security(1, &["BT3-093"])
        .memory(10)
        .start();

    let attacker_perm = runner.place_on_field(0, "ATTKR", Some(0));

    // P0 attacks P1's player → triggers security check on BT3-093.
    runner.attack_player(attacker_perm, 1, false);
    runner.auto_resolve().expect("resolve security play");

    // Davis must be played (without paying cost 4) onto P1's battle area.
    assert!(
        runner
            .game
            .players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT3-093"),
        "BT3-093 must be played onto P1's battle area from security without paying cost"
    );
}
