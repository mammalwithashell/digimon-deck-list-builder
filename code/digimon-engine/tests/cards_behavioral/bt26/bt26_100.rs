//! BT26-100 Dark Field — Option, Purple/Black, use cost 3, Titan/TS.
//!
//! While you have no face-up security cards, you can ignore this card's color
//! requirements.
//! {Security} [All Turns] All of your [Titan] trait Digimon gain <Blocker>.
//! While you have a Digimon with [Plutomon] or [Titamon] in its name, they
//! also get +3000 DP.
//! [Main] Add your bottom security card to the hand and place this card face
//! up as the bottom security card. Then, you may play 1 level 4 or lower
//! [Titan] trait card from your hand or trash without paying the cost.
//! [Security] You may play 1 level 4 or lower [Titan] trait Digimon card from
//! your hand or trash without paying the cost.
//!
//! DCGO: BT26/Purple/BT26_100.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, Keyword};

const CARD_ID: &str = "BT26-100";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-100")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "TITAN4",
            "Titan Four",
            CardColor::Purple,
            4,
            5,
            &["Titan"],
        ))
        .add_card(digimon(
            "TITAN5",
            "Titan Five",
            CardColor::Purple,
            5,
            7,
            &["Titan"],
        ))
        .add_card(digimon(
            "PLUTO",
            "Plutomon",
            CardColor::Purple,
            6,
            13,
            &["Titan"],
        ))
        .add_card(digimon("PLAIN", "Plain", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("ATK", "Attacker", CardColor::Red, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
}

fn use_option(r: &mut DebugRunner) {
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, idx);
}

#[test]
fn bt26_100_main_swaps_bottom_security_then_plays_titan_from_trash() {
    let mut r = builder().security(0, &["FILLER", "TITAN5"]).start();
    r.set_first_player(0);
    push_trash(&mut r, 0, "TITAN4");
    push_trash(&mut r, 0, "TITAN5");
    use_option(&mut r);
    // Bottom security (index 0) went to the hand; Dark Field is the new bottom, face up.
    assert_eq!(hand_ids(&r, 0), vec!["FILLER".to_string()]);
    let bottom = &r.game.players[0].security[0];
    assert_eq!(bottom.card_id(&r.game.card_data), CARD_ID);
    assert!(r.game.players[0]
        .face_up_security
        .contains(&bottom.card_index));
    let v = r.pending_selection_view().expect("optional play pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "only the level-4 [Titan]");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TITAN4".to_string()]);
    assert_eq!(r.memory(), 5 - 3, "only the use cost was paid");
}

#[test]
fn bt26_100_cannot_ignore_colors_with_a_face_up_security_card() {
    let mut r = builder().security(0, &["FILLER", "TITAN5"]).start();
    r.set_first_player(0);
    let idx = r.game.players[0].security[1].card_index;
    r.game.players[0].face_up_security.insert(idx);
    push_hand(&mut r, 0, CARD_ID);
    r.game.enter_main_phase();
    let res = r.game.play_option_from_hand(0, 0);
    assert_eq!(
        format!("{res:?}"),
        "Invalid",
        "no purple/black card in play"
    );
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_100_face_up_security_grants_blocker_and_dp_with_plutomon() {
    let mut r = builder().security(0, &[CARD_ID, "FILLER"]).start();
    r.set_first_player(0);
    let idx = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(idx);
    let titan = r.place_on_field(0, "TITAN4", Some(0));
    let plain = r.place_on_field(0, "PLAIN", Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(titan, Keyword::Blocker));
    assert!(!r.game.has_keyword(plain, Keyword::Blocker));
    assert_eq!(r.effective_dp(titan), Some(4000), "no Plutomon yet");
    let pluto = r.place_on_field(0, "PLUTO", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(titan), Some(7000));
    assert_eq!(r.effective_dp(pluto), Some(9000));
    assert_eq!(r.effective_dp(plain), Some(4000));
}

#[test]
fn bt26_100_face_down_security_grants_nothing() {
    let mut r = builder().security(0, &[CARD_ID, "FILLER"]).start();
    r.set_first_player(0);
    let titan = r.place_on_field(0, "TITAN4", Some(0));
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(titan, Keyword::Blocker));
}

#[test]
fn bt26_100_security_check_plays_titan_digimon_from_hand() {
    let mut r = builder().security(1, &[CARD_ID]).start();
    r.set_first_player(0);
    push_hand(&mut r, 1, "TITAN4");
    push_hand(&mut r, 1, "TITAN5");
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _ = r.attack_player(atk, 1, false);
    let v = r.pending_selection_view().expect("security play pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1);
    pick_first(&mut r, 1);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"TITAN4".to_string()));
}
