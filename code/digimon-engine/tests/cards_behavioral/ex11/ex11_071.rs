//! EX11-071 Cool Boy - Tamer, White, LIBERATOR.
//!
//! Supported slice:
//! - [On Play] reveal top 3; add 1 [Omekamon]/[Omnimon (X Antibody)] and 1
//!   [Royal Knight]/[LIBERATOR] trait card; bottom the rest.
//!
//! Gap-routed:
//! - [Main] By returning this Tamer to the bottom of the deck, may play 1 cost
//!   4+ [Royal Knight]/[LIBERATOR] from hand with play cost reduced by 2. This
//!   needs a source-bound return-to-deck activation cost feeding a reduced-cost
//!   hand play selection.

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
        1,
        "Main return-self reduced play clause is intentionally omitted"
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

#[test]
#[ignore = "pending: RK-G002 — source-bound return-self activation cost into reduced-cost hand play"]
fn ex11_071_main_returns_self_to_bottom_to_play_cost_four_rk_or_liberator_reduced_by_two() {
    panic!("requires return-self cost and reduced-cost hand play selection support");
}
