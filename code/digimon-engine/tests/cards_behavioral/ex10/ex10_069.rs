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
use digimon_engine::action::space::{PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT};
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

/// Play EX10-069 via `play_option_from_hand` and settle the [Main] so that
/// the card ends up placed in the battle area as a Delayed Option.
///
/// The [Main] zone choice ("From hand" / "From trash") offers only a zone
/// holding a [Sunarizamon] / [Close] this effect can play (`legal_when`, DCGO
/// EX10_069.cs:41-71). These fixtures hold none, so no zone qualifies: the
/// choice binds nothing, both picks are skipped, and the pipeline places the
/// card without parking. If a zone did qualify, its first branch is taken and
/// its optional pick resolved by `auto_resolve`.
///
/// After this call EX10-069 is in the battle area as
/// `OptionState::Delayed { trigger: OnEvent(OnSuspend), .. }`.
fn place_ex10_069_via_pipeline(runner: &mut DebugRunner, hand_index: usize) {
    let result = runner.game.play_option_from_hand(0, hand_index);
    assert_ne!(result, OptionPlayResult::Invalid, "EX10-069 must be usable");
    if let Some(view) = runner.pending_selection_view() {
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("choose zone");
    }
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

    // Accept the optional <Delay> activation (16-17-2) first.
    {
        let view = runner
            .pending_selection_view()
            .expect("Close suspend should offer the Delay activation");
        assert_eq!(view.kind, SelectionKind::Replacement);
        assert!(
            runner.pending_is_optional(),
            "the <Delay> activation itself is optional (16-17-2)"
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

    // Accept the optional <Delay> activation (16-17-2) first.
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
/// was placed must NOT fire the Delay body (rule 16-17-3: "after the placing
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

    // ACCEPT the (optional, 16-17-2) Delay activation itself...
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

// ============================================================================
// Candidate legality (G-DSL-PLAY-OR-USE-CANDIDATE-LEGALITY)
//
// [Main] / [Security]: only a [Sunarizamon] / [Close] this effect can actually
// play (CR 4.3 1-3-2: an impossible action is not carried out; 7-1-1). DCGO
// EX10_069.cs:33-37 binds CanPlayAsNewPermanent to [Close] only (C# `||` /
// `&&` precedence) and so offers an unplayable [Sunarizamon]; the rules
// govern both names. The zone choice offers only a zone holding such a card
// and is skipped when one zone does (DCGO :41-71 canSelectHand /
// canSelectTrash, SetBool for a single legal zone).
// <Delay>: the hand pick offers only a card that can digivolve onto the
// chosen Digimon with its cost reduced by 3 payable (DCGO :243-253 payCost:
// true → CardEffectCommons.cs:858-880 CanPlayCardTargetFrame(PayCost: true)).
// ============================================================================

/// A [Sunarizamon] Digimon card.
fn sunarizamon_card(id: &str) -> CardData {
    let mut card = make_test_card(id, "Sunarizamon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 4;
    card.colors = vec![CardColor::Black];
    card.traits = vec!["Mineral".to_string()];
    card
}

/// What the [Main] offered: whether the zone-choice prompt was shown, the
/// hand-pick card ids, and the trash-pick card ids (empty when that pick did
/// not install). The zone choice picks `zone` ("From hand" / "From trash")
/// when it is shown; every card pick is declined.
#[derive(Default, Debug)]
struct MainOffers {
    zone_prompt_labels: Option<Vec<String>>,
    hand: Vec<String>,
    trash: Vec<String>,
}

fn drive_main(runner: &mut DebugRunner, option_hand_index: usize, zone: &str) -> MainOffers {
    let result = runner.game.play_option_from_hand(0, option_hand_index);
    assert_ne!(result, OptionPlayResult::Invalid, "EX10-069 must be usable");
    let mut offers = MainOffers::default();
    for _ in 0..10 {
        let Some(view) = runner.pending_selection_view() else { break };
        let concrete: Vec<u16> = view
            .valid_action_ids
            .iter()
            .copied()
            .filter(|&id| id != PASS)
            .collect();
        let action = if let Some(choices) = view
            .effect_choices
            .as_ref()
            .filter(|c| c.iter().any(|e| e.label.starts_with("From ")))
        {
            offers.zone_prompt_labels = Some(choices.iter().map(|e| e.label.clone()).collect());
            choices
                .iter()
                .find(|e| e.label == zone)
                .map(|e| e.action_id)
                .unwrap_or(choices[0].action_id)
        } else {
            match view.kind {
                SelectionKind::Hand => {
                    offers.hand = concrete
                        .iter()
                        .map(|&id| {
                            runner.game.players[0].hand[(id - PLAY_HAND_START) as usize]
                                .card_id(&runner.game.card_data)
                                .to_string()
                        })
                        .collect();
                    PASS
                }
                SelectionKind::Trash => {
                    offers.trash = runner.game.players[0]
                        .trash
                        .iter()
                        .map(|c| c.card_id(&runner.game.card_data).to_string())
                        .filter(|_| !concrete.is_empty())
                        .collect();
                    PASS
                }
                _ => concrete.first().copied().unwrap_or(view.valid_action_ids[0]),
            }
        };
        runner
            .execute_action(view.selecting_player, action)
            .expect("drive pending selection");
    }
    let _ = runner.auto_resolve();
    offers
}

fn main_runner(hand: &[&str], trash: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("EX10-069 YAML parses")
        .add_card(close_card("CLOSE"))
        .add_card(sunarizamon_card("SUNARI"))
        .add_card(black_anchor("ANCHOR"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10);
    let mut all_hand = vec!["EX10-069"];
    all_hand.extend_from_slice(hand);
    b = b.hand(0, &all_hand);
    let mut runner = b.start();
    for id in trash {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap();
        let idx = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(digimon_engine::card_source::CardSource::new(data_idx, 0, idx));
    }
    runner.place_on_field(0, "ANCHOR", Some(0));
    runner.game.enter_main_phase();
    runner
}

fn lock(runner: &mut DebugRunner, kind: digimon_engine::enums::ModifierType) {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    runner
        .game
        .modifiers
        .add_player_modifier(0, PlayerModifierEntry::simple(kind, 0, Expiry::Permanent, None, 1));
}

/// "You can't play Tamers by effects": [Close] is not offered from the hand;
/// [Sunarizamon] still is. The trash holds nothing, so the zone choice is not
/// a choice (only the hand qualifies) and no prompt is shown.
#[test]
fn ex10_069_main_hand_pick_excludes_close_under_tamer_play_lock() {
    let mut runner = main_runner(&["CLOSE", "SUNARI"], &[]);
    lock(&mut runner, digimon_engine::enums::ModifierType::CannotPlayTamerByEffect);
    let offers = drive_main(&mut runner, 0, "From hand");
    assert_eq!(offers.hand, vec!["SUNARI".to_string()], "{offers:?}");
    assert!(offers.zone_prompt_labels.is_none(), "single legal zone: no prompt: {offers:?}");
}

/// "You can't play Digimon by effects": [Sunarizamon] is not offered from the
/// hand; [Close] still is. (DCGO offers [Sunarizamon] here: its playability
/// check binds only to [Close]; CR 4.3 1-3-2 governs.)
#[test]
fn ex10_069_main_hand_pick_excludes_sunarizamon_under_digimon_play_lock() {
    let mut runner = main_runner(&["CLOSE", "SUNARI"], &[]);
    lock(&mut runner, digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect);
    let offers = drive_main(&mut runner, 0, "From hand");
    assert_eq!(offers.hand, vec!["CLOSE".to_string()], "{offers:?}");
}

/// Only the trash holds a playable card ([Close] in hand is locked): the zone
/// choice is skipped and the trash pick offers [Sunarizamon].
#[test]
fn ex10_069_main_zone_choice_skipped_when_only_trash_qualifies() {
    let mut runner = main_runner(&["CLOSE"], &["SUNARI"]);
    lock(&mut runner, digimon_engine::enums::ModifierType::CannotPlayTamerByEffect);
    let offers = drive_main(&mut runner, 0, "From hand");
    assert!(offers.zone_prompt_labels.is_none(), "no zone prompt: {offers:?}");
    assert!(offers.hand.is_empty(), "{offers:?}");
    assert_eq!(offers.trash, vec!["SUNARI".to_string()], "{offers:?}");
}

/// Both zones hold a playable card: the zone choice offers both.
#[test]
fn ex10_069_main_zone_choice_offers_both_when_both_qualify() {
    let mut runner = main_runner(&["CLOSE"], &["SUNARI"]);
    let offers = drive_main(&mut runner, 0, "From trash");
    assert_eq!(
        offers.zone_prompt_labels,
        Some(vec!["From hand".to_string(), "From trash".to_string()]),
        "{offers:?}"
    );
    assert_eq!(offers.trash, vec!["SUNARI".to_string()], "{offers:?}");
}

/// Nothing playable anywhere (both card types locked): no zone prompt, no
/// pick; the cards stay where they are and EX10-069 is still placed.
#[test]
fn ex10_069_main_offers_nothing_when_nothing_is_playable() {
    let mut runner = main_runner(&["CLOSE"], &["SUNARI"]);
    lock(&mut runner, digimon_engine::enums::ModifierType::CannotPlayTamerByEffect);
    lock(&mut runner, digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect);
    let offers = drive_main(&mut runner, 0, "From hand");
    assert!(offers.zone_prompt_labels.is_none(), "{offers:?}");
    assert!(offers.hand.is_empty() && offers.trash.is_empty(), "{offers:?}");
    assert!(hand_contains(&runner, 0, "CLOSE"));
    assert!(battle_area_contains(&runner, 0, "EX10-069"), "Delay Option placed");
}

/// A full battle area: neither card can be played from either zone.
#[test]
fn ex10_069_main_offers_nothing_when_battle_area_full() {
    let mut runner = main_runner(&["CLOSE"], &["SUNARI"]);
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    let offers = drive_main(&mut runner, 0, "From hand");
    assert!(offers.zone_prompt_labels.is_none(), "{offers:?}");
    assert!(offers.hand.is_empty() && offers.trash.is_empty(), "{offers:?}");
    assert!(hand_contains(&runner, 0, "CLOSE"));
}

/// A [Mineral]+[LIBERATOR] Digimon card with an explicit Black Lv.`from`
/// digivolve cost.
fn mineral_liberator_evo_with(id: &str, level: u8, from: u8, cost: u16) -> CardData {
    let mut card = mineral_liberator_evo(id);
    card.level = Some(level);
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Black as u8,
        level: from,
        memory_cost: cost,
    }];
    card
}

/// <Delay> hand pick: only a card that can digivolve onto the chosen Lv.3
/// Mineral Digimon AND whose cost reduced by 3 is payable. With 0 memory,
/// EVO-CHEAP (3 - 3 = 0) qualifies; EVO-DEAR (14 - 3 = 11 → memory -11, past
/// the -10 floor) can't be paid; EVO-LV6 (from Lv.5) can't digivolve onto a
/// Lv.3 at all.
#[test]
fn ex10_069_delay_hand_pick_offers_only_payable_legal_digivolutions() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("EX10-069 YAML parses")
        .add_card(close_card("CLOSE"))
        .add_card(black_anchor("ANCHOR"))
        .add_card(mineral_base("BASE"))
        .add_card(mineral_liberator_evo_with("EVO-CHEAP", 4, 3, 3))
        .add_card(mineral_liberator_evo_with("EVO-DEAR", 4, 3, 14))
        .add_card(mineral_liberator_evo_with("EVO-LV6", 6, 5, 3))
        .add_card(filler("FILL"))
        .hand(0, &["EX10-069", "EVO-CHEAP", "EVO-DEAR", "EVO-LV6"])
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
    runner.game.set_memory(0);

    runner.game.suspend(close);
    let view = runner.pending_selection_view().expect("Delay activation offered");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
        .expect("accept the Delay activation");
    let view = runner.pending_selection_view().expect("base selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    let base = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("the Mineral base is selectable");
    runner.execute_action(view.selecting_player, base).expect("choose base");

    let view = runner.pending_selection_view().expect("hand evolution selection");
    assert_eq!(view.kind, SelectionKind::Hand);
    let offered: Vec<String> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| {
            runner.game.players[0].hand[(id - PLAY_HAND_START) as usize]
                .card_id(&runner.game.card_data)
                .to_string()
        })
        .collect();
    assert_eq!(offered, vec!["EVO-CHEAP".to_string()]);
}
