//! BT2-076 Pumpkinmon — Digimon, Lv.5, Purple, DP 6000, Cost 6. Traits: [Puppet].
//!
//! # Card text (cards.json effect_description_eng)
//!
//!   Inherited Effect [On Deletion] ＜Draw 2＞. (Draw 2 cards from your deck.)
//!   Then trash 1 card from your hand.
//!
//! INHERITED [On Deletion] (digivolution-source clause). Fires when the HOST
//! Digimon that BT2-076 is stacked under is deleted. ＜Draw 2＞ is the keyword
//! draw; "Then trash 1 card from your hand" is a MANDATORY single discard.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT2/Purple/BT2_076.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - Inherited On Deletion lifecycle trigger
//! - <Draw 2> keyword → unconditional draw
//! - Mandatory single hand trash (forced select_hand, PASS not legal)

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn pad(id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(id, id)
}

fn runner_with_hand(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT2-076")
        .expect("BT2-076 YAML must be present in the embedded DSL pack")
        .add_card(pad("PAD"))
        .add_card(pad("HOST"))
        .deck(0, &["PAD", "PAD", "PAD", "PAD", "PAD", "PAD"])
        .deck(1, &["PAD", "PAD", "PAD", "PAD", "PAD"])
        .hand(0, hand)
        .memory(5)
        .start()
}

/// Place a HOST Digimon on P0's field with BT2-076 as a bottom digivolution
/// source. Deleting the host fires BT2-076's inherited [On Deletion].
fn place_host_over_pumpkinmon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_stack(0, &["BT2-076", "HOST"])
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt2_076_yaml_compiles_and_is_in_pack() {
    let runner = runner_with_hand(&[]);
    assert!(
        runner.compiled_card("BT2-076").is_some(),
        "BT2-076 must be present in the embedded DSL pack"
    );
}

#[test]
fn bt2_076_has_one_inherited_on_deletion_clause() {
    let runner = runner_with_hand(&[]);
    let compiled = runner
        .compiled_card("BT2-076")
        .expect("BT2-076 compiled card must be registered");

    assert_eq!(compiled.effects.len(), 1, "exactly one inherited clause");

    match &compiled.effects[0] {
        CompiledClause::Triggered(clause) => {
            assert_eq!(
                clause.scope,
                CompiledScope::Inherited,
                "BT2-076's [On Deletion] is an inherited (digivolution-source) effect"
            );
            assert_eq!(clause.when, vec![CompiledTiming::OnDeletion]);
            assert!(
                !clause.optional,
                "the printed text has no 'you may' — the effect is mandatory"
            );
        }
        other => panic!("BT2-076 effect must be a Triggered clause; got {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Draw 2 then trash 1
// ═══════════════════════════════════════════════════════════════════════════

/// Empty hand → Draw 2 (hand=2), trash 1 (hand=1). Net +1.
#[test]
fn bt2_076_inherited_on_deletion_draws_two_then_trashes_one() {
    let mut runner = runner_with_hand(&[]);
    let host = place_host_over_pumpkinmon(&mut runner);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        1,
        "Draw 2 then trash 1 from an empty hand leaves exactly 1 card"
    );
}

/// Hand with 1 spare → Draw 2 (hand=3), trash 1 (hand=2). Net +1.
#[test]
fn bt2_076_with_nonempty_hand_player_chooses_discard() {
    let mut runner = runner_with_hand(&["PAD"]);
    let host = place_host_over_pumpkinmon(&mut runner);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        2,
        "1 (start) + 2 (draw) - 1 (trash) = 2"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — No-approximations: the trash pick is forced (no PASS)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt2_076_trash_pick_is_mandatory_no_pass() {
    let mut runner = runner_with_hand(&["PAD"]);
    let host = place_host_over_pumpkinmon(&mut runner);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("a hand-trash selection must be pending after Draw 2");
    assert!(
        !runner.pending_is_optional(),
        "the mandatory 'trash 1 card' pick must not be optional"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "PASS must not be legal while the forced trash pick is required"
    );
}
