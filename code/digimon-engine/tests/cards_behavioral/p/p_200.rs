//! P-200 Kanan Yuki — Tamer, Green, Cost 3. Traits: TS.
//!
//! # Card text (official Bandai DB — data/card_bundles/P-200.md)
//!
//! [Start of Your Main Phase] If you have 4 or less memory, suspend 1 of your
//! opponent's Digimon.
//! [Your Turn] When any of your Digimon would digivolve into a Digimon card
//! with the [TS] trait, by suspending this Tamer, reduce the digivolution cost
//! by 1.
//! [Security] Play this card without paying the cost.
//!
//! (Not a reprint of BT26-090 Kanan Yuki — different text.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Green/P_200.cs
//!   - OnStartMainPhase: mandatory; CanActivate = owner turn + opponent
//!     Digimon exists + MemoryForPlayer <= 4; SelectPermanentEffect
//!     (canNoSelect: false, Mode.Tap).
//!   - BeforePayCost: isOptional TRUE; own Digimon digivolving into a
//!     HasTSTraits card; suspend self (CanActivateSuspendCostEffect) → -1.
//!   - SecuritySkill: PlaySelfTamerSecurityEffect.
//!
//! # Patterns
//! - start_of_your_main_phase + memory_lte gate + mandatory opp-field suspend
//! - cost_reduction when_any_ally_digivolves_into + suspend-self pay_cost,
//!   optional (BT5-092 / BT26-088 idioms)
//! - play_from_security (Tamer)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "P-200";

fn digimon(id: &str, level: u8, traits: &[&str], evo_cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 5;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c.evo_costs = vec![EvoCost {
        level: level - 1,
        card_color: CardColor::Green as u8,
        memory_cost: evo_cost,
    }];
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-200 compiles")
        .add_card(digimon("ALLY3", 3, &[], 0))
        .add_card(digimon("TS4", 4, &["TS"], 3))
        .add_card(digimon("PLAIN4", 4, &["Beast"], 3))
        .add_card(digimon("OPP-A", 3, &[], 0))
        .add_card(digimon("OPP-B", 4, &[], 0))
        .add_card(digimon("FILLER", 3, &[], 0))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
}

fn push_hand(r: &mut DebugRunner, p: u8, card_id: &str) -> usize {
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .hand
        .push(CardSource::new(idx, p, next));
    r.game.players[p as usize].hand.len() - 1
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game.enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

// ─── Structure ───────────────────────────────────────────────────────────────

#[test]
fn p_200_metadata_and_optional_cost_reduction() {
    let r = builder().memory(0).start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.name, "Kanan Yuki");
    assert_eq!(c.cost, Some(3));
    assert!(c.traits.iter().any(|t| t == "TS"));
    let cr = c.effects.iter().find_map(|cl| match cl {
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            optional,
            once_per_turn,
            ..
        }) => Some((*optional, *once_per_turn)),
        _ => None,
    });
    assert_eq!(
        cr,
        Some((true, false)),
        "DCGO isOptional true; no [Once Per Turn]"
    );
}

// ─── [Start of Your Main Phase] ──────────────────────────────────────────────

#[test]
fn p_200_somp_with_4_memory_suspends_chosen_opponent_digimon() {
    let mut r = builder().memory(4).start();
    r.set_first_player(0);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let _a = r.place_on_field(1, "OPP-A", Some(0));
    let b = r.place_on_field(1, "OPP-B", Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    let v = r.pending_selection_view().expect("mandatory suspend pick");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert!(!r.pending_is_optional(), "canNoSelect: false");
    assert_eq!(v.valid_action_ids.len(), 2, "both opponent Digimon offered");
    r.execute_action(0, encode_attack(0, b.index as u16)).unwrap();
    assert!(suspended(&r, b), "chosen opponent Digimon suspended");
    assert!(!suspended(&r, _a), "the other stays unsuspended");
    assert!(!suspended(&r, h), "Kanan is not suspended by this clause");
}

#[test]
fn p_200_somp_with_5_memory_does_nothing() {
    let mut r = builder().memory(5).start();
    r.set_first_player(0);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP-A", Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    assert!(r.pending_selection().is_none(), "memory 5 > 4 → no effect");
    assert!(!suspended(&r, a));
}

#[test]
fn p_200_somp_does_not_affect_own_digimon() {
    let mut r = builder().memory(2).start();
    r.set_first_player(0);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let own = r.place_on_field(0, "ALLY3", Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, h);
    let _ = r.auto_resolve();
    assert!(!suspended(&r, own), "only opponent Digimon are eligible");
}

// ─── [Your Turn] digivolve cost reduction ───────────────────────────────────

fn digivolve_setup() -> (DebugRunner, PermanentHandle, PermanentHandle, i16) {
    let mut r = builder().memory(5).start();
    r.set_first_player(0);
    let kanan = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY3", Some(0));
    let mem = r.memory();
    (r, kanan, ally, mem)
}

#[test]
fn p_200_accepting_suspends_tamer_and_reduces_ts_digivolve_cost() {
    let (mut r, kanan, ally, mem) = digivolve_setup();
    let hi = push_hand(&mut r, 0, "TS4");
    let done = r
        .game
        .digivolve_from_hand(0, hi, ally.index as usize, PlaySource::ByHand);
    assert!(!done, "an optional reduction offer must be pending");
    assert!(r.pending_is_optional(), "the reduction is optional");
    r.accept_optional_trigger().expect("accept");
    let _ = r.auto_resolve();
    r.game.drain_effect_queue();
    assert!(suspended(&r, kanan), "Kanan suspended as the cost");
    assert_eq!(field_ids(&r, 0)[ally.index as usize], "TS4");
    assert_eq!(r.memory(), mem - 2, "evo cost 3 reduced by 1");
}

#[test]
fn p_200_declining_pays_full_cost_and_stays_unsuspended() {
    let (mut r, kanan, ally, mem) = digivolve_setup();
    let hi = push_hand(&mut r, 0, "TS4");
    let _ = r
        .game
        .digivolve_from_hand(0, hi, ally.index as usize, PlaySource::ByHand);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    r.game.drain_effect_queue();
    assert!(!suspended(&r, kanan));
    assert_eq!(field_ids(&r, 0)[ally.index as usize], "TS4");
    assert_eq!(r.memory(), mem - 3, "full evo cost 3");
}

#[test]
fn p_200_non_ts_target_gets_no_offer() {
    let (mut r, kanan, ally, mem) = digivolve_setup();
    let hi = push_hand(&mut r, 0, "PLAIN4");
    let done = r
        .game
        .digivolve_from_hand(0, hi, ally.index as usize, PlaySource::ByHand);
    assert!(done, "no reduction offer for a non-[TS] target");
    r.game.drain_effect_queue();
    assert!(!suspended(&r, kanan));
    assert_eq!(r.memory(), mem - 3);
}

#[test]
fn p_200_already_suspended_tamer_cannot_pay() {
    let (mut r, kanan, ally, mem) = digivolve_setup();
    r.game.players[0].battle_area[kanan.index as usize].is_suspended = true;
    let hi = push_hand(&mut r, 0, "TS4");
    let done = r
        .game
        .digivolve_from_hand(0, hi, ally.index as usize, PlaySource::ByHand);
    assert!(
        done && r.pending_selection().is_none(),
        "DCGO CanActivateSuspendCostEffect: a suspended Tamer is never offered"
    );
    r.game.drain_effect_queue();
    assert_eq!(r.memory(), mem - 3, "suspend cost unpayable → no reduction");
}

#[test]
fn p_200_inactive_on_opponents_turn() {
    let mut r = builder().memory(5).start();
    r.set_first_player(0);
    let kanan = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY3", Some(0));
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1, "it is the opponent's turn");
    r.game.memory = 5;
    let hi = push_hand(&mut r, 0, "TS4");
    let before = r.game.memory;
    let done = r
        .game
        .digivolve_from_hand(0, hi, ally.index as usize, PlaySource::ByHand);
    assert!(done, "P0's own Digimon digivolves on P1's turn with no offer");
    assert!(r.pending_selection().is_none(), "[Your Turn] gate: no reducer offer");
    r.game.drain_effect_queue();
    assert_eq!(field_ids(&r, 0)[ally.index as usize], "TS4");
    assert!(!suspended(&r, kanan), "Kanan stays unsuspended");
    assert_eq!((before - r.game.memory).abs(), 3, "full evo cost 3");
}

// ─── [Security] ──────────────────────────────────────────────────────────────

#[test]
fn p_200_security_plays_itself_for_free() {
    let mut r = builder().security(0, &[CARD_ID]).memory(5).start();
    r.set_first_player(0);
    let attacker = r.place_on_field(1, "OPP-B", Some(0));
    let mem = r.memory();
    r.attack_player(attacker, 0, false);
    let _ = r.auto_resolve();
    r.game.drain_effect_queue();
    assert!(
        field_ids(&r, 0).contains(&CARD_ID.to_string()),
        "[Security] plays Kanan into P0's battle area"
    );
    assert_eq!(r.memory(), mem, "played without paying the cost");
}

