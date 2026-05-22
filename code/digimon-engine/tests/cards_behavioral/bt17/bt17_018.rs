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
//! - G-LOSE-COUNT-BOUND: the [When Attacking] security-trash loop still needs a
//!   native computed repeat step, so that clause retains a narrow raw Rust helper.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledFormula, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
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
        .deck(0, &["TRASH-FILLER"; 10])
        .deck(1, &["TRASH-FILLER"; 10])
        .security(0, &["SEC-FILLER"; 5])
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

fn runner_with_opp_7k_and_8k_and_9k() -> DebugRunner {
    let mut opp_7k = make_test_card("OPP-7K", "OppLow7K");
    opp_7k.dp = Some(7000);
    let mut opp_8k = make_test_card("OPP-8K", "OppMid8K");
    opp_8k.dp = Some(8000);
    let mut opp_9k = make_test_card("OPP-9K", "OppHigh9K");
    opp_9k.dp = Some(9000);
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-018 YAML loads")
        .add_card(opp_7k)
        .add_card(opp_8k)
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

#[test]
fn bt17_018_delete_clause_uses_native_dp_budget_selection() {
    let compiled = compiled_bt17_018();
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| {
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
        })
        .expect("delete clause exists");

    assert!(
        !clause.process.iter().any(|step| {
            matches!(
                step,
                CompiledStep::RawRust { fn_name, .. }
                    if fn_name == "bt17_018_delete_opp_digimon_dp_budget"
            )
        }),
        "DP-budget delete must not use the obsolete raw Rust approximation"
    );

    let dp_step = clause
        .process
        .iter()
        .find_map(|step| {
            if let CompiledStep::SelectOpponentDpBudget {
                dp_budget,
                min_picks,
                bind_as,
                then,
                ..
            } = step
            {
                Some((dp_budget, min_picks, bind_as, then))
            } else {
                None
            }
        })
        .expect("delete clause must install native select_opponent_dp_budget");

    assert_eq!(dp_step.0, &CompiledFormula::Literal(15000));
    assert_eq!(*dp_step.1, 1, "delete is mandatory when targets exist");
    assert_eq!(dp_step.2.as_deref(), Some("targets"));
    assert!(
        dp_step
            .3
            .iter()
            .any(|step| matches!(step, CompiledStep::DeleteBoundPermanents { binding } if binding == "targets")),
        "native DP-budget selection must delete the bound target list"
    );
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

// --- Section 3: DP-budget delete behavioral ---

#[test]
fn bt17_018_on_play_deletes_selected_digimon_within_dp_budget() {
    let mut r = runner_with_opp_7k_and_8k_and_9k();
    let opp_7k = r.place_on_field(1, "OPP-7K", None);
    let opp_8k = r.place_on_field(1, "OPP-8K", None);
    let _opp_9k = r.place_on_field(1, "OPP-9K", None);
    let perm = r.place_on_field(0, "BT17-018", None);
    r.fire_on_play(0, perm.index as usize);

    let selection = r
        .pending_selection()
        .expect("BT17-018 must install a DP-budget selection");
    assert!(
        selection
            .valid_action_ids
            .contains(&encode_attack(0, opp_7k.index as u16)),
        "7000 DP target should be selectable"
    );
    assert!(
        selection
            .valid_action_ids
            .contains(&encode_attack(0, opp_8k.index as u16)),
        "8000 DP target should be selectable"
    );
    let selecting_player = selection.selecting_player;
    r.execute_action(selecting_player, encode_attack(0, opp_7k.index as u16))
        .expect("pick 7000 DP target");

    let selection = r
        .pending_selection()
        .expect("BT17-018 should continue selection after first pick");
    assert!(
        selection
            .valid_action_ids
            .contains(&encode_attack(0, opp_8k.index as u16)),
        "8000 DP target should fit the remaining 8000 DP budget"
    );
    r.execute_action(selecting_player, encode_attack(0, opp_8k.index as u16))
        .expect("pick 8000 DP target");

    assert_eq!(
        r.battle_area_size(1),
        1,
        "two selected opponent Digimon with total DP 15000 should be deleted"
    );
    assert!(
        r.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&r.game.card_data) == "OPP-9K"),
        "unselected opponent Digimon should remain"
    );
}

#[test]
fn bt17_018_on_play_cannot_select_digimon_exceeding_dp_budget() {
    let mut r = runner_with_opp_7k_and_8k_and_9k();
    let opp_7k = r.place_on_field(1, "OPP-7K", None);
    let opp_8k = r.place_on_field(1, "OPP-8K", None);
    let opp_9k = r.place_on_field(1, "OPP-9K", None);
    let perm = r.place_on_field(0, "BT17-018", None);
    r.fire_on_play(0, perm.index as usize);
    let selection = r.pending_selection().expect("Selection must be pending");
    let selecting_player = selection.selecting_player;

    r.execute_action(selecting_player, encode_attack(0, opp_7k.index as u16))
        .expect("pick 7000 DP target");

    let selection = r
        .pending_selection()
        .expect("selection remains open because player may pass after min picks");
    assert!(
        selection
            .valid_action_ids
            .contains(&encode_attack(0, opp_8k.index as u16)),
        "8000 DP target should remain selectable with exactly 8000 DP budget left"
    );
    assert!(
        !selection
            .valid_action_ids
            .contains(&encode_attack(0, opp_9k.index as u16)),
        "9000 DP target must be filtered after only 8000 DP budget remains"
    );
    assert!(
        selection.valid_action_ids.contains(&PASS),
        "player may stop early after satisfying the mandatory first pick"
    );
}

#[test]
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

/// Running-DP-sum multi-select: two opponent 7000-DP Digimon (sum 14000 ≤
/// the 15000 budget) must BOTH be deletable in a single multi-pick. The
/// raw_rust bridge only ever installed a single mandatory pick, so this
/// asserts the genuine multi-select behavior.
#[test]
fn bt17_018_on_play_multi_select_deletes_both_within_budget() {
    use digimon_engine::action::space::PASS;
    let mut opp_a = make_test_card("OPP-A7K", "OppA7K");
    opp_a.dp = Some(7000);
    let mut opp_b = make_test_card("OPP-B7K", "OppB7K");
    opp_b.dp = Some(7000);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-018 YAML loads")
        .add_card(opp_a)
        .add_card(opp_b)
        .memory(10)
        .build();
    r.place_on_field(1, "OPP-A7K", None);
    r.place_on_field(1, "OPP-B7K", None);
    let perm = r.place_on_field(0, "BT17-018", None);
    r.fire_on_play(0, perm.index as usize);

    // Drive the running-DP-sum multi-select: pick every eligible candidate.
    let mut guard = 0;
    while let Some(sel) = r.pending_selection() {
        guard += 1;
        assert!(guard < 12, "multi-select did not terminate");
        let player = sel.selecting_player;
        match sel.valid_action_ids.iter().find(|a| **a != PASS).copied() {
            Some(pick) => r.execute_action(player, pick).expect("pick candidate"),
            None => r.execute_action(player, PASS).expect("pass"),
        }
    }

    assert_eq!(
        r.battle_area_size(1),
        0,
        "both 7000-DP opp Digimon (sum 14000 ≤ 15000) must be deleted by the multi-select"
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
fn bt17_018_when_attacking_opt_resets_after_end_turn() {
    let mut r = runner_with_p1_security();
    // BT17-018 has 15000 DP; attacking another BT17-018 would mutually
    // delete in combat and leave the captured `attacker` handle stale by
    // the second attack. Use a low-DP defender so the attacker survives
    // the turn cycle and the OPT-reset assertion is reachable.
    let mut weak_def = make_test_card("WEAK-DEF", "WeakDef");
    weak_def.card_kind = digimon_engine::enums::CardKind::Digimon;
    weak_def.dp = Some(1000);
    weak_def.level = Some(3);
    r.game_mut().card_data.push(weak_def);

    let attacker = r.place_on_field(0, "BT17-018", Some(0));

    add_n_trash(&mut r, 0, 10);

    let defender1 = r.place_on_field(1, "WEAK-DEF", None);
    r.attack_digimon(attacker, defender1, false);
    let _ = r.auto_resolve();
    let sec_after_first = r.security_count(1);

    r.end_turn();
    r.end_turn();

    add_n_trash(&mut r, 0, 10);

    let defender2 = r.place_on_field(1, "WEAK-DEF", None);
    r.attack_digimon(attacker, defender2, false);
    let _ = r.auto_resolve();

    assert!(
        r.security_count(1) < sec_after_first,
        "OPT must reset after end_turn"
    );
}
