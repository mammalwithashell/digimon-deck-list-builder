//! G-DSL-IN-TEXT-CONTAINS — whole-card "[X] in its text" predicate leaf.
//!
//! Mirrors DCGO `CardSource.HasText` (`CardSource.cs`): a case-insensitive
//! substring scan over the card NAME + also-treated-as / DigiXros aliases +
//! TRAITS (the Type line) + all printed text (effect / inherited / security,
//! both dual faces).
//!
//! Distinct from `effect_text_contains`, which scans ONLY the top card's
//! effect / inherited / security text — it does NOT see traits, so it misses
//! the [Three Musketeers]-TRAIT cards (BT6-017 MagnaKidmon, BT6-065 Gundramon,
//! ST14-09 BeelStarmon) that carry the trait but not the literal string in
//! effect text. `in_text_contains` matches them via the trait scan.
//!
//! Driver family: BT21-098 Ragnarok Cannon ("[Vemmon] in its text").

use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::CardKind;

/// A Digimon with an explicit trait line + effect text, so we can prove the
/// two leaves diverge on the trait vs. text axis.
fn digimon(id: &str, name: &str, traits: &[&str], effect_text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c.effect_text = effect_text.to_string();
    c
}

/// Handle of the Nth hand card of player 0.
fn hand_card(runner: &DebugRunner, n: usize) -> CardHandle {
    runner.game.players[0].hand[n].handle()
}

// ---------------------------------------------------------------------------
// Parse / compile
// ---------------------------------------------------------------------------

#[test]
fn parse_and_compile_in_text_contains_leaf() {
    let yaml = r#"
card: TEST-IN-TEXT
name: In Text Contains
kind: option
color: [black]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_hand:
          of: you
          bind_as: pick
          prompt: Pick a Vemmon-text card
          filter: { in_text_contains: "Vemmon" }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");

    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::SelectHand { filter, .. } = &triggered.process[0] else {
        panic!("expected select_hand");
    };
    assert_eq!(filter.in_text_contains.as_deref(), Some("Vemmon"));
    // `in_text_contains` is a distinct leaf — it must NOT populate the
    // narrower `effect_text_contains` field.
    assert_eq!(filter.effect_text_contains, None);
}

// ---------------------------------------------------------------------------
// Engine eval — the surface DCGO HasText scans
// ---------------------------------------------------------------------------

fn setup() -> DebugRunner {
    DebugRunner::builder()
        // Carries the [Three Musketeers] TRAIT, no such string in effect text.
        // Mirrors BT6-017 / BT6-065 / ST14-09.
        .add_card(digimon(
            "TRAIT-ONLY",
            "MagnaKidmon",
            &["Royal Knight", "Three Musketeers"],
            "[On Play] Draw 1 card.",
        ))
        // The string only appears in the effect text.
        .add_card(digimon(
            "TEXT-ONLY",
            "Some Digimon",
            &["Dragon"],
            "[Your Turn] Cards with [Three Musketeers] in their text get +1000 DP.",
        ))
        // The string only appears in the name.
        .add_card(digimon(
            "NAME-ONLY",
            "Vemmon",
            &["Unknown"],
            "[On Play] Draw 1.",
        ))
        // Nothing matches "Vemmon"/"Three Musketeers".
        .add_card(digimon(
            "UNRELATED",
            "Agumon",
            &["Reptile"],
            "[On Play] Gain 1 memory.",
        ))
        .hand(0, &["TRAIT-ONLY", "TEXT-ONLY", "NAME-ONLY", "UNRELATED"])
        .build()
}

fn in_text(needle: &str) -> digimon_dsl::compiled::CompiledPredicate {
    digimon_dsl::compiled::CompiledPredicate {
        in_text_contains: Some(needle.to_string()),
        ..Default::default()
    }
}

fn effect_text(needle: &str) -> digimon_dsl::compiled::CompiledPredicate {
    digimon_dsl::compiled::CompiledPredicate {
        effect_text_contains: Some(needle.to_string()),
        ..Default::default()
    }
}

#[test]
fn in_text_contains_matches_trait_line_regression() {
    // The load-bearing case: a [Three Musketeers]-TRAIT card with no such
    // string in its effect text. `in_text_contains` MUST match (DCGO scans
    // `Type_ENG`); `effect_text_contains` must NOT (it only scans text).
    let runner = setup();
    let rctx = EffectReadContext::new(&runner.game, hand_card(&runner, 0), None, 0);
    let trait_only = hand_card(&runner, 0);

    assert!(
        eval_predicate_with_bindings(
            &in_text("Three Musketeers"),
            &rctx,
            PredicateSubject::Card(trait_only),
            None
        ),
        "in_text_contains must match the [Three Musketeers] TRAIT"
    );
    assert!(
        !eval_predicate_with_bindings(
            &effect_text("Three Musketeers"),
            &rctx,
            PredicateSubject::Card(trait_only),
            None
        ),
        "effect_text_contains must NOT match a trait-only card (proves the leaves diverge)"
    );
}

#[test]
fn in_text_contains_matches_name_and_effect_text() {
    let runner = setup();
    let rctx = EffectReadContext::new(&runner.game, hand_card(&runner, 0), None, 0);

    // Name-only match.
    assert!(eval_predicate_with_bindings(
        &in_text("Vemmon"),
        &rctx,
        PredicateSubject::Card(hand_card(&runner, 2)),
        None
    ));
    // Effect-text match (superset of effect_text_contains).
    assert!(eval_predicate_with_bindings(
        &in_text("Three Musketeers"),
        &rctx,
        PredicateSubject::Card(hand_card(&runner, 1)),
        None
    ));
}

#[test]
fn in_text_contains_is_case_insensitive() {
    let runner = setup();
    let rctx = EffectReadContext::new(&runner.game, hand_card(&runner, 0), None, 0);
    // "three musketeers" (lowercase) must still match the mixed-case trait.
    assert!(eval_predicate_with_bindings(
        &in_text("three musketeers"),
        &rctx,
        PredicateSubject::Card(hand_card(&runner, 0)),
        None
    ));
    // "VEMMON" (uppercase) must match the "Vemmon" name.
    assert!(eval_predicate_with_bindings(
        &in_text("VEMMON"),
        &rctx,
        PredicateSubject::Card(hand_card(&runner, 2)),
        None
    ));
}

#[test]
fn in_text_contains_does_not_over_match() {
    let runner = setup();
    let rctx = EffectReadContext::new(&runner.game, hand_card(&runner, 0), None, 0);
    // The unrelated Agumon matches neither needle.
    assert!(!eval_predicate_with_bindings(
        &in_text("Vemmon"),
        &rctx,
        PredicateSubject::Card(hand_card(&runner, 3)),
        None
    ));
    assert!(!eval_predicate_with_bindings(
        &in_text("Three Musketeers"),
        &rctx,
        PredicateSubject::Card(hand_card(&runner, 3)),
        None
    ));
}

#[test]
fn parse_and_compile_event_card_in_text_contains_leaf() {
    // The event-side variant (G-DSL-EVENT-CARD-IN-TEXT-CONTAINS) — cheap
    // superset of `event_card_text_contains` for observers on "when you play a
    // card with [X] in its text" (AD1-018 LordKnightmon).
    let yaml = r#"
card: TEST-EVENT-IN-TEXT
name: Event In Text
kind: digimon
color: [white]
level: 6
cost: 12
dp: 12000
effects:
  - when: on_any_digimon_played
    active_when: { event_card_in_text_contains: "Knightmon" }
    process:
      - gain_memory: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    assert_eq!(
        triggered
            .active_when
            .as_ref()
            .and_then(|p| p.event_card_in_text_contains.as_deref()),
        Some("Knightmon")
    );
}

#[test]
fn in_text_contains_scans_also_treated_as_and_digixros_aliases() {
    // Aliases the API drops but our CardData records. DCGO scans only
    // `CardName_ENG`; scanning our recorded aliases is a faithful superset.
    let mut aliased = digimon("ALIAS", "Alphamon", &["Royal Knight"], "[On Play] Draw 1.");
    aliased.also_treated_as = vec!["Omnimon".to_string()];
    aliased.digixros_aliases = vec!["Jesmon".to_string()];

    let runner = DebugRunner::builder()
        .add_card(aliased)
        .hand(0, &["ALIAS"])
        .build();
    let rctx = EffectReadContext::new(&runner.game, hand_card(&runner, 0), None, 0);
    let card = hand_card(&runner, 0);

    assert!(eval_predicate_with_bindings(
        &in_text("Omnimon"),
        &rctx,
        PredicateSubject::Card(card),
        None
    ));
    assert!(eval_predicate_with_bindings(
        &in_text("Jesmon"),
        &rctx,
        PredicateSubject::Card(card),
        None
    ));
}
