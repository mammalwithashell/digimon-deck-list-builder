//! BT13-112 Omnimon

use std::sync::Arc;

use digimon_engine::action::space::{
    encode_attack, PASS, encode_breeding_select, encode_breeding_source_select,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const ON_PLAY_GAIN: i16 = 1;

struct OnPlayGainMemory;

impl CardEffect for OnPlayGainMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("On Play: gain memory")
            .process(|ctx| ctx.gain_memory(ON_PLAY_GAIN))
            .build()]
    }
}

fn test_digimon(card_id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::White];
    card.level = Some(6);
    card.dp = Some(dp);
    card.play_cost = 10;
    card
}

fn royal_knight(card_id: &str, name: &str) -> CardData {
    let mut card = test_digimon(card_id, name, 11000);
    card.traits = vec!["Royal Knight".to_string()];
    card
}

fn runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-112")
        .expect("BT13-112 must load from embedded DSL pack")
        .add_card(test_digimon("KING", "Breeding Host", 2000))
        .add_card(royal_knight("RK-ALPHA", "Alphamon"))
        .add_card(royal_knight("RK-OMEGA-A", "Omnimon"))
        .add_card(royal_knight("RK-OMEGA-B", "Omnimon"))
        .add_card(test_digimon("NON-RK", "Non Royal", 5000))
        .add_card(test_digimon("OPP-DGM", "Opponent", 7000))
        .memory(0)
        .start();
    runner.register_effect("RK-ALPHA", Arc::new(OnPlayGainMemory));
    runner.register_effect("RK-OMEGA-A", Arc::new(OnPlayGainMemory));
    runner.register_effect("RK-OMEGA-B", Arc::new(OnPlayGainMemory));
    runner
}

fn install_breeding_stack(runner: &mut DebugRunner) {
    let handle = runner.place_stack(
        0,
        &["RK-ALPHA", "RK-OMEGA-A", "RK-OMEGA-B", "NON-RK", "KING"],
    );
    let perm = runner.game.players[0]
        .battle_area
        .remove(handle.index as usize);
    runner.game.players[0].breeding_area = Some(perm);
}

fn source_action(runner: &DebugRunner, card_id: &str) -> u16 {
    let idx = runner.game.players[0]
        .breeding_area
        .as_ref()
        .expect("breeding stack")
        .card_sources
        .iter()
        .position(|source| runner.game.card_data[source.data_index].card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} source in breeding"));
    encode_breeding_source_select(0, idx as u16).expect("breeding source action")
}

/// Accept the printed "You may" gate (DCGO `SetUpActivateClass(..., isOptional:
/// true, ...)`), surfaced as an outer optional-trigger prompt.
fn accept_you_may(runner: &mut DebugRunner) {
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "the [On Play] effect is optional (\"You may\") and must prompt first"
    );
    runner.accept_optional_trigger().expect("accept You may");
}

fn install_non_royal_breeding_stack(runner: &mut DebugRunner) {
    let handle = runner.place_stack(0, &["NON-RK", "KING"]);
    let perm = runner.game.players[0]
        .battle_area
        .remove(handle.index as usize);
    runner.game.players[0].breeding_area = Some(perm);
}

fn find_field_card(runner: &DebugRunner, card_id: &str) -> PermanentHandle {
    let index = runner.game.players[0]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} on field"));
    PermanentHandle {
        player: 0,
        index: index as u8,
    }
}

#[test]
fn bt13_112_loads_from_dsl() {
    DebugRunner::builder()
        .dsl_card("BT13-112")
        .expect("BT13-112 must load from embedded DSL pack")
        .start();
}

#[test]
fn bt13_112_delete_branch_deletes_one_opponent_digimon() {
    let mut runner = runner();
    // Royal Knight breeding sources make the play branch legal too, so the
    // branch prompt appears and the delete branch is an explicit pick.
    install_breeding_stack(&mut runner);
    let opp = runner.place_on_field(1, "OPP-DGM", None);
    let omnimon = runner.place_on_field(0, "BT13-112", None);

    runner.fire_on_play(0, omnimon.index as usize);
    accept_you_may(&mut runner);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));

    runner.execute_branch(0).expect("choose delete branch");
    runner
        .execute_action(0, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "delete branch removes the selected opponent Digimon"
    );
}

#[test]
fn bt13_112_source_play_branch_plays_distinct_royal_knights_trashes_breeding_and_grants_rush() {
    let mut runner = runner();
    install_breeding_stack(&mut runner);
    runner.place_on_field(1, "OPP-DGM", None);
    let omnimon = runner.place_on_field(0, "BT13-112", None);
    let memory_before = runner.game.memory;

    runner.fire_on_play(0, omnimon.index as usize);
    accept_you_may(&mut runner);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
    runner
        .execute_branch(1)
        .expect("choose breeding-source play branch");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::BreedingPermanent)
    );
    runner
        .execute_action(0, encode_breeding_select(0).unwrap())
        .expect("choose breeding Digimon");

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect { picked: 0, .. })
        ),
        "source selection should open as a count-capped multi-pick"
    );
    let alpha_action = source_action(&runner, "RK-ALPHA");
    runner
        .execute_action(0, alpha_action)
        .expect("choose Alphamon source");

    let omega_a_action = source_action(&runner, "RK-OMEGA-A");
    let omega_b_action = source_action(&runner, "RK-OMEGA-B");
    let pending = runner
        .pending_selection_view()
        .expect("source selection re-arms after first pick");
    assert!(pending.valid_action_ids.contains(&omega_a_action));
    assert!(pending.valid_action_ids.contains(&omega_b_action));

    runner
        .execute_action(0, omega_a_action)
        .expect("choose one Omnimon source");
    // The duplicate-name Omnimon source is masked, so the batch commits. The
    // played Royal Knights' own [On Play] effects then trigger (DCGO
    // `PlayPermanentCards(..., activateETB: true)`; the printed text does not
    // say "without activating"), so the only thing that may still be pending is
    // the ordering / optional prompt of those triggers — never another source
    // pick.
    assert!(
        !matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect { .. })
        ),
        "duplicate-name Omnimon source is masked, so the batch commits"
    );

    assert!(
        runner.game.players[0].breeding_area.is_none(),
        "a successful source play trashes the breeding Digimon"
    );
    assert_eq!(
        runner.battle_area_size(0),
        3,
        "Omnimon plus two different-name Royal Knight sources are in battle"
    );
    assert_eq!(
        runner.trash_size(0),
        3,
        "remaining duplicate source, non-Royal-Knight source, and breeding top card are trashed"
    );
    for card_id in ["BT13-112", "RK-ALPHA", "RK-OMEGA-A"] {
        let handle = find_field_card(&runner, card_id);
        assert!(
            runner.game.has_keyword(handle, Keyword::Rush),
            "{card_id} should have Rush for the turn"
        );
    }

    // Resolve the two queued [On Play] triggers (order prompt, if any).
    let mut guard = 0;
    while runner.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 10, "trigger resolution must terminate");
        let action = runner
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .iter()
            .copied()
            .find(|id| *id != PASS)
            .expect("an accept / order action");
        runner.execute_action(0, action).expect("resolve trigger prompt");
    }
    assert_eq!(
        runner.game.memory,
        memory_before + 2 * ON_PLAY_GAIN,
        "both played Royal Knights' own [On Play] effects activate"
    );
}

/// W1 regression guard — the activation condition's "opponent has a Digimon"
/// leg used to read `any_permanent: { of: opponent, predicate: { kind: digimon } }`.
/// `predicate` is not a DSL key (ExistentialPredicate flattens its predicate),
/// so it was silently dropped and ANY opponent permanent (e.g. a lone Tamer)
/// satisfied it. DCGO BT13_112.cs CanActivateCondition requires an opponent
/// battle-area DIGIMON (or a breeding Digimon with a Royal Knight source).
#[test]
fn bt13_112_does_not_activate_with_only_an_opponent_tamer_and_no_breeding_sources() {
    let mut tamer = make_test_card("OPP-TAMER", "Opponent Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-112")
        .expect("BT13-112 must load from embedded DSL pack")
        .add_card(tamer)
        .memory(0)
        .start();
    runner.place_on_field(1, "OPP-TAMER", None);
    let omnimon = runner.place_on_field(0, "BT13-112", None);

    runner.fire_on_play(0, omnimon.index as usize);
    assert_eq!(
        runner.pending_kind(),
        None,
        "with only an opponent Tamer (no opponent Digimon) and no breeding Royal \
         Knight sources, the [On Play] effect must not activate"
    );
}

/// "You may" — DCGO BT13_112.cs registers both activations with
/// `isOptional: true`. Declining leaves the opponent's Digimon untouched.
#[test]
fn bt13_112_effect_is_optional_and_can_be_declined() {
    let mut runner = runner();
    runner.place_on_field(1, "OPP-DGM", None);
    let omnimon = runner.place_on_field(0, "BT13-112", None);

    runner.fire_on_play(0, omnimon.index as usize);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
    assert!(runner.pending_is_optional(), "the You-may gate admits PASS");
    runner.decline_optional_trigger().expect("decline");

    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.battle_area_size(1), 1, "declining deletes nothing");
}

/// Issue 1 — the breeding-area branch requires a breeding DIGIMON with at least
/// one [Royal Knight] trait Digimon among its digivolution cards (DCGO
/// `CanSelectPermanentCondition1`: `permanent.DigivolutionCards.Count(IsDigimon
/// && HasRoyalKnightTraits && CanPlayAsNewPermanent) >= 1`). A breeding stack
/// whose sources are all non-Royal-Knight does not satisfy it, so with no
/// opponent Digimon the effect has nothing to do and must not activate.
#[test]
fn bt13_112_breeding_sources_without_a_royal_knight_do_not_enable_activation() {
    let mut runner = runner();
    install_non_royal_breeding_stack(&mut runner);
    let omnimon = runner.place_on_field(0, "BT13-112", None);

    runner.fire_on_play(0, omnimon.index as usize);
    assert_eq!(
        runner.pending_kind(),
        None,
        "a breeding stack with no [Royal Knight] source does not satisfy the \
         activation condition"
    );
    assert!(
        runner.game.players[0].breeding_area.is_some(),
        "the breeding Digimon is untouched"
    );
}

/// Issues 1 + 2 — opponent has a Digimon, breeding stack has only non-Royal-
/// Knight sources: only the delete branch is legal. DCGO
/// (`SetBool(canSelectDelete)`) goes straight to the delete target pick with no
/// branch prompt — a one-option choice is not a choice.
#[test]
fn bt13_112_only_delete_branch_legal_skips_the_branch_prompt() {
    let mut runner = runner();
    install_non_royal_breeding_stack(&mut runner);
    let opp = runner.place_on_field(1, "OPP-DGM", None);
    let omnimon = runner.place_on_field(0, "BT13-112", None);

    runner.fire_on_play(0, omnimon.index as usize);
    accept_you_may(&mut runner);
    assert_ne!(
        runner.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "with only the delete branch legal, no branch prompt is shown"
    );
    runner
        .execute_action(0, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");
    assert_eq!(runner.battle_area_size(1), 0);
    assert!(
        runner.game.players[0].breeding_area.is_some(),
        "the breeding branch did not run"
    );
}

/// Issue 2 — no opponent Digimon, breeding stack has Royal Knight sources: only
/// the play branch is legal, so DCGO (`SetBool(false)`) runs it without a
/// branch prompt.
#[test]
fn bt13_112_only_play_branch_legal_skips_the_branch_prompt() {
    let mut runner = runner();
    install_breeding_stack(&mut runner);
    let omnimon = runner.place_on_field(0, "BT13-112", None);

    runner.fire_on_play(0, omnimon.index as usize);
    accept_you_may(&mut runner);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::BreedingPermanent),
        "with only the breeding branch legal, it runs directly"
    );
}

/// Both branches legal → the branch prompt offers exactly the two branches.
#[test]
fn bt13_112_both_branches_legal_prompts_with_two_options() {
    let mut runner = runner();
    install_breeding_stack(&mut runner);
    runner.place_on_field(1, "OPP-DGM", None);
    let omnimon = runner.place_on_field(0, "BT13-112", None);

    runner.fire_on_play(0, omnimon.index as usize);
    accept_you_may(&mut runner);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
    assert_eq!(runner.pending_action_count(), 2);
}
