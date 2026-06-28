//! BT25-063 Commandramon — Digimon, Lv.3, Black/Purple, DP 2000, Cost 3.
//! Traits: Cyborg, D-Brigade, ACCEL. Attribute: Virus.
//!
//! Printed effect text (cards.json):
//!   [When Moving] [On Play] Reveal the top 3 cards of your deck. Add 1 card
//!   with [Chaosmon] in its name or the [D-Brigade] or [ACCEL] trait among them
//!   to the hand. Return the rest to the top or bottom of the deck.
//!
//! Printed inherited text:
//!   [All Turns] This Digimon gets +1000 DP.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_063.cs
//!
//! Pattern rows (RUST_DSL_TEST_API §4.3): A1/A2 reveal-N single-bucket add by
//! name-or-trait, order_remainder top-OR-bottom player choice (Group E),
//! shared [When Moving][On Play] trigger, D1 inherited All-Turns DP aura,
//! two alt-digivolve recipes ([Missimon] name + Lv.2 [ACCEL]).

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledRevealDestination,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::game::Game;

/// Drive a raw `Game` (e.g. a clone) through every parked selection by always
/// taking the first non-PASS valid action, until no selection remains.
/// Deterministic, so the original and a clone driven the same way reach the
/// same state.
fn drive_to_completion(game: &mut Game, player: u8) {
    while game.pending_selection.is_some() {
        let id = {
            let pend = game.pending_selection.as_ref().unwrap();
            pend.valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .or_else(|| pend.valid_action_ids.first().copied())
                .expect("a resolvable action exists")
        };
        game.resolve_selection(player, id)
            .expect("parked selection resolves");
    }
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT25-063")
        .expect("BT25-063 YAML parses and compiles")
        .build()
}

fn stack_deck_top(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered"));
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .deck
            .push(CardSource::new(data_idx, 0, card_index));
    }
}

fn shared_process(runner: &DebugRunner) -> Vec<CompiledStep> {
    runner
        .compiled_card("BT25-063")
        .expect("compiled")
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => {
                Some(t.process.clone())
            }
            _ => None,
        })
        .expect("shared reveal clause present")
}

// ── Section 1: Structural ──────────────────────────────────────────────────

#[test]
fn bt25_063_shared_clause_is_when_moving_and_on_play() {
    let runner = runner();
    let card = runner.compiled_card("BT25-063").expect("compiled");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-063 must have a face [On Play] clause");

    assert!(
        clause.when.contains(&CompiledTiming::OnMove),
        "shared clause must also fire [When Moving] (on_move)"
    );
    // DCGO optional: false — the search is mandatory (no "you may").
    assert!(
        !clause.optional,
        "search clause is mandatory (DCGO optional: false)"
    );
}

#[test]
fn bt25_063_clause_reveals_adds_to_hand_and_orders_remainder() {
    let runner = runner();
    let process = shared_process(&runner);

    let has_reveal = process
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { .. }));
    let hand_dest = process.iter().any(|s| {
        matches!(
            s,
            CompiledStep::ChooseFromReveal {
                destination: CompiledRevealDestination::Hand,
                ..
            }
        )
    });
    // order_remainder with two destinations = player picks deck top OR bottom.
    let order_two_dest = process
        .iter()
        .filter_map(|s| match s {
            CompiledStep::OrderRemainder { destinations, .. } => Some(destinations.len()),
            _ => None,
        })
        .any(|n| n == 2);

    assert!(has_reveal, "must reveal top 3");
    assert!(hand_dest, "the single pick routes to hand");
    assert!(
        order_two_dest,
        "remainder must offer top-OR-bottom (order_remainder with 2 destinations)"
    );
}

#[test]
fn bt25_063_has_inherited_all_turns_dp_aura() {
    let runner = runner();
    let card = runner.compiled_card("BT25-063").expect("compiled");
    let aura = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Inherited,
                dp_modifier: Some(1000),
                ..
            })
        )
    });
    assert!(aura, "BT25-063 must declare inherited +1000 DP aura");
}

#[test]
fn bt25_063_registers_two_alt_digivolves() {
    let runner = runner();
    let card = runner.compiled_card("BT25-063").expect("compiled");
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(
        digivolve_paths, 2,
        "BT25-063 must register [Missimon]-name and Lv.2 [ACCEL] alt digivolve paths"
    );
}

// ── Section 3: Behavioral — add by name OR trait ───────────────────────────

#[test]
fn bt25_063_adds_accel_trait_card_to_hand() {
    let mut accel = make_test_card("ACCEL-A", "Accel A");
    accel.traits.push("ACCEL".to_string());
    let fa = make_test_card("F063-A", "f a");
    let fb = make_test_card("F063-B", "f b");
    let holder = make_test_card("BT25_063_HOLDER", "Holder");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-063")
        .expect("BT25-063 compiles")
        .add_card(accel)
        .add_card(fa)
        .add_card(fb)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_063_HOLDER", None);
    let src = runner.top_card(holder_perm);
    let deck_before = runner.game.players[0].deck.len();
    stack_deck_top(&mut runner, &["F063-B", "F063-A", "ACCEL-A"]);

    let process = shared_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(holder_perm), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    runner.auto_resolve().expect("pick + order remainder resolve");

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(hand_ids.contains(&"ACCEL-A"), "ACCEL-trait card added to hand: {hand_ids:?}");
    // One card left the deck; the other two were ordered back onto the deck.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 2,
        "one pick to hand, two returned to deck"
    );
}

#[test]
fn bt25_063_adds_chaosmon_name_card_to_hand() {
    // Name match (no D-Brigade / ACCEL trait) — only the name predicate hits.
    let chaos = make_test_card("CHAOS-A", "Chaosmon Variant");
    let fa = make_test_card("F063-C", "f c");
    let fb = make_test_card("F063-D", "f d");
    let holder = make_test_card("BT25_063_HOLDER2", "Holder2");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-063")
        .expect("BT25-063 compiles")
        .add_card(chaos)
        .add_card(fa)
        .add_card(fb)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_063_HOLDER2", None);
    let src = runner.top_card(holder_perm);
    stack_deck_top(&mut runner, &["F063-D", "F063-C", "CHAOS-A"]);

    let process = shared_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(holder_perm), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    runner.auto_resolve().expect("pick + order remainder resolve");

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"CHAOS-A"),
        "[Chaosmon]-in-name card added to hand: {hand_ids:?}"
    );
}

// ── Section 4: Clone-safety (make-engine-cloneable) ────────────────────────

/// The multi-destination `order_remainder` (deck top OR bottom) surfaces a
/// destination effect-choice that then installs the ordered-permutation pick —
/// a frame-installs-frame selection. Cloning the game AT the destination choice
/// must be faithful: the choice is resume-driven (not a closure that clones to
/// the panic-stub), and resolving the clone leaves the original untouched and
/// reaches identical state on replay.
#[test]
fn bt25_063_order_remainder_dest_choice_clones_faithfully() {
    let mut accel = make_test_card("ACCEL-CLN", "Accel Cln");
    accel.traits.push("ACCEL".to_string());
    let fa = make_test_card("F063-CLN-A", "f cln a");
    let fb = make_test_card("F063-CLN-B", "f cln b");
    let holder = make_test_card("BT25_063_HOLDER_CLN", "HolderCln");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-063")
        .expect("BT25-063 compiles")
        .add_card(accel)
        .add_card(fa)
        .add_card(fb)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_063_HOLDER_CLN", None);
    let src = runner.top_card(holder_perm);
    let deck_before = runner.game.players[0].deck.len();
    // Top of deck = ACCEL-CLN (the match → goes to hand); F063-CLN-A/B are the
    // two-card remainder ordered back onto the deck.
    stack_deck_top(&mut runner, &["F063-CLN-B", "F063-CLN-A", "ACCEL-CLN"]);

    let process = shared_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(holder_perm), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }

    // Resolve the reveal pick (ACCEL-CLN → hand). The next installed selection
    // is the multi-dest order_remainder DESTINATION choice (top vs bottom).
    let pick_view = runner.pending_selection_view().expect("reveal pick installs");
    let pick = pick_view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .unwrap();
    runner.execute_action(0, pick).expect("add ACCEL-CLN to hand");

    assert!(
        runner.game.pending_selection.is_some(),
        "multi-dest order_remainder installs a destination choice"
    );
    assert!(
        runner.game.pending_selection_resume.is_some(),
        "the destination choice must be resume-driven (clone-safe), not closure-only"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "remainder still in the reveal pool — not yet placed on the deck"
    );

    // Clone AT the destination choice (the operation MCTS/AlphaZero performs),
    // and drive the clone to completion.
    let mut clone = runner.game.clone();
    drive_to_completion(&mut clone, 0);
    assert!(clone.pending_selection.is_none(), "clone fully resolves");
    assert_eq!(
        clone.players[0].deck.len(),
        deck_before + 2,
        "clone: both remainder cards returned to the deck"
    );

    // INDEPENDENCE: the original is untouched by resolving the clone.
    assert!(
        runner.game.pending_selection.is_some(),
        "original survives cloning + resolving the clone"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "original deck unchanged while the clone resolved"
    );

    // REPLAYS IDENTICALLY: driving the original the same way reaches the clone's
    // deck state.
    drive_to_completion(&mut runner.game, 0);
    assert_eq!(
        runner.game.players[0].deck.len(),
        clone.players[0].deck.len(),
        "original reaches the same deck size as the clone"
    );
}

#[test]
fn bt25_063_no_match_returns_all_to_deck() {
    let fa = make_test_card("F063-E", "f e");
    let fb = make_test_card("F063-F", "f f");
    let fc = make_test_card("F063-G", "f g");
    let holder = make_test_card("BT25_063_HOLDER3", "Holder3");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-063")
        .expect("BT25-063 compiles")
        .add_card(fa)
        .add_card(fb)
        .add_card(fc)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_063_HOLDER3", None);
    let src = runner.top_card(holder_perm);
    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();
    stack_deck_top(&mut runner, &["F063-G", "F063-F", "F063-E"]);

    let process = shared_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(holder_perm), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "no eligible card → nothing added to hand"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 3,
        "all 3 revealed cards return to deck"
    );
    assert!(runner.pending_selection().is_none(), "no pending selection remains");
    assert_eq!(runner.game.revealed_cards.len(), 0, "reveal pool empty");
}
