//! Predicate/formula leaves II — binding / event-scoped (store-champs).
//!
//! Per-leaf coverage: (a) parse-from-YAML + compile into the compiled IR,
//! (b) positive/negative runtime eval against a real `EffectReadContext`, and
//! (c) an inline-fixture behavioral proof for the cited driver shape where the
//! leaf reads trigger/deletion context.
//!
//! Leaves:
//!  1. binding_card_color               (BT1-087  G-DSL-BINDING-CARD-COLOR)
//!  2. binding_card_name_is             (BT21-087 G-DSL-BINDING-CARD-NAME-EQUALS)
//!  3. event_target_trait_contains      (BT11-089 G-DSL-EVENT-TARGET-TRAIT-CONTAINS)
//!  4. event_target source-count leaves (EX1-066  G-DSL-EVENT-TARGET-SOURCE-COUNT)
//!  5. distinct_named_count_gte         (BT21-040 G-DSL-DISTINCT-NAMED-PERMANENT-COUNT)
//!  6. play_cost_eq_binding             (BT19-099 G-DSL-COST-RELATIVE-TO-EVENT-SUBJECT)
//!  7. binding-exclusion for mass ops   (EX11-046 — already expressible: not_in_binding)
//!  8. level_lte_binding                (BT8-107  level binding widening)

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledComparatorOp, CompiledPlayerRef, CompiledPredicate,
    CompiledStep,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::{Permanent, PermanentHandle};
use digimon_engine::trigger_context::{
    DeletedObjectSnapshot, EventCause, TriggerContext,
};

fn parse_compile(yaml: &str) -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    digimon_dsl::compile::compile(&spec).expect("compile yaml")
}

// ─── Leaf 1: binding_card_color (BT1-087) ────────────────────────────────────

#[test]
fn binding_card_color_compiles_from_yaml() {
    let yaml = r#"
card: T-BCC
name: Binding Card Color
kind: option
color: [yellow]
cost: 0
effects:
  - when: main_from_hand
    process:
      - if:
          condition: { binding_card_color: { binding: revealed, color_is: yellow } }
          then:
            - gain_memory: 1
"#;
    let compiled = parse_compile(yaml);
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::If { condition, .. } = &triggered.process[0] else {
        panic!("expected if step");
    };
    let bc = condition
        .binding_card_color
        .as_ref()
        .expect("binding_card_color must compile");
    assert_eq!(bc.binding, "revealed");
    assert_eq!(bc.color_is, digimon_dsl::compiled::CompiledColor::Yellow);
}

#[test]
fn binding_card_color_matches_when_bound_card_shares_color() {
    // BT1-087 driver: "add 1 revealed security card to hand; if THAT card is
    // yellow, trigger <Recovery +1>". The bound card is a security-added card.
    let src = make_test_card("PRED-SRC", "Pred Src");
    let mut yellow = make_test_card("YELLOW-SEC", "Yellow Sec");
    yellow.colors = vec![CardColor::Yellow];
    let mut blue = make_test_card("BLUE-SEC", "Blue Sec");
    blue.colors = vec![CardColor::Blue];
    let runner = DebugRunner::builder()
        .add_card(src)
        .add_card(yellow)
        .add_card(blue)
        .hand(0, &["PRED-SRC", "YELLOW-SEC", "BLUE-SEC"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();
    let yellow_card = runner.game.players[0].hand[1].handle();
    let blue_card = runner.game.players[0].hand[2].handle();
    let rctx = EffectReadContext::new(&runner.game, source_card, None, 0);

    let pred = CompiledPredicate {
        binding_card_color: Some(digimon_dsl::compiled::CompiledBindingCardColorPredicate {
            binding: "revealed".to_string(),
            color_is: digimon_dsl::compiled::CompiledColor::Yellow,
        }),
        ..Default::default()
    };

    let mut yellow_binding = Bindings::new();
    yellow_binding.insert_card("revealed", yellow_card);
    assert!(
        eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, Some(&yellow_binding)),
        "yellow bound card satisfies binding_card_color: yellow"
    );

    let mut blue_binding = Bindings::new();
    blue_binding.insert_card("revealed", blue_card);
    assert!(
        !eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, Some(&blue_binding)),
        "blue bound card does not satisfy binding_card_color: yellow"
    );

    // Fail closed when the binding is unset.
    assert!(
        !eval_predicate_with_bindings(
            &pred,
            &rctx,
            PredicateSubject::None,
            Some(&Bindings::new())
        ),
        "missing binding fails closed"
    );
}

// ─── Leaf 2: binding_card_name_is (BT21-087) ─────────────────────────────────

#[test]
fn binding_card_name_is_compiles_from_yaml() {
    let yaml = r#"
card: T-BCN
name: Binding Card Name
kind: option
color: [purple]
cost: 0
effects:
  - when: main_from_hand
    process:
      - if:
          condition: { binding_card_name_is: { binding: revealed, name_is: Vemmon } }
          then:
            - gain_memory: 1
"#;
    let compiled = parse_compile(yaml);
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::If { condition, .. } = &triggered.process[0] else {
        panic!("expected if step");
    };
    assert_eq!(
        condition.binding_card_name_is,
        Some(("revealed".to_string(), "Vemmon".to_string()))
    );
}

#[test]
fn binding_card_name_is_matches_printed_and_alias_names() {
    let src = make_test_card("PRED-SRC", "Pred Src");
    let vemmon = make_test_card("VEMMON", "Vemmon");
    let mut aliased = make_test_card("ALIAS", "DarkKnightmon");
    aliased.also_treated_as = vec!["Vemmon".to_string()];
    let other = make_test_card("OTHER", "Not Vemmon");
    let runner = DebugRunner::builder()
        .add_card(src)
        .add_card(vemmon)
        .add_card(aliased)
        .add_card(other)
        .hand(0, &["PRED-SRC", "VEMMON", "ALIAS", "OTHER"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();
    let vemmon_card = runner.game.players[0].hand[1].handle();
    let alias_card = runner.game.players[0].hand[2].handle();
    let other_card = runner.game.players[0].hand[3].handle();
    let rctx = EffectReadContext::new(&runner.game, source_card, None, 0);

    let pred = CompiledPredicate {
        binding_card_name_is: Some(("revealed".to_string(), "Vemmon".to_string())),
        ..Default::default()
    };

    for (card, expect, label) in [
        (vemmon_card, true, "printed name Vemmon matches"),
        (alias_card, true, "also_treated_as Vemmon matches"),
        (other_card, false, "unrelated name does not match"),
    ] {
        let mut b = Bindings::new();
        b.insert_card("revealed", card);
        assert_eq!(
            eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, Some(&b)),
            expect,
            "{label}"
        );
    }
}

// ─── Leaf 3: event_target_trait_contains (BT11-089) ──────────────────────────

#[test]
fn event_target_trait_contains_compiles_from_yaml() {
    let yaml = r#"
card: T-ETTC
name: Event Target Trait Contains
kind: tamer
color: [red]
cost: 3
effects:
  - when: on_ally_played
    condition: { event_target_trait_contains: Beast }
    process:
      - gain_memory: 1
"#;
    let compiled = parse_compile(yaml);
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let condition = triggered.condition.as_ref().expect("condition");
    assert_eq!(
        condition.event_target_trait_contains.as_deref(),
        Some("Beast")
    );
}

#[test]
fn event_target_trait_contains_tolerates_plural_and_compound_traits() {
    // BT11-089 Q&A: "[Beast] ... regardless of other words or pluralizations".
    // `Sea Beast`, `Beasts`, `Beast` all match `Beast`; `Bird` does not.
    let src = make_test_card("PRED-SRC", "Pred Src");
    let mut sea_beast = make_test_card("SEA-BEAST", "Sea Beast Digimon");
    sea_beast.traits = vec!["Sea Beast".to_string()];
    let mut beasts = make_test_card("BEASTS", "Beasts Digimon");
    beasts.traits = vec!["Beasts".to_string()];
    let mut bird = make_test_card("BIRD", "Bird Digimon");
    bird.traits = vec!["Bird".to_string()];
    let runner = DebugRunner::builder()
        .add_card(src)
        .add_card(sea_beast)
        .add_card(beasts)
        .add_card(bird)
        .hand(0, &["PRED-SRC", "SEA-BEAST", "BEASTS", "BIRD"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();

    let pred = CompiledPredicate {
        event_target_trait_contains: Some("Beast".to_string()),
        ..Default::default()
    };

    for (idx, expect, label) in [
        (1u16, true, "Sea Beast matches Beast substring"),
        (2, true, "Beasts matches Beast substring"),
        (3, false, "Bird does not match Beast"),
    ] {
        let event_card = runner.game.players[0].hand[idx as usize].handle();
        let mut game = runner.game.clone();
        game.current_trigger_context = Some(TriggerContext {
            event_card: Some(event_card),
            event_permanent: None,
            target_card: Some(event_card),
            ..Default::default()
        });
        let rctx = EffectReadContext::new(&game, source_card, None, 0);
        assert_eq!(
            eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, None),
            expect,
            "{label}"
        );
    }
}

#[test]
fn event_target_trait_contains_reads_deletion_snapshot_traits() {
    // Deletion split: the event target is gone from the field, so the leaf must
    // read the rule-25 snapshot's `traits`.
    let src = make_test_card("PRED-SRC", "Pred Src");
    let victim = make_test_card("VICTIM", "Victim");
    let runner = DebugRunner::builder()
        .add_card(src)
        .add_card(victim)
        .hand(0, &["PRED-SRC"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();
    let victim_handle = runner.game.players[0].hand[0].handle();

    let pred = CompiledPredicate {
        event_target_trait_contains: Some("Sovereign".to_string()),
        ..Default::default()
    };

    let mut game = runner.game.clone();
    game.current_trigger_context = Some(TriggerContext {
        deleted_object: Some(DeletedObjectSnapshot {
            former_controller: 1,
            top_card: victim_handle,
            card_kind: CardKind::Digimon,
            traits: vec!["Holy Beast Sovereign".to_string()],
            level: Some(6),
            dp: Some(12000),
            cause: EventCause::EffectDeletion,
            dp_just_before: Some(12000),
            level_just_before: Some(6),
            cost_just_before: Some(12),
            names_just_before: vec!["Victim".to_string()],
            traits_just_before: vec!["Holy Beast Sovereign".to_string()],
            source_count_just_before: 2,
            digisources_just_before: Vec::new(),
            is_token: false,
        }),
        ..Default::default()
    });
    let rctx = EffectReadContext::new(&game, source_card, None, 0);
    assert!(
        eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, None),
        "snapshot trait 'Holy Beast Sovereign' matches Sovereign substring"
    );
}

// ─── Leaf 4: event_target source-count (EX1-066) ─────────────────────────────

#[test]
fn event_target_source_count_leaves_compile_from_yaml() {
    let yaml = r#"
card: T-ETSC
name: Event Target Source Count
kind: tamer
color: [green]
cost: 3
effects:
  - when: on_any_deletion
    condition:
      all_of:
        - event_target_has_digivolution_cards: true
        - event_target_stack_size_gte: 2
    process:
      - gain_memory: 1
"#;
    let compiled = parse_compile(yaml);
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let condition = triggered.condition.as_ref().expect("condition");
    assert!(condition
        .all_of
        .iter()
        .any(|p| p.event_target_has_digivolution_cards == Some(true)));
    assert!(condition
        .all_of
        .iter()
        .any(|p| p.event_target_stack_size_gte == Some(2)));
}

#[test]
fn event_target_has_digivolution_cards_reads_deletion_snapshot_source_count() {
    // EX1-066: "When one of your ... Digimon WITH A DIGIVOLUTION CARD is
    // deleted". The deletion snapshot's `source_count_just_before` is sources
    // below the top; the leaf tests stack size (top + sources) >= 2.
    let src = make_test_card("PRED-SRC", "Pred Src");
    let victim = make_test_card("VICTIM", "Victim");
    let runner = DebugRunner::builder()
        .add_card(src)
        .add_card(victim)
        .hand(0, &["PRED-SRC"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();
    let victim_handle = runner.game.players[0].hand[0].handle();

    let has_pred = CompiledPredicate {
        event_target_has_digivolution_cards: Some(true),
        ..Default::default()
    };
    let stack_gte_3 = CompiledPredicate {
        event_target_stack_size_gte: Some(3),
        ..Default::default()
    };

    let make_snapshot = |source_count: usize| DeletedObjectSnapshot {
        former_controller: 0,
        top_card: victim_handle,
        card_kind: CardKind::Digimon,
        traits: Vec::new(),
        level: Some(5),
        dp: Some(6000),
        cause: EventCause::EffectDeletion,
        dp_just_before: Some(6000),
        level_just_before: Some(5),
        cost_just_before: Some(6),
        names_just_before: vec!["Victim".to_string()],
        traits_just_before: Vec::new(),
        source_count_just_before: source_count,
        digisources_just_before: Vec::new(),
        is_token: false,
    };

    // 1 source below top → stack size 2 → has a digivolution card.
    let mut game = runner.game.clone();
    game.current_trigger_context = Some(TriggerContext {
        deleted_object: Some(make_snapshot(1)),
        ..Default::default()
    });
    let rctx = EffectReadContext::new(&game, source_card, None, 0);
    assert!(
        eval_predicate_with_bindings(&has_pred, &rctx, PredicateSubject::None, None),
        "1 source below top → has a digivolution card"
    );
    assert!(
        !eval_predicate_with_bindings(&stack_gte_3, &rctx, PredicateSubject::None, None),
        "stack size 2 does not satisfy stack_size_gte: 3"
    );

    // 0 sources → lone top card → no digivolution card.
    let mut game0 = runner.game.clone();
    game0.current_trigger_context = Some(TriggerContext {
        deleted_object: Some(make_snapshot(0)),
        ..Default::default()
    });
    let rctx0 = EffectReadContext::new(&game0, source_card, None, 0);
    assert!(
        !eval_predicate_with_bindings(&has_pred, &rctx0, PredicateSubject::None, None),
        "lone top card → no digivolution card"
    );

    // 2 sources below top → stack size 3 → stack_size_gte: 3 satisfied.
    let mut game2 = runner.game.clone();
    game2.current_trigger_context = Some(TriggerContext {
        deleted_object: Some(make_snapshot(2)),
        ..Default::default()
    });
    let rctx2 = EffectReadContext::new(&game2, source_card, None, 0);
    assert!(
        eval_predicate_with_bindings(&stack_gte_3, &rctx2, PredicateSubject::None, None),
        "stack size 3 satisfies stack_size_gte: 3"
    );
}

#[test]
fn event_target_has_digivolution_cards_reads_live_permanent_stack() {
    // Live (non-deletion) event target: read the permanent's current
    // `card_sources` length.
    let src = make_test_card("PRED-SRC", "Pred Src");
    let mut base = make_test_card("BASE", "Base");
    base.card_kind = CardKind::Digimon;
    let mut top = make_test_card("TOP", "Top");
    top.card_kind = CardKind::Digimon;
    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(base)
        .add_card(top)
        .hand(0, &["PRED-SRC"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();

    // Build a 2-high stack (base + top).
    let stacked = {
        let g = runner.game_mut();
        let turn = g.turn_count;
        let base_idx = g.card_data.iter().position(|c| c.card_id == "BASE").unwrap();
        let top_idx = g.card_data.iter().position(|c| c.card_id == "TOP").unwrap();
        let base_src = CardSource::new(base_idx, 0, g.next_card_index());
        let top_src = CardSource::new(top_idx, 0, g.next_card_index());
        let mut perm = Permanent::new(base_src, turn);
        perm.card_sources.push(top_src);
        g.players[0].battle_area.push(perm);
        PermanentHandle {
            player: 0,
            index: (g.players[0].battle_area.len() - 1) as u8,
        }
    };

    let has_pred = CompiledPredicate {
        event_target_has_digivolution_cards: Some(true),
        ..Default::default()
    };
    let mut game = runner.game.clone();
    game.current_trigger_context = Some(TriggerContext {
        event_permanent: Some(stacked),
        ..Default::default()
    });
    let rctx = EffectReadContext::new(&game, source_card, None, 0);
    assert!(
        eval_predicate_with_bindings(&has_pred, &rctx, PredicateSubject::None, None),
        "live 2-high stack → has a digivolution card"
    );
}

// ─── Leaf 5: distinct_named_count_gte (BT21-040) ─────────────────────────────

#[test]
fn distinct_named_count_gte_compiles_from_yaml() {
    let yaml = r#"
card: T-DNC
name: Distinct Named Count
kind: digimon
level: 5
color: [red]
cost: 6
dp: 7000
effects:
  - kind: aura
    scope: face_up
    active_when:
      distinct_named_count_gte:
        of: you
        n: 3
        filter: { kind: tamer, trait_has: Hero }
    target: { is_source: true }
    dp_modifier: 1000
"#;
    let compiled = parse_compile(yaml);
    let CompiledClause::Declarative(clause) = &compiled.effects[0] else {
        panic!("expected declarative clause");
    };
    let digimon_dsl::compiled::CompiledDeclarativeClause::Aura { active_when, .. } = clause else {
        panic!("expected aura clause");
    };
    let active = active_when.as_ref().expect("active_when compiles");
    let d = active
        .distinct_named_count_gte
        .as_ref()
        .expect("distinct_named_count_gte compiles");
    assert_eq!(d.of, CompiledPlayerRef::You);
    assert_eq!(d.n, 3);
    assert_eq!(d.filter.trait_has.as_deref(), Some("Hero"));
    assert_eq!(d.filter.kind, Some(CompiledCardKind::Tamer));
}

#[test]
fn distinct_named_count_gte_counts_distinct_filtered_names() {
    // BT21-040: "3 or more [Hero] trait Tamers with DIFFERENT names".
    let src = make_test_card("PRED-SRC", "Pred Src");
    let mut hero_a = make_test_card("HERO-A", "Marcus");
    hero_a.card_kind = CardKind::Tamer;
    hero_a.traits = vec!["Hero".to_string()];
    let mut hero_b = make_test_card("HERO-B", "Thomas");
    hero_b.card_kind = CardKind::Tamer;
    hero_b.traits = vec!["Hero".to_string()];
    let mut hero_c = make_test_card("HERO-C", "Keenan");
    hero_c.card_kind = CardKind::Tamer;
    hero_c.traits = vec!["Hero".to_string()];
    // Non-Hero tamer must not count.
    let mut plain = make_test_card("PLAIN-T", "Kudamon");
    plain.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(hero_a)
        .add_card(hero_b)
        .add_card(hero_c)
        .add_card(plain)
        .hand(0, &["PRED-SRC"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();

    let pred = CompiledPredicate {
        distinct_named_count_gte: Some(digimon_dsl::compiled::CompiledDistinctNamedCount {
            of: CompiledPlayerRef::You,
            filter: Box::new(CompiledPredicate {
                kind: Some(CompiledCardKind::Tamer),
                trait_has: Some("Hero".to_string()),
                ..Default::default()
            }),
            n: 3,
        }),
        ..Default::default()
    };

    // 2 distinct Hero Tamers + 1 non-Hero → fails (< 3 distinct Hero names).
    runner.place_on_field(0, "HERO-A", None);
    runner.place_on_field(0, "HERO-B", None);
    runner.place_on_field(0, "PLAIN-T", None);
    let rctx = EffectReadContext::new(&runner.game, source_card, None, 0);
    assert!(
        !eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, None),
        "2 distinct Hero Tamers is below the threshold of 3"
    );

    // Add the 3rd distinct Hero Tamer → passes.
    runner.place_on_field(0, "HERO-C", None);
    let rctx = EffectReadContext::new(&runner.game, source_card, None, 0);
    assert!(
        eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, None),
        "3 distinct Hero Tamers satisfies the threshold"
    );
}

#[test]
fn distinct_named_count_gte_dedupes_same_named_permanents() {
    // Two Hero Tamers with the SAME name count as one distinct name.
    let src = make_test_card("PRED-SRC", "Pred Src");
    let mut hero = make_test_card("HERO-DUP", "Marcus");
    hero.card_kind = CardKind::Tamer;
    hero.traits = vec!["Hero".to_string()];

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(hero)
        .hand(0, &["PRED-SRC"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();
    runner.place_on_field(0, "HERO-DUP", None);
    runner.place_on_field(0, "HERO-DUP", None);

    let pred = CompiledPredicate {
        distinct_named_count_gte: Some(digimon_dsl::compiled::CompiledDistinctNamedCount {
            of: CompiledPlayerRef::You,
            filter: Box::new(CompiledPredicate {
                kind: Some(CompiledCardKind::Tamer),
                trait_has: Some("Hero".to_string()),
                ..Default::default()
            }),
            n: 2,
        }),
        ..Default::default()
    };
    let rctx = EffectReadContext::new(&runner.game, source_card, None, 0);
    assert!(
        !eval_predicate_with_bindings(&pred, &rctx, PredicateSubject::None, None),
        "two same-named Hero Tamers are only 1 distinct name"
    );
}

// ─── Leaf 6: play_cost_eq_binding (BT19-099) ─────────────────────────────────

#[test]
fn play_cost_eq_binding_compiles_from_yaml() {
    let yaml = r#"
card: T-PCB
name: Play Cost Binding
kind: option
color: [purple]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_hand:
          of: you
          bind_as: pick
          prompt: Pick a costlier Digimon
          filter:
            kind: digimon
            trait_has: Wicked God
            play_cost_eq_binding: { binding: leaving, offset: 1, op: eq }
"#;
    let compiled = parse_compile(yaml);
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::SelectHand { filter, .. } = &triggered.process[0] else {
        panic!("expected select_hand");
    };
    let spec = filter
        .play_cost_eq_binding
        .as_ref()
        .expect("play_cost_eq_binding must compile");
    assert_eq!(spec.binding, "leaving");
    assert_eq!(spec.offset, 1);
    assert_eq!(spec.op, CompiledComparatorOp::Eq);
}

#[test]
fn play_cost_eq_binding_compares_candidate_to_bound_permanent_cost_plus_offset() {
    // BT19-099: "play 1 [Wicked God] Digimon with a play cost 1 HIGHER than
    // that [leaving] Digimon". Bind a play-cost-6 permanent; the candidate must
    // have play cost 7.
    let src = make_test_card("PRED-SRC", "Pred Src");
    let mut leaving = make_test_card("LEAVING", "Leaving");
    leaving.card_kind = CardKind::Digimon;
    leaving.play_cost = 6;
    let mut cost7 = make_test_card("COST7", "Cost 7");
    cost7.card_kind = CardKind::Digimon;
    cost7.play_cost = 7;
    let mut cost6 = make_test_card("COST6", "Cost 6");
    cost6.card_kind = CardKind::Digimon;
    cost6.play_cost = 6;

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(leaving)
        .add_card(cost7)
        .add_card(cost6)
        .hand(0, &["PRED-SRC", "COST7", "COST6"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();
    let cost7_card = runner.game.players[0].hand[1].handle();
    let cost6_card = runner.game.players[0].hand[2].handle();
    let leaving_perm = runner.place_on_field(0, "LEAVING", None);

    let pred = CompiledPredicate {
        play_cost_eq_binding: Some(digimon_dsl::compiled::CompiledPlayCostBindingPredicate {
            binding: "leaving".to_string(),
            offset: 1,
            op: CompiledComparatorOp::Eq,
        }),
        ..Default::default()
    };

    let mut bindings = Bindings::new();
    bindings.insert_permanent("leaving", leaving_perm);
    let rctx = EffectReadContext::new(&runner.game, source_card, None, 0);

    assert!(
        eval_predicate_with_bindings(
            &pred,
            &rctx,
            PredicateSubject::Card(cost7_card),
            Some(&bindings)
        ),
        "cost-7 candidate == leaving(6) + 1"
    );
    assert!(
        !eval_predicate_with_bindings(
            &pred,
            &rctx,
            PredicateSubject::Card(cost6_card),
            Some(&bindings)
        ),
        "cost-6 candidate != leaving(6) + 1"
    );
    // Fail closed when the reference binding is unset.
    assert!(
        !eval_predicate_with_bindings(
            &pred,
            &rctx,
            PredicateSubject::Card(cost7_card),
            Some(&Bindings::new())
        ),
        "missing reference binding fails closed"
    );
}

#[test]
fn play_cost_eq_binding_reads_event_target_deletion_cost() {
    // The `event_target` reference resolves from the deletion snapshot's
    // pre-removal cost (BT19-099 fires on a leave-the-field <Delay>).
    let src = make_test_card("PRED-SRC", "Pred Src");
    let victim = make_test_card("VICTIM", "Victim");
    let mut cost7 = make_test_card("COST7", "Cost 7");
    cost7.card_kind = CardKind::Digimon;
    cost7.play_cost = 7;
    let runner = DebugRunner::builder()
        .add_card(src)
        .add_card(victim)
        .add_card(cost7)
        .hand(0, &["PRED-SRC", "COST7"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();
    let cost7_card = runner.game.players[0].hand[1].handle();
    let victim_handle = runner.game.players[0].hand[0].handle();

    let pred = CompiledPredicate {
        play_cost_eq_binding: Some(digimon_dsl::compiled::CompiledPlayCostBindingPredicate {
            binding: "event_target".to_string(),
            offset: 1,
            op: CompiledComparatorOp::Eq,
        }),
        ..Default::default()
    };

    let mut game = runner.game.clone();
    game.current_trigger_context = Some(TriggerContext {
        deleted_object: Some(DeletedObjectSnapshot {
            former_controller: 0,
            top_card: victim_handle,
            card_kind: CardKind::Digimon,
            traits: Vec::new(),
            level: Some(6),
            dp: Some(12000),
            cause: EventCause::Rule,
            dp_just_before: Some(12000),
            level_just_before: Some(6),
            cost_just_before: Some(6),
            names_just_before: vec!["Victim".to_string()],
            traits_just_before: Vec::new(),
            source_count_just_before: 0,
            digisources_just_before: Vec::new(),
            is_token: false,
        }),
        ..Default::default()
    });
    let rctx = EffectReadContext::new(&game, source_card, None, 0);
    assert!(
        eval_predicate_with_bindings(
            &pred,
            &rctx,
            PredicateSubject::Card(cost7_card),
            Some(&Bindings::new())
        ),
        "cost-7 candidate == event_target deletion cost(6) + 1"
    );
}

// ─── Leaf 7: binding-exclusion for mass ops (EX11-046) ───────────────────────
//
// EX11-046: "Choose 1 of your opponent's highest play cost Digimon and delete
// all of their OTHER Digimon." This is already expressible today with a
// `select ... selector: highest_play_cost` bound as `kept`, then a
// `for_each { over: { ..., not_in_binding: kept } }`. The tie case (two Digimon
// tie for highest cost — the chosen one is kept, the other tied one is still
// deleted) is handled because `not_in_binding` on a single-permanent binding
// excludes ONLY the chosen permanent. These tests pin that guarantee via the
// `scan` path used by `for_each`.

#[test]
fn not_in_binding_excludes_only_the_chosen_binding_from_a_field_scan() {
    use digimon_engine::dsl_cards::step::permanent_scan::scan;
    use digimon_engine::effect_context::EffectContext;

    let src = make_test_card("PRED-SRC", "Pred Src");
    let mut a = make_test_card("OPP-A", "Opp A");
    a.card_kind = CardKind::Digimon;
    a.play_cost = 10;
    let mut b = make_test_card("OPP-B", "Opp B");
    b.card_kind = CardKind::Digimon;
    b.play_cost = 10; // ties with A for highest cost
    let mut c = make_test_card("OPP-C", "Opp C");
    c.card_kind = CardKind::Digimon;
    c.play_cost = 3;

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(a)
        .add_card(b)
        .add_card(c)
        .hand(0, &["PRED-SRC"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();
    let kept = runner.place_on_field(1, "OPP-A", None);
    let tied = runner.place_on_field(1, "OPP-B", None);
    let cheap = runner.place_on_field(1, "OPP-C", None);

    let over = CompiledPredicate {
        owner: Some(CompiledPlayerRef::Opponent),
        kind: Some(CompiledCardKind::Digimon),
        not_in_binding: Some("kept".to_string()),
        ..Default::default()
    };

    let mut bindings = Bindings::new();
    bindings.insert_permanent("kept", kept);
    let ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
    let matches = scan(&ctx, &over, Some(&bindings));

    // The chosen (kept) permanent is excluded; the TIED one and the cheap one
    // remain — proving the tie case deletes the other tied Digimon.
    assert!(!matches.contains(&kept), "chosen binding is excluded");
    assert!(
        matches.contains(&tied),
        "the OTHER Digimon that tied for highest cost is still targeted"
    );
    assert!(matches.contains(&cheap), "the cheap Digimon is targeted");
    assert_eq!(matches.len(), 2, "exactly the 2 non-kept opponent Digimon");
}

// ─── Leaf 8: level_lte_binding (BT8-107) ─────────────────────────────────────

#[test]
fn level_lte_binding_compiles_from_yaml() {
    let yaml = r#"
card: T-LLB
name: Level Lte Binding
kind: option
color: [black]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_opponent_permanent:
          bind_as: pick
          prompt: Pick a low-level Digimon
          filter:
            kind: digimon
            is_unsuspended: true
            level_lte_binding: deleted_level
"#;
    let compiled = parse_compile(yaml);
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::SelectOpponentPermanent { filter, .. } = &triggered.process[0] else {
        panic!("expected select_opponent_permanent");
    };
    assert_eq!(filter.level_lte_binding.as_deref(), Some("deleted_level"));
}

#[test]
fn level_lte_binding_filters_by_bound_literal_level() {
    // BT8-107: "delete 1 opponent Digimon with a level <= the deleted Digimon's
    // level". Bind the deleted level (5) as a literal; level-5 matches,
    // level-6 does not.
    let src = make_test_card("PRED-SRC", "Pred Src");
    let mut lv5 = make_test_card("LV5", "Level 5");
    lv5.card_kind = CardKind::Digimon;
    lv5.level = Some(5);
    let mut lv6 = make_test_card("LV6", "Level 6");
    lv6.card_kind = CardKind::Digimon;
    lv6.level = Some(6);

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(lv5)
        .add_card(lv6)
        .hand(0, &["PRED-SRC"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();
    let lv5_perm = runner.place_on_field(1, "LV5", None);
    let lv6_perm = runner.place_on_field(1, "LV6", None);

    let pred = CompiledPredicate {
        level_lte_binding: Some("deleted_level".to_string()),
        ..Default::default()
    };
    let mut bindings = Bindings::new();
    bindings.insert_literal("deleted_level", 5);
    let rctx = EffectReadContext::new(&runner.game, source_card, None, 0);

    assert!(
        eval_predicate_with_bindings(
            &pred,
            &rctx,
            PredicateSubject::Permanent(lv5_perm),
            Some(&bindings)
        ),
        "level-5 permanent <= bound level 5"
    );
    assert!(
        !eval_predicate_with_bindings(
            &pred,
            &rctx,
            PredicateSubject::Permanent(lv6_perm),
            Some(&bindings)
        ),
        "level-6 permanent > bound level 5"
    );
    assert!(
        !eval_predicate_with_bindings(
            &pred,
            &rctx,
            PredicateSubject::Permanent(lv5_perm),
            Some(&Bindings::new())
        ),
        "missing binding fails closed"
    );
}
