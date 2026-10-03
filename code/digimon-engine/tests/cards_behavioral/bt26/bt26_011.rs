//! BT26-011 Buraimon — Lv.4 Red (Birdkin / Iliad / TS).
//!
//! <Raid> [On Play] [When Digivolving] By trashing 1 card with [Chronomon] in
//! its text or the [Shaman] trait from your hand, <Draw 2>.
//! Inherited: <Raid>.
//!
//! DCGO: BT26/Red/BT26_011.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};

const CARD_ID: &str = "BT26-011";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-011")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "SHAMAN",
            "Shamanmon",
            CardColor::Red,
            5,
            7,
            &["Shaman"],
        ))
        .add_card(digimon(
            "CHRONO",
            "Chronomon Kid",
            CardColor::Red,
            4,
            5,
            &[],
        ))
        .add_card(digimon("PLAIN", "Plainmon", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("ROOK", "Rook", CardColor::Red, 3, 3, &["TS"]))
        .add_card(digimon("HOST", "Host", CardColor::Red, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_011_raid_own_and_inherited_and_alt_path() {
    let mut r = setup();
    let own = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(own, Keyword::Raid));
    let carrier = r.place_stack(0, &[CARD_ID, "HOST"]);
    assert!(
        r.game.has_keyword(carrier, Keyword::Raid),
        "inherited <Raid>"
    );
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("TS") && alt.contains("level_eq: Some(3)"),
        "{alt}"
    );
}

#[test]
fn bt26_011_on_play_trash_chronomon_text_card_draws_two() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    push_hand(&mut r, 0, "CHRONO");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, h);
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional);
    let plain = digimon_engine::action::space::PLAY_HAND_START;
    assert!(!v.valid_action_ids.contains(&plain));
    pick_hand(&mut r, 0, "CHRONO");
    assert_eq!(trash_ids(&r, 0), vec!["CHRONO"]);
    assert_eq!(hand_ids(&r, 0).len(), 3, "PLAIN + 2 drawn");
}

#[test]
fn bt26_011_when_digivolving_accepts_shaman_and_decline_draws_nothing() {
    let mut r = setup();
    push_hand(&mut r, 0, "SHAMAN");
    let h = r.place_stack(0, &["ROOK", CARD_ID]);
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    pass(&mut r, 0);
    assert_eq!(hand_ids(&r, 0), vec!["SHAMAN"]);
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    pick_hand(&mut r, 0, "SHAMAN");
    assert_eq!(hand_ids(&r, 0).len(), 2);
}

#[test]
fn bt26_011_no_candidate_no_prompt() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, h);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(hand_ids(&r, 0), vec!["PLAIN"]);
}
