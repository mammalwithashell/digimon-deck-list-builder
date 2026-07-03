//! BT21-056 Vemmon — Digimon, Lv.3, Black, Cost 3, DP 1000.
//! Traits: Unknown / LIBERATOR.
//!
//! # Card text (data/card_bundles/BT21-056.md — official Bandai DB)
//!
//! [On Play] By trashing 1 card with [Vemmon] in its text from your hand,
//! you may return 1 non-Digi-Egg card with [Vemmon] in its text from your
//! trash to the hand.
//!
//! # Inherited effect text
//!
//! [Your Turn] [Once Per Turn] When this Digimon would digivolve into a
//! Digimon card with [Vemmon] in its text, reduce the digivolution cost by 1.
//!
//! # Official Q&A
//!
//! "It refers to a card that contains the specified text or icon in its
//! name, traits, effects, inherited effects, (Rule), digivolution
//! requirements, DNA digivolution, DigiXros requirements, burst digivolve,
//! App Fusion, Link, or Assembly requirements." — i.e. "[Vemmon] in its
//! text" is a whole-card scan (name + traits + body text), NOT body-text
//! only. Both filters in the YAML are widened to
//! `any_of: [effect_text_contains, trait_has, name_contains]` to cover the
//! name/traits axes; several tests below pin the name-only match case
//! specifically (a synthetic card whose ONLY "Vemmon" mention is its name).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_056.cs
//!
//! # Patterns this test covers
//! - E2: optional cost-as-trashing OnPlay (select_hand cost:true) with a
//!   separately-optional trash→hand recovery (select_trash, no cost flag)
//! - Digi-Egg exclusion on the recovery pool (`none_of: [{kind: digi_egg}]`)
//! - Whole-card "[X] in its text" widening: name_contains / trait_has /
//!   effect_text_contains, each pinned independently
//! - Phase 2 Track H digivolve-cost reduction: `source_is_cost_target_permanent`
//!   + `cost_target` gated on the SAME "Vemmon in its text" filter, `scope:
//!   inherited`, `once_per_turn`, `active_when: { your_turn: true }`

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, make_test_egg, DebugRunner,
};
use digimon_engine::enums::{CardColor, PlaySource};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT21-056";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A generic Lv.3 Black Digimon hand/trash card, no Vemmon text anywhere.
fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, "Filler");
    c.colors = vec![CardColor::Black];
    c.level = Some(3);
    c
}

/// A card whose ONLY "Vemmon" mention is body text (effect_text).
fn vemmon_body_text(id: &str) -> CardData {
    let mut c = filler(id);
    c.effect_text = "Some text mentioning [Vemmon] here.".to_string();
    c
}

/// A card whose ONLY "Vemmon" mention is a trait.
fn vemmon_trait(id: &str) -> CardData {
    let mut c = filler(id);
    c.traits = vec!["Vemmon".to_string()];
    c
}

/// A card whose ONLY "Vemmon" mention is its printed name — no body text,
/// no trait. Proves the widened `name_contains` axis of "[Vemmon] in its
/// text" per the card's own official Q&A.
fn vemmon_named_only(id: &str) -> CardData {
    let mut c = make_test_card(id, "Vemmon Prime");
    c.colors = vec![CardColor::Black];
    c.level = Some(3);
    c
}

/// A non-Digi-Egg Vemmon-body-text card at a specific level, used as the
/// digivolve-into target for the inherited cost-reduction tests.
fn vemmon_digivolve_target(id: &str, level: u8) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.colors = vec![CardColor::Black];
    c.effect_text = "Card with [Vemmon] in its text.".to_string();
    c.evo_costs = vec![EvoCost {
        card_color: 5, // Black
        level: level - 1,
        memory_cost: 2,
    }];
    c
}

/// A non-Vemmon digivolve-into target at a specific level (negative control).
fn plain_digivolve_target(id: &str, level: u8) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.colors = vec![CardColor::Black];
    c.evo_costs = vec![EvoCost {
        card_color: 5, // Black
        level: level - 1,
        memory_cost: 2,
    }];
    c
}

fn vemmon_digi_egg(id: &str) -> CardData {
    let mut c = make_test_egg(id, "Vemmon Egg");
    c.effect_text = "Egg with [Vemmon] in its text.".to_string();
    c
}

fn base_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-056 must be in embedded pack")
}

/// Resolve exactly ONE pending selection step (the first legal action), then
/// stop — unlike `auto_resolve()`, which drains the entire cascade. Needed
/// here because Clause 1 installs TWO sequential selections (the trash-cost
/// pick, then the separately-optional recovery pick) and several tests need
/// to inspect state BETWEEN them.
fn resolve_one(runner: &mut DebugRunner) {
    let sel = runner
        .pending_selection()
        .expect("a pending selection must exist to resolve one step");
    let player = sel.selecting_player;
    let action_id = sel
        .valid_action_ids
        .first()
        .copied()
        .unwrap_or(digimon_engine::action::space::PASS);
    runner
        .execute_action(player, action_id)
        .expect("resolve_one: execute_action failed");
}

// ---------------------------------------------------------------------------
// Section 1 — Structural assertions
// ---------------------------------------------------------------------------

#[test]
fn bt21_056_has_one_triggered_and_one_cost_reduction_clause() {
    let runner = base_runner().build();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card must be present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "exactly one triggered (OnPlay) clause");

    let cost_reductions: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                scope,
                once_per_turn,
                ..
            }) => Some((*scope, *once_per_turn)),
            _ => None,
        })
        .collect();
    assert_eq!(
        cost_reductions.len(),
        1,
        "exactly one CostReduction declarative clause"
    );
    assert_eq!(
        cost_reductions[0].0,
        CompiledScope::Inherited,
        "cost reducer must be inherited scope (printed under inherited_effect_description_eng)"
    );
    assert!(
        cost_reductions[0].1,
        "cost reducer must be once_per_turn (DCGO MaxCountPerTurn 1)"
    );
}

#[test]
fn bt21_056_on_play_clause_is_own_optional() {
    let runner = base_runner().build();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card must be present");

    let on_play = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .next()
        .expect("OnPlay clause must exist");

    assert_eq!(
        on_play.scope,
        CompiledScope::FaceUp,
        "OnPlay must be own (face_up) scope"
    );
    assert!(on_play.optional, "OnPlay clause must be optional");
    assert!(
        !on_play.once_per_turn,
        "OnPlay is not printed [Once Per Turn]"
    );
}

// ---------------------------------------------------------------------------
// Section 2 — Clause 1 condition gating
// ---------------------------------------------------------------------------

#[test]
fn bt21_056_on_play_no_prompt_when_hand_has_no_vemmon_text_card() {
    let mut runner = base_runner()
        .add_card(filler("FILLER-HAND"))
        .hand(0, &["BT21-056", "FILLER-HAND"])
        .memory(10)
        .start();

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when hand has no [Vemmon]-text card"
    );
}

#[test]
fn bt21_056_on_play_prompt_installs_when_hand_has_vemmon_text_card() {
    let mut runner = base_runner()
        .add_card(vemmon_body_text("VEM-BODY-HAND"))
        .hand(0, &["BT21-056", "VEM-BODY-HAND"])
        .memory(10)
        .start();

    runner.play(0, 0);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "OnPlay optional cost-select should install a Hand prompt"
    );
    assert!(
        runner.pending_is_optional(),
        "the trash-cost pick must be optional (declinable)"
    );
}

// ---------------------------------------------------------------------------
// Section 3 — Clause 1 behavioral outcomes
// ---------------------------------------------------------------------------

/// Happy path: trash a body-text Vemmon card, then accept the recovery,
/// returning a body-text Vemmon card from trash to hand.
#[test]
fn bt21_056_on_play_trash_and_recover_body_text_match() {
    let mut runner = base_runner()
        .add_card(vemmon_body_text("VEM-BODY-HAND"))
        .add_card(vemmon_body_text("VEM-BODY-TRASH"))
        .hand(0, &["BT21-056", "VEM-BODY-HAND"])
        .memory(10)
        .start();

    // Seed the trash with a recoverable Vemmon-text card.
    runner.inject_trash(0, "VEM-BODY-TRASH");

    let trash_before = runner.trash_size(0);
    let hand_before = runner.hand_size(0);

    runner.play(0, 0);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    resolve_one(&mut runner); // pick the cost card

    // Cost paid: VEM-BODY-HAND moved hand→trash (+1 trash net of the trash
    // seed already counted in trash_before).
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "cost card must be trashed"
    );

    // Recovery prompt should now be pending.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "recovery half must install a Trash-zone prompt"
    );
    assert!(
        runner.pending_is_optional(),
        "the recovery pick must be optional (you may return)"
    );
    resolve_one(&mut runner); // pick the recovery card

    assert_eq!(
        runner.trash_size(0),
        trash_before, // +1 from cost, -1 from recovery = net 0
        "recovered card leaves the trash"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1, // played BT21-056 (-1), trashed cost card (-1), recovered (+1) = net -1
        "hand should be down by exactly the played BT21-056 after cost+recovery nets out"
    );
}

/// Declining the trash-cost aborts the whole clause: no trash, no recovery
/// prompt.
#[test]
fn bt21_056_on_play_decline_cost_does_nothing() {
    let mut runner = base_runner()
        .add_card(vemmon_body_text("VEM-BODY-HAND"))
        .hand(0, &["BT21-056", "VEM-BODY-HAND"])
        .memory(10)
        .start();

    let trash_before = runner.trash_size(0);

    runner.play(0, 0);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));

    let player = runner.pending_selection().unwrap().selecting_player;
    runner
        .execute_action(player, PASS)
        .expect("PASS declines the cost pick");

    assert!(
        runner.pending_selection().is_none(),
        "declining the cost must abort the whole clause"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "no card trashed when the cost is declined"
    );
}

/// Having paid the cost, declining the recovery half leaves the trashed
/// card in the trash (the recovery is independently optional).
#[test]
fn bt21_056_on_play_pays_cost_then_declines_recovery() {
    let mut runner = base_runner()
        .add_card(vemmon_body_text("VEM-BODY-HAND"))
        .add_card(vemmon_body_text("VEM-BODY-TRASH"))
        .hand(0, &["BT21-056", "VEM-BODY-HAND"])
        .memory(10)
        .start();

    runner.inject_trash(0, "VEM-BODY-TRASH");
    let trash_before = runner.trash_size(0);

    runner.play(0, 0);
    resolve_one(&mut runner); // pick the cost card

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
    assert!(runner.pending_is_optional());

    let player = runner.pending_selection().unwrap().selecting_player;
    runner
        .execute_action(player, PASS)
        .expect("PASS declines the recovery pick");

    assert!(
        runner.pending_selection().is_none(),
        "no further selection after declining recovery"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the trashed cost card must remain in trash when recovery is declined"
    );
}

/// A Vemmon-TRAIT-only card is a valid cost AND a valid recovery target
/// (proves the `trait_has` axis of the widened "[Vemmon] in its text").
#[test]
fn bt21_056_trait_only_match_is_valid_cost_and_recovery() {
    let mut runner = base_runner()
        .add_card(vemmon_trait("VEM-TRAIT-HAND"))
        .add_card(vemmon_trait("VEM-TRAIT-TRASH"))
        .hand(0, &["BT21-056", "VEM-TRAIT-HAND"])
        .memory(10)
        .start();

    runner.inject_trash(0, "VEM-TRAIT-TRASH");

    runner.play(0, 0);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "trait-only Vemmon card must be offered as a valid cost"
    );
    resolve_one(&mut runner); // pick the trait-only cost card

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "recovery prompt must install"
    );
    let view = runner
        .pending_selection_view()
        .expect("pending view available");
    assert!(
        !view.valid_action_ids.is_empty(),
        "trait-only Vemmon card in trash must be a valid recovery target"
    );
}

/// A Vemmon-NAMED-only card (no body text, no trait) is a valid cost AND a
/// valid recovery target — proves the `name_contains` axis, the core fix
/// this directive requires (per the official Q&A whole-card HasText scan).
#[test]
fn bt21_056_name_only_match_is_valid_cost_and_recovery() {
    let mut runner = base_runner()
        .add_card(vemmon_named_only("VEM-NAME-HAND"))
        .add_card(vemmon_named_only("VEM-NAME-TRASH"))
        .hand(0, &["BT21-056", "VEM-NAME-HAND"])
        .memory(10)
        .start();

    runner.inject_trash(0, "VEM-NAME-TRASH");

    runner.play(0, 0);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "name-only Vemmon card (\"Vemmon Prime\") must be offered as a valid cost"
    );
    resolve_one(&mut runner); // pick the name-only cost card

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "recovery prompt must install"
    );
    let view = runner
        .pending_selection_view()
        .expect("pending view available");
    assert!(
        !view.valid_action_ids.is_empty(),
        "name-only Vemmon card in trash must be a valid recovery target"
    );
}

/// A non-Digi-Egg gate: a Vemmon-text Digi-Egg sitting in trash must NOT be
/// offered as a recovery target.
#[test]
fn bt21_056_digi_egg_excluded_from_recovery() {
    let mut runner = base_runner()
        .add_card(vemmon_body_text("VEM-BODY-HAND"))
        .add_card(vemmon_digi_egg("VEM-EGG-TRASH"))
        .hand(0, &["BT21-056", "VEM-BODY-HAND"])
        .memory(10)
        .start();

    runner.inject_trash(0, "VEM-EGG-TRASH");

    runner.play(0, 0);
    runner.auto_resolve().expect("pick the cost card");

    // The recovery prompt should either not install (no eligible target) or
    // install with zero legal picks besides declining.
    if let Some(kind) = runner.pending_kind() {
        assert_eq!(kind, SelectionKind::Trash);
        let view = runner
            .pending_selection_view()
            .expect("pending view available");
        assert!(
            view.valid_action_ids.is_empty(),
            "a Digi-Egg with [Vemmon] in its text must not be a legal recovery target"
        );
    }
}

// ---------------------------------------------------------------------------
// Section 4 — Clause 2: inherited digivolve-cost reduction
// ---------------------------------------------------------------------------

/// BT21-056's cost reducer is INHERITED-only (DCGO `activateClass2` requires
/// `!evoRootTops.Contains(card)` — BT21-056 must NOT itself be a new top
/// card; it only fires while already buried as a digivolution source). Bury
/// it under a plain Lv.3 top card, then digivolve that carrier into a
/// Vemmon-body-text Lv.4 card. Base evo cost is 2; expect -1 = 1 spent.
#[test]
fn bt21_056_inherited_reduces_cost_when_digivolving_into_vemmon_text_card() {
    let mut runner = base_runner()
        .add_card(plain_digivolve_target("TOP-LV3", 3))
        .add_card(vemmon_digivolve_target("VEM-TARGET-LV4", 4))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-LV3"]);
    let hand_index = push_to_hand(&mut runner, 0, "VEM-TARGET-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);

    assert!(digivolved, "carrier should digivolve into VEM-TARGET-LV4");
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "base cost 2 should be reduced to 1 when digivolving into a [Vemmon]-text card"
    );
}

/// Negative control: digivolving into a NON-Vemmon-text target pays the
/// full, unreduced cost.
#[test]
fn bt21_056_inherited_does_not_reduce_cost_for_non_vemmon_target() {
    let mut runner = base_runner()
        .add_card(plain_digivolve_target("TOP-LV3", 3))
        .add_card(plain_digivolve_target("PLAIN-TARGET-LV4", 4))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-LV3"]);
    let hand_index = push_to_hand(&mut runner, 0, "PLAIN-TARGET-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);

    assert!(digivolved, "should still digivolve into the plain target");
    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "non-[Vemmon]-text target must pay the full base cost (no reduction)"
    );
}

/// The reduction fires from a BURIED position too (BT21-056 dug under a
/// generic top card, which is itself digivolving further) — proves the
/// `scope: inherited` gate + `is_under` matching works cross-position, not
/// just when BT21-056 is the literal top card.
#[test]
fn bt21_056_inherited_reduces_cost_when_buried_under_top_card() {
    let mut runner = base_runner()
        .add_card(plain_digivolve_target("TOP-LV3", 3))
        .add_card(vemmon_digivolve_target("VEM-TARGET-LV4", 4))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // BT21-056 is buried under TOP-LV3 (a plain Lv.3 top card).
    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-LV3"]);
    let hand_index = push_to_hand(&mut runner, 0, "VEM-TARGET-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);

    assert!(
        digivolved,
        "carrier stack (BT21-056 buried) should digivolve into VEM-TARGET-LV4"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "buried BT21-056 must still reduce the digivolution cost by 1 into a [Vemmon]-text card"
    );
}

/// A name-only Vemmon match ("Vemmon Prime") as the digivolve target also
/// qualifies for the reduction — proves the `cost_target` gate uses the
/// same widened any_of filter as Clause 1, not a narrower body-text-only
/// check.
#[test]
fn bt21_056_inherited_reduces_cost_for_name_only_vemmon_target() {
    let mut runner = base_runner()
        .add_card(plain_digivolve_target("TOP-LV3", 3))
        .add_card(vemmon_digivolve_named_target("VEM-NAME-TARGET-LV4", 4))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-LV3"]);
    let hand_index = push_to_hand(&mut runner, 0, "VEM-NAME-TARGET-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);

    assert!(digivolved, "should digivolve into the name-only target");
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "name-only [Vemmon] target (e.g. \"Vemmon Prime\") must still trigger the -1 reduction"
    );
}

/// [Your Turn] gate: the reducer must NOT fire on the opponent's turn. Set
/// up P1 with a Vemmon-text hand card and a BT21-056 they do NOT control —
/// instead, verify the reducer requires the controller's OWN turn by
/// checking the `active_when: { your_turn: true }` guard directly: place
/// BT21-056 on P0's field, advance to P1's turn, and have P0 attempt to
/// digivolve (illegal outside Main phase / not their turn is out of scope —
/// instead this test asserts the structural guard via compiled predicate,
/// since actually attempting an off-turn digivolve is not a legal action
/// path in the engine).
#[test]
fn bt21_056_inherited_cost_reduction_declares_your_turn_gate() {
    let runner = base_runner().build();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card must be present");

    let cost_reduction = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                active_when,
                ..
            }) => active_when.as_ref(),
            _ => None,
        })
        .expect("CostReduction clause with active_when must exist");

    // `your_turn: true` is nested inside the top-level `all_of` combinator
    // (the clause also ANDs `source_is_cost_target_permanent` and
    // `cost_target`), so check the sub-predicates rather than the top-level
    // field directly.
    let has_your_turn_leaf = cost_reduction.your_turn == Some(true)
        || cost_reduction
            .all_of
            .iter()
            .any(|p| p.your_turn == Some(true));

    assert!(
        has_your_turn_leaf,
        "cost reducer must gate on [Your Turn] (found: {cost_reduction:?})"
    );
}

/// OPT: two digivolutions of the SAME carrier into Vemmon-text targets in
/// the same turn — only the first is reduced.
#[test]
fn bt21_056_inherited_opt_blocks_second_reduction_same_turn() {
    let mut runner = base_runner()
        .add_card(plain_digivolve_target("TOP-LV3", 3))
        .add_card(vemmon_digivolve_target("VEM-TARGET-LV4", 4))
        .add_card(vemmon_digivolve_target("VEM-TARGET-LV5", 5))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-LV3"]);
    let hand_index_lv4 = push_to_hand(&mut runner, 0, "VEM-TARGET-LV4");

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_index_lv4,
        carrier.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved, "first digivolve should succeed");
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "first digivolve into a [Vemmon]-text card must be reduced by 1"
    );

    // Second digivolve of the SAME stack (BT21-056 remains buried, now under
    // TOP-LV3 and VEM-TARGET-LV4) into another Vemmon-text Lv.5 target, same
    // turn.
    let hand_index_lv5 = push_to_hand(&mut runner, 0, "VEM-TARGET-LV5");
    let memory_before_second = runner.game.memory;
    let digivolved_second = runner.game.digivolve_from_hand(
        0,
        hand_index_lv5,
        carrier.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved_second, "second digivolve should succeed");
    assert_eq!(
        runner.game.memory,
        memory_before_second - 2,
        "OPT must block the second reduction in the same turn — full cost paid"
    );
}

/// OPT resets after end_turn: the reducer fires again on the controller's
/// next turn.
#[test]
fn bt21_056_inherited_opt_resets_after_end_turn() {
    let deck_pad = make_test_card("DECK-PAD", "DeckPad");

    let mut runner = base_runner()
        .add_card(plain_digivolve_target("TOP-LV3", 3))
        .add_card(vemmon_digivolve_target("VEM-TARGET-LV4", 4))
        .add_card(vemmon_digivolve_target("VEM-TARGET-LV5", 5))
        .add_card(deck_pad)
        // Both players need a deck so begin_turn draws don't deck out.
        .deck(
            0,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .deck(
            1,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-LV3"]);
    let hand_index_lv4 = push_to_hand(&mut runner, 0, "VEM-TARGET-LV4");

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_index_lv4,
        carrier.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved, "first digivolve should succeed");
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "first digivolve must be reduced"
    );

    runner.end_turn(); // -> P1's turn
    runner.end_turn(); // -> P0's turn again; OPT should reset

    let hand_index_lv5 = push_to_hand(&mut runner, 0, "VEM-TARGET-LV5");
    let memory_before_second = runner.game.memory;
    let digivolved_second = runner.game.digivolve_from_hand(
        0,
        hand_index_lv5,
        carrier.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved_second, "second digivolve should succeed");
    assert_eq!(
        runner.game.memory,
        memory_before_second - 1,
        "OPT must reset after end_turn — the reducer fires again on the next controller turn"
    );
}

// ---------------------------------------------------------------------------
// Helpers used only by Section 4
// ---------------------------------------------------------------------------

fn vemmon_digivolve_named_target(id: &str, level: u8) -> CardData {
    let mut c = make_test_card_with_level(id, "Vemmon Prime", level);
    c.colors = vec![CardColor::Black];
    c.evo_costs = vec![EvoCost {
        card_color: 5, // Black
        level: level - 1,
        memory_cost: 2,
    }];
    c
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            data_index,
            player,
            instance_id,
        ));
    runner.game.players[player as usize].hand.len() - 1
}
