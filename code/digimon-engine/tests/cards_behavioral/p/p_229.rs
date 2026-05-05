//! P-229 Unique Emblem: Narrative Ronde — Option, Cost 2, Yellow, [LIBERATOR].
//!
//! Printed text (`data/cards.json`):
//! - [Main] Reveal the top 3 cards of your deck. Add 1 [Puppet] trait Digimon
//!   card and 1 [LIBERATOR] trait card among them to the hand. Return the rest
//!   to the bottom of the deck. Then, place this card in the battle area.
//! - [Your Turn] When any of your [Mirai Kinosaki]s are played, <Delay>
//!   1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait
//!   card in the hand with the digivolution cost reduced by 3.
//! - Security Effect [Security] Activate this card's [Main] effects.
//!
//! Supported slice in this worker:
//! - Main/security mirrored top-3 dual-bucket reveal search using
//!   `select_reveal_buckets`, with duplicate revealed-card prevention.
//!
//! Known gaps:
//! - `PUPPETS-G004`: event-gated Delay on Mirai Kinosaki being played is still
//!   blocked because `on_ally_played` is virtual/skipped and the Delay body must
//!   expose an effect-initiated digivolve choice.
//! - `PUPPETS-G009`: standard Delay is not a player-controlled Main activation
//!   from battle area yet; automatic scheduling would hide the activation choice.

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledPredicate, CompiledScope,
    CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn p_229_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-229.yaml"))
        .expect("P-229 YAML must exist at cards/p/P-229.yaml")
}

fn p_229_runner() -> DebugRunner {
    let yaml = p_229_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-229 YAML must parse and compile")
        .memory(10)
        .start()
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn puppet_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Puppet".to_string()];
    card
}

fn liberator_option(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["LIBERATOR".to_string()];
    card
}

fn puppet_liberator_digimon(id: &str) -> CardData {
    let mut card = puppet_digimon(id);
    card.traits.push("LIBERATOR".to_string());
    card
}

fn predicate_has_puppet_digimon_filter(predicate: &CompiledPredicate) -> bool {
    predicate.all_of.iter().any(|part| {
        part.kind == Some(CompiledCardKind::Digimon)
            || part
                .all_of
                .iter()
                .any(|nested| nested.kind == Some(CompiledCardKind::Digimon))
    }) && predicate.all_of.iter().any(|part| {
        part.trait_has.as_deref() == Some("Puppet")
            || part
                .all_of
                .iter()
                .any(|nested| nested.trait_has.as_deref() == Some("Puppet"))
    })
}

#[test]
fn p_229_yaml_parses_and_compiles() {
    let _runner = p_229_runner();
}

#[test]
fn p_229_is_yellow_liberator_option_cost_2() {
    let runner = p_229_runner();
    let compiled = runner.compiled_card("P-229").expect("P-229 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(2));
    assert_eq!(compiled.traits, vec!["LIBERATOR".to_string()]);
}

#[test]
fn p_229_has_main_and_security_search_clauses_without_delay_stub() {
    let runner = p_229_runner();
    let compiled = runner.compiled_card("P-229").expect("P-229 compiled");

    assert_eq!(
        compiled.effects.len(),
        2,
        "only the supported Main search and mirrored Security search should be present"
    );

    assert!(matches!(
        &compiled.effects[0],
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::FaceUp
                && t.when.contains(&CompiledTiming::MainFromHand)
                && !t.optional
    ));
    assert!(matches!(
        &compiled.effects[1],
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity)
                && !t.optional
    ));
}

#[test]
fn p_229_main_uses_dual_reveal_buckets_for_puppet_digimon_and_liberator() {
    let runner = p_229_runner();
    let compiled = runner.compiled_card("P-229").expect("P-229 compiled");
    let CompiledClause::Triggered(main) = &compiled.effects[0] else {
        panic!("clause 0 must be MainFromHand");
    };

    assert!(matches!(
        main.process.first(),
        Some(CompiledStep::RevealTopDeck { count: 3, .. })
    ));

    let buckets = main
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
        .expect("Main must use select_reveal_buckets");

    assert!(
        *buckets.1,
        "the same revealed card cannot satisfy both buckets"
    );
    assert_eq!(buckets.0.len(), 2);
    assert_eq!(buckets.0[0].bind_as, "puppet_pick");
    assert_eq!(buckets.0[0].min, 1);
    assert_eq!(buckets.0[0].max, 1);
    assert!(
        predicate_has_puppet_digimon_filter(
            buckets.0[0].filter.as_ref().expect("Puppet bucket filter")
        ),
        "first bucket must require a [Puppet] trait Digimon"
    );
    assert_eq!(buckets.0[1].bind_as, "liberator_pick");
    assert_eq!(buckets.0[1].min, 1);
    assert_eq!(buckets.0[1].max, 1);
    assert_eq!(
        buckets.0[1]
            .filter
            .as_ref()
            .and_then(|filter| filter.trait_has.as_deref()),
        Some("LIBERATOR"),
        "second bucket must accept [LIBERATOR] trait cards of any kind"
    );

    assert_eq!(
        main.process
            .iter()
            .filter(|step| matches!(step, CompiledStep::AddToHandFromReveal { .. }))
            .count(),
        2,
        "both bucket bindings must be added to hand"
    );
    assert!(main.process.iter().any(|step| matches!(
        step,
        CompiledStep::PlaceRemainderOnDeck {
            position: CompiledStackPosition::Bottom,
            ..
        }
    )));
}

#[test]
fn p_229_main_reveal_buckets_expose_mandatory_masked_choices() {
    let yaml = p_229_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-229 YAML parses")
        .add_card(puppet_liberator_digimon("BOTH"))
        .add_card(liberator_option("LIB-OPT"))
        .add_card(filler("FILL"))
        .hand(0, &["P-229"])
        .deck(0, &["FILL", "LIB-OPT", "BOTH"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert!(runner.game.activate_hand_main(0, 0));

    let first = runner
        .pending_selection_view()
        .expect("Puppet bucket selection should be pending");
    assert_eq!(first.valid_action_ids.len(), 1);
    assert!(
        !first.valid_action_ids.contains(&PASS),
        "Puppet bucket is mandatory when an eligible revealed card exists"
    );
    let first_action = first.valid_action_ids[0];
    runner
        .execute_action(first.selecting_player, first_action)
        .expect("choose Puppet bucket card");

    let second = runner
        .pending_selection_view()
        .expect("LIBERATOR bucket selection should be pending");
    assert_eq!(
        second.valid_action_ids.len(),
        1,
        "the already-picked card must be excluded from the LIBERATOR bucket"
    );
    assert!(
        !second.valid_action_ids.contains(&PASS),
        "LIBERATOR bucket is mandatory when an eligible revealed card exists"
    );
}

#[test]
fn p_229_main_adds_distinct_puppet_digimon_and_liberator_card_to_hand() {
    let yaml = p_229_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-229 YAML parses")
        .add_card(puppet_liberator_digimon("BOTH"))
        .add_card(liberator_option("LIB-OPT"))
        .add_card(filler("FILL"))
        .hand(0, &["P-229"])
        .deck(0, &["FILL", "LIB-OPT", "BOTH"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.game.player(0).hand.len();
    let deck_before = runner.deck_size(0);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .auto_resolve()
        .expect("resolve both reveal bucket choices");

    let hand_ids: Vec<_> = runner
        .game
        .player(0)
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(hand_ids.iter().any(|id| id == "BOTH"), "hand={hand_ids:?}");
    assert!(
        hand_ids.iter().any(|id| id == "LIB-OPT"),
        "hand={hand_ids:?}"
    );
    assert_eq!(runner.game.player(0).hand.len(), hand_before + 2);
    assert_eq!(
        deck_before - runner.deck_size(0),
        2,
        "two revealed cards move to hand and the third returns to deck bottom"
    );
}

#[test]
fn p_229_security_clause_mirrors_main_search_process() {
    let runner = p_229_runner();
    let compiled = runner.compiled_card("P-229").expect("P-229 compiled");
    let CompiledClause::Triggered(main) = &compiled.effects[0] else {
        panic!("clause 0 must be MainFromHand");
    };
    let CompiledClause::Triggered(security) = &compiled.effects[1] else {
        panic!("clause 1 must be inherited OnSecurity");
    };

    assert_eq!(
        security.process, main.process,
        "Security 'Activate this card's [Main] effects' must mirror the supported Main search body"
    );
}

#[test]
#[ignore = "pending: PUPPETS-G004 — on_ally_played event-gated Delay and reduced-cost effect digivolve choice are not supported"]
fn p_229_delay_triggers_after_mirai_is_played_and_offers_reduced_cost_digivolve() {
    todo!("play P-229, advance past the placing turn, play Mirai Kinosaki, assert a masked Delay digivolve choice for level <= 6 LIBERATOR cards in hand at cost -3");
}

#[test]
#[ignore = "pending: PUPPETS-G009/PUPPETS-G004 — option placement plus player-controlled Delay activation from battle area are incomplete"]
fn p_229_main_places_this_card_in_battle_area_after_search() {
    todo!("after resolving the Main search through the Option pipeline, P-229 should remain in battle area as a delayed Option rather than trash");
}
