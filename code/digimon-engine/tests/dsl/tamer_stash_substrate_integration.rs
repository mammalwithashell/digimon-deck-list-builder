//! Phase A integration — end-to-end "Tamer face-down stash substrate".
//!
//! Phase A built the substrate in three halves:
//!   * A1/A2 — `place_as_bottom_source` can tuck a source face-down under a
//!     permanent (a Tamer, here).
//!   * A3    — `select_own_sources` / `select_own_permanent` predicate leaves
//!     filter by `is_face_down` / `is_bottom_source` / `has_face_down_source`.
//!   * A4    — the `trash_bottom_face_down_source_under_tamer` verb pays a cost
//!     by trashing the bottom face-down source under one of your Tamers.
//!
//! Every existing test exercises ONE half in isolation, and the placement
//! tests fabricate the stack state directly. This test connects the halves in
//! a SINGLE game: place a face-down source under a Tamer through the genuine
//! A1/A2 `place_as_bottom_source` DSL step, then consume it through the
//! genuine A4 `trash_bottom_face_down_source_under_tamer` verb. It is cheap
//! insurance that the substrate composes correctly and stays composed.
//!
//! Pattern mirrors `place_as_bottom_source_deck_top.rs` (the `{ deck_top: you }`
//! placement) and `trash_bottom_face_down_source_under_tamer.rs` (driving the
//! A4 verb and resolving its 1-option selection).

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;

/// Build a `CardData` like `make_test_card` but with the given `CardKind`, so
/// the placement carrier's TOP card can be a genuine Tamer.
fn make_test_card_kind(card_id: &str, card_name: &str, kind: CardKind) -> CardData {
    let mut d = make_test_card(card_id, card_name);
    d.card_kind = kind;
    d
}

/// The placement card: a Tamer whose `[On Play]` effect tucks the controller's
/// deck-top card face-down under itself (`target: source` resolves to this
/// Tamer's own permanent). A deck-top source runs synchronously — no selection.
const PLACER_YAML: &str = r#"
card: TST-STASH-PLACER
name: StashPlacerTamer
kind: tamer
color: [red]
cost: 3
effects:
  - when: on_play
    process:
      - place_as_bottom_source:
          source: { deck_top: you }
          target: source
          face_down: true
"#;

/// The consumption card: a Digimon whose `[On Play]` effect trashes the bottom
/// face-down source from under one of the controller's Tamers, then gains 1
/// memory as the observable tail.
const CONSUMER_YAML: &str = r#"
card: TST-STASH-CONSUMER
name: StashConsumer
kind: digimon
level: 3
color: [red]
cost: 4
dp: 3000
effects:
  - when: on_play
    process:
      - trash_bottom_face_down_source_under_tamer:
          of: you
      - gain_memory: 1
"#;

#[test]
fn tamer_stash_substrate_place_then_trash_pipeline() {
    let placer_spec: digimon_dsl::CardSpec =
        serde_yml::from_str(PLACER_YAML).expect("placer YAML parses");
    let placer_compiled =
        Arc::new(digimon_dsl::compile::compile(&placer_spec).expect("placer compiles cleanly"));
    let consumer_spec: digimon_dsl::CardSpec =
        serde_yml::from_str(CONSUMER_YAML).expect("consumer YAML parses");
    let consumer_compiled =
        Arc::new(digimon_dsl::compile::compile(&consumer_spec).expect("consumer compiles cleanly"));

    // Seed ONE game with:
    //   * TST-STASH-PLACER  — the placement Tamer, put on the field below.
    //   * TST-STASH-SEED    — the deck-top card that gets tucked face-down.
    //   * TST-STASH-CONSUMER — the consumption Digimon, sits in hand.
    //
    // `DebugRunnerBuilder::deck` treats the LAST element as the top of the
    // deck — exactly what `CardSourceRef::DeckTop` pops.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card_kind(
            "TST-STASH-PLACER",
            "StashPlacerTamer",
            CardKind::Tamer,
        ))
        .add_card(make_test_card("TST-STASH-SEED", "StashSeed"))
        .add_card(make_test_card("TST-STASH-CONSUMER", "StashConsumer"))
        .deck(0, &["TST-STASH-SEED"])
        .hand(0, &["TST-STASH-CONSUMER"])
        .memory(5)
        .start();

    // --- Place the placement Tamer on the field ---------------------------
    // It must be a permanent so the placer's `target: source` binding resolves
    // to a Tamer-kind permanent.
    let tamer = runner.place_on_field(0, "TST-STASH-PLACER", None);
    let tamer_card: CardHandle = runner.game.players[0].battle_area[tamer.index as usize]
        .top_card()
        .handle();

    // Precondition: the placement Tamer's only source is its own face-up card;
    // the deck holds exactly the seed card.
    {
        let perm = &runner.game.players[0].battle_area[tamer.index as usize];
        assert_eq!(
            perm.card_sources.len(),
            1,
            "precondition: the Tamer starts with only its own card on the stack"
        );
        assert!(
            !perm.card_sources[0].face_down,
            "precondition: the Tamer's own card is face-up"
        );
    }
    assert_eq!(
        runner.game.players[0].deck.len(),
        1,
        "precondition: deck holds exactly the seed card"
    );
    let seed_handle: CardHandle = runner.game.players[0]
        .deck
        .last()
        .expect("deck-top seed card")
        .handle();

    // --- STEP 1: place a face-down source under the Tamer via the REAL A1/A2
    // `place_as_bottom_source` DSL step (NOT fabricated card_sources mutation).
    {
        let placer_effect = DslCardEffect::new(placer_compiled);
        let effects = placer_effect.effects(tamer_card);
        assert_eq!(effects.len(), 1, "placer has exactly one OnPlay effect");
        let process = effects[0].process.as_ref().expect("placer OnPlay process");
        let mut ctx = EffectContext::new(&mut runner.game, tamer_card, Some(tamer), 0);
        process(&mut ctx);
    }

    // A deck-top source runs synchronously — no selection is installed.
    assert!(
        runner.game.pending_selection.is_none(),
        "placement via a deck-top source is synchronous — no selection"
    );

    // Assert placement: deck was consumed, the seed sits face-down at the
    // BOTTOM (card_sources[0]) of the Tamer's stack.
    assert_eq!(
        runner.game.players[0].deck.len(),
        0,
        "the deck-top seed card was consumed by placement"
    );
    {
        let perm = &runner.game.players[0].battle_area[tamer.index as usize];
        assert_eq!(
            perm.card_sources.len(),
            2,
            "the Tamer's stack grew by 1 — the face-down stash was placed"
        );
        let bottom = &perm.card_sources[0];
        assert_eq!(
            bottom.handle(),
            seed_handle,
            "the placed source must be at the BOTTOM of the stack (card_sources[0])"
        );
        assert!(
            bottom.face_down,
            "the placed bottom source must be face-down"
        );
    }

    // --- STEP 2: consume the stash via the REAL A4
    // `trash_bottom_face_down_source_under_tamer` verb, in the SAME game.
    let consumer_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;
    assert_eq!(
        runner.game.players[0].trash.len(),
        0,
        "precondition: trash is empty before consumption"
    );

    {
        let consumer_effect = DslCardEffect::new(consumer_compiled);
        let effects = consumer_effect.effects(consumer_card);
        assert_eq!(effects.len(), 1, "consumer has exactly one OnPlay effect");
        let process = effects[0]
            .process
            .as_ref()
            .expect("consumer OnPlay process");
        let mut ctx = EffectContext::new(&mut runner.game, consumer_card, None, 0);
        process(&mut ctx);
    }

    // The A4 verb installs a 1-option `select_own_permanent` even with a single
    // eligible Tamer — the no-approximations policy never auto-resolves.
    let (action_id, selecting_player) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("the A4 verb installs a 1-option Tamer selection");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "exactly one eligible Tamer (the one with the face-down stash)"
        );
        (pending.valid_action_ids[0], pending.selecting_player)
    };

    // The tail has NOT run yet — it runs inside the selection callback.
    assert_eq!(
        runner.game.memory, memory_before,
        "tail (gain_memory) must not run before the selection resolves"
    );

    // Resolve the selection — the callback trashes the bottom face-down source
    // and runs the tail.
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve_selection succeeds");
    assert!(
        runner.game.pending_selection.is_none(),
        "pending selection cleared after resolution"
    );

    // --- Assert the full pipeline outcome --------------------------------
    // The exact source placed in STEP 1 is now in the controller's trash.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == seed_handle),
        "the placed face-down source must be in the controller's trash after the A4 verb"
    );

    // The Tamer's stack shrank back to just its own card.
    {
        let perm = &runner.game.players[0].battle_area[tamer.index as usize];
        assert_eq!(
            perm.card_sources.len(),
            1,
            "the Tamer's stack shrank by 1 — the face-down stash was popped"
        );
        assert_eq!(
            perm.card_sources[0].handle(),
            perm.top_card().handle(),
            "the Tamer retains only its own card as the remaining source"
        );
        assert_ne!(
            perm.card_sources[0].handle(),
            seed_handle,
            "the trashed stash is no longer on the Tamer's stack"
        );
    }

    // The observable tail ran: +1 memory.
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "tail (gain_memory: 1) must run after the selection resolves"
    );
}
