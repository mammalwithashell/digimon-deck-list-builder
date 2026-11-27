//! EX8-015 WarGrowlmon (X Antibody) — behavioral tests.
//!
//! Printed:
//!   [When Digivolving] Until the end of your opponent's turn, this Digimon
//!     can't be returned to hands or decks and gets +3000 DP. Then, if
//!     [WarGrowlmon] or [X Antibody] is in this Digimon's digivolution cards,
//!     delete 1 of your opponent's Digimon with 10000 or less DP.
//!   Inherited: <Security A. +1>

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};

#[test]
fn ex8_015_wd_grants_bounce_immunity_dp_and_deletes_10k_with_wargrowlmon_source() {
    let mut wg_base = make_test_card("EX8-015-BASE", "WarGrowlmon");
    wg_base.card_kind = CardKind::Digimon;
    wg_base.level = Some(5);
    wg_base.colors = vec![CardColor::Red];
    wg_base.dp = Some(8000);

    let mut opp10k = make_test_card("EX8-015-O10K", "Opp10k");
    opp10k.card_kind = CardKind::Digimon;
    opp10k.level = Some(6);
    opp10k.colors = vec![CardColor::Blue];
    opp10k.dp = Some(10000);

    let mut host = make_test_card("EX8-015-HOST", "Gallantmon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(6);
    host.colors = vec![CardColor::Red];
    host.dp = Some(12000);

    let mut r = DebugRunner::builder()
        .dsl_card("EX8-015")
        .unwrap()
        .add_card(wg_base)
        .add_card(opp10k)
        .add_card(host)
        .start();
    r.skip_mulligan();

    let wgx = r.place_stack(0, &["EX8-015-BASE", "EX8-015"]);
    r.place_on_field(1, "EX8-015-O10K", Some(0));

    r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenDigivolving, digimon_engine::selection::TriggerSource::Permanent(wgx));
    r.game.drain_effect_queue();
    let sel = r
        .pending_selection_view()
        .expect("EX8-015 [WD] with [WarGrowlmon] source must prompt to delete opp <=10000 DP");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();

    assert_eq!(r.game.effective_dp(wgx), Some(11000));
    assert!(r.game.modifiers.has(wgx, ModifierType::CannotReturnToHand));
    assert!(r.game.modifiers.has(wgx, ModifierType::CannotBeReturnedToDeck));
    assert!(r.game.players[1].battle_area.is_empty());

    // Inherited: <Security A. +1>.
    let stacked = r.place_stack(0, &["EX8-015", "EX8-015-HOST"]);
    assert!(r.game.has_keyword(stacked, Keyword::SecurityAttackPlus(1)));
}
