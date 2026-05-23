//! BT24-089 Unique Emblem: Blazing Conductor — Option, Cost 3, Red, Traits: LIBERATOR.
//!
//! # Card text (cards.json / printed)
//!
//! **[Main]** You may play 1 \[Elizamon\] or \[Owen Dreadnought\] from your
//! hand or trash without paying the cost. Then, place this card in the battle area.
//!
//! **\[Your Turn\] When any of your \[Owen Dreadnought\]s suspend**, ＜Delay＞
//! (By trashing this card after the placing turn, activate the effect below.)
//! ・1 of your \[Reptile\] or \[Dragonkin\] Digimon may digivolve into a
//! Digimon card with the \[Reptile\] or \[Dragonkin\] trait and the \[LIBERATOR\]
//! trait in the hand with the digivolution cost reduced by 3.
//!
//! **Security:** Activate this card's \[Main\] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_089.cs
//!
//! # Patterns this test covers
//! - Option [Main] free play with a player-visible, origin-preserving union-zone
//!   selection (hand ∪ trash) filtered to [Elizamon] / [Owen Dreadnought].
//! - "Then, place this card in the battle area" via `place_self_as_delay_option`.
//! - Event-gated Delay on a matching [Owen Dreadnought] suspend after the placing
//!   turn (G-DELAY-EVENT-GATED suspend-timing slice; same shape as BT22-098).
//! - Delay body selection sequence: Reptile/Dragonkin Digimon base, then a
//!   Reptile-or-Dragonkin + LIBERATOR hand card, then effect_initiated_digivolve
//!   with the digivolution cost reduced by 3, paid by trashing BT24-089.
//! - Inherited [Security] mirrors the [Main] union-zone play + placement.
//!
//! # Gap status
//! - G-PLACE-SELF-AS-OPTION-PERMANENT — RESOLVED 2026-05-02. `place_self_as_delay_option`
//!   places the resolving Option into the battle area as a Delay permanent.
//! - G-DELAY-SUSPEND-CONDITION — covered by the resolved G-DELAY-EVENT-GATED
//!   suspend-timing slice (`kind: delay` + `trigger: on_suspend` +
//!   `active_when: { event_card_name_contains: ... }`). No longer an open gap;
//!   the Delay clause is modelled directly, all behavioral tests are active.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledCostDelta, CompiledDeclarativeClause, CompiledPlayerRef,
    CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/bt24/BT24-089.yaml");

// ─── helpers ──────────────────────────────────────────────────────────────────

fn make_elizamon() -> CardData {
    let mut c = make_test_card("ELIZAMON", "Elizamon");
    c.card_kind = CardKind::Digimon;
    c
}

fn make_owen() -> CardData {
    let mut c = make_test_card("OWEN", "Owen Dreadnought");
    c.card_kind = CardKind::Tamer;
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Reptile-trait Digimon base eligible for the Delay digivolve.
fn make_reptile_base(id: &str) -> CardData {
    let mut c = make_test_card(id, "Reptile Base");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Reptile".to_string()];
    c
}

/// A Reptile + LIBERATOR Digimon card used as the Delay digivolution target.
fn make_reptile_liberator_evo(id: &str) -> CardData {
    let mut c = make_test_card(id, "Reptile Liberator Evo");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Reptile".to_string(), "LIBERATOR".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 3,
    }];
    c
}

/// A Digimon with neither Reptile nor Dragonkin nor LIBERATOR — must be filtered
/// out of both the Delay base and Delay evolution selections.
fn make_non_target(id: &str) -> CardData {
    let mut c = make_test_card(id, "Non Target");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Beast".to_string()];
    c
}

fn process_contains_place_self_as_delay_option(process: &[CompiledStep]) -> bool {
    process.iter().any(|step| match step {
        CompiledStep::PlaceSelfAsDelayOption => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_place_self_as_delay_option(then)
                || process_contains_place_self_as_delay_option(else_branch)
        }
        CompiledStep::ForEach { body, .. }
        | CompiledStep::PerSelected { body, .. }
        | CompiledStep::ScheduleDelayed { body, .. }
        | CompiledStep::Optional(body)
        | CompiledStep::AsSelectingPlayer { body, .. } => {
            process_contains_place_self_as_delay_option(body)
        }
        CompiledStep::SelectOwnSources { then, .. }
        | CompiledStep::SelectOwnBreedingPermanent { then, .. }
        | CompiledStep::SelectOpponentDpBudget { then, .. } => {
            process_contains_place_self_as_delay_option(then)
        }
        _ => false,
    })
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

fn trash_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner
        .game
        .player(player)
        .trash
        .iter()
        .any(|source| source.card_id(&runner.game.card_data) == card_id)
}

// ─── §1 YAML parse + structural ───────────────────────────────────────────────

/// The YAML must parse and compile without errors.
#[test]
fn bt24_089_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-089 YAML must parse and compile without errors");
}

/// BT24-089 is an Option card with cost 3.
#[test]
fn bt24_089_is_option_cost_3() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Option
    );
    assert_eq!(compiled.cost, Some(3));
}

/// BT24-089 must have exactly 3 clauses:
///   Clause 0: main_from_hand (triggered)
///   Clause 1: kind: delay — event-gated Delay on Owen Dreadnought suspend
///   Clause 2: on_security (triggered, inherited)
#[test]
fn bt24_089_has_three_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    assert_eq!(
        compiled.effects.len(),
        3,
        "expected 3 clauses (main_from_hand, delay, on_security); got {}",
        compiled.effects.len()
    );
}

/// Clause 0 fires at main_from_hand and has FaceUp scope.
#[test]
fn bt24_089_clause0_is_main_from_hand_face_up() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let clause0 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("must have a main_from_hand clause");

    assert_eq!(
        clause0.scope,
        CompiledScope::FaceUp,
        "Clause 0 must have face-up scope"
    );
}

/// The Main clause uses an origin-preserving union-zone (hand ∪ trash) selection,
/// filtered to [Elizamon] / [Owen Dreadnought], and the inner play is optional.
#[test]
fn bt24_089_main_clause_uses_optional_union_zone_select() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let main_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("must have a main_from_hand clause");

    let (filter, optional) = main_clause
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectUnionZone {
                filter, optional, ..
            } => Some((filter, *optional)),
            _ => None,
        })
        .expect("Main process must install a union-zone selection");

    assert!(
        optional,
        "printed 'You may play 1' must surface a declinable union-zone selection"
    );
    assert!(
        predicate_contains_name(filter, "Elizamon")
            && predicate_contains_name(filter, "Owen Dreadnought"),
        "union-zone selection must filter to [Elizamon] or [Owen Dreadnought]"
    );
    assert!(
        main_clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayUnionBoundFree { .. })),
        "selected union-zone target must be played without paying the cost"
    );
}

/// Printed Main says "Then, place this card in the battle area."
/// The Main process must include `place_self_as_delay_option`.
#[test]
fn bt24_089_main_clause_places_self_as_delay_option() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let main_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("must have a main_from_hand clause");

    assert!(
        process_contains_place_self_as_delay_option(&main_clause.process),
        "Main process must include place_self_as_delay_option after resolving \
         the optional Elizamon/Owen play"
    );
}

/// Clause 1 is the event-gated Delay clause, triggering on a suspend event gated
/// by an [Owen Dreadnought] name match on the owner's turn.
#[test]
fn bt24_089_clause1_is_event_gated_delay_on_owen_suspend() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let (trigger, active_when, _process) = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                active_when,
                process,
                ..
            }) => Some((trigger, active_when, process)),
            _ => None,
        })
        .expect("Clause 1 must be a kind: delay clause");

    assert_eq!(
        *trigger,
        CompiledTiming::OnSuspend,
        "Delay must trigger on a suspend event"
    );
    let active_when = active_when
        .as_ref()
        .expect("Delay must be gated to the Owen Dreadnought suspend event");
    assert_eq!(active_when.your_turn, Some(true));
    assert_eq!(active_when.event_target_owner, Some(CompiledPlayerRef::You));
    assert_eq!(
        active_when.event_card_name_contains.as_deref(),
        Some("Owen Dreadnought")
    );
}

/// The Delay body selects a Reptile/Dragonkin Digimon base, a Reptile-or-Dragonkin
/// + LIBERATOR hand card, and digivolves with the cost reduced by 3.
#[test]
fn bt24_089_delay_body_selects_reptile_dragonkin_then_digivolves_cost_minus_3() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let process = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay { process, .. }) => {
                Some(process)
            }
            _ => None,
        })
        .expect("Delay clause must be present");

    let (base_filter, base_optional) = process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectOwnPermanent {
                filter, optional, ..
            } => Some((filter, *optional)),
            _ => None,
        })
        .expect("Delay body must select a Reptile/Dragonkin Digimon base");
    assert!(
        base_optional,
        "printed Delay digivolve says '1 of your ... may digivolve'"
    );
    assert!(
        predicate_contains_trait(base_filter, "Reptile")
            && predicate_contains_trait(base_filter, "Dragonkin"),
        "base selection must allow [Reptile] or [Dragonkin] Digimon"
    );

    let evo_filter = process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectHand { filter, .. } => Some(filter),
            _ => None,
        })
        .expect("Delay body must select the evolution card from hand");
    assert!(
        predicate_contains_trait(evo_filter, "Reptile")
            && predicate_contains_trait(evo_filter, "Dragonkin")
            && predicate_contains_trait(evo_filter, "LIBERATOR"),
        "evolution card must be a [Reptile] or [Dragonkin] AND [LIBERATOR] Digimon"
    );

    assert!(
        process.iter().any(|step| matches!(
            step,
            CompiledStep::EffectInitiatedDigivolve {
                cost: CompiledCostDelta::Reduce(3),
                ignore_requirements: false,
                ..
            }
        )),
        "Delay body must digivolve with the digivolution cost reduced by 3"
    );
}

/// Clause 2 fires at on_security, is an inherited security effect, and is not
/// optional (the security trigger always activates the Main effects).
#[test]
fn bt24_089_clause2_is_on_security_inherited() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let security_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have an on_security clause");

    assert!(
        !security_clause.optional,
        "Security activates the Main effects; the trigger itself is not declinable"
    );
    assert_eq!(
        security_clause.scope,
        CompiledScope::Inherited,
        "BT24-089's [Security] text is the inherited security effect — same shape \
         as BT22-098 / P-035 / P-103 which all use scope: inherited"
    );
}

/// Security activates this card's [Main] effects, so it mirrors both the
/// union-zone play and the `place_self_as_delay_option` placement.
#[test]
fn bt24_089_security_clause_mirrors_main() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let security_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have an on_security clause");

    let (filter, optional) = security_clause
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectUnionZone {
                filter, optional, ..
            } => Some((filter, *optional)),
            _ => None,
        })
        .expect("security mirror must install a union-zone selection");
    assert!(optional, "the mirrored 'may play 1' choice stays optional");
    assert!(
        predicate_contains_name(filter, "Elizamon")
            && predicate_contains_name(filter, "Owen Dreadnought"),
        "security union-zone selection must filter to [Elizamon] or [Owen Dreadnought]"
    );
    assert!(
        security_clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayUnionBoundFree { .. })),
        "security mirror must play the selected target without paying cost"
    );
    assert!(
        process_contains_place_self_as_delay_option(&security_clause.process),
        "Security mirrors the Main effects and must include place_self_as_delay_option"
    );
}

/// BT24-089 has exactly 2 triggered clauses (main_from_hand + on_security) and
/// exactly 1 declarative Delay clause.
#[test]
fn bt24_089_has_two_triggered_and_one_delay_clause() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let triggered = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    let delays = compiled
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Delay { .. })
            )
        })
        .count();

    assert_eq!(
        triggered, 2,
        "expected 2 triggered clauses (main_from_hand + on_security); got {triggered}"
    );
    assert_eq!(delays, 1, "expected exactly 1 Delay clause; got {delays}");
}

// ─── §2 Main clause behavioral ────────────────────────────────────────────────

/// When BT24-089's [Main] effect is activated from hand and the player has an
/// Elizamon in hand, a pending union-zone selection fires.
///
/// Note: `activate_hand_main` fires the `MainFromHand` effect without paying
/// cost or physically moving the card — the correct dispatch for Option card
/// [Main] effects in the engine.
#[test]
fn bt24_089_main_from_hand_installs_union_zone_prompt() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_elizamon())
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "ELIZAMON"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must return true for BT24-089 at hand index 0"
    );

    let view = runner
        .pending_selection_view()
        .expect("Main clause must install a union-zone selection when Elizamon is in hand");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "selection should be a union-zone pick (hand ∪ trash)"
    );
    assert!(
        runner.pending_is_optional(),
        "PASS must be legal — the printed text says 'You may play 1'"
    );
}

/// When BT24-089's [Main] fires from hand, choosing the Elizamon plays it into
/// the battle area without paying its cost.
#[test]
fn bt24_089_main_plays_elizamon_from_hand_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_elizamon())
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "ELIZAMON"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let memory_before = runner.memory();
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for BT24-089");

    let view = runner
        .pending_selection_view()
        .expect("union-zone prompt must be present");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Elizamon should be an eligible union-zone target"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose Elizamon");
    runner.auto_resolve().expect("resolve Main");

    assert!(
        battle_area_contains(&runner, 0, "ELIZAMON"),
        "Elizamon must be played into the battle area"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "playing Elizamon via this effect must not charge its play cost"
    );
}

/// Plays an Owen Dreadnought from hand when BT24-089's [Main] fires.
#[test]
fn bt24_089_main_plays_owen_from_hand_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_owen())
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "OWEN"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let memory_before = runner.memory();
    assert!(runner.game.activate_hand_main(0, 0));

    let view = runner
        .pending_selection_view()
        .expect("union-zone prompt must be present");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Owen Dreadnought should be eligible"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose Owen Dreadnought");
    runner.auto_resolve().expect("resolve Main");

    assert!(
        battle_area_contains(&runner, 0, "OWEN"),
        "Owen Dreadnought must be played into the battle area"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "playing Owen Dreadnought via this effect must not charge its play cost"
    );
}

/// The Main union-zone choice is a real hand ∪ trash pick: Elizamon in trash and
/// Owen Dreadnought in hand are both eligible, non-matching cards are excluded.
#[test]
fn bt24_089_main_picks_from_hand_or_trash_in_one_masked_choice() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_elizamon())
        .add_card(make_owen())
        .add_card(make_non_target("BAD_HAND"))
        .add_card(make_non_target("BAD_TRASH"))
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "OWEN", "BAD_HAND"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Push Elizamon (eligible) and a non-matching Digimon into player 0's trash.
    for trash_id in &["ELIZAMON", "BAD_TRASH"] {
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
    assert!(runner.game.activate_hand_main(0, 0));

    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must be pending after Main activation");
    assert!(
        runner.pending_is_optional(),
        "union-zone pick must expose PASS — the text says 'You may'"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "only [Owen Dreadnought] (hand) and [Elizamon] (trash) should be eligible; \
         non-matching cards must be filtered out"
    );

    let trash_before = runner.game.players[0].trash.len();
    let hand_before = runner.game.players[0].hand.len();
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select one valid union target");
    runner.auto_resolve().expect("resolve Main");

    let trash_after = runner.game.players[0].trash.len();
    let hand_after = runner.game.players[0].hand.len();
    // BT24-089 itself was placed as a Delay permanent, so it left hand. Exactly
    // one *additional* card must leave hand ∪ trash (the chosen Elizamon/Owen).
    assert_eq!(
        (trash_before + hand_before) - (trash_after + hand_after),
        2,
        "BT24-089 (placed as Delay) and the chosen target both leave hand ∪ trash"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "play_union_bound_free must not charge the target card's play cost"
    );
}

/// When BT24-089's [Main] effect activates with no eligible card in either zone,
/// the Main clause resolves without panic and still places BT24-089.
#[test]
fn bt24_089_main_no_eligible_card_no_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    runner
        .auto_resolve()
        .expect("main resolves with no eligible target");
    // No panic is the primary assertion.
}

/// The Main player may DECLINE the optional Elizamon/Owen play. The clause still
/// resolves (and still places BT24-089 in the battle area).
#[test]
fn bt24_089_main_decline_optional_play_still_places_self() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_elizamon())
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "ELIZAMON"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert!(runner.game.activate_hand_main(0, 0));
    assert!(
        runner.pending_is_optional(),
        "the union-zone play must be declinable"
    );

    // Decline the optional union-zone play by resolving PASS.
    runner
        .decline_optional_trigger()
        .expect("decline the optional play");
    runner
        .auto_resolve()
        .expect("clause resolves after decline");

    assert!(
        hand_contains(&runner, 0, "ELIZAMON"),
        "declining the optional play must leave Elizamon in hand"
    );
    assert!(
        battle_area_contains(&runner, 0, "BT24-089"),
        "BT24-089 must still be placed in the battle area after declining"
    );
}

/// "Then, place this card in the battle area" — after the Main effect, BT24-089
/// becomes a battle-area Delay permanent owned by the controller.
#[test]
fn bt24_089_main_places_self_as_delayed_battle_area_permanent() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_elizamon())
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "ELIZAMON"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert!(runner.game.activate_hand_main(0, 0));
    runner.auto_resolve().expect("main selections resolve");

    let placed = runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "BT24-089")
        .expect("BT24-089 must become a battle-area Delay permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed { owner: 0, .. }
    ));
}

// ─── §3 Delay clause behavioral ──────────────────────────────────────────────

/// After the placing turn, when one of the controller's [Owen Dreadnought]s
/// suspends on their turn, the Delay opens: a Reptile/Dragonkin base selection
/// fires, then a hand evolution selection, then the digivolve. BT24-089 is
/// trashed as the Delay's cost.
#[test]
fn bt24_089_delay_opens_when_owen_dreadnought_suspends() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_owen())
        .add_card(make_reptile_base("BASE"))
        .add_card(make_reptile_liberator_evo("EVO"))
        .add_card(make_non_target("OTHER"))
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "EVO", "OTHER"])
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

    let owen = runner.place_on_field(0, "OWEN", Some(0));

    // Advance past the placing turn: end our turn, opponent's turn, back to ours.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    // Owen Dreadnought suspends — opens the event-gated Delay.
    runner.game.suspend(owen);

    let view = runner
        .pending_selection_view()
        .expect("Owen Dreadnought suspend must open the Delay base selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "Delay digivolve is optional at the Reptile/Dragonkin base selection"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Reptile Digimon base should be a legal target"
    );
}

/// The Delay does NOT open when a non-Owen permanent suspends.
#[test]
fn bt24_089_delay_does_not_open_for_non_owen_suspend() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_reptile_base("BASE"))
        .add_card(make_reptile_liberator_evo("EVO"))
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Trashed
    );

    // Advance past the placing turn.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    // A non-Owen permanent (the Reptile base, not an Owen Dreadnought) suspends.
    runner.game.suspend(base);

    assert!(
        runner.game.pending_selection.is_none(),
        "the Delay must NOT open when a non-[Owen Dreadnought] permanent suspends"
    );
}

/// Full Delay path: after Owen suspends, choose the Reptile base and the
/// Reptile+LIBERATOR hand card; the digivolve completes, the evolution card
/// leaves hand and joins the field stack, and BT24-089 is trashed as the cost.
#[test]
fn bt24_089_delay_body_digivolves_reptile_dragonkin_with_cost_reduction() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_owen())
        .add_card(make_reptile_base("BASE"))
        .add_card(make_reptile_liberator_evo("EVO"))
        .add_card(make_non_target("OTHER"))
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "EVO", "OTHER"])
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

    let owen = runner.place_on_field(0, "OWEN", Some(0));

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    runner.game.suspend(owen);

    // Base selection: choose the Reptile Digimon.
    {
        let view = runner
            .pending_selection_view()
            .expect("Owen suspend should fire the Reptile/Dragonkin base selection");
        assert_eq!(view.kind, SelectionKind::OwnField);
        assert_eq!(
            view.valid_action_ids.len(),
            1,
            "only the Reptile Digimon base should be legal"
        );
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("choose Reptile base");
    }

    // Hand evolution selection: choose the Reptile+LIBERATOR card.
    {
        let view = runner
            .pending_selection_view()
            .expect("choosing the base should expose the hand evolution selection");
        assert_eq!(view.kind, SelectionKind::Hand);
        assert_eq!(
            view.valid_action_ids.len(),
            1,
            "only the Reptile+LIBERATOR hand card should be legal"
        );
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("choose Reptile+LIBERATOR evolution");
    }

    runner.auto_resolve().expect("complete Delay digivolve");

    assert!(
        !hand_contains(&runner, 0, "EVO"),
        "the selected evolution card must leave hand"
    );
    assert!(
        battle_area_contains(&runner, 0, "EVO"),
        "the selected evolution card must become part of the field stack"
    );
    assert!(
        trash_contains(&runner, 0, "BT24-089"),
        "Delay activation must trash BT24-089 as its cost — \
         this trash IS the Delay's printed activation cost"
    );
}

/// Event-log test (cost-firing clause). The Delay's printed cost is "By trashing
/// this card …" — the trash IS the cost. The engine's `Trash` GameEvent variant
/// is not yet emitted by the DSL digivolve/trash path, so the concrete cost
/// proof is BT24-089 leaving the battle area for the trash. This test ALSO
/// checkpoints the event log around the Main free-play to confirm the
/// cost-bypassing play emits a `Play` event for the chosen Elizamon.
#[test]
fn bt24_089_main_free_play_emits_play_event() {
    use digimon_engine::events::GameEvent;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_elizamon())
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "ELIZAMON"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let checkpoint = runner.event_checkpoint();
    assert!(runner.game.activate_hand_main(0, 0));

    let view = runner
        .pending_selection_view()
        .expect("union-zone prompt present");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose Elizamon");
    runner.auto_resolve().expect("resolve Main");

    let events = runner.events_since(checkpoint);
    let played_elizamon = events
        .iter()
        .any(|e| matches!(e, GameEvent::Play { player: 0, card_id, .. } if card_id == "ELIZAMON"));
    assert!(
        played_elizamon,
        "the cost-free [Main] play must emit a Play event for Elizamon; events={events:?}"
    );
}

// ─── §4 Security clause behavioral ───────────────────────────────────────────

/// The on_security clause fires from the security trigger without panic and
/// installs the same union-zone selection as the Main clause.
#[test]
fn bt24_089_security_clause_fires_union_zone_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_elizamon())
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .hand(0, &["ELIZAMON"])
        .hand(1, &["FILL"])
        .memory(10)
        .start();

    let field_handle = runner.place_on_field(0, "BT24-089", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(field_handle),
    );
    runner.game.drain_effect_queue();

    // The security clause mirrors Main: a union-zone Elizamon/Owen pick fires.
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            matches!(view.kind, SelectionKind::UnionZone { .. }),
            "security clause should install a union-zone (hand ∪ trash) selection"
        );
    }

    // Resolve to completion without panic.
    runner.auto_resolve().expect("security clause resolves");
}
