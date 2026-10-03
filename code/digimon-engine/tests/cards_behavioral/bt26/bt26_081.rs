//! BT26-081 Mervamon — Lv.6 Purple/Yellow/Black.
//!
//! [On Play] [When Digivolving] You may play up to 8 play cost's total worth of
//! [Iliad] trait cards from your hand or trash without paying the costs. Then,
//! to 1 of your opponent's Digimon, give -4000 DP until their turn ends for each
//! of your [Iliad] or [TS] trait Digimon or Tamers.
//! [All Turns] All of your [Iliad] trait Digimon gain <Alliance>, <Reboot>,
//! <Blocker> and +2000 DP.
//!
//! DCGO: BT26/*/BT26_081.cs. G-ENGINE-PLAY-COST-BUDGET-FROM-HAND-OR-TRASH.

use super::support::*;
use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase, Keyword};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT26-081";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-081")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "IL3",
            "Iliad Three",
            CardColor::Yellow,
            3,
            3,
            &["Iliad"],
        ))
        .add_card(digimon(
            "IL5",
            "Iliad Five",
            CardColor::Yellow,
            4,
            5,
            &["Iliad"],
        ))
        .add_card(digimon(
            "IL6",
            "Iliad Six",
            CardColor::Yellow,
            5,
            6,
            &["Iliad"],
        ))
        .add_card(digimon("TS4", "TS Four", CardColor::Yellow, 4, 4, &["TS"]))
        .add_card(digimon("PLAIN", "Plain", CardColor::Yellow, 3, 2, &[]))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 9, 9, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r.game.players[0].hand.clear();
    r
}

fn hand_action(r: &DebugRunner, id: &str) -> u16 {
    let i = hand_ids(r, 0).iter().position(|h| h == id).unwrap();
    PLAY_HAND_START + i as u16
}

fn trash_action(r: &DebugRunner, id: &str) -> u16 {
    let i = trash_ids(r, 0).iter().position(|h| h == id).unwrap();
    TRASH_EFFECT_START + i as u16
}

fn budget_remaining(r: &DebugRunner) -> Option<i32> {
    match r.game.pending_selection.as_ref().map(|s| &s.kind) {
        Some(SelectionKind::PlayCostBudget {
            remaining_play_cost,
            ..
        }) => Some(*remaining_play_cost),
        _ => None,
    }
}

#[test]
fn bt26_081_plays_up_to_8_cost_from_hand_and_trash_together() {
    let mut r = setup();
    push_hand(&mut r, 0, "IL3");
    push_hand(&mut r, 0, "IL6");
    push_hand(&mut r, 0, "PLAIN");
    push_trash(&mut r, 0, "IL5");
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::OnPlay, me);

    // One prompt over BOTH zones, under the 8-cost budget; "may" ⇒ PASS.
    assert_eq!(budget_remaining(&r), Some(8));
    assert_eq!(r.game.current_phase, GamePhase::SelectBudgeted);
    let v = r.pending_selection_view().unwrap();
    assert!(v.is_optional);
    for id in [
        hand_action(&r, "IL3"),
        hand_action(&r, "IL6"),
        trash_action(&r, "IL5"),
    ] {
        assert!(v.valid_action_ids.contains(&id));
    }
    assert!(
        !v.valid_action_ids.contains(&hand_action(&r, "PLAIN")),
        "not [Iliad]"
    );

    r.execute_action(0, hand_action(&r, "IL3")).unwrap();
    assert_eq!(budget_remaining(&r), Some(5));
    let v = r.pending_selection_view().unwrap();
    assert!(
        !v.valid_action_ids.contains(&hand_action(&r, "IL6")),
        "6 > 5 left"
    );
    r.execute_action(0, trash_action(&r, "IL5")).unwrap();
    // Budget spent → both play at once, then the mandatory DP pick.
    let field = field_ids(&r, 0);
    assert!(field.contains(&"IL3".to_string()) && field.contains(&"IL5".to_string()));
    assert!(hand_ids(&r, 0).contains(&"IL6".to_string()));
    drain_first(&mut r);
    let _ = r.auto_resolve();
}

#[test]
fn bt26_081_dp_debuff_scales_and_applies_even_if_nothing_is_played() {
    let mut r = setup();
    push_hand(&mut r, 0, "IL3");
    let me = r.place_on_field(0, CARD_ID, Some(0)); // itself [Iliad]/[TS]
    r.place_on_field(0, "TS4", Some(0));
    r.place_on_field(0, "PLAIN", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    let before = r.effective_dp(opp).unwrap();
    fire(&mut r, EffectTiming::OnPlay, me);
    r.execute_action(0, PASS).unwrap(); // play nothing
    let v = r.pending_selection_view().expect("mandatory DP pick");
    assert!(!v.is_optional);
    drain_first(&mut r);
    let _ = r.auto_resolve();
    // Mervamon + TS4 = 2 → -8000.
    assert_eq!(r.effective_dp(opp).unwrap(), before - 8000);
    assert!(hand_ids(&r, 0).contains(&"IL3".to_string()));
}

#[test]
fn bt26_081_all_turns_aura_buffs_own_iliad_digimon() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    let il = r.place_on_field(0, "IL3", Some(0));
    let plain = r.place_on_field(0, "PLAIN", Some(0));
    r.game.tick_declarative_effects();
    for kw in [Keyword::Alliance, Keyword::Reboot, Keyword::Blocker] {
        assert!(r.game.has_keyword(il, kw), "{kw:?}");
        assert!(!r.game.has_keyword(plain, kw));
    }
    assert_eq!(r.effective_dp(il).unwrap(), 3000 + 2000);
    assert_eq!(r.effective_dp(plain).unwrap(), 3000);
}

#[test]
fn bt26_081_alt_paths_and_assembly() {
    let r = setup();
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("Minervamon") && alt.contains("Assembly"),
        "{alt}"
    );
}
