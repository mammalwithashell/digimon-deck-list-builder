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
        predicate_contains_trait(sel, "Royal Knight") && predicate_contains_trait(sel, "LIBERATOR"),
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

// ═══════════════════════════════════════════════════════════════════════════
// G-ENGINE-MAIN-ON-FIELD-ACTIVATION-COST-UNPAID
//
// "[Main] **By returning this Tamer to the bottom of the deck**, you may play
// 1 play cost 4 or higher [Royal Knight] or [LIBERATOR] trait card from your
// hand with the play cost reduced by 2."
//
// "By X, Y" is an optional processing condition (general_rule.pdf §15-7-1);
// §15-7-2 — if the condition's content isn't executed, the processing after it
// can't be executed. The [Main] ACTION path (`Game::activate_field_main`) must
// therefore return the Tamer to the deck bottom before the reduced-cost play
// body runs. DCGO EX11_071.cs wraps the body in
// `CardEffectCommons.DeckBouncePeremanentAndProcessAccordingToResult(...)`
// with `successProcess` only (EX11/White/EX11_071.cs:108-116), i.e. no bounce
// => no play.
// ═══════════════════════════════════════════════════════════════════════════

/// Activating the [Main] through the ACTION path pays the printed
/// "By returning this Tamer to the bottom of the deck" cost.
#[test]
fn ex11_071_main_action_path_returns_self_to_deck_bottom() {
    use digimon_engine::card_data::CardData;
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::enums::CardKind;

    fn knight(id: &str) -> CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.level = Some(5);
        c.dp = Some(6000);
        c.play_cost = 6;
        c.traits = vec!["Royal Knight".to_string()];
        c
    }

    let mut r = DebugRunner::builder()
        .dsl_card("EX11-071")
        .expect("EX11-071 YAML loads")
        .add_card(knight("RK-IN-HAND"))
        .add_card({
            let mut c = make_test_card("EX11-FILL", "Filler");
            c.card_kind = CardKind::Digimon;
            c
        })
        .hand(0, &["RK-IN-HAND"])
        .deck(0, &["EX11-FILL"; 8])
        .memory(10)
        .start();

    let cool_boy = r.place_on_field(0, "EX11-071", Some(0));
    r.game.enter_main_phase();

    let deck_before = r.game.player(0).deck.len();
    let fired = r.game.activate_field_main(0, cool_boy.index as usize);
    assert!(fired, "[Main] fires through the action path");

    assert_eq!(
        r.game.player(0).deck.len(),
        deck_before + 1,
        "the printed 'By returning this Tamer to the bottom of the deck' cost \
         must be PAID on the [Main] action path (general_rule.pdf §15-7-2)"
    );
    assert_eq!(
        r.game.player(0).deck[0].card_id(&r.game.card_data),
        "EX11-071",
        "the Tamer is on the BOTTOM of the deck (index 0)"
    );
    assert!(
        !r.game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "EX11-071"),
        "the Tamer left the battle area as the cost"
    );
}
