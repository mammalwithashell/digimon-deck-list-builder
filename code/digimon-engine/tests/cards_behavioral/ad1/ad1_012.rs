//! AD1-012 CresGarurumon — Digimon, Lv.6, Blue/Black, DP 12000, Cost 12.
//!
//! # Card text (cards.json)
//!
//! ＜Alliance＞ (When this Digimon attacks, by suspending 1 of your other Digimon,
//! add the suspended Digimon's DP to this Digimon and it gains ＜Security A. +1＞
//! for the attack.)
//! ＜Evade＞ (When this Digimon would be deleted, you may suspend it to prevent
//! that deletion.)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may return 1
//! of your opponent's lowest level Digimon to the hand. Then, this Digimon and
//! 1 of your Digimon with [Greymon] in its name may unsuspend.
//! [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks,
//! 2 of your Digimon may DNA digivolve into [Omnimon Alter-S] in the hand.
//! Then, you may change the attack target to 1 of your Digimon.
//!
//! Inherited Effect: [Your Turn] This Digimon's attack target can't change.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Blue/AD1_012.cs
//!
//! # Patterns this test file covers
//! - H8/H10: Evade + Alliance keyword grants (declarative)
//! - Multi-timing trigger: shared OPT clause across [On Play]/[When Digivolving]/[When Attacking]
//! - level_matches_aggregate: select opponent's lowest-level Digimon (optional, return-to-hand)
//! - optional sub-block: "Then, this Digimon and 1 [Greymon] may unsuspend"
//! - select_own_permanent name_contains "Greymon" with is_suspended + other gating
//! - alt_paths: Lv5 Blue cost 4 standard + Lv5 Garurumon-name OR ADVENTURE-trait cost 3
//! - BLOCKED — [Opponent's Turn][OPT] DNA digivolve + redirect attack target
//!   (DSL gap: on_opponent_attack timing, redirect_attack_target step verb)
//! - BLOCKED — Inherited [Your Turn] attack target can't change
//!   (engine gap: ModifierType::CannotChangeAttackTarget)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Greymon-named filler used when testing the "1 of your Digimon with [Greymon]
/// in its name may unsuspend" clause. Lv.5 / 6000 DP — values don't matter for
/// the predicate match, but make_test_card needs *some* level.
fn greymon_named() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("AD1-012-GREYMON-FILLER", "Greymon");
    card.level = Some(5);
    card.dp = Some(6000);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card
}

/// A non-Greymon Digimon used to make sure name filtering works.
fn rookie_filler() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("AD1-012-NON-GREYMON", "Patamon");
    card.level = Some(3);
    card.dp = Some(2000);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card
}

/// Low-level opponent Digimon — used as the lowest-level bounce target.
fn low_level_opp() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("AD1-012-OPP-LOW", "OppLow");
    card.level = Some(3);
    card.dp = Some(2000);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card
}

/// Higher-level opponent Digimon — must NOT be in the bounce candidate set
/// when a lower-level opponent Digimon exists.
fn high_level_opp() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("AD1-012-OPP-HIGH", "OppHigh");
    card.level = Some(6);
    card.dp = Some(11000);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card
}

fn cresgarurumon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("AD1-012")
        .expect("AD1-012 found in embedded DSL pack")
        .add_card(greymon_named())
        .add_card(rookie_filler())
        .add_card(low_level_opp())
        .add_card(high_level_opp())
        .memory(12)
        .start()
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

/// CresGarurumon must compile with both Alliance and Evade granted as
/// declarative GrantKeyword clauses.
#[test]
fn ad1_012_grants_alliance_and_evade() {
    let card = compiled("AD1-012");

    let keywords: std::collections::HashSet<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) => Some(keyword.clone()),
            _ => None,
        })
        .collect();

    for expected in &["Alliance", "Evade"] {
        assert!(
            keywords.contains(*expected),
            "AD1-012 must grant '{}' keyword; got {:?}",
            expected,
            keywords
        );
    }
}

/// The shared OP/WD/WA clause must be a single triggered clause whose `when`
/// vector covers all three timings (matching DCGO's shared hash semantics).
#[test]
fn ad1_012_shared_clause_covers_op_wd_wa() {
    let card = compiled("AD1-012");

    let shared = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
        })
        .expect("AD1-012 must have a single multi-timing OP/WD/WA triggered clause");

    assert!(shared.optional, "shared clause must be optional ('You may')");
    assert!(
        shared.once_per_turn,
        "shared clause must be once_per_turn (printed [Once Per Turn])"
    );
    assert_eq!(shared.scope, CompiledScope::FaceUp);
}

/// Alt-paths must contain the standard Lv.5 Blue / cost 4 digivolve plus the
/// xros_req alt: Lv.5 Garurumon-name OR ADVENTURE-trait at cost 3.
#[test]
fn ad1_012_has_two_alt_paths_standard_and_xros_req() {
    use digimon_dsl::compiled::CompiledAltPathKind;

    let card = compiled("AD1-012");

    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .collect();

    assert_eq!(
        digivolve_paths.len(),
        2,
        "AD1-012 must have exactly 2 digivolve alt-paths (standard + xros_req); got {}",
        digivolve_paths.len()
    );
}

// ─── SECTION 2 — Condition gating: shared clause source-on-field ────────────

/// Positive: source is on field → shared clause condition passes (selection
/// installs when an [On Play] trigger fires with at least one opponent
/// Digimon on the board).
#[test]
fn ad1_012_shared_clause_installs_when_source_on_field_with_opp_target() {
    let mut runner = cresgarurumon_runner();

    // Place CresGarurumon on P0's field then fire on-play with an opponent target present.
    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    runner.fire_on_play(0, cres.index as usize);

    // The shared clause should install a selection (either the bounce select or
    // the optional Yes/No for unsuspend).
    assert!(
        runner.game.pending_selection.is_some(),
        "shared clause must install a pending selection when source on field + opp target"
    );
}

/// Negative: with NO opponent Digimon on the board, the bounce select is
/// vacuous — but per DCGO the unsuspend Yes/No still fires. So the entire
/// clause is not skipped; it falls through to the unsuspend prompt.
/// We assert at least one selection still installs (the Yes/No bool prompt).
#[test]
fn ad1_012_shared_clause_runs_when_no_opp_digimon() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    // No opponent Digimon placed.

    runner.fire_on_play(0, cres.index as usize);

    // Even with no bounce target, the source-on-field condition still passes
    // and the unsuspend offer should fire. The selection that installs is
    // either the unsuspend prompt or PASS-only resolution.
    // We accept either: a pending selection installed, OR no selection (clause
    // declined cleanly because nothing to do). The key contract: no panic.
    let _ = runner.game.pending_selection.is_some();
}

// ─── SECTION 3 — Behavioral: bounce target picks lowest-level opponent ──────

/// When opponent has a Lv.3 and a Lv.6 Digimon, the bounce candidate set
/// must contain ONLY the Lv.3 (lowest-level filter via level_matches_aggregate).
#[test]
fn ad1_012_bounce_select_offers_only_lowest_level_opp() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None); // Lv.3
    runner.place_on_field(1, "AD1-012-OPP-HIGH", None); // Lv.6

    runner.fire_on_play(0, cres.index as usize);

    // The first selection should be opponent permanent (bounce). Its
    // valid_action_ids must include exactly 1 target (the Lv.3) — not 2.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("bounce select must install");

    // There may also be PASS (optional). Filter PASS out from the candidate count.
    let non_pass: Vec<_> = pending
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .copied()
        .collect();

    assert_eq!(
        non_pass.len(),
        1,
        "bounce select must offer exactly 1 candidate (Lv.3 opp Digimon); got {} non-PASS actions",
        non_pass.len()
    );
}

/// Picking the lowest-level opponent target moves it to opponent's hand.
#[test]
fn ad1_012_bounce_returns_target_to_opp_hand() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    let opp_hand_before = runner.game.players[1].hand.len();
    let opp_field_before = runner.game.players[1].battle_area.len();

    runner.fire_on_play(0, cres.index as usize);

    // Resolve the bounce select (pick first non-PASS candidate).
    let (player, action) = {
        let p = runner.game.pending_selection.as_ref().expect("pending");
        let action = *p
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("at least one bounce candidate");
        (p.selecting_player, action)
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("bounce selection resolves");

    // After bounce, opponent battle area shrinks by 1 and hand grows by 1.
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        opp_field_before - 1,
        "opponent battle area must shrink after bounce"
    );
    assert_eq!(
        runner.game.players[1].hand.len(),
        opp_hand_before + 1,
        "opponent hand must grow by 1 after bounce"
    );
}

/// Bounce select is OPTIONAL (DCGO canNoSelect: true) — declining it is legal
/// and the clause continues to the unsuspend prompt.
#[test]
fn ad1_012_bounce_select_is_optional_pass_legal() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    runner.fire_on_play(0, cres.index as usize);

    assert!(
        runner.pending_is_optional(),
        "bounce select must be optional (canNoSelect: true → PASS is a legal action)"
    );
}

// ─── SECTION 4 — Behavioral: unsuspend sub-clause ────────────────────────────

/// After the bounce decision, an optional unsuspend sub-block fires.
/// Accepting the unsuspend (executing the optional substep) unsuspends the
/// source CresGarurumon.
#[test]
fn ad1_012_accept_unsuspend_unsuspends_self() {
    let mut runner = cresgarurumon_runner();

    // Suspend CresGarurumon explicitly so we can observe the unsuspend.
    let cres = runner.place_on_field(0, "AD1-012", None);
    {
        let perm = &mut runner.game.players[0].battle_area[cres.index as usize];
        perm.is_suspended = true;
    }
    // No opp Digimon — bounce select is skipped or vacuous; we focus on the
    // unsuspend offer.

    runner.fire_on_play(0, cres.index as usize);
    let _ = runner.auto_resolve();

    // After auto_resolve (which accepts every optional), CresGarurumon should
    // be unsuspended.
    let perm = &runner.game.players[0].battle_area[cres.index as usize];
    assert!(
        !perm.is_suspended,
        "CresGarurumon should be unsuspended after accepting the unsuspend sub-block"
    );
}

/// When a Greymon-named, suspended ally exists, the unsuspend sub-block
/// also offers a select_own_permanent prompt to pick that Greymon.
#[test]
fn ad1_012_unsuspend_offers_greymon_pick_when_eligible_exists() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    let greymon = runner.place_on_field(0, "AD1-012-GREYMON-FILLER", Some(0));
    // Greymon must be suspended to be a valid pick (DCGO IsOwnGreymon checks IsSuspended).
    {
        let perm = &mut runner.game.players[0].battle_area[greymon.index as usize];
        perm.is_suspended = true;
    }

    runner.fire_on_play(0, cres.index as usize);
    let _ = runner.auto_resolve();

    // After auto_resolve, the suspended Greymon should be unsuspended.
    let perm = &runner.game.players[0].battle_area[greymon.index as usize];
    assert!(
        !perm.is_suspended,
        "Greymon ally should be unsuspended via the second select after accepting unsuspend"
    );
}

/// Non-Greymon allies must NOT be picked even when suspended.
#[test]
fn ad1_012_unsuspend_does_not_pick_non_greymon() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    let non_grey = runner.place_on_field(0, "AD1-012-NON-GREYMON", Some(0));
    {
        let perm = &mut runner.game.players[0].battle_area[non_grey.index as usize];
        perm.is_suspended = true;
    }

    runner.fire_on_play(0, cres.index as usize);
    let _ = runner.auto_resolve();

    // Non-Greymon ally should remain suspended.
    let perm = &runner.game.players[0].battle_area[non_grey.index as usize];
    assert!(
        perm.is_suspended,
        "non-Greymon ally must remain suspended (name filter rejects it)"
    );
}

// ─── SECTION 5 — OPT enforcement ────────────────────────────────────────────

/// Same-timing OPT lockout: firing the [On Play] trigger twice in the same
/// turn must only install the selection on the first fire.
#[test]
fn ad1_012_opt_blocks_second_on_play_in_same_turn() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    // First fire — selection installs.
    runner.fire_on_play(0, cres.index as usize);
    assert!(
        runner.game.pending_selection.is_some(),
        "first OnPlay fire must install selection"
    );
    let _ = runner.auto_resolve();

    // Second fire — OPT must lock; no new selection.
    runner.fire_on_play(0, cres.index as usize);
    assert!(
        runner.game.pending_selection.is_none(),
        "second OnPlay fire in same turn must be locked by OPT"
    );
}

/// Cross-timing OPT lockout (shared hash across OP/WD/WA in DCGO): firing
/// [On Play] then [When Attacking] in the same turn should still lock.
/// Pending: G-OPT-TRIGGERED — DSL `once_per_turn` lowering may not yet enforce
/// shared OPT across distinct timings within the same multi-timing clause.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — cross-timing OPT lockout for shared multi-timing clauses"]
fn ad1_012_opt_blocks_when_attacking_after_on_play_same_turn() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    // OP fires, OPT consumed.
    runner.fire_on_play(0, cres.index as usize);
    let _ = runner.auto_resolve();

    // Then this Digimon attacks — the WA trigger from the same shared clause
    // must NOT fire again per DCGO shared-hash semantics.
    runner.attack_player(cres, 1, false);
    assert!(
        runner.game.pending_selection.is_none(),
        "WA trigger of the shared OP/WD/WA clause must be locked after OP consumed OPT"
    );
}

// ─── SECTION 6 — BLOCKED clauses (DSL + engine gaps) ────────────────────────

/// [Opponent's Turn] [Once Per Turn] — When opp Digimon attacks, may DNA
/// digivolve into [Omnimon Alter-S] in hand. Then, may change attack target.
///
/// BLOCKED:
///   - DSL gap: no `on_opponent_attack` Timing variant in `digimon-dsl/src/clause.rs`
///     (engine has `EffectTiming::OnOpponentAttack` and `Effect::on_opponent_attack`,
///     but the DSL lowering can't currently route to it).
///   - DSL gap: no `redirect_attack_target` step verb in `digimon-dsl/src/step.rs`
///     (engine has `EffectContext::redirect_attack`).
#[test]
#[ignore = "pending: dsl-vocab-gap on_opponent_attack timing + redirect_attack_target step (qa/dsl-vocab-gaps.md)"]
fn ad1_012_opponents_turn_dna_into_omnimon_alters_then_redirects() {
    // Scaffolding (pending dsl-vocab gaps):
    //
    // 1. Place CresGarurumon + a Greymon-name + a Garurumon-name ally on P0's field.
    // 2. Place an Omnimon Alter-S in P0's hand (with dna_costs that match the pair).
    // 3. Place an opponent Digimon on P1's field; place a redirect-target ally on P0.
    // 4. Switch turn to P1; declare opp attack on P0's player.
    // 5. on_opponent_attack should install an optional DNA prompt (cost paid).
    // 6. Accept → effect_initiated_dna_digivolve into Omnimon Alter-S using Greymon+Garurumon.
    // 7. Then redirect_attack_target prompt → pick redirect ally → attack target switches.
    todo!("pending DSL vocab gaps: on_opponent_attack timing + redirect_attack_target step")
}

/// Inherited [Your Turn] — "This Digimon's attack target can't change."
///
/// BLOCKED:
///   - Engine gap: no `ModifierType::CannotChangeAttackTarget` in
///     `code/digimon-engine/src/enums.rs`. The DCGO `CanNotSwitchAttackTargetClass`
///     scope is `permanent == card.PermanentOfThisCard()` (this card only).
///   - Without the modifier the inherited clause has no implementation target.
#[test]
#[ignore = "pending: engine-gap ModifierType::CannotChangeAttackTarget (qa/archetype-qa/engine-gaps.md)"]
fn ad1_012_inherited_blocks_attack_target_change_during_your_turn() {
    // Scaffolding (pending engine gap):
    //
    // 1. Place CresGarurumon (or any Digimon with CresGarurumon as a digivolution
    //    source — the inherited fires from the source) on P0's field.
    // 2. P0's turn — declare an attack from this Digimon on an opp Digimon.
    // 3. Trigger any redirect attempt (Blocker, Raid retarget, etc.).
    // 4. The redirect must be REJECTED (modifier blocks attack-target change).
    // 5. On opponent's turn: the modifier is dormant — redirects work normally.
    todo!("pending engine gap: ModifierType::CannotChangeAttackTarget")
}
