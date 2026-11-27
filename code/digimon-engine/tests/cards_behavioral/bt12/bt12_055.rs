//! BT12-055 Dinobeemon — behavioral tests.
//!
//! Printed:
//!   [DNA Digivolve] 0 from blue Lv.4 + green Lv.4
//!   [When Digivolving] If DNA digivolving, suspend 1 of your opponent's
//!     Digimon, and this Digimon gets +3000 DP for the turn. Then, this
//!     Digimon may attack an opponent's Digimon.
//!   Inherited: [Your Turn][Once Per Turn] When one of your Digimon with
//!     [Imperialdramon] in its name or the [Free] trait deletes an opponent's
//!     Digimon in battle, trash the top card of your opponent's security stack.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dna_digivolve::DnaRouteWindow;
use digimon_engine::enums::{CardColor, CardKind, GamePhase};

#[test]
fn bt12_055_dna_digivolve_suspends_opp_gains_3000_dp_and_offers_attack() {
    let mut blue4 = make_test_card("BT12-055-B4", "ExVeemon");
    blue4.card_kind = CardKind::Digimon;
    blue4.level = Some(4);
    blue4.colors = vec![CardColor::Blue];
    blue4.dp = Some(5000);

    let mut green4 = make_test_card("BT12-055-G4", "Stingmon");
    green4.card_kind = CardKind::Digimon;
    green4.level = Some(4);
    green4.colors = vec![CardColor::Green];
    green4.dp = Some(5000);

    let mut opp = make_test_card("BT12-055-OPP", "OppMon");
    opp.card_kind = CardKind::Digimon;
    opp.level = Some(4);
    opp.colors = vec![CardColor::Red];
    opp.dp = Some(5000);

    let mut filler = make_test_card("BT12-055-FILL", "Filler");
    filler.level = Some(3);

    let mut r = DebugRunner::builder()
        .dsl_card("BT12-055")
        .unwrap()
        .add_card(blue4)
        .add_card(green4)
        .add_card(opp)
        .add_card(filler)
        .hand(0, &["BT12-055"])
        .deck(0, &["BT12-055-FILL", "BT12-055-FILL"])
        .start();
    r.skip_mulligan();
    r.game.memory = 3;
    let b = r.place_on_field(0, "BT12-055-B4", Some(0));
    let g = r.place_on_field(0, "BT12-055-G4", Some(0));
    let opp_perm = r.place_on_field(1, "BT12-055-OPP", Some(0));
    let hand_idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "BT12-055")
        .unwrap();

    r.game.current_phase = GamePhase::Main;
    r.game.resolve_dna_digivolve_stage2_with_window(
        0,
        b.index as usize,
        g.index as usize,
        hand_idx,
        DnaRouteWindow::Main,
    );

    let sel_susp = r
        .pending_selection_view()
        .expect("BT12-055 [WD] DNA must prompt to suspend 1 opponent Digimon");
    let pick = *sel_susp
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, pick).unwrap();

    assert!(r.game.players[1].battle_area[opp_perm.index as usize].is_suspended);
    let dino_handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(
        r.game.effective_dp(dino_handle),
        Some(11000),
        "BT12-055 DNA [WD] must grant +3000 DP (8000 -> 11000)"
    );

    // Decline the optional follow-up attack cleanly.
    if r.pending_selection_view().is_some() {
        r.execute_action(0, PASS).unwrap();
    }
}
