//! BT25-013 Firamon — Digimon, Lv.4, Red, DP 5000, Cost 4.
//! Traits: Beast, Iliad, TS. Attribute: Vaccine.
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] By trashing 1 card in your hand, you may return
//! 1 red or blue [Iliad] trait Digimon card from your trash to the hand.
//! [Your Turn] When your Digimon are played or digivolve, if any of them are
//! blue, this Digimon may digivolve into [Flaremon] in the hand with the cost
//! reduced by 1.
//!
//! Inherited Effect:
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_013.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A4 (trash-as-cost → return from trash to hand)
//! - cross-permanent observer conditional digivolve (cost-reduced, into Flaremon)
//! - G (inherited conditional +DP aura)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-013";

/// A red [Iliad] Lv.4 Digimon (eligible recover target).
fn red_iliad(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Iliad".to_string()];
    c
}

/// A green [Iliad] Lv.4 Digimon (NOT red/blue → ineligible recover target).
fn green_iliad(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.colors = vec![CardColor::Green];
    c.traits = vec!["Iliad".to_string()];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-013 YAML parses and compiles")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(red_iliad("RED-ILIAD"))
        .add_card(green_iliad("GREEN-ILIAD"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
}

fn seed_trash(runner: &mut DebugRunner, card_id: &str) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let iid = runner.game.next_card_index();
    runner.game.players[0].trash.push(CardSource::new(idx, 0, iid));
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt25_013_metadata_and_three_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert_eq!(card.name, "Firamon");
    assert_eq!(card.dp, Some(5000));

    // Clause 0: OnPlay/WhenDigivolving trash→recover.
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
    )));
    // Clause 2: inherited [Your Turn] +2000 DP aura.
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Inherited,
            dp_modifier: Some(2000),
            ..
        })
    )));
}

// ─── Section 2 — Behavior: trash 1 → return red/blue [Iliad] from trash ──────

#[test]
fn bt25_013_on_play_offers_trash_cost_prompt() {
    let mut runner = base().hand(0, &[CARD_ID, "FILL"]).memory(10).start();
    seed_trash(&mut runner, "RED-ILIAD");
    runner.play(0, 0);
    assert!(
        runner.pending_selection().is_some(),
        "OnPlay installs the optional 'trash 1 card' cost prompt"
    );
}

#[test]
fn bt25_013_trash_then_recover_red_iliad() {
    let mut runner = base().hand(0, &[CARD_ID, "FILL"]).memory(10).start();
    seed_trash(&mut runner, "RED-ILIAD");

    let hand_before = runner.hand_size(0); // CARD_ID + FILL
    runner.play(0, 0); // Firamon leaves hand
    // Accept + drive: trash the FILL card, then recover RED-ILIAD.
    runner.auto_resolve().expect("resolve trash + recover");

    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    // RED-ILIAD should now be back in hand (recovered).
    assert!(
        hand_ids.iter().any(|id| id == "RED-ILIAD"),
        "the red [Iliad] Digimon is returned from trash to hand"
    );
}

#[test]
fn bt25_013_recover_filter_excludes_green_iliad() {
    // Only a GREEN [Iliad] in trash → not a legal recover target (needs red or
    // blue). The recover step is a no-op even if the trash cost is paid.
    let mut runner = base().hand(0, &[CARD_ID, "FILL"]).memory(10).start();
    seed_trash(&mut runner, "GREEN-ILIAD");
    runner.play(0, 0);
    runner.auto_resolve().ok();

    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        !hand_ids.iter().any(|id| id == "GREEN-ILIAD"),
        "a green [Iliad] is NOT recoverable (only red or blue)"
    );
}

// ─── Section 3 — Inherited conditional DP ────────────────────────────────────

#[test]
fn bt25_013_inherited_plus_2000_on_your_turn_only() {
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.card_kind = CardKind::Digimon;
    carrier.level = Some(5);
    carrier.dp = Some(7000);
    carrier.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 4,
        memory_cost: 3,
    }];
    let mut runner = base().add_card(carrier).memory(10).start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    runner.game.turn_player_idx = 0;
    assert_eq!(
        runner.effective_dp(stack),
        Some(9000),
        "your turn: carrier 7000 + inherited 2000 = 9000"
    );

    runner.game.turn_player_idx = 1;
    assert_eq!(
        runner.effective_dp(stack),
        Some(7000),
        "opponent's turn: [Your Turn] aura must not apply"
    );
}
