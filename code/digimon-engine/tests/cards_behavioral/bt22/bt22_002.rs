//! BT22-002 Kyaromon — Digi-Egg, Lv.2, Blue.
//! Traits: Lesser, LIBERATOR.
//!
//! # Card text (cards.json)
//!
//! Inherited Effect [Your Turn] [Once Per Turn] When any of your Tokens or
//! other [Puppet] trait Digimon are deleted, <Draw 1> (Draw 1 card from your
//! deck.).
//!
//! # Patterns this test covers
//! - G4 inherited triggered effect on Digi-Egg
//! - Event observer: on_any_deletion with event-target kind/owner/trait filters
//! - Once-per-turn mandatory draw
//! - Printed "other [Puppet]" exclusion for the carrier itself
//!
//! # Blocked behavior
//!
//! The inherited deletion observer is intentionally omitted from YAML pending
//! `G-ON-ANY-DELETION-EVENT-CONTEXT`: OnAnyDeletion observer fan-out currently
//! scans PlayerBattleArea without carrying the deleted permanent/card in
//! TriggerContext.event_permanent/event_card. As a result, event_target_*
//! predicates evaluate the observing carrier, not the deleted object.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::{Permanent, PermanentHandle};

fn make_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn make_token(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Token;
    card.level = None;
    card.dp = Some(3000);
    card.play_cost = 0;
    card.traits = Vec::new();
    card
}

fn runner_with_kyaromon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT22-002")
        .expect("BT22-002 in embedded DSL pack")
        .add_card(make_digimon("CARRIER-PUPPET", &["Puppet"]))
        .add_card(make_digimon("ALLY-PUPPET", &["Puppet"]))
        .add_card(make_digimon("ALLY-BEAST", &["Beast"]))
        .add_card(make_token("TEST-TOKEN"))
        .add_card(make_test_card("DECK-A", "Deck A"))
        .add_card(make_test_card("DECK-B", "Deck B"))
        .add_card(make_test_card("DECK-C", "Deck C"))
        .deck(0, &["DECK-A", "DECK-B", "DECK-C"])
        .memory(10)
        .start()
}

fn place_kyaromon_under_carrier(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_stack(0, &["BT22-002", "CARRIER-PUPPET"])
}

fn place_token_on_field(runner: &mut DebugRunner, player: u8, card_id: &str) -> PermanentHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("token card data registered");
    let next_idx = runner.game.next_card_index();
    let mut source = CardSource::new_token(data_idx, player, next_idx);
    source.card_index = next_idx;
    let permanent = Permanent::new(source, runner.game.turn_count);
    runner.game.players[player as usize]
        .battle_area
        .push(permanent);
    PermanentHandle {
        player,
        index: (runner.game.players[player as usize].battle_area.len() - 1) as u8,
    }
}

#[test]
fn bt22_002_is_blue_digi_egg_with_lesser_liberator_traits() {
    let runner = runner_with_kyaromon();
    let card = runner
        .compiled_card("BT22-002")
        .expect("BT22-002 compiled card present");

    assert_eq!(
        card.kind,
        CompiledCardKind::DigiEgg,
        "kind must be Digi-Egg"
    );
    assert_eq!(card.level, Some(2), "level must be 2");
    assert!(
        card.color.contains(&CompiledColor::Blue),
        "BT22-002 must be blue"
    );
    assert!(
        card.traits.iter().any(|trait_name| trait_name == "Lesser"),
        "BT22-002 must keep Lesser trait"
    );
    assert!(
        card.traits
            .iter()
            .any(|trait_name| trait_name == "LIBERATOR"),
        "BT22-002 must keep LIBERATOR trait"
    );
}

#[test]
fn bt22_002_deletion_observer_is_omitted_pending_deleted_event_context() {
    let runner = runner_with_kyaromon();
    let card = runner
        .compiled_card("BT22-002")
        .expect("BT22-002 compiled card present");

    let triggered_count = card
        .effects
        .iter()
        .filter(|clause| matches!(clause, CompiledClause::Triggered(_)))
        .count();

    assert_eq!(
        triggered_count, 0,
        "BT22-002 must not ship an approximating OnAnyDeletion clause until \
         deleted-permanent event context is available"
    );
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — OnAnyDeletion lacks deleted permanent/card TriggerContext for event_target_* predicates"]
fn bt22_002_draws_when_own_token_is_deleted() {
    let mut runner = runner_with_kyaromon();
    place_kyaromon_under_carrier(&mut runner);
    let token = place_token_on_field(&mut runner, 0, "TEST-TOKEN");

    let hand_before = runner.hand_size(0);
    runner.game.delete_permanent_with_effects(token);
    runner.auto_resolve().expect("resolve deletion observer");

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "own Token deletion should trigger inherited Draw 1"
    );
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — OnAnyDeletion lacks deleted permanent/card TriggerContext for event_target_* predicates"]
fn bt22_002_draws_when_own_other_puppet_digimon_is_deleted() {
    let mut runner = runner_with_kyaromon();
    place_kyaromon_under_carrier(&mut runner);
    let puppet = runner.place_on_field(0, "ALLY-PUPPET", None);

    let hand_before = runner.hand_size(0);
    runner.game.delete_permanent_with_effects(puppet);
    runner.auto_resolve().expect("resolve deletion observer");

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "own other Puppet Digimon deletion should trigger inherited Draw 1"
    );
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — OnAnyDeletion lacks deleted permanent/card TriggerContext for event_target_* predicates"]
fn bt22_002_does_not_draw_for_own_non_puppet_digimon_deletion() {
    let mut runner = runner_with_kyaromon();
    place_kyaromon_under_carrier(&mut runner);
    let non_puppet = runner.place_on_field(0, "ALLY-BEAST", None);

    let hand_before = runner.hand_size(0);
    runner.game.delete_permanent_with_effects(non_puppet);
    runner.auto_resolve().expect("resolve deletion");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "own non-Puppet Digimon deletion must not trigger BT22-002"
    );
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — OnAnyDeletion lacks deleted permanent/card TriggerContext for event_target_* predicates"]
fn bt22_002_does_not_draw_for_opponent_puppet_deletion() {
    let mut runner = runner_with_kyaromon();
    place_kyaromon_under_carrier(&mut runner);
    let opponent_puppet = runner.place_on_field(1, "ALLY-PUPPET", None);

    let hand_before = runner.hand_size(0);
    runner.game.delete_permanent_with_effects(opponent_puppet);
    runner.auto_resolve().expect("resolve deletion");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "opponent Puppet deletion must not trigger your BT22-002"
    );
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — OnAnyDeletion lacks deleted permanent/card TriggerContext for event_target_* predicates"]
fn bt22_002_does_not_draw_when_carrier_itself_is_deleted() {
    let mut runner = runner_with_kyaromon();
    let carrier = place_kyaromon_under_carrier(&mut runner);

    let hand_before = runner.hand_size(0);
    runner.game.delete_permanent_with_effects(carrier);
    runner.auto_resolve().expect("resolve carrier deletion");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "printed 'other [Puppet]' excludes the carrier itself"
    );
}

#[test]
#[ignore = "pending: G-ON-ANY-DELETION-EVENT-CONTEXT — OnAnyDeletion lacks deleted permanent/card TriggerContext for event_target_* predicates"]
fn bt22_002_once_per_turn_blocks_second_eligible_deletion() {
    let mut runner = runner_with_kyaromon();
    place_kyaromon_under_carrier(&mut runner);
    let token = place_token_on_field(&mut runner, 0, "TEST-TOKEN");
    let puppet = runner.place_on_field(0, "ALLY-PUPPET", None);

    let hand_before = runner.hand_size(0);
    runner.game.delete_permanent_with_effects(token);
    runner
        .auto_resolve()
        .expect("resolve first deletion observer");
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "first eligible deletion should draw 1"
    );

    runner.game.delete_permanent_with_effects(puppet);
    runner
        .auto_resolve()
        .expect("resolve second deletion observer");
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "second eligible deletion in same turn must be blocked by OPT"
    );
}
