//! BT26-042 Okuwamon — Lv.5 Green, Insectoid/Titan/TS.
//!
//! [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers.
//! Then, 1 of their Digimon or Tamers can't unsuspend until their turn ends.
//! [On Play] [When Attacking] [Once Per Turn] Until your opponent's turn ends,
//! 1 of your [Insectoid] or [Titan] trait Digimon gains <Piercing> and +3000 DP.
//! Inherited: [All Turns] [Once Per Turn] When this Digimon deletes your
//! opponent's Digimon in battle, trash their top security card.
//!
//! DCGO: BT26/Green/BT26_042.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword, ModifierType};

const CARD_ID: &str = "BT26-042";

fn setup() -> DebugRunner {
    let mut carrier = digimon("CARRIER", "Carrier", CardColor::Green, 6, 9, &[]);
    carrier.dp = Some(9000);
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-042 in embedded DSL pack")
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
        .add_card(digimon("OPP", "Opp", CardColor::Red, 3, 3, &[]))
        .add_card(tamer("OPPT", "Opp Tamer", CardColor::Red, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(1, &["FILLER", "FILLER", "FILLER"])
        .memory(3)
        .start();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.turn_count = 1;
    r
}

#[test]
fn bt26_042_wd_suspend_then_lock_tamer() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    let t = r.place_on_field(1, "OPPT", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    let v = r.pending_selection_view().expect("suspend pick");
    assert!(!v.is_optional, "the suspend is mandatory");
    assert_eq!(non_pass(&r).len(), 2, "Digimon and Tamer");
    pick_side_field(&mut r, 0, opp);
    let v = r.pending_selection_view().expect("lock pick");
    assert!(!v.is_optional);
    pick_side_field(&mut r, 0, t);
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[opp.index as usize].is_suspended);
    assert!(r.modifiers().has(t, ModifierType::CannotUnsuspend));
}

#[test]
fn bt26_042_wd_no_opponent_permanents_is_silent() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
}

#[test]
fn bt26_042_wa_grants_piercing_and_3000_once_per_turn() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let beast = r.place_on_field(0, "BEAST", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, h);
    // Mandatory; only Okuwamon itself is [Insectoid]/[Titan].
    let v = r.pending_selection_view();
    if v.is_some() {
        assert_eq!(non_pass(&r).len(), 1);
        pick_first(&mut r, 0);
    }
    let _ = r.auto_resolve();
    assert!(r.game.has_keyword(h, Keyword::Piercing));
    assert!(!r.game.has_keyword(beast, Keyword::Piercing));
    assert_eq!(r.effective_dp(h), Some(10000));
    // [Once Per Turn]: a second attack trigger does nothing more.
    fire(&mut r, EffectTiming::WhenAttacking, h);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(h), Some(10000));
    // Lasts through the opponent's turn.
    r.end_turn();
    assert!(r.game.has_keyword(h, Keyword::Piercing));
    assert_eq!(r.effective_dp(h), Some(10000));
}

#[test]
fn bt26_042_inherited_battle_deletion_trashes_opponent_security() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.attack_digimon(carrier, opp, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 2);
}

#[test]
fn bt26_042_inherited_fires_on_opponents_turn_too() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.turn_player_idx = 1;
    r.attack_digimon(opp, carrier, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 2, "[All Turns]");
}
