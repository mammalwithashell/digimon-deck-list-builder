//! EX11-041 Oblivimon — behavioral tests.
//!
//! Printed:
//!   {Security} [End of Opponent's Turn] Play this card without paying the cost.
//!   [On Play] [When Digivolving] Flip your opponent's top face-down security
//!     card face up and <De-Digivolve 1> 1 of their Digimon. Then, if it's
//!     their turn, this Digimon may digivolve into [Invisimon] in the hand
//!     without paying the cost.
//!   [Your Turn] When any of your Digimon check face-up security cards, you may
//!     place this Digimon's top stacked card face up as the bottom security card.
//!   Inherited: [Your Turn] This Digimon's attack target can't change.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};

#[test]
fn ex11_041_on_play_flips_opp_security_dedigivolves_and_inherited_locks_target() {
    let mut opp3 = make_test_card("EX11-041-O3", "OppLv3");
    opp3.card_kind = CardKind::Digimon;
    opp3.level = Some(3);
    opp3.colors = vec![CardColor::Blue];
    opp3.dp = Some(2000);

    let mut opp4 = make_test_card("EX11-041-O4", "OppLv4");
    opp4.card_kind = CardKind::Digimon;
    opp4.level = Some(4);
    opp4.colors = vec![CardColor::Blue];
    opp4.dp = Some(5000);

    let mut host = make_test_card("EX11-041-HOST", "Invisimon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(6);
    host.colors = vec![CardColor::Black];
    host.dp = Some(12000);

    let mut r = DebugRunner::builder()
        .dsl_card("EX11-041")
        .unwrap()
        .add_card(opp3)
        .add_card(opp4)
        .add_card(host)
        .security(1, &["EX11-041-O3"])
        .start();
    r.skip_mulligan();

    let obl = r.place_on_field(0, "EX11-041", Some(0));
    let opp = r.place_stack(1, &["EX11-041-O3", "EX11-041-O4"]);

    r.fire_on_play(0, obl.index as usize);
    let sel = r
        .pending_selection_view()
        .expect("EX11-041 [On Play] must prompt to <De-Digivolve 1> 1 opp Digimon");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert_eq!(
        r.game.players[1].face_up_security.len(),
        1,
        "EX11-041 [On Play] must flip opponent's top face-down security card face up"
    );
    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "EX11-041 [On Play] must <De-Digivolve 1> opponent's Digimon"
    );

    // Inherited: [Your Turn] This Digimon's attack target can't change.
    let stacked = r.place_stack(0, &["EX11-041", "EX11-041-HOST"]);
    r.game.tick_declarative_effects();
    assert!(
        r.game
            .modifiers
            .has(stacked, ModifierType::CannotSwitchAttackTarget),
        "EX11-041 inherited must prevent attack target change on Your Turn"
    );
}
