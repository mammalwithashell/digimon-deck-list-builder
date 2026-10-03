//! BT26-036 Lalamon — Lv.3 Green, Vegetation/DATA SQUAD.
//!
//! [When Moving] [On Play] Reveal the top 3 cards of your deck. Add 1 card with
//! the [Vegetation], [Fairy] or [DATA SQUAD] trait or 1 green Tamer card among
//! them to the hand. Return the rest to the bottom of the deck.
//! Inherited: [When Attacking] [Once Per Turn] You may suspend 1 of your
//! opponent's Digimon.
//!
//! DCGO: BT26/Green/BT26_036.cs (inherited: RemoveUse when nothing picked).

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-036";

fn setup(deck_top: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut deck: Vec<&str> = vec!["FILLER"; 4];
    deck.extend(deck_top.iter().rev());
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-036 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "FAIRY",
            "Lilamon",
            CardColor::Green,
            5,
            6,
            &["Fairy"],
        ))
        .add_card(tamer("GREEN-T", "Some Tamer", CardColor::Green, &[]))
        .add_card(tamer("RED-T", "Other Tamer", CardColor::Red, &[]))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .deck(0, &deck)
        .deck(1, &["FILLER"; 5])
        .memory(3)
        .start();
    r.set_first_player(0);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    (r, h)
}

#[test]
fn bt26_036_on_play_adds_fairy() {
    let (mut r, h) = setup(&["FILLER", "FAIRY", "RED-T"]);
    r.fire_on_play(0, h.index as usize);
    pick_reveal(&mut r, 0, "FAIRY");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["FAIRY".to_string()]);
    assert_eq!(r.deck_size(0), 6);
}

#[test]
fn bt26_036_green_tamer_is_legal_red_tamer_is_not() {
    let (mut r, h) = setup(&["RED-T", "GREEN-T", "FILLER"]);
    r.fire_on_play(0, h.index as usize);
    let v = r.pending_selection_view().expect("reveal prompt");
    assert_eq!(
        v.valid_action_ids.len(),
        1,
        "only the green Tamer qualifies"
    );
    pick_reveal(&mut r, 0, "GREEN-T");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["GREEN-T".to_string()]);
}

#[test]
fn bt26_036_has_when_moving_timing() {
    use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
    let (r, _) = setup(&["FILLER", "FILLER", "FILLER"]);
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card
        .effects
        .iter()
        .any(|c| matches!(c, CompiledClause::Triggered(t)
        if t.when.contains(&CompiledTiming::OnPlay) && t.when.contains(&CompiledTiming::OnMove))));
}

#[test]
fn bt26_036_inherited_may_suspend_opponent_digimon() {
    let (mut r, _) = setup(&["FILLER", "FILLER", "FILLER"]);
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let v = r.pending_selection_view().expect("optional suspend pick");
    assert!(v.is_optional, "'you may suspend'");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[opp.index as usize].is_suspended);
}

#[test]
fn bt26_036_declining_refunds_once_per_turn() {
    let (mut r, _) = setup(&["FILLER", "FILLER", "FILLER"]);
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!r.game.players[1].battle_area[opp.index as usize].is_suspended);
    // DCGO RemoveUse: the OPT was not spent, so a later attack re-offers it.
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    assert!(
        r.pending_selection_view().is_some(),
        "re-offered after a decline"
    );
}

#[test]
fn bt26_036_inherited_silent_without_unsuspended_target() {
    let (mut r, _) = setup(&["FILLER", "FILLER", "FILLER"]);
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
}
