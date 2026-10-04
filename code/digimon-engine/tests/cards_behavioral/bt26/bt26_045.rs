//! BT26-045 GranKuwagamon — Lv.6 Green, Insectoid/Titan/TS.
//!
//! When this card would be played, if your hand has fewer cards than your
//! opponent's, reduce the cost by 4.
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may play 1
//! level 4 or lower [Insectoid] or [Titan] trait Digimon card from your hand
//! without paying the cost.
//! [Your Turn] All of your [Insectoid] or [Titan] trait Digimon gain
//! <Alliance>, <Piercing> and <Vortex>.
//!
//! DCGO: BT26/Green/BT26_045.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};

const CARD_ID: &str = "BT26-045";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-045 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "BUG4",
            "Bug Four",
            CardColor::Green,
            4,
            5,
            &["Insectoid"],
        ))
        .add_card(digimon(
            "TITAN3",
            "Titan Three",
            CardColor::Purple,
            3,
            3,
            &["Titan"],
        ))
        .add_card(digimon(
            "BUG5",
            "Bug Five",
            CardColor::Green,
            5,
            7,
            &["Insectoid"],
        ))
        .add_card(digimon(
            "BEAST4",
            "Beast Four",
            CardColor::Green,
            4,
            5,
            &["Beast"],
        ))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.turn_count = 1;
    r
}

#[test]
fn bt26_045_play_cost_reduced_with_smaller_hand() {
    let mut r = setup();
    let idx = r.add_to_hand(0, CARD_ID);
    push_hand(&mut r, 1, "FILLER");
    push_hand(&mut r, 1, "FILLER");
    r.play(0, idx).expect("played");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 5 - (11 - 4));
}

#[test]
fn bt26_045_play_cost_full_with_equal_hands() {
    let mut r = setup();
    let idx = r.add_to_hand(0, CARD_ID);
    push_hand(&mut r, 1, "FILLER");
    r.play(0, idx).expect("played");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 5 - 11);
}

#[test]
fn bt26_045_on_play_plays_level4_insectoid_free() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BUG4");
    push_hand(&mut r, 0, "BUG5");
    push_hand(&mut r, 0, "BEAST4");
    push_hand(&mut r, 0, "TITAN3");
    fire(&mut r, EffectTiming::OnPlay, h);
    let v = r.pending_selection_view().expect("optional hand pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 2, "BUG4 + TITAN3 only");
    pick_hand(&mut r, 0, "BUG4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"BUG4".to_string()));
    assert_eq!(r.memory(), 5, "without paying the cost");
    // [Once Per Turn] across the shared timings.
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.pending_selection_view().is_none());
}

#[test]
fn bt26_045_decline_refunds_once_per_turn() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BUG4");
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.pending_selection_view().is_some(), "re-offered");
}

#[test]
fn bt26_045_your_turn_grants_alliance_piercing_vortex() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let bug = r.place_on_field(0, "BUG4", Some(0));
    let beast = r.place_on_field(0, "BEAST4", Some(0));
    r.game.tick_declarative_effects();
    for k in [Keyword::Alliance, Keyword::Piercing, Keyword::Vortex] {
        assert!(r.game.has_keyword(h, k), "{k:?} on self");
        assert!(r.game.has_keyword(bug, k), "{k:?} on Insectoid");
        assert!(!r.game.has_keyword(beast, k), "{k:?} not on Beast");
    }
    // Opponent's turn: the [Your Turn] grants drop.
    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert!(
        !r.game.has_keyword(bug, Keyword::Piercing),
        "[Your Turn] only"
    );
    assert!(
        !r.game.has_keyword(h, Keyword::Alliance),
        "[Your Turn] only"
    );
}

#[test]
fn bt26_045_granted_vortex_offers_end_of_turn_attack() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    let bug = r.place_on_field(0, "BUG4", Some(0));
    let opp = r.place_on_field(1, "BEAST4", Some(0));
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(bug, Keyword::Vortex));
    assert!(
        r.game.has_end_of_turn_keywords(0),
        "<Vortex> end-of-turn attack offered"
    );
}
