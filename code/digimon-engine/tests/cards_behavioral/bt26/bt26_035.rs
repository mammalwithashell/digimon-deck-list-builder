//! BT26-035 Morphomon — Lv.3 Green, Insectoid/NSp.
//!
//! [When Moving] [On Play] You may suspend 1 Digimon.
//! Inherited: [Your Turn] [Once Per Turn] When this Digimon wins a battle, 1 of
//! your [Insectoid] or [NSp] trait Digimon may digivolve into an [Insectoid] or
//! [NSp] trait Digimon card in the hand with the cost reduced by 1.
//!
//! DCGO: BT26/Green/BT26_035.cs (inherited: RemoveUse when no Digimon picked).

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-035";

fn setup() -> DebugRunner {
    let mut carrier = digimon("CARRIER", "Carrier", CardColor::Green, 4, 5, &["Insectoid"]);
    carrier.dp = Some(8000);
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-035 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(carrier)
        .add_card(digimon_evo(
            "INSECT5",
            "Insect Five",
            CardColor::Green,
            5,
            7,
            &["Insectoid"],
            4,
            3,
        ))
        .add_card(digimon_evo(
            "PLAIN5",
            "Plain Five",
            CardColor::Green,
            5,
            7,
            &["Beast"],
            4,
            3,
        ))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 3, 3, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.turn_count = 1;
    r
}

fn win_battle(r: &mut DebugRunner) -> PermanentHandle {
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.attack_digimon(carrier, opp, false);
    carrier
}

#[test]
fn bt26_035_on_play_may_suspend_any_digimon() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, digimon_engine::enums::EffectTiming::OnPlay, h);
    let v = r.pending_selection_view().expect("optional suspend pick");
    assert!(v.is_optional, "'you may suspend'");
    // Both battle areas are candidates (own Morphomon + opponent's Digimon).
    assert_eq!(non_pass(&r).len(), 2);
    pick_any_field(&mut r, 0, opp);
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[opp.index as usize].is_suspended);
}

#[test]
fn bt26_035_on_play_decline_suspends_nothing() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, digimon_engine::enums::EffectTiming::OnPlay, h);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!r.game.players[1].battle_area[opp.index as usize].is_suspended);
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
}

#[test]
fn bt26_035_has_when_moving_timing() {
    use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card
        .effects
        .iter()
        .any(|c| matches!(c, CompiledClause::Triggered(t)
        if t.when.contains(&CompiledTiming::OnPlay) && t.when.contains(&CompiledTiming::OnMove))));
}

#[test]
fn bt26_035_inherited_win_digivolves_insectoid_cost_minus_one() {
    let mut r = setup();
    push_hand(&mut r, 0, "INSECT5");
    push_hand(&mut r, 0, "PLAIN5");
    let carrier = win_battle(&mut r);
    let v = r.pending_selection_view().expect("optional Digimon pick");
    assert!(v.is_optional);
    pick_side_field(&mut r, 0, carrier);
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "only the [Insectoid] card qualifies");
    pick_hand(&mut r, 0, "INSECT5");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, carrier), "INSECT5");
    assert_eq!(r.memory(), 5 - (3 - 1));
}

#[test]
fn bt26_035_inherited_decline_refunds_once_per_turn() {
    let mut r = setup();
    push_hand(&mut r, 0, "INSECT5");
    let carrier = win_battle(&mut r);
    assert!(r.pending_selection_view().is_some());
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, carrier), "CARRIER");
    // RemoveUse: a second win this turn re-offers it.
    r.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    let opp2 = r.place_on_field(1, "OPP", Some(0));
    r.attack_digimon(carrier, opp2, false);
    assert!(
        r.pending_selection_view().is_some(),
        "re-offered after decline"
    );
}

#[test]
fn bt26_035_inherited_silent_on_opponents_turn() {
    let mut r = setup();
    push_hand(&mut r, 0, "INSECT5");
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.turn_player_idx = 1;
    r.attack_digimon(opp, carrier, false);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, carrier), "CARRIER");
}
