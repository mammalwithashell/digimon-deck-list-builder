//! EX4-005 Agumon — Digimon, Lv.3, Red/Yellow, DP 2000, Cost 3. Trait: Dinosaur.
//!
//! # Card text (EX4-005.json)
//! `[Start of Your Main Phase] If you have a red or yellow Tamer in play,`
//! `gain 1 memory.`
//! `Inherited: [Your Turn] [Once Per Turn] When one of your red or yellow`
//! `Tamers becomes suspended, ＜Draw 1＞ (Draw 1 card from your deck.)`
//! Digivolve: [Koromon]: Cost 0 (xros_req).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX4/Red/EX4_005.cs
//!   - OnStartMainPhase ActivateClass, isOptional: false (mandatory); gate =
//!     own red/yellow Tamer in play → gain 1 memory.
//!   - OnTappedAnyone inherited, IsOwnerTurn, maxUsableCount: 1, own red/yellow
//!     Tamer suspend → Draw 1, isOptional: false.
//!
//! # Patterns (test API §4.3): B1 (start-of-main conditional gain_memory),
//! B3/E2 (inherited OPT suspend draw). Both clauses mandatory.

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

fn agumon() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("EX4-005")
}

fn make_tamer(card_id: &str, name: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.dp = None;
    card.level = None;
    card
}

fn make_digimon(card_id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.dp = Some(3000);
    card.level = Some(4);
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX4-005")
        .expect("EX4-005 in pack")
        .add_card(make_digimon("HOST", CardColor::Red))
        .add_card(make_tamer("RED-TAMER", "Red Tamer", CardColor::Red))
        .add_card(make_tamer("YEL-TAMER", "Yellow Tamer", CardColor::Yellow))
        .add_card(make_tamer("GRN-TAMER", "Green Tamer", CardColor::Green))
        .add_card(make_digimon("FILLER", CardColor::Blue))
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start()
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn ex4_005_metadata_red_yellow_lv3_dinosaur() {
    let card = agumon();
    assert_eq!(card.card, "EX4-005");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
    let colors: Vec<_> = card.color.iter().copied().collect();
    assert!(colors.contains(&digimon_dsl::compiled::CompiledColor::Red));
    assert!(colors.contains(&digimon_dsl::compiled::CompiledColor::Yellow));
}

#[test]
fn ex4_005_has_start_main_and_inherited_suspend_clauses() {
    let card = agumon();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2);

    let start_main = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("start-of-main clause");
    assert!(!start_main.optional, "gain-memory is mandatory");
    assert_eq!(start_main.scope, CompiledScope::FaceUp);

    let inherited = triggered
        .iter()
        .find(|t| t.scope == CompiledScope::Inherited)
        .expect("inherited suspend clause");
    assert!(inherited.when.contains(&CompiledTiming::OnSuspend));
    assert!(inherited.once_per_turn);
    assert!(!inherited.optional, "draw is mandatory");
}

#[test]
fn ex4_005_has_koromon_digivolve_alt_path_cost0() {
    let card = agumon();
    let found = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(0))
    });
    assert!(found, "must declare a cost-0 [Koromon] digivolve; got {:?}", card.alt_paths);
}

// ─── SECTION 2 — Clause 1 [Start of Your Main Phase] gain 1 memory ──────────

#[test]
fn ex4_005_gains_memory_with_red_tamer_in_play() {
    let mut runner = runner();
    runner.place_on_field(0, "EX4-005", Some(0));
    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.game.set_memory(0);

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(runner.memory(), 1, "red Tamer in play → +1 memory at start of main");
}

#[test]
fn ex4_005_gains_memory_with_yellow_tamer_in_play() {
    let mut runner = runner();
    runner.place_on_field(0, "EX4-005", Some(0));
    runner.place_on_field(0, "YEL-TAMER", Some(0));
    runner.game.set_memory(0);

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(runner.memory(), 1, "yellow Tamer in play → +1 memory");
}

#[test]
fn ex4_005_no_memory_without_red_or_yellow_tamer() {
    let mut runner = runner();
    runner.place_on_field(0, "EX4-005", Some(0));
    runner.place_on_field(0, "GRN-TAMER", Some(0)); // green Tamer doesn't count
    runner.game.set_memory(0);

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        0,
        "no red/yellow Tamer in play → no memory gain"
    );
}

#[test]
fn ex4_005_does_not_gain_on_opponents_main_phase() {
    let mut runner = runner();
    runner.place_on_field(0, "EX4-005", Some(0));
    runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.game.turn_player_idx = 1; // opponent's turn
    runner.game.set_memory(0);

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        0,
        "[Start of YOUR Main Phase] must not fire on the opponent's main phase"
    );
}

// ─── SECTION 3 — Clause 2 inherited [Your Turn][OPT] suspend → Draw 1 ───────
//
// Exercised with EX4-005 as a digivolution SOURCE under a host Digimon.

fn host_with_agumon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_stack(0, &["EX4-005", "HOST"])
}

#[test]
fn ex4_005_draws_when_own_red_tamer_suspends() {
    let mut runner = runner();
    host_with_agumon(&mut runner);
    let tamer = runner.place_on_field(0, "RED-TAMER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1);
}

#[test]
fn ex4_005_draws_when_own_yellow_tamer_suspends() {
    let mut runner = runner();
    host_with_agumon(&mut runner);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1);
}

#[test]
fn ex4_005_inherited_does_not_draw_on_green_tamer() {
    let mut runner = runner();
    host_with_agumon(&mut runner);
    let tamer = runner.place_on_field(0, "GRN-TAMER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].hand.len(), hand_before);
}

#[test]
fn ex4_005_inherited_does_not_draw_on_opponent_tamer() {
    let mut runner = runner();
    host_with_agumon(&mut runner);
    let opp = runner.place_on_field(1, "RED-TAMER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(opp);
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].hand.len(), hand_before);
}

#[test]
fn ex4_005_inherited_does_not_draw_on_opponents_turn() {
    let mut runner = runner();
    host_with_agumon(&mut runner);
    let tamer = runner.place_on_field(0, "RED-TAMER", Some(0));
    runner.game.turn_player_idx = 1;

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].hand.len(), hand_before);
}

#[test]
fn ex4_005_inherited_opt_blocks_second_suspend_same_turn() {
    let mut runner = runner();
    host_with_agumon(&mut runner);
    let t1 = runner.place_on_field(0, "RED-TAMER", Some(0));
    let t2 = runner.place_on_field(0, "YEL-TAMER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(t1);
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1);

    runner.game.suspend(t2);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "[Once Per Turn] blocks the second suspend-draw same turn"
    );
}
