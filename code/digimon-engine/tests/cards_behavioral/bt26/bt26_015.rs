//! BT26-015 Butenmon — Lv.5 Red/Yellow (Shaman / Iliad / TS).
//!
//! [On Play] [When Digivolving] 1 of your opponent's Digimon gets -4000 DP
//! until their turn ends. Then, by returning 1 card in your trash to the
//! bottom of the deck, delete 1 of your opponent's 5000 DP or lower Digimon.
//! [Your Turn] [Once Per Turn] When your effects add to decks, 1 of your
//! Digimon may get +3000 DP until your opponent's turn ends and attack.
//! Inherited: [All Turns] [Once Per Turn] When your effects add to decks, this
//! Digimon with [Chronomon] in its text may unsuspend.
//!
//! DCGO: BT26/Red/BT26_015.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-015";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-015")
        .from_dsl_yaml(DECKER_YAML)
        .expect("decker")
        .add_card(filler("FILLER"))
        .add_card(digimon("OPP6", "Opp Six", CardColor::Blue, 6, 9, &[]))
        .add_card(digimon("OPP3", "Opp Three", CardColor::Blue, 3, 3, &[]))
        .add_card(digimon("ATK", "Attacker", CardColor::Red, 4, 5, &[]))
        .add_card(digimon(
            "CHRONOTOP",
            "Chronomon Top",
            CardColor::Red,
            6,
            12,
            &[],
        ))
        .add_card(digimon("PLAINTOP", "Plain Top", CardColor::Red, 6, 12, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r.game.turn_count = 1;
    r
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

#[test]
fn bt26_015_alt_path() {
    let r = setup();
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("TS") && alt.contains("level_eq: Some(4)"),
        "{alt}"
    );
}

#[test]
fn bt26_015_on_play_debuff_then_bottom_trash_card_to_delete() {
    let mut r = setup();
    let opp6 = r.place_on_field(1, "OPP6", Some(0));
    r.place_on_field(1, "OPP3", Some(0));
    push_trash(&mut r, 0, "FILLER");
    let b = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, b);
    // -4000 pick is mandatory.
    let v = r.pending_selection_view().expect("debuff pick");
    assert!(!v.is_optional);
    pick_first(&mut r, 0); // OPP6 (index 0)
    assert_eq!(r.effective_dp(opp6), Some(2000));
    // "by returning 1 card in your trash" — optional.
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional);
    pick_first(&mut r, 0);
    assert!(trash_ids(&r, 0).is_empty());
    assert_eq!(deck_ids(&r, 0).len(), 7);
    // Delete pick: both opp Digimon are now <= 5000.
    let v = r.pending_selection_view().expect("delete pick");
    assert!(!v.is_optional);
    pick_first(&mut r, 0);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    // The deck add fires this card's own [Your Turn] clause; decline it.
    let v = r
        .pending_selection_view()
        .expect("[Your Turn] +3000/attack pick");
    assert!(v.is_optional);
    pass(&mut r, 0);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn bt26_015_declining_the_trash_cost_skips_the_delete() {
    let mut r = setup();
    r.place_on_field(1, "OPP6", Some(0));
    push_trash(&mut r, 0, "FILLER");
    let b = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::WhenDigivolving, b);
    pick_first(&mut r, 0); // debuff OPP6 → 2000
    pass(&mut r, 0); // decline the trash cost
    assert!(r.game.pending_selection.is_none(), "no delete, no deck add");
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(trash_ids(&r, 0), vec!["FILLER"]);
}

#[test]
fn bt26_015_your_turn_deck_add_buffs_and_forces_attack() {
    let mut r = setup();
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "FILLER");
    let decker = r.place_on_field(0, "T-DECKER", None);
    fire(&mut r, EffectTiming::OnPlay, decker);
    pick_first(&mut r, 0); // decker trash pick
    let v = r.pending_selection_view().expect("Butenmon pick");
    assert!(v.is_optional);
    pick_first(&mut r, 0); // ATK is battle-area index 0
    assert_eq!(r.effective_dp(atk), Some(4000 + 3000));
    let v = r.pending_selection_view().expect("attack target selection");
    assert!(!v.is_optional, "it attacks (not 'may attack')");
    assert!(!suspended(&r, atk));
}

#[test]
fn bt26_015_declining_refunds_once_per_turn() {
    let mut r = setup();
    r.place_on_field(0, "ATK", Some(0));
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "FILLER");
    push_trash(&mut r, 0, "FILLER");
    let decker = r.place_on_field(0, "T-DECKER", None);
    fire(&mut r, EffectTiming::OnPlay, decker);
    pick_first(&mut r, 0);
    pass(&mut r, 0); // decline → RemoveUse
    assert!(r.game.pending_selection.is_none());
    fire(&mut r, EffectTiming::OnPlay, decker);
    pick_first(&mut r, 0);
    assert!(
        r.pending_selection_view().is_some(),
        "OPT not spent by declining"
    );
}

#[test]
fn bt26_015_your_turn_clause_not_on_opponents_turn() {
    let mut r = setup();
    r.place_on_field(0, "ATK", Some(0));
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "FILLER");
    let decker = r.place_on_field(0, "T-DECKER", None);
    r.set_first_player(1);
    fire(&mut r, EffectTiming::OnPlay, decker);
    pick_first(&mut r, 0);
    assert!(r.game.pending_selection.is_none(), "[Your Turn]");
}

#[test]
fn bt26_015_inherited_unsuspends_chronomon_text_digimon_on_any_turn() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "CHRONOTOP"]);
    r.game.players[0].battle_area[host.index as usize].is_suspended = true;
    push_trash(&mut r, 0, "FILLER");
    let decker = r.place_on_field(0, "T-DECKER", None);
    r.set_first_player(1); // [All Turns]
    fire(&mut r, EffectTiming::OnPlay, decker);
    pick_first(&mut r, 0);
    assert!(r.pending_is_optional(), "'may unsuspend'");
    r.accept_optional_trigger().expect("accept");
    assert!(!suspended(&r, host));
}

#[test]
fn bt26_015_inherited_needs_chronomon_text_and_own_effect() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "PLAINTOP"]);
    r.game.players[0].battle_area[host.index as usize].is_suspended = true;
    push_trash(&mut r, 0, "FILLER");
    let decker = r.place_on_field(0, "T-DECKER", None);
    fire(&mut r, EffectTiming::OnPlay, decker);
    pick_first(&mut r, 0);
    assert!(
        r.game.pending_selection.is_none(),
        "top has no [Chronomon] text"
    );
    assert!(suspended(&r, host));

    // Chronomon top, but the OPPONENT's effect adds to the deck.
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "CHRONOTOP"]);
    r.game.players[0].battle_area[host.index as usize].is_suspended = true;
    push_trash(&mut r, 1, "FILLER");
    let decker = r.place_on_field(1, "T-DECKER", None);
    fire(&mut r, EffectTiming::OnPlay, decker);
    drain_first(&mut r);
    assert!(suspended(&r, host), "only YOUR effects");
}
