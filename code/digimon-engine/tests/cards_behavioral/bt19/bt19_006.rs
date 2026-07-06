//! BT19-006 Pagumon — Digi-Egg (In-Training), Lv.2, Purple, no DP, Cost 0.
//! Traits: [Lesser].
//!
//! # Card text (data/card_bundles/BT19-006.md — official Bandai DB, verbatim)
//!
//!   Inherited Effect
//!   [On Deletion] If deleted other than in battle, return 1 level 3 purple
//!   Digimon card from your trash to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT19/Purple/BT19_006.cs
//!
//! DCGO shape: ONE inherited ActivateClass gated on `EffectTiming.
//! OnDestroyedAnyone` (`SetIsInheritedEffect(true)`).
//!   - `CanUseCondition`: `CardEffectCommons.CanTriggerOnDeletion(...)` AND
//!     `!CardEffectCommons.IsByBattle(...)` — "deleted other than in battle".
//!   - `CanActivateCondition`: `CanActivateOnDeletion(...)` AND
//!     `HasMatchConditionOwnersCardInTrash(card, PurpleLevelThree)` — the
//!     trigger only activates when a matching card exists in trash.
//!   - `ActivateCoroutine`: `SelectCardEffect` over the trash root, filter
//!     `PurpleLevelThree` (`HasCardColor(Purple) && IsLevel3`), `maxCount: 1`,
//!     `canNoSelect: () => false` — the inner pick is MANDATORY once the
//!     trigger activates (no "you may" in the printed text), `mode:
//!     AddHand`.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3 / §14)
//! - Inherited [On Deletion] on a Digi-Egg source (BT3-006 idiom)
//! - `not: { event_cause: battle_deletion }` gate (EX11-020 idiom)
//! - Mandatory `select_trash` that silently short-circuits with no match
//!   (BT17-007 / LM-027 idiom) — "no approximations": PASS must not be legal
//!   while the forced pick is required, and the trigger must not fire at all
//!   when trash holds no matching card.
//! - `add_to_hand_from_trash` payoff (BT21-007 / BT22-008 idiom)

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT19-006";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn pad(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A level-3 purple Digimon — the exact match filter (color AND level).
fn purple_level_3(id: &str, name: &str) -> CardData {
    let mut card = make_test_card_with_level(id, name, 3);
    card.colors = vec![CardColor::Purple];
    card
}

/// A level-3 RED Digimon — wrong color, must not be offered.
fn red_level_3(id: &str, name: &str) -> CardData {
    let mut card = make_test_card_with_level(id, name, 3);
    card.colors = vec![CardColor::Red];
    card
}

/// A level-4 PURPLE Digimon — wrong level, must not be offered.
fn purple_level_4(id: &str, name: &str) -> CardData {
    let mut card = make_test_card_with_level(id, name, 4);
    card.colors = vec![CardColor::Purple];
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-006 YAML must be present in the embedded DSL pack")
        .add_card(pad("HOST"))
        .add_card(pad("PAD"))
        .add_card(purple_level_3("PURP3-A", "Purple Lv3 A"))
        .add_card(purple_level_3("PURP3-B", "Purple Lv3 B"))
        .add_card(red_level_3("RED3", "Red Lv3"))
        .add_card(purple_level_4("PURP4", "Purple Lv4"))
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(5)
        .start()
}

fn runner_no_pad() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-006 YAML must be present in the embedded DSL pack")
        .add_card(pad("HOST"))
        .add_card(purple_level_3("PURP3-A", "Purple Lv3 A"))
        .add_card(red_level_3("RED3", "Red Lv3"))
        .add_card(purple_level_4("PURP4", "Purple Lv4"))
        .memory(5)
        .start()
}

/// Place a HOST Digimon on P0's field with BT19-006 as its bottom Digi-Egg
/// source. Deleting the host fires BT19-006's inherited [On Deletion].
fn place_host_over_egg(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_stack(0, &[CARD_ID, "HOST"])
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize].trash.push(
        digimon_engine::card_source::CardSource::new(data_index, player, instance_id),
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_006_yaml_compiles_and_is_in_pack() {
    let runner = runner();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "BT19-006 must be present in the embedded DSL pack"
    );
}

#[test]
fn bt19_006_has_one_inherited_on_deletion_clause() {
    let runner = runner();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT19-006 compiled card must be registered");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 1, "exactly one triggered clause");
    let clause = triggered[0];
    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "BT19-006's [On Deletion] is an inherited (Digi-Egg source) effect"
    );
    assert_eq!(clause.when, vec![CompiledTiming::OnDeletion]);
    assert!(
        !clause.optional,
        "the printed text has no 'you may' — the effect is mandatory"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating: not-battle-deletion
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_006_fires_when_deleted_by_effect_other_than_battle() {
    let mut runner = runner();
    let host = place_host_over_egg(&mut runner);
    push_to_trash(&mut runner, 0, "PURP3-A");

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_some(),
        "a mandatory trash-return selection must install on a non-battle deletion"
    );
}

#[test]
fn bt19_006_does_not_fire_when_deleted_in_battle() {
    let mut runner = runner();
    let host = place_host_over_egg(&mut runner);
    push_to_trash(&mut runner, 0, "PURP3-A");

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::Battle);

    assert!(
        runner.pending_selection().is_none(),
        "battle deletion must not offer the trash-return prompt"
    );
    assert_eq!(
        runner.hand_size(0),
        0,
        "no card should have been returned to hand on a battle deletion"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — No-approximations: the trash-return pick is forced (no PASS),
// and the trigger short-circuits silently when no matching card exists.
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_006_return_pick_is_mandatory_no_pass() {
    let mut runner = runner();
    let host = place_host_over_egg(&mut runner);
    push_to_trash(&mut runner, 0, "PURP3-A");

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("a trash-return selection must be pending");
    assert!(
        !runner.pending_is_optional(),
        "the mandatory 'return 1 card' pick must not be optional"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "PASS must not be legal while the forced return pick is required"
    );
}

#[test]
fn bt19_006_does_not_fire_when_trash_has_no_matching_card() {
    let mut runner = runner_no_pad();
    let host = place_host_over_egg(&mut runner);
    // Trash holds only wrong-color / wrong-level cards — no level-3 purple.
    push_to_trash(&mut runner, 0, "RED3");
    push_to_trash(&mut runner, 0, "PURP4");

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "with no level-3 purple Digimon in trash the mandatory trigger must not fire at all"
    );
    assert_eq!(runner.hand_size(0), 0, "no card returned when no match exists");
}

#[test]
fn bt19_006_does_not_fire_when_trash_is_empty() {
    let mut runner = runner_no_pad();
    let host = place_host_over_egg(&mut runner);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "an empty trash means the mandatory trigger short-circuits silently"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Filter correctness: only level-3 PURPLE Digimon are offered
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_006_offers_only_level_3_purple_digimon_from_trash() {
    let mut runner = runner();
    let host = place_host_over_egg(&mut runner);
    push_to_trash(&mut runner, 0, "PURP3-A");
    push_to_trash(&mut runner, 0, "RED3");
    push_to_trash(&mut runner, 0, "PURP4");

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("a trash-return selection must be pending");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the level-3 purple Digimon (PURP3-A) is a legal pick"
    );
}

#[test]
fn bt19_006_wrong_color_level_3_card_alone_does_not_trigger() {
    let mut runner = runner_no_pad();
    let host = place_host_over_egg(&mut runner);
    push_to_trash(&mut runner, 0, "RED3");

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "a level-3 RED Digimon does not satisfy the purple-color filter"
    );
}

#[test]
fn bt19_006_wrong_level_purple_card_alone_does_not_trigger() {
    let mut runner = runner_no_pad();
    let host = place_host_over_egg(&mut runner);
    push_to_trash(&mut runner, 0, "PURP4");

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "a level-4 purple Digimon does not satisfy the level-3 filter"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Behavioral outcome: the selected card returns to hand
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_006_selected_card_moves_from_trash_to_hand() {
    let mut runner = runner();
    let host = place_host_over_egg(&mut runner);
    push_to_trash(&mut runner, 0, "PURP3-A");

    let trash_before = runner.trash_size(0);
    let hand_before = runner.hand_size(0);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("trash-return selection pending");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the level-3 purple Digimon");
    runner.auto_resolve().expect("finish the return-to-hand flow");

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "hand gains the returned card"
    );
    // trash_before included the on-deletion trashing of the egg/host stack
    // itself (2 cards) plus the seeded PURP3-A; after the return, trash
    // shrinks by exactly one relative to its post-deletion size.
    assert!(
        runner.trash_size(0) < trash_before + 2,
        "the returned card must leave the trash"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "PURP3-A"),
        "PURP3-A must be the card that landed in hand"
    );
    assert!(
        runner
            .game
            .players[0]
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "PURP3-A"),
        "PURP3-A must no longer be in the trash"
    );
}

#[test]
fn bt19_006_choosing_among_two_matches_returns_only_the_chosen_one() {
    let mut runner = runner();
    let host = place_host_over_egg(&mut runner);
    push_to_trash(&mut runner, 0, "PURP3-A");
    push_to_trash(&mut runner, 0, "PURP3-B");

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("trash-return selection pending");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both level-3 purple Digimon in trash are legal picks"
    );

    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose one of the two matches");
    runner.auto_resolve().expect("finish the return-to-hand flow");

    assert_eq!(runner.hand_size(0), 1, "exactly one card returned to hand");
    let remaining_in_trash: Vec<&str> = runner.game.players[0]
        .trash
        .iter()
        .filter(|c| {
            let id = c.card_id(&runner.game.card_data);
            id == "PURP3-A" || id == "PURP3-B"
        })
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert_eq!(
        remaining_in_trash.len(),
        1,
        "exactly one of the two candidates remains in trash"
    );
}
