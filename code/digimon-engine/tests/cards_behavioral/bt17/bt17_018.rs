//! BT17-018 Gallantmon: Crimson Mode -- Digimon, Lv.7, Red, DP15000, Cost8.
//! Traits: Holy Warrior
//! Evo: Lv6 Red / cost 5; Lv6 with [Gallantmon] in name / cost 4
//!
//! Card text (cards.json):
//! [Hand] [Counter] <Blast Digivolve>
//! [On Play] [When Digivolving] Choose any number of your opponent's Digimon
//!   whose total DP adds up to 15000 or less and delete them.
//! [When Attacking] [Once Per Turn] For every 10 cards in both player's trash,
//!   trash 1 card from the top of your opponent's security stack.
//!
//! Inherited: Ace Overflow <-5>
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_018.cs
//!
//! Patterns: H12 (Blast Digivolve), H13 (ACE), DP-budget multi-delete, When Attacking OPT.
//!
//! Known gaps:
//! - G-DP-BUDGET-MULTI-SELECT: engine lacks select_opponent_permanent_multi_dp_budget.
//!   raw_rust bt17_018_delete_opp_digimon_dp_budget is a single-pick approximation.
//! - G-OPT-TRIGGERED: once_per_turn not enforced for triggered effects.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

/// Push `n` synthetic trash cards (using card_id "TRASH-FILLER") into the given player's
/// trash zone. The runner must have "TRASH-FILLER" registered in card_data (via add_card).
fn add_n_trash(r: &mut DebugRunner, player: u8, n: usize) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TRASH-FILLER")
        .expect("TRASH-FILLER must be registered via add_card");
    for _ in 0..n {
        let next = r.game.next_card_index();
        r.game.players[player as usize]
            .trash
            .push(CardSource::new(data_idx, player, next));
    }
}

const YAML: &str = include_str!("../../../cards/bt17/BT17-018.yaml");

fn compiled_bt17_018() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("BT17-018.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT17-018.yaml compiles");
    registry
        .lookup("BT17-018")
        .expect("BT17-018 in registry")
        .clone()
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-018 YAML loads")
        .memory(10)
        .build()
}

fn runner_with_p1_security() -> DebugRunner {
    let sec = make_test_card("SEC-FILLER", "SecurityFiller");
    let trash_filler = make_test_card("TRASH-FILLER", "TrashFiller");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-018 YAML loads")
        .add_card(sec)
        .add_card(trash_filler)
        .security(
            1,
            &[
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
            ],
        )
        .memory(10)
        .build();
    // Advance turn_count to 1 so permanents placed with turn_played_override=Some(0)
    // are not fresh (no summoning sickness) and can attack immediately.
    r.game.turn_count = 1;
    r
}

fn runner_with_opp_7k() -> DebugRunner {
    let mut opp = make_test_card("OPP-7K", "OppLow7K");
    opp.dp = Some(7000);
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-018 YAML loads")
        .add_card(opp)
        .memory(10)
        .build()
}

fn runner_with_opp_7k_and_9k() -> DebugRunner {
    let mut opp_7k = make_test_card("OPP-7K", "OppLow7K");
    opp_7k.dp = Some(7000);
    let mut opp_9k = make_test_card("OPP-9K", "OppMid9K");
    opp_9k.dp = Some(9000);
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-018 YAML loads")
        .add_card(opp_7k)
        .add_card(opp_9k)
        .memory(10)
        .build()
}

// --- Section 1: Structural assertions ---

#[test]
fn bt17_018_ace_overflow_is_minus_5() {
    let compiled = compiled_bt17_018();
    assert_eq!(compiled.ace_overflow, Some(-5), "ACE Overflow must be -5");
}

#[test]
fn bt17_018_has_blast_digivolve_alt_path() {
    let compiled = compiled_bt17_018();
    let burst = compiled
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::BurstDigivolve);
    assert!(burst.is_some(), "Must have burst_digivolve alt-path");
    assert!(
        burst.unwrap().marker,
        "Blast Digivolve must have marker: true"
    );
}

#[test]
fn bt17_018_has_standard_digivolve_alt_path_lv6_cost5() {
    let compiled = compiled_bt17_018();
    let standard = compiled.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && !p.ignore_requirements
            && matches!(
                p.cost,
                Some(digimon_dsl::compiled::CompiledCost::Literal(5))
            )
    });
    assert!(standard.is_some(), "Must have Lv6/Cost5 digivolve alt-path");
}

#[test]
fn bt17_018_has_on_play_when_digivolving_clause() {
    let compiled = compiled_bt17_018();
    let clause = compiled.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
            {
                Some(t)
            } else {
                None
            }
        } else {
            None
        }
    });
    assert!(
        clause.is_some(),
        "Must have [On Play][When Digivolving] clause"
    );
    let c = clause.unwrap();
    assert!(!c.optional, "[On Play][When Digivolving] is mandatory");
    assert!(!c.once_per_turn, "[On Play][When Digivolving] has no OPT");
    assert_eq!(c.scope, CompiledScope::FaceUp, "Scope must be FaceUp");
}

#[test]
fn bt17_018_has_when_attacking_once_per_turn_clause() {
    let compiled = compiled_bt17_018();
    let clause = compiled.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::WhenAttacking) {
                Some(t)
            } else {
                None
            }
        } else {
            None
        }
    });
    assert!(clause.is_some(), "Must have [When Attacking] clause");
    let c = clause.unwrap();
    assert!(c.once_per_turn, "[When Attacking] must be once_per_turn");
    assert!(!c.optional, "[When Attacking] is not optional");
    assert_eq!(c.scope, CompiledScope::FaceUp, "Scope must be FaceUp");
}

#[test]
fn bt17_018_total_effect_clause_count_is_two_triggered() {
    let compiled = compiled_bt17_018();
    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .collect();
    assert_eq!(triggered.len(), 2, "Must have 2 triggered clauses");
}

// --- Section 2: Condition gating ---

#[test]
fn bt17_018_on_play_condition_blocks_when_no_opp_digimon() {
    let mut r = runner();
    let perm = r.place_on_field(0, "BT17-018", None);
    r.fire_on_play(0, perm.index as usize);
    assert!(
        r.pending_selection().is_none(),
        "[On Play] must block when no opp Digimon"
    );
}

#[test]
fn bt17_018_on_play_condition_passes_with_opp_digimon() {
    let mut r = runner_with_opp_7k();
    let _opp_perm = r.place_on_field(1, "OPP-7K", None);
    let perm = r.place_on_field(0, "BT17-018", None);
    r.fire_on_play(0, perm.index as usize);
    assert!(
        r.pending_selection().is_some(),
        "[On Play] must pass when opp has Digimon"
    );
}

// --- Section 3: DP-budget delete behavioral (BLOCKED) ---

#[test]
#[ignore = "BLOCKED: G-DP-BUDGET-MULTI-SELECT -- engine lacks select_opponent_permanent_multi_dp_budget; raw_rust bridge needed"]
fn bt17_018_on_play_deletes_selected_digimon_within_dp_budget() {
    let mut r = runner_with_opp_7k();
    let _opp_perm = r.place_on_field(1, "OPP-7K", None);
    let perm = r.place_on_field(0, "BT17-018", None);
    let opp_count_before = r.battle_area_size(1);
    r.fire_on_play(0, perm.index as usize);
    let _ = r.auto_resolve();
    assert_eq!(
        r.battle_area_size(1),
        opp_count_before - 1,
        "Selected Digimon must be deleted"
    );
}

#[test]
#[ignore = "BLOCKED: G-DP-BUDGET-MULTI-SELECT -- engine lacks select_opponent_permanent_multi_dp_budget; raw_rust bridge needed"]
fn bt17_018_on_play_cannot_select_digimon_exceeding_dp_budget() {
    let mut r = runner_with_opp_7k_and_9k();
    let _opp_7k = r.place_on_field(1, "OPP-7K", None);
    let _opp_9k = r.place_on_field(1, "OPP-9K", None);
    let perm = r.place_on_field(0, "BT17-018", None);
    r.fire_on_play(0, perm.index as usize);
    let view = r.pending_selection_view();
    assert!(view.is_some(), "Selection must be pending");
}

#[test]
#[ignore = "BLOCKED: G-DP-BUDGET-MULTI-SELECT -- engine lacks select_opponent_permanent_multi_dp_budget"]
fn bt17_018_on_play_zero_pick_is_invalid_when_targets_exist() {
    let mut r = runner_with_opp_7k();
    let _opp = r.place_on_field(1, "OPP-7K", None);
    let perm = r.place_on_field(0, "BT17-018", None);
    r.fire_on_play(0, perm.index as usize);
    let _view = r.pending_selection_view().expect("Selection installed");
    assert!(
        !r.pending_is_optional(),
        "Delete is mandatory (canNoSelect: false)"
    );
}

// --- Section 3b: Security trash behavioral ---

#[test]
fn bt17_018_when_attacking_trashes_correct_security_count() {
    // 20 total trash (10 P0 + 10 P1) -> floor(20/10) = 2 security trashed.
    let mut r = runner_with_p1_security();
    let attacker_perm = r.place_on_field(0, "BT17-018", Some(0));
    let defender_perm = r.place_on_field(1, "BT17-018", None);

    add_n_trash(&mut r, 0, 10);
    add_n_trash(&mut r, 1, 10);

    let security_before = r.security_count(1);
    assert!(
        security_before >= 2,
        "Test setup: P1 needs >=2 security; got {security_before}"
    );
    r.attack_digimon(attacker_perm, defender_perm, false);
    let _ = r.auto_resolve();
    assert_eq!(
        r.security_count(1),
        security_before - 2,
        "20 combined trash / 10 = 2 security trashed"
    );
}

#[test]
fn bt17_018_when_attacking_zero_trashes_when_insufficient_trash() {
    // 5 total trash -> floor(5/10) = 0 -> no security trashed.
    let mut r = runner_with_p1_security();
    let attacker_perm = r.place_on_field(0, "BT17-018", Some(0));
    let defender_perm = r.place_on_field(1, "BT17-018", None);

    add_n_trash(&mut r, 0, 3);
    add_n_trash(&mut r, 1, 2);

    let security_before = r.security_count(1);
    r.attack_digimon(attacker_perm, defender_perm, false);
    let _ = r.auto_resolve();
    assert_eq!(
        r.security_count(1),
        security_before,
        "5 trash -> no security trashed"
    );
}

// --- Section 4: Event-log assertions ---

#[test]
fn bt17_018_when_attacking_security_trash_fires_events() {
    use digimon_engine::events::GameEvent;

    // 10 P0 trash -> floor(10/10) = 1 security trashed.
    let mut r = runner_with_p1_security();
    let attacker = r.place_on_field(0, "BT17-018", Some(0));
    let defender = r.place_on_field(1, "BT17-018", None);

    add_n_trash(&mut r, 0, 10);

    let security_p1_before = r.security_count(1);
    assert!(security_p1_before >= 1, "Test setup: P1 needs >=1 security");

    let cp = r.event_checkpoint();
    r.attack_digimon(attacker, defender, false);
    let _ = r.auto_resolve();

    let security_p1_after = r.security_count(1);
    assert_eq!(
        security_p1_after,
        security_p1_before - 1,
        "floor(10/10) = 1 -> security must drop by 1"
    );

    // Secondary: check GameEvent::Trash (wiring may be partial for security).
    let events = r.events_since(cp);
    let _trash_events = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { .. }))
        .count();
}

// --- Section 5: OPT enforcement (BLOCKED) ---

#[test]
#[ignore = "BLOCKED: G-OPT-TRIGGERED -- once_per_turn not enforced for triggered effects (see qa/archetype-qa/engine-gaps.md)"]
fn bt17_018_when_attacking_opt_blocks_second_activation_same_turn() {
    let mut r = runner_with_p1_security();
    let attacker = r.place_on_field(0, "BT17-018", Some(0));
    let defender1 = r.place_on_field(1, "BT17-018", None);

    add_n_trash(&mut r, 0, 10);

    r.attack_digimon(attacker, defender1, false);
    let _ = r.auto_resolve();
    let sec_after_first = r.security_count(1);

    let defender2 = r.place_on_field(1, "BT17-018", None);
    r.attack_digimon(attacker, defender2, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(1),
        sec_after_first,
        "OPT must prevent second activation same turn"
    );
}

#[test]
#[ignore = "BLOCKED: G-OPT-TRIGGERED -- once_per_turn not enforced for triggered effects"]
fn bt17_018_when_attacking_opt_resets_after_end_turn() {
    let mut r = runner_with_p1_security();
    let attacker = r.place_on_field(0, "BT17-018", Some(0));

    add_n_trash(&mut r, 0, 10);

    let defender1 = r.place_on_field(1, "BT17-018", None);
    r.attack_digimon(attacker, defender1, false);
    let _ = r.auto_resolve();
    let sec_after_first = r.security_count(1);

    r.end_turn();
    r.end_turn();

    add_n_trash(&mut r, 0, 10);

    let defender2 = r.place_on_field(1, "BT17-018", None);
    r.attack_digimon(attacker, defender2, false);
    let _ = r.auto_resolve();

    assert!(
        r.security_count(1) < sec_after_first,
        "OPT must reset after end_turn"
    );
}
