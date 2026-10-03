//! BT26-033 Jupitermon — DUAL (Lv.6 Digimon // Option).
//!
//! <Raid> <Alliance> <Engage>
//! [When Digivolving] Add your top security card to the hand. Then, if it's
//! your turn, you may play or use 1 [Iliad] or [TS] trait card from your hand
//! with the cost reduced by 5.
//! [All Turns] When any of your [TS] trait Digimon or Tamers would leave the
//! battle area, by placing this Digimon's top stacked card as the bottom
//! security card, they don't leave.
//! Option face: use cost +1 per own security card
//! (G-ENGINE-OPTION-SELF-USE-COST-INCREASE); [Main] delete all opponent's
//! lowest-DP Digimon, then <Recovery +1>; <Arts Digivolve>.
//!
//! DCGO: BT26/Yellow/BT26_033.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT26-033";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-033")
        .add_card(filler("FILLER"))
        .add_card(digimon("TS5", "Ts Five", CardColor::Yellow, 5, 7, &["TS"]))
        .add_card(digimon(
            "ILIAD",
            "Iliad Guy",
            CardColor::Yellow,
            5,
            7,
            &["Iliad"],
        ))
        .add_card(digimon("PLAIN", "Plain", CardColor::Yellow, 4, 5, &[]))
        .add_card(tamer("TS-T", "Ts Tamer", CardColor::Yellow, &["TS"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER", "FILLER", "PLAIN"])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| {
            v.valid_action_ids
                .into_iter()
                .filter(|&x| x != digimon_engine::action::space::PASS)
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn bt26_033_keywords_and_alt_path() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    let effs = format!("{:?}", card.effects);
    for kw in ["Raid", "Alliance", "Engage"] {
        assert!(effs.contains(kw), "missing {kw}");
    }
    assert!(format!("{:?}", card.alt_paths).contains("TS"));
}

#[test]
fn bt26_033_wd_adds_top_security_then_plays_iliad_reduced_by_5() {
    let mut r = setup();
    push_hand(&mut r, 0, "ILIAD");
    let j = r.place_stack(0, &["TS5", CARD_ID]);
    let mem = r.memory();
    fire(&mut r, EffectTiming::WhenDigivolving, j);
    assert_eq!(r.security_count(0), 2, "top security removed");
    assert!(
        hand_ids(&r, 0).contains(&"PLAIN".to_string()),
        "added to hand"
    );
    let v = r.pending_selection_view().expect("may play/use prompt");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "ILIAD");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"ILIAD".to_string()));
    assert_eq!(r.memory(), mem - 2, "cost 7 reduced by 5");
}

#[test]
fn bt26_033_wd_play_is_optional() {
    let mut r = setup();
    push_hand(&mut r, 0, "ILIAD");
    let j = r.place_stack(0, &["TS5", CARD_ID]);
    fire(&mut r, EffectTiming::WhenDigivolving, j);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&"ILIAD".to_string()));
    assert_eq!(r.security_count(0), 2);
}

#[test]
fn bt26_033_wd_non_iliad_cards_are_not_offered() {
    let mut r = setup();
    // Only PLAIN (no Iliad/TS) will be in hand after the security add.
    let j = r.place_stack(0, &["TS5", CARD_ID]);
    fire(&mut r, EffectTiming::WhenDigivolving, j);
    assert!(non_pass(&r).is_empty(), "no Iliad/TS candidate");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["PLAIN".to_string()]);
}

#[test]
fn bt26_033_protects_ts_permanent_by_placing_top_card_to_bottom_security() {
    let mut r = setup();
    let j = r.place_stack(0, &["TS5", CARD_ID]);
    let t = r.place_on_field(0, "TS-T", Some(0));
    r.game
        .delete_permanents_batch(vec![t], ReplacementCause::OpponentEffect);
    let v = r.pending_selection_view().expect("replacement prompt");
    assert!(v.is_optional, "by placing … (may)");
    let accept = non_pass(&r)[0];
    r.execute_action(0, accept).unwrap();
    let _ = r.auto_resolve();
    let ids = field_ids(&r, 0);
    assert!(
        ids.contains(&"TS-T".to_string()),
        "Tamer didn't leave: {ids:?}"
    );
    assert!(
        ids.contains(&"TS5".to_string()),
        "Jupitermon de-stacked to TS5"
    );
    assert!(!ids.contains(&CARD_ID.to_string()));
    assert_eq!(r.security_count(0), 4);
    let bottom = &r.game.players[0].security[0];
    assert_eq!(
        bottom.card_id(&r.game.card_data),
        CARD_ID,
        "bottom security"
    );
    assert!(
        !r.game.players[0]
            .face_up_security
            .contains(&bottom.card_index),
        "face down"
    );
    let _ = j;
}

#[test]
fn bt26_033_protection_declined_lets_it_leave() {
    let mut r = setup();
    r.place_stack(0, &["TS5", CARD_ID]);
    let t = r.place_on_field(0, "TS-T", Some(0));
    r.game
        .delete_permanents_batch(vec![t], ReplacementCause::OpponentEffect);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"TS-T".to_string()));
    assert_eq!(r.security_count(0), 3);
}

#[test]
fn bt26_033_no_protection_without_stacked_cards_or_for_non_ts() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0)); // no digivolution cards
    let t = r.place_on_field(0, "TS-T", Some(0));
    r.game
        .delete_permanents_batch(vec![t], ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_none(),
        "unpayable ⇒ not offered"
    );
    assert!(!field_ids(&r, 0).contains(&"TS-T".to_string()));

    let mut r = setup();
    r.place_stack(0, &["TS5", CARD_ID]);
    let p = r.place_on_field(0, "PLAIN", Some(0));
    r.game
        .delete_permanents_batch(vec![p], ReplacementCause::OpponentEffect);
    assert!(r.game.pending_selection.is_none(), "non-[TS] not protected");
}

#[test]
fn bt26_033_option_use_cost_grows_with_own_security() {
    // Use cost 2 + 1 per own security card (3) = 5.
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    let card = r.game.players[0].hand.last().unwrap().clone();
    assert_eq!(r.game.option_use_cost(&card, 0), 5);
    r.game.players[0].security.pop();
    assert_eq!(r.game.option_use_cost(&card, 0), 4, "re-read live");
}

#[test]
fn bt26_033_option_unaffordable_at_the_increased_cost() {
    // Memory -7: the printed 2 would end at -9 (legal), but 2 + 3 security
    // ends at -12, past the -10 floor.
    let mut r = setup();
    r.game.memory = -7;
    r.place_on_field(0, "TS-T", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    r.game.enter_main_phase();
    let res = r.game.play_option_from_hand(0, idx);
    assert_eq!(format!("{res:?}"), "Invalid");
    assert!(hand_ids(&r, 0).contains(&CARD_ID.to_string())); // Control: with no security cards the cost is the printed 2 → usable.
    r.game.players[0].security.clear();
    let idx = r.game.players[0].hand.len() - 1;
    let res = r.game.play_option_from_hand(0, idx);
    assert_ne!(format!("{res:?}"), "Invalid");
}

#[test]
fn bt26_033_option_main_deletes_all_lowest_dp_then_recovers() {
    let mut r = setup();
    r.game.memory = 10;
    r.place_on_field(0, "TS-T", Some(0));
    r.place_on_field(1, "PLAIN", Some(0)); // 4000 DP
    r.place_on_field(1, "PLAIN", Some(0)); // 4000 DP (tie: both deleted)
    r.place_on_field(1, "TS5", Some(0)); // 5000 DP
    push_hand(&mut r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["TS5".to_string()]);
    assert_eq!(r.security_count(0), 4, "<Recovery +1>");
    assert_eq!(r.memory(), 10 - 5, "paid 2 + 3 security");
}
