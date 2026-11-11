//! ST21-01 Tsunomon — Digi-Egg (In-Training), Lv.2, Purple, no DP, Cost 0.
//! Traits: [Lesser, ADVENTURE].
//!
//! # Card text (data/card_bundles/ST21-01.md — official Bandai DB, verbatim)
//!
//! ```text
//! Inherited Effect
//! [On Deletion] You may return 1 Digimon card with the [ADVENTURE] trait from
//! your trash to the hand.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST21/Purple/ST21_01.cs
//!
//! DCGO shape: ONE inherited ActivateClass on `EffectTiming.OnDestroyedAnyone`
//! (`SetIsInheritedEffect(true)`, `isOptional: true`).
//!   - `CanUseCondition`: `CanTriggerOnDeletion` — NO battle gate, so every
//!     deletion cause fires it (unlike BT19-006 Pagumon).
//!   - `CanActivateCondition`: `CanActivateOnDeletion` AND a matching
//!     (`IsDigimon && HasAdventureTraits`) card exists in the owner's trash —
//!     with no candidate the trigger never activates (no prompt).
//!   - `ActivateCoroutine`: `SelectCardEffect` over the trash root,
//!     `maxCount: 1`, `canNoSelect: () => true` ("You may"), `AddHand`.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - Inherited [On Deletion] on a Digi-Egg source (BT3-006 / BT19-006 idiom)
//! - Optional ("You may") trash-to-hand pick, PASS declines (BT26-014 idiom)
//! - Card filter `kind: digimon` AND `trait_has: ADVENTURE` (a Tamer/Option or a
//!   non-ADVENTURE Digimon must never be offered)
//! - Silent short-circuit when the trash holds no legal candidate
//! - Post-trash OnDeletion handler can return the freshly deleted host itself
//!   (CLAUDE.md rule 25)
//!
//! No cost is paid by this card (no event-log cost test) and it carries no
//! once-per-turn clause (no OPT lockout test).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{PendingSelectionView, SelectionKind};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-01";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn pad(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Digimon card; `adventure` toggles the [ADVENTURE] trait.
fn digimon(id: &str, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Purple];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

/// A Tamer card with the [ADVENTURE] trait — NOT a Digimon card, so it must
/// never be offered by ST21-01.
fn adventure_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = vec![CardColor::Purple];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-01 YAML must be present in the embedded DSL pack")
        .add_card(pad("HOST"))
        .add_card(pad("PAD"))
        .add_card(digimon("ADV-A", true))
        .add_card(digimon("ADV-B", true))
        .add_card(digimon("ADV-HOST", true))
        .add_card(digimon("PLAIN-DIGI", false))
        .add_card(adventure_tamer("ADV-TAMER"))
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(5)
        .start()
}

/// Stack ST21-01 as the bottom (Digi-Egg) source of a plain HOST Digimon.
fn host_over_egg(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_stack(0, &[CARD_ID, "HOST"])
}

/// Install the pick: if an outer accept/decline confirm was installed, accept
/// it first, then return the trash-pick view.
fn settle_to_pick(runner: &mut DebugRunner) -> PendingSelectionView {
    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        runner
            .accept_optional_trigger()
            .expect("accept the optional trigger confirm");
    }
    runner
        .pending_selection_view()
        .expect("trash pick must be pending")
}

fn in_hand(runner: &DebugRunner, player: u8, id: &str) -> bool {
    runner.game.players[player as usize]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == id)
}

fn in_trash(runner: &DebugRunner, player: u8, id: &str) -> bool {
    runner.game.players[player as usize]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == id)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_01_identity_matches_printed_card() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Tsunomon");
    assert_eq!(card.kind, CompiledCardKind::DigiEgg);
    assert_eq!(card.level, Some(2));
    assert_eq!(card.cost, Some(0));
    assert_eq!(card.dp, None, "Digi-Eggs print no DP");
    assert_eq!(card.color, vec![CompiledColor::Purple]);
    assert!(
        card.traits.iter().any(|t| t == "Lesser"),
        "{:?}",
        card.traits
    );
    assert!(
        card.traits.iter().any(|t| t == "ADVENTURE"),
        "{:?}",
        card.traits
    );
    assert!(
        card.alt_paths
            .iter()
            .all(|p| p.kind != CompiledAltPathKind::Digivolve),
        "a Digi-Egg prints no digivolve circle"
    );
}

#[test]
fn st21_01_has_one_inherited_optional_on_deletion_clause() {
    let card = compiled(CARD_ID);
    let triggered: Vec<_> = card
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
        "the [On Deletion] box is an Inherited Effect"
    );
    assert_eq!(clause.when, vec![CompiledTiming::OnDeletion]);
    assert!(
        clause.optional,
        "printed 'You may' makes the clause optional"
    );
    assert!(
        !clause.once_per_turn,
        "the printed text carries no [Once Per Turn]"
    );
    let declarative = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Declarative(_)))
        .count();
    assert_eq!(declarative, 0, "no static/declarative clause is printed");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (deletion cause, candidates, scope)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_01_fires_when_deleted_by_effect() {
    let mut r = runner();
    let host = host_over_egg(&mut r);
    r.inject_trash(0, "ADV-A");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert!(
        r.pending_selection().is_some(),
        "an ADVENTURE Digimon in trash must make the optional return prompt install"
    );
    assert!(
        r.pending_is_optional(),
        "'You may' — the return is optional (PASS legal)"
    );
}

#[test]
fn st21_01_also_fires_when_deleted_in_battle() {
    // DCGO CanTriggerOnDeletion has no battle gate: unlike BT19-006, every
    // deletion cause fires ST21-01.
    let mut r = runner();
    let host = host_over_egg(&mut r);
    r.inject_trash(0, "ADV-A");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::Battle);

    assert!(
        r.pending_selection().is_some(),
        "a battle deletion must still offer the return prompt"
    );
}

#[test]
fn st21_01_does_not_fire_without_the_egg_in_the_stack() {
    let mut r = runner();
    let plain = r.place_on_field(0, "HOST", Some(0));
    r.inject_trash(0, "ADV-A");

    r.game
        .delete_permanent_with_cause(plain, ReplacementCause::OpponentEffect);

    assert!(
        r.pending_selection().is_none(),
        "the effect is INHERITED — a Digimon without ST21-01 under it must not trigger it"
    );
    assert_eq!(r.hand_size(0), 0);
}

#[test]
fn st21_01_does_not_fire_when_trash_has_no_adventure_digimon() {
    let mut r = runner();
    let host = host_over_egg(&mut r);
    // A non-ADVENTURE Digimon and an ADVENTURE Tamer: neither qualifies.
    r.inject_trash(0, "PLAIN-DIGI");
    r.inject_trash(0, "ADV-TAMER");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert!(
        r.pending_selection().is_none(),
        "no [ADVENTURE] Digimon card in trash ⇒ the trigger never activates"
    );
    assert_eq!(r.hand_size(0), 0);
}

#[test]
fn st21_01_does_not_fire_when_trash_is_empty() {
    let mut r = runner();
    let host = host_over_egg(&mut r);
    // HOST (a plain Digimon) and the egg land in trash on deletion, but neither
    // is an [ADVENTURE] Digimon card.

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 0);
}

#[test]
fn st21_01_ignores_adventure_digimon_in_the_opponents_trash() {
    let mut r = runner();
    let host = host_over_egg(&mut r);
    r.inject_trash(1, "ADV-A");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert!(
        r.pending_selection().is_none(),
        "'from your trash' — the opponent's trash is not a legal source"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Filter correctness
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_01_offers_only_adventure_digimon_cards() {
    let mut r = runner();
    let host = host_over_egg(&mut r);
    r.inject_trash(0, "ADV-A");
    r.inject_trash(0, "ADV-B");
    r.inject_trash(0, "PLAIN-DIGI");
    r.inject_trash(0, "ADV-TAMER");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    let view = settle_to_pick(&mut r);
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "only ADV-A and ADV-B are legal picks (not the plain Digimon, not the ADVENTURE Tamer): {:?}",
        view.valid_action_ids
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Behavioral outcome
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_01_accepting_returns_the_chosen_card_to_hand() {
    let mut r = runner();
    let host = host_over_egg(&mut r);
    r.inject_trash(0, "ADV-A");
    r.inject_trash(0, "PLAIN-DIGI");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    let view = settle_to_pick(&mut r);
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose ADV-A");
    r.auto_resolve().expect("finish the return flow");

    assert_eq!(r.hand_size(0), 1, "exactly one card returned to hand");
    assert!(in_hand(&r, 0, "ADV-A"), "ADV-A must be in hand");
    assert!(!in_trash(&r, 0, "ADV-A"), "ADV-A must have left the trash");
    assert!(
        in_trash(&r, 0, "PLAIN-DIGI"),
        "the non-qualifying card stays in trash"
    );
}

#[test]
fn st21_01_choosing_between_two_returns_only_the_chosen_one() {
    let mut r = runner();
    let host = host_over_egg(&mut r);
    r.inject_trash(0, "ADV-A");
    r.inject_trash(0, "ADV-B");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    let view = settle_to_pick(&mut r);
    assert_eq!(view.valid_action_ids.len(), 2);
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose one of the two");
    r.auto_resolve().expect("finish");

    assert_eq!(r.hand_size(0), 1, "'1 Digimon card' — exactly one returned");
    let left_in_trash = ["ADV-A", "ADV-B"]
        .iter()
        .filter(|id| in_trash(&r, 0, id))
        .count();
    assert_eq!(
        left_in_trash, 1,
        "exactly one of the two candidates remains"
    );
}

#[test]
fn st21_01_declining_leaves_everything_in_place() {
    let mut r = runner();
    let host = host_over_egg(&mut r);
    r.inject_trash(0, "ADV-A");
    let trash_before = r.trash_size(0);

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    let trash_after_deletion = r.trash_size(0);
    assert!(
        trash_after_deletion > trash_before,
        "the deleted stack lands in trash"
    );

    assert!(r.pending_is_optional(), "PASS must be legal ('You may')");
    r.execute_action(0, PASS).expect("decline");
    let _ = r.auto_resolve();

    assert_eq!(r.hand_size(0), 0, "declined: nothing returns to hand");
    assert!(in_trash(&r, 0, "ADV-A"), "ADV-A stays in trash");
    assert_eq!(r.trash_size(0), trash_after_deletion);
    assert!(r.pending_selection().is_none());
}

#[test]
fn st21_01_can_return_the_just_deleted_adventure_host_itself() {
    // CLAUDE.md rule 25: OnDeletion handlers fire AFTER the carrier moved to
    // trash, so a deleted [ADVENTURE] Digimon host is itself a legal pick.
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "ADV-HOST"]);

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    let view = settle_to_pick(&mut r);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "the deleted ADV-HOST is the only [ADVENTURE] Digimon card in trash"
    );
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("return the deleted host");
    r.auto_resolve().expect("finish");

    assert!(in_hand(&r, 0, "ADV-HOST"));
    assert!(!in_trash(&r, 0, "ADV-HOST"));
}
