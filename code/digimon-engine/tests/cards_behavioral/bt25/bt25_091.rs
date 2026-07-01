//! BT25-091 Monica Simmons — Tamer, Purple, Cost 4, [TS].
//!
//! # Card text (cards.json)
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [On Play] You may return 1 [TS] trait Option card from your trash to the
//!   hand. If this effect didn't return, <Draw 1>.
//! [Your Turn] When you use [TS] trait Option cards, by suspending this Tamer,
//!   1 of your opponent's Digimon can't attack until their turn ends.
//! Inherited [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_091.cs
//!
//! # Patterns this test covers
//! - B1 start-of-turn tamer (memory swing — set to 3)
//! - A4 trash → hand recursion (optional) with else-draw branch
//! - Tamer [Security] play-self
//! - [Your Turn] OnUseOption clause is BLOCKED (dsl) — G-DSL-ON-USE-OPTION-TIMING

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const YAML: &str = include_str!("../../../cards/bt25/BT25-091.yaml");

// ── Section 1: structural ────────────────────────────────────────────────

#[test]
fn bt25_091_structure_start_of_turn_on_play_and_security() {
    let runner = monica_runner().start();
    let compiled = runner
        .compiled_card("BT25-091")
        .expect("BT25-091 compiled card present");

    assert_eq!(compiled.card, "BT25-091");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(4));
    assert!(compiled.traits.iter().any(|t| t == "TS"));

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::StartOfYourTurn]),
        "start-of-your-turn clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnPlay]),
        "on-play clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "security play-self clause present"
    );
}

// ── Section 2: [Start of Your Turn] set memory to 3 ──────────────────────

#[test]
fn bt25_091_start_of_turn_sets_low_memory_to_three() {
    let mut runner = monica_runner()
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(1)
        .start();
    runner.place_on_field(0, "BT25-091", Some(0));
    // start() may already have fired StartOfYourTurn; reset to the pre-fire
    // value and cycle P0 → P1 → P0 so begin_turn fires it with Monica in play.
    runner.game.memory = 1;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 3, "memory <=2 is set to 3");
}

#[test]
fn bt25_091_start_of_turn_does_not_change_high_memory() {
    let mut runner = monica_runner()
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(5)
        .start();
    runner.place_on_field(0, "BT25-091", Some(0));
    runner.game.memory = 5;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.memory(),
        5,
        "memory >2 is unchanged (condition gates)"
    );
}

// ── Section 3: [On Play] return-or-draw ──────────────────────────────────

#[test]
fn bt25_091_on_play_returns_ts_option_from_trash() {
    let mut runner = monica_runner()
        .hand(0, &["BT25-091"])
        .add_card(make_ts_option("TS-OPT"))
        .memory(8)
        .start();
    runner.inject_trash(0, "TS-OPT");

    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);
    let field_index = runner.play(0, 0).expect("Monica plays from hand");
    runner.fire_on_play(0, field_index);

    // Optional select_trash over the TS Option should install.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "[On Play] offers an optional trash return"
    );
    let view = runner.pending_selection_view().unwrap();
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("return the TS Option");
    runner.auto_resolve();

    // The TS Option moved trash → hand. (Net hand: -Monica +TS-OPT = same count
    // as before play; assert the option is in hand and trash shrank.)
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TS-OPT"),
        "returned TS Option is in hand"
    );
    assert_eq!(runner.trash_size(0), trash_before - 1, "trash shrank by 1");
    let _ = hand_before;
}

#[test]
fn bt25_091_on_play_draws_when_no_return() {
    // No TS Option in trash → cannot return → fallback Draw 1.
    let mut runner = monica_runner()
        .hand(0, &["BT25-091"])
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .memory(8)
        .start();

    let field_index = runner.play(0, 0).expect("Monica plays from hand");
    let deck_before = runner.deck_size(0);
    runner.fire_on_play(0, field_index);
    let _ = runner.auto_resolve();

    // The optional return had no legal target, so the else-branch Draw 1 fires.
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "Draw 1 fires when nothing was returned"
    );
}

// ── fixtures ─────────────────────────────────────────────────────────────

fn monica_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT25-091 YAML loads")
}

fn make_ts_option(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Purple];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = vec!["TS".to_string()];
    card
}

fn make_filler(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card
}
