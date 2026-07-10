//! EX10-069 Unique Emblem: Gravel Hearts — Option, Cost 3, Black.
//!
//! # Card text (cards.json)
//!
//! [Main] You may play 1 [Sunarizamon] or [Close] from your hand or trash
//! without paying the cost. Then, place this card in the battle area.
//!
//! [Your Turn] When any of your [Close]s suspend, <Delay> (By trashing
//! this card after the placing turn, activate the effect below.)
//! ・1 of your [Mineral] or [Rock] trait Digimon may digivolve into a Digimon
//! card with the [Mineral] trait and [LIBERATOR] trait in the hand with the
//! digivolution cost reduced by 3.
//!
//! Security Effect [Security] Activate this card's [Main] effects.
//!
//! # Patterns this test covers
//! - [Main] hand/trash play, optional, filtered to [Sunarizamon] or [Close].
//! - DSL-level structure of the event-gated Delay clause.
//! - Option pipeline places EX10-069 as a Delayed Option (OnSuspend).
//! - Delay fires after the placing turn when a [Close] suspends.
//! - Delay body: [Mineral]/[Rock] base selection + [Mineral]+[LIBERATOR]
//!   hand evolution selection + effect_initiated_digivolve cost -3.
//! - Negative: Delay does not fire on the placing turn.
//! - Negative: Delay does not fire when a non-[Close] suspends.
//! - Delay trashes self as cost even when the digivolve is declined.
//!
//! # Structural note
//! EX10-069 is structurally identical to BT22-098 Unique Emblem: Fable Waltz
//! (Puppets archetype), with [Close] in place of [Arisa Kinosaki] and
//! [Mineral]/[Rock] in place of [Puppet].

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledCostDelta, CompiledDeclarativeClause,
    CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

fn yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/ex10/EX10-069.yaml"))
        .expect("EX10-069 YAML must exist at cards/ex10/EX10-069.yaml")
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("EX10-069 YAML must parse and compile")
        .memory(10)
        .start()
}

// ---------- card fixtures ---------------------------------------------------

/// A [Close] Tamer card (Black, LIBERATOR).
fn close_card(id: &str) -> CardData {
    let mut card = make_test_card(id, "Close");
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 4;
    card.colors = vec![CardColor::Black];
    card.traits = vec!["LIBERATOR".to_string()];
    card
}

/// Black anchor Digimon placed on the field to satisfy the Option color
/// requirement when calling `play_option_from_hand`.
fn black_anchor(id: &str) -> CardData {
    let mut card = make_test_card(id, "Black Anchor");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Black];
    card.traits = vec!["Machine".to_string()];
    card
}

/// A [Mineral] trait Digimon suitable as an evolution base.
fn mineral_base(id: &str) -> CardData {
    let mut card = make_test_card(id, "Mineral Base");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Black];
    card.traits = vec!["Mineral".to_string()];
    card
}

/// A [Rock] trait Digimon suitable as an evolution base.
fn rock_base(id: &str) -> CardData {
    let mut card = make_test_card(id, "Rock Base");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Black];
    card.traits = vec!["Rock".to_string()];
    card
}

/// A [Mineral]+[LIBERATOR] Digimon card for the evolution hand target.
fn mineral_liberator_evo(id: &str) -> CardData {
    let mut card = make_test_card(id, "Mineral Liberator Evo");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Black];
    card.traits = vec!["Mineral".to_string(), "LIBERATOR".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Black as u8,
        level: 3,
        memory_cost: 3,
    }];
    card
}

/// A filler Digimon that is not a [Mineral], [Rock], or [Close] card.
fn non_target(id: &str) -> CardData {
    let mut card = make_test_card(id, "Non Target");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Black];
    card.traits = vec!["Beast".to_string()];
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

// ---------- predicate helpers -----------------------------------------------

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

// ---------- legacy helpers (kept from prior pass) ---------------------------

fn has_select_hand(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|step| match step {
        CompiledStep::SelectHand { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => has_select_hand(then) || has_select_hand(else_branch),
        _ => false,
    })
}

fn has_select_trash(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|step| match step {
        CompiledStep::SelectTrash { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => has_select_trash(then) || has_select_trash(else_branch),
        _ => false,
    })
}

// ---------- placement helper ------------------------------------------------

/// Play EX10-069 via `play_option_from_hand` and drive through the
/// `select_effect_choice` → `auto_resolve` path so that the card ends up
/// placed in the battle area as a Delayed Option.
///
/// EX10-069's [Main] clause starts with `select_effect_choice`, so
/// `play_option_from_hand` always returns `Pending`. This helper:
///   1. asserts `Pending`
///   2. picks "From hand" (choice 0) at the zone-choice prompt
///   3. calls `auto_resolve` to settle any remaining steps and place the card
///
/// After the zone-choice is resolved, the hand sub-selection auto-skips
/// (no Sunarizamon/Close in hand), so `advance_pending_option` fires
/// immediately and `dispose_option` places EX10-069 as a Delayed Option.
///
/// After this call EX10-069 is in the battle area as
/// `OptionState::Delayed { trigger: OnEvent(OnSuspend), .. }`.
fn place_ex10_069_via_pipeline(runner: &mut DebugRunner, hand_index: usize) {
    assert_eq!(
        runner.game.play_option_from_hand(0, hand_index),
        OptionPlayResult::Pending,
        "EX10-069 [Main] select_effect_choice must park the pipeline"
    );
    // Zone-choice selection (effect_choice): pick "From hand" (choice 0).
    let view = runner
        .pending_selection_view()
        .expect("zone-choice selection must be pending");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose zone");
    // After the zone-choice resolves, the hand sub-selection is auto-skipped
    // (empty filter match), advance_pending_option fires synchronously, and
    // dispose_option places EX10-069 in the battle area. No second selection
    // should be pending, but auto_resolve settles any residual drains.
    runner
        .auto_resolve()
        .expect("resolve [Main] and battle-area placement");
}

// ============================================================================
// Structural / compilation tests
// ============================================================================

#[test]
fn ex10_069_yaml_parses_and_compiles() {
    let _runner = runner();
}

#[test]
fn ex10_069_has_main_play_from_hand_or_trash_paths() {
    let runner = DebugRunner::builder()
        .dsl_card("EX10-069")
        .expect("EX10-069 YAML parses and compiles")
        .build();
    let card = runner.compiled_card("EX10-069").expect("EX10-069 compiled");
    let main = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::MainFromHand) =>
        {
            Some(triggered)
        }
        _ => None,
    });
    let main = main.expect("EX10-069 must have MainFromHand clause");
    assert!(has_select_hand(&main.process));
    assert!(has_select_trash(&main.process));
}

#[test]
fn ex10_069_has_event_gated_delay_for_own_close_suspend() {
    let runner = runner();
    let compiled = runner.compiled_card("EX10-069").expect("compiled card");
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
        .expect("EX10-069 must have a Delay clause for the [Your Turn] Close-suspend body");

    assert_eq!(*delay.0, CompiledTiming::OnSuspend);
    let active_when = delay
        .1
        .as_ref()
        .expect("Delay must be gated to the Close suspend event");
    assert_eq!(active_when.your_turn, Some(true));
    assert_eq!(active_when.event_target_owner, Some(CompiledPlayerRef::You));
    assert_eq!(
        active_when.event_card_name_contains.as_deref(),
        Some("Close")
    );

    // The Delay body must select a [Mineral] or [Rock] base Digimon (optional).
    let own_perm = delay
        .2
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectOwnPermanent {
                filter, optional, ..
            } => Some((filter, optional)),
            _ => None,
        })
        .expect("Delay body must select a Mineral/Rock base Digimon");
    assert!(*own_perm.1, "printed Delay digivolve says 'may'");
    assert!(predicate_contains_kind(
        own_perm.0,
        CompiledCardKind::Digimon
    ));
    assert!(
        predicate_contains_trait(own_perm.0, "Mineral")
            || predicate_contains_trait(own_perm.0, "Rock"),
        "base selection must filter to [Mineral] or [Rock] trait"
    );

    // The Delay body must select the evolution card from hand.
    let evo = delay
        .2
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectHand { filter, .. } => Some(filter),
            _ => None,
        })
        .expect("Delay body must select the evolution card from hand");
    assert!(predicate_contains_kind(evo, CompiledCardKind::Digimon));
    assert!(
        predicate_contains_trait(evo, "Mineral"),
        "evolution target must have [Mineral] trait"
    );
    assert!(
        predicate_contains_trait(evo, "LIBERATOR"),
        "evolution target must have [LIBERATOR] trait"
    );

    // The digivolve step must reduce cost by 3.
    assert!(
        delay.2.iter().any(|step| {
            matches!(
                step,
                CompiledStep::EffectInitiatedDigivolve {
                    cost: CompiledCostDelta::Reduce(3),
                    ignore_requirements: false,
                    ..
                }
            )
        }),
        "Delay body must include effect_initiated_digivolve with cost reduced by 3"
    );
}

// ============================================================================
// Behavioral: option pipeline places EX10-069 as a Delayed option
// ============================================================================

/// Activating the [Main] clause via the Option pipeline places EX10-069 in the
/// battle area as an OnSuspend-gated Delayed Option.
#[test]
fn ex10_069_option_pipeline_places_as_delayed_option_after_main() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("EX10-069 YAML parses")
        .add_card(black_anchor("ANCHOR"))
        .add_card(filler("FILL"))
        .hand(0, &["EX10-069"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "ANCHOR", Some(0));
    runner.game.enter_main_phase();

    place_ex10_069_via_pipeline(&mut runner, 0);

    // EX10-069 must be placed in the battle area as a Delayed Option.
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "EX10-069")
        .expect("EX10-069 should be placed in the battle area as a Delay option");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnSuspend),
            ..
        }
    ));
}

// ============================================================================
// Behavioral: Delay fires after the placing turn when Close suspends
// ============================================================================

/// Full positive path: After the placing turn, a [Close] suspend fires the
/// Delay, prompts for a [Mineral]/[Rock] base, then a [Mineral]+[LIBERATOR]
/// hand card, then performs the digivolve and trashes EX10-069.
#[test]
fn ex10_069_delay_after_close_suspend_exposes_base_then_hand_evo_choices() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("EX10-069 YAML parses")
        .add_card(close_card("CLOSE"))
        .add_card(black_anchor("ANCHOR"))
        .add_card(mineral_base("BASE"))
        .add_card(mineral_liberator_evo("EVO"))
        .add_card(non_target("OTHER"))
        .add_card(filler("FILL"))
        .hand(0, &["EX10-069", "EVO", "OTHER"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.place_on_field(0, "ANCHOR", Some(0));
    runner.game.enter_main_phase();

    place_ex10_069_via_pipeline(&mut runner, 0);

    let close = runner.place_on_field(0, "CLOSE", Some(0));

    // Advance past the placing turn.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    runner.game.suspend(close);

    // Accept the optional <Delay> activation (16-16-2) first.
    {
        let view = runner
            .pending_selection_view()
            .expect("Close suspend should offer the Delay activation");
        assert_eq!(view.kind, SelectionKind::Replacement);
        assert!(
            runner.pending_is_optional(),
            "the <Delay> activation itself is optional (16-16-2)"
        );
        runner
            .execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
            .expect("accept the Delay activation");
    }

    // First body selection: [Mineral]/[Rock] base Digimon (optional).
    {
        let view = runner
            .pending_selection_view()
            .expect("accepting the Delay should fire the base selection");
        assert_eq!(view.kind, SelectionKind::OwnField);
        assert!(
            runner.pending_is_optional(),
            "Delay digivolve is optional at the base selection"
        );
        assert!(
            view.valid_action_ids.len() >= 1,
            "the Mineral Digimon base must be selectable"
        );
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("choose Mineral base");
    }

    // Second selection: [Mineral]+[LIBERATOR] hand card.
    {
        let view = runner
            .pending_selection_view()
            .expect("choosing the base should expose the hand evolution selection");
        assert_eq!(view.kind, SelectionKind::Hand);
        assert_eq!(
            view.valid_action_ids.len(),
            1,
            "only the Mineral+LIBERATOR hand card should be legal"
        );
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("choose Mineral+LIBERATOR evolution");
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
            .any(|source| source.card_id(&runner.game.card_data) == "EX10-069"),
        "Delay activation should trash EX10-069 as its cost"
    );
}

/// [Rock] trait Digimon is also eligible as the Delay digivolve base.
#[test]
fn ex10_069_delay_accepts_rock_trait_digimon_as_base() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("EX10-069 YAML parses")
        .add_card(close_card("CLOSE"))
        .add_card(black_anchor("ANCHOR"))
        .add_card(rock_base("ROCK_BASE"))
        .add_card(mineral_liberator_evo("EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["EX10-069", "EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "ROCK_BASE", Some(0));
    runner.place_on_field(0, "ANCHOR", Some(0));
    runner.game.enter_main_phase();

    place_ex10_069_via_pipeline(&mut runner, 0);

    let close = runner.place_on_field(0, "CLOSE", Some(0));

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    runner.game.suspend(close);

    // Accept the optional <Delay> activation (16-16-2) first.
    let view = runner
        .pending_selection_view()
        .expect("Close suspend offers the Delay activation");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
        .expect("accept the Delay activation");

    let view = runner
        .pending_selection_view()
        .expect("accepting the Delay fires the base selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        view.valid_action_ids.len() >= 1,
        "the [Rock] trait Digimon must be legal as the digivolve base"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose Rock base");
    let view = runner
        .pending_selection_view()
        .expect("hand evolution selection");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose evo");
    runner.auto_resolve().expect("complete Delay digivolve");
    assert!(battle_area_contains(&runner, 0, "EVO"));
}

// ============================================================================
// Behavioral: Delay does NOT fire on the placing turn
// ============================================================================

/// Negative gate (placing turn): a [Close] suspend on the same turn EX10-069
/// was placed must NOT fire the Delay body (rule 16-16-3: "after the placing
/// turn").
#[test]
fn ex10_069_delay_does_not_fire_on_the_placing_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("EX10-069 YAML parses")
        .add_card(close_card("CLOSE"))
        .add_card(black_anchor("ANCHOR"))
        .add_card(mineral_base("BASE"))
        .add_card(mineral_liberator_evo("EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["EX10-069", "EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.place_on_field(0, "ANCHOR", Some(0));
    runner.game.enter_main_phase();

    place_ex10_069_via_pipeline(&mut runner, 0);

    let close = runner.place_on_field(0, "CLOSE", Some(0));

    // Suspend Close on the SAME turn EX10-069 was placed.
    runner.game.suspend(close);

    assert!(
        runner.pending_selection().is_none(),
        "the Delay must not fire on its placing turn"
    );
    assert!(
        battle_area_contains(&runner, 0, "EX10-069"),
        "EX10-069 stays parked when the Delay does not fire"
    );
    assert!(
        hand_contains(&runner, 0, "EVO"),
        "the evolution card stays in hand because the Delay body never ran"
    );
}

// ============================================================================
// Behavioral: Delay does NOT fire when a non-Close suspends
// ============================================================================

/// Negative gate (event predicate): suspending a non-[Close] permanent after
/// the placing turn must NOT fire the Delay body.
#[test]
fn ex10_069_delay_does_not_fire_when_non_close_suspends() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("EX10-069 YAML parses")
        .add_card(black_anchor("ANCHOR"))
        .add_card(mineral_base("BASE"))
        .add_card(mineral_liberator_evo("EVO"))
        .add_card(non_target("OTHER"))
        .add_card(filler("FILL"))
        .hand(0, &["EX10-069", "EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.place_on_field(0, "ANCHOR", Some(0));
    runner.game.enter_main_phase();

    place_ex10_069_via_pipeline(&mut runner, 0);

    // OTHER is a non-Close Digimon (Beast trait).
    let other = runner.place_on_field(0, "OTHER", Some(0));

    // Advance past the placing turn.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    runner.game.suspend(other);

    assert!(
        runner.pending_selection().is_none(),
        "the Delay must not fire when a non-Close permanent suspends"
    );
    assert!(
        battle_area_contains(&runner, 0, "EX10-069"),
        "EX10-069 stays parked when the suspend event does not match"
    );
}

// ============================================================================
// Behavioral: Delay trashes self as cost even when digivolve declined
// ============================================================================

/// Cost-firing clause: the <Delay> activation cost is "by trashing this card".
/// When the Close suspend fires the Delay after the placing turn, EX10-069 is
/// trashed even if the optional digivolve body is declined.
#[test]
fn ex10_069_delay_trashes_self_as_cost_even_when_digivolve_is_declined() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("EX10-069 YAML parses")
        .add_card(close_card("CLOSE"))
        .add_card(black_anchor("ANCHOR"))
        .add_card(mineral_base("BASE"))
        .add_card(mineral_liberator_evo("EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["EX10-069", "EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BASE", Some(0));
    runner.place_on_field(0, "ANCHOR", Some(0));
    runner.game.enter_main_phase();

    place_ex10_069_via_pipeline(&mut runner, 0);

    let close = runner.place_on_field(0, "CLOSE", Some(0));

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    runner.game.suspend(close);

    // ACCEPT the (optional, 16-16-2) Delay activation itself...
    let view = runner
        .pending_selection_view()
        .expect("Close suspend after the placing turn offers the Delay activation");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
        .expect("accept the Delay activation");

    // ...then decline the optional base digivolve body with PASS.
    let view = runner
        .pending_selection_view()
        .expect("accepting the Delay activation exposes the digivolve body");
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
    // ...but the activation cost still trashed EX10-069.
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|source| source.card_id(&runner.game.card_data) == "EX10-069"),
        "the <Delay> cost trashes EX10-069 even when the digivolve is declined"
    );
    assert!(
        !battle_area_contains(&runner, 0, "EX10-069"),
        "EX10-069 must leave the battle area once the Delay activation cost is paid"
    );
}
