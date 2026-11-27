//! EX9-018 MetalMamemon — behavioral tests.
//!
//! Printed:
//!   When this card would be played, by trashing 1 [Cyborg] or [Ver.2] trait
//!     card from your hand, reduce the play cost by 2.
//!   [On Play] [When Digivolving] By placing 1 Digimon card from your trash
//!     face down as this Digimon's bottom digivolution card, from 1 of your
//!     opponent's Digimon, trash any 1 digivolution card for each of this
//!     Digimon's face-down digivolution cards. Then, return 1 of their Digimon
//!     with no digivolution cards to the bottom of the deck.
//!   Inherited: [End of Your Turn] [Once Per Turn] Unsuspend 1 of your Digimon.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};

#[test]
fn ex9_018_on_play_bottoms_opp_without_sources_and_inherited_unsuspends_eot() {
    let mut opp = make_test_card("EX9-018-OPP", "OppMon");
    opp.card_kind = CardKind::Digimon;
    opp.level = Some(4);
    opp.colors = vec![CardColor::Blue];
    opp.dp = Some(5000);

    let mut host = make_test_card("EX9-018-HOST", "Machinedramon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(6);
    host.colors = vec![CardColor::Black];
    host.dp = Some(12000);

    let mut r = DebugRunner::builder()
        .dsl_card("EX9-018")
        .unwrap()
        .add_card(opp)
        .add_card(host)
        .start();
    r.skip_mulligan();

    // [On Play] with empty trash skips face-down placement and bottoms opp Digimon with 0 sources.
    let mm = r.place_on_field(0, "EX9-018", Some(0));
    r.place_on_field(1, "EX9-018-OPP", Some(0));
    r.fire_on_play(0, mm.index as usize);
    let sel = r
        .pending_selection_view()
        .expect("EX9-018 [On Play] must prompt to bottom-deck opp Digimon with no sources");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    assert!(r.game.players[1].battle_area.is_empty());
    assert_eq!(r.game.players[1].deck.len(), 1);

    // Inherited [End of Your Turn][OPT]: unsuspend 1 of your Digimon.
    let stacked = r.place_stack(0, &["EX9-018", "EX9-018-HOST"]);
    r.game.players[0].battle_area[stacked.index as usize].is_suspended = true;
    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        digimon_engine::selection::TriggerSource::Permanent(stacked),
    );
    r.game.drain_effect_queue();
    let sel_u = r
        .pending_selection_view()
        .expect("EX9-018 inherited [EoT] must prompt to unsuspend 1 of your Digimon");
    let pick_u = *sel_u
        .valid_action_ids
        .iter()
        .rev()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, pick_u).unwrap();
    assert!(!r.game.players[0].battle_area[stacked.index as usize].is_suspended);
}
