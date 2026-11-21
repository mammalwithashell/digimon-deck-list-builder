//! BT24-069 Vilemon - Digimon, Lv.4, Purple, DP 4000, Cost 4.
//! Traits: [Evil]. Form: Champion. Attribute: Virus.
//!
//! # Card text (data/card_bundles/BT24-069.md - verbatim)
//!
//! ```text
//! [When Moving] [When Digivolving] Trash 1 card in your hand. Then, your
//!   opponent may trash 1 card in their hand. If your opponent didn't trash
//!   with this effect, trash the top 2 cards of your opponent's deck.
//! [All Turns] While your opponent has 10 or more cards in their trash, this
//!   Digimon gains <Blocker> and +2000 DP.
//! Inherited: [When Attacking] [Once Per Turn] Trash the top card of both
//!   players' decks.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT24/Purple/BT24_069.cs`
//!   - Shared WM/WD body: the controller's mandatory hand discard (when the
//!     hand is non-empty); then the OPPONENT's `canNoSelect: true` hand
//!     discard (only when their hand is non-empty); if they did not trash,
//!     mill 2 of the opponent's deck.
//!   - Static Blocker + 2000 DP while the opponent has >= 10 trash cards.
//!   - Inherited WA OPT: mill 1 of both decks.
//!
//! Regression origin: a recorded PvP game (an EX10 deck line with Vilemon in
//! the breeding area / digivolved) hit this card with no spec. These tests use
//! synthetic boards.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_digivolve, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT24-069";

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.colors = vec![CardColor::Purple];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-069 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_test_card("TRASH-FILLER", "Trash Filler"))
        .add_card(make_test_card("H-A", "Hand A"))
        .add_card(make_test_card("H-B", "Hand B"))
        .add_card(make_test_card("O-A", "Opp Hand A"))
        .add_card(make_test_card("O-B", "Opp Hand B"))
        .add_card(digimon("BASE-L3", 3))
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
}

fn seed_trash(runner: &mut DebugRunner, player: u8, count: usize) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TRASH-FILLER")
        .unwrap();
    for _ in 0..count {
        let next = runner.game.next_card_index();
        runner.game.players[player as usize]
            .trash
            .push(CardSource::new(idx, player, next));
    }
}

/// Vilemon on the field with its [When Digivolving] trigger fired (DebugRunner
/// card data carries no standard evo circle, so the trigger is queued directly).
fn digivolve(runner: &mut DebugRunner) {
    let h = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    runner.game.drain_effect_queue();
}

fn hand(runner: &DebugRunner, p: usize) -> Vec<String> {
    runner.game.players[p]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn pick_hand(runner: &mut DebugRunner, player: u8, card: &str) {
    let idx = hand(runner, player as usize)
        .iter()
        .position(|c| c == card)
        .unwrap_or_else(|| panic!("{card} not in P{player}'s hand"));
    runner
        .execute_action(player, PLAY_HAND_START + idx as u16)
        .expect("hand pick");
}

// ─── Structure ────────────────────────────────────────────────────────────────

#[test]
fn bt24_069_has_printed_metadata_and_move_digivolve_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Vilemon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
    let shared = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnMove) => Some(t),
            _ => None,
        })
        .expect("[When Moving] clause");
    assert!(shared.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!shared.optional);
}

// ─── [When Digivolving] ───────────────────────────────────────────────────────

#[test]
fn bt24_069_wd_own_discard_is_mandatory_then_opponent_may_trash_to_avoid_the_mill() {
    let mut runner = base()
        .hand(0, &["H-A", "H-B"])
        .hand(1, &["O-A", "O-B"])
        .memory(10)
        .start();
    let d1 = runner.deck_size(1);
    digivolve(&mut runner);

    // 1) The controller must trash 1 card (no PASS).
    let sel = runner.pending_selection().expect("own discard");
    assert_eq!(sel.selecting_player, 0);
    assert!(!runner.pending_is_optional(), "canNoSelect: false");
    pick_hand(&mut runner, 0, "H-A");
    assert_eq!(hand(&runner, 0), vec!["H-B".to_string()]);

    // 2) The OPPONENT chooses, and may decline.
    let sel = runner.pending_selection().expect("opponent discard");
    assert_eq!(sel.selecting_player, 1, "the opponent is the chooser");
    assert!(runner.pending_is_optional(), "canNoSelect: true");
    pick_hand(&mut runner, 1, "O-A");

    assert_eq!(hand(&runner, 1), vec!["O-B".to_string()], "the opponent trashed 1 card");
    assert_eq!(runner.deck_size(1), d1, "they trashed -> NO mill");
    assert!(runner.pending_selection().is_none());
}

#[test]
fn bt24_069_wd_opponent_declining_gets_their_deck_milled_by_2() {
    let mut runner = base()
        .hand(0, &["H-A"])
        .hand(1, &["O-A", "O-B"])
        .memory(10)
        .start();
    let d1 = runner.deck_size(1);
    let t1 = runner.trash_size(1);
    digivolve(&mut runner);
    pick_hand(&mut runner, 0, "H-A");

    runner.execute_action(1, PASS).expect("the opponent declines");

    assert_eq!(runner.deck_size(1), d1 - 2, "top 2 of the opponent's deck trashed");
    assert_eq!(runner.trash_size(1), t1 + 2);
    assert_eq!(hand(&runner, 1).len(), 2, "their hand is untouched");
}

#[test]
fn bt24_069_wd_opponent_with_empty_hand_is_milled_without_a_prompt() {
    let mut runner = base().hand(0, &["H-A"]).memory(10).start();
    let d1 = runner.deck_size(1);
    digivolve(&mut runner);
    pick_hand(&mut runner, 0, "H-A");

    assert!(runner.pending_selection().is_none(), "nothing to trash -> no prompt");
    assert_eq!(runner.deck_size(1), d1 - 2, "'didn't trash' -> mill 2");
}

#[test]
fn bt24_069_wd_with_empty_own_hand_skips_the_discard_and_still_asks_the_opponent() {
    let mut runner = base().hand(1, &["O-A"]).memory(10).start();
    digivolve(&mut runner);
    let sel = runner.pending_selection().expect("opponent discard");
    assert_eq!(sel.selecting_player, 1);
}

// ─── [When Moving] ────────────────────────────────────────────────────────────

#[test]
fn bt24_069_when_moving_runs_the_same_body_for_this_digimon() {
    let mut runner = base()
        .hand(0, &["H-A"])
        .hand(1, &["O-A"])
        .memory(10)
        .start();
    runner.place_in_breeding(0, CARD_ID);
    assert!(runner.move_from_breeding(0), "Vilemon moves from the breeding area");

    let sel = runner.pending_selection().expect("own discard prompt on move");
    assert_eq!(sel.selecting_player, 0);
    pick_hand(&mut runner, 0, "H-A");
    let sel = runner.pending_selection().expect("opponent discard prompt");
    assert_eq!(sel.selecting_player, 1);
}

// ─── [All Turns] static: Blocker + 2000 DP at 10+ opponent trash ──────────────

#[test]
fn bt24_069_gains_blocker_and_2000_dp_only_while_opponent_trash_is_10_or_more() {
    let mut runner = base().memory(5).start();
    let vile = runner.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut runner, 1, 9);
    assert_eq!(runner.effective_dp(vile), Some(4000));
    assert!(!runner.game.has_keyword(vile, Keyword::Blocker));

    seed_trash(&mut runner, 1, 1);
    assert_eq!(runner.effective_dp(vile), Some(6000), "+2000 DP at exactly 10");
    assert!(runner.game.has_keyword(vile, Keyword::Blocker), "<Blocker> at exactly 10");
}

// ─── Inherited [When Attacking] [Once Per Turn] ───────────────────────────────

#[test]
fn bt24_069_inherited_attack_mills_one_from_each_deck_once_per_turn() {
    let mut runner = base().add_card(digimon("CARRIER", 5)).memory(5).start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let d0 = runner.deck_size(0);
    let d1 = runner.deck_size(1);

    for _ in 0..2 {
        runner
            .game
            .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
        runner.game.drain_effect_queue();
    }
    assert_eq!(runner.deck_size(0), d0 - 1);
    assert_eq!(runner.deck_size(1), d1 - 1);
}
