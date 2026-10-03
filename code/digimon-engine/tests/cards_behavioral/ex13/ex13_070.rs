//! EX13-070 Davis Motomiya & Ken Ichijoji — Tamer, Blue/Green, cost 4.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-070.md`)
//!
//! [Start of Your Turn] If you have 2 or less memory, set it to 3. [End of Your
//! Turn] By suspending this Tamer, activate 1 of the effects below:
//! ・1 of your Digimon may digivolve into a Digimon card with [Imperialdramon]
//!   in its name or the [Free] trait in the hand. Reduce this effect's paid
//!   cost by 1 for each of your opponent's Digimon.
//! ・2 of your Digimon may DNA digivolve into a [Free] trait Digimon card in
//!   the hand.
//!
//! Security Effect: [Security] Play this card without paying the cost.
//!
//! Official Q&A: "You can activate the effect and digivolve, but its cost won't
//! be reduced." (no opponent Digimon → no reduction).
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_070.cs`); printed text + general_rule.pdf
//! govern. Idioms: BT21-084 (memory floor, security self-play, suspend-self
//! activation cost), `select_effect_choice` branches (EX13-030), BT20-016
//! (`select_dna_pair` + `effect_initiated_dna_digivolve`, here `cost: printed`).
//!
//! # Patterns
//! - B1: [Start of Your Turn] memory floor.
//! - E1: optional cost → choose-one of two effects.
//! - D2: effect digivolve with a per-opponent-Digimon cost reduction.
//! - C2: effect DNA digivolve paying the printed DNA cost.
//! - H: [Security] self-play.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{DnaCost, DnaRequirement};
use digimon_engine::debug_runner::dna_req_lv;
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-070";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-070 YAML loads")
        .add_card(named("BLUE-L5", "Paildramon", CardColor::Blue, 5, 7000))
        .add_card(named("BLUE-L4", "ExVeemon", CardColor::Blue, 4, 5000))
        .add_card(named("GREEN-L4", "Stingmon", CardColor::Green, 4, 5000))
        .add_card(with_evo(
            named("IMPERIAL", "Imperialdramon: Dragon Mode", CardColor::Blue, 6, 12000),
            CardColor::Blue,
            5,
            4,
        ))
        .add_card(with_evo(
            digimon("FREE-6", CardColor::Blue, 6, 11000, &["Free"]),
            CardColor::Blue,
            5,
            4,
        ))
        .add_card(with_evo(
            named("PLAIN-6", "Plainmon", CardColor::Blue, 6, 11000),
            CardColor::Blue,
            5,
            4,
        ))
        .add_card({
            let mut c = digimon("FREE-DNA", CardColor::Blue, 5, 9000, &["Free"]);
            c.dna_costs = vec![DnaCost {
                memory_cost: 2,
                requirement1: dna_req_lv(4),
                requirement2: dna_req_lv(4),
            }];
            c
        })
        .add_card({
            let mut c = named("PLAIN-DNA", "Plaindna", CardColor::Blue, 5, 9000);
            c.dna_costs = vec![DnaCost {
                memory_cost: 2,
                requirement1: dna_req_lv(4),
                requirement2: dna_req_lv(4),
            }];
            c
        })
        .add_card(named("OPP", "Oppmon", CardColor::Red, 4, 5000))
        .add_card(named("ATK", "Attacker", CardColor::Red, 5, 7000))
        .add_card(named("FILL", "Filler", CardColor::Blue, 3, 3000))
}

fn start(hand: &[&str], mem: i16) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(mem)
        .start()
}

/// The first memory change after checkpoint `ck` (the effect's payment; the
/// turn may pass right after, which would make `memory()` misleading).
fn first_memory_delta(r: &DebugRunner, ck: usize) -> Option<i32> {
    r.events_since(ck).iter().find_map(|e| match e {
        digimon_engine::events::GameEvent::MemoryChange { delta, .. } => Some(*delta as i32),
        _ => None,
    })
}

/// End the turn and accept the [End of Your Turn] cost prompt; returns with
/// the choose-one prompt pending.
fn end_turn_and_pay(r: &mut DebugRunner, me: PermanentHandle) {
    r.end_turn();
    assert!(r.pending_is_optional(), "'By suspending this Tamer' is optional");
    accept(r);
    assert!(is_suspended(r, me), "cost paid");
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_070_is_a_blue_green_tamer_with_three_clauses() {
    let r = start(&[], 3);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.kind, CompiledCardKind::Tamer);
    assert_eq!(c.color, vec![CompiledColor::Blue, CompiledColor::Green]);
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    let whens: Vec<_> = t.iter().map(|t| t.when.clone()).collect();
    assert_eq!(
        whens,
        vec![
            vec![CompiledTiming::StartOfYourTurn],
            vec![CompiledTiming::EndOfYourTurn],
            vec![CompiledTiming::OnSecurity]
        ]
    );
    assert!(t[1].optional);
}

// ─── Section 2/3 — [Start of Your Turn] ──────────────────────────────────────

#[test]
fn ex13_070_start_of_turn_low_memory_set_to_3() {
    let mut r = start(&[], 3);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, me, true); // no [End of Your Turn] prompt
    cycle_to_my_turn_with_memory(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!((r.turn_player(), r.memory()), (0, 3));
}

#[test]
fn ex13_070_start_of_turn_high_memory_unchanged() {
    let mut r = start(&[], 3);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, me, true);
    cycle_to_my_turn_with_memory(&mut r, 4);
    let _ = r.auto_resolve();
    assert_eq!((r.turn_player(), r.memory()), (0, 4));
}

// ─── Section 2/3 — [End of Your Turn] ────────────────────────────────────────

#[test]
fn ex13_070_end_of_turn_declining_the_cost_does_nothing() {
    let mut r = start(&["IMPERIAL"], 6);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(0, "BLUE-L5", Some(0));
    r.end_turn();
    assert!(r.pending_is_optional());
    decline(&mut r);
    let _ = r.auto_resolve();
    assert!(!is_suspended(&r, me));
    assert_eq!(top_id(&r, base), "BLUE-L5");
}

#[test]
fn ex13_070_suspended_tamer_gets_no_end_of_turn_prompt() {
    let mut r = start(&["IMPERIAL"], 6);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, me, true);
    r.place_on_field(0, "BLUE-L5", Some(0));
    r.end_turn();
    assert!(r.pending_selection().is_none(), "cost unpayable");
}

#[test]
fn ex13_070_choice_1_digivolves_with_reduction_per_opponent_digimon() {
    let mut r = start(&["IMPERIAL", "PLAIN-6"], 6);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(0, "BLUE-L5", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    end_turn_and_pay(&mut r, me);
    r.execute_branch(0).expect("digivolve branch");
    // Pick the Digimon, then the hand card.
    pick_field(&mut r, base);
    assert_eq!(offered_hand_ids(&r), vec!["IMPERIAL".to_string()], "[Imperialdramon]/[Free] only");
    let ck = r.event_checkpoint();
    pick_hand(&mut r, "IMPERIAL");
    assert_eq!(top_id(&r, base), "IMPERIAL");
    assert_eq!(first_memory_delta(&r, ck), Some(-2), "cost 4 − 2 opponent Digimon");
}

#[test]
fn ex13_070_choice_1_cost_paid_is_reduced_by_opponent_digimon_count() {
    let mut r = start(&["FREE-6"], 6);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(0, "BLUE-L5", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    end_turn_and_pay(&mut r, me);
    r.execute_branch(0).unwrap();
    pick_field(&mut r, base);
    let ck = r.event_checkpoint();
    pick_hand(&mut r, "FREE-6");
    assert_eq!(top_id(&r, base), "FREE-6", "[Free] trait card");
    assert_eq!(first_memory_delta(&r, ck), Some(-1), "cost 4 − 3 opponent Digimon = 1");
}

#[test]
fn ex13_070_choice_1_no_opponent_digimon_means_no_reduction() {
    let mut r = start(&["FREE-6"], 6);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(0, "BLUE-L5", Some(0));
    end_turn_and_pay(&mut r, me);
    r.execute_branch(0).unwrap();
    pick_field(&mut r, base);
    let ck = r.event_checkpoint();
    pick_hand(&mut r, "FREE-6");
    assert_eq!(top_id(&r, base), "FREE-6");
    assert_eq!(first_memory_delta(&r, ck), Some(-4), "Official Q&A: digivolve, cost not reduced");
}

#[test]
fn ex13_070_choice_2_dna_digivolves_two_digimon_into_free_card_paying_printed_cost() {
    let mut r = start(&["FREE-DNA", "PLAIN-DNA"], 6);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(0, "BLUE-L4", Some(0));
    let b = r.place_on_field(0, "GREEN-L4", Some(0));
    end_turn_and_pay(&mut r, me);
    r.execute_branch(1).expect("DNA branch");
    assert_eq!(offered_hand_ids(&r), vec!["FREE-DNA".to_string()], "[Free] trait only");
    pick_hand(&mut r, "FREE-DNA");
    pick_field(&mut r, a);
    let ck = r.event_checkpoint();
    pick_field(&mut r, b);
    let field = field_ids(&r, 0);
    assert!(field.contains(&"FREE-DNA".to_string()), "DNA result on field: {field:?}");
    assert!(!field.contains(&"BLUE-L4".to_string()) && !field.contains(&"GREEN-L4".to_string()));
    assert_eq!(first_memory_delta(&r, ck), Some(-2), "printed DNA cost");
}

#[test]
fn ex13_070_choice_2_may_be_declined_after_choosing() {
    let mut r = start(&["FREE-DNA"], 6);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "BLUE-L4", Some(0));
    r.place_on_field(0, "GREEN-L4", Some(0));
    end_turn_and_pay(&mut r, me);
    r.execute_branch(1).unwrap();
    assert!(r.pending_is_optional(), "'may DNA digivolve'");
    decline(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0).iter().filter(|c| *c == "FREE-DNA").count(), 1);
}

// ─── Section 3 — [Security] ──────────────────────────────────────────────────

#[test]
fn ex13_070_security_plays_without_paying() {
    let mut r = builder()
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&CARD_ID.to_string()));
    assert_eq!(r.memory(), 0);
}

