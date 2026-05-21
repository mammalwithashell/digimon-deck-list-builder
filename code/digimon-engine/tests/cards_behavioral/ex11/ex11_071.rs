//! EX11-071 Cool Boy - Tamer, White, LIBERATOR.
//!
//! Implemented slice (Phase 2 Track J PR 2 closure of RK-G002):
//! - [On Play] reveal top 3; add 1 [Omekamon]/[Omnimon (X Antibody)] and 1
//!   [Royal Knight]/[LIBERATOR] trait card; bottom the rest.
//! - [Main] By returning this Tamer to the bottom of the deck, may play 1 cost
//!   4+ [Royal Knight]/[LIBERATOR] from hand with play cost reduced by 2.
//!   Lowers onto Phase 2 Track B's `activation_cost: { return_self_to_deck_bottom: true }`
//!   plus a `play_cost_gte: 4` filter (predicate added in this PR).

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledPredicate, CompiledStackPosition,
    CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-071")
        .expect("EX11-071 YAML loads")
        .memory(10)
        .start()
}

fn predicate_contains_name(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.name_is.as_deref() == Some(needle)
        || predicate.name_contains.as_deref() == Some(needle)
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_name(part, needle))
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_name(part, needle))
}

fn predicate_contains_trait(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.trait_has.as_deref() == Some(needle)
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_trait(part, needle))
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_trait(part, needle))
}

#[test]
fn ex11_071_has_printed_metadata() {
    let runner = runner();
    let card = runner
        .compiled_card("EX11-071")
        .expect("EX11-071 compiled card present");

    assert_eq!(card.name, "Cool Boy");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "LIBERATOR"));
}

#[test]
fn ex11_071_on_play_uses_dual_reveal_buckets_and_bottoms_remainder() {
    let runner = runner();
    let card = runner
        .compiled_card("EX11-071")
        .expect("EX11-071 compiled card present");

    assert_eq!(
        card.effects.len(),
        2,
        "On Play search clause plus [Main] return-self reduced-cost play clause"
    );
    let on_play = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("On Play search clause exists");

    assert!(matches!(
        on_play.process.first(),
        Some(CompiledStep::RevealTopDeck { count: 3, .. })
    ));

    let buckets = on_play
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectRevealBuckets {
                buckets,
                no_duplicate_cards,
                ..
            } => Some((buckets, no_duplicate_cards)),
            _ => None,
        })
        .expect("On Play must use select_reveal_buckets");

    assert!(
        *buckets.1,
        "one revealed card cannot satisfy both additions"
    );
    assert_eq!(buckets.0.len(), 2);
    assert!(
        predicate_contains_name(
            buckets.0[0].filter.as_ref().expect("first bucket filter"),
            "Omekamon"
        ) && predicate_contains_name(
            buckets.0[0].filter.as_ref().expect("first bucket filter"),
            "Omnimon (X Antibody)"
        ),
        "first bucket must accept [Omekamon] or [Omnimon (X Antibody)]"
    );
    assert!(
        predicate_contains_trait(
            buckets.0[1].filter.as_ref().expect("second bucket filter"),
            "Royal Knight"
        ) && predicate_contains_trait(
            buckets.0[1].filter.as_ref().expect("second bucket filter"),
            "LIBERATOR"
        ),
        "second bucket must accept [Royal Knight] or [LIBERATOR] trait cards"
    );
    assert_eq!(
        on_play
            .process
            .iter()
            .filter(|step| matches!(step, CompiledStep::AddToHandFromReveal { .. }))
            .count(),
        2
    );
    assert!(on_play.process.iter().any(|step| matches!(
        step,
        CompiledStep::PlaceRemainderOnDeck {
            position: CompiledStackPosition::Bottom,
            ..
        }
    )));
}

/// The [Main] clause compiles to an `activation_cost: return_self_to_deck_bottom`
/// step followed by `select_hand` filtered by `play_cost_gte: 4` and
/// `Royal Knight`/LIBERATOR trait, then `play_from_hand` with `cost_delta:
/// { reduce: 2 }`.
#[test]
fn ex11_071_main_clause_uses_activation_cost_and_reduced_play_from_hand() {
    use digimon_dsl::compiled::{
        CompiledActivationCostKind, CompiledCostDelta, CompiledDpConstraint,
    };

    let runner = runner();
    let card = runner
        .compiled_card("EX11-071")
        .expect("EX11-071 compiled card present");

    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::MainOnField) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Main] clause exists");

    assert!(main.optional, "printed 'you may' surfaces as optional");

    assert!(
        main.process.iter().any(|step| matches!(
            step,
            CompiledStep::ActivationCost {
                kind: CompiledActivationCostKind::ReturnSelfToDeckBottom,
                ..
            }
        )),
        "first step is activation_cost: return_self_to_deck_bottom"
    );
    let sel = main
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectHand { filter, .. } => Some(filter),
            _ => None,
        })
        .expect("hand selection step exists");
    fn play_cost_gte(p: &CompiledPredicate) -> Option<CompiledDpConstraint> {
        if p.play_cost_gte.is_some() {
            return p.play_cost_gte.clone();
        }
        for part in p.all_of.iter().chain(p.any_of.iter()) {
            if let Some(v) = play_cost_gte(part) {
                return Some(v);
            }
        }
        None
    }
    assert_eq!(
        play_cost_gte(sel),
        Some(CompiledDpConstraint::Literal(4)),
        "select filter requires play_cost_gte: 4"
    );
    assert!(
        predicate_contains_trait(sel, "Royal Knight")
            && predicate_contains_trait(sel, "LIBERATOR"),
        "hand filter accepts Royal Knight or LIBERATOR"
    );

    assert!(
        main.process.iter().any(|step| matches!(
            step,
            CompiledStep::PlayFromHand {
                cost_delta: Some(CompiledCostDelta::Reduce(2)),
                ..
            }
        )),
        "play_from_hand uses cost_delta reduce 2"
    );
}
