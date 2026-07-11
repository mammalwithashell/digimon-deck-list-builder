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

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

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
        .add_card(opp_digimon("MY-RED", "My Red Digimon", 3000))
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

/// Regression (user report): "can't use options if there aren't any targets".
///
/// Rules: an Option's use legality is phase + cost payable (1-3-11-1) + the
/// color requirement — never the availability of effect targets. Effects are
/// performed "whenever possible" (15-1-5); with no eligible target the [Main]
/// effect simply does nothing and the card is still used and trashed. DCGO
/// agrees: ST1_15.cs `CanUseCondition` is the generic
/// `CanTriggerOptionMainEffect` (identity plumbing only); the target check
/// lives INSIDE `ActivateCoroutine` (`if (HasMatchConditionPermanent(...))`).
///
/// Here the opponent's only Digimon has 6000 DP (> 4000), so zero targets are
/// eligible — the Option must still be offered by the action mask and resolve
/// as a no-op.
#[test]
fn st1_15_usable_with_no_eligible_targets_and_resolves_as_noop() {
    let mut runner = runner();
    // A red Digimon of our own satisfies the Option color requirement — the
    // ONLY use requirement besides phase + cost.
    runner.place_on_field(0, "MY-RED", Some(0));
    // Only an INELIGIBLE opponent Digimon (6000 DP > 4000) on the field.
    runner.place_on_field(1, "OPP-BIG", Some(0));

    // 1. The action mask must offer the hand play (color req is met by the
    //    opponent-independent check further down the pipeline; the missing
    //    piece was the synthetic target-availability condition gating use).
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[0], 1.0,
        "ST1-15 must be usable from hand with zero eligible targets \
         (option use legality = phase + cost + color, per 1-3-11-1 + DCGO)"
    );

    // 2. Playing it through the production Option pipeline succeeds: cost
    //    paid, card leaves hand, no selection installs, nothing is deleted,
    //    the Option is trashed. (`play_option_from_hand` is what the action
    //    decoder invokes for the masked bit.)
    let hand_before = runner.hand_size(0);
    let mem_before = runner.memory();
    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        !matches!(result, OptionPlayResult::Invalid),
        "the Option pipeline must accept the use with zero eligible targets; got {result:?}"
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "the Option must actually be used (leave the hand)"
    );
    assert_eq!(
        runner.memory(),
        mem_before - 6,
        "the printed use cost (6) must be paid"
    );
    assert!(
        runner.pending_selection().is_none(),
        "with zero eligible targets no delete selection may install"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the ineligible 6000-DP Digimon must survive"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "ST1-15"),
        "the used Option must be placed in its owner's trash"
    );
}

/// Same report, empty opponent field: usable even with NO opponent Digimon at
/// all (e.g. to thin the hand / trigger "when you use an Option" effects).
#[test]
fn st1_15_usable_with_empty_opponent_field() {
    let mut runner = runner();
    // Color requirement satisfied; the opponent's field stays empty.
    runner.place_on_field(0, "MY-RED", Some(0));
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[0], 1.0,
        "ST1-15 must be usable from hand even when the opponent controls no Digimon"
    );
}
