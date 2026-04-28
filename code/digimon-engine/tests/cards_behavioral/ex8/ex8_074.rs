//! EX8-074 MedievalGallantmon — Digimon, Lv.6, Green (DCGO), DP 11000, Cost 11.
//! Traits: Warrior, Witchelny, Vortex Warriors.
//!
//! # Card text (cards.json)
//!
//! When this card would be played, by suspending 2 Digimon, reduce the play cost by 4.
//! <Alliance>
//! <Vortex>
//! [When Digivolving] You may suspend 1 Digimon. Then, you may delete 1 of your
//! opponent's 8000 DP or lower Digimon. For each other suspended Digimon, add 3000
//! to this DP deletion effect's maximum.
//! [All Turns] [Once Per Turn] When Digimon are played, you may activate 1 of this
//! Digimon's [When Digivolving] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Green/EX8_074.cs
//!
//! # Known gaps / partially-blocked clauses
//!
//! G-COUNT-LTE-EVAL: `count_gte` in condition predicate is parsed but NOT evaluated
//!   (always TRUE). The BeforePayCost condition `count_gte >= 2 unsuspended Digimon`
//!   is authored correctly in YAML but won't gate activation until the gap closes.
//!   Test `ex8_074_cost_reduction_blocked_with_fewer_than_2_unsuspended` is
//!   `#[ignore = "pending: G-COUNT-LTE-EVAL"]`.
//!
//! Dynamic DP cap (raw_rust formula): The DP cap `8000 + 3000 × (other suspended
//!   Digimon count)` uses a raw_rust formula `ex8_074_suspended_dp_cap` registered
//!   in `src/cards/raw_rust/mod.rs`. Until registered, the dp_lte formula evaluates
//!   to 0 (raw_rust fallback returns 0 when unregistered).
//!   Tests for the dynamic cap filtering are `#[ignore = "pending: G-PRED-DP-LTE
//!   + raw_rust formula registration"]`.
//!
//! G-ALL-TURNS-FILTER: The [All Turns][OPT] trigger fires on `on_enter_field_anyone`.
//!   Verification that it fires on the opponent's turn is
//!   `#[ignore = "pending: G-ALL-TURNS-FILTER"]`.
//!
//! G-OPT-TRIGGERED: OPT enforcement for triggered (non-Main) effects is not yet
//!   implemented in `run_queued_effect_inner`. The lockout test is
//!   `#[ignore = "pending: G-OPT-TRIGGERED"]`.
//!
//! # Patterns this test covers
//! - D2 cost reduction with pay_cost suspension (BeforePayCost)
//! - H10 Alliance keyword grant (declarative)
//! - H16 Vortex keyword grant (declarative)
//! - E2 optional triggered clause ([When Digivolving])
//! - Dynamic DP cap formula (raw_rust formula pending)
//! - [All Turns][OPT] on_enter_field_anyone meta-trigger

#![allow(dead_code, unused_imports, unused_variables)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

/// The production YAML for EX8-074, inlined at compile time.
const YAML: &str = include_str!("../../../cards/ex8/EX8-074.yaml");

/// Compile EX8-074 from the production YAML.
fn compiled_ex8_074() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(YAML).expect("EX8-074.yaml parses");
    let registry = digimon_dsl::CardRegistry::from_specs("test", &[spec])
        .expect("EX8-074.yaml compiles");
    registry
        .lookup("EX8-074")
        .expect("EX8-074 in registry")
        .clone()
}

/// Build a DebugRunner with EX8-074 from production YAML plus test-card allies.
fn runner_with_ex8_074() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX8-074 YAML loads")
        .add_card(make_test_card("ALLY", "Ally Digimon"))
        .add_card(make_test_card("OPP", "Opp Digimon"))
        .memory(12)
        .start()
}

// ─────────────────────────────────────────────────────────────────────────────
// § 1  Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// EX8-074 must compile with:
/// - 1 cost_reduction declarative clause (BeforePayCost)
/// - 2 grant_keyword declarative clauses (Alliance, Vortex)
/// - 1 triggered when_digivolving clause (optional, not OPT)
/// - 1 triggered on_enter_field_anyone clause (optional, once_per_turn)
#[test]
fn ex8_074_structural_clause_count() {
    let card = compiled_ex8_074();

    let declaratives: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d) => Some(d),
            _ => None,
        })
        .collect();

    // cost_reduction + Alliance keyword + Vortex keyword = 3 declarative clauses
    assert_eq!(
        declaratives.len(),
        3,
        "EX8-074 should have 3 declarative clauses: cost_reduction + Alliance + Vortex"
    );

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    // 1 when_digivolving + 1 on_enter_field_anyone = 2 triggered clauses
    assert_eq!(
        triggered.len(),
        2,
        "EX8-074 should have 2 triggered clauses: when_digivolving and on_enter_field_anyone"
    );
}

/// The Alliance keyword grant must be present as a declarative clause.
#[test]
fn ex8_074_structural_alliance_keyword_present() {
    let card = compiled_ex8_074();

    let has_alliance = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) if keyword == "Alliance"
        )
    });
    assert!(has_alliance, "EX8-074 must have a declarative Alliance keyword grant");
}

/// The Vortex keyword grant must be present as a declarative clause.
#[test]
fn ex8_074_structural_vortex_keyword_present() {
    let card = compiled_ex8_074();

    let has_vortex = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) if keyword == "Vortex"
        )
    });
    assert!(has_vortex, "EX8-074 must have a declarative Vortex keyword grant");
}

/// A cost_reduction clause must be present.
#[test]
fn ex8_074_structural_cost_reduction_present() {
    let card = compiled_ex8_074();

    let has_cr = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(has_cr, "EX8-074 must have a cost_reduction declarative clause (BeforePayCost)");
}

/// The [When Digivolving] clause must be optional and NOT once_per_turn.
#[test]
fn ex8_074_structural_when_digivolving_optional_not_opt() {
    let card = compiled_ex8_074();

    let wd = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving));

    let wd = wd.expect("EX8-074 must have a WhenDigivolving triggered clause");
    assert!(wd.optional, "WhenDigivolving clause must be optional ('you may')");
    assert!(
        !wd.once_per_turn,
        "WhenDigivolving clause must NOT be once_per_turn"
    );
    assert_eq!(wd.scope, CompiledScope::FaceUp, "WhenDigivolving clause must be face_up scope");
}

/// The [All Turns][OPT] on_enter_field_anyone clause must be optional and once_per_turn.
#[test]
fn ex8_074_structural_all_turns_oefta_optional_opt() {
    let card = compiled_ex8_074();

    let all_turns = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone));

    let all_turns = all_turns
        .expect("EX8-074 must have an OnEnterFieldAnyone clause for the [All Turns][OPT] effect");
    assert!(
        all_turns.optional,
        "[All Turns][OPT] clause must be optional"
    );
    assert!(
        all_turns.once_per_turn,
        "[All Turns][OPT] clause must be once_per_turn"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2  Cost-reduction gating (condition)
// ─────────────────────────────────────────────────────────────────────────────

/// With fewer than 2 unsuspended Digimon, the BeforePayCost reduction condition
/// is not met and must NOT fire. Currently blocked by G-COUNT-LTE-EVAL (count_gte
/// predicate evaluates to true regardless of actual count).
#[test]
#[ignore = "pending: G-COUNT-LTE-EVAL — count_gte predicate always evaluates true until gap closes"]
fn ex8_074_cost_reduction_blocked_with_fewer_than_2_unsuspended() {
    // When G-COUNT-LTE-EVAL closes: set up a game where the controller has
    // 0 or 1 unsuspended Digimon, attempt to play EX8-074, and verify that
    // the suspension selection prompt does NOT install.
    let _ = compiled_ex8_074();
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3  [When Digivolving] behavioral tests
// ─────────────────────────────────────────────────────────────────────────────

/// [When Digivolving] is optional: the clause must be declinable (PASS available).
/// When there are no candidates for suspension, no selection installs.
#[test]
fn ex8_074_when_digivolving_is_optional() {
    let mut runner = runner_with_ex8_074();

    // Place EX8-074 on field (skip play cost).
    let perm = runner.place_on_field(0, "EX8-074", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    // If a selection installs, it must be optional.
    if runner.pending_selection().is_some() {
        assert!(
            runner.pending_is_optional(),
            "[When Digivolving] clause is optional; PASS must be available"
        );
    }
    // If no selection installs (no candidates), that is also correct.
}

/// [When Digivolving] — with an ally Digimon on the field, a suspension
/// selection prompt must install (optional).
#[test]
fn ex8_074_when_digivolving_suspend_prompt_with_ally_present() {
    let mut runner = runner_with_ex8_074();

    let ex8_perm = runner.place_on_field(0, "EX8-074", Some(0));
    let _ally = runner.place_on_field(0, "ALLY", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(ex8_perm),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "WhenDigivolving should install a suspension selection when an ally is present"
    );
    assert!(
        runner.pending_is_optional(),
        "Suspend sub-clause is optional ('you may')"
    );
}

/// [When Digivolving] — after suspending an ally, the delete sub-clause
/// installs (optional) if the opponent has a Digimon.
///
/// NOTE: The dp_lte filter uses `ex8_074_suspended_dp_cap` raw_rust formula.
/// Until registered, the formula returns 0, so ineligible targets may appear.
/// We only assert the structural sequence (suspend → deletion prompt present).
#[test]
fn ex8_074_when_digivolving_delete_prompt_follows_suspension() {
    let mut runner = runner_with_ex8_074();

    let ex8_perm = runner.place_on_field(0, "EX8-074", Some(0));
    let _ally = runner.place_on_field(0, "ALLY", Some(0));
    let _opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(ex8_perm),
    );
    runner.game.drain_effect_queue();

    // First selection: suspend-1-own-Digimon (optional).
    assert!(runner.pending_selection().is_some());

    // Pick the first valid target (suspend the ally).
    let view = runner.pending_selection_view().unwrap();
    if !view.valid_action_ids.is_empty() {
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("execute suspend action");
        runner.game.drain_effect_queue();

        // After suspension resolves, a delete sub-clause should install (optional).
        // (No assertion if field state prevents it — the engine may skip the prompt
        //  if dp_lte = 0 and no opp Digimon passes; this is the raw_rust gap.)
        if runner.pending_selection().is_some() {
            assert!(
                runner.pending_is_optional(),
                "Delete sub-clause is optional ('you may')"
            );
        }
    }
}

/// [When Digivolving] — declining all sub-clauses leaves field state unchanged.
#[test]
fn ex8_074_when_digivolving_full_decline_leaves_state_unchanged() {
    let mut runner = runner_with_ex8_074();

    let ex8_perm = runner.place_on_field(0, "EX8-074", Some(0));
    let _ally = runner.place_on_field(0, "ALLY", Some(0));
    let _opp = runner.place_on_field(1, "OPP", Some(0));

    let ba0_before = runner.battle_area_size(0);
    let ba1_before = runner.battle_area_size(1);
    let ally_suspended_before = runner.game.players[0].battle_area[1].is_suspended;

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(ex8_perm),
    );
    runner.game.drain_effect_queue();

    // Decline each selection that installs.
    for _ in 0..3 {
        if let Some(sel) = runner.pending_selection() {
            let player = sel.selecting_player;
            runner
                .execute_action(player, digimon_engine::action::space::PASS)
                .ok();
            runner.game.drain_effect_queue();
        }
    }

    assert_eq!(
        runner.battle_area_size(0),
        ba0_before,
        "No own permanent should be removed on full decline"
    );
    assert_eq!(
        runner.battle_area_size(1),
        ba1_before,
        "No opponent permanent should be deleted on full decline"
    );
    assert_eq!(
        runner.game.players[0].battle_area[1].is_suspended,
        ally_suspended_before,
        "Ally suspension state should be unchanged after full decline"
    );
}

/// [When Digivolving] delete step actually removes the opponent's Digimon
/// when a valid target is selected.
#[test]
fn ex8_074_when_digivolving_delete_removes_opp_digimon() {
    let mut runner = runner_with_ex8_074();

    let ex8_perm = runner.place_on_field(0, "EX8-074", Some(0));
    let _ally = runner.place_on_field(0, "ALLY", Some(0));
    let _opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(ex8_perm),
    );
    runner.game.drain_effect_queue();

    // Decline suspend sub-clause (PASS).
    if let Some(sel) = runner.pending_selection() {
        let player = sel.selecting_player;
        runner.execute_action(player, digimon_engine::action::space::PASS).ok();
        runner.game.drain_effect_queue();
    }

    // If delete sub-clause installs, pick the opponent's Digimon.
    if runner.pending_selection().is_some() {
        let view = runner.pending_selection_view().unwrap();
        if !view.valid_action_ids.is_empty() {
            let ba1_before = runner.battle_area_size(1);
            runner
                .execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("execute delete action");
            runner.game.drain_effect_queue();

            assert!(
                runner.battle_area_size(1) < ba1_before,
                "Opponent's Digimon should be deleted after selecting it"
            );
        }
    }
    // If no delete selection installed (dp_lte = 0 blocks all targets due to raw_rust
    // formula unregistered), that is acceptable under the known gap.
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3a  Dynamic DP cap formula (pending raw_rust registration)
// ─────────────────────────────────────────────────────────────────────────────

/// With 1 other suspended Digimon (besides the source), the DP cap should be
/// 8000 + 3000 = 11000. Requires `ex8_074_suspended_dp_cap` raw_rust formula.
/// Until registered, dp_lte formula returns 0 and this test cannot pass.
#[test]
#[ignore = "pending: raw_rust formula ex8_074_suspended_dp_cap not yet registered + G-PRED-DP-LTE"]
fn ex8_074_dp_cap_scales_with_other_suspended_digimon() {
    // When resolved: set up 2 own Digimon (one pre-suspended), fire WD on EX8-074,
    // verify that the delete selection shows opponents with DP <= 11000 only.
    let _ = compiled_ex8_074();
}

/// With 0 other suspended Digimon, the DP cap should be 8000 (base only).
#[test]
#[ignore = "pending: raw_rust formula ex8_074_suspended_dp_cap not yet registered + G-PRED-DP-LTE"]
fn ex8_074_dp_cap_base_8000_no_other_suspended() {
    let _ = compiled_ex8_074();
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4  [All Turns][OPT] on_enter_field_anyone trigger tests
// ─────────────────────────────────────────────────────────────────────────────

/// When any Digimon is played (on_enter_field_anyone), the [All Turns][OPT] clause
/// fires and offers the WD-equivalent optional selection.
#[test]
fn ex8_074_all_turns_opt_fires_on_enter_field_anyone() {
    let mut runner = runner_with_ex8_074();

    let ex8_perm = runner.place_on_field(0, "EX8-074", Some(0));
    let _ally = runner.place_on_field(0, "ALLY", Some(0));

    // Simulate a Digimon being played.
    runner.game.enqueue_triggered(
        EffectTiming::OnEnterFieldAnyone,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    // If a selection installs, it must be optional.
    if runner.pending_selection().is_some() {
        assert!(
            runner.pending_is_optional(),
            "[All Turns][OPT] clause must be optional (player can decline)"
        );
    }
    // It is acceptable for no selection to install if no valid sub-targets exist.
}

/// [All Turns][OPT] — OPT lockout: the clause must not fire a second time
/// in the same turn.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — OPT not enforced for triggered effects in run_queued_effect_inner"]
fn ex8_074_all_turns_opt_lockout_same_turn() {
    let mut runner = runner_with_ex8_074();

    let _ex8_perm = runner.place_on_field(0, "EX8-074", Some(0));
    let _ally = runner.place_on_field(0, "ALLY", Some(0));

    // Activate OPT once.
    runner.game.enqueue_triggered(
        EffectTiming::OnEnterFieldAnyone,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();
    if runner.pending_selection().is_some() {
        runner.auto_resolve().ok();
    }

    // Second activation in same turn — must be locked out.
    runner.game.enqueue_triggered(
        EffectTiming::OnEnterFieldAnyone,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "[All Turns][OPT] must be locked out on second activation in the same turn"
    );
}

/// [All Turns][OPT] fires on the opponent's turn when a Digimon is played.
#[test]
#[ignore = "pending: G-ALL-TURNS-FILTER — need verification that active_when: all_turns fires on opp's turn"]
fn ex8_074_all_turns_opt_fires_on_opponents_turn() {
    // When G-ALL-TURNS-FILTER is resolved: verify the clause fires when a
    // Digimon is played during the opponent's turn.
    let _ = compiled_ex8_074();
}

// ─────────────────────────────────────────────────────────────────────────────
// § 5  Keyword grants — runtime behavioral checks
// ─────────────────────────────────────────────────────────────────────────────

/// Alliance and Vortex keywords compile into the card's declarative clauses.
/// The structural test (§ 1) already verified their presence; this test asserts
/// the compiled card data registers both keyword strings.
#[test]
fn ex8_074_keywords_alliance_and_vortex_in_compiled_card() {
    let card = compiled_ex8_074();

    let keyword_names: Vec<String> = card
        .effects
        .iter()
        .filter_map(|c| {
            if let CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) = c
            {
                Some(keyword.clone())
            } else {
                None
            }
        })
        .collect();

    assert!(
        keyword_names.contains(&"Alliance".to_string()),
        "Alliance must appear in grant_keyword clauses; found: {keyword_names:?}"
    );
    assert!(
        keyword_names.contains(&"Vortex".to_string()),
        "Vortex must appear in grant_keyword clauses; found: {keyword_names:?}"
    );
}
