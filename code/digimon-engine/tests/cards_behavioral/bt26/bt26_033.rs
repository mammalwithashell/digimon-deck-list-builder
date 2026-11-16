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
        // Play cost 18 / 19 reduced by 5 at memory 3: 13 → -10 (payable),
        // 14 → -11 (not payable).
        .add_card(digimon("ILIAD-18", "Iliad 18", CardColor::Yellow, 7, 18, &["Iliad"]))
        .add_card(digimon("ILIAD-19", "Iliad 19", CardColor::Yellow, 7, 19, &["Iliad"]))
        // Tokomon (BT25-001): an [Iliad]/[TS] Digi-Egg.
        .dsl_card("BT25-001")
        .expect("BT25-001 Tokomon (Digi-Egg)")
        // [Iliad] Options: yellow (requirement met by Jupitermon) and purple
        // (unmet on a yellow/red board, CR 4.3 4-22).
        .add_card({
            let mut o = gd_option("ILIAD-OPT", 3);
            o.traits = vec!["Iliad".to_string()];
            o
        })
        .add_card({
            let mut o = gd_option("ILIAD-OPT-PURPLE", 3);
            o.traits = vec!["Iliad".to_string()];
            o.colors = vec![CardColor::Purple];
            o
        })
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

// ─── Hand-pick candidate legality (select-hand playability audit) ────────────
// DCGO BT26_033.cs:101-105 offers only `(EqualsTraits("Iliad") ||
// EqualsTraits("TS")) && CanPlayOrUse(card, activateClass, fixedCost: max(0,
// GetCostItself - 5))` (PlayEffects.cs:286-293): an Option (a DUAL counts)
// with `!CanNotPlayThisOption` (no "can't use Option" lock, colour requirement
// or <Use Req.> met — CR 4.3 4-22, Q1500 / Q2785) and `MaxMemoryCost >= cost`,
// or a card with a play cost (never a Digi-Egg, CR 4.3 7-1-1) that
// `CanPlayAsNewPermanent` (empty frame, no play lock, payable). A card the
// play / use would refuse is never offered (CR 4.3 1-3-2).

/// Fire [When Digivolving] on a TS5 + Jupitermon stack and return the
/// hand-pick candidates' card ids (empty when no hand pick installs). PLAIN
/// (from security) joins the hand last and is never a candidate.
fn wd_offered(r: &mut DebugRunner, hand: &[&str]) -> Vec<String> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    for id in hand {
        push_hand(r, 0, id);
    }
    let j = r.place_stack(0, &["TS5", CARD_ID]);
    fire(r, EffectTiming::WhenDigivolving, j);
    let Some(v) = r.pending_selection_view() else {
        return Vec::new();
    };
    assert!(
        r.game
            .pending_selection
            .as_ref()
            .is_none_or(|p| p.card_choice.is_none()),
        "hand pick uses PLAY_HAND_START action ids"
    );
    let ids = hand_ids(r, 0);
    v.valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS && a >= PLAY_HAND_START && ((a - PLAY_HAND_START) as usize) < ids.len())
        .map(|a| ids[(a - PLAY_HAND_START) as usize].clone())
        .collect()
}

/// Tokomon (BT25-001) is an [Iliad]/[TS] Digi-Egg: no play cost, never played.
#[test]
fn bt26_033_wd_pick_excludes_iliad_digi_egg() {
    let mut r = setup();
    let offered = wd_offered(&mut r, &["BT25-001", "ILIAD"]);
    assert_eq!(offered, vec!["ILIAD".to_string()]);
}

/// A purple [Iliad] Option's colour requirement is unmet: not offered; the
/// yellow one is.
#[test]
fn bt26_033_wd_pick_excludes_option_whose_colour_is_unmet() {
    let mut r = setup();
    let offered = wd_offered(&mut r, &["ILIAD-OPT-PURPLE", "ILIAD-OPT"]);
    assert_eq!(offered, vec!["ILIAD-OPT".to_string()]);
}

/// "You can't use Option cards": the Option is not offered, the Digimon is.
#[test]
fn bt26_033_wd_pick_excludes_option_when_options_cannot_be_used() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = setup();
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotUseOptionCards, 0, Expiry::Permanent, None, 1),
    );
    let offered = wd_offered(&mut r, &["ILIAD-OPT", "ILIAD"]);
    assert_eq!(offered, vec!["ILIAD".to_string()]);
}

/// "You can't play Digimon by effects": the Digimon is not offered, the Option
/// still is.
#[test]
fn bt26_033_wd_pick_excludes_digimon_when_effect_play_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = setup();
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    let offered = wd_offered(&mut r, &["ILIAD-OPT", "ILIAD"]);
    assert_eq!(offered, vec!["ILIAD-OPT".to_string()]);
}

/// A full battle area: the Digimon and the Tamer are not offered, the Option
/// still is.
#[test]
fn bt26_033_wd_pick_excludes_digimon_and_tamer_when_battle_area_full() {
    let mut r = setup();
    r.game.rules.field_slots = 1; // the Jupitermon stack alone fills it
    let offered = wd_offered(&mut r, &["ILIAD-OPT", "ILIAD", "TS-T"]);
    assert_eq!(offered, vec!["ILIAD-OPT".to_string()]);
}

/// The play pays the cost reduced by 5: a card whose reduced cost would take
/// memory past -10 is not offered.
#[test]
fn bt26_033_wd_pick_excludes_unaffordable_digimon() {
    let mut r = setup();
    let offered = wd_offered(&mut r, &["ILIAD-18", "ILIAD-19"]);
    assert_eq!(offered, vec!["ILIAD-18".to_string()]);
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

/// G-ENGINE-DUAL-END-OF-TURN-WINDOW: a board-resident DUAL card is
/// `CardKind::Dual`; the end-of-turn keyword must still count it as a
/// Digimon so `<Engage>` offers its attack. Since 2026-10-18 (§16-44-2,
/// G-ENGINE-ENGAGE-ATTACK-AFTER-END-OF-TURN-TRIGGERS) <Engage> triggers at the
/// end of the turn and offers the attack as its own decision inside the
/// `[End of Your Turn]` pool, not in an `EndOfTurnAction` window.
#[test]
fn bt26_033_engage_parks_end_of_turn_attack_window_for_dual_card() {
    use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
    use digimon_engine::enums::GamePhase;

    let mut r = setup();
    let jupi = r.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(r.game.turn_player(), 0);

    r.game.end_turn();
    assert_ne!(r.game.current_phase, GamePhase::EndOfTurnAction);
    let sel = r
        .pending_selection()
        .expect("<Engage> on a DUAL top card must offer its end-of-turn attack");
    assert_eq!(sel.selecting_player, 0);
    let mask = digimon_engine::build_action_mask(&r.game, 0);
    assert_eq!(
        mask[encode_attack(jupi.index as u16, SECURITY_TARGET) as usize],
        1.0
    );
    assert_eq!(mask[PASS as usize], 1.0, "Engage is optional");
}
