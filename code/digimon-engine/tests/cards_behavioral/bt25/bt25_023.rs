//! BT25-023 Gaogamon — Digimon, Lv.4, Blue, DP 6000, Cost 5.
//! Traits: Beast, DATA SQUAD. Attribute: Data.
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] If you have 1 or fewer Tamers, you may play 1
//! [Thomas H. Norstein] from your hand without paying the cost.
//!
//! Inherited Effect:
//! [When Attacking] [Once Per Turn] Both players ＜Draw 1＞.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_023.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - B2-adjacent (conditional play-from-hand free, gated on Tamer count)
//! - E2 (optional "you may")
//! - G4-adjacent (inherited When Attacking OPT both-players draw)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-023";

fn thomas() -> CardData {
    let mut c = make_test_card("THOMAS", "Thomas H. Norstein");
    c.card_kind = CardKind::Tamer;
    c
}

fn other_tamer() -> CardData {
    let mut c = make_test_card("OTHER-TAMER", "Other Tamer");
    c.card_kind = CardKind::Tamer;
    c
}

fn carrier_lv5() -> CardData {
    let mut c = make_test_card("CARRIER5", "Carrier5");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.evo_costs = vec![EvoCost {
        card_color: 1,
        level: 4,
        memory_cost: 3,
    }];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-023 YAML parses and compiles")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(thomas())
        .add_card(other_tamer())
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt25_023_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert_eq!(card.name, "Gaogamon");
    assert_eq!(card.dp, Some(6000));
    assert!(card.traits.contains(&"DATA SQUAD".to_string()));
}

#[test]
fn bt25_023_has_op_wd_optional_clause_and_inherited_opt() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");

    let op = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OnPlay/WhenDigivolving clause present");
    assert!(op.optional, "the 'you may play' clause is optional");

    let inh = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited When Attacking clause present");
    assert!(inh.once_per_turn, "inherited clause is [Once Per Turn]");
}

// ─── Section 2 — Behavior: conditional play Thomas free ──────────────────────

#[test]
fn bt25_023_offers_play_when_zero_tamers() {
    let mut runner = base().hand(0, &[CARD_ID, "THOMAS"]).memory(10).start();
    runner.play(0, 0); // play Gaogamon; 0 Tamers on field → clause active
    assert!(
        runner.pending_selection().is_some(),
        "with <=1 Tamers and Thomas in hand, the optional play prompt installs"
    );
}

#[test]
fn bt25_023_plays_thomas_for_free() {
    let mut runner = base().hand(0, &[CARD_ID, "THOMAS"]).memory(10).start();
    let tamers_before = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_id(&runner.game.card_data) == "THOMAS")
        .count();
    runner.play(0, 0);
    let memory_after_play = runner.memory();
    runner.accept_optional_trigger().ok();
    runner.auto_resolve().expect("resolve play-Thomas-free");

    let thomas_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "THOMAS");
    assert!(thomas_on_field, "Thomas H. Norstein is played to the field");
    // Free play: memory unchanged by the Thomas play itself.
    assert_eq!(
        runner.memory(),
        memory_after_play,
        "playing Thomas via the effect costs no memory"
    );
}

#[test]
fn bt25_023_gated_out_when_two_or_more_tamers() {
    // Two Tamers already on field → "1 or fewer Tamers" condition fails.
    let mut runner = base().hand(0, &[CARD_ID, "THOMAS"]).memory(10).start();
    runner.place_on_field(0, "THOMAS", Some(0));
    runner.place_on_field(0, "OTHER-TAMER", Some(0));
    // Need a fresh Thomas to play — but the gate should block regardless.
    runner.play(0, 0);
    assert!(
        runner.pending_selection().is_none(),
        ">=2 Tamers → the play clause is gated out"
    );
}

// ─── Section 3 — Behavior: inherited both-players draw ───────────────────────

#[test]
fn bt25_023_inherited_both_players_draw_on_attack() {
    let mut runner = base().add_card(carrier_lv5()).memory(10).start();
    let attacker = runner.place_stack(0, &[CARD_ID, "CARRIER5"]);
    runner.game.turn_player_idx = 0;

    let p0_before = runner.deck_size(0);
    let p1_before = runner.deck_size(1);
    runner.attack_player(attacker, 1, false);
    if runner.pending_selection().is_some() {
        let _ = runner.accept_optional_trigger();
        let _ = runner.auto_resolve();
    }
    assert_eq!(runner.deck_size(0), p0_before - 1, "controller draws 1");
    assert_eq!(runner.deck_size(1), p1_before - 1, "opponent draws 1");
}
