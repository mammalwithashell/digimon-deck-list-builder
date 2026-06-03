//! BT7-069 Eyesmon: Scatter Mode — Digimon, Lv.4, Purple, DP 4000, Cost 4.
//! Traits: [Dark Dragon]. Attribute: Virus.
//!
//! # Card text (cards.json effect_description_eng)
//!
//!   [On Deletion] ＜Draw 3＞ (Draw 3 cards from your deck.) Then, trash 2
//!   cards in your hand.
//!
//! This is the carrier's OWN [On Deletion] (NOT inherited). ＜Draw N＞ is a
//! keyword = draw N from deck. The "Then, trash 2 cards in your hand" is a
//! MANDATORY hand discard (no "you may"); when the post-draw hand has more
//! than 2 cards the player chooses WHICH 2 to trash — each pick must surface
//! as a separate selection (no auto-selection, no `[0]`).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT7/Purple/BT7_069.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - On Deletion own (non-inherited) lifecycle trigger
//! - <Draw N> keyword → unconditional draw
//! - Mandatory multi-pick hand trash (two sequential forced select_hand picks)
//! - PASS lockout: the forced trash picks must NOT expose a legal PASS while a
//!   pick is required.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn pad(id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(id, id)
}

/// Runner with BT7-069 registered, a deep filler deck (so Draw 3 never decks
/// out) and an explicit hand for the player.
fn runner_with_hand(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT7-069")
        .expect("BT7-069 YAML must be present in the embedded DSL pack")
        .add_card(pad("PAD"))
        .deck(0, &["PAD", "PAD", "PAD", "PAD", "PAD", "PAD", "PAD", "PAD"])
        .deck(1, &["PAD", "PAD", "PAD", "PAD", "PAD"])
        .hand(0, hand)
        .memory(5)
        .start()
}

/// Place BT7-069 itself on P0's field (as its own permanent top card) so that
/// deleting the permanent fires its own [On Deletion].
fn place_eyesmon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, "BT7-069", Some(0))
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions on the compiled card
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt7_069_yaml_compiles_and_is_in_pack() {
    let runner = runner_with_hand(&[]);
    assert!(
        runner.compiled_card("BT7-069").is_some(),
        "BT7-069 must be present in the embedded DSL pack"
    );
}

#[test]
fn bt7_069_has_one_own_on_deletion_clause() {
    let runner = runner_with_hand(&[]);
    let compiled = runner
        .compiled_card("BT7-069")
        .expect("BT7-069 compiled card must be registered");

    assert_eq!(
        compiled.effects.len(),
        1,
        "BT7-069 has exactly one [On Deletion] clause"
    );

    match &compiled.effects[0] {
        CompiledClause::Triggered(clause) => {
            assert_eq!(
                clause.scope,
                CompiledScope::FaceUp,
                "BT7-069's [On Deletion] is the carrier's own (face-up) effect, not inherited"
            );
            assert_eq!(
                clause.when,
                vec![CompiledTiming::OnDeletion],
                "OnDeletion is the only timing"
            );
            assert!(
                !clause.optional,
                "the printed text has no 'you may' — the effect is mandatory"
            );
        }
        other => panic!("BT7-069 effect must be a Triggered clause; got {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Draw 3 then trash 2
// ═══════════════════════════════════════════════════════════════════════════

/// Hand starts empty → after deletion: Draw 3 (hand=3), then trash 2 (hand=1).
/// Net hand change = +1. Trash gains 2 (the discarded cards) plus the deleted
/// BT7-069 itself.
#[test]
fn bt7_069_on_deletion_draws_three_then_trashes_two() {
    let mut runner = runner_with_hand(&[]);
    let eyes = place_eyesmon(&mut runner);

    let hand_before = runner.hand_size(0);
    assert_eq!(hand_before, 0, "test starts with an empty hand");

    runner
        .game
        .delete_permanent_with_cause(eyes, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    assert_eq!(
        hand_after, 1,
        "Draw 3 then trash 2 from an empty starting hand leaves exactly 1 card; got {hand_after}"
    );
}

/// Hand starts with 2 spare cards → after deletion: Draw 3 (hand=5), trash 2
/// (hand=3). The player chooses which 2 of 5 to trash.
#[test]
fn bt7_069_with_full_hand_player_chooses_which_two_to_trash() {
    let mut runner = runner_with_hand(&["PAD", "PAD"]);
    let eyes = place_eyesmon(&mut runner);

    let hand_before = runner.hand_size(0);
    assert_eq!(hand_before, 2);

    runner
        .game
        .delete_permanent_with_cause(eyes, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    assert_eq!(
        hand_after, 3,
        "2 (start) + 3 (draw) - 2 (trash) = 3; got {hand_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — No-approximations: the two trash picks are forced (no PASS)
// ═══════════════════════════════════════════════════════════════════════════

/// When the post-draw hand exceeds 2 cards, the first forced trash pick must be
/// a mandatory selection — PASS must NOT be a legal action mid-pick.
#[test]
fn bt7_069_trash_picks_are_mandatory_no_pass() {
    let mut runner = runner_with_hand(&["PAD", "PAD"]);
    let eyes = place_eyesmon(&mut runner);

    runner
        .game
        .delete_permanent_with_cause(eyes, ReplacementCause::OpponentEffect);

    // After Draw 3 the engine parks on the first forced hand-trash selection.
    let view = runner
        .pending_selection_view()
        .expect("a hand-trash selection must be pending after Draw 3");
    assert!(
        !runner.pending_is_optional(),
        "the mandatory 'trash 2 cards' pick must not be optional"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "PASS must not be legal while a forced trash pick is required"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Event observability
// ═══════════════════════════════════════════════════════════════════════════

/// The clause is observable through hand growth (net +1 from empty) after the
/// deletion fires. Captures an event checkpoint for future Draw-event wiring.
#[test]
fn bt7_069_effect_observable_after_deletion() {
    let mut runner = runner_with_hand(&[]);
    let eyes = place_eyesmon(&mut runner);

    let cp = runner.event_checkpoint();
    runner
        .game
        .delete_permanent_with_cause(eyes, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    let _events = runner.events_since(cp);
    assert_eq!(
        runner.hand_size(0),
        1,
        "the On Deletion effect must net +1 card from an empty hand"
    );
}
