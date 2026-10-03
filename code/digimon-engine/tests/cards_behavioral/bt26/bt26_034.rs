//! BT26-034 Palmon — Lv.3 Green Digimon (Vegetation / Iliad / TS).
//!
//! [Start of Your Main Phase] If you have 4 or less memory, this Digimon may
//! digivolve into a Digimon card with the [Vegetation] or [TS] trait in the
//! hand without paying the cost.
//! Inherited: [When Attacking] [Once Per Turn] You may suspend 1 of your
//! opponent's Digimon.
//!
//! DCGO: BT26/Green/BT26_034.cs.

use super::support::*;
use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-034";

/// Lv.4 Digimon that can digivolve from a green Lv.3 for `cost`.
fn evo(id: &str, traits: &[&str]) -> CardData {
    let mut c = digimon(id, id, CardColor::Green, 4, 5, traits);
    c.evo_costs = vec![EvoCost {
        card_color: 3,
        level: 3,
        memory_cost: 3,
    }];
    c
}

fn setup(hand: &[&str], memory: i16) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-034")
        .add_card(filler("FILLER"))
        .add_card(evo("VEG", &["Vegetation"]))
        .add_card(evo("TSD", &["TS"]))
        .add_card(evo("BEAST", &["Beast"]))
        // [TS] Lv.5 — no route onto a Lv.3 Palmon.
        .add_card(digimon("TS5", "TS5", CardColor::Green, 5, 7, &["TS"]))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("CARRIER", "Carrier", CardColor::Green, 4, 5, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .hand(0, hand)
        .memory(memory)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_034_metadata_and_ts_alt_path() {
    let r = setup(&[], 3);
    let c = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(c.level, Some(3));
    for t in ["Vegetation", "Iliad", "TS"] {
        assert!(c.traits.contains(&t.to_string()));
    }
    let p = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("alt digivolve");
    assert_eq!(p.cost, Some(CompiledCost::Literal(0)));
    assert!(format!("{:?}", p.from).contains("TS"));
}

#[test]
fn bt26_034_somp_free_digivolves_into_vegetation() {
    let mut r = setup(&["VEG"], 3);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.game.enter_main_phase();
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional, "may");
    pick_hand(&mut r, 0, "VEG");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["VEG".to_string()]);
    assert_eq!(sources(&r, h), 1);
    assert_eq!(r.memory(), 3, "without paying the cost");
}

#[test]
fn bt26_034_somp_accepts_ts_trait() {
    let mut r = setup(&["TSD"], 4);
    let _h = r.place_on_field(0, CARD_ID, Some(0));
    r.game.enter_main_phase();
    pick_hand(&mut r, 0, "TSD");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TSD".to_string()]);
}

#[test]
fn bt26_034_somp_decline_keeps_palmon() {
    let mut r = setup(&["VEG"], 3);
    r.place_on_field(0, CARD_ID, Some(0));
    r.game.enter_main_phase();
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
    assert!(hand_ids(&r, 0).contains(&"VEG".to_string()));
}

#[test]
fn bt26_034_somp_gated_by_memory() {
    let mut r = setup(&["VEG"], 5);
    r.place_on_field(0, CARD_ID, Some(0));
    r.game.enter_main_phase();
    assert!(r.pending_selection_view().is_none(), "5 memory → no effect");
}

#[test]
fn bt26_034_somp_requires_legal_digivolve_and_trait() {
    // Beast (wrong trait) and a Lv.5 [TS] (no digivolve route) only.
    let mut r = setup(&["BEAST", "TS5"], 3);
    r.place_on_field(0, CARD_ID, Some(0));
    r.game.enter_main_phase();
    assert!(r.pending_selection_view().is_none(), "no legal candidate");
}

#[test]
fn bt26_034_inherited_wa_suspends_opp_once_per_turn() {
    let mut r = setup(&[], 3);
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let a = r.place_on_field(1, "OPP", Some(0));
    let b = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    assert!(r.pending_is_optional(), "may");
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    let n = [a, b]
        .iter()
        .filter(|h| r.game.players[1].battle_area[h.index as usize].is_suspended)
        .count();
    assert_eq!(n, 1);
    // Once per turn.
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    assert!(
        r.pending_selection_view().is_none(),
        "already used this turn"
    );
}

#[test]
fn bt26_034_inherited_wa_decline_keeps_use() {
    let mut r = setup(&[], 3);
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let a = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    r.decline_optional_trigger().unwrap();
    assert!(!r.game.players[1].battle_area[a.index as usize].is_suspended);
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    assert!(
        r.pending_selection_view().is_some(),
        "declined use not consumed"
    );
}

#[test]
fn bt26_034_inherited_wa_needs_unsuspended_target() {
    let mut r = setup(&[], 3);
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let a = r.place_on_field(1, "OPP", Some(0));
    r.game.players[1].battle_area[a.index as usize].is_suspended = true;
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    assert!(r.pending_selection_view().is_none());
}
