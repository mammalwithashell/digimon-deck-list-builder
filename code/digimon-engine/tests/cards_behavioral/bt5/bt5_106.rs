//! BT5-106 Demonic Disaster - Option, Purple, Cost 1.
//!
//! # Card text (cards.json)
//!
//! [Main] You may delete 1 of your Digimon to unsuspend 1 of your purple Digimon.
//!
//! Security Effect [Security] You may play 1 level 3 purple Digimon card from
//! your trash without paying its memory cost. Any [On Play] effects on Digimon
//! played with this effect don't activate.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT5/Purple/BT5_106.cs (submodule not initialized)
//!
//! # Patterns this test covers
//! - Option [Main] with a visible sacrifice selection.
//! - Follow-up visible own purple Digimon unsuspend selection.
//! - Security free-play from trash is blocked pending PUPPETS-G030 On Play
//!   suppression provenance.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

fn digimon(id: &str, color: CardColor) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card
}

fn option_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT5-106")
        .expect("BT5-106 YAML parses and compiles")
        .add_card(digimon("PURPLE-COST", CardColor::Purple))
        .add_card(digimon("PURPLE-TARGET", CardColor::Purple))
        .add_card(digimon("YELLOW-TARGET", CardColor::Yellow))
        .hand(0, &["BT5-106"])
        .memory(10)
        .start()
}

#[test]
fn bt5_106_is_purple_option_cost_1() {
    let runner = option_runner();
    let compiled = runner
        .compiled_card("BT5-106")
        .expect("BT5-106 compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(1));
    assert!(
        compiled
            .color
            .contains(&digimon_dsl::compiled::CompiledColor::Purple),
        "Demonic Disaster must be a purple Option"
    );
}

#[test]
fn bt5_106_has_main_clause_only_for_supported_slice() {
    let runner = option_runner();
    let compiled = runner
        .compiled_card("BT5-106")
        .expect("BT5-106 compiled card");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        1,
        "only the supported Main slice should ship until Security On Play suppression is supported"
    );
    let main = triggered[0];
    assert_eq!(main.scope, CompiledScope::FaceUp);
    assert!(main.when.contains(&CompiledTiming::MainFromHand));
}

#[test]
fn bt5_106_main_prompts_for_sacrifice_then_purple_unsuspend_target() {
    let mut runner = option_runner();
    let cost = runner.place_on_field(0, "PURPLE-COST", Some(0));
    let target = runner.place_on_field(0, "PURPLE-TARGET", Some(0));
    let yellow = runner.place_on_field(0, "YELLOW-TARGET", Some(0));
    runner.game.suspend(cost);
    runner.game.suspend(target);
    runner.game.suspend(yellow);

    assert!(runner.game.activate_hand_main(0, 0));

    let first = runner
        .pending_selection_view()
        .expect("Main effect must first prompt for the Digimon to delete");
    assert_eq!(first.kind, SelectionKind::OwnField);
    assert!(
        first.is_optional,
        "the sacrifice selection is optional because the card says 'You may'"
    );
    assert_eq!(
        first.valid_action_ids,
        vec![
            encode_attack(0, 0),
            encode_attack(0, 1),
            encode_attack(0, 2)
        ],
        "all own Digimon should be legal sacrifice choices"
    );

    runner
        .execute_action(0, encode_attack(0, 0))
        .expect("choose PURPLE-COST to delete");

    assert_eq!(
        runner.battle_area_size(0),
        2,
        "the selected sacrifice Digimon must be deleted before the unsuspend target is chosen"
    );

    let second = runner
        .pending_selection_view()
        .expect("after paying the cost, Main must prompt for a purple Digimon to unsuspend");
    assert_eq!(second.kind, SelectionKind::OwnField);
    assert!(
        !second.is_optional,
        "the follow-up target choice is mandatory after cost payment"
    );
    assert_eq!(
        second.valid_action_ids,
        vec![encode_attack(0, 0)],
        "only the remaining suspended purple Digimon should be a legal unsuspend target"
    );

    runner
        .execute_action(0, encode_attack(0, 0))
        .expect("choose PURPLE-TARGET to unsuspend");

    assert!(
        !runner.game.player(0).battle_area[target.index as usize - 1].is_suspended,
        "the chosen purple Digimon should be unsuspended"
    );
    assert!(
        runner.game.player(0).battle_area[yellow.index as usize - 1].is_suspended,
        "non-purple Digimon must not be affected"
    );
}

#[test]
fn bt5_106_main_decline_sacrifice_short_circuits_effect() {
    let mut runner = option_runner();
    let cost = runner.place_on_field(0, "PURPLE-COST", Some(0));
    let target = runner.place_on_field(0, "PURPLE-TARGET", Some(0));
    runner.game.suspend(cost);
    runner.game.suspend(target);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .execute_action(0, PASS)
        .expect("decline the optional sacrifice");

    assert!(
        runner.pending_selection().is_none(),
        "declining the optional sacrifice must not continue to the unsuspend choice"
    );
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "no Digimon should be deleted"
    );
    assert!(
        runner.game.player(0).battle_area[target.index as usize].is_suspended,
        "target stays suspended when the effect is declined"
    );
}

#[test]
fn bt5_106_main_requires_suspended_purple_unsuspend_target() {
    let mut runner = option_runner();
    let cost = runner.place_on_field(0, "PURPLE-COST", Some(0));
    let purple = runner.place_on_field(0, "PURPLE-TARGET", Some(0));
    let yellow = runner.place_on_field(0, "YELLOW-TARGET", Some(0));
    runner.game.suspend(cost);
    runner.game.suspend(yellow);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .execute_action(0, encode_attack(0, 0))
        .expect("choose PURPLE-COST to delete");

    assert!(
        runner.pending_selection().is_none(),
        "no follow-up target prompt should install when no suspended purple Digimon remains"
    );
    assert!(
        !runner.game.player(0).battle_area[purple.index as usize - 1].is_suspended,
        "unsuspended purple Digimon should not be offered or changed"
    );
    assert!(
        runner.game.player(0).battle_area[yellow.index as usize - 1].is_suspended,
        "suspended non-purple Digimon should not be offered"
    );
}

#[test]
#[ignore = "pending: PUPPETS-G030 — security play-from-trash with On Play suppression provenance"]
fn bt5_106_security_prompts_for_level_3_purple_digimon_in_trash() {
    todo!("Security effect must select a level 3 purple Digimon from trash and play it for free");
}

#[test]
#[ignore = "pending: PUPPETS-G030 — security play-from-trash with On Play suppression provenance"]
fn bt5_106_security_suppresses_on_play_effects_of_played_digimon() {
    todo!("played Digimon's On Play effects must not activate");
}

// ─── Failure-mode audit (PR #456 cluster 3f — effect-driven play / sacrifice) ──

/// **Adjacent edge-case (cluster 3f):** BT5-106 [Main] is a "you may"
/// sacrifice. With NO own Digimon on field, the optional sacrifice
/// pool is empty. The effect must short-circuit cleanly: no
/// pending_selection installed, no field changes, no crash. Guards
/// against the empty-candidate-pool branch silently keeping the
/// selection alive.
#[test]
fn bt5_106_main_with_no_own_digimon_short_circuits_without_panic() {
    let mut runner = option_runner();
    // No place_on_field calls — P0 has no Digimon.
    assert_eq!(runner.battle_area_size(0), 0);

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "[Main] still fires; the optional inner step is what's empty"
    );

    // Selection should not park (no targets to pick from).
    assert!(
        runner.game.pending_selection.is_none(),
        "no pending_selection should be installed when sacrifice candidate pool is empty"
    );
    assert_eq!(
        runner.battle_area_size(0),
        0,
        "no field state can change when the optional sacrifice has no targets"
    );
}
