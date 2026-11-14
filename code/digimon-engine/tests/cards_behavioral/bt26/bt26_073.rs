//! BT26-073 Aegiochusmon: Dark — Lv.5 Purple/Red, Shaman/Iliad/TS/(Rule) Wizard.
//!
//! [On Play] [When Digivolving] By deleting this Digimon or returning 1 [Shaman]
//! or [TS] trait card from your trash to the bottom of the deck, delete 1 of
//! your opponent's level 5 or lower Digimon.
//! [On Deletion] You may play 1 [TS] trait card with a play cost of 5 or less
//! from your hand or trash without paying the cost.
//! Inherited: <Security A. +1>. Assembly -2 (Lv.4- w/[Chronomon] in text or [TS]).
//!
//! DCGO: BT26/Purple/BT26_073.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-073";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-073")
        .add_card(filler("FILLER"))
        .add_card(digimon("BIG", "Big", CardColor::Red, 6, 12, &[]))
        .add_card(digimon("SMALL", "Small", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("TS5", "Ts Five", CardColor::Red, 4, 5, &["TS"]))
        .add_card(digimon("TS6", "Ts Six", CardColor::Red, 4, 6, &["TS"]))
        .add_card(digimon(
            "SHAMAN",
            "Shaman Guy",
            CardColor::Red,
            3,
            3,
            &["Shaman"],
        ))
        .add_card(tamer("TS-T", "Ts Tamer", CardColor::Red, &["TS"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_073_identity_rule_trait_alt_paths_and_inherited() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(format!("{:?}", card.traits).contains("Wizard"));
    let alts = format!("{:?}", card.alt_paths);
    assert!(alts.contains("Aegiomon"), "{alts}");
    assert!(
        alts.contains("Chronomon"),
        "assembly material filter: {alts}"
    );
    let effs = format!("{:?}", card.effects);
    assert!(effs.contains("SecurityAttackPlus"), "{effs}");
}

#[test]
fn bt26_073_on_play_delete_self_deletes_level5_or_lower() {
    let mut r = setup();
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "BIG", Some(0));
    r.place_on_field(1, "SMALL", Some(0));
    fire(&mut r, EffectTiming::OnPlay, ae);
    // Empty trash ⇒ the 2-way modal {Delete this | Don't pay}.
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.valid_action_ids.len(), 2, "{:?}", v.valid_action_ids);
    r.execute_branch(0).expect("delete this Digimon");
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(
        field_ids(&r, 1),
        vec!["BIG".to_string()],
        "only Lv.5- target"
    );
}

#[test]
fn bt26_073_when_digivolving_return_trash_card_keeps_self() {
    let mut r = setup();
    push_trash(&mut r, 0, "SHAMAN");
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "SMALL", Some(0));
    r.place_on_field(1, "BIG", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, ae);
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.valid_action_ids.len(), 3, "trash candidate ⇒ 3-way modal");
    r.execute_branch(1).expect("return from trash");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).is_empty(), "returned out of trash");
    assert!(deck_ids(&r, 0).contains(&"SHAMAN".to_string()));
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()), "self kept");
    assert_eq!(field_ids(&r, 1), vec!["BIG".to_string()]);
}

#[test]
fn bt26_073_declined_trash_pick_does_not_delete() {
    let mut r = setup();
    push_trash(&mut r, 0, "TS5");
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "SMALL", Some(0));
    fire(&mut r, EffectTiming::OnPlay, ae);
    r.execute_branch(1).expect("return from trash");
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(trash_ids(&r, 0), vec!["TS5".to_string()]);
    assert_eq!(
        field_ids(&r, 1),
        vec!["SMALL".to_string()],
        "not paid ⇒ no delete"
    );
}

#[test]
fn bt26_073_dont_pay() {
    let mut r = setup();
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "SMALL", Some(0));
    fire(&mut r, EffectTiming::OnPlay, ae);
    r.execute_branch(1).expect("don't pay");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(field_ids(&r, 1), vec!["SMALL".to_string()]);
}

#[test]
fn bt26_073_on_deletion_plays_ts_from_trash_free() {
    let mut r = setup();
    push_trash(&mut r, 0, "TS5");
    push_trash(&mut r, 0, "TS6"); // cost 6 — not eligible
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    let mem = r.memory();
    r.game.delete_permanents_batch(
        vec![ae],
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    let v = r.pending_selection_view().expect("on-deletion play pick");
    assert!(v.is_optional, "you may");
    let picks = v
        .valid_action_ids
        .iter()
        .filter(|&&x| x != digimon_engine::action::space::PASS)
        .count();
    assert_eq!(picks, 1, "only the cost-5 [TS] card");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TS5".to_string()]);
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn bt26_073_on_deletion_plays_ts_tamer_from_hand() {
    let mut r = setup();
    push_hand(&mut r, 0, "TS-T");
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    r.game.delete_permanents_batch(
        vec![ae],
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TS-T".to_string()]);
    assert!(hand_ids(&r, 0).is_empty());
}

#[test]
fn bt26_073_on_deletion_may_decline() {
    let mut r = setup();
    push_hand(&mut r, 0, "TS-T");
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    r.game.delete_permanents_batch(
        vec![ae],
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(hand_ids(&r, 0), vec!["TS-T".to_string()]);
}

// ─── [On Deletion] candidate legality (G-DSL-PLAY-CANDIDATE-ORIGIN-ZONE) ─────
// DCGO BT26_073.cs:234-240 CanPlayCondition: `HasTSTraits && HasPlayCost &&
// GetCostItself <= 5 && CanPlayAsNewPermanent(card, payCost:false,
// activateClass)`, and CanActivateCondition (:247-252) needs one in hand or
// trash. CR 4.3 1-3-2 (an impossible action is not carried out), 7-1-1 (playing
// places a new card with a play cost), 4-6-2 (a DUAL is digivolved or used,
// never played).

/// Sorted card ids of the non-PASS candidates of the pending hand∪trash pick.
fn offered(r: &DebugRunner) -> Vec<String> {
    if r.game.pending_selection.is_none() {
        return Vec::new();
    }
    let mut ids: Vec<String> = physical_menu::commits(&r.game)
        .into_iter()
        .filter_map(|row| row.card_id)
        .collect();
    ids.sort();
    ids
}

fn add_player_lock(r: &mut DebugRunner, m: digimon_engine::enums::ModifierType) {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    r.game
        .modifiers
        .add_player_modifier(0, PlayerModifierEntry::simple(m, 0, Expiry::Permanent, None, 1));
}

fn delete_it(r: &mut DebugRunner, ae: digimon_engine::permanent::PermanentHandle) {
    r.game.delete_permanents_batch(
        vec![ae],
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
}

/// "You can't play Digimon by effects": the [TS] Digimon is not offered, the
/// [TS] Tamer still is.
#[test]
fn bt26_073_on_deletion_effect_play_prohibition_excludes_digimon() {
    let mut r = setup();
    push_hand(&mut r, 0, "TS-T");
    push_trash(&mut r, 0, "TS5");
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    add_player_lock(&mut r, digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect);
    delete_it(&mut r, ae);
    assert_eq!(offered(&r), vec!["TS-T".to_string()]);
}

/// "You can't play cards from the trash" (BT23-014; the trash-origin play
/// refuses): the trash card is not offered, the hand card still is.
#[test]
fn bt26_073_on_deletion_trash_play_lock_excludes_trash_cards() {
    let mut r = setup();
    push_hand(&mut r, 0, "TS-T");
    push_trash(&mut r, 0, "TS5");
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    add_player_lock(&mut r, digimon_engine::enums::ModifierType::CannotPlayFromTrash);
    delete_it(&mut r, ae);
    assert_eq!(offered(&r), vec!["TS-T".to_string()]);
}

/// A full battle area after the deletion: nothing can be played, so nothing is
/// offered and the cards stay where they are.
#[test]
fn bt26_073_on_deletion_full_battle_area_offers_nothing() {
    let mut r = setup();
    push_hand(&mut r, 0, "TS-T");
    push_trash(&mut r, 0, "TS5");
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "FILLER", Some(0));
    r.place_on_field(0, "FILLER", Some(0));
    // After Aegiochusmon leaves, the two fillers fill a 2-slot battle area.
    r.game.rules.field_slots = 2;
    delete_it(&mut r, ae);
    assert!(offered(&r).is_empty(), "offered {:?}", offered(&r));
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["TS-T".to_string()]);
    assert!(trash_ids(&r, 0).contains(&"TS5".to_string()));
}

/// Regression guard: a [TS] DUAL card (Jupitermon BT26-033, stored cost 2) has
/// no play cost and can't be played (CR 4.3 7-1-1, 4-6-2), so it is never offered.
#[test]
fn bt26_073_on_deletion_dual_ts_card_not_offered() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-073")
        .dsl_card("BT26-033")
        .expect("BT26-033")
        .add_card(filler("FILLER"))
        .add_card(digimon("TS5", "Ts Five", CardColor::Red, 4, 5, &["TS"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    push_trash(&mut r, 0, "BT26-033");
    push_trash(&mut r, 0, "TS5");
    let ae = r.place_on_field(0, CARD_ID, Some(0));
    delete_it(&mut r, ae);
    assert_eq!(offered(&r), vec!["TS5".to_string()]);
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;
