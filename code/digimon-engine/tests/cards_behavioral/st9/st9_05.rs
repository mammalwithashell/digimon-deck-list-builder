//! ST9-05 Paildramon — Digimon, Lv.5, Blue+Green, DP 8000, Cost 8.
//! Traits: Dragonkin.
//!
//! # Card text (cards.json — verbatim)
//!
//! ```text
//! [When Digivolving] When DNA digivolving, return 1 of your opponent's Digimon
//! with 6000 DP or less to the bottom of its owner's deck.
//! [When Attacking] [Once Per Turn] Unsuspend this Digimon.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST9/Blue/ST9_05.cs
//!
//! # Patterns this test covers
//! - G2 DNA digivolve (multi-source materials, dna_costs)
//! - `when: on_dna_digivolve` timing: fires only on DNA digivolve, not regular digivolve
//! - A-bounce: select_opponent_permanent + dp_lte: 6000 + return_to_deck position: bottom
//! - [When Attacking][OPT] Unsuspend self: when_attacking + once_per_turn + unsuspend source
//! - E5: OPT lockout same turn
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | on_dna_digivolve fires (DNA path) | none | PASS |
//! | on_dna_digivolve doesn't fire on regular digivolve | none | PASS |
//! | Return opp Digimon DP<=6000 to deck bottom | none | PASS |
//! | No selection when opp has no DP<=6000 Digimon | none | PASS |
//! | No selection when opp has only DP>6000 Digimon | none | PASS |
//! | [When Attacking][OPT] Unsuspend self | none | PASS |
//! | OPT lockout blocks second activation same turn | none | PASS |

#![allow(unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase};
use digimon_engine::selection::SelectionKind;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Build a DebugRunner with ST9-05 loaded from the embedded DSL pack.
fn paildramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 must be in embedded DSL pack")
        .memory(10)
        .start()
}

/// A simple Lv.4 Blue Digimon — valid DNA material for ST9-05 (material 1).
fn make_lv4_blue(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.colors = vec![CardColor::Blue];
    c
}

/// A simple Lv.4 Green Digimon — valid DNA material for ST9-05 (material 2).
fn make_lv4_green(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.colors = vec![CardColor::Green];
    c
}

/// A Lv.4 Red Digimon with a standard evo-cost — valid for regular digivolve tests.
fn make_lv4_regular_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.colors = vec![CardColor::Blue];
    c.evo_costs = vec![EvoCost {
        card_color: 0, // any color
        level: 3,
        memory_cost: 0,
    }];
    c
}

/// An opponent Digimon with DP <= 6000 — eligible target for Clause 0.
fn make_opp_low_dp(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// An opponent Digimon with DP > 6000 — NOT an eligible target for Clause 0.
fn make_opp_high_dp(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(5);
    c.dp = Some(8000);
    c
}

/// Build a runner wired for a DNA digivolve scenario with ST9-05.
///
/// The runner holds:
/// - ST9-05 in P0's hand (index 0)
/// - MAT-BLUE (Lv.4 Blue) on P0's battle area (placed via `place_on_field`)
/// - MAT-GREEN (Lv.4 Green) on P0's battle area (placed via `place_on_field`)
///
/// Returns the runner. Callers place opponent Digimon before initiating DNA.
fn dna_runner_with_mats() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 must be in embedded DSL pack")
        .add_card(make_lv4_blue("MAT-BLUE"))
        .add_card(make_lv4_green("MAT-GREEN"))
        .hand(0, &["ST9-05"])
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// ST9-05 compiles and is present in the embedded DSL pack.
#[test]
fn st9_05_compiles_and_is_in_pack() {
    let runner = paildramon_runner();
    let compiled = runner.compiled_card("ST9-05");
    assert!(
        compiled.is_some(),
        "ST9-05 must be present in the embedded DSL pack"
    );
}

/// ST9-05 has exactly 2 triggered clauses and no declarative clauses.
#[test]
fn st9_05_has_exactly_two_triggered_clauses() {
    let runner = paildramon_runner();
    let compiled = runner.compiled_card("ST9-05").expect("ST9-05 in pack");

    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered_count, 2,
        "ST9-05 must have exactly 2 triggered clauses (on_dna_digivolve + when_attacking)"
    );

    let declarative_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Declarative(_)))
        .count();
    assert_eq!(
        declarative_count, 0,
        "ST9-05 has no declarative (aura / keyword / cost-reduction) clauses"
    );
}

/// Clause 0 is an on_dna_digivolve triggered clause with FaceUp scope,
/// not optional, no OPT flag.
#[test]
fn st9_05_clause0_is_on_dna_digivolve_mandatory() {
    let runner = paildramon_runner();
    let compiled = runner.compiled_card("ST9-05").expect("ST9-05 in pack");

    let dna_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnDnaDigivolve))
        .expect("on_dna_digivolve clause must exist in ST9-05");

    assert_eq!(
        dna_clause.scope,
        CompiledScope::FaceUp,
        "on_dna_digivolve clause must have FaceUp (own) scope"
    );
    assert!(
        !dna_clause.optional,
        "on_dna_digivolve clause must be mandatory (no 'you may' in printed text)"
    );
    assert!(
        !dna_clause.once_per_turn,
        "on_dna_digivolve clause has no [OPT] in printed text"
    );
}

/// Clause 1 is a when_attacking triggered clause with FaceUp scope, OPT, mandatory.
#[test]
fn st9_05_clause1_is_when_attacking_opt_mandatory() {
    let runner = paildramon_runner();
    let compiled = runner.compiled_card("ST9-05").expect("ST9-05 in pack");

    let wa_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("when_attacking clause must exist in ST9-05");

    assert_eq!(
        wa_clause.scope,
        CompiledScope::FaceUp,
        "when_attacking clause must have FaceUp (own) scope"
    );
    assert!(
        wa_clause.once_per_turn,
        "when_attacking clause must have [Once Per Turn] flag"
    );
    assert!(
        !wa_clause.optional,
        "when_attacking clause must be mandatory (no 'you may' in printed text)"
    );
}

/// ST9-05 has exactly 2 alt-paths: standard digivolve and DNA digivolve.
#[test]
fn st9_05_has_standard_and_dna_digivolve_alt_paths() {
    let runner = paildramon_runner();
    let compiled = runner.compiled_card("ST9-05").expect("ST9-05 in pack");

    let digivolve_count = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(
        digivolve_count, 1,
        "ST9-05 must have exactly 1 standard digivolve alt-path"
    );

    let dna_count = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .count();
    assert_eq!(
        dna_count, 1,
        "ST9-05 must have exactly 1 DNA digivolve alt-path"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 0 behavioral: on_dna_digivolve fires only on DNA digivolve
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: DNA digivolving into ST9-05 installs a selection for an opp
/// Digimon with DP <= 6000 when one is on the field.
///
/// Uses `game.initiate_dna_digivolve` + material selection to drive the
/// DNA digivolve action, then asserts the OppField prompt installs.
#[test]
fn st9_05_dna_digivolve_installs_opp_selection_for_dp_lte_6000() {
    let opp_target = make_opp_low_dp("OPP-LOW", 5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 must be in embedded DSL pack")
        .add_card(make_lv4_blue("MAT-BLUE"))
        .add_card(make_lv4_green("MAT-GREEN"))
        .add_card(opp_target)
        .hand(0, &["ST9-05"])
        .memory(10)
        .start();

    // Place both DNA materials on P0's field.
    runner.place_on_field(0, "MAT-BLUE", Some(0));
    runner.place_on_field(0, "MAT-GREEN", Some(0));

    // Place an opponent Digimon (DP 5000 ≤ 6000) on P1's field.
    runner.place_on_field(1, "OPP-LOW", Some(0));

    // Set to Main phase so DNA action is valid.
    runner.game.current_phase = GamePhase::Main;

    // Initiate the DNA digivolve (player 0, result card index 0 in hand).
    assert!(
        runner.game.initiate_dna_digivolve(0, 0),
        "initiate_dna_digivolve must succeed"
    );

    // The engine should now be parked on a Material selection for the first source.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Material),
        "must prompt for first DNA material"
    );

    // Select MAT-BLUE as material 1 (field slot 0).
    runner
        .execute_action(0, 0)
        .expect("select first DNA material (MAT-BLUE)");

    // Material 2 prompt.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Material),
        "must prompt for second DNA material"
    );

    // Select MAT-GREEN as material 2 (field slot 1 — both materials still on field
    // during selection; slot indices are stable across the two picks).
    runner
        .execute_action(0, 1)
        .expect("select second DNA material (MAT-GREEN)");

    // ST9-05 should now be on P0's field with both materials merged.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "ST9-05 should be the only permanent on P0's field after DNA digivolve"
    );

    // The on_dna_digivolve clause must have installed an OppField selection
    // for the opponent Digimon with DP <= 6000.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "on_dna_digivolve must install an OppField selection for the opp Digimon with DP<=6000"
    );
}

/// Positive: after selecting the target, it is removed from opp's field and
/// placed at the bottom of opp's deck.
#[test]
fn st9_05_dna_digivolve_returns_target_to_bottom_of_opp_deck() {
    let opp_target = make_opp_low_dp("OPP-LOW", 4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 must be in embedded DSL pack")
        .add_card(make_lv4_blue("MAT-BLUE"))
        .add_card(make_lv4_green("MAT-GREEN"))
        .add_card(opp_target)
        .hand(0, &["ST9-05"])
        .memory(10)
        .start();

    runner.place_on_field(0, "MAT-BLUE", Some(0));
    runner.place_on_field(0, "MAT-GREEN", Some(0));
    let _opp_perm = runner.place_on_field(1, "OPP-LOW", Some(0));

    runner.game.current_phase = GamePhase::Main;

    // Drive the DNA digivolve.
    assert!(runner.game.initiate_dna_digivolve(0, 0));
    runner.execute_action(0, 0).expect("select MAT-BLUE");
    runner.execute_action(0, 1).expect("select MAT-GREEN");

    // Verify the OppField selection installed.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));

    let deck_before = runner.game.players[1].deck.len();
    let field_before = runner.game.players[1].battle_area.len();

    // Select the only available opp target (the first valid_action_id).
    let action_id = runner
        .pending_selection()
        .unwrap()
        .valid_action_ids
        .first()
        .copied()
        .expect("at least one valid action for OppField selection");
    runner
        .execute_action(0, action_id)
        .expect("select opp target");

    // Opp's field should now be empty (the Digimon was returned to deck).
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        field_before - 1,
        "opp Digimon must leave the field after return_to_deck"
    );

    // Opp's deck should have grown (by 1 card stack).
    assert!(
        runner.game.players[1].deck.len() > deck_before,
        "opp deck must have grown after the Digimon was returned to its bottom"
    );

    // No further pending selection.
    assert!(
        runner.pending_selection().is_none(),
        "no further selection should be pending after resolving the return effect"
    );
}

/// Negative: when opp has no Digimon with DP <= 6000, no selection installs.
/// The existential guard on select_opponent_permanent silently no-ops.
#[test]
fn st9_05_dna_digivolve_no_selection_when_opp_has_only_high_dp() {
    let opp_high = make_opp_high_dp("OPP-HIGH");

    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 must be in embedded DSL pack")
        .add_card(make_lv4_blue("MAT-BLUE"))
        .add_card(make_lv4_green("MAT-GREEN"))
        .add_card(opp_high)
        .hand(0, &["ST9-05"])
        .memory(10)
        .start();

    runner.place_on_field(0, "MAT-BLUE", Some(0));
    runner.place_on_field(0, "MAT-GREEN", Some(0));
    // Place an opp Digimon with DP 8000 > 6000 — outside the filter.
    runner.place_on_field(1, "OPP-HIGH", Some(0));

    runner.game.current_phase = GamePhase::Main;

    assert!(runner.game.initiate_dna_digivolve(0, 0));
    runner.execute_action(0, 0).expect("select MAT-BLUE");
    runner.execute_action(0, 1).expect("select MAT-GREEN");

    // ST9-05 is on field; no eligible opp target → no selection.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "ST9-05 must be on field"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no selection must install when opp has no Digimon with DP <= 6000"
    );
}

/// Negative: when opp has no Digimon at all, no selection installs.
#[test]
fn st9_05_dna_digivolve_no_selection_when_opp_field_empty() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 must be in embedded DSL pack")
        .add_card(make_lv4_blue("MAT-BLUE"))
        .add_card(make_lv4_green("MAT-GREEN"))
        .hand(0, &["ST9-05"])
        .memory(10)
        .start();

    runner.place_on_field(0, "MAT-BLUE", Some(0));
    runner.place_on_field(0, "MAT-GREEN", Some(0));
    // P1 has no Digimon on field.

    runner.game.current_phase = GamePhase::Main;

    assert!(runner.game.initiate_dna_digivolve(0, 0));
    runner.execute_action(0, 0).expect("select MAT-BLUE");
    runner.execute_action(0, 1).expect("select MAT-GREEN");

    assert!(
        runner.pending_selection().is_none(),
        "no selection must install when opp field is empty"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 0 negative: regular digivolve does NOT fire on_dna_digivolve
// ═══════════════════════════════════════════════════════════════════════════════

/// Negative: a regular (standard) digivolve into ST9-05 must NOT fire
/// the on_dna_digivolve clause. The DSL `when: on_dna_digivolve` timing is
/// distinct from `when_digivolving` and only fires on DNA paths.
///
/// Verified in tests/dsl/phase3e_on_dna_digivolve.rs::non_dna_digivolve_does_not_fire.
#[test]
fn st9_05_regular_digivolve_does_not_fire_on_dna_digivolve_clause() {
    let opp_target = make_opp_low_dp("OPP-LOW", 3000);
    let base = make_lv4_regular_base("BASE-LV4");

    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 must be in embedded DSL pack")
        .add_card(base)
        .add_card(opp_target)
        .hand(0, &["ST9-05"])
        .memory(10)
        .start();

    // Place a Lv.4 base on P0's field so a regular digivolve is possible.
    runner.place_on_field(0, "BASE-LV4", Some(0));
    runner.place_on_field(1, "OPP-LOW", Some(0));

    runner.game.current_phase = GamePhase::Main;

    // Perform a regular digivolve (digivolve_from_hand: player 0, hand index 0,
    // field slot 0, pay cost from source).
    use digimon_engine::enums::PlaySource;
    let did_digivolve = runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    assert!(did_digivolve, "regular digivolve into ST9-05 must succeed");

    // ST9-05 is now on field with BASE-LV4 as a source.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "exactly 1 permanent after regular digivolve"
    );

    // The on_dna_digivolve clause must NOT have installed any selection.
    assert!(
        runner.pending_selection().is_none(),
        "on_dna_digivolve clause must NOT fire during a regular digivolve"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 1 behavioral: [When Attacking][OPT] Unsuspend self
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: when ST9-05 attacks, the when_attacking clause fires and
/// unsuspends ST9-05 (it was suspended by the attack action, then immediately
/// unsuspended by the effect).
///
/// The test places ST9-05 directly on the field (turn_played_override = Some(0)
/// so it can attack immediately), then attacks the opponent player, and asserts
/// ST9-05 is unsuspended after the effect resolves.
#[test]
fn st9_05_when_attacking_unsuspends_itself() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 must be in embedded DSL pack")
        .memory(10)
        .start();

    // Place ST9-05 on P0's field, eligible to attack immediately.
    let paildramon = runner.place_on_field(0, "ST9-05", Some(0));

    // Verify ST9-05 starts unsuspended.
    assert!(
        !runner.game.players[0].battle_area[paildramon.index as usize].is_suspended,
        "ST9-05 must start unsuspended"
    );

    // Set to Main phase.
    runner.game.current_phase = GamePhase::Main;

    // Attack the opponent player (security check).
    // Attacking suspends the attacker; the WhenAttacking effect then fires.
    runner.attack_player(paildramon, 1, false);

    // The on_dna_digivolve clause does not fire here (not a DNA digivolve).
    // The when_attacking clause fires and unsuspends ST9-05.
    // Resolve any pending selections (the effect is mandatory, no PASS needed).
    let _ = runner.auto_resolve();

    // ST9-05 must be unsuspended after the when_attacking effect resolved.
    assert!(
        !runner.game.players[0].battle_area[paildramon.index as usize].is_suspended,
        "ST9-05 must be unsuspended after the [When Attacking][OPT] Unsuspend effect"
    );
}

/// Positive: when ST9-05 attacks a Digimon, the when_attacking clause fires
/// and unsuspends ST9-05.
#[test]
fn st9_05_when_attacking_digimon_unsuspends_itself() {
    let opp_blocker = make_test_card("OPP-DEF", "OppDefender");

    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 must be in embedded DSL pack")
        .add_card(opp_blocker)
        .memory(10)
        .start();

    let paildramon = runner.place_on_field(0, "ST9-05", Some(0));
    let defender = runner.place_on_field(1, "OPP-DEF", Some(0));

    runner.game.current_phase = GamePhase::Main;

    // Attack the opponent Digimon.
    runner.attack_digimon(paildramon, defender, false);
    let _ = runner.auto_resolve();

    // ST9-05 must be unsuspended (it may have survived combat; if it was
    // destroyed, the battle area check would panic — but with default DP
    // values the test Digimon has 0 DP and ST9-05 has 8000, so ST9-05 wins).
    if runner.game.players[0].battle_area.len() > 0
        && paildramon.index < runner.game.players[0].battle_area.len() as u8
    {
        assert!(
            !runner.game.players[0].battle_area[paildramon.index as usize].is_suspended,
            "ST9-05 must be unsuspended after [When Attacking] clause resolves"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT enforcement: second attack same turn does NOT fire again
// ═══════════════════════════════════════════════════════════════════════════════

/// OPT: the [When Attacking][Once Per Turn] clause may only fire once per turn.
/// After the first attack (which fires the unsuspend), a second attack in the
/// same turn must NOT fire the clause again.
///
/// The test drives two sequential attacks: first attack fires the clause;
/// second attack (after re-suspending ST9-05) must produce no selection
/// from the when_attacking clause (the OPT lockout prevents it).
#[test]
fn st9_05_when_attacking_opt_blocks_second_activation_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 must be in embedded DSL pack")
        .memory(10)
        .start();

    let paildramon = runner.place_on_field(0, "ST9-05", Some(0));
    runner.game.current_phase = GamePhase::Main;

    // First attack: fires the when_attacking unsuspend clause.
    runner.attack_player(paildramon, 1, false);
    let _ = runner.auto_resolve();

    // ST9-05 should be unsuspended after first attack's when_attacking clause.
    // Manually suspend it again to simulate a second attack scenario.
    runner.game.players[0].battle_area[paildramon.index as usize].is_suspended = true;

    // Second attack in the same turn. The OPT flag should prevent the
    // when_attacking clause from firing again — ST9-05 stays suspended.
    // (We can't actually attack_player again in the same turn without resetting
    // the attack state; so we drive the when_attacking timing directly via
    // enqueue_triggered to test OPT isolation.)
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    // No pending selection should install (the OPT flag blocks it).
    assert!(
        runner.pending_selection().is_none(),
        "OPT must block the when_attacking clause from firing a second time in the same turn"
    );

    // ST9-05 must still be suspended (the unsuspend did NOT fire).
    assert!(
        runner.game.players[0].battle_area[paildramon.index as usize].is_suspended,
        "ST9-05 must remain suspended when OPT blocks the second activation"
    );
}

/// OPT resets after end of turn: the when_attacking clause fires again in the
/// next turn after the OPT lockout clears.
///
/// This test verifies the OPT reset path via `Permanent::new_turn()` —
/// which clears `effect_activations`. We call `perm.new_turn()` directly to
/// simulate the per-turn reset that `begin_turn()` triggers, then verify the
/// WhenAttacking effect fires again.
///
/// Note: Using `runner.end_turn()` twice to return to P0's turn proved
/// fragile because `begin_turn()` can short-circuit before reaching
/// `new_turn()` if any pending state exists. This test instead calls
/// `perm.new_turn()` directly so the reset is explicit and unambiguous.
#[test]
fn st9_05_when_attacking_opt_resets_after_turn_end() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    let mut runner = DebugRunner::builder()
        .dsl_card("ST9-05")
        .expect("ST9-05 must be in embedded DSL pack")
        .memory(10)
        .start();

    let paildramon = runner.place_on_field(0, "ST9-05", Some(0));
    runner.game.current_phase = GamePhase::Main;

    // Fire WhenAttacking once — OPT counter increments to 1.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    // Verify OPT consumed (second fire in same turn is blocked).
    runner.game.players[0].battle_area[paildramon.index as usize].is_suspended = true;
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.game.players[0].battle_area[paildramon.index as usize].is_suspended,
        "OPT must block the second activation in the same turn"
    );

    // Simulate the per-turn OPT reset that `begin_turn()` / `new_turn()` provides.
    // `perm.new_turn()` clears `effect_activations`, which is what the engine
    // calls at the start of each turn for the turn player.
    runner.game.players[0].battle_area[paildramon.index as usize].new_turn();

    // ST9-05 is still suspended (new_turn does not unsuspend; only unsuspend_all does).
    assert!(
        runner.game.players[0].battle_area[paildramon.index as usize].is_suspended,
        "ST9-05 must still be suspended after new_turn() (new_turn clears activations, not suspension)"
    );

    // Fire WhenAttacking again — OPT counter reset by new_turn(), so it fires.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    // After OPT reset, the when_attacking clause should have fired and unsuspended.
    assert!(
        !runner.game.players[0].battle_area[paildramon.index as usize].is_suspended,
        "OPT must reset after new_turn(); ST9-05 should be unsuspended by the refired clause"
    );
}
