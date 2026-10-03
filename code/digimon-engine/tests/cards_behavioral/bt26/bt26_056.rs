//! BT26-056 Cerberusmon: Werewolf Mode — DUAL (Digimon Lv.5 Black/Purple //
//! Option Black, use cost 3).
//!
//! Digimon: <Jamming> <Reboot> <Blocker> [On Deletion] You may play 1 level 4
//! or lower Digimon card with the [Titan] trait from your trash without paying
//! the cost. (Rule) Trait: Has [Dark Animal] Type.
//! Option: <Use Req. ([TS] trait)> [Main] Trash 1 card in your hand. Then,
//! <De-Digivolve 3> 1 of your opponent's Digimon. <Arts Digivolve>.
//!
//! DCGO: BT26/Purple/BT26_056.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-056";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-056")
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
        .add_card(digimon("D3", "Three", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("D4", "Four", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("D5", "Five", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("D6", "Six", CardColor::Red, 6, 12, &[]))
        .add_card(tamer("TS-TAMER", "TS Tamer", CardColor::Purple, &["TS"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(6)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_056_traits_include_rule_dark_animal_and_keywords() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    for t in ["Wizard", "Titan", "TS", "Dark Animal"] {
        assert!(c.traits.contains(&t.to_string()), "{t}");
    }
    let dbg = format!("{c:?}");
    for k in ["Jamming", "Reboot", "Blocker", "Cerberusmon"] {
        assert!(dbg.contains(k), "{k}");
    }
}

#[test]
fn bt26_056_on_deletion_plays_level_four_titan_free() {
    let mut r = setup();
    let cerb = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN4");
    push_trash(&mut r, 0, "TITAN5");
    fire(&mut r, EffectTiming::OnDeletion, cerb);
    let v = r.pending_selection_view().expect("optional trash pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "level 5 excluded");
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"TITAN4".to_string()));
    assert_eq!(r.memory(), 6, "free");
}

#[test]
fn bt26_056_on_deletion_no_candidate_no_prompt() {
    let mut r = setup();
    let cerb = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN5");
    fire(&mut r, EffectTiming::OnDeletion, cerb);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
}

fn use_option(r: &mut DebugRunner) {
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let _ = r.game.play_option_from_hand(0, idx);
}

#[test]
fn bt26_056_option_trashes_hand_then_de_digivolves_three() {
    let mut r = setup();
    r.place_on_field(0, "TS-TAMER", Some(0));
    let opp = r.place_stack(1, &["D3", "D4", "D5", "D6"]);
    push_hand(&mut r, 0, "FILLER");
    use_option(&mut r);
    let v = r.pending_selection_view().expect("mandatory hand trash");
    assert!(!v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("de-digivolve pick");
    assert!(!v.is_optional);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, opp), "D3", "3 cards trashed off the top");
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.memory(), 6 - 3);
}

#[test]
fn bt26_056_option_de_digivolve_stops_at_level_three() {
    let mut r = setup();
    r.place_on_field(0, "TS-TAMER", Some(0));
    let opp = r.place_stack(1, &["D3", "D4"]);
    use_option(&mut r);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, opp), "D3");
}
