//! EX10-046 Devimon - Digimon, Lv.4, Purple, DP 5000, Cost 5.
//! Traits: [Fallen Angel]. Form: Champion. Attribute: Virus.
//!
//! # Card text (data/card_bundles/EX10-046.md - verbatim)
//!
//! ```text
//! [Start of Your Main Phase] [When Digivolving] If your opponent has 10 or
//!   fewer cards in their trash, trash the top 2 cards of both players'
//!   decks. Then, if they have 10 or more cards in their trash, you may
//!   return 1 card with the [Fallen Angel] or [Undead] trait from your trash
//!   to the hand.
//! Inherited: [When Attacking] [Once Per Turn] Trash the top card of both
//!   players' decks.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/EX10/Purple/EX10_046.cs`
//!   - Shared body: mill 2+2 iff opp trash <= 10; THEN (fresh re-read) iff opp
//!     trash >= 10 and a [Fallen Angel]/[Undead] card is in your trash, an
//!     optional (`canNoSelect: () => true`) pick returns it to hand.
//!   - Inherited WA OPT: unconditional 1+1 mill.
//!
//! Regression origin: a recorded PvP game's Devimon -> SkullSatamon line
//! (Devimon had no spec). These tests use synthetic boards.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX10-046";

fn card_with(id: &str, kind: CardKind, level: Option<u8>, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.level = level;
    c.dp = if kind == CardKind::Digimon { Some(3000) } else { None };
    c.colors = vec![CardColor::Purple];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX10-046 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_test_card("TRASH-FILLER", "Trash Filler"))
        .add_card(card_with("BASE-L3", CardKind::Digimon, Some(3), &[]))
        .add_card(card_with("FA-CARD", CardKind::Digimon, Some(4), &["Fallen Angel"]))
        .add_card(card_with("UNDEAD-CARD", CardKind::Digimon, Some(5), &["Undead"]))
        .add_card(card_with("UNDEAD-OPTION", CardKind::Option, None, &["Undead"]))
        .add_card(card_with("OTHER-CARD", CardKind::Digimon, Some(4), &["Evil"]))
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
}

fn seed_trash(runner: &mut DebugRunner, player: u8, card_id: &str, count: usize) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card {card_id}"));
    for _ in 0..count {
        let next = runner.game.next_card_index();
        runner.game.players[player as usize]
            .trash
            .push(CardSource::new(idx, player, next));
    }
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn offered_trash_count(runner: &DebugRunner) -> usize {
    runner
        .pending_selection()
        .expect("trash pick parked")
        .valid_action_ids
        .iter()
        .filter(|&&id| id != PASS)
        .count()
}

// ─── Structure ────────────────────────────────────────────────────────────────

#[test]
fn ex10_046_has_printed_metadata_and_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Devimon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    let shared = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::FaceUp => Some(t),
            _ => None,
        })
        .expect("own clause");
    assert!(shared.when.contains(&CompiledTiming::StartOfYourMainPhase));
    assert!(shared.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!shared.optional, "the clause is mandatory; only the return is 'you may'");
}

// ─── [Start of Your Main Phase] ───────────────────────────────────────────────

#[test]
fn ex10_046_mills_two_from_each_deck_when_opponent_trash_is_10_or_fewer() {
    let mut runner = base().memory(5).start();
    let devi = runner.place_on_field(0, CARD_ID, Some(0));
    let d0 = runner.deck_size(0);
    let d1 = runner.deck_size(1);

    fire(&mut runner, EffectTiming::StartOfYourMainPhase, devi);

    assert_eq!(runner.deck_size(0), d0 - 2);
    assert_eq!(runner.deck_size(1), d1 - 2);
    assert!(
        runner.pending_selection().is_none(),
        "opponent trash is only 2 after the mill (< 10): no return offered"
    );
}

#[test]
fn ex10_046_no_mill_when_opponent_trash_is_above_10_but_return_is_offered() {
    let mut runner = base().memory(5).start();
    let devi = runner.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut runner, 1, "TRASH-FILLER", 11);
    seed_trash(&mut runner, 0, "FA-CARD", 1);
    let d0 = runner.deck_size(0);

    fire(&mut runner, EffectTiming::StartOfYourMainPhase, devi);

    assert_eq!(runner.deck_size(0), d0, "no mill above 10");
    assert!(runner.pending_is_optional(), "DCGO canNoSelect: () => true");
    assert_eq!(offered_trash_count(&runner), 1);
}

#[test]
fn ex10_046_return_condition_is_re_read_after_the_mill() {
    // 8 <= 10: mill 2 -> the opponent's trash is 10 -> "if they have 10 or
    // more" is now true and the return is offered.
    let mut runner = base().memory(5).start();
    let devi = runner.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut runner, 1, "TRASH-FILLER", 8);
    seed_trash(&mut runner, 0, "UNDEAD-CARD", 1);

    fire(&mut runner, EffectTiming::StartOfYourMainPhase, devi);

    assert_eq!(runner.game.players[1].trash.len(), 10);
    assert!(
        runner.pending_selection().is_some(),
        "the mill pushed the opponent to 10: the return is offered"
    );
}

#[test]
fn ex10_046_return_offers_fallen_angel_or_undead_cards_of_any_kind() {
    let mut runner = base().memory(5).start();
    let devi = runner.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut runner, 1, "TRASH-FILLER", 10);
    for id in ["FA-CARD", "UNDEAD-CARD", "UNDEAD-OPTION", "OTHER-CARD", "PAD"] {
        seed_trash(&mut runner, 0, id, 1);
    }

    fire(&mut runner, EffectTiming::StartOfYourMainPhase, devi);

    // The two mill-2 cards are PAD (non-matching) too.
    assert_eq!(
        offered_trash_count(&runner),
        3,
        "[Fallen Angel] and [Undead] trait cards (including an Option) are offered; [Evil] and filler are not"
    );
}

#[test]
fn ex10_046_returning_a_card_moves_it_from_trash_to_hand() {
    let mut runner = base().memory(5).start();
    let devi = runner.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut runner, 1, "TRASH-FILLER", 10);
    seed_trash(&mut runner, 0, "FA-CARD", 1);

    fire(&mut runner, EffectTiming::StartOfYourMainPhase, devi);
    runner.auto_resolve().expect("take the only candidate");

    assert!(hand_ids(&runner).contains(&"FA-CARD".to_string()));
}

#[test]
fn ex10_046_the_return_may_be_declined() {
    let mut runner = base().memory(5).start();
    let devi = runner.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut runner, 1, "TRASH-FILLER", 10);
    seed_trash(&mut runner, 0, "FA-CARD", 1);

    fire(&mut runner, EffectTiming::StartOfYourMainPhase, devi);
    runner.execute_action(0, PASS).expect("decline");

    assert!(!hand_ids(&runner).contains(&"FA-CARD".to_string()));
}

// ─── [When Digivolving] ───────────────────────────────────────────────────────

#[test]
fn ex10_046_when_digivolving_runs_the_same_body() {
    let mut runner = base().memory(10).start();
    let devi = runner.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut runner, 1, "TRASH-FILLER", 9);
    seed_trash(&mut runner, 0, "UNDEAD-CARD", 1);
    let d0 = runner.deck_size(0);

    fire(&mut runner, EffectTiming::WhenDigivolving, devi);

    assert_eq!(runner.deck_size(0), d0 - 2, "mills 2 (9 <= 10)");
    assert!(runner.pending_selection().is_some(), "9 + 2 = 11 >= 10: the return is offered");
}

// ─── Inherited [When Attacking] [Once Per Turn] ───────────────────────────────

#[test]
fn ex10_046_inherited_attack_mills_one_from_each_deck_with_no_conditions() {
    let mut runner = base()
        .add_card(card_with("CARRIER", CardKind::Digimon, Some(5), &[]))
        .memory(5)
        .start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    seed_trash(&mut runner, 1, "TRASH-FILLER", 30);
    let d0 = runner.deck_size(0);
    let d1 = runner.deck_size(1);

    fire(&mut runner, EffectTiming::WhenAttacking, stack);
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.deck_size(0), d0 - 1);
    assert_eq!(runner.deck_size(1), d1 - 1);

    // [Once Per Turn]
    fire(&mut runner, EffectTiming::WhenAttacking, stack);
    assert_eq!(runner.deck_size(0), d0 - 1, "second attack this turn does not mill again");
}
