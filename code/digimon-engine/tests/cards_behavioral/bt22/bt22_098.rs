//! BT22-098 Unique Emblem: Fable Waltz - Option, Cost 3, Yellow, LIBERATOR.
//!
//! # Card text (cards.json)
//!
//! [Main] You may play 1 [Shoemon] or [Arisa Kinosaki] from your hand or trash
//! without paying the cost. Then, place this card in the battle area.
//!
//! [Your Turn] When any of your [Arisa Kinosaki] suspend, <Delay> (By trashing
//! this card after the placing turn, activate the effect below.)
//! 1 of your [Puppet] trait Digimon may digivolve into a [Puppet] and
//! [LIBERATOR] trait Digimon card in the hand with the digivolution cost
//! reduced by 3.
//!
//! Local cards.json also stores inherited text:
//! Security Effect [Security] Activate this card's [Main] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT22/Yellow/BT22_098.cs
//!
//! # Patterns this test covers
//! - C5/E2-adjacent: Option [Main] origin-preserving union-zone (hand ∪ trash)
//!   free play with a player-visible, optional ("you may") selection.
//! - Integrated Option pipeline: `play_option_from_hand` parks the union-zone
//!   selection, then post-resolution places BT22-098 as a Delayed battle-area
//!   Option ("Then, place this card in the battle area").
//! - Inherited [Security] re-runs the [Main] union-zone play plus the same
//!   battle-area placement.
//! - Event-gated Delay on a matching Arisa suspend event after the placing turn.
//! - Delay body selection sequence: Puppet Digimon base, then Puppet+LIBERATOR
//!   hand card, then effect_initiated_digivolve with cost reduced by 3.
//!
//! # Gap-closure record
//! - PUPPETS-G014 (origin-preserving union-zone play): CLOSED — Puppets
//!   substrate sweep 2026-05-20. The [Main] hand-or-trash play is expressed as a
//!   single filtered, origin-preserving union-zone choice (`select_union_zone` +
//!   `play_union_bound_free`). Both hand-origin and trash-origin branches are
//!   covered behaviorally below.
//! - PUPPETS-G009 (Standard Delay [Main] activation): CLOSED — same sweep.
//! - PUPPETS-G033 (Option-pipeline integrated resolution: pending optional play
//!   + post-resolution battle-area placement): the engine path is proven by the
//!   sibling cards P-105 / LM-054 (`play_option_from_hand` returns `Pending`,
//!   the parked selection is driven, then `advance_pending_option` →
//!   `dispose_option` places the card as a Delayed Option). The tracker entry
//!   pre-dates that sweep; these tests verify the same integrated flow for
//!   BT22-098, so no slice remains `#[ignore]`d.

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledCostDelta, CompiledDeclarativeClause,
    CompiledPlayerRef, CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

fn yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/bt22/BT22-098.yaml"))
        .expect("BT22-098 YAML must exist at cards/bt22/BT22-098.yaml")
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT22-098 YAML must parse and compile")
        .memory(10)
        .start()
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn shoemon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Shoemon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Puppet".to_string(), "LIBERATOR".to_string()];
    card
}

fn arisa(id: &str) -> CardData {
    let mut card = make_test_card(id, "Arisa Kinosaki");
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 4;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["LIBERATOR".to_string()];
    card
}

fn shoeshoemon(id: &str) -> CardData {
    let mut card = make_test_card(id, "ShoeShoemon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Puppet".to_string(), "LIBERATOR".to_string()];
    card
}

fn color_anchor(id: &str) -> CardData {
    let mut card = make_test_card(id, "Color Anchor");
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["LIBERATOR".to_string()];
    card
}

fn puppet_base(id: &str) -> CardData {
    let mut card = make_test_card(id, "Puppet Base");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Puppet".to_string()];
    card
}

fn puppet_liberator_evo(id: &str) -> CardData {
    let mut card = make_test_card(id, "Puppet Liberator Evo");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Puppet".to_string(), "LIBERATOR".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 3,
        memory_cost: 3,
    }];
    card
}

fn non_target(id: &str) -> CardData {
    let mut card = make_test_card(id, "Not a named target");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Beast".to_string()];
    card
}

fn predicate_contains_name(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.name_is.as_deref() == Some(needle)
        || predicate.name_contains.as_deref() == Some(needle)
        || predicate
            .name_in
            .iter()
            .flatten()
            .any(|name| name == needle)
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_name(part, needle))
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_name(part, needle))
}

fn predicate_contains_trait(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.trait_has.as_deref() == Some(needle)
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_trait(part, needle))
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_trait(part, needle))
}

fn predicate_contains_kind(predicate: &CompiledPredicate, kind: CompiledCardKind) -> bool {
    predicate.kind == Some(kind)
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_kind(part, kind))
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_kind(part, kind))
}

fn battle_area_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner
        .game
        .player(player)
        .battle_area
        .iter()
        .any(|permanent| {
            permanent
                .card_sources
                .iter()
                .any(|source| source.card_id(&runner.game.card_data) == card_id)
        })
}

fn hand_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner
        .game
        .player(player)
        .hand
        .iter()
        .any(|source| source.card_id(&runner.game.card_data) == card_id)
}

#[test]
fn bt22_098_yaml_parses_and_compiles() {
    let _runner = runner();
}

#[test]
fn bt22_098_is_yellow_liberator_option_cost_3() {
    let runner = runner();
    let compiled = runner.compiled_card("BT22-098").expect("compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.traits, vec!["LIBERATOR".to_string()]);
}

#[test]
fn bt22_098_main_hand_slice_selects_shoemon_or_arisa_and_plays_free() {
    let runner = runner();
    let compiled = runner.compiled_card("BT22-098").expect("compiled card");
    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::MainFromHand) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("main_from_hand clause");

    // Since G014 substrate landed, the clause uses select_union_zone (hand ∪ trash)
    // rather than the earlier hand-only select_hand approximation.
    let select = main
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectUnionZone {
                filter, optional, ..
            } => Some((filter, optional)),
            _ => None,
        })
        .expect("G014 union-zone slice must install a union-zone selection");

    assert!(
        *select.1,
        "printed 'may play 1' must surface PASS at the union-zone selection"
    );
    assert!(
        predicate_contains_name(select.0, "Shoemon")
            && predicate_contains_name(select.0, "Arisa Kinosaki"),
        "union-zone selection must filter to [Shoemon] or [Arisa Kinosaki]"
    );
    assert!(
        main.process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayUnionBoundFree { .. })),
        "selected union-zone target must be played without paying the cost"
    );
}

#[test]
fn bt22_098_security_mirrors_supported_main_hand_slice() {
    let runner = runner();
    let compiled = runner.compiled_card("BT22-098").expect("compiled card");
    let security = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnSecurity) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("inherited security clause");

    assert_eq!(security.scope, CompiledScope::Inherited);
    assert!(
        !security.optional,
        "Security activates the Main slice; the target play inside remains optional"
    );
    // Since G014 substrate landed, the security clause mirrors the union-zone [Main] slice.
    let select = security
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectUnionZone {
                filter, optional, ..
            } => Some((filter, optional)),
            _ => None,
        })
        .expect("security mirror must install a union-zone selection");
    assert!(*select.1, "the mirrored 'may play 1' choice stays optional");
    assert!(
        predicate_contains_name(select.0, "Shoemon")
            && predicate_contains_name(select.0, "Arisa Kinosaki"),
        "security union-zone selection must filter to [Shoemon] or [Arisa Kinosaki]"
    );
    assert!(
        security
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayUnionBoundFree { .. })),
        "security mirror must play the selected union-zone target without paying cost"
    );
}

#[test]
fn bt22_098_has_event_gated_delay_for_own_arisa_suspend() {
    let runner = runner();
    let compiled = runner.compiled_card("BT22-098").expect("compiled card");
    let delay = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                active_when,
                process,
                ..
            }) => Some((trigger, active_when, process)),
            _ => None,
        })
        .expect("Delay clause");

    assert_eq!(*delay.0, CompiledTiming::OnSuspend);
    let active_when = delay
        .1
        .as_ref()
        .expect("Delay must be gated to the Arisa suspend event");
    assert_eq!(active_when.your_turn, Some(true));
    assert_eq!(active_when.event_target_owner, Some(CompiledPlayerRef::You));
    assert_eq!(
        active_when.event_card_name_contains.as_deref(),
        Some("Arisa Kinosaki")
    );

    let own_perm = delay
        .2
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectOwnPermanent {
                filter, optional, ..
            } => Some((filter, optional)),
            _ => None,
        })
        .expect("Delay body must select a Puppet base Digimon");
    assert!(*own_perm.1, "printed Delay digivolve says 'may'");
    assert!(predicate_contains_kind(
        own_perm.0,
        CompiledCardKind::Digimon
    ));
    assert!(predicate_contains_trait(own_perm.0, "Puppet"));

    let evo = delay
        .2
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectHand { filter, .. } => Some(filter),
            _ => None,
        })
        .expect("Delay body must select the evolution card from hand");
    assert!(predicate_contains_kind(evo, CompiledCardKind::Digimon));
    assert!(predicate_contains_trait(evo, "Puppet"));
    assert!(predicate_contains_trait(evo, "LIBERATOR"));

    assert!(delay.2.iter().any(|step| {
        matches!(
            step,
            CompiledStep::EffectInitiatedDigivolve {
                cost: CompiledCostDelta::Reduce(3),
                ignore_requirements: false,
                ..
            }
        )
    }));
}

#[test]
fn bt22_098_main_hand_target_is_masked_and_played_without_extra_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT22-098 YAML parses")
        .add_card(shoemon("SHOE"))
        .add_card(shoeshoemon("SHOESHOE"))
        .add_card(color_anchor("ANCHOR"))
        .add_card(non_target("OTHER"))
        .add_card(filler("FILL"))
        .hand(0, &["BT22-098", "SHOE", "SHOESHOE", "OTHER"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let memory_before = runner.memory();
    assert!(runner.game.activate_hand_main(0, 0));

    let view = runner
        .pending_selection_view()
        .expect("Main union-zone selection must be pending");
    // Since G014 substrate landed, the selection is now a union-zone pick
    // (hand ∪ trash) rather than a hand-only pick.
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "selection should be a union-zone pick (hand ∪ trash)"
    );
    assert!(
        runner.pending_is_optional(),
        "PASS must be legal because the target play is optional"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only exact Shoemon/Arisa should be legal union targets; ShoeShoemon must not match [Shoemon]"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose Shoemon");
    runner.auto_resolve().expect("resolve Main");

    assert!(
        battle_area_contains(&runner, 0, "SHOE"),
        "selected Shoemon should be played"
    );
    assert!(
        !battle_area_contains(&runner, 0, "OTHER"),
        "non-matching hand card must not be played"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "activating the clause directly should not charge Shoemon's play cost"
    );
}

/// PUPPETS-G033 — the integrated Option pipeline. Playing BT22-098 through
/// `play_option_from_hand` parks the [Main] union-zone selection (the engine
/// returns `Pending`); driving that selection to completion then places
/// BT22-098 in the battle area as an event-gated Delayed Option ("Then, place
/// this card in the battle area").
#[test]
fn bt22_098_option_pipeline_plays_target_then_places_as_delayed_option() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT22-098 YAML parses")
        .add_card(shoemon("SHOE"))
        .add_card(color_anchor("ANCHOR"))
        .add_card(filler("FILL"))
        .hand(0, &["BT22-098", "SHOE"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "ANCHOR", Some(0));
    runner.game.enter_main_phase();

    // The [Main] clause installs a union-zone selection, so the pipeline
    // suspends with `Pending` instead of resolving synchronously.
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "the [Main] union-zone selection must park the Option pipeline"
    );

    // The parked selection is the optional hand-or-trash play; pick SHOE.
    let view = runner
        .pending_selection_view()
        .expect("Main union-zone selection must be pending");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "parked selection should be the union-zone pick"
    );
    assert!(
        runner.pending_is_optional(),
        "the 'you may play' choice keeps PASS legal"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose Shoemon from hand");
    runner
        .auto_resolve()
        .expect("resolve Main play and battle-area placement");

    assert!(
        battle_area_contains(&runner, 0, "SHOE"),
        "the chosen Shoemon should be played for free"
    );
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "BT22-098")
        .expect("BT22-098 should be placed as a Delay option after the Main body resolves");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnSuspend),
            ..
        }
    ));
}

#[test]
fn bt22_098_delay_after_arisa_suspend_exposes_base_then_hand_evo_choices() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT22-098 YAML parses")
        .add_card(arisa("ARISA"))
        .add_card(puppet_base("BASE"))
        .add_card(puppet_liberator_evo("EVO"))
        .add_card(non_target("OTHER"))
        .add_card(filler("FILL"))
        .hand(0, &["BT22-098", "EVO", "OTHER"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Trashed
    );

    let arisa = runner.place_on_field(0, "ARISA", Some(0));

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    runner.game.suspend(arisa);

    {
        let view = runner
            .pending_selection_view()
            .expect("Arisa suspend should fire the delayed Puppet base selection");
        assert_eq!(view.kind, SelectionKind::OwnField);
        assert!(
            runner.pending_is_optional(),
            "Delay digivolve is optional at the Puppet base selection"
        );
        assert_eq!(
            view.valid_action_ids.len(),
            1,
            "only the Puppet Digimon base should be legal"
        );
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("choose Puppet base");
    }

    {
        let view = runner
            .pending_selection_view()
            .expect("choosing the base should expose the hand evolution selection");
        assert_eq!(view.kind, SelectionKind::Hand);
        assert_eq!(
            view.valid_action_ids.len(),
            1,
            "only the Puppet+LIBERATOR hand card should be legal"
        );
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("choose Puppet+LIBERATOR evolution");
    }

    runner.auto_resolve().expect("complete Delay digivolve");

    assert!(
        !hand_contains(&runner, 0, "EVO"),
        "selected evolution card should leave hand"
    );
    assert!(
        battle_area_contains(&runner, 0, "EVO"),
        "selected evolution card should become part of the field stack"
    );
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|source| source.card_id(&runner.game.card_data) == "BT22-098"),
        "Delay activation should trash BT22-098 as its cost"
    );
}

/// Negative gate (placing turn): the `<Delay>` text is only activatable
/// "after the placing turn" (RULES_CONTEXT 16-16-3). An Arisa suspend on the
/// same turn BT22-098 was placed must NOT fire the Delay body.
#[test]
fn bt22_098_delay_does_not_fire_on_the_placing_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT22-098 YAML parses")
        .add_card(arisa("ARISA"))
        .add_card(puppet_base("BASE"))
        .add_card(puppet_liberator_evo("EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["BT22-098", "EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Trashed
    );
    let arisa = runner.place_on_field(0, "ARISA", Some(0));

    // Suspend Arisa on the SAME turn BT22-098 was placed.
    runner.game.suspend(arisa);

    assert!(
        runner.pending_selection().is_none(),
        "the Delay must not fire on its placing turn"
    );
    assert!(
        battle_area_contains(&runner, 0, "BT22-098"),
        "BT22-098 stays parked when the Delay does not fire"
    );
    assert!(
        hand_contains(&runner, 0, "EVO"),
        "the evolution card stays in hand because the Delay body never ran"
    );
}

/// Negative gate (event predicate): the Delay is gated by
/// `event_card_name_contains: "Arisa Kinosaki"`. Suspending a non-Arisa
/// permanent after the placing turn must NOT fire the Delay body.
#[test]
fn bt22_098_delay_does_not_fire_when_a_non_arisa_suspends() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT22-098 YAML parses")
        .add_card(puppet_base("BASE"))
        .add_card(puppet_liberator_evo("EVO"))
        .add_card(non_target("OTHER"))
        .add_card(filler("FILL"))
        .hand(0, &["BT22-098", "EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Trashed
    );
    // OTHER is a non-Arisa Digimon (Beast trait, "Not a named target").
    let other = runner.place_on_field(0, "OTHER", Some(0));

    // Advance past the placing turn so the placing-turn gate is satisfied.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    runner.game.suspend(other);

    assert!(
        runner.pending_selection().is_none(),
        "the Delay must not fire when a non-Arisa permanent suspends"
    );
    assert!(
        battle_area_contains(&runner, 0, "BT22-098"),
        "BT22-098 stays parked when the suspend event does not match"
    );
}

/// Cost-firing clause (rule 6): the `<Delay>` activation cost is "by trashing
/// this card". When the Arisa suspend fires the Delay after the placing turn,
/// BT22-098 is trashed even if the optional digivolve body is declined — the
/// trash is the activation cost, not part of the optional "may digivolve".
#[test]
fn bt22_098_delay_trashes_self_as_cost_even_when_digivolve_is_declined() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT22-098 YAML parses")
        .add_card(arisa("ARISA"))
        .add_card(puppet_base("BASE"))
        .add_card(puppet_liberator_evo("EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["BT22-098", "EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Trashed
    );
    let arisa = runner.place_on_field(0, "ARISA", Some(0));

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    runner.game.suspend(arisa);

    // The Delay fires; decline the optional Puppet-base digivolve with PASS.
    let view = runner
        .pending_selection_view()
        .expect("Arisa suspend after the placing turn fires the Delay body");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "the Delay digivolve is the printed 'may', so PASS is legal"
    );
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the Delay digivolve");
    runner.auto_resolve().expect("settle the declined Delay");

    // Body declined — no digivolve happened.
    assert!(
        hand_contains(&runner, 0, "EVO"),
        "declining the Delay leaves the evolution card in hand"
    );
    // ...but the activation cost still trashed BT22-098.
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|source| source.card_id(&runner.game.card_data) == "BT22-098"),
        "the <Delay> cost trashes BT22-098 even when the digivolve is declined"
    );
    assert!(
        !battle_area_contains(&runner, 0, "BT22-098"),
        "BT22-098 must leave the battle area once the Delay activation cost is paid"
    );
}

#[test]
fn bt22_098_main_can_choose_shoemon_or_arisa_from_hand_or_trash_in_one_masked_choice() {
    // Setup: Shoemon in trash (trash-origin), Arisa in hand (hand-origin).
    // Non-matching cards (wrong name) in both zones must be excluded.
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT22-098 YAML parses")
        .add_card(shoemon("SHOE_TRASH"))
        .add_card(arisa("ARISA_HAND"))
        .add_card(non_target("BAD_HAND"))
        .add_card(non_target("BAD_TRASH"))
        .add_card(filler("FILL"))
        // BT22-098 is at hand[0]; ARISA_HAND and BAD_HAND also in hand
        .hand(0, &["BT22-098", "ARISA_HAND", "BAD_HAND"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Manually push SHOE_TRASH and BAD_TRASH into player 0's trash.
    for trash_id in &["SHOE_TRASH", "BAD_TRASH"] {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *trash_id)
            .expect("card registered");
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(data_idx, 0, card_index));
    }

    let memory_before = runner.memory();
    // Activate BT22-098 from hand (it is at index 0).
    assert!(runner.game.activate_hand_main(0, 0));

    // Should see a union-zone selection (SelectionKind::UnionZone or similar).
    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must be pending after Main activation");

    // Must be optional (the card says "You may").
    assert!(
        runner.pending_is_optional(),
        "G014 union-zone pick must expose PASS because the text says 'You may'"
    );

    // Exactly 2 valid targets: ARISA_HAND (hand-origin) and SHOE_TRASH (trash-origin).
    // BAD_HAND and BAD_TRASH have wrong names and must be excluded.
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "only [Shoemon] (trash) and [Arisa Kinosaki] (hand) should be eligible; \
         non-matching cards must be filtered out"
    );

    // Choose the first valid action (whichever it is) and verify origin-preserving play.
    let trash_before = runner.game.players[0].trash.len();
    let hand_before = runner.game.players[0].hand.len();

    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select one valid union target");
    runner.auto_resolve().expect("resolve Main");

    // The selected card must be on the field; cost must not be paid.
    let field_ids: Vec<_> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    let played_shoe_or_arisa = field_ids.contains(&"SHOE_TRASH".to_string())
        || field_ids.contains(&"ARISA_HAND".to_string());
    assert!(
        played_shoe_or_arisa,
        "a Shoemon or Arisa Kinosaki card must be on the field after selection"
    );

    // Verify origin preservation: total cards in hand + trash shrank by exactly 1.
    let trash_after = runner.game.players[0].trash.len();
    let hand_after = runner.game.players[0].hand.len();
    assert_eq!(
        (trash_before + hand_before) - (trash_after + hand_after),
        1,
        "exactly 1 card should leave hand ∪ trash after a union pick"
    );

    // Memory must not change from free play (no cost paid).
    assert_eq!(
        runner.memory(),
        memory_before,
        "play_union_bound_free must not charge the target card's play cost"
    );
}

/// PUPPETS-G033 — declining the optional "you may play" still completes the
/// mandatory placement tail. The printed text is "You may play 1 ... Then,
/// place this card in the battle area"; the placement is not conditional on
/// the play happening. Passing the union-zone selection must still leave
/// BT22-098 parked in the battle area as a Delayed Option.
#[test]
fn bt22_098_option_pipeline_decline_still_places_as_delayed_option() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT22-098 YAML parses")
        .add_card(shoemon("SHOE"))
        .add_card(color_anchor("ANCHOR"))
        .add_card(filler("FILL"))
        .hand(0, &["BT22-098", "SHOE"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "ANCHOR", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "the [Main] union-zone selection must park the Option pipeline"
    );

    // Decline the optional play with PASS.
    let view = runner
        .pending_selection_view()
        .expect("Main union-zone selection must be pending");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    assert!(
        runner.pending_is_optional(),
        "PASS must be legal so the play can be declined"
    );
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the optional Shoemon/Arisa play");
    runner
        .auto_resolve()
        .expect("resolve battle-area placement after declining");

    assert!(
        hand_contains(&runner, 0, "SHOE"),
        "declining the optional play leaves the candidate untouched in hand"
    );
    assert!(
        !battle_area_contains(&runner, 0, "SHOE"),
        "declined target must not be played"
    );
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "BT22-098")
        .expect("BT22-098 should be placed even when the optional play is declined");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnSuspend),
            ..
        }
    ));
}

/// PUPPETS-G014/G033 — the inherited [Security] effect ("Activate this card's
/// [Main] effects") re-runs the same origin-preserving union-zone play and
/// then places BT22-098 in the battle area as a Delayed Option.
#[test]
fn bt22_098_security_activates_main_union_play_then_places_self() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT22-098 YAML parses")
        .add_card(shoemon("ATTACKER"))
        .add_card(arisa("ARISA_HAND"))
        .add_card(filler("FILL"))
        .hand(1, &["ARISA_HAND"])
        .security(1, &["BT22-098"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    // Attack player 1; the lone security card BT22-098 is checked.
    let _ = runner.attack_player(attacker, 1, false);

    // The inherited [Security] clause installs the same union-zone selection.
    let view = runner
        .pending_selection_view()
        .expect("security [Main] union-zone selection must be pending");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "security mirror should surface the union-zone pick"
    );
    assert!(
        runner.pending_is_optional(),
        "the security 'you may play' choice keeps PASS legal"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Arisa Kinosaki card in player 1's hand is an eligible target"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose Arisa Kinosaki from hand during the security effect");
    runner
        .auto_resolve()
        .expect("resolve the security play and battle-area placement");

    assert!(
        battle_area_contains(&runner, 1, "ARISA_HAND"),
        "the security effect plays the chosen Arisa Kinosaki for free"
    );
    assert_eq!(
        runner.security_count(1),
        0,
        "BT22-098 leaves the security stack"
    );
    assert_eq!(
        runner
            .game
            .player(1)
            .trash
            .iter()
            .filter(|source| source.card_id(&runner.game.card_data) == "BT22-098")
            .count(),
        0,
        "BT22-098 is placed in the battle area, not trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "BT22-098")
        .expect("BT22-098 should be placed as a Delayed Option by its security effect");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnSuspend),
            ..
        }
    ));
}
