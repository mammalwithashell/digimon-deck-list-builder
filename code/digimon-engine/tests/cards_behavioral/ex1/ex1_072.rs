//! EX1-072 Emergency Program Shutdown! — Option, White, Cost 3, no traits.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX1-072.md, card image)
//!
//! **[Main]** Your opponent can't use Option cards until the end of their next
//! turn.
//! **[Security]** Your opponent can't use Option cards this turn. Then, add
//! this card to its owner's hand.
//!
//! Official Q&A: an Option's [Security] effect is not "using" an Option card,
//! so it still activates under this lock.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX1/White/EX1_072.cs (CanNotPlayClass on
//! enemy Option cards; UntilOpponentTurnEnd / UntilEachTurnEnd).
//!
//! # Patterns
//! - Security "Then, add this card to its owner's hand" (`add_this_option_to_hand`).
//! - Player-scoped `CannotUseOptionCards` modifier (G-ENGINE-CANNOT-USE-OPTION-CARDS).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::CompiledCardKind;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "EX1-072";
const OPP_OPTION: &str = "ST1-13";

fn digimon(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card
}

fn ids(cards: &[CardSource], runner: &DebugRunner) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    ids(&runner.game.player(player).hand, runner)
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    ids(&runner.game.player(player).trash, runner)
}

fn option_usable(runner: &mut DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.enter_main_phase();
    let slot = hand_ids(runner, player)
        .iter()
        .position(|c| c == card_id)
        .expect("option in hand");
    build_action_mask(&runner.game, player)[PLAY_HAND_START as usize + slot] == 1.0
}

fn base_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX1-072 YAML loads")
        .dsl_card(OPP_OPTION)
        .expect("ST1-13 YAML loads")
        .add_card(digimon("WHITE-MON", CardColor::White))
        .add_card(digimon("RED-MON", CardColor::Red))
        .add_card(make_test_card("FILL", "Fill"))
}

// ── §1 Structural ──────────────────────────────────────────────────────────

#[test]
fn ex1_072_is_white_option_cost_3() {
    let runner = base_builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
}

// ── §2 [Security] tail ─────────────────────────────────────────────────────

/// "Then, add this card to its owner's hand." — the card goes to the
/// defender's (owner's) hand, not the trash.
#[test]
fn ex1_072_security_adds_this_card_to_owners_hand() {
    let mut runner = base_builder()
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"; 3])
        .deck(1, &["FILL"; 3])
        .memory(5)
        .start();
    let attacker = runner.place_on_field(0, "RED-MON", Some(0));
    let result = runner.attack_player(attacker, 1, false);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert!(hand_ids(&runner, 1).contains(&CARD_ID.to_string()));
    assert!(!trash_ids(&runner, 1).contains(&CARD_ID.to_string()));
}

// ── §3 Option-use lock ───────────────────────────────────────────

/// Control (active): without EX1-072, the opponent's Option is usable.
#[test]
fn ex1_072_control_opponent_option_usable_without_lock() {
    let mut runner = base_builder()
        .hand(1, &[OPP_OPTION])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    runner.place_on_field(1, "RED-MON", Some(0));
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1);
    assert!(option_usable(&mut runner, 1, OPP_OPTION));
}

/// [Main]: opponent can't use Option cards during their next turn; own
/// Options are unaffected; the lock ends after the opponent's next turn.
#[test]
fn ex1_072_main_locks_opponent_options_until_end_of_their_next_turn() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID, OPP_OPTION])
        .hand(1, &[OPP_OPTION])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    runner.place_on_field(0, "WHITE-MON", Some(0));
    runner.place_on_field(0, "RED-MON", Some(0));
    runner.place_on_field(1, "RED-MON", Some(0));

    assert!(option_usable(&mut runner, 0, CARD_ID), "EX1-072 is usable");
    let slot = hand_ids(&runner, 0)
        .iter()
        .position(|c| c == CARD_ID)
        .unwrap();
    let _ = runner.game.play_option_from_hand(0, slot);
    runner.auto_resolve().expect("resolve main");
    assert!(
        option_usable(&mut runner, 0, OPP_OPTION),
        "the user's own Options are not locked"
    );

    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1);
    assert!(
        !option_usable(&mut runner, 1, OPP_OPTION),
        "opponent can't use Option cards during their next turn"
    );

    runner.end_turn();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1);
    assert!(
        option_usable(&mut runner, 1, OPP_OPTION),
        "the lock ends at the end of the opponent's next turn"
    );
}

/// [Security]: the attacking player can't use Option cards for the rest of
/// this turn.
#[test]
fn ex1_072_security_locks_attackers_options_this_turn() {
    let mut runner = base_builder()
        .hand(0, &[OPP_OPTION])
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    let attacker = runner.place_on_field(0, "RED-MON", Some(0));
    runner.place_on_field(0, "RED-MON", Some(0));
    assert!(
        option_usable(&mut runner, 0, OPP_OPTION),
        "usable before the check"
    );
    let _ = runner.attack_player(attacker, 1, false);
    assert!(
        !option_usable(&mut runner, 0, OPP_OPTION),
        "attacker can't use Option cards this turn"
    );
    runner.end_turn();
    runner.end_turn();
    assert!(
        option_usable(&mut runner, 0, OPP_OPTION),
        "the security lock lasts only this turn"
    );
}

// ── §4 Lock scope: Counter-timing / effect-driven uses, security effects ──

/// Minimal Counter-timing hand Option (fixture) — "[Counter] <Draw 1>".
const COUNTER_OPTION_YAML: &str = r#"
card: CTR-OPT
name: Counter Fixture
kind: option
color: [red]
cost: 0
effects:
  - when: counter
    summary: "[Counter] Draw 1"
    process:
      - draw: { of: you, count: 1 }
"#;

fn play_ex1_072_main(runner: &mut DebugRunner) {
    runner.game.enter_main_phase();
    let slot = hand_ids(runner, 0)
        .iter()
        .position(|c| c == CARD_ID)
        .unwrap();
    let _ = runner.game.play_option_from_hand(0, slot);
    runner.auto_resolve().expect("resolve main");
}

fn counter_runner() -> DebugRunner {
    base_builder()
        .from_dsl_yaml(COUNTER_OPTION_YAML)
        .expect("counter fixture")
        .hand(0, &[CARD_ID])
        .hand(1, &["CTR-OPT"])
        .security(1, &["FILL"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start()
}

/// Control: without the lock the defender's Counter Option opens a Counter
/// window.
#[test]
fn ex1_072_control_counter_option_offered_without_lock() {
    let mut runner = counter_runner();
    runner.place_on_field(0, "WHITE-MON", Some(0));
    let attacker = runner.place_on_field(0, "RED-MON", Some(0));
    runner.place_on_field(1, "RED-MON", Some(0));
    runner.game.enter_main_phase();
    runner.attack_player(attacker, 1, false);
    assert_eq!(
        runner.current_phase(),
        digimon_engine::enums::GamePhase::CounterTiming,
        "a Counter-timing hand Option is a counter candidate"
    );
}

/// Under the [Main] lock the opponent's hand Counter-timing Option is a use
/// and is not offered.
#[test]
fn ex1_072_lock_blocks_opponents_counter_timing_option() {
    let mut runner = counter_runner();
    runner.place_on_field(0, "WHITE-MON", Some(0));
    let attacker = runner.place_on_field(0, "RED-MON", Some(0));
    runner.place_on_field(1, "RED-MON", Some(0));
    play_ex1_072_main(&mut runner);
    runner.attack_player(attacker, 1, false);
    assert_ne!(
        runner.current_phase(),
        digimon_engine::enums::GamePhase::CounterTiming,
        "the locked player can't use an Option at Counter timing"
    );
}

/// Effect-driven uses ("use 1 Option card from hand") are uses too.
#[test]
fn ex1_072_lock_blocks_effect_driven_option_use() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID])
        .hand(1, &[OPP_OPTION])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    runner.place_on_field(0, "WHITE-MON", Some(0));
    runner.place_on_field(1, "RED-MON", Some(0));
    play_ex1_072_main(&mut runner);
    runner.end_turn();
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.use_option_from_hand_without_paying_cost(1, 0),
        digimon_engine::selection::OptionPlayResult::Invalid,
        "an effect can't make the locked player use an Option card"
    );
    assert!(hand_ids(&runner, 1).contains(&OPP_OPTION.to_string()));
}

/// Official Q&A: an Option's [Security] effect is not "using" an Option card —
/// the LOCKED player's own security Option still activates.
#[test]
fn ex1_072_lock_does_not_stop_security_effects() {
    let mut runner = base_builder()
        .hand(0, &[CARD_ID])
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    runner.place_on_field(0, "WHITE-MON", Some(0));
    let attacker = runner.place_on_field(0, "RED-MON", Some(0));
    play_ex1_072_main(&mut runner); // P1 is now locked.
    assert!(runner.game.option_use_blocked(1));
    let result = runner.attack_player(attacker, 1, false);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert!(
        hand_ids(&runner, 1).contains(&CARD_ID.to_string()),
        "the locked player's [Security] Option effect still resolves"
    );
}

// ── §5 DSL use-option selections hide locked candidates ───────────────────

/// Fixture: "[On Play] You may use 1 Option card from your hand (or trash)
/// without paying the cost."
const USER_YAML: &str = r#"
card: USE-OPT-MON
name: Use Option Fixture
kind: digimon
color: [red]
cost: 0
level: 3
dp: 3000
effects:
  - when: on_play
    summary: "[On Play] You may use 1 Option card from your hand without paying the cost"
    process:
      - use_option_from_hand:
          of: you
          optional: true
          filter: { kind: option }
"#;

fn use_option_runner() -> DebugRunner {
    base_builder()
        .from_dsl_yaml(USER_YAML)
        .expect("fixture")
        .hand(0, &[CARD_ID])
        .hand(1, &["USE-OPT-MON", OPP_OPTION])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start()
}

/// Control: unlocked, the hand Option is a candidate of the DSL use step.
#[test]
fn ex1_072_control_dsl_use_option_offers_hand_option() {
    let mut runner = use_option_runner();
    runner.place_on_field(1, "RED-MON", Some(0));
    runner.end_turn();
    runner.game.enter_main_phase();
    let slot = hand_ids(&runner, 1)
        .iter()
        .position(|c| c == "USE-OPT-MON")
        .unwrap();
    runner.play(1, slot);
    let view = runner
        .pending_selection_view()
        .expect("the use-option pick is offered");
    assert!(
        view.valid_action_ids
            .iter()
            .any(|a| *a != digimon_engine::action::space::PASS),
        "the hand Option is a legal candidate"
    );
}

/// Locked: the DSL `use_option_from_hand` step surfaces no dead candidate.
#[test]
fn ex1_072_lock_hides_option_from_dsl_use_option_selection() {
    let mut runner = use_option_runner();
    runner.place_on_field(0, "WHITE-MON", Some(0));
    runner.place_on_field(1, "RED-MON", Some(0));
    play_ex1_072_main(&mut runner);
    runner.end_turn();
    runner.game.enter_main_phase();
    assert!(runner.game.option_use_blocked(1));
    let slot = hand_ids(&runner, 1)
        .iter()
        .position(|c| c == "USE-OPT-MON")
        .unwrap();
    runner.play(1, slot);
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            view.valid_action_ids
                .iter()
                .all(|a| *a == digimon_engine::action::space::PASS),
            "no Option candidate may reach the action space while locked: {view:?}"
        );
    }
    assert!(hand_ids(&runner, 1).contains(&OPP_OPTION.to_string()));
}
