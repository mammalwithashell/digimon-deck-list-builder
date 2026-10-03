//! BT26-012 Manekimon — Lv.4 Red/Yellow (Puppet / Shambala / TB).
//!
//! [Main] [Once Per Turn] You may play or use 1 [TB] trait card from your
//! hand with the cost reduced by 2.
//! Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
//! gets -2000 DP for the turn.
//!
//! DCGO: BT26/Red/BT26_012.cs.

use super::support::*;
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PLAY_HAND_START,
};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase};

const CARD_ID: &str = "BT26-012";

fn main_action(field_index: u8) -> u16 {
    FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-012")
        .add_card(filler("FILLER"))
        .add_card(digimon("TBMON", "TBmon", CardColor::Red, 4, 5, &["TB"]))
        .add_card(tamer("TBTAMER", "TB Tamer", CardColor::Red, &["TB"]))
        .add_card(digimon("PLAIN", "Plainmon", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("HOST", "Host", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("OPP4", "Opp4", CardColor::Blue, 4, 5, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    r
}

fn mask_has(r: &DebugRunner, a: u16) -> bool {
    build_action_mask(&r.game, 0)[a as usize] == 1.0
}

#[test]
fn bt26_012_alt_path_shambala() {
    let r = setup();
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("Shambala") && alt.contains("level_eq: Some(3)"),
        "{alt}"
    );
}

#[test]
fn bt26_012_main_plays_tb_digimon_with_cost_reduced_by_2() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    push_hand(&mut r, 0, "TBMON");
    let m = r.place_on_field(0, CARD_ID, Some(0));
    let a = main_action(m.index);
    assert!(mask_has(&r, a));
    let mem = r.memory();
    r.game.decode_action(a, 0);
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional);
    assert!(
        !v.valid_action_ids.contains(&PLAY_HAND_START),
        "PLAIN is not [TB]"
    );
    pick_hand(&mut r, 0, "TBMON");
    assert_eq!(field_ids(&r, 0), vec![CARD_ID, "TBMON"]);
    assert_eq!((mem - r.memory()).abs(), 3, "cost 5 - 2");
    // Once per turn.
    push_hand(&mut r, 0, "TBTAMER");
    assert!(!mask_has(&r, a));
}

#[test]
fn bt26_012_main_plays_tb_tamer() {
    let mut r = setup();
    push_hand(&mut r, 0, "TBTAMER");
    let m = r.place_on_field(0, CARD_ID, Some(0));
    let mem = r.memory();
    r.game.decode_action(main_action(m.index), 0);
    pick_hand(&mut r, 0, "TBTAMER");
    assert_eq!(field_ids(&r, 0), vec![CARD_ID, "TBTAMER"]);
    assert_eq!((mem - r.memory()).abs(), 1, "cost 3 - 2");
}

#[test]
fn bt26_012_declining_refunds_once_per_turn() {
    let mut r = setup();
    push_hand(&mut r, 0, "TBMON");
    let m = r.place_on_field(0, CARD_ID, Some(0));
    let a = main_action(m.index);
    r.game.decode_action(a, 0);
    pass(&mut r, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(hand_ids(&r, 0), vec!["TBMON"]);
    assert!(mask_has(&r, a), "declined → OPT not spent (DCGO RemoveUse)");
}

#[test]
fn bt26_012_main_unavailable_without_tb_card() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    let m = r.place_on_field(0, CARD_ID, Some(0));
    assert!(!mask_has(&r, main_action(m.index)));
}

#[test]
fn bt26_012_inherited_when_attacking_minus_2000_once_per_turn() {
    let mut r = setup();
    let opp = r.place_on_field(1, "OPP4", Some(0));
    let c = r.place_stack(0, &[CARD_ID, "HOST"]);
    fire(&mut r, EffectTiming::WhenAttacking, c);
    if r.pending_selection_view().is_some() {
        pick_first(&mut r, 0);
    }
    assert_eq!(r.effective_dp(opp), Some(2000));
    fire(&mut r, EffectTiming::WhenAttacking, c);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.effective_dp(opp), Some(2000), "once per turn");
    r.end_turn();
    assert_eq!(r.effective_dp(opp), Some(4000), "for the turn");
}
