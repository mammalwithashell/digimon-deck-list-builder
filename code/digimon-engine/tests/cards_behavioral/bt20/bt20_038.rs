//! BT20-038 Falcomon — Digimon, Lv.3, Green/Yellow, DP 1000, Cost 3 (Avian,
//! ACCEL).
//!
//! Printed (card image + official Bandai DB bundle + DCGO BT20_038.cs):
//! - Evo box: Lv.2 Green/Yellow split circle / cost 1 (NO Lv.3 circle);
//!   "Digivolve [Pinamon] or Lv.2 w/[ACCEL] trait: Cost 0".
//! - [Your Turn] When this Digimon would digivolve into a Digimon card with the
//!   [ACCEL] trait, reduce the digivolution cost by 1.
//! - Inherited: <Piercing>.
//!
//! Regression (hunt cluster alt-digivolve-xros-req-cards): no YAML spec, so the
//! cost-0 route lived only in `xros_req` (a Green/Yellow Lv.2 [ACCEL] base got
//! no cost-choice prompt; a Pinamon / off-colour ACCEL base could not digivolve).
//! Synthetic boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::{build_action_mask, space::encode_digivolve};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, Keyword};

const ID: &str = "BT20-038";

#[test]
fn bt20_038_green_or_yellow_lv2_circle_costs_1() {
    assert_single_route(ID, base("G2", "Green Rookie", 2, &[GREEN], &["Lesser"]), 1);
    assert_single_route(ID, base("Y2", "Yellow Rookie", 2, &[YELLOW], &["Lesser"]), 1);
}

#[test]
fn bt20_038_accel_lv2_of_any_colour_costs_0() {
    assert_single_route(ID, base("A2", "Red Accel", 2, &[RED], &["ACCEL"]), 0);
}

#[test]
fn bt20_038_pinamon_by_name_costs_0() {
    assert_single_route(ID, base("PINA", "Pinamon", 2, &[BLUE], &["Bird"]), 0);
}

#[test]
fn bt20_038_green_accel_lv2_prompts_cost_0_vs_1() {
    assert_cost_choice(ID, base("GA2", "Green Accel", 2, &[GREEN], &["ACCEL"]), 0, 1);
    assert_cost_choice(ID, base("YP", "Pinamon", 2, &[YELLOW], &["Bird"]), 0, 1);
}

#[test]
fn bt20_038_has_no_lv3_circle_and_rejects_other_bases() {
    // The old data carried a bogus {green, Lv.3, cost 1} circle; the card image
    // and the official DB print only the Lv.2 split circle.
    assert_not_offered(ID, base("G3", "Green Champion", 3, &[GREEN], &["Lesser"]));
    // Lv.2 non-ACCEL off-colour, and a Lv.3 ACCEL: no route.
    assert_not_offered(ID, base("B2", "Blue Rookie", 2, &[BLUE], &["Lesser"]));
    assert_not_offered(ID, base("A3", "Accel Champion", 3, &[RED], &["ACCEL"]));
}

// ─── [Your Turn] digivolution cost -1 into an [ACCEL] Digimon ─────────────────

fn accel_runner() -> DebugRunner {
    let mut plain = make_test_card("PLAIN", "Plain Rookie");
    plain.level = Some(3);
    plain.colors = vec![CardColor::Green];
    let mut targets = vec![];
    for (id, name, traits) in [
        ("ACCEL4", "Accel Champion", vec!["ACCEL"]),
        ("PLAIN4", "Plain Champion", vec!["Beast"]),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(4);
        c.dp = Some(5000);
        c.traits = traits.into_iter().map(str::to_owned).collect();
        c.evo_costs = vec![EvoCost {
            level: 3,
            card_color: 3,
            memory_cost: 3,
        }];
        targets.push(c);
    }
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT20-038 production YAML loads")
        .add_card(plain);
    for c in targets {
        b = b.add_card(c);
    }
    b.hand(0, &["ACCEL4", "PLAIN4"])
        .deck(0, &["PLAIN"; 12])
        .deck(1, &["PLAIN"; 12])
        .memory(10)
        .start()
}

fn top(r: &DebugRunner, h: digimon_engine::permanent::PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

#[test]
fn bt20_038_reduces_only_digivolutions_into_accel_digimon_by_1() {
    for (hand, expected_top, expected_cost) in [(0u16, "ACCEL4", 2), (1, "PLAIN4", 3)] {
        let mut r = accel_runner();
        r.skip_mulligan();
        let h = r.place_on_field(0, ID, Some(0));
        let action = encode_digivolve(hand, h.index as u16);
        assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
        r.game.decode_action(action, 0);
        assert_eq!(top(&r, h), expected_top);
        assert_eq!(r.memory(), 10 - expected_cost, "{expected_top}");
    }
}

#[test]
fn bt20_038_reduction_does_not_apply_from_the_inherited_position() {
    let mut r = accel_runner();
    r.skip_mulligan();
    // Falcomon is only a digivolution source here: its face-up [Your Turn]
    // clause is inactive, so the ACCEL digivolution pays the full 3.
    let h = r.place_stack(0, &[ID, "PLAIN"]);
    let action = encode_digivolve(0, h.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    assert_eq!(top(&r, h), "ACCEL4");
    assert_eq!(r.memory(), 7);
}

#[test]
fn bt20_038_inherited_piercing_reaches_the_host() {
    let mut r = accel_runner();
    let host = r.place_stack(0, &[ID, "PLAIN"]);
    assert!(r.game.has_keyword(host, Keyword::Piercing));
    let top_only = r.place_on_field(0, ID, Some(1));
    assert!(
        !r.game.has_keyword(top_only, Keyword::Piercing),
        "<Piercing> is inherited text: only active from under another card"
    );
}
