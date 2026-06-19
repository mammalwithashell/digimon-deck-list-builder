//! EX8-070 Zofr Kabus — Option, Purple, Cost 2.
//! Traits: [LIBERATOR]
//!
//! # Card text (cards.json)
//!
//! [Main] By trashing any 1 digivolution card of 1 of your Digimon with the
//! [Mineral] or [Rock] trait, until the end of your opponent's turn, that
//! Digimon gains ＜Collision＞, ＜Piercing＞, ＜Reboot＞, and your opponent's
//! effects can't return it to hands or decks, and it gets +3000 DP.
//!
//! Security Effect [Security] Delete 1 of your opponent's Digimon with the
//! lowest play cost.
//!
//! # DCGO C# reference
//! Absent (DCGO submodule uninitialized).
//!
//! # Patterns this test covers
//! - D6 Flood gate / CannotBeReturnedToHand / CannotBeReturnedToDeck modifiers
//!   with end_of_opponents_turn expiry (BT18-064, BT23-054 pattern)
//! - D1 DP buff +3000 with end_of_opponents_turn expiry
//! - H3 Piercing keyword grant (procedural, temporary)
//! - H5-adjacent Collision keyword grant (procedural, temporary)
//! - H7 Reboot keyword grant (procedural, temporary)
//! - E2 Optional-by-cost: "By trashing" cost gate with select_own_permanent
//!   (Mineral/Rock filter) + select_own_sources
//! - F5-adjacent security delete lowest-play-cost Digimon (raw_rust workaround,
//!   pending G-PLAY-COST-AGGREGATE from qa/dsl-vocab-gaps.md)

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{Keyword, ModifierType};
use digimon_engine::selection::SelectionKind;
use digimon_engine::CardData;

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Build a Mineral-trait Digimon test card.
fn make_mineral_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} Mineral Digi"));
    card.traits.push("Mineral".to_string());
    card.level = Some(4);
    card.dp = Some(5000);
    card
}

/// Build a Rock-trait Digimon test card.
fn make_rock_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} Rock Digi"));
    card.traits.push("Rock".to_string());
    card.level = Some(4);
    card.dp = Some(5000);
    card
}

/// Build a plain source card (Digimon Lv3) with no special traits.
fn make_source_card(id: &str) -> CardData {
    let mut card = make_test_card(id, &format!("{id} Source"));
    card.level = Some(3);
    card.dp = Some(2000);
    card
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .memory(10)
        .build()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX8-070 must have exactly two triggered clauses:
/// - Clause 1: main_from_hand
/// - Clause 2: on_security
#[test]
fn ex8_070_has_two_triggered_clauses() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX8-070")
        .expect("EX8-070 in embedded pack");

    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "EX8-070 must have exactly 2 triggered clauses (main_from_hand + on_security)"
    );
}

/// Clause 1 fires on MainFromHand timing.
#[test]
fn ex8_070_has_main_from_hand_clause() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX8-070")
        .expect("EX8-070 in embedded pack");

    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
        )),
        "EX8-070 must have a MainFromHand clause"
    );
}

/// Clause 2 fires on OnSecurity timing.
#[test]
fn ex8_070_has_on_security_clause() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX8-070")
        .expect("EX8-070 in embedded pack");

    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "EX8-070 must have an OnSecurity clause"
    );
}

/// Main clause is own-scope (FaceUp, not inherited, not security).
#[test]
fn ex8_070_main_clause_is_own_scope() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX8-070")
        .expect("EX8-070 in embedded pack");

    let main_clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("MainFromHand clause must exist");

    assert_eq!(
        main_clause.scope,
        CompiledScope::FaceUp,
        "main clause must be own (FaceUp) scope"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating: main clause requires a Mineral/Rock Digimon
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: playing EX8-070 with a Mineral Digimon + source on field prompts
/// the player to select a target Digimon (OwnField selection).
#[test]
fn ex8_070_main_prompts_selection_with_mineral_digimon() {
    let mineral_digi = make_mineral_digimon("MINERAL-TGT");
    let source_card = make_source_card("SRC-M");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(source_card)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "MINERAL-TGT", None);
    runner.push_source(host, "SRC-M");

    runner.play(0, 0);

    // Should prompt for a target Digimon selection (OwnField).
    let kind = runner
        .pending_kind()
        .expect("Selection must be pending after playing EX8-070");
    assert!(
        matches!(kind, SelectionKind::OwnField),
        "EX8-070 main must prompt for OwnField (Mineral/Rock Digimon selection), got {kind:?}"
    );
}

/// Positive: playing EX8-070 with a Rock Digimon + source on field prompts
/// the player to select a target Digimon.
#[test]
fn ex8_070_main_prompts_selection_with_rock_digimon() {
    let rock_digi = make_rock_digimon("ROCK-TGT");
    let source_card = make_source_card("SRC-R");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(rock_digi)
        .add_card(source_card)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "ROCK-TGT", None);
    runner.push_source(host, "SRC-R");

    runner.play(0, 0);

    let kind = runner
        .pending_kind()
        .expect("Selection must be pending after playing EX8-070 with Rock Digimon");
    assert!(
        matches!(kind, SelectionKind::OwnField),
        "EX8-070 main must prompt for OwnField with Rock Digimon, got {kind:?}"
    );
}

/// Negative: playing EX8-070 with no Digimon on field installs no selection
/// and has no effect.
#[test]
fn ex8_070_main_no_selection_when_no_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when no Mineral/Rock Digimon is on field"
    );
}

/// Negative: playing EX8-070 with a non-Mineral/non-Rock Digimon on field
/// installs no selection (filter excludes it).
#[test]
fn ex8_070_main_no_selection_when_digimon_lacks_mineral_rock_trait() {
    let mut plain_digi = make_test_card("PLAIN-DIGI", "Plain Digi");
    plain_digi.level = Some(4);
    plain_digi.dp = Some(5000);
    // traits: none matching Mineral or Rock

    let source_card = make_source_card("SRC-PLAIN");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(plain_digi)
        .add_card(source_card)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "PLAIN-DIGI", None);
    runner.push_source(host, "SRC-PLAIN");

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "EX8-070 must not prompt when only non-Mineral/Rock Digimon are on field"
    );
}

/// Negative: playing EX8-070 with a Mineral Digimon but NO digivolution sources
/// installs no selection (no source to trash).
#[test]
fn ex8_070_main_no_selection_when_mineral_digimon_has_no_sources() {
    let mineral_digi = make_mineral_digimon("MINERAL-NO-SRC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(mineral_digi)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    // Place the Digimon with no extra sources (single-card stack, no push_source).
    runner.place_on_field(0, "MINERAL-NO-SRC", None);

    runner.play(0, 0);

    // The Digimon has no digivolution sources (single card = stack_size_gte:2 fails),
    // so the select_own_permanent filter excludes it.
    assert!(
        runner.pending_selection().is_none(),
        "EX8-070 must not prompt when Mineral Digimon has no digivolution sources to trash"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: keyword grants after cost payment
// ═══════════════════════════════════════════════════════════════════════════════

/// After selecting a Mineral Digimon and trashing 1 source, the target gains
/// Collision keyword.
#[test]
fn ex8_070_grants_collision_after_trash() {
    let mineral_digi = make_mineral_digimon("MINERAL-COL");
    let source_card = make_source_card("SRC-COL");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(source_card)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "MINERAL-COL", None);
    runner.push_source(host, "SRC-COL");

    runner.play(0, 0);

    // Select the Mineral Digimon as target.
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    runner.auto_resolve().expect("target selection resolves");

    // Source selection may follow.
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("source selection resolves");
    }

    assert!(
        runner.game.has_keyword(host, Keyword::Collision),
        "EX8-070 must grant Collision to the target Digimon"
    );
}

/// After playing EX8-070, target Digimon gains Piercing keyword.
#[test]
fn ex8_070_grants_piercing_after_trash() {
    let mineral_digi = make_mineral_digimon("MINERAL-PIE");
    let source_card = make_source_card("SRC-PIE");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(source_card)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "MINERAL-PIE", None);
    runner.push_source(host, "SRC-PIE");

    runner.play(0, 0);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    runner.auto_resolve().expect("target selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("source selection resolves");
    }

    assert!(
        runner.game.has_keyword(host, Keyword::Piercing),
        "EX8-070 must grant Piercing to the target Digimon"
    );
}

/// After playing EX8-070, target Digimon gains Reboot keyword.
#[test]
fn ex8_070_grants_reboot_after_trash() {
    let mineral_digi = make_mineral_digimon("MINERAL-RBT");
    let source_card = make_source_card("SRC-RBT");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(source_card)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "MINERAL-RBT", None);
    runner.push_source(host, "SRC-RBT");

    runner.play(0, 0);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    runner.auto_resolve().expect("target selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("source selection resolves");
    }

    assert!(
        runner.game.has_keyword(host, Keyword::Reboot),
        "EX8-070 must grant Reboot to the target Digimon"
    );
}

/// After playing EX8-070, target Digimon gets +3000 DP.
#[test]
fn ex8_070_grants_plus_3000_dp_to_target() {
    let mineral_digi = make_mineral_digimon("MINERAL-DP");
    let source_card = make_source_card("SRC-DP");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(source_card)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "MINERAL-DP", None);
    runner.push_source(host, "SRC-DP");

    let dp_before = runner.dp_of(host).unwrap_or(0);

    runner.play(0, 0);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    runner.auto_resolve().expect("target selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("source selection resolves");
    }

    let dp_after = runner.dp_of(host).unwrap_or(0);
    assert_eq!(
        dp_after - dp_before,
        3000,
        "EX8-070 must give +3000 DP to the target Digimon (before: {dp_before}, after: {dp_after})"
    );
}

/// After playing EX8-070, target Digimon gets CannotBeReturnedToHand modifier.
#[test]
fn ex8_070_grants_cannot_be_returned_to_hand_modifier() {
    let mineral_digi = make_mineral_digimon("MINERAL-RTH");
    let source_card = make_source_card("SRC-RTH");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(source_card)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "MINERAL-RTH", None);
    runner.push_source(host, "SRC-RTH");

    runner.play(0, 0);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    runner.auto_resolve().expect("target selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("source selection resolves");
    }

    assert!(
        runner
            .game
            .modifiers
            .has(host, ModifierType::CannotBeReturnedToHand),
        "EX8-070 must grant CannotBeReturnedToHand to target"
    );
}

/// After playing EX8-070, target Digimon gets CannotBeReturnedToDeck modifier.
#[test]
fn ex8_070_grants_cannot_be_returned_to_deck_modifier() {
    let mineral_digi = make_mineral_digimon("MINERAL-RTD");
    let source_card = make_source_card("SRC-RTD");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(source_card)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "MINERAL-RTD", None);
    runner.push_source(host, "SRC-RTD");

    runner.play(0, 0);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    runner.auto_resolve().expect("target selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("source selection resolves");
    }

    assert!(
        runner
            .game
            .modifiers
            .has(host, ModifierType::CannotBeReturnedToDeck),
        "EX8-070 must grant CannotBeReturnedToDeck to target"
    );
}

/// Source card is moved to trash after being selected as cost.
#[test]
fn ex8_070_trashes_selected_source_card() {
    let mineral_digi = make_mineral_digimon("MINERAL-TRASH");
    let source_card = make_source_card("SRC-TRASH");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(source_card)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "MINERAL-TRASH", None);
    runner.push_source(host, "SRC-TRASH");

    let sources_before = runner.game.player(0).battle_area[host.index as usize]
        .card_sources
        .len();
    assert!(
        sources_before >= 2,
        "host must have at least 1 digivolution source"
    );

    let trash_before = runner.trash_size(0);

    runner.play(0, 0);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    runner.auto_resolve().expect("target selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("source selection resolves");
    }

    let sources_after = runner.game.player(0).battle_area[host.index as usize]
        .card_sources
        .len();
    let trash_after = runner.trash_size(0);

    assert_eq!(
        sources_after,
        sources_before - 1,
        "one digivolution source must be removed from the Digimon's stack"
    );
    assert!(
        trash_after > trash_before,
        "the trashed source must land in trash (trash before: {trash_before}, after: {trash_after})"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Expiry: modifiers and keywords expire at end of opponent's turn
// ═══════════════════════════════════════════════════════════════════════════════

/// Keywords granted by EX8-070 persist through the end of the current (your)
/// turn — they last until end of OPPONENT's turn, not your own.
#[test]
fn ex8_070_buffs_persist_through_own_turn_end() {
    let mineral_digi = make_mineral_digimon("MINERAL-EXP");
    let source_card = make_source_card("SRC-EXP");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(source_card)
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "MINERAL-EXP", None);
    runner.push_source(host, "SRC-EXP");

    runner.play(0, 0);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    runner.auto_resolve().expect("target selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("source selection resolves");
    }

    // Buffs should be present immediately.
    assert!(runner.game.has_keyword(host, Keyword::Collision));
    assert!(runner
        .game
        .modifiers
        .has(host, ModifierType::CannotBeReturnedToHand));

    // End player 0's turn — buffs should still be active during opponent's turn.
    runner.end_turn();

    assert!(
        runner.game.has_keyword(host, Keyword::Collision),
        "Collision must still be active at start of opponent's turn (expiry is end_of_opponents_turn)"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(host, ModifierType::CannotBeReturnedToHand),
        "CannotBeReturnedToHand must still be active at start of opponent's turn"
    );
}

/// After opponent's turn ends, all EX8-070 buffs expire.
#[test]
fn ex8_070_buffs_expire_after_opponents_turn_ends() {
    let mineral_digi = make_mineral_digimon("MINERAL-EXP2");
    let source_card = make_source_card("SRC-EXP2");
    // P1 needs at least one deck card so begin_turn draw does not deckout and
    // abort the turn before expire_end_of_turn(1) fires.
    let p1_filler = make_test_card("EXP2-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(source_card)
        .add_card(p1_filler)
        .hand(0, &["EX8-070"])
        .deck(1, &["EXP2-PAD", "EXP2-PAD", "EXP2-PAD"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "MINERAL-EXP2", None);
    let dp_base = runner.dp_of(host).unwrap_or(0);
    runner.push_source(host, "SRC-EXP2");

    runner.play(0, 0);
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ));
    runner.auto_resolve().expect("target selection resolves");
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("source selection resolves");
    }

    // End player 0's turn then player 1's turn.
    runner.end_turn(); // → player 1's turn
    runner.end_turn(); // → back to player 0; modifiers should be expired

    assert!(
        !runner.game.has_keyword(host, Keyword::Collision),
        "Collision must expire after opponent's turn ends"
    );
    assert!(
        !runner.game.has_keyword(host, Keyword::Piercing),
        "Piercing must expire after opponent's turn ends"
    );
    assert!(
        !runner.game.has_keyword(host, Keyword::Reboot),
        "Reboot must expire after opponent's turn ends"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(host, ModifierType::CannotBeReturnedToHand),
        "CannotBeReturnedToHand must expire after opponent's turn ends"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(host, ModifierType::CannotBeReturnedToDeck),
        "CannotBeReturnedToDeck must expire after opponent's turn ends"
    );
    let dp_after_expiry = runner.dp_of(host).unwrap_or(0);
    assert_eq!(
        dp_after_expiry,
        dp_base,
        "DP must return to base after opponent's turn ends (base: {dp_base}, got: {dp_after_expiry})"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Security clause: delete 1 opponent Digimon with lowest play cost
//
// The security delete uses `select_opponent_permanent { selector: lowest_play_cost }`
// + `delete_permanent`. The selector constrains the legal target set to the
// minimum-play-cost opponent Digimon; when several tie, the controller chooses
// among them through `pending_selection` (no-approximations — "Delete 1 of"
// requires a player choice). This replaced the former raw_rust auto-pick which
// deleted the lowest battle-area index (a rule-17 violation).
// ═══════════════════════════════════════════════════════════════════════════════

fn opponent_card_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[1]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Security effect deletes the (unique) opponent Digimon with the lowest play cost.
#[test]
fn ex8_070_security_deletes_lowest_cost_digimon() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    let mut cheap = make_test_card("CHEAP-S", "Cheap Security Target");
    cheap.play_cost = 2;
    cheap.level = Some(4);
    let mut expensive = make_test_card("EXPEN-S", "Expensive Security Target");
    expensive.play_cost = 8;
    expensive.level = Some(6);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(cheap)
        .add_card(expensive)
        .memory(10)
        .start();

    // For an Option card security test, use place_on_field + enqueue_triggered
    // with SecuritySkill timing (the engine timing for on_security effects).
    let security_handle = runner.place_on_field(0, "EX8-070", None);
    runner.place_on_field(1, "CHEAP-S", Some(0));
    runner.place_on_field(1, "EXPEN-S", Some(0));

    assert_eq!(runner.battle_area_size(1), 2);

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(security_handle),
    );
    runner.game.drain_effect_queue();

    // Only the cheap Digimon is at the minimum play cost, so it is the sole legal
    // target. Resolve the selection (whether the engine exposes a 1-option
    // pending selection or auto-resolves the forced target).
    if runner.game.pending_selection.is_some() {
        runner.auto_resolve().expect("resolve lowest-cost target");
    }

    // The cheap Digimon (cost 2) is deleted; expensive (cost 8) survives.
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "security delete must remove the lowest-play-cost Digimon"
    );
    assert_eq!(
        opponent_card_ids(&runner),
        vec!["EXPEN-S".to_string()],
        "the surviving Digimon must be the higher-cost one (cheap was deleted, not auto-index)"
    );
}

/// When multiple opponent Digimon tie for the lowest play cost, the controller
/// must be offered a CHOICE (pending_selection) rather than an auto-pick.
#[test]
fn ex8_070_security_tie_exposes_player_choice() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    let mut a = make_test_card("TIE-A", "Tie Target A");
    a.play_cost = 3;
    a.level = Some(4);
    let mut b = make_test_card("TIE-B", "Tie Target B");
    b.play_cost = 3;
    b.level = Some(4);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .add_card(a)
        .add_card(b)
        .memory(10)
        .start();

    let security_handle = runner.place_on_field(0, "EX8-070", None);
    runner.place_on_field(1, "TIE-A", Some(0));
    runner.place_on_field(1, "TIE-B", Some(0));
    assert_eq!(runner.battle_area_size(1), 2);

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(security_handle),
    );
    runner.game.drain_effect_queue();

    // Both Digimon share the lowest play cost → the choice MUST be surfaced.
    assert!(
        runner.game.pending_selection.is_some(),
        "a tie at the lowest play cost must surface a player choice, not auto-pick"
    );

    // Resolve the choice; exactly one of the tied Digimon is deleted.
    runner
        .auto_resolve()
        .expect("resolve the tied lowest-cost target selection");
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "exactly one tied Digimon is deleted after the player chooses"
    );
}

/// Security effect: no crash when opponent has no Digimon.
#[test]
fn ex8_070_security_no_crash_when_opponent_has_no_digimon() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 YAML parses and compiles")
        .memory(10)
        .start();

    let security_handle = runner.place_on_field(0, "EX8-070", None);

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(security_handle),
    );
    runner.game.drain_effect_queue();

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.battle_area_size(1), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Compilation smoke test
// ═══════════════════════════════════════════════════════════════════════════════

/// EX8-070 YAML must compile and appear in the embedded DSL pack.
#[test]
fn ex8_070_yaml_compiles_without_error() {
    let runner = base_runner();
    let card = runner.compiled_card("EX8-070");
    assert!(
        card.is_some(),
        "EX8-070 must be present in the embedded DSL pack (YAML must parse + compile)"
    );
}
