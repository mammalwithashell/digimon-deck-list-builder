//! BT26-022 Sorcermon — Lv.4 Blue/Yellow, Wizard/Witchelny/Iliad/TS.
//!
//! [On Play] [When Digivolving] Add your top security card to the hand and
//! <Recovery +1>.
//! [End of Your Turn] If you have a red or purple Digimon, by placing this
//! Digimon as the bottom security card, you may play 1 blue or red [Iliad]
//! trait Digimon card from your hand with the cost reduced by 4.
//! Inherited: <Barrier>.
//!
//! DCGO: BT26/Blue/BT26_022.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;

const CARD_ID: &str = "BT26-022";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-022")
        .add_card(filler("FILLER"))
        .add_card(digimon("SEC-A", "SecA", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("SEC-B", "SecB", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("DECKTOP", "DeckTop", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("TS3", "TS Three", CardColor::Green, 3, 3, &["TS"]))
        .add_card(digimon("RED-ALLY", "Red Ally", CardColor::Red, 4, 5, &[]))
        .add_card(digimon(
            "GREEN-ALLY",
            "Green Ally",
            CardColor::Green,
            4,
            5,
            &[],
        ))
        .add_card(digimon_colors(
            "ILIAD-RED",
            6,
            7,
            &[CardColor::Red],
            &["Iliad"],
        ))
        .add_card(digimon_colors(
            "ILIAD-GREEN",
            6,
            7,
            &[CardColor::Green],
            &["Iliad"],
        ))
        .add_card(digimon_colors(
            "RED-NOT-ILIAD",
            6,
            7,
            &[CardColor::Red],
            &["TS"],
        ))
        .deck(0, &["FILLER", "FILLER", "DECKTOP"])
        .deck(1, &["FILLER"; 6])
        .security(0, &["SEC-A", "SEC-B"])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_022_ts_alt_digivolve_path_compiled() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.alt_paths.len(), 1, "[Digivolve] Lv.3 w/[TS]: Cost 2");
}

#[test]
fn bt26_022_on_play_adds_top_security_and_recovers() {
    let mut r = setup();
    let s = r.place_on_field(0, CARD_ID, Some(0));
    let top = security_ids(&r, 0).last().cloned().unwrap();
    fire(&mut r, EffectTiming::OnPlay, s);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec![top.clone()], "top security → hand");
    let sec = security_ids(&r, 0);
    assert_eq!(sec.len(), 2, "2 - 1 + Recovery 1");
    assert_eq!(
        sec.last().map(String::as_str),
        Some("DECKTOP"),
        "<Recovery +1> from deck top"
    );
}

#[test]
fn bt26_022_when_digivolving_with_empty_security_still_recovers() {
    let mut r = setup();
    r.game.players[0].security.clear();
    let s = r.place_stack(0, &["TS3", CARD_ID]);
    fire(&mut r, EffectTiming::WhenDigivolving, s);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(security_ids(&r, 0), vec!["DECKTOP".to_string()]);
}

#[test]
fn bt26_022_eot_places_self_bottom_security_and_plays_iliad_at_minus_four() {
    let mut r = setup();
    let s = r.place_stack(0, &["TS3", CARD_ID]);
    r.place_on_field(0, "RED-ALLY", Some(0));
    push_hand(&mut r, 0, "ILIAD-GREEN");
    push_hand(&mut r, 0, "RED-NOT-ILIAD");
    push_hand(&mut r, 0, "ILIAD-RED");
    let mem = r.memory();
    fire(&mut r, EffectTiming::EndOfYourTurn, s);
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().expect("accept");
    assert_eq!(
        security_ids(&r, 0).first().map(String::as_str),
        Some(CARD_ID),
        "placed as the BOTTOM security card"
    );
    assert!(
        trash_ids(&r, 0).contains(&"TS3".to_string()),
        "source trashed"
    );
    assert!(!field_ids(&r, 0).contains(&CARD_ID.to_string()));
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 1, "only the red [Iliad] Digimon");
    pick_hand(&mut r, 0, "ILIAD-RED");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"ILIAD-RED".to_string()));
    assert_eq!(r.memory(), mem - 3, "cost 7 reduced by 4");
}

#[test]
fn bt26_022_eot_requires_red_or_purple_digimon() {
    let mut r = setup();
    let s = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "GREEN-ALLY", Some(0));
    push_hand(&mut r, 0, "ILIAD-RED");
    fire(&mut r, EffectTiming::EndOfYourTurn, s);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(r.security_count(0), 2);
}

#[test]
fn bt26_022_eot_declined_keeps_sorcermon() {
    let mut r = setup();
    let s = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "RED-ALLY", Some(0));
    fire(&mut r, EffectTiming::EndOfYourTurn, s);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(r.security_count(0), 2);
}

#[test]
fn bt26_022_eot_cost_fails_when_security_cannot_be_added() {
    let mut r = setup();
    let s = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "RED-ALLY", Some(0));
    push_hand(&mut r, 0, "ILIAD-RED");
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotAddSecurityByEffect,
            0,
            Expiry::EndOfTurn,
            None,
            1,
        ),
    );
    fire(&mut r, EffectTiming::EndOfYourTurn, s);
    if r.pending_is_optional() {
        r.accept_optional_trigger().expect("accept");
    }
    let _ = r.auto_resolve();
    assert!(
        field_ids(&r, 0).contains(&CARD_ID.to_string()),
        "cost unpaid"
    );
    assert_eq!(r.hand_size(0), 1, "no play without the cost");
}

#[test]
fn bt26_022_inherits_barrier() {
    let mut r = setup();
    let h = r.place_stack(0, &[CARD_ID, "TS3"]);
    assert!(r.game.has_keyword(h, Keyword::Barrier));
}
