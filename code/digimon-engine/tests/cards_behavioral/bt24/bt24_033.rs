//! BT24-033 Salamon — Digimon, Lv.3, Yellow, Cost 3, DP 1000.
//! Traits: Mammal / Iliad / TS. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT24-033.md)
//!
//! Digivolve: Yellow Lv.2 / cost 0 ; Red Lv.2 / cost 0
//! [Digivolve] Lv.2 w/[TS] trait: Cost 0
//! [Your Turn] When this Digimon would digivolve into a Digimon card with the
//! [Iliad] trait, reduce the digivolution cost by 1.
//! Inherited: <Barrier>
//!
//! DATA NOTE: cards.json drops the Red Lv.2 / cost 0 circle; the YAML declares
//! it as an explicit alt_path.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_033.cs
//!   - AddSelfDigivolutionRequirementStaticEffect(IsLevel2 && HasTSTraits, 0)
//!   - ChangeDigivolutionCostStaticEffect(-1) — digivolving permanent is this
//!     card's permanent, target CardTraits contains "Iliad", IsOwnerTurn.
//!   - BarrierSelfEffect(isInheritedEffect: true)
//!
//! # Patterns
//! - D2 static digivolve-into cost reduction (BT23-037 / BT23-005 idiom)
//! - alt_paths: colour circles + trait alt path
//! - inherited grant_keyword Barrier (BT23-027 idiom)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT24-033";

fn lv2(id: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(2);
    c.dp = None;
    c.play_cost = 0;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Lv.4 yellow target with a Yellow Lv.3 / cost `evo` circle.
fn lv4(id: &str, traits: &[&str], evo: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: CardColor::Yellow as u8,
        memory_cost: evo,
    }];
    c
}

fn lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-033 compiles")
        .add_card(lv2("RED-LV2", CardColor::Red, &[]))
        .add_card(lv2("YEL-LV2", CardColor::Yellow, &[]))
        .add_card(lv2("BLUE-TS-LV2", CardColor::Blue, &["TS"]))
        .add_card(lv2("BLUE-LV2", CardColor::Blue, &[]))
        .add_card(lv4("ILIAD4", &["Iliad"], 2))
        .add_card(lv4("PLAIN4", &["Beast"], 2))
        .add_card(lv3("OTHER3"))
        .deck(0, &["OTHER3"; 6])
        .deck(1, &["OTHER3"; 6])
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

fn top(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn start(mem: i16) -> DebugRunner {
    let mut r = builder().memory(mem).start();
    r.set_first_player(0);
    r
}

// ─── Structure ───────────────────────────────────────────────────────────────

#[test]
fn bt24_033_metadata_matches_printed() {
    let r = start(0);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.name, "Salamon");
    assert_eq!(c.level, Some(3));
    assert_eq!(c.dp, Some(1000));
    assert_eq!(c.cost, Some(3));
    for t in ["Mammal", "Iliad", "TS"] {
        assert!(c.traits.iter().any(|x| x == t), "trait {t}");
    }
    let cr = c.effects.iter().find_map(|cl| match cl {
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            optional,
            once_per_turn,
            ..
        }) => Some((*optional, *once_per_turn)),
        _ => None,
    });
    assert_eq!(cr, Some((false, false)), "static, mandatory, not OPT");
    assert!(
        c.alt_paths
            .iter()
            .all(|p| p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(0))),
        "all digivolve routes cost 0"
    );
}

// ─── Digivolution requirements ──────────────────────────────────────────────

fn digivolve_onto_base(base_id: &str) -> (DebugRunner, PermanentHandle, bool, i16) {
    let mut r = start(5);
    let base = r.place_on_field(0, base_id, Some(0));
    let hi = push_hand(&mut r, 0, CARD_ID);
    let mem = r.memory();
    let mut ok = r
        .game
        .digivolve_from_hand(0, hi, base.index as usize, PlaySource::ByHand);
    // A base satisfying several cost-0 routes may prompt for the route
    // (rule 17); every route costs 0, so take the first.
    if !ok {
        if let Some(v) = r.pending_selection_view() {
            r.execute_action(v.selecting_player, v.valid_action_ids[0])
                .unwrap();
            ok = top(&r, base) == CARD_ID;
        }
    }
    (r, base, ok, mem)
}

#[test]
fn bt24_033_digivolves_from_red_lv2_for_cost_zero() {
    let (r, base, ok, mem) = digivolve_onto_base("RED-LV2");
    assert!(ok, "Red Lv.2 circle (official DB) must be a legal route");
    assert_eq!(top(&r, base), CARD_ID);
    assert_eq!(r.memory(), mem);
}

#[test]
fn bt24_033_digivolves_from_yellow_lv2_for_cost_zero() {
    let (r, base, ok, mem) = digivolve_onto_base("YEL-LV2");
    assert!(ok);
    assert_eq!(top(&r, base), CARD_ID);
    assert_eq!(r.memory(), mem);
}

#[test]
fn bt24_033_digivolves_from_off_colour_ts_lv2_for_cost_zero() {
    let (r, base, ok, mem) = digivolve_onto_base("BLUE-TS-LV2");
    assert!(ok, "Lv.2 w/[TS] trait alt route");
    assert_eq!(top(&r, base), CARD_ID);
    assert_eq!(r.memory(), mem);
}

#[test]
fn bt24_033_cannot_digivolve_from_off_colour_non_ts_lv2() {
    let mut r = start(5);
    let base = r.place_on_field(0, "BLUE-LV2", Some(0));
    let hi = push_hand(&mut r, 0, CARD_ID);
    let ok = r
        .game
        .digivolve_from_hand(0, hi, base.index as usize, PlaySource::ByHand);
    assert!(!ok);
    assert!(r.pending_selection().is_none());
    assert_eq!(top(&r, base), "BLUE-LV2");
}

// ─── [Your Turn] digivolve-into-[Iliad] cost reduction ──────────────────────

#[test]
fn bt24_033_reduces_cost_digivolving_into_iliad() {
    let mut r = start(5);
    let s = r.place_on_field(0, CARD_ID, Some(0));
    let hi = push_hand(&mut r, 0, "ILIAD4");
    let mem = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, hi, s.index as usize, PlaySource::ByHand);
    assert!(ok);
    assert_eq!(top(&r, s), "ILIAD4");
    assert_eq!(r.memory(), mem - 1, "evo cost 2 reduced by 1");
}

#[test]
fn bt24_033_no_reduction_into_non_iliad() {
    let mut r = start(5);
    let s = r.place_on_field(0, CARD_ID, Some(0));
    let hi = push_hand(&mut r, 0, "PLAIN4");
    let mem = r.memory();
    assert!(r
        .game
        .digivolve_from_hand(0, hi, s.index as usize, PlaySource::ByHand));
    assert_eq!(r.memory(), mem - 2);
}

#[test]
fn bt24_033_no_reduction_for_a_different_digimon() {
    let mut r = start(5);
    let _s = r.place_on_field(0, CARD_ID, Some(0));
    let other = r.place_on_field(0, "OTHER3", Some(0));
    let hi = push_hand(&mut r, 0, "ILIAD4");
    let mem = r.memory();
    assert!(r
        .game
        .digivolve_from_hand(0, hi, other.index as usize, PlaySource::ByHand));
    assert_eq!(r.memory(), mem - 2, "only THIS Digimon's digivolution");
}

#[test]
fn bt24_033_reduction_applies_for_controller_on_own_turn() {
    let mut r = start(5);
    r.place_on_field(0, "OTHER3", Some(0));
    let s = r.place_on_field(1, CARD_ID, Some(0));
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1);
    // P1 controls this Salamon and it is P1's turn → reduction applies for P1.
    r.game.memory = 5;
    let hi = push_hand(&mut r, 1, "ILIAD4");
    let before = r.game.memory;
    assert!(r
        .game
        .digivolve_from_hand(1, hi, s.index as usize, PlaySource::ByHand));
    assert_eq!(
        (before - r.game.memory).abs(),
        1,
        "the owner's own turn: reduced (sanity for the turn gate)"
    );
}

#[test]
fn bt24_033_reduction_requires_owner_turn() {
    let mut r = start(5);
    let s = r.place_on_field(0, CARD_ID, Some(0));
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1, "it is the opponent's turn");
    r.game.memory = 5;
    let hi = push_hand(&mut r, 0, "ILIAD4");
    let before = r.game.memory;
    assert!(r
        .game
        .digivolve_from_hand(0, hi, s.index as usize, PlaySource::ByHand));
    assert_eq!(top(&r, s), "ILIAD4");
    assert_eq!(
        (before - r.game.memory).abs(),
        2,
        "[Your Turn] gate: no reduction on the opponent's turn — full cost 2"
    );
}

// ─── Inherited <Barrier> ─────────────────────────────────────────────────────

#[test]
fn bt24_033_inherited_barrier_on_carrier() {
    let mut r = start(0);
    let carrier = r.place_stack(0, &[CARD_ID, "PLAIN4"]);
    assert!(r.game.has_keyword(carrier, Keyword::Barrier));
}

#[test]
fn bt24_033_no_barrier_on_face_up_salamon() {
    let mut r = start(0);
    let s = r.place_on_field(0, CARD_ID, Some(0));
    assert!(
        !r.game.has_keyword(s, Keyword::Barrier),
        "<Barrier> is inherited-only"
    );
}
