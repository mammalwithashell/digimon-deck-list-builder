//! BT21-015 Cyclonemon — Digimon, Lv.4, Red, DP 5000, Cost 5.
//! Traits: Dragonkin
//! Evo: Red Lv.3 / 2 memory (standard circle — official Bandai DB / cards.json)
//!
//! # Card text (cards.json)
//!
//! **[Security]** At the end of the battle, play this card without paying the cost.
//!
//! **[On Play] [When Digivolving]** Delete 1 of your opponent's Digimon with
//! 4000 DP or less.
//!
//! **Inherited:** [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_015.cs
//!
//! # Patterns this test covers
//! - Clause 1: [Security] play-self-after-battle (on_security / play_from_security)
//! - Clause 2: [On Play] [When Digivolving] mandatory delete ≤4000 DP opponent
//!   (no PASS legal when eligible target exists)
//! - Clause 3: Inherited [Your Turn] +2000 DP self-aura (D4 declarative aura)
//! - §6 structural: 2 triggered clauses + 1 inherited aura clause

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CYCLONEMON_YAML: &str = include_str!("../../../cards/bt21/BT21-015.yaml");

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_opp_digimon(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.dp = Some(dp);
    c
}

/// Build a runner that already has CARRIER-LV5 registered in card_data.
/// This is needed for `place_cyclonemon_as_source`.
fn runner_with_carrier() -> DebugRunner {
    let mut carrier = make_test_card("CARRIER-LV5", "CarrierLv5");
    carrier.level = Some(5);
    carrier.dp = Some(6000);

    DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .add_card(carrier)
        .memory(10)
        .start()
}

/// Place Cyclonemon as a digivolution source under a dummy carrier on P0's field.
/// Returns the handle of the stacked permanent.
///
/// Stack layout: [BT21-015 (source 0), CARRIER-LV5 (source 1/top)]
fn place_cyclonemon_as_source(runner: &mut DebugRunner) -> PermanentHandle {
    // Place Cyclonemon on the field first (it becomes the base slot).
    let cyclo_handle = runner.place_on_field(0, "BT21-015", Some(0));

    // Simulate digivolving a carrier on top by pushing its CardSource into the
    // Cyclonemon permanent's digivolution stack — same pattern as BT23-005 tests.
    let carrier_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "CARRIER-LV5")
        .expect("CARRIER-LV5 in card_data");
    let next_idx = runner.game.next_card_index();
    let carrier_source = CardSource::new(carrier_data_idx, 0, next_idx);
    let turn = runner.game.turn_count;
    runner.game.players[0].battle_area[cyclo_handle.index as usize].digivolve(carrier_source, turn);

    cyclo_handle
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn bt21_015_compiles_with_three_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT21-015")
        .expect("BT21-015 in compiled_cards");

    assert_eq!(
        compiled.effects.len(),
        3,
        "BT21-015 must have exactly 3 clauses: on_security, on_play/when_digivolving, inherited aura"
    );
}

#[test]
fn bt21_015_has_on_security_triggered_clause() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT21-015")
        .expect("BT21-015 in compiled_cards");

    let on_security = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .any(|t| t.when == vec![CompiledTiming::OnSecurity]);

    assert!(
        on_security,
        "BT21-015 must have a triggered clause with when: on_security"
    );
}

#[test]
fn bt21_015_has_on_play_when_digivolving_triggered_clause() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT21-015")
        .expect("BT21-015 in compiled_cards");

    let delete_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .any(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        });

    assert!(
        delete_clause,
        "BT21-015 must have a triggered clause with when: [on_play, when_digivolving]"
    );
}

#[test]
fn bt21_015_delete_clause_is_not_optional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT21-015")
        .expect("BT21-015 in compiled_cards");

    let delete_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("delete clause exists");

    assert!(
        !delete_clause.optional,
        "Clause 2 must NOT be optional — DCGO canNoSelect=false means the selection is \
         mandatory when a legal target exists"
    );
}

#[test]
fn bt21_015_has_inherited_aura_clause_with_correct_dp_and_scope() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT21-015")
        .expect("BT21-015 in compiled_cards");

    let aura = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) => Some((*scope, *dp_modifier)),
        _ => None,
    });

    let (scope, dp) = aura.expect("BT21-015 must have a Declarative::Aura clause");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "the aura clause must have Inherited scope"
    );
    assert_eq!(dp, Some(2000), "the aura dp_modifier must be +2000 DP");
}

// ─── Section 2: Clause 2 behavioral — [On Play] delete ≤4000 DP ─────────────

/// Positive: play Cyclonemon when opponent has a 4000 DP Digimon.
/// A target-selection prompt for OppField must install.
#[test]
fn bt21_015_on_play_installs_delete_selection_when_eligible_target_exists() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .add_card(make_opp_digimon("OPP-4000", 4000))
        .hand(0, &["BT21-015"])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-4000", Some(0));

    runner.play(0, 0).expect("Cyclonemon plays from hand");

    let kind = runner
        .pending_kind()
        .expect("delete prompt must install after OnPlay");
    assert!(
        matches!(kind, SelectionKind::OppField),
        "expected OppField selection, got {:?}",
        kind
    );
}

/// Positive: after selecting the target, opponent's Digimon is deleted.
#[test]
fn bt21_015_on_play_deletes_selected_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .add_card(make_opp_digimon("OPP-4000-DEL", 4000))
        .hand(0, &["BT21-015"])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-4000-DEL", Some(0));

    assert_eq!(runner.battle_area_size(1), 1, "pre: opponent has 1 Digimon");

    runner.play(0, 0).expect("Cyclonemon plays from hand");
    runner.auto_resolve().expect("delete selection resolves");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "opponent's 4000 DP Digimon must be deleted after selection"
    );
}

/// Negative: opponent has only a 5000 DP Digimon — no eligible target.
/// When the select_opponent_permanent filter finds zero valid targets,
/// the engine must not install a pending selection.
///
/// IGNORED: `dp_lte` predicate is compiled into the IR but not evaluated
/// in `eval_card_fields` (predicate.rs). The DP filter is silently skipped
/// so the selection installs unconditionally regardless of target DP.
/// Tracked as engine gap — unblock when `eval_card_fields` gains dp checks.
#[test]
fn bt21_015_on_play_no_selection_when_no_eligible_target() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .add_card(make_opp_digimon("OPP-5000", 5000))
        .hand(0, &["BT21-015"])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-5000", Some(0));

    runner.play(0, 0).expect("Cyclonemon plays from hand");

    // No eligible target → mandatory select with zero candidates → no prompt installs.
    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when the opponent's only Digimon is >4000 DP; \
         got {:?}",
        runner.pending_kind()
    );
    // Opponent's 5000 DP Digimon must survive.
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "5000 DP Digimon must survive"
    );
}

/// The delete clause must include exactly the 4000 DP target at the boundary.
#[test]
fn bt21_015_on_play_targets_4000_dp_exactly_at_boundary() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .add_card(make_opp_digimon("OPP-4000-EXACT", 4000))
        .hand(0, &["BT21-015"])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-4000-EXACT", Some(0));
    runner.play(0, 0).expect("Cyclonemon plays from hand");

    let pending = runner
        .pending_selection()
        .expect("4000 DP is exactly at threshold — must be a valid target");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "exactly 1 target at the 4000 DP boundary"
    );
}

/// When opponent has two Digimon (one ≤4000, one >4000), only the eligible one
/// appears in the selection.
///
/// IGNORED: same dp_lte engine gap as `bt21_015_on_play_no_selection_when_no_eligible_target`.
/// The 5000 DP Digimon is not filtered from the selection candidates because `eval_card_fields`
/// does not evaluate `pred.dp_lte`. Unblock when predicate.rs gains DP evaluation.
#[test]
fn bt21_015_on_play_filters_ineligible_targets_correctly() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .add_card(make_opp_digimon("OPP-3000", 3000))
        .add_card(make_opp_digimon("OPP-5000-B", 5000))
        .hand(0, &["BT21-015"])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-3000", Some(0));
    runner.place_on_field(1, "OPP-5000-B", Some(0));

    runner.play(0, 0).expect("Cyclonemon plays from hand");

    let pending = runner
        .pending_selection()
        .expect("at least one eligible target");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the 3000 DP Digimon (≤4000) is a valid target; the 5000 DP one is filtered out"
    );
}

// ─── Section 3: Clause 2 behavioral — [When Digivolving] delete ≤4000 DP ────

/// Digivolving into Cyclonemon should also fire the delete clause.
/// We simulate this by directly enqueuing WhenDigivolving on a stacked permanent.
#[test]
fn bt21_015_when_digivolving_installs_delete_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .add_card(make_opp_digimon("OPP-4000-WD", 4000))
        .add_card(make_test_card("BASE-LV3", "BaseLv3"))
        .memory(10)
        .start();

    // Place a Lv3 base on P0's field.
    let base = runner.place_on_field(0, "BASE-LV3", Some(0));

    // Push Cyclonemon as the top card source (simulate digivolve).
    let cyclo_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "BT21-015")
        .expect("BT21-015 in card_data");
    let next = runner.game.next_card_index();
    let cyclo_src = CardSource::new(cyclo_idx, 0, next);
    let turn = runner.game.turn_count;
    runner.game.players[0].battle_area[base.index as usize].digivolve(cyclo_src, turn);

    // Place opponent target.
    runner.place_on_field(1, "OPP-4000-WD", Some(0));

    // Fire the WhenDigivolving event for the stacked permanent.
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(base),
    );
    runner.game.drain_effect_queue();

    let kind = runner
        .pending_kind()
        .expect("delete prompt must install on WhenDigivolving");
    assert!(
        matches!(kind, SelectionKind::OppField),
        "expected OppField, got {:?}",
        kind
    );
}

// ─── Section 4: Clause 1 behavioral — [Security] play-self ───────────────────

/// Structural: on_security clause exists with scope FaceUp (default) and its
/// timing is OnSecurity.
#[test]
fn bt21_015_on_security_clause_has_face_up_scope() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT21-015")
        .expect("BT21-015 in compiled_cards");

    let sec_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("on_security clause exists");

    assert_eq!(
        sec_clause.scope,
        CompiledScope::FaceUp,
        "security clause must have default FaceUp scope"
    );
    assert!(
        !sec_clause.optional,
        "security play-self must NOT be optional (the card plays automatically)"
    );
}

/// Integration: place Cyclonemon in P1's security, attack with P0's Digimon.
/// After the battle, Cyclonemon should enter P1's battle area (play_from_security).
#[test]
fn bt21_015_security_effect_plays_card_onto_field_after_battle() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .memory(10)
        .start();

    // Place attacker on P0's field (turn_played=0 to skip summoning sickness).
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    // Put Cyclonemon at the top of P1's security stack.
    {
        let cyclo_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT21-015")
            .expect("BT21-015 in card_data");
        let next = runner.game.next_card_index();
        let cyclo_src = CardSource::new(cyclo_idx, 1, next);
        // Push so it becomes the top (last) entry.
        runner.game.players[1].security.push(cyclo_src);
    }

    let security_before = runner.security_count(1);
    assert_eq!(
        security_before, 1,
        "pre: P1 has exactly 1 security (Cyclonemon)"
    );

    // P0 attacks P1's security.
    runner.attack_player(attacker, 1, false);
    // Resolve any pending selections from the security effect (e.g., delete clause
    // if no eligible targets → resolves immediately; or eligible target → pick first).
    let _ = runner.auto_resolve();

    // The security card should have been removed.
    assert_eq!(
        runner.security_count(1),
        0,
        "security card (Cyclonemon) must be removed by the attack"
    );

    // Cyclonemon should now be on P1's battle area (played for free by security effect).
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "Cyclonemon must be played onto P1's battle area by its [Security] effect"
    );
}

// ─── Section 5: Clause 3 behavioral — Inherited [Your Turn] +2000 DP ─────────

/// On the controller's turn, the inherited +2000 DP must contribute via
/// source_dp_contribution.
#[test]
fn bt21_015_inherited_dp_active_on_your_turn() {
    let mut runner = runner_with_carrier();
    let carrier_handle = place_cyclonemon_as_source(&mut runner);

    // Source index 0 = Cyclonemon (the base), index 1 = carrier (top card).
    let cyclo_src_idx = 0;

    assert_eq!(
        runner.turn_player(),
        0,
        "precondition: it is player 0's turn"
    );

    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, cyclo_src_idx);

    assert_eq!(
        contribution, 2000,
        "Cyclonemon's inherited +2000 DP must contribute on the controller's turn; \
         got {} instead",
        contribution
    );
}

/// On the opponent's turn, the [Your Turn] gate must suppress the DP buff.
#[test]
fn bt21_015_inherited_dp_inactive_on_opponents_turn() {
    let mut runner = runner_with_carrier();
    let carrier_handle = place_cyclonemon_as_source(&mut runner);

    runner.end_turn();
    assert_eq!(
        runner.turn_player(),
        1,
        "precondition: it is now player 1's turn"
    );

    let cyclo_src_idx = 0;
    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, cyclo_src_idx);

    assert_eq!(
        contribution, 0,
        "Cyclonemon's inherited +2000 DP must NOT contribute on the opponent's turn; \
         got {} instead",
        contribution
    );
}

/// The carrier's top-card slot must NOT carry Cyclonemon's +2000 buff.
#[test]
fn bt21_015_top_card_slot_does_not_carry_inherited_dp() {
    let mut runner = runner_with_carrier();
    let carrier_handle = place_cyclonemon_as_source(&mut runner);

    // Stack: [Cyclonemon (0), CARRIER-LV5 (1/top)]
    // The top-card slot belongs to the carrier, not Cyclonemon.
    let top_src_idx = 1;
    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, top_src_idx);

    assert_eq!(
        contribution, 0,
        "the top-card slot must not carry Cyclonemon's inherited buff"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Digivolve cost fidelity (sister of the BT21-017 free-digivolve bug,
// 2026-07-09). The YAML mis-authored the printed standard circle
// (Red Lv.3 / cost 2 — official Bandai DB + cards.json agree) as an ungated
// `alt_paths: {level_eq: 3} / cost 0`, creating a phantom FREE digivolve
// route from any-colour Lv.3.
// ═══════════════════════════════════════════════════════════════════════════════

/// Digivolving BT21-015 over a red Lv.3 must pay exactly the printed cost (2):
/// no cost-choice prompt (single printed route) and no free route.
#[test]
fn bt21_015_digivolve_pays_printed_cost() {
    use digimon_engine::card_data::EvoCost;
    use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};

    let mut base = make_test_card("BASE-RED3", "BaseRed3");
    base.card_kind = CardKind::Digimon;
    base.level = Some(3);
    base.colors = vec![CardColor::Red];
    base.dp = Some(2000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CYCLONEMON_YAML)
        .expect("BT21-015 YAML parses")
        .add_card(base)
        .add_card(make_test_card("FILLER-DECK", "FILLER-DECK"))
        .hand(0, &["BT21-015"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(10)
        .start();
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT21-015")
            .expect("BT21-015 in card_data");
        runner.game.card_data[idx].evo_costs = vec![EvoCost {
            card_color: 0, // Red
            level: 3,
            memory_cost: 2,
        }];
    }
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
    runner.place_on_field(0, "BASE-RED3", Some(0));

    let mem_before = runner.game.memory;
    let proceeded = runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);

    if let Some(view) = runner.pending_selection_view() {
        assert_ne!(
            view.kind,
            SelectionKind::EffectChoice,
            "BT21-015 has exactly one printed digivolve route (Red Lv.3 / 2); \
             a cost-choice prompt means a phantom route leaked in: {:?}",
            view.effect_choices
        );
    }
    assert!(
        proceeded,
        "digivolve must proceed directly at the single printed cost"
    );
    // ([When Digivolving] delete ≤4000 DP has no opponent target here → no prompt.)
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.memory,
        mem_before - 2,
        "the digivolve cost (2) must be paid in memory — not free"
    );
}
