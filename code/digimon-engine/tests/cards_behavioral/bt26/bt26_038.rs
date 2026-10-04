//! BT26-038 Kuwagamon — Lv.4 Green, Insectoid/Titan/TS.
//!
//! [When Moving] [On Play] [When Digivolving] You may suspend 1 Digimon. Then,
//! 1 of your Digimon with the [Insectoid] or [Titan] trait gets +3000 DP until
//! your opponent's turn ends.
//! Inherited: [Your Turn] [Once Per Turn] When this Digimon wins a battle, 1 of
//! your [Insectoid] or [Titan] trait Digimon may digivolve into an [Insectoid]
//! or [Titan] trait Digimon card in the hand with the cost reduced by 1.
//!
//! DCGO: BT26/Green/BT26_038.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-038";

fn setup() -> DebugRunner {
    let mut carrier = digimon("CARRIER", "Carrier", CardColor::Green, 5, 7, &["Titan"]);
    carrier.dp = Some(9000);
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-038 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(carrier)
        .add_card(digimon(
            "BEAST",
            "Beast",
            CardColor::Green,
            4,
            5,
            &["Beast"],
        ))
        .add_card(digimon_evo(
            "TITAN6",
            "Titan Six",
            CardColor::Green,
            6,
            12,
            &["Titan"],
            5,
            4,
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

#[test]
fn bt26_038_on_play_suspend_then_buff_insectoid() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let beast = r.place_on_field(0, "BEAST", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::OnPlay, h);
    let v = r.pending_selection_view().expect("optional suspend");
    assert!(v.is_optional);
    pick_any_field(&mut r, 0, opp);
    // Buff pick: only Kuwagamon (Insectoid/Titan) — not the [Beast].
    let v = r.pending_selection_view().expect("buff pick");
    assert!(!v.is_optional, "the +3000 is mandatory");
    assert_eq!(non_pass(&r).len(), 1);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[opp.index as usize].is_suspended);
    assert_eq!(r.effective_dp(h), Some(5000 + 3000));
    assert_eq!(r.effective_dp(beast), Some(4000));
}

#[test]
fn bt26_038_decline_suspend_still_buffs() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!r.game.players[1].battle_area[opp.index as usize].is_suspended);
    assert_eq!(r.effective_dp(h), Some(8000));
}

#[test]
fn bt26_038_buff_lasts_through_opponents_turn() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, h);
    // Only own Kuwagamon is suspendable; decline it.
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    r.end_turn();
    assert_eq!(r.effective_dp(h), Some(8000), "still up on opponent's turn");
}

#[test]
fn bt26_038_has_when_moving_timing() {
    use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card
        .effects
        .iter()
        .any(|c| matches!(c, CompiledClause::Triggered(t)
        if t.when.contains(&CompiledTiming::OnMove)
            && t.when.contains(&CompiledTiming::WhenDigivolving))));
}

fn win_battle(r: &mut DebugRunner) -> PermanentHandle {
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.attack_digimon(carrier, opp, false);
    carrier
}

#[test]
fn bt26_038_inherited_win_digivolves_titan_cost_minus_one() {
    let mut r = setup();
    push_hand(&mut r, 0, "TITAN6");
    let carrier = win_battle(&mut r);
    pick_side_field(&mut r, 0, carrier);
    pick_hand(&mut r, 0, "TITAN6");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, carrier), "TITAN6");
    assert_eq!(r.memory(), 5 - (4 - 1));
}

#[test]
fn bt26_038_inherited_decline_refunds_once_per_turn() {
    let mut r = setup();
    push_hand(&mut r, 0, "TITAN6");
    let carrier = win_battle(&mut r);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, carrier), "CARRIER");
    r.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    let opp2 = r.place_on_field(1, "OPP", Some(0));
    r.attack_digimon(carrier, opp2, false);
    assert!(
        r.pending_selection_view().is_some(),
        "re-offered after decline"
    );
}
