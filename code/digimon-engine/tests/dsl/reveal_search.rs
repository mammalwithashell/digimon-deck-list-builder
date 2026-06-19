//! collapse-dsl-step-idioms §2 — `reveal_search` composite behavioral tests.
//!
//! `reveal_search` lowers to reveal_top_deck → ONE select_reveal_buckets over
//! all buckets → per-bucket reveal-move (the §2.1 multi-card verbs) →
//! place_remainder_on_deck. These tests assert parity with that longhand idiom
//! across the corner cases the hand-rolled form had to handle (§2.3): a
//! mandatory "add up to N" bucket, an optional bucket declined, an empty/short
//! pool, cross-bucket de-dup with two destinations, and remainder ordering.

use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;

/// Run a one-OnPlay `reveal_search` card's process from `card` (player 0).
fn run_on_play(runner: &mut DebugRunner, compiled: digimon_dsl::compiled::CompiledCard, card: CardHandle) {
    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl_effect.effects(card);
    assert_eq!(effects.len(), 1, "one OnPlay effect");
    let process = effects[0].process.as_ref().expect("has process");
    let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
    process(&mut ctx);
}

/// Greedily resolve a reveal-bucket multi-pick by always taking the first valid
/// action (a card pick when picks remain). Bounded guard against non-convergence.
fn resolve_all_picks(runner: &mut DebugRunner) {
    let mut guard = 0;
    while runner.game.pending_selection.is_some() {
        let (aid, p) = {
            let s = runner.game.pending_selection.as_ref().unwrap();
            (s.valid_action_ids[0], s.selecting_player)
        };
        runner.game.resolve_selection(p, aid).expect("resolve pick");
        guard += 1;
        assert!(guard < 16, "reveal_search selection did not converge");
    }
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn compile(yaml: &str) -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("valid YAML");
    digimon_dsl::compile::compile(&spec).expect("compiles")
}

const SEARCHER: &str = r#"
card: RS-001
name: Searcher
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - reveal_search:
          of: you
          count: 4
          buckets:
            - filter: { name_contains: "Garurumon" }
              to: hand
              max: 2
              prompt: "Add up to 2 [Garurumon] cards to hand"
          remainder: bottom
"#;

/// Mandatory "add up to 2" bucket with exactly 2 matching cards in the pool:
/// both land in hand (the §2.1 multi-card add), the rest go to the deck bottom.
#[test]
fn reveal_search_adds_two_matching_to_hand_rest_to_bottom() {
    let compiled = compile(SEARCHER);
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RS-001", "Searcher"))
        .add_card(make_test_card("F1", "Filler"))
        .add_card(make_test_card("F2", "Filler"))
        .add_card(make_test_card("M1", "Garurumon"))
        .add_card(make_test_card("M2", "Garurumon"))
        .hand(0, &["RS-001"])
        .deck(0, &["F1", "F2", "M1", "M2"])
        .build();

    let card = runner.game.players[0].hand[0].handle();
    run_on_play(&mut runner, compiled, card);
    resolve_all_picks(&mut runner);

    let hand = hand_ids(&runner);
    assert!(
        hand.contains(&"M1".to_string()) && hand.contains(&"M2".to_string()),
        "both matching cards added to hand: {hand:?}"
    );
    assert!(
        !hand.contains(&"F1".to_string()) && !hand.contains(&"F2".to_string()),
        "non-matching cards must NOT be in hand: {hand:?}"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        2,
        "the 2 non-matching cards return to the deck bottom"
    );
    assert!(
        runner.game.revealed_cards.is_empty(),
        "reveal pool emptied after reveal_search resolves"
    );
}

/// Fewer matches than `max`: a mandatory `max: 2` bucket with only 1 matching
/// card adds the 1 (no soft-lock when the candidate pool is exhausted below min).
#[test]
fn reveal_search_adds_all_when_fewer_than_max_match() {
    let compiled = compile(SEARCHER);
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RS-001", "Searcher"))
        .add_card(make_test_card("F1", "Filler"))
        .add_card(make_test_card("F2", "Filler"))
        .add_card(make_test_card("F3", "Filler"))
        .add_card(make_test_card("M1", "Garurumon"))
        .hand(0, &["RS-001"])
        .deck(0, &["F1", "F2", "F3", "M1"])
        .build();

    let card = runner.game.players[0].hand[0].handle();
    run_on_play(&mut runner, compiled, card);
    resolve_all_picks(&mut runner);

    let hand = hand_ids(&runner);
    assert!(hand.contains(&"M1".to_string()), "the lone match is added: {hand:?}");
    assert_eq!(runner.game.players[0].deck.len(), 3, "3 fillers return to deck");
    assert!(runner.game.pending_selection.is_none());
}

/// Empty/short pool: deck has fewer cards than `count` and none match — the
/// bucket installs no pick (no candidate), and the only selection is the
/// remainder ordering inherited from `place_remainder_on_deck`. No card is
/// added; the lone filler returns to the deck. No soft-lock.
#[test]
fn reveal_search_short_pool_adds_nothing_returns_remainder() {
    let compiled = compile(SEARCHER);
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RS-001", "Searcher"))
        .add_card(make_test_card("F1", "Filler"))
        .hand(0, &["RS-001"])
        .deck(0, &["F1"])
        .build();

    let card = runner.game.players[0].hand[0].handle();
    run_on_play(&mut runner, compiled, card);
    // The bucket exposes no candidate (nothing matches); the only pending is
    // the remainder ordering for the lone filler — resolve it.
    resolve_all_picks(&mut runner);

    let hand = hand_ids(&runner);
    assert!(!hand.contains(&"F1".to_string()), "no match → filler NOT added to hand");
    assert_eq!(runner.game.players[0].deck.len(), 1, "filler returned to deck");
    assert!(runner.game.revealed_cards.is_empty(), "reveal pool emptied");
}

const OPTIONAL_SEARCHER: &str = r#"
card: RS-002
name: Optional Searcher
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - reveal_search:
          of: you
          count: 4
          buckets:
            - filter: { name_contains: "Garurumon" }
              to: hand
              max: 2
              optional: true
              prompt: "You may add up to 2 [Garurumon]"
          remainder: bottom
"#;

/// All-optional bucket declined: even with matching cards present, the player
/// may PASS and add nothing; the whole pool becomes the remainder.
#[test]
fn reveal_search_optional_bucket_declined_adds_nothing() {
    let compiled = compile(OPTIONAL_SEARCHER);
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RS-002", "Optional Searcher"))
        .add_card(make_test_card("F1", "Filler"))
        .add_card(make_test_card("F2", "Filler"))
        .add_card(make_test_card("M1", "Garurumon"))
        .add_card(make_test_card("M2", "Garurumon"))
        .hand(0, &["RS-002"])
        .deck(0, &["F1", "F2", "M1", "M2"])
        .build();

    let card = runner.game.players[0].hand[0].handle();
    run_on_play(&mut runner, compiled, card);

    // The optional bucket is the first pending and exposes PASS (is_optional).
    let pending = runner.game.pending_selection.as_ref().expect("optional bucket pending");
    assert!(pending.is_optional, "optional bucket must expose PASS");
    let p = pending.selecting_player;
    // Decline the bucket entirely.
    runner
        .game
        .resolve_selection(p, digimon_engine::action::space::PASS)
        .expect("PASS the optional bucket");
    // Resolve the remainder ordering.
    resolve_all_picks(&mut runner);

    let hand = hand_ids(&runner);
    assert!(
        !hand.contains(&"M1".to_string()) && !hand.contains(&"M2".to_string()),
        "declined optional bucket adds nothing: {hand:?}"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        4,
        "all 4 revealed cards become the remainder and return to deck"
    );
}

const TWO_BUCKET_SEARCHER: &str = r#"
card: RS-003
name: Split Searcher
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - reveal_search:
          of: you
          count: 4
          buckets:
            - filter: { name_contains: "Garurumon" }
              to: hand
              max: 1
            - filter: { name_contains: "Garurumon" }
              to: trash
              max: 1
          remainder: bottom
"#;

/// Two buckets drawing from one pool with cross-bucket de-dup: a card picked
/// into the hand bucket is no longer a candidate for the trash bucket, so the
/// two Garurumon cards route to DISTINCT destinations (one hand, one trash).
#[test]
fn reveal_search_two_buckets_dedup_routes_distinctly() {
    let compiled = compile(TWO_BUCKET_SEARCHER);
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RS-003", "Split Searcher"))
        .add_card(make_test_card("F1", "Filler"))
        .add_card(make_test_card("F2", "Filler"))
        .add_card(make_test_card("M1", "Garurumon"))
        .add_card(make_test_card("M2", "Garurumon"))
        .hand(0, &["RS-003"])
        .deck(0, &["F1", "F2", "M1", "M2"])
        .build();

    let card = runner.game.players[0].hand[0].handle();
    run_on_play(&mut runner, compiled, card);
    resolve_all_picks(&mut runner);

    let hand: Vec<String> = hand_ids(&runner)
        .into_iter()
        .filter(|id| id == "M1" || id == "M2")
        .collect();
    let trash: Vec<String> = runner.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .filter(|id| id == "M1" || id == "M2")
        .collect();

    assert_eq!(hand.len(), 1, "exactly one Garurumon to hand: {hand:?}");
    assert_eq!(trash.len(), 1, "exactly one Garurumon to trash: {trash:?}");
    assert_ne!(hand[0], trash[0], "de-dup: the two buckets get DISTINCT cards");
    assert_eq!(runner.game.players[0].deck.len(), 2, "the 2 fillers return to deck");
}
