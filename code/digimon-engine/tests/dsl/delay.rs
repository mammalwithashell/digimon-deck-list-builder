//! Tests for Task 3: Delay clause lowering.
//!
//! `CompiledDeclarativeClause::Delay` should emit exactly one `Effect` with
//! `timing == DelayEffect`. The `delay_trigger` field is mapped from
//! `CompiledTiming::EndOfYourTurn` → `DelayTrigger::EndOfThisTurn`; attack
//! and event observer timings map to event-backed triggers; other timings
//! default to `DelayTrigger::EndOfYourNextTurn`.
//! `CompiledScope::Inherited` sets `inherited == true`.

use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledPredicate,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectSourceKind, EffectTiming};
use digimon_engine::permanent::OptionState;
use std::sync::Arc;

// ── Fixture ───────────────────────────────────────────────────────────────────

/// `fixture_delay` builds a clause with `process: []`, which `lower_delay.rs`
/// treats as a **marker-only** Delay (ST23-15 / ST24-15 print no `<Delay>` and
/// carry `kind: delay` with an empty process purely so the Option stays in the
/// battle area for their real clause). A marker is deliberately not activatable
/// and therefore not optional.
///
/// Tests about `<Delay>`'s §16-16-2 optionality need an ACTIVATABLE Delay, so
/// they use this instead. Added rather than changing `fixture_delay`, which
/// seven passing tests depend on for its marker-only shape.
fn fixture_delay_activatable(scope: CompiledScope, trigger: CompiledTiming) -> CompiledCard {
    let mut card = fixture_delay(scope, trigger);
    if let CompiledClause::Declarative(CompiledDeclarativeClause::Delay { process, .. }) =
        &mut card.effects[0]
    {
        *process = vec![CompiledStep::Draw {
            of: digimon_dsl::compiled::CompiledPlayerRef::You,
            count: 1,
        }];
    }
    card
}

fn fixture_delay(scope: CompiledScope, trigger: CompiledTiming) -> CompiledCard {
    CompiledCard {
        card: "F-DELAY".into(),
        name: "Fixture Delay".into(),
        kind: CompiledCardKind::Option,
        level: None,
        color: vec![],
        cost: Some(4),
        dp: None,
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
        dual: None,
        use_requirement: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::Delay {
                scope,
                active_when: None,
                trigger,
                process: vec![],
                summary: None,
                summary_key: None,
            },
        )],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// (a) `CompiledTiming::EndOfYourTurn` maps to `DelayTrigger::EndOfThisTurn`
/// and the emitted effect has `timing == DelayEffect`.
#[test]
fn delay_end_of_your_turn_maps_to_end_of_this_turn() {
    let dsl = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::EndOfYourTurn,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1, "expected exactly one effect for Delay");
    assert_eq!(
        effects[0].timing,
        EffectTiming::DelayEffect,
        "Delay effect must have DelayEffect timing"
    );
    assert_eq!(
        effects[0].delay_trigger,
        Some(DelayTrigger::EndOfThisTurn),
        "EndOfYourTurn must map to EndOfThisTurn"
    );
}

/// PUPPETS-G009 — `CompiledTiming::Delayed` (`trigger: delayed` in YAML) maps
/// to `DelayTrigger::MainPhaseActivated`: the standard printed `<Delay>`
/// player-visible `[Main]`-phase activation. The emitted effect still has
/// `timing == DelayEffect`.
#[test]
fn delay_delayed_trigger_maps_to_main_phase_activated() {
    let dsl = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::Delayed,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, EffectTiming::DelayEffect);
    assert_eq!(
        effects[0].delay_trigger,
        Some(DelayTrigger::MainPhaseActivated),
        "`delayed` trigger must map to the player-activated MainPhaseActivated"
    );
}

/// (b) A non-turn-scheduled timing (e.g. `OnPlay`) is **event-gated**.
///
/// This used to assert `EndOfYourNextTurn`, which was the `_ =>` catch-all that
/// `73733429d` removed and its message calls "THE BIGGER DEFECT ... the silent
/// default": a Delay on any unlisted timing became a turn-scheduled auto-fire —
/// wrong trigger, wrong time. Turn-scheduled forms are now enumerated
/// explicitly and everything else is event-gated by construction.
#[test]
fn delay_other_timing_is_event_gated() {
    let dsl = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::OnPlay,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(
        effects[0].timing,
        EffectTiming::DelayEffect,
        "timing must always be DelayEffect"
    );
    assert_eq!(
        effects[0].delay_trigger,
        Some(DelayTrigger::OnEvent(EffectTiming::OnPlay)),
        "a non-turn-scheduled timing must be event-gated, not silently          turned into a turn-scheduled auto-fire (16-16-1)"
    );
}

/// (c) The emitted Effect has a `process` closure.
#[test]
fn delay_emits_effect_with_process() {
    let dsl = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::EndOfYourTurn,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert!(
        effects[0].process.is_some(),
        "Delay effect must have a process closure"
    );
}

/// (d) `CompiledScope::Inherited` sets `inherited == true`; `FaceUp` does not.
#[test]
fn delay_inherited_scope_sets_inherited_flag() {
    let inherited = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::Inherited,
        CompiledTiming::EndOfYourTurn,
    )));
    let face_up = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::EndOfYourTurn,
    )));

    let inh_effects = inherited.effects(CardHandle(0));
    let fu_effects = face_up.effects(CardHandle(0));

    assert_eq!(inh_effects.len(), 1);
    assert_eq!(fu_effects.len(), 1);

    assert!(
        inh_effects[0].inherited,
        "scope: Inherited should set the inherited flag"
    );
    assert!(
        !fu_effects[0].inherited,
        "scope: FaceUp must NOT set the inherited flag"
    );
}

#[test]
fn delay_start_of_your_turn_maps_to_start_of_your_next_turn() {
    let dsl = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::StartOfYourTurn,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(
        effects[0].delay_trigger,
        Some(DelayTrigger::StartOfYourNextTurn)
    );
}

#[test]
fn delay_event_trigger_lowers_to_on_event_delay() {
    let mut card = fixture_delay_activatable(CompiledScope::FaceUp, CompiledTiming::OnSuspend);
    if let CompiledClause::Declarative(CompiledDeclarativeClause::Delay { active_when, .. }) =
        &mut card.effects[0]
    {
        *active_when = Some(CompiledPredicate {
            event_card_name_contains: Some("Arisa Kinosaki".into()),
            ..Default::default()
        });
    }

    let dsl = DslCardEffect::new(Arc::new(card));
    let effects = dsl.effects(CardHandle(0));

    assert_eq!(effects.len(), 1);
    assert_eq!(
        effects[0].delay_trigger,
        Some(DelayTrigger::OnEvent(EffectTiming::OnSuspend))
    );
    assert!(
        effects[0].condition.is_some(),
        "Delay active_when should lower to a runtime condition"
    );
    // Rules manual 16-16-2: "The processing from <Delay> is optional." The
    // event-gated Delay must queue as an OPTIONAL trigger with the outer
    // accept/decline activation prompt (declining leaves the Option parked).
    assert!(
        effects[0].optional,
        "event-gated <Delay> must be optional (16-16-2)"
    );
    assert!(
        effects[0].needs_outer_optional_prompt,
        "event-gated <Delay> must install the outer activation prompt"
    );
}

/// The standard `trigger: delayed` (`MainPhaseActivated`) `<Delay>` is
/// activated by an explicit player `[Main]` action — optionality is inherent
/// in the action, so the lowering must NOT add the outer prompt (it would
/// double-prompt after the player already chose to activate).
#[test]
fn delay_main_phase_activated_does_not_double_prompt() {
    let dsl = DslCardEffect::new(Arc::new(fixture_delay(
        CompiledScope::FaceUp,
        CompiledTiming::Delayed,
    )));
    let effects = dsl.effects(CardHandle(0));
    assert!(
        !effects[0].optional,
        "MainPhaseActivated Delay must not be marked optional"
    );
    assert!(
        !effects[0].needs_outer_optional_prompt,
        "MainPhaseActivated Delay must not install an outer prompt"
    );
}

#[test]
fn delay_attack_triggers_lower_to_on_event_delay() {
    for (compiled, expected) in [
        (CompiledTiming::OnAttack, EffectTiming::OnAttack),
        (CompiledTiming::OnAllyAttack, EffectTiming::OnAllyAttack),
        (
            CompiledTiming::OnOpponentAttack,
            EffectTiming::OnOpponentAttack,
        ),
    ] {
        let dsl = DslCardEffect::new(Arc::new(fixture_delay_activatable(
            CompiledScope::FaceUp,
            compiled,
        )));
        let effects = dsl.effects(CardHandle(0));

        assert_eq!(
            effects[0].delay_trigger,
            Some(DelayTrigger::OnEvent(expected)),
            "{compiled:?} must lower to an event-backed Delay trigger"
        );
        assert!(
            effects[0].optional && effects[0].needs_outer_optional_prompt,
            "{compiled:?}: event-gated <Delay> must be optional with an outer prompt (16-16-2)"
        );
    }
}

#[test]
fn place_self_as_delay_option_step_parses_and_compiles() {
    let yaml = r#"
card: TEST-PLACE
name: "Inherited Security Placement"
kind: option
color: [red]
cost: 0
traits: []
effects:
  - scope: inherited
    when: on_security
    process:
      - place_self_as_delay_option: {}
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");

    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    assert!(matches!(
        triggered.process.as_slice(),
        [CompiledStep::PlaceSelfAsDelayOption]
    ));
}

#[test]
fn place_self_as_delay_option_step_lowers_to_placement_helper() {
    let mut host_card = make_test_card("HOST", "HOST");
    host_card.colors = vec![CardColor::Red];
    let mut option = make_test_card("P-035", "P-035");
    option.card_kind = CardKind::Option;
    option.level = None;
    option.dp = None;
    option.colors = vec![CardColor::Red];

    let mut runner = DebugRunner::builder()
        .add_card(host_card)
        .add_card(option)
        .start();
    let host = runner.place_on_field(0, "HOST", Some(0));
    let source = runner.push_source(host, "P-035");

    let step = CompiledStep::PlaceSelfAsDelayOption;
    let mut bindings = Bindings::new();
    {
        let mut ctx = EffectContext::new_with_source_kind(
            &mut runner.game,
            source,
            Some(host),
            EffectSourceKind::Option,
            0,
        );
        run_steps(&[step], &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.player(0).battle_area[host.index as usize]
            .card_sources
            .len(),
        1
    );
    let placed = runner
        .game
        .player(0)
        .battle_area
        .last()
        .expect("placed option permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed { owner: 0, .. }
    ));
    assert_eq!(placed.top_card().card_id(&runner.game.card_data), "P-035");
}
