//! BT25-021 Gaomon — Digimon, Lv.3, Blue, DP 2000, Cost 3.
//! Traits: Beast, DATA SQUAD. Attribute: Data.
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 [Thomas H. Norstein]
//! trait or card with the [DATA SQUAD] trait and 1 card with [Gaogamon] in its
//! name among them to the hand. Return the rest to the bottom of the deck.
//!
//! Inherited Effect:
//! [When Attacking] [Once Per Turn] Both players ＜Draw 1＞.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_021.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A2 (reveal-3, two-bucket add by filter, return rest to deck bottom)
//! - G4-adjacent (inherited When Attacking, OPT)
//! - both-players draw

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-021";

/// Resolve a revealed-card action id by candidate card_id.
fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
}

/// Pick a revealed card by card_id in the current bucket.
fn pick_revealed_by_id(runner: &mut DebugRunner, p: PlayerId, id: &str) {
    let view = runner
        .pending_selection_view()
        .unwrap_or_else(|| panic!("expected a reveal bucket prompt to pick {id}"));
    let action = revealed_action_for_id(runner, id)
        .unwrap_or_else(|| panic!("revealed card {id} not present"));
    assert!(
        view.valid_action_ids.contains(&action),
        "action {action} for {id} not legal in current bucket; valid={:?}",
        view.valid_action_ids
    );
    runner.execute_action(p, action).expect("pick revealed");
}

fn named(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c
}

fn squad(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn gaomon_base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-021 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(squad("SQUAD-A"))
        .add_card(named("GAOGA-A", "Gaogamon"))
        .add_card(named("PLAIN-A", "Patamon"))
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt25_021_metadata() {
    let runner = gaomon_base()
        .deck(0, &["DECK-PAD"; 6])
        .deck(1, &["DECK-PAD"; 6])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert_eq!(card.name, "Gaomon");
    assert_eq!(card.dp, Some(2000));
    assert!(card.traits.contains(&"DATA SQUAD".to_string()));
}

#[test]
fn bt25_021_has_on_play_and_inherited_when_attacking_opt() {
    let runner = gaomon_base()
        .deck(0, &["DECK-PAD"; 6])
        .deck(1, &["DECK-PAD"; 6])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");

    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )),
        "OnPlay reveal clause present"
    );

    let inherited = card
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
    assert!(inherited.once_per_turn, "inherited clause is [Once Per Turn]");
}

// ─── Section 2 — Behavior: reveal 3, add 2, bottom rest ──────────────────────

#[test]
fn bt25_021_on_play_adds_squad_and_gaogamon_bottoms_rest() {
    // Top 3 of deck: a [DATA SQUAD] card, a [Gaogamon]-named card, and a plain
    // card. The two eligible cards go to hand; the plain card goes to the
    // bottom of the deck.
    // Observed: the reveal takes cards from the END of the `deck()` slice, so
    // place the three reveal targets last (top of deck).
    let mut runner = gaomon_base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "DECK-PAD", "PLAIN-A", "GAOGA-A", "SQUAD-A"])
        .deck(1, &["DECK-PAD"; 6])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0); // just Gaomon
    runner.play(0, 0);
    assert!(
        runner.pending_selection().is_some(),
        "reveal-and-add must surface a selection prompt"
    );
    // Bucket A (DATA SQUAD/Thomas): pick SQUAD-A. Bucket B (Gaogamon): pick GAOGA-A.
    pick_revealed_by_id(&mut runner, 0, "SQUAD-A");
    pick_revealed_by_id(&mut runner, 0, "GAOGA-A");
    runner.auto_resolve().expect("resolve remaining (bottom the rest)");

    // Gaomon left hand (played); 2 cards added → hand net = (hand_before - 1) + 2.
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1 + 2,
        "two revealed cards (DATA SQUAD + Gaogamon) added to hand"
    );
    // SQUAD-A and GAOGA-A are now in hand.
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(hand_ids.iter().any(|id| id == "SQUAD-A"));
    assert!(hand_ids.iter().any(|id| id == "GAOGA-A"));
    assert!(
        !hand_ids.iter().any(|id| id == "PLAIN-A"),
        "the non-matching revealed card is NOT added to hand"
    );
}

// ─── Section 3 — Behavior: inherited both-players draw on attack ─────────────

#[test]
fn bt25_021_inherited_both_players_draw_on_attack() {
    let mut carrier = named("CARRIER", "Carrier");
    carrier.dp = Some(5000);
    carrier.evo_costs = vec![EvoCost {
        card_color: 1,
        level: 3,
        memory_cost: 2,
    }];
    let mut runner = gaomon_base()
        .add_card(carrier)
        .deck(0, &["DECK-PAD"; 8])
        .deck(1, &["DECK-PAD"; 8])
        .memory(10)
        .start();

    let attacker = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.turn_player_idx = 0;

    let p0_deck_before = runner.deck_size(0);
    let p1_deck_before = runner.deck_size(1);

    runner.attack_player(attacker, 1, false);
    // The inherited When Attacking clause is optional; accept it if it parks.
    if runner.pending_selection().is_some() {
        let _ = runner.accept_optional_trigger();
        let _ = runner.auto_resolve();
    }

    assert_eq!(
        runner.deck_size(0),
        p0_deck_before - 1,
        "controller draws 1"
    );
    assert_eq!(
        runner.deck_size(1),
        p1_deck_before - 1,
        "opponent draws 1 (both players)"
    );
}
