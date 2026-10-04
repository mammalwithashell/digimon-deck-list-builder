//! BT26-041 Hudiemon — Lv.4 Green/Yellow, Insectoid/NSp.
//!
//! [On Play] [When Digivolving] Add your top security card to the hand and
//! <Recovery +1>. Then, you may suspend 1 Digimon.
//! Inherited: [Your Turn] [Once Per Turn] When this Digimon wins a battle,
//! gain 1 memory.
//!
//! DCGO: BT26/Green/BT26_041.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-041";

fn setup(security: &[&str]) -> DebugRunner {
    let mut carrier = digimon("CARRIER", "Carrier", CardColor::Green, 5, 7, &[]);
    carrier.dp = Some(9000);
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-041 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(filler("SECTOP"))
        .add_card(filler("DECKTOP"))
        .add_card(carrier)
        .add_card(digimon("OPP", "Opp", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("LARVA3", "Larva", CardColor::Red, 3, 3, &["Larva"]))
        .deck(0, &["FILLER", "FILLER", "FILLER", "DECKTOP"])
        .deck(1, &["FILLER"; 6])
        .security(0, security)
        .memory(3)
        .start();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.turn_count = 1;
    r
}

#[test]
fn bt26_041_on_play_security_to_hand_recovery_then_may_suspend() {
    let mut r = setup(&["FILLER", "SECTOP"]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::OnPlay, h);
    assert_eq!(hand_ids(&r, 0), vec!["SECTOP".to_string()]);
    assert_eq!(
        security_ids(&r, 0),
        vec!["FILLER".to_string(), "DECKTOP".to_string()],
        "<Recovery +1> places the deck's top card on top of security"
    );
    let v = r.pending_selection_view().expect("optional suspend");
    assert!(v.is_optional);
    pick_any_field(&mut r, 0, opp);
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[opp.index as usize].is_suspended);
}

#[test]
fn bt26_041_empty_security_still_recovers() {
    let mut r = setup(&[]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    // Only Hudiemon itself is suspendable — decline.
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).is_empty());
    assert_eq!(security_ids(&r, 0), vec!["DECKTOP".to_string()]);
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
}

#[test]
fn bt26_041_alt_digivolve_from_larva_level_3() {
    use digimon_dsl::compiled::CompiledAltPathKind;
    let r = setup(&[]);
    let card = r.compiled_card(CARD_ID).unwrap();
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("alt digivolve");
    assert_eq!(
        path.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(2))
    );
}

#[test]
fn bt26_041_inherited_win_gains_one_memory_once_per_turn() {
    let mut r = setup(&["FILLER"]);
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.attack_digimon(carrier, opp, false);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 4);
    r.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    let opp2 = r.place_on_field(1, "OPP", Some(0));
    r.attack_digimon(carrier, opp2, false);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 4, "[Once Per Turn]");
}

#[test]
fn bt26_041_inherited_silent_on_opponents_turn() {
    let mut r = setup(&["FILLER"]);
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.turn_player_idx = 1;
    let before = r.game.memory;
    r.attack_digimon(opp, carrier, false);
    let _ = r.auto_resolve();
    assert_eq!(r.game.memory, before);
}
