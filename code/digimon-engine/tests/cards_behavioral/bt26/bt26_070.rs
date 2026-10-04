//! BT26-070 NightChiropmon — Digimon Lv.4, Purple, DP 5000, Cost 5.
//! Traits: Beastkin, Glowing Dawn, BEATBREAK.
//! Digivolve: Purple Lv.3 / cost 2; [Digivolve] Lv.3 w/[Glowing Dawn]: cost 2.
//!
//! [On Play] [When Digivolving] <Draw 1> and trash 1 card in your hand.
//! [Main] [Once Per Turn] By trashing 2 bottom face-down cards from under any of
//! your Tamers, you may use 1 Option card with the [Glowing Dawn] trait from
//! your trash with the cost reduced by 2.
//! Inherited: <Retaliation>.
//! Official Q&A: the [Main] can't be activated with only 1 face-down card.
//!
//! DCGO: BT26/Purple/BT26_070.cs.

use super::support::*;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-070";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-070")
        .from_dsl_yaml(GD_DRAW_OPTION_YAML)
        .expect("gd option")
        .add_card(filler("FILLER"))
        .add_card(tamer(
            "GDT",
            "GD Tamer",
            CardColor::Yellow,
            &["Glowing Dawn"],
        ))
        .add_card({
            let mut o = gd_option("PLAIN-OPT", 5);
            o.traits.clear();
            o
        })
        .add_card(digimon("HOST", "Host", CardColor::Purple, 5, 5, &[]))
        .add_card(digimon("A", "A", CardColor::Purple, 3, 3, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

fn main_bit(field_index: usize) -> u16 {
    FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

fn main_legal(r: &DebugRunner, field_index: usize) -> bool {
    build_action_mask(&r.game, 0)[main_bit(field_index) as usize] > 0.0
}

fn total_face_down(r: &DebugRunner, tamers: &[PermanentHandle]) -> usize {
    tamers.iter().map(|&t| sources(r, t)).sum()
}

/// Answer the trash-2-face-down cost picks (Tamer picks) until both are gone.
fn pay_face_down(r: &mut DebugRunner, tamers: &[PermanentHandle], before: usize) {
    for _ in 0..4 {
        if total_face_down(r, tamers) + 2 <= before {
            return;
        }
        pick_first(r, 0);
    }
}

// ─── [On Play] / [When Digivolving] ─────────────────────────────────────────

#[test]
fn bt26_070_on_play_draws_then_trashes_one() {
    let mut r = setup();
    push_hand(&mut r, 0, "A");
    let n = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, n);
    assert_eq!(r.hand_size(0), 2, "<Draw 1>");
    let v = r.pending_selection_view().expect("hand trash pick");
    assert!(!v.is_optional, "trashing 1 is mandatory");
    pick_hand(&mut r, 0, "A");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["FILLER".to_string()]);
    assert_eq!(trash_ids(&r, 0), vec!["A".to_string()]);
}

#[test]
fn bt26_070_when_digivolving_draws_then_trashes_one() {
    let mut r = setup();
    let n = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, n);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 0, "drew 1, trashed 1");
    assert_eq!(r.trash_size(0), 1);
}

// ─── [Main] [Once Per Turn] ─────────────────────────────────────────────────

#[test]
fn bt26_070_main_trashes_two_face_down_and_uses_option_from_trash() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 2);
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "T-GDOPT");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    let mem = r.memory();
    let hand = r.hand_size(0);
    assert!(main_legal(&r, 1), "[Main] offered");
    r.game.decode_action(main_bit(1), 0);
    pay_face_down(&mut r, &[t], 2);
    assert_eq!(sources(&r, t), 0, "2 face-down cards trashed");
    let v = r.pending_selection_view().expect("trash Option pick");
    assert!(v.is_optional, "'you may use'");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1, "the Option's [Main] <Draw 1> ran");
    assert_eq!(r.memory(), mem - 3, "use cost 5 - 2 = 3");
    assert!(!main_legal(&r, 1), "[Once Per Turn]");
}

#[test]
fn bt26_070_main_spreads_cost_across_two_tamers() {
    let mut r = setup();
    let t1 = tamer_with_face_down(&mut r, 0, "GDT", 1);
    let t2 = tamer_with_face_down(&mut r, 0, "GDT", 1);
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "T-GDOPT");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    assert!(main_legal(&r, 2), "1 + 1 face-down cards pay the cost");
    r.game.decode_action(main_bit(2), 0);
    pay_face_down(&mut r, &[t1, t2], 2);
    assert_eq!(sources(&r, t1) + sources(&r, t2), 0);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 5 - 3);
}

#[test]
fn bt26_070_main_needs_two_face_down_cards() {
    let mut r = setup();
    let _t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "T-GDOPT");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    assert!(!main_legal(&r, 1), "Q&A: only 1 face-down card can't pay");
}

#[test]
fn bt26_070_main_option_use_may_be_declined_after_paying() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 2);
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "T-GDOPT");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    let mem = r.memory();
    r.game.decode_action(main_bit(1), 0);
    pay_face_down(&mut r, &[t], 2);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0, "cost stays paid");
    assert_eq!(r.memory(), mem);
    assert!(trash_ids(&r, 0).contains(&"T-GDOPT".to_string()));
}

#[test]
fn bt26_070_main_only_glowing_dawn_options() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 2);
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "PLAIN-OPT");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    let mem = r.memory();
    r.game.decode_action(main_bit(1), 0);
    pay_face_down(&mut r, &[t], 2);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem, "no eligible Option ⇒ nothing used");
    assert!(trash_ids(&r, 0).contains(&"PLAIN-OPT".to_string()));
}

// ─── Inherited <Retaliation> ────────────────────────────────────────────────

#[test]
fn bt26_070_inherited_retaliation() {
    let mut r = setup();
    let h = r.place_stack(0, &[CARD_ID, "HOST"]);
    assert!(r.game.has_keyword(h, Keyword::Retaliation));
    let own = r.place_on_field(0, CARD_ID, Some(0));
    assert!(
        !r.game.has_keyword(own, Keyword::Retaliation),
        "inherited only"
    );
}
