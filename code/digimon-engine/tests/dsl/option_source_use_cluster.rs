//! Option-source USE cluster (round-2 reconciliation, ported onto the round-1
//! Option-USE core). Proves the effect-driven Option USE entry points for the
//! non-hand / non-trash origins — the game-level reveal pool
//! (`EffectContext::use_option_from_revealed`, driver EX7-048) and a permanent's
//! digivolution / link stack (`EffectContext::use_option_from_source`, driver
//! BT25-085) — plus the three DSL verbs that consume an upstream selection
//! binding and dispatch to the shared `Game::use_option_from` pipeline:
//!
//! * `use_option_from_revealed { of, card }` — consume a `select_reveal` pick
//!   (EX7-048 clause 1).
//! * `use_option_from_sources { of, card }` — consume a `select_own_sources`
//!   pick over a permanent's `card_sources` OR `linked_cards` (BT25-085).
//! * `use_option_bound { binding }` — consume a `select_union_zone{hand,trash}`
//!   pick, dispatching to the true origin zone (BT21-062).
//!
//! Every origin routes through the SAME round-1 `play_option_core` [Main]
//! resolution + `dispose_option`; only source validation / removal forks on the
//! `OptionSource` variant. The engine-core forks themselves are proved in
//! `option_lifecycle_cluster.rs`; this cluster proves the reveal / source /
//! union origins end-to-end (with the DSL verbs on top).

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCardKind, CompiledPlayerRef, CompiledPredicate,
    CompiledRevealBucket, CompiledStep, CompiledZone,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, CostDelta};
use digimon_engine::selection::OptionPlayResult;

/// A visible `[Main]` Option body that draws 1 card for its owner when it
/// resolves, so tests can assert the body actually ran (draw is player-scoped
/// and sign-unambiguous, unlike the shared memory gauge).
struct OptionMainDraw;

impl CardEffect for OptionMainDraw {
    fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("OptionMain draw 1")
            .process(|ctx: &mut EffectContext| {
                let owner = ctx.player;
                ctx.draw(owner, 1);
            })
            .build()]
    }
}

fn ts_option(id: &str, cost: u16) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.play_cost = cost;
    card.traits = vec!["TS".to_string()];
    card
}

/// Push a fresh Option `card_id` into player 1's reveal pool; return its handle.
fn push_option_to_reveal(
    runner: &mut DebugRunner,
    card_id: &str,
) -> digimon_engine::card_source::CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("known card id");
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, 1, next_idx);
    let handle = card.handle();
    runner.game.revealed_cards.push(card);
    handle
}

// ══════════════════════════════════════════════════════════════════════════════
// Engine-core forks — reveal pool + digivolution / link stack
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn use_option_from_revealed_plays_body_free_and_consumes_only_that_card() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(ts_option("TS-REV", 4))
        .add_card(make_test_card("REV-FILLER", "Filler"))
        .add_card(make_test_card("DECKTOP", "Deck Top"))
        .deck(1, &["DECKTOP"])
        .hand(1, &["SRC"])
        .memory(2)
        .start();
    runner.register_effect("TS-REV", Arc::new(OptionMainDraw));

    // Two cards in the reveal pool; the Option is at index 1.
    let _filler = push_option_to_reveal(&mut runner, "REV-FILLER");
    let opt = push_option_to_reveal(&mut runner, "TS-REV");
    assert_eq!(runner.game.revealed_cards.len(), 2);

    let source_card = runner.game.players[1].hand[0].handle();
    let mem_before = runner.game.memory;
    let hand_before = runner.game.players[1].hand.len();
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 1);
        let result = ctx.use_option_from_revealed(opt, CostDelta::Free);
        assert_eq!(result, OptionPlayResult::Trashed, "standard option resolves");
    }

    // The body ran (drew 1 card) and no use cost (4) was paid.
    assert_eq!(
        runner.game.players[1].hand.len(),
        hand_before + 1,
        "OptionMain body ran (drew 1)"
    );
    assert_eq!(runner.game.memory, mem_before, "use cost NOT paid (free)");

    // The used Option left the reveal pool and disposed to trash; the filler
    // remains in the pool (the enclosing reveal step returns it to deck).
    assert!(
        !runner.game.revealed_cards.iter().any(|c| c.handle() == opt),
        "used option lifted out of reveal pool"
    );
    assert_eq!(
        runner.game.revealed_cards.len(),
        1,
        "only the used option was consumed"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TS-REV"),
        "used option disposed to owner's trash"
    );
}

#[test]
fn use_option_from_own_source_digivolution_card_plays_body_free() {
    let mut runner = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("BASE-3", "Base 3");
            c.card_kind = CardKind::Digimon;
            c.level = Some(3);
            c
        })
        .add_card({
            let mut c = make_test_card("TOP-4", "Top 4");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c
        })
        .add_card(ts_option("TS-STACK", 5))
        .add_card(make_test_card("DECKTOP", "Deck Top"))
        .deck(1, &["DECKTOP"])
        .memory(1)
        .start();
    runner.register_effect("TS-STACK", Arc::new(OptionMainDraw));

    // Build a battle-area Digimon, then slip a [TS] Option into its
    // digivolution stack (below the top card).
    let host = runner.place_stack(1, &["BASE-3", "TOP-4"]);
    let opt = runner.push_source_owned(host, "TS-STACK", 1);
    let source_card = runner.game.players[1].battle_area[host.index as usize]
        .top_card()
        .handle();

    let mem_before = runner.game.memory;
    let hand_before = runner.game.players[1].hand.len();
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(host), 1);
        let result = ctx.use_option_from_source(host, opt, CostDelta::Free);
        assert_eq!(result, OptionPlayResult::Trashed);
    }

    assert_eq!(
        runner.game.players[1].hand.len(),
        hand_before + 1,
        "OptionMain body ran (drew 1)"
    );
    assert_eq!(runner.game.memory, mem_before, "use cost NOT paid (free)");

    // The Option left the digivolution stack and disposed to trash; the host
    // and its remaining stack survive.
    let perm = &runner.game.players[1].battle_area[host.index as usize];
    assert!(
        !perm.card_sources.iter().any(|c| c.handle() == opt),
        "used option lifted out of digivolution stack"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TS-STACK"),
        "used option disposed to trash"
    );
}

#[test]
fn use_option_from_own_source_link_card_plays_body_free() {
    let mut runner = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("BASE-4", "Base 4");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c
        })
        .add_card(ts_option("TS-LINK", 3))
        .add_card(make_test_card("DECKTOP", "Deck Top"))
        .deck(1, &["DECKTOP"])
        .memory(1)
        .start();
    runner.register_effect("TS-LINK", Arc::new(OptionMainDraw));

    let host = runner.place_stack(1, &["BASE-4"]);
    // A linked Option card on the host — include_link_cards union origin.
    let opt = runner.push_linked_owned(host, "TS-LINK", 1);
    let source_card = runner.game.players[1].battle_area[host.index as usize]
        .top_card()
        .handle();

    let mem_before = runner.game.memory;
    let hand_before = runner.game.players[1].hand.len();
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(host), 1);
        let result = ctx.use_option_from_source(host, opt, CostDelta::Free);
        assert_eq!(result, OptionPlayResult::Trashed);
    }

    assert_eq!(
        runner.game.players[1].hand.len(),
        hand_before + 1,
        "OptionMain body ran (drew 1)"
    );
    assert_eq!(runner.game.memory, mem_before, "use cost NOT paid (free)");
    let perm = &runner.game.players[1].battle_area[host.index as usize];
    assert!(
        !perm.linked_cards.iter().any(|c| c.handle() == opt),
        "used option lifted out of link cards"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TS-LINK"),
        "used option disposed to trash"
    );
}

#[test]
fn use_option_from_revealed_invalid_when_card_not_in_pool() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(ts_option("TS-GHOST", 4))
        .hand(1, &["SRC"])
        .memory(2)
        .start();
    runner.register_effect("TS-GHOST", Arc::new(OptionMainDraw));

    // A handle for a card that is NOT in the reveal pool → Invalid, no mutation.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TS-GHOST")
        .unwrap();
    let next_idx = runner.game.next_card_index();
    let phantom = CardSource::new(data_idx, 1, next_idx).handle();

    let source_card = runner.game.players[1].hand[0].handle();
    let before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 1);
        let result = ctx.use_option_from_revealed(phantom, CostDelta::Free);
        assert_eq!(result, OptionPlayResult::Invalid);
    }
    assert_eq!(runner.game.memory, before, "invalid use mutates nothing");
}

// ══════════════════════════════════════════════════════════════════════════════
// DSL-verb proofs (compiled steps driven end-to-end)
// ══════════════════════════════════════════════════════════════════════════════

/// EX7-048 clause-1 shape: `select_reveal { bind_as: picked }` →
/// `use_option_from_revealed { of: you, card: picked }`. Proves the DSL verb
/// resolves the reveal-pool binding, uses the Option free (body runs), and
/// leaves the remaining revealed cards in the pool for the reveal step. Also
/// asserts the parked select clones (make-engine-cloneable §28).
#[test]
fn dsl_use_option_from_revealed_uses_picked_option_free() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC-EX7", "Source"))
        .add_card(ts_option("TS-REV", 4))
        .add_card(make_test_card("REV-DIGI", "RevDigi"))
        .add_card(make_test_card("DECKTOP", "Deck Top"))
        .deck(1, &["DECKTOP"])
        .hand(1, &["SRC-EX7"])
        .memory(2)
        .start();
    runner.register_effect("TS-REV", Arc::new(OptionMainDraw));

    let _digi = push_option_to_reveal(&mut runner, "REV-DIGI");
    let opt = push_option_to_reveal(&mut runner, "TS-REV");
    let src = runner.game.players[1].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectReveal {
            then: vec![],
            of: CompiledPlayerRef::You,
            filter: CompiledPredicate {
                kind: Some(CompiledCardKind::Option),
                trait_has: Some("TS".to_string()),
                ..CompiledPredicate::default()
            },
            bind_as: Some("picked".to_string()),
            prompt: "Pick a TS Option".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::UseOptionFromRevealed {
            of: CompiledPlayerRef::You,
            card: CompiledBindingRef::Named("picked".to_string()),
            cost_delta: None,
        },
    ];

    let hand_before = runner.game.players[1].hand.len();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 1);
        let mut bindings = Bindings::new();
        assert_eq!(run_steps(&steps, &mut ctx, &mut bindings), RunOutcome::Parked);
    }

    // Clone-safety (make-engine-cloneable §28): the parked select carries a
    // data-driven resume frame (SelectReveal), and the `use_option_from_revealed`
    // step installs no bespoke closure — so `Game` clones without hitting the
    // PendingSelection panic-stub. Cloning + resolving the clone must not panic.
    {
        let mut cloned = runner.game.clone();
        let (aid, sp) = {
            let p = cloned
                .pending_selection
                .as_ref()
                .expect("clone kept the select");
            (p.valid_action_ids[0], p.selecting_player)
        };
        cloned
            .resolve_selection(sp, aid)
            .expect("cloned select resolves");
    }

    // Only the TS Option is selectable.
    let (action_id, sp) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("reveal select");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "only the TS option is eligible"
        );
        (pending.valid_action_ids[0], pending.selecting_player)
    };
    runner.game.resolve_selection(sp, action_id).expect("resolve");

    // The option was used free (body drew 1), left the pool, and disposed to
    // trash. The non-option revealed card stays in the pool.
    assert_eq!(
        runner.game.players[1].hand.len(),
        hand_before + 1,
        "used option's [Main] body ran (drew 1)"
    );
    assert!(
        !runner.game.revealed_cards.iter().any(|c| c.handle() == opt),
        "used option left the reveal pool"
    );
    assert_eq!(
        runner.game.revealed_cards.len(),
        1,
        "the non-option revealed card remains for the reveal step"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TS-REV"),
        "used option disposed to trash"
    );
}

/// EX7-048 final clause-1 shape — the CardList-binding REGRESSION test.
///
/// `select_reveal_buckets` (min:0/max:1) binds its pick as
/// `ResolvedBinding::CardList` via `insert_card_list`
/// (`run_reveal_bucket_terminal`, selections.rs) — NOT the singular
/// `ResolvedBinding::Card` a bare `select_reveal` produces. Before the
/// executor widening, `CompiledStep::UseOptionFromRevealed` matched ONLY the
/// `Card` shape, so the bucket-bound pick fell through the match and the
/// Option was silently never used (the central EX7-048 payoff no-oped).
/// This proves the CardList shape uses the Option end-to-end, mirroring
/// `PlayFromRevealedFree`'s dual-shape handling.
#[test]
fn dsl_use_option_from_revealed_accepts_bucket_cardlist_binding() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC-EX7", "Source"))
        .add_card(ts_option("TS-REV", 4))
        .add_card(make_test_card("REV-DIGI", "RevDigi"))
        .add_card(make_test_card("DECKTOP", "Deck Top"))
        .deck(1, &["DECKTOP"])
        .hand(1, &["SRC-EX7"])
        .memory(2)
        .start();
    runner.register_effect("TS-REV", Arc::new(OptionMainDraw));

    let digi = push_option_to_reveal(&mut runner, "REV-DIGI");
    let opt = push_option_to_reveal(&mut runner, "TS-REV");
    let src = runner.game.players[1].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectRevealBuckets {
            from: "revealed".to_string(),
            buckets: vec![CompiledRevealBucket {
                bind_as: "picked".to_string(),
                filter: Some(CompiledPredicate {
                    kind: Some(CompiledCardKind::Option),
                    trait_has: Some("TS".to_string()),
                    ..CompiledPredicate::default()
                }),
                min: 0,
                max: 1,
            }],
            no_duplicate_cards: true,
            prompt: Some("You may use 1 TS Option".to_string()),
        },
        CompiledStep::UseOptionFromRevealed {
            of: CompiledPlayerRef::You,
            card: CompiledBindingRef::Named("picked".to_string()),
            cost_delta: None,
        },
    ];

    let hand_before = runner.game.players[1].hand.len();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 1);
        let mut bindings = Bindings::new();
        bindings.insert_card_list("revealed", vec![digi, opt]);
        assert_eq!(run_steps(&steps, &mut ctx, &mut bindings), RunOutcome::Parked);
    }

    // Clone-safety (make-engine-cloneable §28): the parked bucket carries a
    // data-driven RevealBucketStep resume frame — cloning + resolving the
    // clone must not hit the PendingSelection panic-stub.
    {
        let mut cloned = runner.game.clone();
        let (aid, sp) = {
            let p = cloned
                .pending_selection
                .as_ref()
                .expect("clone kept the bucket select");
            (p.valid_action_ids[0], p.selecting_player)
        };
        cloned
            .resolve_selection(sp, aid)
            .expect("cloned bucket select resolves");
    }

    // Only the TS Option (reveal index 1) is a legal bucket pick.
    let (action_id, sp) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("bucket select parked");
        assert!(
            matches!(
                pending.kind,
                digimon_engine::selection::SelectionKind::RevealBucket { .. }
            ),
            "bucket step parks a RevealBucket selection"
        );
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "only the TS option is eligible"
        );
        (pending.valid_action_ids[0], pending.selecting_player)
    };
    runner.game.resolve_selection(sp, action_id).expect("pick");

    // The CardList-bound option was used free: body ran (drew 1), it left the
    // reveal pool, and disposed to trash — NOT silently skipped.
    assert_eq!(
        runner.game.players[1].hand.len(),
        hand_before + 1,
        "used option's [Main] body ran (drew 1) — the CardList binding was consumed"
    );
    assert!(
        !runner.game.revealed_cards.iter().any(|c| c.handle() == opt),
        "used option left the reveal pool"
    );
    assert_eq!(
        runner.game.revealed_cards.len(),
        1,
        "the non-option revealed card remains for the reveal step"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TS-REV"),
        "used option disposed to trash"
    );
    assert_eq!(runner.game.memory, 2, "used free — memory unchanged");
}

/// Decline shape for the bucket binding: PASS on the min:0 bucket binds
/// `picked` as an EMPTY CardList — `use_option_from_revealed` must no-op
/// cleanly (no panic, nothing consumed) and the tail after it must still run.
#[test]
fn dsl_use_option_from_revealed_empty_bucket_cardlist_noops() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC-EX7", "Source"))
        .add_card(ts_option("TS-REV", 4))
        .add_card(make_test_card("DECKTOP", "Deck Top"))
        .deck(1, &["DECKTOP"])
        .hand(1, &["SRC-EX7"])
        .memory(2)
        .start();
    runner.register_effect("TS-REV", Arc::new(OptionMainDraw));

    let opt = push_option_to_reveal(&mut runner, "TS-REV");
    let src = runner.game.players[1].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectRevealBuckets {
            from: "revealed".to_string(),
            buckets: vec![CompiledRevealBucket {
                bind_as: "picked".to_string(),
                filter: Some(CompiledPredicate {
                    kind: Some(CompiledCardKind::Option),
                    ..CompiledPredicate::default()
                }),
                min: 0,
                max: 1,
            }],
            no_duplicate_cards: true,
            prompt: Some("You may use 1 Option".to_string()),
        },
        CompiledStep::UseOptionFromRevealed {
            of: CompiledPlayerRef::You,
            card: CompiledBindingRef::Named("picked".to_string()),
            cost_delta: None,
        },
        // The tail after the (declined) use must still run — the EX7-048
        // remainder-placement guarantee.
        CompiledStep::GainMemory(1),
    ];

    let hand_before = runner.game.players[1].hand.len();
    let mem_before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 1);
        let mut bindings = Bindings::new();
        bindings.insert_card_list("revealed", vec![opt]);
        assert_eq!(run_steps(&steps, &mut ctx, &mut bindings), RunOutcome::Parked);
    }

    let sp = runner
        .game
        .pending_selection
        .as_ref()
        .expect("bucket select parked")
        .selecting_player;
    runner
        .game
        .resolve_selection(sp, digimon_engine::action::space::PASS)
        .expect("decline the optional pick");

    assert!(runner.game.pending_selection.is_none(), "resolved");
    assert_eq!(
        runner.game.players[1].hand.len(),
        hand_before,
        "declined: the option's body did NOT run"
    );
    assert!(
        runner.game.revealed_cards.iter().any(|c| c.handle() == opt),
        "declined: the option stays in the reveal pool"
    );
    // The shared memory gauge is player-0-perspective: player 1 gaining 1
    // memory moves it NEGATIVE.
    assert_eq!(
        runner.game.memory,
        mem_before - 1,
        "the step tail after the declined use still runs"
    );
}

/// A synthetic Option mirroring EX7-070 Der Blitz's [Main] shape: a body that
/// PARKS MID-BODY (delete pick inside an `if`) with a second `if` leg still to
/// run after it, whose tail relocates the Option itself
/// (`place_self_under_permanent`).
const MID_PARK_OPTION: &str = r#"
card: TST-OPT-MIDPARK
name: MidParkOption
kind: option
color: [black]
cost: 4
traits: [TS]
effects:
  - when: main_from_hand
    summary: "[Main] Delete 1 opp Digimon; then place this card under 1 of your TS Digimon"
    process:
      - if:
          condition:
            any_permanent: { of: opponent, zone: [battle_area], kind: digimon }
          then:
            - select_opponent_permanent:
                bind_as: t
                filter: { kind: digimon }
                prompt: "Delete 1 opponent Digimon"
            - delete_permanent: { target: t }
      - if:
          condition:
            any_permanent: { of: you, zone: [battle_area], kind: digimon, trait_has: TS }
          then:
            - select_own_permanent:
                bind_as: p
                filter:
                  all_of:
                    - kind: digimon
                    - trait_has: TS
                prompt: "Place this card under 1 of your TS Digimon"
            - place_self_under_permanent: { target: p, face_down: false }
"#;

/// G-DSL-OUTER-TAIL-NESTED-PARK, effect-driven Option-USE facet (the EX7-048 +
/// EX7-070 end-to-end bug). A used Option whose body parks MID-BODY leaves its
/// remaining steps in the global `dsl_outer_tail` slot; without the
/// `compose_parked_used_option_body_tail` re-composition, the outer clause's
/// tail (here `gain_memory`) steals that slot's sequencing position — the
/// outer tail resolves in the MIDDLE of the Option's [Main], and the stolen
/// body tail runs under the OUTER card's provenance, so
/// `place_self_under_permanent` fails its `pending_option` claim and the
/// Option mis-disposes to trash. This proves both fix facets:
///   1. ordering — the outer tail does NOT run until the Option's [Main]
///      fully resolves (memory unchanged while the place pick is pending);
///   2. provenance — the Option seats itself under the chosen host (ends in
///      `card_sources`, NOT in trash).
#[test]
fn dsl_use_option_from_revealed_mid_body_park_resolves_before_outer_tail() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(MID_PARK_OPTION)
        .unwrap()
        .add_card(make_test_card("SRC-EX7", "Source"))
        .add_card({
            let mut c = make_test_card("TS-HOST", "TS Host");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.traits = vec!["TS".to_string()];
            c
        })
        .add_card({
            let mut c = make_test_card("OPP-DIGI", "Opp Digi");
            c.card_kind = CardKind::Digimon;
            c.level = Some(3);
            c.play_cost = 3;
            c
        })
        .add_card(make_test_card("DECKTOP", "Deck Top"))
        .deck(1, &["DECKTOP"])
        .hand(1, &["SRC-EX7"])
        .memory(2)
        .start();

    let host = runner.place_on_field(1, "TS-HOST", Some(0));
    let _opp = runner.place_on_field(0, "OPP-DIGI", Some(0));
    let opt = push_option_to_reveal(&mut runner, "TST-OPT-MIDPARK");
    let src = runner.game.players[1].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectRevealBuckets {
            from: "revealed".to_string(),
            buckets: vec![CompiledRevealBucket {
                bind_as: "picked".to_string(),
                filter: Some(CompiledPredicate {
                    kind: Some(CompiledCardKind::Option),
                    ..CompiledPredicate::default()
                }),
                min: 0,
                max: 1,
            }],
            no_duplicate_cards: true,
            prompt: Some("You may use 1 Option".to_string()),
        },
        CompiledStep::UseOptionFromRevealed {
            of: CompiledPlayerRef::You,
            card: CompiledBindingRef::Named("picked".to_string()),
            cost_delta: None,
        },
        // Stand-in for EX7-048's mandatory remainder placement: an outer-tail
        // step that must NOT run until the Option's [Main] fully resolves.
        CompiledStep::GainMemory(1),
    ];

    let mem_before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 1);
        let mut bindings = Bindings::new();
        bindings.insert_card_list("revealed", vec![opt]);
        assert_eq!(run_steps(&steps, &mut ctx, &mut bindings), RunOutcome::Parked);
    }

    // 1. Bucket pick — take the Option.
    let (aid, sp) = {
        let p = runner.game.pending_selection.as_ref().expect("bucket parked");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("pick the option");

    // 2. The Option's [Main] first leg: the mandatory delete pick.
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("the option's delete pick parks");
        assert_eq!(
            p.prompt, "Delete 1 opponent Digimon",
            "the option's [Main] body is resolving"
        );
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("delete");

    // 3. ORDERING: the option's SECOND leg (place-under pick) must park next —
    //    the outer tail must not have run yet.
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("the option's place-under pick parks BEFORE the outer tail runs");
        assert_eq!(
            p.prompt, "Place this card under 1 of your TS Digimon",
            "the mid-body-parked second leg resolves in order"
        );
        (p.valid_action_ids[0], p.selecting_player)
    };
    assert_eq!(
        runner.game.memory, mem_before,
        "the outer tail (gain_memory) must NOT run while the option's [Main] is still resolving"
    );
    runner.game.resolve_selection(sp, aid).expect("place under host");

    assert!(runner.game.pending_selection.is_none(), "flow resolved");
    // PROVENANCE: the option seated itself under the host (claim succeeded)…
    let host_perm = &runner.game.players[1].battle_area[host.index as usize];
    assert_eq!(
        host_perm
            .card_sources
            .first()
            .map(|c| c.card_id(&runner.game.card_data)),
        Some("TST-OPT-MIDPARK"),
        "the used option placed ITSELF as the host's bottom digivolution card"
    );
    assert!(
        !runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TST-OPT-MIDPARK"),
        "the used option was NOT mis-disposed to trash"
    );
    // …the delete leg resolved…
    assert!(
        runner.game.players[0].battle_area.is_empty(),
        "the opponent Digimon was deleted by the option's [Main]"
    );
    // …and the outer tail ran exactly once, AFTER the option resolved.
    assert_ne!(
        runner.game.memory, mem_before,
        "the outer tail (gain_memory) ran after the option's [Main] completed"
    );
}

/// BT21-062 shape: `select_union_zone { zones: [hand, trash] }` →
/// `use_option_bound { binding }`. Proves the DSL verb consumes the union
/// binding and uses the picked Option free from its true origin zone (here:
/// trash).
#[test]
fn dsl_use_option_bound_uses_union_picked_option_free_from_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC-BT21", "Source"))
        .add_card({
            let mut c = ts_option("RAGNAROK", 6);
            c.traits = vec!["Ragnarok".to_string()];
            c
        })
        .add_card(make_test_card("DECKTOP", "Deck Top"))
        .deck(1, &["DECKTOP"])
        .hand(1, &["SRC-BT21"])
        .memory(3)
        .start();
    runner.register_effect("RAGNAROK", Arc::new(OptionMainDraw));

    // The Ragnarok Cannon lives in trash (the trash leg of the union).
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "RAGNAROK")
        .unwrap();
    let next_idx = runner.game.next_card_index();
    let ragnarok = CardSource::new(data_idx, 1, next_idx);
    let ragnarok_handle = ragnarok.handle();
    runner.game.players[1].trash.push(ragnarok);
    let src = runner.game.players[1].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectUnionZone {
            of: CompiledPlayerRef::You,
            zones: vec![CompiledZone::Hand, CompiledZone::Trash],
            material_of: None,
            filter: CompiledPredicate {
                kind: Some(CompiledCardKind::Option),
                ..CompiledPredicate::default()
            },
            zone_filters: Default::default(),
            material_carrier_filter: None,
            bind_as: Some("bound".to_string()),
            prompt: "Pick a Ragnarok Cannon from hand or trash".to_string(),
            prompt_key: None,
            optional: false,
            cost: false,
            then: vec![],
        },
        CompiledStep::UseOptionBound {
            binding: "bound".to_string(),
            cost_delta: None,
        },
    ];

    let hand_before = runner.game.players[1].hand.len();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 1);
        let mut bindings = Bindings::new();
        assert_eq!(run_steps(&steps, &mut ctx, &mut bindings), RunOutcome::Parked);
    }

    let (action_id, sp) = {
        let pending = runner.game.pending_selection.as_ref().expect("union select");
        (pending.valid_action_ids[0], pending.selecting_player)
    };
    runner.game.resolve_selection(sp, action_id).expect("resolve");

    assert_eq!(
        runner.game.players[1].hand.len(),
        hand_before + 1,
        "bound option's [Main] body ran (drew 1)"
    );
    // A Standard Option lifts OUT of trash into `pending_option`, resolves, then
    // disposes back to trash — so the SAME handle round-trips to trash. The
    // proof of a real USE (not a trivial no-op) is the body having run (hand
    // grew above) and the use being free (memory unchanged below).
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.handle() == ragnarok_handle),
        "used option disposed back to trash (round-trip)"
    );
    assert_eq!(runner.game.memory, 3, "used free — memory unchanged");
}

/// BT25-085 clause-1 shape: `select_own_sources { include_link_cards: true }` →
/// `use_option_from_sources { of: you, card: picked }`. Proves the DSL verb
/// consumes a source-refs binding over a permanent's digivolution stack and
/// uses the Option free (body runs), lifting it out of the stack.
#[test]
fn dsl_use_option_from_sources_uses_picked_source_option_free() {
    let mut runner = DebugRunner::builder()
        .add_card({
            let mut c = make_test_card("BASE-3", "Base 3");
            c.card_kind = CardKind::Digimon;
            c.level = Some(3);
            c
        })
        .add_card({
            let mut c = make_test_card("TOP-4", "Top 4");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.traits = vec!["TS".to_string()];
            c
        })
        .add_card(ts_option("TS-STACK", 5))
        .add_card(make_test_card("DECKTOP", "Deck Top"))
        .deck(1, &["DECKTOP"])
        .memory(1)
        .start();
    runner.register_effect("TS-STACK", Arc::new(OptionMainDraw));

    // Host with a [TS] Option in its digivolution stack (below the top).
    let host = runner.place_stack(1, &["BASE-3", "TOP-4"]);
    let opt = runner.push_source_owned(host, "TS-STACK", 1);
    let src = runner.game.players[1].battle_area[host.index as usize]
        .top_card()
        .handle();

    let steps = vec![
        CompiledStep::SelectOwnSources {
            target: None,
            filter: CompiledPredicate {
                kind: Some(CompiledCardKind::Option),
                trait_has: Some("TS".to_string()),
                ..CompiledPredicate::default()
            },
            min: 1,
            max: 1,
            bind_as: Some("picked".to_string()),
            prompt: "Pick a TS Option from your digivolution cards".to_string(),
            then: vec![],
        },
        CompiledStep::UseOptionFromSources {
            of: CompiledPlayerRef::You,
            card: CompiledBindingRef::Named("picked".to_string()),
            cost_delta: None,
        },
    ];

    let hand_before = runner.game.players[1].hand.len();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(host), 1);
        let mut bindings = Bindings::new();
        assert_eq!(run_steps(&steps, &mut ctx, &mut bindings), RunOutcome::Parked);
    }

    let (action_id, sp) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("source select");
        (pending.valid_action_ids[0], pending.selecting_player)
    };
    runner.game.resolve_selection(sp, action_id).expect("resolve");

    assert_eq!(
        runner.game.players[1].hand.len(),
        hand_before + 1,
        "used source option's [Main] body ran (drew 1)"
    );
    let perm = &runner.game.players[1].battle_area[host.index as usize];
    assert!(
        !perm.card_sources.iter().any(|c| c.handle() == opt),
        "used option lifted out of digivolution stack"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TS-STACK"),
        "used option disposed to trash"
    );
    assert_eq!(runner.game.memory, 1, "used free — memory unchanged");
}
