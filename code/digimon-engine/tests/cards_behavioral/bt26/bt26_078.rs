//! BT26-078 Cherubimon — Lv.6 Purple/Green Digimon (Cherub / Titan / TS).
//!
//! [Trash] [Your Turn] When any of your [Chronomon] text or [Titan] trait
//! Digimon are played, if your opponent has 5 or more memory, by returning
//! this card to the bottom of the deck, 1 of them gains <Rush> and <Execute>
//! for the turn.
//! [On Play] [When Digivolving] By deleting this Digimon, you may play 1 play
//! cost 12 or lower [Chronomon] text or [Titan] trait card from your trash
//! without paying the cost.
//!
//! DCGO: BT26/Purple/BT26_078.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};

const CARD_ID: &str = "BT26-078";

fn setup(memory: i16) -> DebugRunner {
    let mut texted = digimon("TEXTED", "Texted", CardColor::Purple, 5, 8, &[]);
    texted.effect_text = "[When Digivolving] If you have [Chronomon] ...".to_string();
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-078")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "TITAN",
            "Titan Guy",
            CardColor::Purple,
            5,
            8,
            &["Titan"],
        ))
        .add_card(digimon(
            "BIG-TITAN",
            "Big",
            CardColor::Purple,
            7,
            14,
            &["Titan"],
        ))
        .add_card(texted)
        .add_card(tamer(
            "TITAN-T",
            "Titan Tamer",
            CardColor::Purple,
            &["Titan"],
        ))
        .add_card(digimon("PLAIN", "Plain", CardColor::Purple, 5, 8, &["TS"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(memory)
        .start();
    r.set_first_player(0);
    r
}

fn has_kw(r: &DebugRunner, h: digimon_engine::permanent::PermanentHandle, k: Keyword) -> bool {
    r.game.has_keyword(h, k)
}

#[test]
fn bt26_078_traits_and_alt_path() {
    let r = setup(0);
    let c = r.compiled_card(CARD_ID).unwrap();
    for t in ["Cherub", "Titan", "TS"] {
        assert!(c.traits.contains(&t.to_string()), "{t}");
    }
    assert!(format!("{:?}", c.alt_paths).contains("TS"));
}

#[test]
fn bt26_078_trash_titan_played_grants_rush_execute() {
    let mut r = setup(-5);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "TITAN", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    assert!(r.pending_is_optional(), "by returning — optional");
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(!trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(deck_ids(&r, 0).first().map(String::as_str), Some(CARD_ID));
    assert!(has_kw(&r, t, Keyword::Rush));
    assert!(has_kw(&r, t, Keyword::Execute));
}

#[test]
fn bt26_078_trash_chronomon_text_counts() {
    let mut r = setup(-6);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "TEXTED", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(has_kw(&r, t, Keyword::Execute));
}

#[test]
fn bt26_078_trash_needs_opponent_five_memory() {
    let mut r = setup(-4);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "TITAN", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    assert!(r.pending_selection_view().is_none());
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt26_078_trash_ignores_non_matching_digimon() {
    let mut r = setup(-5);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "PLAIN", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    assert!(r.pending_selection_view().is_none());
}

#[test]
fn bt26_078_trash_decline_keeps_card() {
    let mut r = setup(-5);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "TITAN", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    r.decline_optional_trigger().unwrap();
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert!(!has_kw(&r, t, Keyword::Rush));
}

#[test]
fn bt26_078_on_play_delete_self_then_play_titan_free() {
    let mut r = setup(3);
    let c = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN");
    push_trash(&mut r, 0, "BIG-TITAN"); // cost 14 — ineligible
    push_trash(&mut r, 0, "TITAN-T");
    push_trash(&mut r, 0, "PLAIN"); // no Titan/Chronomon — ineligible
    fire(&mut r, EffectTiming::OnPlay, c);
    assert!(r.pending_is_optional(), "may");
    r.accept_optional_trigger().unwrap();
    // Self deleted as the cost; then optional trash pick (TITAN, TITAN-T).
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional);
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count(),
        2
    );
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    let f = field_ids(&r, 0);
    assert_eq!(f.len(), 1);
    assert!(f[0] == "TITAN" || f[0] == "TITAN-T");
    assert_eq!(r.memory(), 3, "without paying the cost");
}

#[test]
fn bt26_078_on_play_decline_keeps_cherubimon() {
    let mut r = setup(3);
    let c = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN");
    fire(&mut r, EffectTiming::WhenDigivolving, c);
    r.decline_optional_trigger().unwrap();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
}

#[test]
fn bt26_078_trash_only_on_your_turn() {
    let mut r = setup(5); // opponent's turn: the gauge from P0's view is -5
    push_trash(&mut r, 0, CARD_ID);
    r.game.turn_player_idx = 1;
    let t = r.place_on_field(0, "TITAN", None);
    r.fire_play_event_triggers(0, t.index as usize, true, false);
    assert!(r.pending_selection_view().is_none(), "[Your Turn] only");
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}
