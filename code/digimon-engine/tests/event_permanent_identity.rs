//! Event subjects retain permanent identity through ordinary stack edits (rules
//! 3-4-4/3-4-5, which absorbed 3.6's 8-1-2-3/-8), but DNA makes a new Digimon (8-2-2-1, official manual pp.18-20).
use digimon_dsl::compiled::CompiledBindingRef;
use digimon_engine::{debug_runner::{DebugRunner, make_test_card, make_test_card_with_level}, effect_context::EffectContext,
    enums::EffectTiming, game::Game, permanent::{Permanent, PermanentHandle},
    selection::TriggerSource, trigger_context::TriggerContext};
use digimon_engine::dsl_cards::{binding_ref::{resolve_binding_ref, ResolvedBinding}, bindings::Bindings};

fn setup() -> (DebugRunner, PermanentHandle, TriggerContext) {
    let mut r = DebugRunner::builder().from_dsl_yaml(r#"
card: OBSERVER
name: Observer
kind: tamer
color: [white]
cost: 3
effects:
  - when: on_digivolve
    process:
      - gain_memory: 1
  - when: on_own_security_removed
    process:
      - gain_memory: 1
  - when: on_digivolution_card_trashed
    process:
      - gain_memory: 1
"#).unwrap()
        .add_card(make_test_card_with_level("BASE", "Base", 4))
        .add_card(make_test_card("LOWER", "Lower"))
        .add_card(make_test_card("EVOLUTION", "Evolution"))
        .hand(0, &["EVOLUTION", "LOWER"]).start();
    r.place_on_field(0, "OBSERVER", None);
    let subject = r.place_on_field(0, "BASE", None);
    let card = r.game.players[0].battle_area[subject.index as usize].top_card().handle();
    r.game.enqueue_triggered(EffectTiming::OnDigivolve, TriggerSource::Digivolved {
        player: 0, permanent: subject, card, effect_initiated: false, dna_origin: false,
    });
    let context = r.game.effect_queue.pop_front().expect("observer enqueued").trigger_context.unwrap();
    (r, subject, context)
}

fn resolve(game: &mut Game, context: TriggerContext) -> Option<ResolvedBinding> {
    let source = context.event_card.unwrap();
    game.current_trigger_context = Some(context);
    let ctx = EffectContext::new(game, source, None, 0);
    let resolved = resolve_binding_ref(&CompiledBindingRef::EventTarget, &ctx, &Bindings::new());
    let expected = match resolved { Some(ResolvedBinding::Permanent(handle)) => Some(handle), _ => None };
    assert_eq!(ctx.event_permanent(), expected);
    assert_eq!(ctx.as_read().event_permanent(), expected);
    resolved
}

#[test]
fn event_subject_follows_compaction_and_ordinary_evolution_in_cloned_games() {
    let (mut r, subject, context) = setup();
    let mut fork = r.game.clone();
    for game in [&mut r.game, &mut fork] {
        let evolution = game.players[0].hand.remove(0);
        assert!(game.digivolve_onto(0, subject.index as usize, evolution));
        game.players[0].battle_area.remove(0);
        assert_eq!(resolve(game, context.clone()), Some(ResolvedBinding::Permanent(PermanentHandle { player: 0, index: 0 })));
    }
}

#[test]
fn event_subject_survives_removal_of_its_former_top_during_de_digivolution() {
    let (mut r, subject, context) = setup();
    // The event's original top can itself be removed; a surviving lower card
    // remains the same Digimon, so card-containment tracking is insufficient.
    let lower = r.game.players[0].hand.remove(1);
    r.game.players[0].battle_area[subject.index as usize].card_sources.insert(0, lower);
    assert!(r.game.de_digivolve_from_effect(subject, 1, 1));
    assert_eq!(resolve(&mut r.game, context), Some(ResolvedBinding::Permanent(subject)));
}

#[test]
fn event_subject_does_not_rebind_to_same_physical_card_replayed() {
    let (mut r, subject, context) = setup();
    let departed = r.game.players[0].battle_area.remove(subject.index as usize);
    let same_card = departed.top_card().clone();
    r.game.players[0].battle_area.push(Permanent::new(same_card, 1));
    assert!(!matches!(resolve(&mut r.game, context), Some(ResolvedBinding::Permanent(_))));
}

#[test]
fn event_subject_does_not_rebind_after_dna() {
    let (mut r, subject, context) = setup();
    let partner = r.place_on_field(0, "BASE", None);
    let card = r.game.players[0].hand[0].handle();
    let mut ctx = EffectContext::new(&mut r.game, card, None, 0);
    assert!(ctx.effect_initiated_dna_digivolve(subject, partner, card, 0, true).is_some());
    assert!(!matches!(resolve(&mut r.game, context.clone()), Some(ResolvedBinding::Permanent(_))));
    for pred in live_predicates() {
        assert!(!matches_event(&mut r.game, context.clone(), pred.clone()), "DNA result must not inherit event subject {pred:?}");
    }
}

fn matches_event(game: &mut Game, context: TriggerContext, pred: digimon_dsl::compiled::CompiledPredicate) -> bool {
    use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
    let card = context.event_card.unwrap();
    game.current_trigger_context = Some(context);
    let ctx = EffectContext::new(game, card, None, 0);
    eval_predicate(&pred, &ctx.as_read(), PredicateSubject::None)
}

fn live_predicates() -> Vec<digimon_dsl::compiled::CompiledPredicate> {
    use digimon_dsl::compiled::{CompiledPredicate, CompiledDpConstraint, CompiledCardKind};
    vec![
        CompiledPredicate { event_target_stack_size_gte: Some(2), ..Default::default() },
        CompiledPredicate { event_target_dp_eq: Some(CompiledDpConstraint::Literal(2000)), ..Default::default() },
        CompiledPredicate { event_target_same_level_as_previous: Some(true), ..Default::default() },
        CompiledPredicate { event_target_kind: Some(CompiledCardKind::Digimon), ..Default::default() },
    ]
}

#[test]
fn live_event_predicates_follow_compaction() {
    let (mut r, subject, context) = setup();
    let same_level = r.place_on_field(0, "BASE", None);
    let card = r.game.players[0].battle_area.remove(same_level.index as usize).top_card().clone();
    assert!(r.game.digivolve_onto(0, subject.index as usize, card));
    r.game.players[0].battle_area.remove(0);
    for pred in live_predicates() {
        assert!(matches_event(&mut r.game, context.clone(), pred.clone()), "live moved subject must satisfy {pred:?}");
    }
}

#[test]
fn live_event_predicates_reject_departed_replayed_subject() {
    let (mut r, subject, context) = setup();
    let card = context.event_card.unwrap();
    r.game.delete_permanent_with_cause(subject, digimon_engine::replacement::ReplacementCause::OwnEffect);
    assert!(r.game.players[0].trash.iter().any(|c| c.handle() == card));
    let mut ctx = EffectContext::new(&mut r.game, card, None, 0);
    let replayed = ctx.play_from_trash_free_unsuspended(card).expect("same physical card replayed");
    let extra = r.place_on_field(0, "BASE", None);
    let top = r.game.players[0].battle_area.remove(extra.index as usize).top_card().clone();
    assert!(r.game.digivolve_onto(0, replayed.index as usize, top));
    let mut predicates = live_predicates();
    predicates.push(digimon_dsl::compiled::CompiledPredicate { event_target_same_level_as_previous: Some(false), ..Default::default() });
    for pred in predicates {
        assert!(!matches_event(&mut r.game, context.clone(), pred.clone()), "departed subject must not satisfy {pred:?}");
    }
}

#[test]
fn card_events_do_not_turn_the_observer_into_event_permanent() {
    use digimon_engine::trigger_context::EventCause;
    let (mut r, host, _) = setup();
    let card = r.game.players[0].hand[1].handle();
    let host_card = r.game.players[0].battle_area[host.index as usize].top_card().handle();
    let events = [
        (EffectTiming::OnOwnSecurityRemoved, TriggerSource::SecurityRemoved {
            affected_player: 0, observer_player: 0, source_player: 1, card, cause: EventCause::SecurityRemoval,
        }),
        (EffectTiming::OnDigivolutionCardTrashed, TriggerSource::SourceTrashedFromStack {
            player: 0, host, host_card, card, cause: EventCause::OpponentEffect,
        }),
    ];
    for (timing, source) in events {
        r.game.enqueue_triggered(timing, source);
        let event = r.game.effect_queue.pop_front().expect("observer queued").trigger_context.unwrap();
        assert!(event.event_permanent.is_none());
        r.game.current_trigger_context = Some(event);
        let ctx = EffectContext::new(&mut r.game, card, None, 0);
        assert_eq!(ctx.event_permanent(), None, "{timing:?} must not invent observer as subject");
        assert_eq!(ctx.as_read().event_permanent(), None);
    }
}

#[test]
fn event_self_comparisons_follow_compaction() {
    use digimon_dsl::compiled::CompiledPredicate;
    use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
    let (mut r, _, context) = setup();
    r.game.players[0].battle_area.remove(0);
    let moved = PermanentHandle { player: 0, index: 0 };
    let card = context.event_card.unwrap();
    r.game.current_trigger_context = Some(context);
    let ctx = EffectContext::new(&mut r.game, card, Some(moved), 0);
    for pred in [
        CompiledPredicate { event_target_is_source: Some(true), ..Default::default() },
        CompiledPredicate { event_permanent_is_source: Some(true), ..Default::default() },
    ] {
        assert!(eval_predicate(&pred, &ctx.as_read(), PredicateSubject::Permanent(moved)), "{pred:?}");
    }
}

#[test]
fn live_event_card_predicates_follow_new_top_after_ordinary_evolution() {
    use digimon_dsl::compiled::CompiledPredicate;
    let (mut r, subject, context) = setup();
    let evolution = r.game.players[0].hand.remove(0);
    assert!(r.game.digivolve_onto(0, subject.index as usize, evolution));
    assert!(matches_event(&mut r.game, context, CompiledPredicate { event_target_level_eq: Some(3), ..Default::default() }));
}

#[test]
fn captured_event_card_level_remains_immutable_after_evolution_and_departure() {
    use digimon_dsl::compiled::CompiledPredicate;
    let (mut r, subject, context) = setup();
    let evolution = r.game.players[0].hand.remove(0);
    assert!(r.game.digivolve_onto(0, subject.index as usize, evolution));
    assert!(matches_event(&mut r.game, context.clone(), CompiledPredicate { event_target_level_eq: Some(3), ..Default::default() }));
    let captured = CompiledPredicate { event_card_level_eq: Some(4), ..Default::default() };
    assert!(matches_event(&mut r.game, context.clone(), captured.clone()), "event card remains original level 4 while live target is level 3");
    r.game.delete_permanent_with_cause(subject, digimon_engine::replacement::ReplacementCause::OwnEffect);
    assert!(matches_event(&mut r.game, context.clone(), captured), "captured event card retains level after departure");
    assert!(!matches_event(&mut r.game, context, CompiledPredicate { event_target_level_eq: Some(3), ..Default::default() }));
}
