//! EX3-045 Hydramon — Digimon, Lv.6, Green, DP 13000, Cost 13.
//! Traits: Vegetation. Form: Mega. Attribute: Virus. Digivolve: Green Lv.5 / 5.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX3-045.md)
//! [When Digivolving] You may suspend 1 Digimon.
//! [All Turns][Once Per Turn] When an opponent's Digimon becomes suspended, for
//! each other suspended Digimon with [Vegetation], [Plant], or [Fairy] in one of
//! its traits you have in play, gain 1 memory.
//! [End of Your Turn][Once Per Turn] If you have 2 or more suspended Digimon
//! with [Vegetation], [Plant], or [Fairy] in one of their traits, return 1 of
//! your opponent's suspended Digimon to the bottom of its owner's deck.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX3/Green/EX3_045.cs
//! - "other" = `permanent != card.PermanentOfThisCard()` (other than Hydramon).
//! - EoT gate counts Hydramon itself; PutLibraryBottom trashes sources and
//!   puts only the top card on the deck bottom.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API 4.3)
//! - Real digivolve flow (encode_digivolve) → [When Digivolving] any-side
//!   optional suspend pick
//! - on_suspend observer gated by event_target_owner/kind, OPT lockout
//! - gain_memory formula with `other: true` + trait_contains any_of
//! - End-of-your-turn conditional mandatory pick + return_to_deck bottom

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX3-045";

fn make_digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(5000);
    card.colors = vec![CardColor::Green];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX3-045 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("GREEN-LV5", 5, &["Beast"]))
        .add_card(make_digimon("VEG", 4, &["Vegetation"]))
        .add_card(make_digimon("DARK-PLANT", 4, &["Dark Plant"]))
        .add_card(make_digimon("FAIRY", 4, &["Fairy"]))
        .add_card(make_digimon("BEAST", 4, &["Beast"]))
        .add_card(make_digimon("OPP-SRC", 3, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

fn is_suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn pick(r: &mut DebugRunner, h: PermanentHandle) {
    let id = match r.pending_kind() {
        Some(SelectionKind::AnyField) => encode_attack(h.player as u16, h.index as u16),
        _ => encode_attack(0, h.index as u16),
    };
    let player = r.pending_selection().unwrap().selecting_player;
    r.execute_action(player, id).expect("pick target");
}

fn pass(r: &mut DebugRunner) {
    let player = r.pending_selection().unwrap().selecting_player;
    r.execute_action(player, PASS).expect("PASS");
}

fn digivolve_onto(r: &mut DebugRunner, host: PermanentHandle) {
    r.game.enter_main_phase();
    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[action as usize], 1.0, "Green Lv.5 → Hydramon is legal");
    r.game.decode_action(action, 0);
}

// ─── Structural ─────────────────────────────────────────────────────────────

#[test]
fn ex3_045_metadata_and_clauses() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Hydramon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(13));
    assert_eq!(card.dp, Some(13000));
    assert!(card.traits.contains(&"Vegetation".to_string()));

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    let on_suspend = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnSuspend))
        .expect("on_suspend clause");
    assert!(on_suspend.once_per_turn);
    assert!(on_suspend.condition.is_none(), "DCGO: 0-count activation still spends OPT");
    let eot = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::EndOfYourTurn))
        .expect("end-of-your-turn clause");
    assert!(eot.once_per_turn);
    assert!(eot.condition.is_some(), "2+ suspended Veg/Plant/Fairy gate");
}

// ─── [When Digivolving] + [All Turns] memory ────────────────────────────────

/// WD offers any-side optional suspend; suspending an opponent's Digimon
/// triggers the [All Turns] memory gain: 1 per OTHER own suspended matching
/// Digimon (substring trait match: [Dark Plant] counts, [Beast] does not).
#[test]
fn ex3_045_wd_suspend_opponent_triggers_memory_gain() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let host = r.place_on_field(0, "GREEN-LV5", Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    let plant = r.place_on_field(0, "DARK-PLANT", Some(0));
    let beast = r.place_on_field(0, "BEAST", Some(0));
    let opp = r.place_on_field(1, "BEAST", Some(0));
    r.game.suspend(veg);
    r.game.suspend(plant);
    r.game.suspend(beast);
    digivolve_onto(&mut r, host);
    let after_cost = r.memory();
    assert_eq!(after_cost, 5, "digivolve cost 5");

    assert!(matches!(r.pending_kind(), Some(SelectionKind::AnyField)));
    let view = r.pending_selection_view().unwrap();
    assert!(view.is_optional, "you MAY suspend");
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        2,
        "unsuspended Digimon on both sides: Hydramon + opponent's Digimon"
    );

    pick(&mut r, opp);
    r.auto_resolve().ok();
    assert!(is_suspended(&r, opp));
    assert_eq!(r.memory(), after_cost + 2, "VEG + [Dark Plant] → +2 ([Beast] ignored)");
}

/// Declining the WD suspend: nothing suspends, no memory.
#[test]
fn ex3_045_wd_decline_no_suspend() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let host = r.place_on_field(0, "GREEN-LV5", Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    r.game.suspend(veg);
    let opp = r.place_on_field(1, "BEAST", Some(0));
    digivolve_onto(&mut r, host);
    let after_cost = r.memory();
    pass(&mut r);
    r.auto_resolve().ok();
    assert!(!is_suspended(&r, opp));
    assert!(!is_suspended(&r, host));
    assert_eq!(r.memory(), after_cost);
}

/// "other": a suspended Hydramon does not count itself.
#[test]
fn ex3_045_memory_count_excludes_hydramon_itself() {
    let mut r = base().memory(3).start();
    let hydra = r.place_on_field(0, CARD_ID, Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    let opp = r.place_on_field(1, "BEAST", Some(0));
    r.game.suspend(hydra);
    r.game.suspend(veg);
    r.auto_resolve().ok();
    let before = r.memory();
    assert_eq!(before, 3, "own suspensions never trigger");

    r.game.suspend(opp);
    r.auto_resolve().ok();
    assert_eq!(r.memory(), before + 1, "VEG only — Hydramon is not 'other'");
}

/// Once Per Turn: a second opponent suspension in the same turn gains nothing;
/// the next turn it is available again.
#[test]
fn ex3_045_memory_gain_once_per_turn() {
    let mut r = base().memory(3).start();
    let hydra = r.place_on_field(0, CARD_ID, Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    let fairy = r.place_on_field(0, "FAIRY", Some(0));
    let opp1 = r.place_on_field(1, "BEAST", Some(0));
    let opp2 = r.place_on_field(1, "BEAST", Some(0));
    r.game.suspend(veg);
    r.game.suspend(fairy);
    r.auto_resolve().ok();

    r.game.suspend(opp1);
    r.auto_resolve().ok();
    assert_eq!(r.memory(), 5, "+2 (VEG + FAIRY)");

    r.game.suspend(opp2);
    r.auto_resolve().ok();
    assert_eq!(r.memory(), 5, "OPT spent — no second gain this turn");
}

/// Negative: only an OPPONENT's Digimon suspending triggers.
#[test]
fn ex3_045_own_suspend_does_not_trigger() {
    let mut r = base().memory(3).start();
    let hydra = r.place_on_field(0, CARD_ID, Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    let fairy = r.place_on_field(0, "FAIRY", Some(0));
    r.game.suspend(veg);
    r.auto_resolve().ok();
    r.game.suspend(fairy);
    r.auto_resolve().ok();
    assert_eq!(r.memory(), 3, "own Digimon suspending never gains memory");
}

// ─── [End of Your Turn] ─────────────────────────────────────────────────────

/// 2+ own suspended matching Digimon (Hydramon counts here) → mandatory pick
/// among the opponent's SUSPENDED Digimon; the top card goes to the bottom of
/// the owner's deck and its digivolution cards are trashed (DCGO
/// DiscardEvoRoots).
#[test]
fn ex3_045_eot_returns_suspended_opponent_to_deck_bottom() {
    let mut r = base().memory(3).start();
    let hydra = r.place_on_field(0, CARD_ID, Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    let opp_free = r.place_on_field(1, "BEAST", Some(0));
    let opp_target = r.place_stack(1, &["OPP-SRC", "BEAST"]);
    r.game.suspend(hydra);
    r.game.suspend(veg);
    r.game.suspend(opp_target); // triggers the memory clause (+1: VEG) — irrelevant here
    r.auto_resolve().ok();
    let deck_before = r.deck_size(1);
    let trash_before = r.trash_size(1);

    r.end_turn();
    assert!(
        r.pending_selection().is_some(),
        "EoT bounce prompt installs (2 suspended matching Digimon)"
    );
    assert!(!r.pending_is_optional(), "mandatory return");
    let view = r.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "only the opponent's SUSPENDED Digimon is a candidate"
    );
    pick(&mut r, opp_target);
    r.auto_resolve().ok();

    assert_eq!(r.battle_area_size(1), 1, "suspended Digimon left the field");
    assert_eq!(r.deck_size(1), deck_before + 1, "top card only to deck");
    assert_eq!(
        r.game.players[1].deck[0].card_id(&r.game.card_data),
        "BEAST",
        "returned to the BOTTOM of the owner's deck"
    );
    assert_eq!(r.trash_size(1), trash_before + 1, "digivolution card trashed");
}

/// Negative: only 1 own suspended matching Digimon ([Beast] does not count) →
/// no bounce.
#[test]
fn ex3_045_eot_needs_two_matching_suspended() {
    let mut r = base().memory(3).start();
    let hydra = r.place_on_field(0, CARD_ID, Some(0));
    let beast = r.place_on_field(0, "BEAST", Some(0));
    let opp = r.place_on_field(1, "BEAST", Some(0));
    r.game.suspend(hydra);
    r.game.suspend(beast);
    r.game.suspend(opp);
    r.auto_resolve().ok();

    r.end_turn();
    r.auto_resolve().ok();
    assert_eq!(r.battle_area_size(1), 1, "opponent's Digimon stays");
    assert_eq!(r.turn_player(), 1, "turn passed without a bounce");
}
