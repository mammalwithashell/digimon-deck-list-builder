//! BT18-093 Violet Inboots — Tamer, Purple, Cost 4. Traits: [LIBERATOR].
//!
//! # Card text (card image / official DB — authoritative)
//!
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [Start of Your Main Phase] By trashing 1 Option card or 1 card with the
//! [Ghost] or [Three Musketeers] trait in your hand, ＜Draw 1＞ (Draw 1 card
//! from your deck.)
//!
//! Inherited (Security):
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT18/Purple/BT18_093.cs
//!   - OnStartTurn → `SetMemoryTo3TamerEffect`.
//!   - OnStartMainPhase → ActivateClass(isOptional = true), CanActivateCondition
//!     = `HasMatchConditionOwnersHand(IsOption || Ghost || Three Musketeers)`;
//!     SelectHandEffect(maxCount 1, canNoSelect true, mode Discard) → on ≥1
//!     pick, `DrawClass(owner, 1)`.
//!   - SecuritySkill → `PlaySelfTamerSecurityEffect`.
//!
//! # Patterns this test covers
//! - B1 start-of-main tamer (draw) + B2 play-4 anchor (start-of-turn memory gate)
//! - E2 optional cost ("By trashing …") with explicit decline
//! - Selection: `SelectionKind::Hand` filtered to Option / [Ghost] / [Three Musketeers]
//! - Tamer [Security] play-self (`play_from_security`)

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT18-093";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Purple];
    c
}

/// A plain purple Digimon with no relevant trait — NOT trashable fodder.
fn plain_digimon(id: &str) -> CardData {
    filler(id)
}

fn option_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = vec![CardColor::Purple];
    c
}

fn ghost_digimon(id: &str) -> CardData {
    let mut c = filler(id);
    c.traits = vec!["Ghost".to_string()];
    c
}

fn musketeer_digimon(id: &str) -> CardData {
    let mut c = filler(id);
    c.traits = vec!["Three Musketeers".to_string()];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT18-093 must load from the embedded DSL pack")
        .add_card(filler("FILL"))
        .add_card(plain_digimon("PLAIN"))
        .add_card(option_card("OPT"))
        .add_card(ghost_digimon("GHOST"))
        .add_card(musketeer_digimon("TM"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
}

fn find_permanent(runner: &DebugRunner, player: u8, card_id: &str) -> digimon_engine::permanent::PermanentHandle {
    let idx = runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not on player {player}'s field"));
    runner.perm_handle(player, idx)
}

/// Fire the [Start of Your Main Phase] trigger for Violet Inboots on P0's
/// field (BT18-092 idiom — the real trigger path, without the turn cycle).
fn fire_start_of_main(runner: &mut DebugRunner) {
    let inboots = find_permanent(runner, 0, CARD_ID);
    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(inboots));
    runner.game.drain_effect_queue();
}

// ─── Section 1: structural ───────────────────────────────────────────────────

#[test]
fn bt18_093_metadata_and_three_triggered_clauses() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled card present");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(4));
    assert!(compiled.traits.iter().any(|t| t == "LIBERATOR"));

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 3, "start_of_your_turn + start_of_your_main_phase + on_security");

    let sot = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::StartOfYourTurn])
        .expect("start_of_your_turn clause");
    assert!(!sot.optional, "the memory gate is not player-optional");
    assert!(!sot.once_per_turn);
    assert_eq!(sot.scope, CompiledScope::FaceUp);

    let som = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::StartOfYourMainPhase])
        .expect("start_of_your_main_phase clause");
    assert!(som.optional, "'By trashing …' is an optional processing condition (§15-7-1)");
    assert!(!som.once_per_turn, "no [Once Per Turn] is printed");
    assert_eq!(som.scope, CompiledScope::FaceUp);

    let sec = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("on_security clause");
    assert!(!sec.optional, "[Security] play-self is mandatory");
}

// ─── Section 2: [Start of Your Turn] memory gate ─────────────────────────────

#[test]
fn bt18_093_start_of_turn_sets_low_memory_to_three() {
    let mut runner = base().memory(1).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 1;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 3, "memory ≤ 2 is set to 3 at the start of P0's turn");
}

#[test]
fn bt18_093_start_of_turn_leaves_high_memory_alone() {
    let mut runner = base().memory(5).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 5;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 5, "memory > 2 is untouched (condition gates)");
}

// ─── Section 3: [Start of Your Main Phase] trash-1 → Draw 1 ──────────────────

#[test]
fn bt18_093_start_of_main_offers_only_eligible_hand_cards() {
    let mut runner = base()
        .hand(0, &["PLAIN", "OPT", "GHOST", "TM"])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    fire_start_of_main(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("an eligible card in hand must offer the trash-cost selection");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional, "'By trashing' is declinable — PASS must be legal");
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        picks.len(),
        3,
        "only the Option, the [Ghost] card and the [Three Musketeers] card are offered (PLAIN excluded)"
    );
}

#[test]
fn bt18_093_start_of_main_no_prompt_without_eligible_card() {
    let mut runner = base().hand(0, &["PLAIN", "PLAIN"]).memory(5).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    fire_start_of_main(&mut runner);
    assert!(
        runner.pending_selection().is_none(),
        "no Option / [Ghost] / [Three Musketeers] card in hand → nothing to offer (DCGO HasMatchConditionOwnersHand)"
    );
}

#[test]
fn bt18_093_trashing_option_draws_one() {
    let mut runner = base().hand(0, &["PLAIN", "OPT"]).memory(5).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);

    fire_start_of_main(&mut runner);
    let view = runner.pending_selection_view().expect("trash-cost prompt");
    let pick = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("the Option is a legal pick");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("trash the Option");
    let _ = runner.auto_resolve();

    assert_eq!(runner.trash_size(0), trash_before + 1, "the Option was trashed");
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "OPT"),
        "the trashed card is the chosen Option"
    );
    assert_eq!(runner.deck_size(0), deck_before - 1, "<Draw 1> fired");
    assert_eq!(runner.hand_size(0), hand_before, "-1 trashed +1 drawn = net 0");
}

#[test]
fn bt18_093_declining_trash_cost_changes_nothing() {
    let mut runner = base().hand(0, &["TM"]).memory(5).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);

    fire_start_of_main(&mut runner);
    let view = runner.pending_selection_view().expect("trash-cost prompt");
    assert!(view.is_optional);
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the cost");
    let _ = runner.auto_resolve();

    assert_eq!(runner.hand_size(0), hand_before, "nothing trashed, nothing drawn");
    assert_eq!(runner.deck_size(0), deck_before, "no draw on decline (§15-7-2)");
    assert_eq!(runner.trash_size(0), trash_before);
}

// ─── Section 4: [Security] play without paying the cost ──────────────────────

#[test]
fn bt18_093_security_plays_itself_free() {
    let mut attacker = make_test_card("ATK", "ATK");
    attacker.dp = Some(5000);
    let mut runner = base()
        .add_card(attacker)
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    let atk = runner.place_on_field(0, "ATK", Some(0));
    let memory_before = runner.memory();

    let _ = runner.attack_player(atk, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(runner.security_count(1), 0, "the revealed Tamer left security");
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Violet Inboots was played into P1's battle area by its [Security] effect"
    );
    assert_eq!(runner.memory(), memory_before, "played without paying the cost");
}
