//! Tamer-colour count predicates. Card-text idioms: "your Tamers have 3 or
//! more total colors" (ST20-10 / ST21-10 warp, BT21-061 / BT21-078 / ST21-04
//! "for every 2 of your Tamers' colors" gates) and "your Tamers with the
//! [ADVENTURE] trait have 3 or more total colors" (ST20-03 / ST21-08).
//!
//! Leaves:
//!  1. distinct_tamer_colors_gte: n        — every own battle-area Tamer
//!  2. distinct_colors_gte: {of, filter, n} — own battle-area permanents
//!     matching `filter`, counted by rules colours (gained colours included)

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDistinctColorsCount, CompiledPlayerRef,
    CompiledPredicate, CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};
use digimon_engine::CardData;

fn parse_compile(yaml: &str) -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    digimon_dsl::compile::compile(&spec).expect("compile yaml")
}

fn tamer(id: &str, colors: &[CardColor], traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = colors.to_vec();
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn digimon(id: &str, colors: &[CardColor]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = colors.to_vec();
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("PRED-SRC", "Pred Src"))
        .add_card(tamer("T-RB", &[CardColor::Red, CardColor::Blue], &["ADVENTURE"]))
        .add_card(tamer("T-R", &[CardColor::Red], &["ADVENTURE"]))
        .add_card(tamer("T-B", &[CardColor::Blue], &["ADVENTURE"]))
        .add_card(tamer("T-Y-PLAIN", &[CardColor::Yellow], &[]))
        .add_card(tamer("T-G", &[CardColor::Green], &["ADVENTURE"]))
        .add_card(digimon("D-P", &[CardColor::Purple]))
        .hand(0, &["PRED-SRC"])
        .build()
}

fn holds(runner: &DebugRunner, pred: &CompiledPredicate) -> bool {
    let source_card = runner.game.players[0].hand[0].handle();
    let rctx = EffectReadContext::new(&runner.game, source_card, None, 0);
    eval_predicate_with_bindings(pred, &rctx, PredicateSubject::None, None)
}

fn all_tamers(n: u8) -> CompiledPredicate {
    CompiledPredicate { distinct_tamer_colors_gte: Some(n), ..Default::default() }
}

fn adventure_tamers(n: u8) -> CompiledPredicate {
    CompiledPredicate {
        distinct_colors_gte: Some(CompiledDistinctColorsCount {
            of: CompiledPlayerRef::You,
            filter: Box::new(CompiledPredicate {
                all_of: vec![
                    CompiledPredicate { kind: Some(CompiledCardKind::Tamer), ..Default::default() },
                    CompiledPredicate {
                        trait_has: Some("ADVENTURE".to_string()),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }),
            n,
        }),
        ..Default::default()
    }
}

// ─── Leaf 1: distinct_tamer_colors_gte ───────────────────────────────────────

#[test]
fn distinct_tamer_colors_gte_compiles_from_yaml() {
    let compiled = parse_compile(
        r#"
card: T-DTC
name: Distinct Tamer Colors
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
effects:
  - when: on_play
    process:
      - if:
          condition: { distinct_tamer_colors_gte: 3 }
          then:
            - gain_memory: 1
"#,
    );
    let CompiledClause::Triggered(t) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::If { condition, .. } = &t.process[0] else {
        panic!("expected if step");
    };
    assert_eq!(condition.distinct_tamer_colors_gte, Some(3));
}

#[test]
fn distinct_tamer_colors_gte_counts_distinct_colours_of_own_tamers_only() {
    let mut r = runner();
    assert!(!holds(&r, &all_tamers(1)), "no Tamers → 0 colours");

    // A multicolour Tamer contributes each of its colours.
    r.place_on_field(0, "T-RB", None);
    assert!(holds(&r, &all_tamers(2)));
    assert!(!holds(&r, &all_tamers(3)));

    // A duplicate colour adds nothing; a Digimon's colour never counts.
    r.place_on_field(0, "T-R", None);
    r.place_on_field(0, "D-P", None);
    assert!(!holds(&r, &all_tamers(3)), "Red twice + a Purple Digimon is still 2 Tamer colours");

    // The opponent's Tamers never count.
    r.place_on_field(1, "T-G", None);
    assert!(!holds(&r, &all_tamers(3)), "opponent's Green Tamer is not ours");

    // Every own Tamer counts, whatever its traits.
    r.place_on_field(0, "T-Y-PLAIN", None);
    assert!(holds(&r, &all_tamers(3)), "Red + Blue + Yellow");
}

// ─── Leaf 2: distinct_colors_gte ─────────────────────────────────────────────

#[test]
fn distinct_colors_gte_compiles_from_yaml() {
    let compiled = parse_compile(
        r#"
card: T-DCG
name: Distinct Filtered Colors
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
effects:
  - when: on_play
    process:
      - if:
          condition:
            distinct_colors_gte:
              of: you
              filter:
                all_of:
                  - kind: tamer
                  - trait_has: ADVENTURE
              n: 3
          then:
            - gain_memory: 1
"#,
    );
    let CompiledClause::Triggered(t) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::If { condition, .. } = &t.process[0] else {
        panic!("expected if step");
    };
    let d = condition.distinct_colors_gte.as_ref().expect("distinct_colors_gte compiles");
    assert_eq!(d.of, CompiledPlayerRef::You);
    assert_eq!(d.n, 3);
    assert_eq!(d.filter.all_of.len(), 2);
}

#[test]
fn distinct_colors_gte_counts_only_permanents_matching_the_filter() {
    let mut r = runner();
    r.place_on_field(0, "T-RB", None);
    r.place_on_field(0, "T-Y-PLAIN", None);
    r.place_on_field(0, "D-P", None);
    assert!(holds(&r, &all_tamers(3)), "control: all Tamers have 3 colours");
    assert!(
        !holds(&r, &adventure_tamers(3)),
        "non-[ADVENTURE] Yellow Tamer and the Purple Digimon are filtered out"
    );
    assert!(holds(&r, &adventure_tamers(2)));

    r.place_on_field(1, "T-G", None);
    assert!(!holds(&r, &adventure_tamers(3)), "opponent's [ADVENTURE] Tamer is not ours");

    r.place_on_field(0, "T-G", None);
    assert!(holds(&r, &adventure_tamers(3)), "Red + Blue + Green [ADVENTURE] Tamers");
}

#[test]
fn distinct_colors_gte_counts_gained_colours() {
    let mut r = runner();
    r.place_on_field(0, "T-R", None);
    let blue = r.place_on_field(0, "T-B", None);
    assert!(!holds(&r, &adventure_tamers(3)));
    r.game.modifiers.add(
        blue,
        ModifierEntry::simple(ModifierType::AddColor, 0, Expiry::Permanent, 0)
            .with_payload(ModifierPayload::Colors { value: vec![CardColor::Green], base: false }),
    );
    assert!(holds(&r, &adventure_tamers(3)), "Blue Tamer gaining Green → 3 rules colours");
}
