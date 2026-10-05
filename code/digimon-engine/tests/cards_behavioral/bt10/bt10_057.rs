//! BT10-057 BloomLordmon — Digimon, Lv.6, Green, DP 12000, Cost 12.
//! Traits: Fairy. Form: Mega. Attribute: Vaccine. Digivolve: Green Lv.5 / 4.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT10-057.md)
//! [When Digivolving] You may suspend 1 of your Digimon. Then, gain 1 memory
//! for each of your suspended Digimon with [Vegetation], [Plant] or [Fairy] in
//! any of their traits. If this effect gains 2 or more memory, this Digimon
//! unsuspends and gains <Piercing> for the turn.
//! [Your Turn] For every 2 of your suspended Digimon, this Digimon gains
//! <Security A. +1> and +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT10/Green/BT10_057.cs
//! - WD count = own suspended Plant/Fairy-trait Digimon, THIS ONE INCLUDED.
//! - [Your Turn] count = own suspended Digimon (no trait filter, incl. self) / 2.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API 4.3)
//! - Real digivolve flow (encode_digivolve) → [When Digivolving]
//! - Optional own-permanent suspend pick (PASS exposed)
//! - gain_memory formula (card_count_in_zone w/ trait_contains any_of)
//! - Conditional unsuspend-self + keyword grant for the turn
//! - Self aura with formula (dp_modifier_fn multiply/floor_div,
//!   base-inclusive security_attack_fn), your_turn gate

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT10-057";

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
        .expect("BT10-057 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("GREEN-LV5", 5, &["Beast"]))
        .add_card(make_digimon("VEG", 4, &["Vegetation"]))
        .add_card(make_digimon("PLANT", 4, &["Plant"]))
        .add_card(make_digimon("FAIRY", 4, &["Fairy"]))
        .add_card(make_digimon("BEAST", 4, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

fn is_suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn pick_own(r: &mut DebugRunner, h: PermanentHandle) {
    let player = r.pending_selection().unwrap().selecting_player;
    r.execute_action(player, encode_attack(0, h.index as u16))
        .expect("pick own permanent");
}

fn pass(r: &mut DebugRunner) {
    let player = r.pending_selection().unwrap().selecting_player;
    r.execute_action(player, PASS).expect("PASS");
}

/// Digivolve BloomLordmon (hand idx 0) onto `host` through the real action.
fn digivolve_onto(r: &mut DebugRunner, host: PermanentHandle) {
    r.game.enter_main_phase();
    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[action as usize], 1.0, "Green Lv.5 → BloomLordmon is legal");
    r.game.decode_action(action, 0);
}

// ─── Structural ─────────────────────────────────────────────────────────────

#[test]
fn bt10_057_metadata() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "BloomLordmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert!(card.traits.contains(&"Fairy".to_string()));
    assert_eq!(card.alt_paths.len(), 1, "single printed circle Green Lv.5 / 4");
}

#[test]
fn bt10_057_has_wd_clause_and_two_your_turn_auras() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let wd = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .expect("WD clause");
    assert!(wd.process.iter().any(|s| matches!(s, CompiledStep::GainMemory { .. })));
    let auras = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })))
        .count();
    assert_eq!(auras, 2, "DP aura + Security A. aura");
}

// ─── [When Digivolving] ─────────────────────────────────────────────────────

/// The suspend pick is optional and offers only YOUR unsuspended Digimon.
#[test]
fn bt10_057_wd_prompt_is_optional_own_digimon_only() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let host = r.place_on_field(0, "GREEN-LV5", Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    let opp = r.place_on_field(1, "BEAST", Some(0));
    digivolve_onto(&mut r, host);

    assert!(matches!(r.pending_kind(), Some(SelectionKind::OwnField)));
    let view = r.pending_selection_view().unwrap();
    assert!(view.is_optional, "you MAY suspend");
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        2,
        "BloomLordmon itself + own VEG; never the opponent's Digimon"
    );
}

/// Suspend 1 [Plant] Digimon: 1 matching suspended Digimon → +1 memory; below
/// the 2-memory threshold BloomLordmon gets no Piercing.
#[test]
fn bt10_057_wd_gain_one_memory_no_piercing() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let host = r.place_on_field(0, "GREEN-LV5", Some(0));
    let plant = r.place_on_field(0, "PLANT", Some(0));
    digivolve_onto(&mut r, host);
    let after_cost = r.memory();
    assert_eq!(after_cost, 6, "digivolve cost 4");

    pick_own(&mut r, plant);
    r.auto_resolve().ok();

    assert!(is_suspended(&r, plant));
    assert_eq!(r.memory(), after_cost + 1, "1 suspended [Plant] → +1 memory");
    assert!(!r.game.has_keyword(host, Keyword::Piercing), "gained < 2 → no Piercing");
}

/// Suspend BloomLordmon itself with a [Vegetation] already suspended: it counts
/// itself ([Fairy]) → +2 memory → it unsuspends and gains Piercing.
#[test]
fn bt10_057_wd_gain_two_unsuspends_self_and_grants_piercing() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let host = r.place_on_field(0, "GREEN-LV5", Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    r.game.suspend(veg);
    digivolve_onto(&mut r, host);
    let after_cost = r.memory();

    pick_own(&mut r, host);
    r.auto_resolve().ok();

    assert_eq!(r.memory(), after_cost + 2, "VEG + BloomLordmon ([Fairy]) → +2");
    assert!(!is_suspended(&r, host), "gained 2+ → BloomLordmon unsuspends");
    assert!(r.game.has_keyword(host, Keyword::Piercing), "gains <Piercing>");

    // "for the turn": Piercing is gone on the opponent's turn.
    r.end_turn();
    r.auto_resolve().ok();
    assert!(!r.game.has_keyword(host, Keyword::Piercing), "Piercing expires at turn end");
}

/// Declining the suspend still gains memory for already-suspended matching
/// Digimon; non-matching suspended Digimon ([Beast]) and the opponent's
/// suspended Digimon never count. Substring trait match: [Fairy] counts.
#[test]
fn bt10_057_wd_decline_counts_only_own_matching_suspended() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let host = r.place_on_field(0, "GREEN-LV5", Some(0));
    let beast = r.place_on_field(0, "BEAST", Some(0));
    let fairy = r.place_on_field(0, "FAIRY", Some(0));
    let opp = r.place_on_field(1, "VEG", Some(0));
    r.game.suspend(beast);
    r.game.suspend(fairy);
    r.game.suspend(opp);
    digivolve_onto(&mut r, host);
    let after_cost = r.memory();

    pass(&mut r);
    r.auto_resolve().ok();

    assert_eq!(r.memory(), after_cost + 1, "only own suspended FAIRY counts");
    assert!(!r.game.has_keyword(host, Keyword::Piercing));
}

// ─── [Your Turn] aura ───────────────────────────────────────────────────────

#[test]
fn bt10_057_your_turn_aura_scales_per_two_own_suspended() {
    let mut r = base().start();
    let bloom = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(0, "BEAST", Some(0));
    let b = r.place_on_field(0, "BEAST", Some(0));
    let c = r.place_on_field(0, "BEAST", Some(0));
    let opp1 = r.place_on_field(1, "BEAST", Some(0));
    let opp2 = r.place_on_field(1, "BEAST", Some(0));
    r.game.suspend(opp1);
    r.game.suspend(opp2);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(bloom), Some(12000), "0 own suspended (opp's don't count)");
    assert_eq!(r.game.effective_security_strike(bloom), 1);

    r.game.suspend(a);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(bloom), Some(12000), "1 suspended → floor(1/2)=0");
    assert_eq!(r.game.effective_security_strike(bloom), 1);

    r.game.suspend(b);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(bloom), Some(14000), "2 suspended → +2000");
    assert_eq!(r.game.effective_security_strike(bloom), 2, "2 suspended → Security A.+1");

    r.game.suspend(c);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(bloom), Some(14000), "3 suspended → still +2000");
    assert_eq!(r.game.effective_security_strike(bloom), 2);

    // BloomLordmon itself counts (DCGO GetBattleAreaDigimons incl. self).
    r.game.suspend(bloom);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(bloom), Some(16000), "4 suspended → +4000");
    assert_eq!(r.game.effective_security_strike(bloom), 3, "4 suspended → Security A.+2");
}

#[test]
fn bt10_057_aura_inactive_on_opponents_turn() {
    let mut r = base().start();
    let bloom = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(0, "BEAST", Some(0));
    let b = r.place_on_field(0, "BEAST", Some(0));
    r.game.suspend(a);
    r.game.suspend(b);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(bloom), Some(14000));

    r.end_turn();
    r.auto_resolve().ok();
    assert_eq!(r.turn_player(), 1);
    assert!(is_suspended(&r, a) && is_suspended(&r, b), "ours stay suspended");
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(bloom), Some(12000), "[Your Turn] only");
    assert_eq!(r.game.effective_security_strike(bloom), 1);
}
