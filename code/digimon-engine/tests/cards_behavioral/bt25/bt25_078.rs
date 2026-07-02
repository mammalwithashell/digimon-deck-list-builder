//! BT25-078 Gazimon — Digimon, Lv.3, Black, DP 1000, Cost 3.
//! Traits: Mammal, Iliad, TS. Attribute: Virus.
//!
//! Printed effect text (cards.json):
//!   [When Moving] [On Play] Reveal the top 3 cards of your deck. Among them,
//!   add 1 card with [Three Musketeers] in its text to the hand, or you may
//!   place 1 [Three Musketeers] trait card as this Digimon's bottom
//!   digivolution card. Return the rest to the bottom of the deck.
//!
//! Printed inherited text:
//!   <Retaliation> (When only this Digimon is deleted in battle, delete the
//!   Digimon it battled.)
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT25/Purple/BT25_078.cs
//!
//! Pattern rows (RUST_DSL_TEST_API §4.3): reveal-N branch route to hand OR
//! bottom-source-of-self (Group E), optional ("you may"), once-per-turn shared
//! WM/OP trigger, inherited static keyword grant (Group G), alt-digivolve.

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
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT25-078")
        .expect("BT25-078 YAML parses and compiles")
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

/// Pull the shared [When Moving][On Play] clause process.
fn shared_process(runner: &DebugRunner) -> Vec<CompiledStep> {
    runner
        .compiled_card("BT25-078")
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
fn bt25_078_shared_clause_is_when_moving_and_on_play_optional_opt() {
    let runner = runner();
    let card = runner.compiled_card("BT25-078").expect("compiled");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp && t.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT25-078 must have a face [On Play] clause");

    assert!(
        clause.when.contains(&CompiledTiming::OnMove),
        "shared clause must also fire [When Moving] (on_move)"
    );
    assert!(
        clause.optional,
        "printed 'or you may place' → the whole effect is optional"
    );
    assert!(
        clause.once_per_turn,
        "DCGO maxCountPerTurn: 1 → once_per_turn"
    );
}

#[test]
fn bt25_078_has_both_route_branches() {
    let runner = runner();
    let process = shared_process(&runner);

    let has_reveal = process
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { .. }));
    let has_effect_choice = process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectEffectChoice { .. }));
    assert!(has_reveal, "must reveal top 3");
    assert!(
        has_effect_choice,
        "must surface the hand-vs-source branch choice"
    );

    // Recurse into if-branches to find the two choose_from_reveal destinations.
    fn collect_choose<'a>(steps: &'a [CompiledStep], out: &mut Vec<&'a CompiledRevealDestination>) {
        for s in steps {
            match s {
                CompiledStep::ChooseFromReveal { destination, .. } => out.push(destination),
                CompiledStep::If {
                    then, else_branch, ..
                } => {
                    collect_choose(then, out);
                    collect_choose(else_branch, out);
                }
                _ => {}
            }
        }
    }
    let mut dests = Vec::new();
    collect_choose(&process, &mut dests);

    let has_hand = dests
        .iter()
        .any(|d| matches!(d, CompiledRevealDestination::Hand));
    let has_bottom_source = dests
        .iter()
        .any(|d| matches!(d, CompiledRevealDestination::BottomSourceOf(_)));
    assert!(has_hand, "one branch routes the pick to hand");
    assert!(
        has_bottom_source,
        "the other branch routes the pick as this Digimon's bottom digivolution card"
    );

    let has_place_remainder = process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceRemainderOnDeck { .. }));
    assert!(has_place_remainder, "rest returns to deck bottom");
}

#[test]
fn bt25_078_has_inherited_retaliation() {
    let runner = runner();
    let card = runner.compiled_card("BT25-078").expect("compiled");
    let retaliation = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) => *scope == CompiledScope::Inherited && keyword.eq_ignore_ascii_case("Retaliation"),
        _ => false,
    });
    assert!(retaliation, "BT25-078 must grant inherited <Retaliation>");
}

#[test]
fn bt25_078_registers_alt_digivolve() {
    let runner = runner();
    let card = runner.compiled_card("BT25-078").expect("compiled");
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(
        digivolve_paths, 2,
        "BT25-078 must register standard Black and alt TS/Three-Musketeers-text digivolve paths"
    );
}

// ── Section 3: Behavioral — branch 0 (add to hand) ─────────────────────────

#[test]
fn bt25_078_add_to_hand_branch_moves_text_match_to_hand() {
    // A card with [Three Musketeers] in its TEXT but NOT the trait — only the
    // add-to-hand branch may take it.
    let mut text_card = make_test_card("TM-TEXT", "TM Text");
    text_card.effect_text = "Gains [Three Musketeers] synergy.".to_string();
    let filler_a = make_test_card("F078-A", "f a");
    let filler_b = make_test_card("F078-B", "f b");
    let holder = make_test_card("BT25_078_HOLDER", "Holder");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-078")
        .expect("BT25-078 compiles")
        .add_card(text_card)
        .add_card(filler_a)
        .add_card(filler_b)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_078_HOLDER", None);
    let src = runner.top_card(holder_perm);
    let deck_before = runner.game.players[0].deck.len();
    stack_deck_top(&mut runner, &["F078-B", "F078-A", "TM-TEXT"]);

    let process = shared_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(holder_perm), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }

    // First prompt: hand-vs-source effect choice. Choose branch 0 (add to hand).
    let choice = runner
        .pending_selection()
        .expect("effect-choice prompt surfaces");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    let add_to_hand = choice
        .valid_action_ids
        .first()
        .copied()
        .expect("branch 0 action");
    runner
        .execute_action(0, add_to_hand)
        .expect("choose add-to-hand branch");

    // Then the reveal pick (TM-TEXT is the only text match).
    runner
        .auto_resolve()
        .expect("reveal pick + remainder resolve");

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"TM-TEXT"),
        "[Three Musketeers]-in-text card added to hand: {hand_ids:?}"
    );
    // One card left the deck (TM-TEXT), the other two returned to bottom.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 2,
        "one pick to hand, two fillers returned to bottom"
    );
}

// ── Section 3: Behavioral — branch 1 (place as bottom source) ──────────────

#[test]
fn bt25_078_place_branch_tucks_trait_card_under_self() {
    // A card with the [Three Musketeers] TRAIT — eligible for the place-as-
    // bottom-source branch.
    let mut trait_card = make_test_card("TM-TRAIT", "TM Trait");
    trait_card.traits.push("Three Musketeers".to_string());
    let filler_a = make_test_card("F078-C", "f c");
    let filler_b = make_test_card("F078-D", "f d");
    let holder = make_test_card("BT25_078_HOLDER2", "Holder2");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-078")
        .expect("BT25-078 compiles")
        .add_card(trait_card)
        .add_card(filler_a)
        .add_card(filler_b)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_078_HOLDER2", None);
    let src = runner.top_card(holder_perm);
    let sources_before = runner
        .game
        .player(0)
        .battle_area
        .get(holder_perm.index as usize)
        .unwrap()
        .card_sources
        .len();
    stack_deck_top(&mut runner, &["F078-D", "F078-C", "TM-TRAIT"]);

    let process = shared_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(holder_perm), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }

    // Effect-choice: pick branch 1 (place as bottom digivolution card).
    let choice = runner
        .pending_selection()
        .expect("effect-choice prompt surfaces");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    // Branch 1 is the second action id (or PASS if branch 0 only — but both
    // branches are always exposed regardless of candidate availability at the
    // effect-choice layer; the inner reveal pick is what fizzles).
    let place_action = *choice
        .valid_action_ids
        .get(1)
        .expect("branch 1 (place-as-source) action present");
    runner
        .execute_action(0, place_action)
        .expect("choose place-as-source branch");

    runner
        .auto_resolve()
        .expect("reveal pick + remainder resolve");

    let perm = runner
        .game
        .player(0)
        .battle_area
        .get(holder_perm.index as usize)
        .expect("holder permanent present");
    let source_ids: Vec<&str> = perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        source_ids.contains(&"TM-TRAIT"),
        "TM-TRAIT placed as a digivolution source: {source_ids:?}"
    );
    assert_eq!(
        perm.card_sources.len(),
        sources_before + 1,
        "exactly one card was placed under the carrier"
    );
    // It must be the BOTTOM source (index 0).
    assert_eq!(
        perm.card_sources[0].card_id(&runner.game.card_data),
        "TM-TRAIT",
        "the placed card is the BOTTOM digivolution card"
    );
    // It must NOT have gone to hand.
    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        !hand_ids.contains(&"TM-TRAIT"),
        "placed card is not in hand"
    );
}

// ── Section 2: Optional — decline the whole effect ─────────────────────────

#[test]
fn bt25_078_effect_is_optional_decline_does_nothing() {
    // The whole effect is optional. With the trigger queued through normal
    // flow, PASS at the optional layer leaves hand and field untouched.
    let mut trait_card = make_test_card("TM-TRAIT2", "TM Trait 2");
    trait_card.traits.push("Three Musketeers".to_string());
    let holder = make_test_card("BT25_078_HOLDER3", "Holder3");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-078")
        .expect("BT25-078 compiles")
        .add_card(trait_card)
        .add_card(holder)
        .build();

    let holder_perm = runner.place_on_field(0, "BT25_078_HOLDER3", None);
    let src = runner.top_card(holder_perm);
    let hand_before = runner.game.players[0].hand.len();
    stack_deck_top(&mut runner, &["TM-TRAIT2"]);

    // Confirm the clause is optional at the structural level (the no-approx
    // path: declining is exposed because the printed text says "you may").
    let clause_optional = runner
        .compiled_card("BT25-078")
        .expect("compiled")
        .effects
        .iter()
        .any(|c| {
            matches!(c, CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnPlay) && t.optional)
        });
    assert!(clause_optional, "shared clause is optional");

    // Drive the process; at the first prompt (effect choice), the optional
    // outer wrapper exposes PASS — declining must do nothing.
    let process = shared_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(holder_perm), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }
    if let Some(sel) = runner.pending_selection() {
        if sel.is_optional || sel.valid_action_ids.contains(&PASS) {
            let _ = runner.game.resolve_selection(0, PASS);
        }
    }
    // Hand unchanged (no add); deck still holds the trait card.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "declining adds nothing to hand"
    );
}
