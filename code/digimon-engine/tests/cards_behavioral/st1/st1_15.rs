//! ST1-15 Giga Destroyer — opponent-field capped-multi-select target encoding.
//!
//! [Main] Delete up to 2 of your opponent's Digimon with 4000 DP or less.
//!
//! Regression guard for the UI bug where the capped-multi-select over the
//! opponent's battle area surfaced its targets in an encoding/kind the
//! frontend's field-click router could not consume (`SelectionKind::
//! CountCappedMultiSelect` + `encode_attack(player, slot)` ids), leaving
//! "delete an opponent's Digimon" prompts unclickable. The engine must expose
//! these targets the SAME way single-target opponent-field prompts do (see
//! `EffectContext::install_field_selection`): `SelectionKind::OppField` with
//! `encode_attack(0, slot)` = `ATTACK_START + slot` ids, disambiguated to the
//! opponent's side purely by the `OppField` kind.

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

fn opp_digimon(card_id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.level = Some(3);
    card.dp = Some(dp);
    card.play_cost = 3;
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST1-15")
        .expect("ST1-15 YAML loads")
        .add_card(opp_digimon("OPP-SMALL", "Small Opp", 2000))
        .add_card(opp_digimon("OPP-BIG", "Big Opp", 6000))
        .hand(0, &["ST1-15"])
        .memory(10)
        .start()
}

#[test]
fn st1_15_main_exposes_opponent_targets_as_oppfield_field_slots() {
    let mut runner = runner();
    // Opponent (player 1): one eligible (<=4000 DP) Digimon at slot 0, one
    // ineligible (6000 DP) at slot 1.
    let small = runner.place_on_field(1, "OPP-SMALL", Some(0));
    let _big = runner.place_on_field(1, "OPP-BIG", Some(1));

    runner.play(0, 0).expect("play Giga Destroyer from hand");

    // The selection MUST present as the field-target kind the frontend's
    // field-click router understands, not the bespoke multi-select tag.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "capped-multi-select over the opponent's field must use OppField"
    );

    let valid = &runner.pending_selection().unwrap().valid_action_ids;

    // The eligible Digimon (slot 0) is offered as encode_attack(0, slot) =
    // ATTACK_START + slot — identical to single-target opponent-field prompts.
    assert!(
        valid.contains(&encode_attack(0, small.index as u16)),
        "eligible opponent Digimon at slot {} must be offered as \
         encode_attack(0, slot); got valid_action_ids={:?}",
        small.index,
        valid,
    );

    // The 6000-DP Digimon (slot 1) is filtered out by `dp_lte: 4000`.
    assert!(
        !valid.contains(&encode_attack(0, 1)),
        "ineligible 6000-DP Digimon must not be a target; got {:?}",
        valid,
    );
}
