//! BT20-100 The Last Guardian - Option, White, Royal Knight.
//!
//! Supported slice:
//! - [Main] reveal top 3; add 1 [Cool Boy] and 1 [Royal Knight]/[X Antibody]
//!   trait card; bottom the rest; place self in battle area.
//! - [Security] branch over hand/trash for optional [Omekamon]/[Cool Boy] play,
//!   then place self in battle area.
//!
//! Gap-routed:
//! - Delay leave-prevention for an [Omnimon]-named Digimon needs a
//!   cross-permanent replacement that trashes the Delay option and cancels the
//!   specific would-leave event.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledPredicate, CompiledStackPosition,
    CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT20-100")
        .expect("BT20-100 YAML loads")
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
fn bt20_100_has_printed_metadata() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-100")
        .expect("BT20-100 compiled card present");

    assert_eq!(card.name, "The Last Guardian");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
}

#[test]
fn bt20_100_main_uses_dual_reveal_buckets_and_places_self_as_delay_option() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-100")
        .expect("BT20-100 compiled card present");

    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::MainFromHand) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Main] reveal/search clause exists");

    assert!(matches!(
        main.process.first(),
        Some(CompiledStep::RevealTopDeck { count: 3, .. })
    ));
    let (buckets, no_duplicate_cards) = main
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
        .expect("main search uses reveal buckets");
    assert!(*no_duplicate_cards);
    assert_eq!(buckets.len(), 2);
    assert!(predicate_contains_name(
        buckets[0].filter.as_ref().expect("Cool Boy bucket filter"),
        "Cool Boy"
    ));
    assert!(
        predicate_contains_trait(
            buckets[1].filter.as_ref().expect("trait bucket filter"),
            "Royal Knight"
        ) && predicate_contains_trait(
            buckets[1].filter.as_ref().expect("trait bucket filter"),
            "X Antibody"
        )
    );
    assert!(main.process.iter().any(|step| matches!(
        step,
        CompiledStep::PlaceRemainderOnDeck {
            position: CompiledStackPosition::Bottom,
            ..
        }
    )));
    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)));
}

#[test]
fn bt20_100_security_places_self_after_optional_hand_or_trash_play_branch() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-100")
        .expect("BT20-100 compiled card present");

    let security = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnSecurity) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Security] clause exists");

    assert!(security.optional, "printed Security play says 'may'");
    assert!(security
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::SelectEffectChoice { .. })));
    assert!(security
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)));
}

#[test]
#[ignore = "pending: RK-G003 — Delay leave-prevention replacement for Omnimon-name subject"]
fn bt20_100_delay_prevents_omnimon_named_digimon_from_leaving() {
    panic!("requires Delay option replacement that cancels a cross-permanent would-leave event");
}
