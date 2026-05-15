//! ST19-14 Arisa Kinosaki.
//! Printed text covered here: [Start of Your Turn] memory setter,
//! [Your Turn] effect-played Token/Puppet Rush grant, and [Security] play this card.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCardKind, CompiledClause, CompiledPlayerRef, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

#[test]
fn st19_14_start_of_turn_sets_memory_to_3_when_lte_2() {
    let filler = make_test_card("FILLER-ST19-14", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-14")
        .expect("ST19-14 YAML loads")
        .add_card(filler)
        .deck(0, &["FILLER-ST19-14"])
        .deck(1, &["FILLER-ST19-14"])
        .memory(2)
        .start();

    runner.place_on_field(0, "ST19-14", Some(0));
    runner.game.memory = 2;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        3,
        "Arisa sets memory to 3 at start of your turn when memory is 2 or less"
    );
}

#[test]
fn st19_14_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-14")
        .expect("ST19-14 YAML loads")
        .add_card(attacker)
        .security(1, &["ST19-14"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "ST19-14"),
        "Arisa is played from the defender's security"
    );
}

#[test]
fn st19_14_has_effect_played_token_or_puppet_rush_observer() {
    let runner = runner();
    let card = runner
        .compiled_card("ST19-14")
        .expect("ST19-14 must be compiled");

    let observer = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnAnyDigimonPlayed) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Your Turn effect-played Token/Puppet Rush observer should exist");

    let condition = observer
        .condition
        .as_ref()
        .expect("observer must inspect the played-object event context");
    assert!(
        predicate_has_event_target_owner(condition, CompiledPlayerRef::You),
        "observer must be scoped to your played objects"
    );
    assert!(
        condition.event_is_effect_initiated == Some(true)
            || condition
                .all_of
                .iter()
                .any(|child| child.event_is_effect_initiated == Some(true)),
        "observer must reject normal player-action plays"
    );
    assert!(
        predicate_has_event_target_kind(condition, CompiledCardKind::Token)
            && predicate_has_event_target_kind(condition, CompiledCardKind::Digimon),
        "observer must cover Tokens and Puppet Digimon"
    );
    assert!(
        predicate_has_event_target_trait(condition, "Puppet"),
        "Digimon branch must require the Puppet trait"
    );
    assert!(
        steps_grant_rush_to_event_target(&observer.process),
        "the played Token/Puppet event target must gain Rush for the turn"
    );
}

#[test]
fn st19_14_effect_played_puppet_may_gain_rush_by_suspending_arisa() {
    let mut runner = rush_runner();
    let arisa = runner.place_on_field(0, "ST19-14", Some(0));

    let played = effect_play_puppet_from_hand(&mut runner, arisa);

    let choice = runner
        .pending_selection_view()
        .expect("effect-played Puppet should offer Arisa activation");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(choice.selecting_player, choice.valid_action_ids[0])
        .expect("accept Arisa activation");
    runner.auto_resolve().expect("finish Rush grant");

    assert!(
        runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "Arisa must suspend as the visible activation cost"
    );
    assert!(
        runner.game.has_keyword(played, Keyword::Rush),
        "the effect-played Puppet Digimon should gain Rush"
    );
}

#[test]
fn st19_14_effect_played_token_may_gain_rush_by_suspending_arisa() {
    let mut runner = rush_runner();
    let arisa = runner.place_on_field(0, "ST19-14", Some(0));

    let played = effect_play_familiar_token(&mut runner, arisa);

    let choice = runner
        .pending_selection_view()
        .expect("effect-played Token should offer Arisa activation");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(choice.selecting_player, choice.valid_action_ids[0])
        .expect("accept Arisa activation");
    runner.auto_resolve().expect("finish Token Rush grant");

    assert!(
        runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "Arisa must suspend as the visible activation cost"
    );
    assert!(
        runner.game.has_keyword(played, Keyword::Rush),
        "the effect-played Token should gain Rush"
    );
}

#[test]
fn st19_14_effect_played_puppet_rush_grant_can_be_declined() {
    let mut runner = rush_runner();
    let arisa = runner.place_on_field(0, "ST19-14", Some(0));

    let played = effect_play_puppet_from_hand(&mut runner, arisa);

    let choice = runner
        .pending_selection_view()
        .expect("effect-played Puppet should offer Arisa activation");
    runner
        .execute_action(choice.selecting_player, choice.valid_action_ids[1])
        .expect("decline Arisa activation");
    runner.auto_resolve().expect("finish declined trigger");

    assert!(
        !runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "declining must not pay the suspend cost"
    );
    assert!(
        !runner.game.has_keyword(played, Keyword::Rush),
        "declining must not grant Rush"
    );
}

#[test]
fn st19_14_normal_played_puppet_does_not_trigger_rush_grant() {
    let mut runner = rush_runner();
    runner.place_on_field(0, "ST19-14", Some(0));

    runner.play(0, 0).expect("normal play Puppet from hand");

    assert!(
        runner.pending_selection_view().is_none(),
        "normal player-action plays must not satisfy 'effects play'"
    );
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST19-14")
        .expect("ST19-14 YAML loads")
        .start()
}

fn rush_runner() -> DebugRunner {
    let mut source = make_test_card("EFFECT-SOURCE-ST19-14", "Effect Source");
    source.card_kind = CardKind::Digimon;
    source.level = Some(3);
    source.play_cost = 0;
    source.dp = Some(1000);

    DebugRunner::builder()
        .dsl_card("ST19-14")
        .expect("ST19-14 YAML loads")
        .add_card(source)
        .add_card(puppet_digimon("PUPPET-HAND-ST19-14"))
        .hand(0, &["PUPPET-HAND-ST19-14"])
        .memory(20)
        .start()
}

fn effect_play_puppet_from_hand(
    runner: &mut DebugRunner,
    source: PermanentHandle,
) -> PermanentHandle {
    let source_card = runner.top_card(source);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
    ctx.play_from_hand_free(0, 0)
        .expect("effect should play Puppet from hand")
}

fn effect_play_familiar_token(runner: &mut DebugRunner, source: PermanentHandle) -> PermanentHandle {
    let source_card = runner.top_card(source);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
    ctx.play_token(0, "familiar")
        .expect("effect should play Familiar Token")
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

fn predicate_has_event_target_kind(
    predicate: &digimon_dsl::compiled::CompiledPredicate,
    kind: CompiledCardKind,
) -> bool {
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

fn predicate_has_event_target_owner(
    predicate: &digimon_dsl::compiled::CompiledPredicate,
    owner: CompiledPlayerRef,
) -> bool {
    predicate.event_target_owner == Some(owner)
        || predicate
            .all_of
            .iter()
            .any(|child| predicate_has_event_target_owner(child, owner))
        || predicate
            .any_of
            .iter()
            .any(|child| predicate_has_event_target_owner(child, owner))
}

fn predicate_has_event_target_trait(
    predicate: &digimon_dsl::compiled::CompiledPredicate,
    trait_name: &str,
) -> bool {
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

fn steps_grant_rush_to_event_target(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|step| match step {
        CompiledStep::GrantKeyword {
            target: CompiledBindingRef::EventTarget,
            keyword,
            expiry,
            ..
        } => keyword.eq_ignore_ascii_case("Rush") && expiry == "end_of_turn",
        CompiledStep::If {
            then, else_branch, ..
        } => steps_grant_rush_to_event_target(then) || steps_grant_rush_to_event_target(else_branch),
        CompiledStep::AsSelectingPlayer { body, .. }
        | CompiledStep::ForEach { body, .. }
        | CompiledStep::PerSelected { body, .. }
        | CompiledStep::ScheduleDelayed { body, .. } => steps_grant_rush_to_event_target(body),
        _ => false,
    })
}
