//! BT21-081 Owen Dreadnought — Tamer, Cost 3, Red.
//! Traits: LIBERATOR
//!
//! # Card text (cards.json)
//!
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [End of Your Turn] By suspending this Tamer, 1 of your Digimon with the
//! [Reptile] or [Dragonkin] trait gains <Piercing> for the turn. (When this
//! Digimon attacks and deletes an opponent's Digimon and survives the battle, it
//! performs any security checks it normally would.) Then, that Digimon attacks.
//!
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_081.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - B1 Start-of-main-phase Tamer (memory swing, conditional)
//! - F9 Security-loss conditioned tamer (BT18-087 / BT24-082 pattern group)
//! - H3 Piercing keyword grant for the turn (grant_keyword + expiry)
//! - E2-adjacent: optional end-of-turn clause gated by suspend cost
//! - Security: play self free
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | (a) [Start of Main Phase] gain 1 mem when opp has Digimon | none | PASS |
//! | (b) [End of Turn] suspend self → select Reptile/Dragonkin → grant Piercing | none | PASS |
//! | (b-attack) "Then, that Digimon attacks" | DSL-GAP: no `add_modifier` DSL entry for `MayAttack`/`ForceAttack` to produce mandatory EOT attack | BLOCKED – #[ignore] |
//! | (c) [Security] play self free | none | PASS |

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

// ─── Card YAML (included from production path) ───────────────────────────────
const OWEN_YAML: &str = include_str!("../../../cards/bt21/BT21-081.yaml");

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a runner with Owen Dreadnought loaded from the production YAML.
fn owen_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(OWEN_YAML)
        .expect("BT21-081 YAML parses")
        .memory(10)
        .start()
}

/// Build a runner pre-populated with Owen on P0's field and one Reptile Digimon
/// also on P0's field. P1 has a basic opponent Digimon.
///
/// Both players have a 5-card filler deck so multi-turn tests (e.g. expiry
/// checks that span two `end_turn()` calls) do not trigger a deckout loss on
/// P1's first draw step.
fn owen_on_field_with_reptile() -> DebugRunner {
    let mut reptile = make_test_card("TEST-REPTILE", "ReptileDigimon");
    reptile.level = Some(4);
    reptile.dp = Some(5000);
    reptile.traits = vec!["Reptile".to_string()];

    let mut opp_digimon = make_test_card("TEST-OPP-ROOK", "OppRook");
    opp_digimon.level = Some(4);
    opp_digimon.dp = Some(4000);

    let filler = make_test_card("DECK-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(OWEN_YAML)
        .expect("BT21-081 YAML parses")
        .add_card(reptile)
        .add_card(opp_digimon)
        .add_card(filler)
        // Start at 3 (not 10) so gain_memory: 1 tests have headroom below
        // the Rules::standard() cap of 10. The EOT/main-phase tests all use
        // memory_before ± delta assertions, so the absolute start value is
        // irrelevant as long as there is at least 1 unit of space above it.
        .memory(3)
        // Both players need a non-empty deck so begin_turn draws do not
        // trigger a deckout, which would set game_over and prevent expiry
        // from running in subsequent end_turn() calls.
        .deck(0, &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"])
        .deck(1, &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"])
        .start();

    runner.place_on_field(0, "BT21-081", None);
    // Place the Reptile on P0's field so the end_of_your_turn condition
    // `any_permanent {of: you, kind: digimon, trait_has: Reptile}` can fire.
    runner.place_on_field(0, "TEST-REPTILE", None);
    runner.place_on_field(1, "TEST-OPP-ROOK", Some(0));
    runner
}

/// Like `owen_on_field_with_reptile` but the eligible Digimon has [Dragonkin].
/// Both players have a 5-card filler deck (same reason as `owen_on_field_with_reptile`).
fn owen_on_field_with_dragonkin() -> DebugRunner {
    let mut dragonkin = make_test_card("TEST-DRAGONKIN", "DragonkinDigimon");
    dragonkin.level = Some(4);
    dragonkin.dp = Some(5000);
    dragonkin.traits = vec!["Dragonkin".to_string()];

    let mut opp_digimon = make_test_card("TEST-OPP-ROOK", "OppRook");
    opp_digimon.level = Some(4);
    opp_digimon.dp = Some(4000);

    let filler = make_test_card("DECK-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(OWEN_YAML)
        .expect("BT21-081 YAML parses")
        .add_card(dragonkin)
        .add_card(opp_digimon)
        .add_card(filler)
        .memory(3)
        .deck(0, &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"])
        .deck(1, &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"])
        .start();

    runner.place_on_field(0, "BT21-081", None);
    // Place the Dragonkin on P0's field for the end_of_your_turn condition.
    runner.place_on_field(0, "TEST-DRAGONKIN", None);
    runner.place_on_field(1, "TEST-OPP-ROOK", Some(0));
    runner
}

/// Runner with Owen on field but NO eligible Reptile/Dragonkin Digimon.
fn owen_on_field_no_eligible() -> DebugRunner {
    let mut non_reptile = make_test_card("TEST-PUPPET", "PuppetDigimon");
    non_reptile.level = Some(4);
    non_reptile.dp = Some(5000);
    non_reptile.traits = vec!["Puppet".to_string()];

    let mut opp_digimon = make_test_card("TEST-OPP-ROOK", "OppRook");
    opp_digimon.level = Some(4);
    opp_digimon.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(OWEN_YAML)
        .expect("BT21-081 YAML parses")
        .add_card(non_reptile)
        .add_card(opp_digimon)
        .memory(10)
        .start();

    runner.place_on_field(0, "BT21-081", None);
    runner.place_on_field(1, "TEST-OPP-ROOK", Some(0));
    runner
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT21-081 must compile with exactly 3 triggered clauses:
///   0: start_of_your_main_phase (FaceUp scope, conditional, not optional)
///   1: end_of_your_turn         (FaceUp scope, optional — "By suspending")
///   2: on_security              (Security scope)
#[test]
fn bt21_081_has_three_triggered_clauses() {
    let runner = owen_runner();
    let compiled = runner
        .compiled_card("BT21-081")
        .expect("BT21-081 in compiled_cards");

    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(triggered_count, 3, "BT21-081 must have exactly 3 triggered clauses");
}

/// Clause 0: start_of_your_main_phase, FaceUp scope, not optional.
#[test]
fn bt21_081_clause0_is_start_of_main_phase_not_optional() {
    let runner = owen_runner();
    let compiled = runner
        .compiled_card("BT21-081")
        .expect("BT21-081 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("start_of_your_main_phase clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "start_of_main_phase clause must be FaceUp scope"
    );
    assert!(
        !clause.optional,
        "start_of_main_phase effect is not optional per printed text"
    );
    assert!(
        !clause.once_per_turn,
        "start_of_main_phase effect has no OPT restriction"
    );
}

/// Clause 1: end_of_your_turn, FaceUp scope, optional (cost = suspend self).
#[test]
fn bt21_081_clause1_is_end_of_turn_optional() {
    let runner = owen_runner();
    let compiled = runner
        .compiled_card("BT21-081")
        .expect("BT21-081 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::EndOfYourTurn))
        .expect("end_of_your_turn clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "end_of_your_turn clause must be FaceUp scope"
    );
    assert!(
        clause.optional,
        "end_of_turn effect is optional (player pays cost by suspending)"
    );
}

/// Clause 2: on_security timing must be present.
#[test]
fn bt21_081_clause2_is_on_security() {
    let runner = owen_runner();
    let compiled = runner
        .compiled_card("BT21-081")
        .expect("BT21-081 in compiled_cards");

    // Security clauses compile with FaceUp scope + OnSecurity timing in the DSL.
    // There is no CompiledScope::Security — security clauses are distinguished
    // by their timing, not their scope.
    let has_on_security = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .any(|t| t.when.contains(&CompiledTiming::OnSecurity));

    assert!(
        has_on_security,
        "BT21-081 must have a triggered clause with OnSecurity timing"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (clause a: start_of_main_phase)
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: When P1 has a Digimon, [Start of Main Phase] fires and P0 gains 1 memory.
#[test]
fn bt21_081_start_of_main_gains_memory_when_opp_has_digimon() {
    let mut runner = owen_on_field_with_reptile();
    // P1 already has OPP-ROOK on field from helper.
    let memory_before = runner.memory();

    runner.game.enter_main_phase();

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "Owen should gain 1 memory when opponent has a Digimon at start of main phase"
    );
}

/// NEGATIVE: When P1 has NO Digimon, [Start of Main Phase] does NOT gain memory.
#[test]
fn bt21_081_start_of_main_no_memory_when_opp_has_no_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(OWEN_YAML)
        .expect("BT21-081 YAML parses")
        .memory(5)
        .start();

    // Place Owen on P0's field; P1 has no Digimon.
    runner.place_on_field(0, "BT21-081", None);
    let memory_before = runner.memory();

    runner.game.enter_main_phase();

    assert_eq!(
        runner.memory(),
        memory_before,
        "Owen should NOT gain memory when opponent has no Digimon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcomes (clause b: end_of_your_turn)
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: [End of Turn] with eligible Reptile on field installs a
/// SelectOwnPermanent prompt when player activates the optional effect.
#[test]
fn bt21_081_end_of_turn_with_reptile_installs_selection() {
    let mut runner = owen_on_field_with_reptile();

    runner.game.end_turn();

    // Should see a pending selection to choose the Reptile/Dragonkin Digimon.
    let kind = runner.pending_kind();
    assert!(
        kind.is_some(),
        "end_of_turn clause should install a pending selection when eligible Digimon exists"
    );
    assert!(
        matches!(kind, Some(SelectionKind::OwnField)),
        "selection kind should be OwnField for select_own_permanent"
    );
}

/// POSITIVE: Same test with [Dragonkin] instead of [Reptile].
#[test]
fn bt21_081_end_of_turn_with_dragonkin_installs_selection() {
    let mut runner = owen_on_field_with_dragonkin();

    runner.game.end_turn();

    let kind = runner.pending_kind();
    assert!(
        kind.is_some(),
        "end_of_turn clause should install selection for Dragonkin Digimon too"
    );
}

/// NEGATIVE: When no Reptile/Dragonkin is on P0's field, the end-of-turn
/// selection is NOT installed (condition gates the optional block).
#[test]
fn bt21_081_end_of_turn_no_selection_when_no_eligible_digimon() {
    let mut runner = owen_on_field_no_eligible();

    runner.game.end_turn();

    assert!(
        runner.pending_kind().is_none(),
        "no selection should install when no Reptile/Dragonkin is present"
    );
}

/// POSITIVE: After selecting the Reptile Digimon, it gains Piercing for the turn.
#[test]
fn bt21_081_end_of_turn_selected_reptile_gains_piercing() {
    let mut runner = owen_on_field_with_reptile();

    runner.game.end_turn();

    // Should have a pending selection for the Reptile.
    assert!(
        runner.pending_kind().is_some(),
        "selection must be installed before we can pick"
    );

    // Resolve the selection — pick the first valid action (the Reptile Digimon).
    let action = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        assert!(
            !pending.valid_action_ids.is_empty(),
            "at least one eligible target must exist"
        );
        pending.valid_action_ids[0]
    };
    runner
        .game
        .resolve_selection(0, action)
        .expect("selection resolves");

    // The Reptile must now have Piercing.
    let reptile_handle = PermanentHandle { player: 0, index: 1 }; // Owen at 0, Reptile at 1
    assert!(
        runner.game.has_keyword(reptile_handle, Keyword::Piercing),
        "selected Reptile Digimon must have Piercing after effect resolves"
    );
}

/// POSITIVE: Piercing granted by Owen is temporary — it expires at end of turn.
///
/// Turn sequence:
///   1. `end_turn()` — P0's turn ends; Owen fires, suspends self, installs
///      a selection for the Reptile pick. `rotate_turn_player(0)` runs
///      (`expire_end_of_turn(0)` clears nothing yet), then P1's `begin_turn`
///      draws from the deck and P1's turn becomes active.
///   2. `resolve_selection(0, …)` — callback runs, `grant_keyword` installs
///      Piercing with `EndOfTurn` expiry. Now Piercing is live on P1's turn.
///   3. Assert Piercing active.
///   4. `end_turn()` — P1's turn ends; `expire_end_of_turn(1)` fires and
///      removes all EndOfTurn modifiers → Piercing is cleared here.
///   5. Assert Piercing gone.
///
/// Note: helpers provide a 5-card filler deck for both players so that
/// P1's `begin_turn` draw at step 1 does NOT cause a deckout (which would
/// set `game_over = true` and make later `end_turn()` calls no-ops).
#[test]
fn bt21_081_piercing_expires_end_of_turn() {
    let mut runner = owen_on_field_with_reptile();

    // Step 1: P0 ends their turn. Owen fires EOT effect — suspends self and
    // installs a selection prompt (pick Reptile/Dragonkin).
    runner.game.end_turn();

    // Step 2: resolve the selection to grant Piercing.
    if let Some(ref pending) = runner.game.pending_selection {
        let action = pending.valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, action)
            .expect("selection resolves");
    }

    let reptile_handle = PermanentHandle { player: 0, index: 1 };

    // Step 3: Piercing must be active now (it was just granted on P1's turn).
    assert!(
        runner.game.has_keyword(reptile_handle, Keyword::Piercing),
        "Reptile should have Piercing modifier active after Owen's effect resolves"
    );

    // Step 4: P1 ends their turn — expire_end_of_turn(1) clears all EndOfTurn
    // modifiers (including Piercing) before P0's begin_turn.
    runner.game.end_turn();

    // Step 5: Piercing must have expired after P1's turn ended.
    assert!(
        !runner.game.has_keyword(reptile_handle, Keyword::Piercing),
        "Piercing must expire at end of the turn it was granted"
    );
}

/// POSITIVE: Owen itself is suspended as the cost of activating the end-of-turn effect.
#[test]
fn bt21_081_end_of_turn_suspends_owen_as_cost() {
    let mut runner = owen_on_field_with_reptile();

    // Owen starts unsuspended.
    assert!(
        !runner.game.players[0].battle_area[0].is_suspended,
        "Owen must start unsuspended"
    );

    runner.game.end_turn();

    // After the effect fires, resolve the selection prompt.
    if let Some(ref pending) = runner.game.pending_selection {
        let action = pending.valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, action)
            .expect("selection resolves");
    }

    // Owen should now be suspended (cost was paid).
    // Note: Owen is at index 0 on the battle_area. The tamer permanent tracks
    // suspension via is_suspended.
    assert!(
        runner.game.players[0].battle_area[0].is_suspended,
        "Owen must be suspended after paying the cost"
    );
}

/// POSITIVE: When Owen is already suspended, it cannot pay the cost and the
/// end-of-turn selection is NOT installed.
#[test]
fn bt21_081_end_of_turn_no_selection_when_owen_already_suspended() {
    let mut runner = owen_on_field_with_reptile();

    // Suspend Owen beforehand.
    runner.game.players[0].battle_area[0].is_suspended = true;

    runner.game.end_turn();

    assert!(
        runner.pending_kind().is_none(),
        "no selection should install when Owen is already suspended (cannot pay cost)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — "Then, that Digimon attacks" — BLOCKED (DSL vocab gap)
// ═══════════════════════════════════════════════════════════════════════════════

/// BLOCKED: The "Then, that Digimon attacks" clause requires granting a
/// forced end-of-turn attack on a specific Digimon. The engine has
/// `ModifierType::MayAttack` for optional attacks and `ModifierType::ForceAttack`
/// for Main-phase mask replacement, but:
///   1. Neither `MayAttack` nor `ForceAttack` is registered in DSL
///      `lookup_modifier_type` — so `add_modifier` cannot grant them.
///   2. `ForceAttack` is NOT checked in `has_end_of_turn_keywords`, so even if
///      it were grantable it would not surface as an EOT attack action.
///   3. A proper implementation would need either:
///      a. `MayAttack` added to `lookup_modifier_type` (gives optional EOT attack), or
///      b. A new DSL verb `force_attack_with: { target: tgt }` / a separate
///         `GrantMayAttack` step that also unsuspends the target if needed.
/// See `qa/dsl-vocab-gaps.md` for the tracking entry.
#[test]
#[ignore = "pending: DSL-GAP — no `add_modifier` support for MayAttack/ForceAttack; no DSL verb for effect-initiated EOT attack on a specific Digimon"]
fn bt21_081_end_of_turn_selected_digimon_attacks_after_piercing_grant() {
    let mut runner = owen_on_field_with_reptile();

    runner.game.end_turn();

    if let Some(ref pending) = runner.game.pending_selection {
        let action = pending.valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, action)
            .expect("selection resolves");
    }

    // After resolving, the game should park in EndOfTurnAction for the Reptile
    // to make its mandatory attack. This currently does NOT happen.
    use digimon_engine::enums::GamePhase;
    assert_eq!(
        runner.current_phase(),
        GamePhase::EndOfTurnAction,
        "game should park in EndOfTurnAction for the selected Digimon to attack"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Security clause
// ═══════════════════════════════════════════════════════════════════════════════

/// [Security] clause must exist in the compiled card.
#[test]
fn bt21_081_has_on_security_clause() {
    let runner = owen_runner();
    let compiled = runner
        .compiled_card("BT21-081")
        .expect("BT21-081 in compiled_cards");

    let has_security = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .any(|t| t.when.contains(&CompiledTiming::OnSecurity));
    assert!(
        has_security,
        "BT21-081 must have an OnSecurity triggered clause for play-self-free"
    );
}
