//! BT22-088 Arisa Kinosaki.
//! Printed text covered here: Security Effect [Security] Play this card
//! without paying the cost, and the All Turns Token/Puppet played observer.
//!
//! Partial: the start-of-main return-this-Tamer cost and all-turns
//! Arisa/Shoemon play branch is omitted until the engine can expose the
//! required optional return-this-Tamer cost without auto-resolution.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledPredicate,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

#[test]
fn bt22_088_is_yellow_tamer_cost_3_liberator() {
    let runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .start();
    let compiled = runner.compiled_card("BT22-088").expect("compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.traits, vec!["LIBERATOR".to_string()]);
}

#[test]
fn bt22_088_exposes_security_and_token_or_puppet_played_observer() {
    let runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .start();
    let compiled = runner.compiled_card("BT22-088").expect("compiled card");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 2, "security and played-observer slices are live");
    let security = triggered
        .iter()
        .find(|triggered| triggered.when.contains(&CompiledTiming::OnSecurity))
        .expect("security clause is present");
    assert!(security.when.contains(&CompiledTiming::OnSecurity));
    assert!(!security.optional, "[Security] play this card is mandatory");
    assert_eq!(security.scope, CompiledScope::FaceUp);

    let observer = triggered
        .iter()
        .find(|triggered| {
            triggered
                .when
                .contains(&CompiledTiming::OnAnyDigimonPlayed)
        })
        .expect("All Turns Token/Puppet played observer is present");
    let condition = observer
        .condition
        .as_ref()
        .expect("observer needs event-target gates");
    assert!(
        predicate_has_event_target_kind(condition, CompiledCardKind::Token)
            && predicate_has_event_target_kind(condition, CompiledCardKind::Digimon),
        "observer must cover Tokens and Puppet Digimon"
    );
    assert!(
        predicate_has_event_target_trait(condition, "Puppet"),
        "Digimon branch must require Puppet"
    );
    assert!(
        steps_lead_with_activation_cost_suspend_self_then_draw(&observer.process),
        "body must lead with activation_cost: suspend_self then draw"
    );
}

#[test]
fn bt22_088_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER-BT22-088", "Attacker");
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .add_card(attacker)
        .security(1, &["BT22-088"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER-BT22-088", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT22-088"),
        "BT22-088 is played from the defender's security"
    );
}

#[test]
#[ignore = "BLOCKED: PUPPETS-G028 - single optional triggered effects auto-fire, so returning this Tamer cannot be exposed as a visible optional cost"]
fn bt22_088_start_of_main_can_decline_before_returning_tamer_to_deck_bottom() {
    todo!("place BT22-088 in battle area, enter start of main, assert PASS is legal before the Tamer leaves, then decline and assert it remains in battle area");
}

#[test]
#[ignore = "BLOCKED: PUPPETS-G028 - start-of-main return-self cost must be chosen before exact named free-play branches resolve"]
fn bt22_088_start_of_main_accepts_cost_then_plays_exact_arisa_from_hand() {
    todo!("after accepting the return-to-bottom cost, assert only exact-name Arisa Kinosaki, not longer names, is legal from hand and is played free");
}

#[test]
#[ignore = "BLOCKED: PUPPETS-G028 - no-Digimon Shoemon trash branch is chained after the blocked return-self cost"]
fn bt22_088_start_of_main_no_digimon_branch_plays_exact_shoemon_from_trash() {
    todo!("after accepting the return-to-bottom cost with no Digimon, assert only exact Shoemon, not ShoeShoemon, is legal from trash and is played free");
}

#[test]
fn bt22_088_all_turns_token_or_puppet_played_suspends_this_tamer_to_draw() {
    let mut runner = observer_runner(&["PUPPET-HAND-BT22-088"], &["DRAW-BT22-088"]);
    let arisa = runner.place_on_field(0, "BT22-088", Some(0));

    runner.play(0, 0).expect("own Puppet plays");
    runner.auto_resolve().expect("finish activation_cost + Draw 1");

    assert!(
        runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "BT22-088 must suspend as the activation cost"
    );
    assert_eq!(runner.hand_size(0), 1, "Draw 1 adds the deck card to hand");
    assert_eq!(runner.deck_size(0), 0, "Draw 1 consumes one deck card");
}

#[test]
fn bt22_088_token_played_by_effect_can_trigger_draw_observer() {
    let mut runner = observer_runner(&[], &["DRAW-BT22-088"]);
    let arisa = runner.place_on_field(0, "BT22-088", Some(0));
    let source = runner.place_on_field(0, "EFFECT-SOURCE-BT22-088", Some(0));

    let source_card = runner.top_card(source);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
    ctx.play_token(0, "familiar")
        .expect("effect should play Familiar Token");

    runner.auto_resolve().expect("finish Token observer");

    assert!(
        runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "BT22-088 must suspend for the Token branch"
    );
    assert_eq!(runner.hand_size(0), 1);
    assert_eq!(runner.deck_size(0), 0);
}

#[test]
fn bt22_088_played_observer_silently_skips_when_arisa_already_suspended() {
    let mut runner = observer_runner(&["PUPPET-HAND-BT22-088"], &["DRAW-BT22-088"]);
    let arisa = runner.place_on_field(0, "BT22-088", Some(0));
    runner.game.player_mut(0).battle_area[arisa.index as usize].is_suspended = true;

    runner.play(0, 0).expect("own Puppet plays");
    runner.auto_resolve().expect("finish trigger");

    assert!(
        runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "pre-suspended Arisa stays suspended (cost cannot be paid again)"
    );
    assert_eq!(runner.hand_size(0), 0, "cost failure must not draw");
    assert_eq!(runner.deck_size(0), 1, "cost failure leaves the deck untouched");
}

#[test]
fn bt22_088_played_observer_rejects_non_puppet_and_opponent_puppet() {
    let mut runner = observer_runner(
        &["NON-PUPPET-BT22-088"],
        &["DRAW-BT22-088", "DRAW-BT22-088"],
    );
    runner.place_on_field(0, "BT22-088", Some(0));

    runner.play(0, 0).expect("own non-Puppet plays");
    runner.auto_resolve().expect("settle non-Puppet play");
    assert!(
        runner.pending_selection_view().is_none(),
        "own non-Puppet Digimon must not trigger"
    );

    runner.play(1, 0).expect("opponent Puppet plays");
    runner.auto_resolve().expect("settle opponent Puppet play");
    assert!(
        runner.pending_selection_view().is_none(),
        "opponent Puppet Digimon must not trigger"
    );
}

#[test]
fn bt22_088_suspend_cost_preflight_is_bound_to_this_tamer() {
    let mut runner = observer_runner(&["PUPPET-HAND-BT22-088"], &["DRAW-BT22-088", "DRAW-BT22-088"]);
    let suspended = runner.place_on_field(0, "BT22-088", Some(0));
    let unsuspended = runner.place_on_field(0, "BT22-088", Some(1));
    runner.game.player_mut(0).battle_area[suspended.index as usize].is_suspended = true;

    runner.play(0, 0).expect("own Puppet plays");
    runner.auto_resolve().expect("finish source-bound activation");

    assert!(
        runner.game.player(0).battle_area[suspended.index as usize].is_suspended,
        "already-suspended source stays suspended and must not pay the cost again"
    );
    assert!(
        runner.game.player(0).battle_area[unsuspended.index as usize].is_suspended,
        "the unsuspended BT22-088 source should pay the cost"
    );
    assert!(
        runner.pending_selection_view().is_none(),
        "only the source that can pay the suspend cost should trigger"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "exactly one Arisa pays the cost and draws once"
    );
}

fn observer_runner(hand: &[&str], deck: &[&str]) -> DebugRunner {
    let mut source = make_test_card("EFFECT-SOURCE-BT22-088", "Effect Source");
    source.card_kind = CardKind::Digimon;
    source.level = Some(3);
    source.play_cost = 0;
    source.dp = Some(1000);

    DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .add_card(source)
        .add_card(puppet_digimon("PUPPET-HAND-BT22-088"))
        .add_card(non_puppet_digimon("NON-PUPPET-BT22-088"))
        .add_card(draw_card("DRAW-BT22-088"))
        .hand(0, hand)
        .hand(1, &["PUPPET-HAND-BT22-088"])
        .deck(0, deck)
        .memory(20)
        .start()
}

fn puppet_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.traits = vec!["Puppet".to_string()];
    card
}

fn non_puppet_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.traits = vec!["LIBERATOR".to_string()];
    card
}

fn draw_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 2;
    card
}

fn predicate_has_event_target_kind(predicate: &CompiledPredicate, kind: CompiledCardKind) -> bool {
    predicate.event_target_kind == Some(kind)
        || predicate
            .all_of
            .iter()
            .any(|child| predicate_has_event_target_kind(child, kind))
        || predicate
            .any_of
            .iter()
            .any(|child| predicate_has_event_target_kind(child, kind))
}

fn predicate_has_event_target_trait(predicate: &CompiledPredicate, trait_name: &str) -> bool {
    predicate.event_target_trait_has.as_deref() == Some(trait_name)
        || predicate
            .all_of
            .iter()
            .any(|child| predicate_has_event_target_trait(child, trait_name))
        || predicate
            .any_of
            .iter()
            .any(|child| predicate_has_event_target_trait(child, trait_name))
}

fn steps_lead_with_activation_cost_suspend_self_then_draw(steps: &[CompiledStep]) -> bool {
    use digimon_dsl::compiled::CompiledActivationCostKind;
    let mut iter = steps.iter();
    let leads_with_suspend_self = matches!(
        iter.next(),
        Some(CompiledStep::ActivationCost {
            kind: CompiledActivationCostKind::SuspendSelf
        })
    );
    if !leads_with_suspend_self {
        return false;
    }
    matches!(
        iter.next(),
        Some(CompiledStep::Draw { count: 1, .. })
    )
}
