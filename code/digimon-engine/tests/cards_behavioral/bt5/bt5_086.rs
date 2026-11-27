//! BT5-086 Omnimon — behavioral tests.
//!
//! Printed:
//!   [When Digivolving] <Blitz>
//!   [When Digivolving] Unsuspend this Digimon.
//!   [All Turns] When an opponent's effect would delete or return this Digimon
//!     to hand/deck, by trashing 1 level 6 Digimon card in its digivolution
//!     cards, prevent it from leaving.

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt5_086_when_digivolving_unsuspends_self() {
    let mut lv6 = make_test_card("BT5-086-BASE", "WarGreymon");
    lv6.card_kind = CardKind::Digimon;
    lv6.level = Some(6);
    lv6.colors = vec![CardColor::Red];
    lv6.dp = Some(12000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT5-086")
        .unwrap()
        .add_card(lv6)
        .start();
    r.skip_mulligan();
    r.game.memory = 5;
    let perm = r.place_stack(0, &["BT5-086-BASE", "BT5-086"]);
    r.game.players[0].battle_area[perm.index as usize].is_suspended = true;

    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(perm),
    );
    r.game.drain_effect_queue();
    // Two simultaneous [When Digivolving] triggers (<Blitz> + Unsuspend) install a trigger-order prompt.
    while let Some(sel) = r.pending_selection_view() {
        let pick = sel.valid_action_ids[0];
        r.execute_action(0, pick).unwrap();
    }
    assert!(
        !r.game.players[0].battle_area[perm.index as usize].is_suspended,
        "BT5-086 [When Digivolving] must unsuspend this Digimon"
    );
}

#[test]
fn bt5_086_replacement_trashes_lv6_source_to_prevent_opponent_deletion() {
    let mut lv6 = make_test_card("BT5-086-LV6", "MetalGarurumon");
    lv6.card_kind = CardKind::Digimon;
    lv6.level = Some(6);
    lv6.colors = vec![CardColor::Blue];
    lv6.dp = Some(11000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT5-086")
        .unwrap()
        .add_card(lv6)
        .start();
    r.skip_mulligan();
    let perm = r.place_stack(0, &["BT5-086-LV6", "BT5-086"]);

    // Attempt opponent-effect deletion -> installs Replacement selection.
    let _ = r
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    let sel = r
        .pending_selection_view()
        .expect("BT5-086 replacement prompt must be installed");
    assert!(sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();

    // If a source-selection prompt is presented, select the Lv.6 source explicitly.
    if let Some(src_sel) = r.pending_selection_view() {
        let pick = *src_sel
            .valid_action_ids
            .iter()
            .find(|&&a| a != digimon_engine::action::space::PASS)
            .expect("Lv.6 source must be selectable");
        r.execute_action(0, pick).unwrap();
    }

    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "BT5-086 must survive opponent deletion after trashing Lv.6 source"
    );
    assert_eq!(
        r.game.players[0].battle_area[0].card_sources.len(),
        1,
        "Lv.6 digivolution card must be trashed as replacement cost"
    );
    assert_eq!(r.game.players[0].trash.len(), 1);
}
