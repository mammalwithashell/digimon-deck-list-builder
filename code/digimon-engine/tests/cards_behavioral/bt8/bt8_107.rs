//! BT8-107 Pandemonium Flame — Option, Purple, Cost 2.
//!
//! # Card text (card_bundles/BT8-107.md — official Bandai DB; matches
//! # cards.json / card image)
//!
//! [Main] You may delete 1 of your Digimon to delete 1 of your opponent's
//! unsuspended Digimon with a level less than or equal to the deleted
//! Digimon's level.
//!
//! Security Effect [Security] Delete 1 of your opponent's unsuspended
//! Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT8/Purple/BT8_107.cs
//!
//! DCGO analysis:
//!   - `EffectTiming.OptionSkill` (the [Main] clause): `ActivateClass` with
//!     `SetUpActivateClass(..., isOptional: false /* the ActivateClass
//!     itself is not optional-flagged — optionality comes from
//!     `canNoSelect: true` on the SelectPermanentEffect for the own-Digimon
//!     pick */)`. `CanSelectPermanentCondition` = "any own battle-area
//!     Digimon". `canNoSelect: true` — the own-Digimon delete pick may be
//!     declined (the printed "you may"). On a successful delete,
//!     `SuccessProcess` computes `CanSelectPermanentCondition1` gating a
//!     SECOND `SelectPermanentEffect` (`canNoSelect: false`, `mode:
//!     Destroy`) over opponent battle-area Digimon with
//!     `permanent1.Level <= permanent.LevelJustBeforeRemoveField` AND
//!     `!permanent1.IsSuspended`. Both prompts install ONLY when a legal
//!     candidate exists (`HasMatchConditionPermanent`).
//!   - `EffectTiming.SecuritySkill` (the [Security] clause):
//!     `CanSelectPermanentCondition` = opponent battle-area Digimon that is
//!     unsuspended (NO level cap). `canNoSelect: false` — mandatory
//!     (Security effects have no opt-out per rules §16).
//!
//! # Patterns this test covers
//! - E2 OPT-adjacent optional decline (the own-Digimon delete-as-cost is
//!   "you may"; declining aborts the whole [Main] effect —
//!   `continue_on_decline: false`, the historical `select_own_permanent`
//!   decline semantic, matches P-169's "by suspending this Tamer" idiom).
//! - F-adjacent: mandatory delete of an opponent **unsuspended** Digimon
//!   ([Security] clause — BT9-108 idiom).
//! - Binding-level comparator: `bind_permanent_property { property: level }`
//!   captures the about-to-be-deleted own Digimon's level as a literal
//!   binding BEFORE `delete_permanent` runs (the handle is unresolvable
//!   after deletion), then `level_lte: { formula: { binding_value: ... } }`
//!   filters the opponent target (BT17-078 `bind_permanent_property` +
//!   `level_eq_binding` idiom, generalized to `level_lte`).
//! - Condition gating: own-Digimon select only installs when the controller
//!   has >=1 own Digimon; opponent-target select only installs when a
//!   legal (unsuspended, level <= deleted level) opponent Digimon exists.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::CardColor;
use digimon_engine::selection::SelectionKind;

// ─── Card-data helpers ────────────────────────────────────────────────────────

/// Own Digimon the controller may delete as the cost. `level` is the pivot
/// for the opponent-target cap.
fn own_digimon(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(id, id, level);
    card.colors = vec![CardColor::Purple];
    card.dp = Some(3000);
    card
}

/// An opponent Digimon candidate. `level` drives the level-cap filter.
fn opp_digimon(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(id, id, level);
    card.colors = vec![CardColor::Red];
    card.dp = Some(4000);
    card
}

/// Base runner: BT8-107 in P0's hand, ample memory, helper card-data
/// registered.
fn pandemonium_flame_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT8-107")
        .expect("BT8-107 YAML parses and compiles")
        .add_card(own_digimon("OWN-L4", 4))
        .add_card(own_digimon("OWN-L2", 2))
        .add_card(opp_digimon("OPP-L4", 4))
        .add_card(opp_digimon("OPP-L5", 5))
        .add_card(opp_digimon("OPP-L3", 3))
        .hand(0, &["BT8-107"])
        .memory(10)
        .start()
}

// ─── Section 1 — Structural assertions ────────────────────────────────────────

#[test]
fn bt8_107_is_purple_option_cost_2() {
    let runner = pandemonium_flame_runner();
    let compiled = runner
        .compiled_card("BT8-107")
        .expect("BT8-107 compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(2));
    assert!(
        compiled.color.contains(&CompiledColor::Purple),
        "Pandemonium Flame must be a purple Option"
    );
}

#[test]
fn bt8_107_has_exactly_one_main_and_one_security_clause() {
    let runner = pandemonium_flame_runner();
    let compiled = runner
        .compiled_card("BT8-107")
        .expect("BT8-107 compiled card");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "BT8-107 ships exactly the [Main] clause and the [Security] clause"
    );
}

#[test]
fn bt8_107_main_clause_is_face_up_main_from_hand() {
    let runner = pandemonium_flame_runner();
    let compiled = runner
        .compiled_card("BT8-107")
        .expect("BT8-107 compiled card");

    let main = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("[Main] clause present");

    assert_eq!(main.scope, CompiledScope::FaceUp);
}

#[test]
fn bt8_107_security_clause_is_on_security() {
    let runner = pandemonium_flame_runner();
    let compiled = runner
        .compiled_card("BT8-107")
        .expect("BT8-107 compiled card");

    let security = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("[Security] clause present");

    // The printed card carries its OWN Security Effect block (not an
    // "Inherited" mirror header like BT9-108) — scope should be face_up.
    assert_eq!(
        security.scope,
        CompiledScope::FaceUp,
        "the [Security] clause is this card's own printed Security Effect"
    );
}

// ─── Section 2 — Condition gating ──────────────────────────────────────────────

#[test]
fn bt8_107_main_with_own_digimon_prompts_optional_delete() {
    let mut runner = pandemonium_flame_runner();
    runner.place_on_field(0, "OWN-L4", Some(0));

    runner.play(0, 0).expect("play BT8-107 from hand");

    let sel = runner
        .pending_selection_view()
        .expect("[Main] must prompt to (optionally) delete an own Digimon");
    assert_eq!(sel.kind, SelectionKind::OwnField);
    assert!(
        sel.is_optional,
        "the printed 'you may delete' makes the own-Digimon pick optional"
    );
}

#[test]
fn bt8_107_main_with_no_own_digimon_installs_no_prompt() {
    let mut runner = pandemonium_flame_runner();
    // No own Digimon placed.

    runner.play(0, 0).expect("play BT8-107 from hand");

    assert!(
        runner.pending_selection().is_none(),
        "no own Digimon exists — the optional delete-as-cost has no legal \
         candidate, so no prompt installs (DCGO HasMatchConditionPermanent)"
    );
}

// ─── Section 3 — Optional decline aborts the whole effect ─────────────────────

#[test]
fn bt8_107_main_declining_own_digimon_delete_aborts_effect() {
    let mut runner = pandemonium_flame_runner();
    runner.place_on_field(0, "OWN-L4", Some(0));
    runner.place_on_field(1, "OPP-L3", Some(0));

    // Snapshot AFTER playing BT8-107 — while an Option resolves its
    // `main_from_hand` clause it occupies a battle-area slot itself
    // (face-up scope), so the pre-play size is not the right baseline
    // (BT9-108 idiom: snapshot immediately around the action under test).
    runner.play(0, 0).expect("play BT8-107 from hand");
    let battle_before_decline_p0 = runner.battle_area_size(0);
    let battle_before_decline_p1 = runner.battle_area_size(1);

    runner
        .execute_action(0, PASS)
        .expect("decline the optional own-Digimon delete");

    assert!(
        runner.pending_selection().is_none(),
        "declining the cost ends the effect — no opponent-target prompt installs"
    );
    assert_eq!(
        runner.battle_area_size(0),
        battle_before_decline_p0,
        "the own Digimon must NOT be deleted when the cost is declined"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OWN-L4"),
        "the declined own Digimon stays on the field"
    );
    assert_eq!(
        runner.battle_area_size(1),
        battle_before_decline_p1,
        "the opponent Digimon must NOT be deleted when the cost is declined"
    );
}

// ─── Section 4 — Accepting the cost: level-cap correctness ────────────────────

#[test]
fn bt8_107_main_accepting_delete_prompts_opponent_target_with_level_cap() {
    let mut runner = pandemonium_flame_runner();
    runner.place_on_field(0, "OWN-L4", Some(0)); // level 4 pivot
    let low = runner.place_on_field(1, "OPP-L3", Some(0)); // level 3 <= 4: legal
    runner.place_on_field(1, "OPP-L5", Some(0)); // level 5 > 4: illegal

    runner.play(0, 0).expect("play BT8-107 from hand");
    let own_sel = runner
        .pending_selection_view()
        .expect("own-Digimon delete prompt installs");
    runner
        .execute_action(0, own_sel.valid_action_ids[0])
        .expect("accept the delete-as-cost");

    let sel = runner
        .pending_selection_view()
        .expect("opponent-target prompt installs after the cost is paid");
    assert_eq!(sel.kind, SelectionKind::OppField);
    assert!(
        !sel.is_optional,
        "once the cost is paid, the opponent delete is mandatory when a legal target exists"
    );
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, low.index as u16)],
        "only the level-3 opponent Digimon (<= the deleted level-4 Digimon) is a legal target; \
         the level-5 opponent Digimon is filtered out by the level cap"
    );
}

#[test]
fn bt8_107_main_equal_level_opponent_is_a_legal_target() {
    let mut runner = pandemonium_flame_runner();
    runner.place_on_field(0, "OWN-L4", Some(0)); // level 4 pivot
    let equal = runner.place_on_field(1, "OPP-L4", Some(0)); // level 4 == 4: legal ("less than or equal")

    runner.play(0, 0).expect("play BT8-107 from hand");
    let own_sel = runner
        .pending_selection_view()
        .expect("own-Digimon delete prompt installs");
    runner
        .execute_action(0, own_sel.valid_action_ids[0])
        .expect("accept the delete-as-cost");

    let sel = runner
        .pending_selection_view()
        .expect("opponent-target prompt installs");
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, equal.index as u16)],
        "an opponent Digimon at EXACTLY the deleted level is a legal target \
         ('level less than or equal to')"
    );
}

#[test]
fn bt8_107_main_higher_level_opponent_only_installs_no_prompt() {
    let mut runner = pandemonium_flame_runner();
    runner.place_on_field(0, "OWN-L2", Some(0)); // level 2 pivot
    runner.place_on_field(1, "OPP-L4", Some(0)); // level 4 > 2: illegal, only candidate

    runner.play(0, 0).expect("play BT8-107 from hand");
    let own_sel = runner
        .pending_selection_view()
        .expect("own-Digimon delete prompt installs");
    runner
        .execute_action(0, own_sel.valid_action_ids[0])
        .expect("accept the delete-as-cost");

    assert!(
        runner.pending_selection().is_none(),
        "the only opponent Digimon exceeds the deleted Digimon's level — \
         no legal target, so no prompt installs and the effect ends"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the too-high-level opponent Digimon must not be deleted"
    );
}

#[test]
fn bt8_107_main_suspended_opponent_digimon_is_not_a_legal_target() {
    let mut runner = pandemonium_flame_runner();
    runner.place_on_field(0, "OWN-L4", Some(0)); // level 4 pivot
    let suspended = runner.place_on_field(1, "OPP-L3", Some(0)); // level 3 <= 4 but suspended
    runner.game.suspend(suspended);

    runner.play(0, 0).expect("play BT8-107 from hand");
    let own_sel = runner
        .pending_selection_view()
        .expect("own-Digimon delete prompt installs");
    runner
        .execute_action(0, own_sel.valid_action_ids[0])
        .expect("accept the delete-as-cost");

    assert!(
        runner.pending_selection().is_none(),
        "a suspended opponent Digimon is not a legal target even though its \
         level qualifies — no prompt installs"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the suspended opponent Digimon must not be deleted"
    );
}

// ─── Section 5 — Full resolution: both deletions land ──────────────────────────

#[test]
fn bt8_107_main_deletes_own_digimon_when_cost_accepted() {
    let mut runner = pandemonium_flame_runner();
    let own = runner.place_on_field(0, "OWN-L4", Some(0));
    runner.place_on_field(1, "OPP-L3", Some(0));

    // Snapshot AFTER playing BT8-107 — while the Option resolves its
    // `main_from_hand` clause it occupies a battle-area slot itself
    // (face-up scope), so the pre-play size is not the right baseline.
    runner.play(0, 0).expect("play BT8-107 from hand");
    let battle_before_delete = runner.battle_area_size(0);

    runner
        .execute_action(0, encode_attack(0, own.index as u16))
        .expect("delete the own Digimon as the cost");

    assert_eq!(
        runner.battle_area_size(0),
        battle_before_delete - 1,
        "the chosen own Digimon is deleted immediately as the cost"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "OWN-L4"),
        "the deleted own Digimon goes to its owner's trash"
    );
}

#[test]
fn bt8_107_main_deletes_chosen_opponent_target_and_leaves_own_deletion_intact() {
    let mut runner = pandemonium_flame_runner();
    runner.place_on_field(0, "OWN-L4", Some(0));
    runner.place_on_field(1, "OPP-L3", Some(0));

    runner.play(0, 0).expect("play BT8-107 from hand");
    let battle_before_delete = runner.battle_area_size(0);

    let own_sel = runner
        .pending_selection_view()
        .expect("own-Digimon delete prompt installs");
    runner
        .execute_action(0, own_sel.valid_action_ids[0])
        .expect("accept the delete-as-cost");

    let opp_sel = runner
        .pending_selection_view()
        .expect("opponent-target prompt installs");
    runner
        .execute_action(0, opp_sel.valid_action_ids[0])
        .expect("delete the chosen opponent Digimon");

    assert_eq!(
        runner.battle_area_size(0),
        battle_before_delete - 1,
        "the own Digimon stays deleted"
    );
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the chosen opponent Digimon is deleted"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "OPP-L3"),
        "the deleted opponent Digimon goes to its owner's trash"
    );
}

#[test]
fn bt8_107_main_multiple_legal_opponent_targets_offers_a_choice() {
    let mut runner = pandemonium_flame_runner();
    runner.place_on_field(0, "OWN-L4", Some(0)); // level 4 pivot
    let low1 = runner.place_on_field(1, "OPP-L3", Some(0)); // level 3 <= 4
    let low2 = runner.place_on_field(1, "OPP-L4", Some(0)); // level 4 <= 4

    runner.play(0, 0).expect("play BT8-107 from hand");
    let own_sel = runner
        .pending_selection_view()
        .expect("own-Digimon delete prompt installs");
    runner
        .execute_action(0, own_sel.valid_action_ids[0])
        .expect("accept the delete-as-cost");

    let sel = runner
        .pending_selection_view()
        .expect("opponent-target prompt installs");
    let mut expected = vec![
        encode_attack(0, low1.index as u16),
        encode_attack(0, low2.index as u16),
    ];
    let mut actual = sel.valid_action_ids.clone();
    expected.sort_unstable();
    actual.sort_unstable();
    assert_eq!(
        actual, expected,
        "both level-eligible opponent Digimon are offered as legal targets"
    );
}

// ─── Section 6 — [Security] clause ─────────────────────────────────────────────
//
// "[Security] Delete 1 of your opponent's unsuspended Digimon." — mandatory,
// NO level cap (unlike the [Main] clause's cost-driven cap).

/// Build a runner where P0 attacks P1, whose single Security card is
/// BT8-107. P0 also has a second, non-attacking Digimon that stays
/// unsuspended — the legal delete target for the [Security] effect (the
/// attacker itself suspends on attack declaration, so `is_unsuspended`
/// filters it out).
fn security_runner() -> (
    DebugRunner,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
) {
    let attacker = own_digimon("ATTACKER", 4);
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-107")
        .expect("BT8-107 YAML parses and compiles")
        .add_card(attacker)
        .add_card(own_digimon("BYSTANDER", 9))
        .security(1, &["BT8-107"])
        .deck(1, &["BYSTANDER", "BYSTANDER", "BYSTANDER"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    // A second unsuspended P0 Digimon that does NOT attack — with a level
    // (9) that would exceed any plausible "deleted Digimon" level cap,
    // proving the [Security] clause has NO level cap (unlike [Main]).
    let bystander = runner.place_on_field(0, "BYSTANDER", Some(0));
    (runner, attacker, bystander)
}

#[test]
fn bt8_107_security_prompts_to_delete_unsuspended_opponent_digimon() {
    let (mut runner, attacker, bystander) = security_runner();

    runner.attack_player(attacker, 1, false);

    let sel = runner
        .pending_selection_view()
        .expect("BT8-107 [Security] must install a delete prompt");
    assert_eq!(sel.kind, SelectionKind::OppField);
    assert_eq!(
        sel.selecting_player, 1,
        "the security owner controls the [Security] effect"
    );
    assert!(
        !sel.is_optional,
        "[Security] effects have no opt-out (rules §16) — the delete is mandatory"
    );
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, bystander.index as u16)],
        "only P0's unsuspended bystander is a legal delete target — the attacker \
         suspended on attack declaration and is filtered out by is_unsuspended, \
         and there is NO level cap on the [Security] clause"
    );
}

#[test]
fn bt8_107_security_with_no_unsuspended_opponent_digimon_installs_no_prompt() {
    let attacker = own_digimon("ATTACKER", 4);
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-107")
        .expect("BT8-107 YAML parses and compiles")
        .add_card(attacker)
        .security(1, &["BT8-107"])
        .deck(1, &["ATTACKER", "ATTACKER", "ATTACKER"])
        .memory(10)
        .start();

    // P0 has only the attacker, which suspends on attack declaration — no
    // unsuspended P0 Digimon remains.
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);

    assert!(
        runner.pending_selection().is_none(),
        "no unsuspended opponent Digimon exists — the [Security] delete has \
         no legal candidate, so no prompt installs"
    );
}

#[test]
fn bt8_107_security_deletes_chosen_unsuspended_digimon() {
    let (mut runner, attacker, _bystander) = security_runner();
    let battle_before = runner.battle_area_size(0);

    runner.attack_player(attacker, 1, false);

    let sel = runner
        .pending_selection_view()
        .expect("[Security] delete prompt installs");
    let pick = sel.valid_action_ids[0];
    runner
        .execute_action(1, pick)
        .expect("security owner deletes the chosen unsuspended Digimon");

    assert_eq!(
        runner.battle_area_size(0),
        battle_before - 1,
        "the [Security] delete removes P0's unsuspended bystander Digimon"
    );
}
