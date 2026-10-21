//! BT14-044 Palmon — Digimon, Lv.3, Green, DP 1000, Cost 3.
//! Traits: Vegetation. Form: Rookie. Attribute: Data.
//! Digivolve: green Lv.2, cost 0.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT14-044.md; card image agrees)
//!
//! ```text
//! [Start of Your Main Phase] 1 of your opponent's Digimon gains "[All Turns]
//! When this Digimon becomes suspended, lose 2 memory." until the end of their
//! turn.
//! Inherited: [Your Turn][Once Per Turn] When this Digimon would digivolve, if
//! you have a green Tamer, reduce the digivolution cost by 1.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT14/Green/BT14_044.cs
//!   - OnStartMainPhase ActivateClass (not optional), IsOwnerTurn, CanActivate
//!     = an opponent battle-area Digimon. SelectPermanentEffect (canNoSelect
//!     false, maxCount 1) → AddEffectToPermanent(selected,
//!     UntilOpponentTurnEnd, OnTappedAnyone) whose body is
//!     `selectedPermanent.TopCard.Owner.AddMemory(-2)`: the SUSPENDED
//!     Digimon's controller (the opponent) loses the memory.
//!   - Inherited: ChangeCostClass -1 (maxCountPerTurn 1), IsOwnerTurn, owner
//!     has a green Tamer, applies to a digivolution of THIS permanent.
//!
//! # Rules
//! - "lose 2 memory" in a gained effect is the gaining Digimon's controller's
//!   loss (the gained text is that Digimon's own effect).
//! - Attacking suspends the attacker (§11), which is "becomes suspended".
//!
//! # Patterns this test covers
//! - B-row `start_of_your_main_phase` trigger, mandatory opponent-Digimon pick
//! - `grant_triggered_effect` on a selected binding (EX10-034 shape) with
//!   `on_suspend` timing + `end_of_opponents_turn` expiry
//! - inherited `cost_reduction` on this permanent's digivolve gated on a green
//!   Tamer, OPT (BT14-046 shape)

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, ATTACK_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT14-044";

fn digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c
}

fn sec_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c
}

/// Green digimon of `level` that digivolves from a green `level - 1` for 3.
fn evo_target(id: &str, level: u8) -> CardData {
    let mut c = digimon(id, CardColor::Green, level);
    c.evo_costs = vec![EvoCost {
        card_color: 3,
        level: level - 1,
        memory_cost: 3,
    }];
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT14-044 in pack")
        .add_card(digimon("OPP-A", CardColor::Red, 4))
        .add_card(digimon("OPP-B", CardColor::Red, 4))
        .add_card(digimon("OWN-X", CardColor::Blue, 4))
        .add_card(tamer("OPP-TAMER", CardColor::Red))
        .add_card(tamer("G-TAMER", CardColor::Green))
        .add_card(tamer("R-TAMER", CardColor::Red))
        .add_card(digimon("TOP-G4", CardColor::Green, 4))
        .add_card(evo_target("G-LV4", 4))
        .add_card(evo_target("G-LV5", 5))
        .add_card(evo_target("G-LV6", 6))
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(sec_option("SEC"))
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .security(0, &["SEC"; 3])
        .security(1, &["SEC"; 3])
}

/// P0 has Palmon on the field; P1 has OPP-A (and OPP-B when `two`).
fn setup(two: bool) -> (DebugRunner, PermanentHandle, Option<PermanentHandle>) {
    let mut r = builder().memory(0).start();
    r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP-A", Some(0));
    let b = if two {
        Some(r.place_on_field(1, "OPP-B", Some(0)))
    } else {
        None
    };
    (r, a, b)
}

/// End P0's turn, then P1's turn: P0's next Start of Main Phase.
fn to_p0_next_main(r: &mut DebugRunner) {
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
}

fn grants(r: &DebugRunner, h: PermanentHandle) -> usize {
    r.game
        .modifiers
        .granted_triggered_for_timing(h, EffectTiming::OnSuspend)
        .len()
}

fn opp_pick(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

/// Resolve Palmon's pick with `h` (P0 selects).
fn pick(r: &mut DebugRunner, h: PermanentHandle) {
    let v = r.pending_selection_view().expect("Palmon's pick");
    assert!(v.valid_action_ids.contains(&opp_pick(h)), "{v:?}");
    r.execute_action(0, opp_pick(h)).unwrap();
    let _ = r.auto_resolve();
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt14_044_metadata_and_green_lv2_cost0_circle() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Palmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert_eq!(card.traits, vec!["Vegetation".to_string()]);
    assert_eq!(card.attribute, ["Data"]);
    assert_eq!(card.alt_paths.len(), 1);
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn bt14_044_start_of_main_grants_on_suspend_lose_memory_until_their_turn_end() {
    let card = dsl_card_data::compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::StartOfYourMainPhase] => {
                Some(t)
            }
            _ => None,
        })
        .expect("Start of Your Main Phase clause");
    assert!(!clause.optional, "mandatory");
    assert!(!clause.once_per_turn);
    let (timing, expiry, body, trigger_if) = clause
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::GrantTriggeredEffect {
                timing,
                expiry,
                body,
                trigger_if,
                ..
            } => Some((timing.clone(), expiry.clone(), body.clone(), trigger_if.clone())),
            _ => None,
        })
        .expect("grant_triggered_effect step");
    assert_eq!(timing, "on_suspend");
    assert_eq!(
        trigger_if.and_then(|p| p.event_target_is_source),
        Some(true),
        "\"When THIS Digimon becomes suspended\""
    );
    assert_eq!(expiry, "end_of_opponents_turn");
    assert!(format!("{body:?}").contains("LoseMemory"), "{body:?}");
}

#[test]
fn bt14_044_inherited_reducer_is_mandatory_opt() {
    let card = dsl_card_data::compiled(CARD_ID);
    let found: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                scope: CompiledScope::Inherited,
                once_per_turn,
                optional,
                amount,
                ..
            }) => Some((*once_per_turn, *optional, *amount)),
            _ => None,
        })
        .collect();
    assert_eq!(found, vec![(true, false, Some(1))]);
}

// ─── SECTION 2 — [Start of Your Main Phase] grant ──────────────────────────

/// Mandatory P0 pick over the opponent's Digimon only; only the chosen one
/// gains the effect.
#[test]
fn bt14_044_start_of_main_grants_only_the_selected_opponent_digimon() {
    let (mut r, a, b) = setup(true);
    let b = b.unwrap();
    let own = r.place_on_field(0, "OWN-X", Some(0));
    let tamer = r.place_on_field(1, "OPP-TAMER", Some(0));
    to_p0_next_main(&mut r);
    assert_eq!(r.turn_player(), 0);
    let v = r.pending_selection_view().expect("P0 picks an opponent Digimon");
    assert_eq!(v.selecting_player, 0);
    assert!(!v.is_optional, "the pick is mandatory");
    assert!(v.valid_action_ids.contains(&opp_pick(a)));
    assert!(v.valid_action_ids.contains(&opp_pick(b)));
    assert!(!v.valid_action_ids.contains(&opp_pick(tamer)), "Digimon only");
    assert_eq!(v.kind, SelectionKind::OppField, "the opponent's Digimon only");
    let _ = own;
    pick(&mut r, a);
    assert_eq!(grants(&r, a), 1, "the chosen Digimon gains the effect");
    assert_eq!(grants(&r, b), 0, "the other does not");
}

#[test]
fn bt14_044_no_prompt_without_opponent_digimon() {
    let mut r = builder().memory(0).start();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-TAMER", Some(0));
    to_p0_next_main(&mut r);
    assert_eq!(r.turn_player(), 0);
    assert!(r.pending_selection().is_none(), "no opponent Digimon: nothing to pick");
}

/// "[Start of YOUR Main Phase]": nothing at the opponent's main phase.
#[test]
fn bt14_044_does_not_trigger_at_the_opponents_main_phase() {
    let (mut r, a, _) = setup(false);
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    assert!(r.pending_selection().is_none(), "P1's main phase: no Palmon prompt");
    let _ = r.auto_resolve();
    assert_eq!(grants(&r, a), 0);
}

/// [All Turns]: the carrier suspended during P0's turn makes ITS controller
/// (P1) lose 2 — the counter moves 2 toward P0 (turn player).
#[test]
fn bt14_044_carrier_suspended_on_your_turn_opponent_loses_2() {
    let (mut r, a, _) = setup(false);
    to_p0_next_main(&mut r);
    pick(&mut r, a);
    r.game.set_memory(3);
    r.game.suspend(a);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 5, "P1 lost 2: P0's side +2");
}

/// The carrier attacking on its controller's turn (the attack suspends it)
/// costs that player 2 memory.
#[test]
fn bt14_044_carrier_attacking_on_their_turn_loses_them_2() {
    let (mut r, a, _) = setup(false);
    to_p0_next_main(&mut r);
    pick(&mut r, a);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    assert_eq!(grants(&r, a), 1, "still active during their turn");
    r.game.set_memory(5);
    let _ = r.attack_player(a, 0, false);
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[a.index as usize].is_suspended);
    assert_eq!(r.memory(), 3, "the attacker's controller (turn player) loses 2");
}

#[test]
fn bt14_044_unchosen_digimon_suspending_loses_nothing() {
    let (mut r, a, b) = setup(true);
    let b = b.unwrap();
    to_p0_next_main(&mut r);
    pick(&mut r, a);
    r.game.set_memory(3);
    r.game.suspend(b);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 3);
}

/// "until the end of their turn": gone once P1's turn ends; suspending it
/// afterwards costs nothing.
#[test]
fn bt14_044_grant_expires_at_the_end_of_the_opponents_turn() {
    let (mut r, a, _) = setup(false);
    to_p0_next_main(&mut r);
    pick(&mut r, a);
    r.end_turn(); // → P1
    let _ = r.auto_resolve();
    assert_eq!(grants(&r, a), 1);
    r.end_turn(); // P1's turn ends → P0
    assert_eq!(r.turn_player(), 0);
    assert_eq!(grants(&r, a), 0, "expired at the end of their turn");
    // Palmon re-prompts this Start of Main Phase: the fresh grant is the only
    // one (1, not 2).
    pick(&mut r, a);
    assert_eq!(grants(&r, a), 1, "only the fresh grant");
}

// ─── SECTION 3 — Inherited digivolve -1 with a green Tamer ─────────────────

fn inh_runner() -> DebugRunner {
    let mut r = builder().memory(10).start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.game.set_memory(10);
    r
}

fn hand_idx(r: &DebugRunner, id: &str) -> usize {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in hand"))
}

fn digivolve(r: &mut DebugRunner, carrier: PermanentHandle, id: &str) -> i16 {
    r.add_to_hand(0, id);
    let idx = hand_idx(r, id);
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, idx, carrier.index as usize, PlaySource::ByHand);
    assert!(ok, "digivolve resolves");
    let _ = r.auto_resolve();
    // The counter is stored from the turn player's side: take the magnitude.
    (before - r.memory()).abs()
}

#[test]
fn bt14_044_inherited_with_green_tamer_reduces_digivolve_cost_by_1() {
    let mut r = inh_runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 2, "3 - 1");
}

#[test]
fn bt14_044_inherited_without_a_tamer_pays_full() {
    let mut r = inh_runner();
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 3);
}

#[test]
fn bt14_044_inherited_with_only_a_red_tamer_pays_full() {
    let mut r = inh_runner();
    r.place_on_field(0, "R-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 3, "needs a GREEN Tamer");
}

#[test]
fn bt14_044_inherited_opponents_green_tamer_does_not_count() {
    let mut r = inh_runner();
    r.place_on_field(1, "G-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 3, "'you have' a green Tamer");
}

#[test]
fn bt14_044_inherited_does_not_reduce_another_permanents_digivolve() {
    let mut r = inh_runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    let other = r.place_on_field(0, "TOP-G4", Some(0));
    assert_eq!(digivolve(&mut r, other, "G-LV5"), 3, "only THIS Digimon");
}

/// Inherited text is active only from the digivolution cards: with Palmon
/// itself the top card, its own digivolution pays the full cost.
#[test]
fn bt14_044_inherited_inactive_while_palmon_is_the_top_card() {
    let mut r = inh_runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    let me = r.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(digivolve(&mut r, me, "G-LV4"), 3, "inherited, not top-card text");
}

#[test]
fn bt14_044_inherited_once_per_turn() {
    let mut r = inh_runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 2);
    assert_eq!(digivolve(&mut r, carrier, "G-LV6"), 3, "[Once Per Turn]");
}

#[test]
fn bt14_044_inherited_not_on_the_opponents_turn() {
    let mut r = inh_runner();
    r.place_on_field(0, "G-TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP-G4"]);
    r.game.turn_player_idx = 1;
    assert_eq!(digivolve(&mut r, carrier, "G-LV5"), 3, "[Your Turn]");
}
