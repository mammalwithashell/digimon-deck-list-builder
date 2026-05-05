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
//! # Patterns this test covers
//! - Option [Main] free play with a player-visible hand selection.
//! - Inherited [Security] activates the currently supported hand-origin [Main]
//!   slice.
//! - Event-gated Delay on a matching Arisa suspend event after the placing turn.
//! - Delay body selection sequence: Puppet Digimon base, then Puppet+LIBERATOR
//!   hand card, then effect_initiated_digivolve with cost reduced by 3.
//!
//! # Known gaps
//! - G-UNION-ZONE-PLAY-FROM-ORIGIN / PUPPETS-G014: the exact hand-or-trash
//!   [Main] play cannot be expressed as a single filtered, origin-preserving
//!   union-zone choice yet. This file implements and tests the hand-origin slice.
//! - PUPPETS-G009: security battle-area placement for the full Main mirror is
//!   still not represented here.

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledCostDelta, CompiledDeclarativeClause,
    CompiledPlayerRef, CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
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

    let select = main
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectHand {
                filter, optional, ..
            } => Some((filter, optional)),
            _ => None,
        })
        .expect("hand-origin slice must install a hand selection");

    assert!(
        *select.1,
        "printed 'may play 1' must surface PASS at the hand selection"
    );
    assert!(
        predicate_contains_name(select.0, "Shoemon")
            && predicate_contains_name(select.0, "Arisa Kinosaki"),
        "hand selection must filter to [Shoemon] or [Arisa Kinosaki]"
    );
    assert!(
        main.process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayFromHandFree { .. })),
        "selected hand target must be played without paying the cost"
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
        "Security activates the supported Main slice; the target play inside remains optional"
    );
    let select = security
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectHand {
                filter, optional, ..
            } => Some((filter, optional)),
            _ => None,
        })
        .expect("security mirror must install a hand selection");
    assert!(*select.1, "the mirrored 'may play 1' choice stays optional");
    assert!(
        predicate_contains_name(select.0, "Shoemon")
            && predicate_contains_name(select.0, "Arisa Kinosaki"),
        "security hand selection must filter to [Shoemon] or [Arisa Kinosaki]"
    );
    assert!(
        security
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayFromHandFree { .. })),
        "security mirror must play the selected hand target without paying cost"
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
        .expect("Main hand-origin selection must be pending");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "PASS must be legal because the target play is optional"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only exact Shoemon/Arisa should be legal hand targets; ShoeShoemon must not match [Shoemon]"
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

#[test]
fn bt22_098_option_pipeline_places_as_delayed_option() {
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
        OptionPlayResult::Trashed
    );

    assert!(
        hand_contains(&runner, 0, "SHOE"),
        "option placement path should not silently play the optional hand target"
    );
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "BT22-098")
        .expect("BT22-098 should be placed as a Delay option");
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

#[test]
#[ignore = "pending: G-UNION-ZONE-PLAY-FROM-ORIGIN / PUPPETS-G014 - exact hand-or-trash free play must preserve selected origin and enforce filters"]
fn bt22_098_main_can_choose_shoemon_or_arisa_from_hand_or_trash_in_one_masked_choice() {
    todo!("put Shoemon in hand, Arisa in trash, and invalid cards in both zones; assert one pending union choice exposes only the two eligible origin-preserving actions and plays the selected origin for free");
}

#[test]
#[ignore = "pending: PUPPETS-G009 - integrated option Main resolution with pending optional play plus post-resolution battle-area placement is not yet proven"]
fn bt22_098_full_main_decline_still_places_as_delayed_option() {
    todo!("play BT22-098 through the Option pipeline, decline the optional Shoemon/Arisa play, and assert the same resolving Option is placed as a delayed battle-area Option");
}

#[test]
#[ignore = "pending: PUPPETS-G014/PUPPETS-G009 - security must mirror the full hand-or-trash Main and battle-area placement flow"]
fn bt22_098_security_activates_main_effects() {
    todo!("security should run the same Main hand-or-trash play plus battle-area placement flow");
}
