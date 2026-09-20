//! BT24-030 Neptunemon.

use digimon_dsl::compiled::{
    CompiledAggregateSelector, CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledStep,
};
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;
use digimon_engine::PermanentHandle;

const YAML: &str = include_str!("../../../cards/bt24/BT24-030.yaml");

#[test]
fn bt24_030_has_printed_stats_alt_path_cost_reduction_and_fewest_material_sweep() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-030 YAML parses")
        .start();
    let compiled = runner
        .compiled_card("BT24-030")
        .expect("BT24-030 compiled card present");

    assert_eq!(compiled.card, "BT24-030");
    assert_eq!(compiled.name, "Neptunemon");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Blue, CompiledColor::Purple]
    );
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    for trait_name in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }
    assert!(compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(3))
            && path.from.as_ref().is_some_and(|from| {
                from.all_of.iter().any(|pred| pred.level_eq == Some(5))
                    && from.all_of.iter().any(|pred| {
                        pred.any_of
                            .iter()
                            .any(|inner| inner.trait_has.as_deref() == Some("TS"))
                    })
            })
    }));

    let cost_reduction = compiled
        .effects
        .iter()
        .find(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                    when_playing_this: true,
                    amount: Some(5),
                    ..
                })
            )
        })
        .expect("play-cost reduction clause");
    assert!(
        matches!(cost_reduction, CompiledClause::Declarative(_)),
        "cost reduction should lower as a declarative clause"
    );

    let sweep = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.process.iter().any(|step| {
                matches!(
                    step,
                    CompiledStep::ForEach { over, .. }
                        if over
                            .materials_count_matches_aggregate
                            .is_some_and(|(selector, _)| selector == CompiledAggregateSelector::FewestMaterials)
                )
            }) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("fewest-material sweep clause");
    assert!(
        sweep.process.iter().any(|step| matches!(
            step,
            CompiledStep::ForEach { body, .. }
                if body.iter().any(|inner| matches!(
                    inner,
                    CompiledStep::ReturnToDeck { include_sources: true, .. }
                ))
        )),
        "fewest-material sweep should return full matching stacks to deck bottom"
    );
}

#[test]
fn bt24_030_play_cost_reduces_only_when_opponent_has_two_digimon() {
    let mut reduced = neptunemon_runner()
        .hand(0, &["BT24-030"])
        .memory(20)
        .start();
    let printed_cost = reduced.game.players[0].hand[0].play_cost(&reduced.game.card_data);
    reduced.place_on_field(1, "OPP-A", Some(0));
    reduced.place_on_field(1, "OPP-B", Some(0));

    let memory_before = reduced.memory();
    reduced.play(0, 0).expect("play Neptunemon with reduction");
    let paid_cost = memory_before - reduced.memory();
    assert_eq!(printed_cost, 12);
    assert_eq!(
        paid_cost, 7,
        "12-cost Neptunemon should cost 7 when opponent has 2 Digimon"
    );

    let mut unreduced = neptunemon_runner()
        .hand(0, &["BT24-030"])
        .memory(20)
        .start();
    unreduced.place_on_field(1, "OPP-A", Some(0));

    let memory_before = unreduced.memory();
    unreduced
        .play(0, 0)
        .expect("play Neptunemon without reduction");
    let paid_cost = memory_before - unreduced.memory();
    assert_eq!(
        paid_cost, 12,
        "Neptunemon should cost its printed 12 when opponent has fewer than 2 Digimon"
    );
}

#[test]
fn bt24_030_on_play_bottom_decks_all_opponent_digimon_tied_for_fewest_materials() {
    let mut runner = neptunemon_runner()
        .add_card(make_test_card("SRC", "Source"))
        .start();
    let neptunemon = runner.place_on_field(0, "BT24-030", Some(0));
    let zero_a = runner.place_on_field(1, "OPP-A", Some(0));
    let zero_b = runner.place_on_field(1, "OPP-B", Some(0));
    let _stacked = runner.place_stack(1, &["SRC", "OPP-C"]);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(neptunemon));
    runner.game.drain_effect_queue();

    assert!(
        !permanent_exists(&runner, zero_a, "OPP-A"),
        "first sourceless opponent Digimon should be returned"
    );
    assert!(
        !permanent_exists(&runner, zero_b, "OPP-B"),
        "second sourceless opponent Digimon tied for fewest should be returned"
    );
    assert!(battle_area_contains(&runner, 1, "OPP-C"));
    assert!(deck_contains(&runner, 1, "OPP-A"));
    assert!(deck_contains(&runner, 1, "OPP-B"));
}

#[test]
fn bt24_030_self_suspend_may_unsuspend_once_per_turn() {
    let mut runner = neptunemon_runner().start();
    let neptunemon = runner.place_on_field(0, "BT24-030", Some(0));

    runner.game.suspend(neptunemon);
    runner
        .auto_resolve()
        .expect("resolve optional self-unsuspend");

    assert!(
        !runner.game.players[0].battle_area[neptunemon.index as usize].is_suspended,
        "Neptunemon should unsuspend after its own on-suspend trigger resolves"
    );

    runner.game.suspend(neptunemon);
    runner
        .auto_resolve()
        .expect("second suspend should not produce another OPT unsuspend");
    assert!(
        runner.game.players[0].battle_area[neptunemon.index as usize].is_suspended,
        "self-unsuspend is once per turn"
    );
}

/// Declaring an attack IS suspending the attacker (general_rule.pdf 11-2-1:
/// "The turn player can suspend their Digimon in the battle area and make an
/// attack declaration"; 11-1-4 names the triggered effects that resolve at
/// declaration). DCGO suspends the attacker through `SuspendPermanentsClass
/// (...).Tap()` (`AttackProcess.cs:166`), which fires `OnTappedAnyone` — the
/// timing BT24_030.cs keys its unsuspend on. So Neptunemon's "[All Turns][Once
/// Per Turn] When this Digimon suspends, it may unsuspend" must open on its own
/// attack, and the attack still proceeds to the security check
/// (G-ENGINE-ATTACK-SUSPENSION-NO-ONSUSPEND; exam BT24-030#effect#3).
#[test]
fn bt24_030_attack_declaration_suspension_triggers_may_unsuspend() {
    let mut runner = neptunemon_runner()
        .security(1, &["OPP-A", "OPP-B"])
        .deck(0, &["OPP-A"; 4])
        .deck(1, &["OPP-A"; 4])
        .start();
    let neptunemon = runner.place_on_field(0, "BT24-030", Some(0));

    runner.attack_player(neptunemon, 1, false);
    assert!(
        runner.game.pending_selection.is_some() && runner.pending_is_optional(),
        "attack-declaration suspension must open Neptunemon's optional unsuspend"
    );
    runner
        .auto_resolve()
        .expect("accept the unsuspend and finish the attack");

    assert!(
        !runner.game.players[0].battle_area[neptunemon.index as usize].is_suspended,
        "Neptunemon should be unsuspended after accepting its on-suspend trigger"
    );
    assert_eq!(
        runner.security_count(1),
        1,
        "the attack continues to its security check after the unsuspend"
    );
}

#[test]
fn bt24_030_protects_matching_digimon_from_opponent_effects_by_suspending() {
    let mut runner = neptunemon_runner().start();
    let neptunemon = runner.place_on_field(0, "BT24-030", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));

    runner.game.return_to_hand_from_effect(target, 1);

    assert!(
        runner.pending_is_optional(),
        "replacement protection should be player-declinable"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Neptunemon protection");
    runner
        .auto_resolve()
        .expect("resolve Neptunemon self-unsuspend trigger after paying protection cost");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "TS-TARGET"),
        "protected TS Digimon should remain in the battle area"
    );
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|card| { card.card_id(&runner.game.card_data) == "TS-TARGET" }),
        "the original opponent effect should be cancelled"
    );
    assert!(
        !runner.game.players[0].battle_area[neptunemon.index as usize].is_suspended,
        "Neptunemon's separate on-suspend trigger may unsuspend it after the protection cost"
    );
}

fn neptunemon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-030 YAML loads")
        .add_card(make_digimon("OPP-A", &["Plain"]))
        .add_card(make_digimon("OPP-B", &["Plain"]))
        .add_card(make_digimon("OPP-C", &["Plain"]))
        .add_card(make_digimon("TS-TARGET", &["TS"]))
}

fn make_digimon(id: &str, traits: &[&str]) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn permanent_exists(runner: &DebugRunner, handle: PermanentHandle, card_id: &str) -> bool {
    runner
        .game
        .players
        .get(handle.player as usize)
        .and_then(|player| player.battle_area.get(handle.index as usize))
        .is_some_and(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

fn battle_area_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

fn deck_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .deck
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

// ─── Option-trash timing vs a trigger raised by the NON-turn player mid-body ──
//
// Exam BT24-030#effect#4 (qa/dcgo-exams/BT24/BT24-030-effect4.yaml): the TURN
// player uses Gaia Force (ST1-16) on the NON-turn player's Neptunemon; that
// player pays the would-leave protection by suspending Neptunemon, which
// raises Neptunemon's own "[All Turns][Once Per Turn] When this Digimon
// suspends, it may unsuspend" (effect#3) while the Option's [Main] is still
// resolving.
//
// Rules: the used Option is trashed at the timing its 1st [Main] has resolved,
// as PENDING PROCESSING (general_rule.pdf 9-1-5, p.19); pending processing that
// coincides with other processing is ordered like simultaneous triggering
// (18-1-2, p.40 -> 15-4-3); and 15-4-3-5-1/-2 (p.23) resolve ALL of the turn
// player's simultaneous items before ANY of the non-turn player's. The Option's
// trash belongs to the turn player (its user) and the unsuspend trigger to the
// non-turn player, so the trash strictly precedes the prompt -- this sub-case is
// NOT order-ambiguous. DCGO agrees: `CardController.cs`
// `UseOptionClass.UseOption` calls `AddTrashCard(card)` right after the
// OptionSkill process, before the stacked triggers resolve.
// G-ENGINE-OPTION-TRASH-VS-ON-USE-TRIGGER-ORDER.
#[test]
fn bt24_030_used_option_is_trashed_before_the_non_turn_players_on_suspend_prompt() {
    let mut runner = neptunemon_runner()
        .dsl_card("ST1-16")
        .expect("ST1-16 is in the embedded DSL pack")
        .hand(1, &["ST1-16"])
        .memory(10)
        .start();
    runner.set_first_player(1);
    assert_eq!(runner.turn_player(), 1, "P1 is the turn player / Option user");
    let neptunemon = runner.place_on_field(0, "BT24-030", Some(0));
    // Gaia Force is red: its user needs a red card in play (4-19 color
    // requirements) -- the exam line uses Tai Kamiya ST1-12 for this.
    runner.place_on_field(1, "OPP-A", Some(0));

    let result = runner.game.play_option_from_hand(1, 0);
    assert!(
        !matches!(result, digimon_engine::selection::OptionPlayResult::Invalid),
        "P1 can use Gaia Force: {result:?}"
    );

    // Gaia Force's [Main] target pick: Neptunemon is P0's only Digimon.
    let view = runner
        .pending_selection_view()
        .expect("Gaia Force target pick");
    assert_eq!(view.selecting_player, 1);
    let pick = view.valid_action_ids[0];
    runner
        .execute_action(1, pick)
        .expect("target Neptunemon for deletion");

    // P0's would-leave replacement: pay by suspending Neptunemon.
    assert!(
        runner.pending_is_optional(),
        "Neptunemon's protection is declinable"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Neptunemon's protection");

    // The suspension raised P0's own optional unsuspend trigger ...
    let view = runner
        .pending_selection_view()
        .expect("P0's [All Turns][OPT] on-suspend prompt");
    assert_eq!(
        view.selecting_player, 0,
        "the non-turn player answers their own on-suspend trigger"
    );
    // ... and the turn player's used Option is ALREADY in their trash.
    let p1_trash: Vec<String> = runner.game.players[1]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        p1_trash,
        vec!["ST1-16".to_string()],
        "the used Option is trashed before the non-turn player's body-raised trigger resolves"
    );

    runner.decline_optional_trigger().expect("decline the unsuspend");
    runner.auto_resolve().ok();
    assert!(
        battle_area_contains(&runner, 0, "BT24-030"),
        "the protection held: Neptunemon stays in the battle area"
    );
    assert!(
        runner.game.players[0].battle_area[neptunemon.index as usize].is_suspended,
        "the paid cost stays visible: Neptunemon is suspended"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        1,
        "the Option is trashed exactly once"
    );
}
