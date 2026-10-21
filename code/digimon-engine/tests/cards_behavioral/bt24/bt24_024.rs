//! BT24-024 Submarimon — Digimon, Lv.4, Blue/Yellow, DP 5000, Play Cost 5.
//! Trait line (card image): Armor Form | Free | Aquatic/Iliad/TS.
//!
//! # Card text (card image + official Bandai DB bundle — authoritative)
//!
//! Digivolve: Blue Lv.3 / Cost 3 · Yellow Lv.3 / Cost 3
//! [Digivolve] [Armadillomon]/Lv.3 w/[TS] trait: Cost 2
//! <Armor Purge>
//! [When Attacking] [Once Per Turn] You may play 1 Tamer card with the [TS]
//!   trait from your hand with the play cost reduced by 2.
//! (No inherited effect.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Blue/BT24_024.cs
//! - AddSelfDigivolutionRequirementStaticEffect(EqualsCardName("Armadillomon")
//!   || (IsLevel3 && HasTSTraits), cost 2)
//! - ArmorPurgeEffect
//! - OnAllyAttack ActivateClass (maxCount 1, optional; CanActivateCondition:
//!   a [TS] Tamer in hand): SelectHandEffect canNoSelect: false, then
//!   PlayPermanentCards(payCost: true) under a temporary "Play Cost -2"
//!   ChangeCostClass for the owner's [TS] Tamers
//!
//! # Patterns this test covers
//! - B2 play a Tamer from hand at a reduced cost (when attacking, OPT)
//! - E2 optional confirm guarded by candidate existence; OPT lockout
//! - H-ArmorPurge keyword
//! - Alt paths: dual-colour circle + name / trait special condition

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT24-024";

fn digimon(id: &str, name: &str, level: u8, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.play_cost = cost;
    c.colors = vec![CardColor::Blue];
    c.level = None;
    c.dp = None;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-024 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(tamer("TS-TAMER-5", 5, &["TS"]))
        .add_card(tamer("TS-TAMER-1", 1, &["TS"]))
        .add_card(tamer("PLAIN-TAMER", 3, &["Iliad"]))
        .add_card(digimon("TS-DIGI", "TS Digi", 3, CardColor::Blue, &["TS"]))
        .add_card(digimon("BLUE-3", "Blue 3", 3, CardColor::Blue, &[]))
        .add_card(digimon("YELLOW-3", "Yellow 3", 3, CardColor::Yellow, &[]))
        .add_card(digimon("ARMADILLO", "Armadillomon", 3, CardColor::Green, &[]))
        .add_card(digimon("ARMADILLO-2", "Armadillomon", 2, CardColor::Green, &[]))
        .add_card(digimon("RED-TS-3", "Red TS 3", 3, CardColor::Red, &["TS"]))
        .add_card(digimon("RED-3", "Red 3", 3, CardColor::Red, &[]))
        .add_card(digimon("RED-TS-2", "Red TS 2", 2, CardColor::Red, &["TS"]))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn fire_when_attacking(r: &mut DebugRunner, at: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(at));
    r.game.drain_effect_queue();
}

fn submarimon_on_field(hand: &[&str], memory: i16) -> (DebugRunner, PermanentHandle) {
    let mut r = base().hand(0, hand).memory(memory).start();
    let sub = r.place_on_field(0, CARD_ID, Some(0));
    r.game.enter_main_phase();
    (r, sub)
}

// ─── Section 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt24_024_printed_metadata() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(card.name, "Submarimon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.cost, Some(5));
    for t in ["Aquatic", "Iliad", "TS"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t}");
    }
}

#[test]
fn bt24_024_clause_shapes() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
            if keyword == "ArmorPurge"
    )), "<Armor Purge>");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0].when, vec![CompiledTiming::WhenAttacking]);
    assert!(triggered[0].once_per_turn);
    assert!(triggered[0].optional);
    assert_eq!(triggered[0].scope, CompiledScope::FaceUp, "no inherited effect");
}

// ─── Section 2 — Digivolution requirements ──────────────────────────────────

fn try_digivolve_over(base_id: &str) -> (bool, i16) {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, base_id, Some(0));
    let ok = r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    (ok, r.game.memory - 10)
}

#[test]
fn bt24_024_digivolves_from_blue_lv3_for_3() {
    assert_eq!(try_digivolve_over("BLUE-3"), (true, -3));
}

#[test]
fn bt24_024_digivolves_from_yellow_lv3_for_3() {
    assert_eq!(try_digivolve_over("YELLOW-3"), (true, -3));
}

#[test]
fn bt24_024_digivolves_from_armadillomon_for_2() {
    assert_eq!(try_digivolve_over("ARMADILLO"), (true, -2), "[Armadillomon]: Cost 2");
}

#[test]
fn bt24_024_digivolves_from_any_level_armadillomon_for_2() {
    // DCGO: EqualsCardName("Armadillomon") — the name alone qualifies.
    assert_eq!(try_digivolve_over("ARMADILLO-2"), (true, -2));
}

#[test]
fn bt24_024_digivolves_from_off_colour_lv3_ts_for_2() {
    assert_eq!(try_digivolve_over("RED-TS-3"), (true, -2), "Lv.3 w/[TS] trait: Cost 2");
}

#[test]
fn bt24_024_no_digivolve_from_red_lv3_without_ts() {
    assert_eq!(try_digivolve_over("RED-3"), (false, 0));
}

#[test]
fn bt24_024_no_digivolve_from_lv2_ts() {
    assert_eq!(try_digivolve_over("RED-TS-2"), (false, 0), "the [TS] route needs Lv.3");
}

// ─── Section 3 — <Armor Purge> ──────────────────────────────────────────────

#[test]
fn bt24_024_armor_purge_trashes_top_card_instead_of_deletion() {
    let mut r = base().start();
    let sub = r.place_stack(0, &["ARMADILLO", CARD_ID]);
    r.game
        .delete_permanent_with_cause(sub, ReplacementCause::OpponentEffect);
    let view = r.pending_selection_view().expect("Armor Purge prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional);
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    r.auto_resolve().ok();
    assert_eq!(field_ids(&r, 0), vec!["ARMADILLO".to_string()]);
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == CARD_ID));
}

// ─── Section 4 — [When Attacking][OPT] play a [TS] Tamer at cost -2 ─────────

#[test]
fn bt24_024_when_attacking_plays_ts_tamer_reduced_by_2() {
    let (mut r, sub) = submarimon_on_field(&["TS-TAMER-5"], 5);
    fire_when_attacking(&mut r, sub);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement), "'You may play' confirm");
    r.accept_optional_trigger().unwrap();
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    let view = r.pending_selection_view().unwrap();
    assert_eq!(view.valid_action_ids.len(), 1);
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    r.auto_resolve().ok();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string(), "TS-TAMER-5".to_string()]);
    assert_eq!(r.game.memory, 2, "play cost 5 reduced by 2 → 3 paid");
}

#[test]
fn bt24_024_reduction_floors_at_zero() {
    let (mut r, sub) = submarimon_on_field(&["TS-TAMER-1"], 5);
    fire_when_attacking(&mut r, sub);
    r.accept_optional_trigger().unwrap();
    let view = r.pending_selection_view().unwrap();
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    r.auto_resolve().ok();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string(), "TS-TAMER-1".to_string()]);
    assert_eq!(r.game.memory, 5, "play cost 1 reduced by 2 → 0, no memory gained");
}

#[test]
fn bt24_024_only_ts_tamers_are_offered() {
    let (mut r, sub) = submarimon_on_field(&["PLAIN-TAMER", "TS-DIGI", "TS-TAMER-5"], 5);
    fire_when_attacking(&mut r, sub);
    r.accept_optional_trigger().unwrap();
    let view = r.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "a non-[TS] Tamer and a [TS] Digimon are not eligible"
    );
}

#[test]
fn bt24_024_no_ts_tamer_in_hand_no_prompt() {
    let (mut r, sub) = submarimon_on_field(&["PLAIN-TAMER", "TS-DIGI"], 5);
    fire_when_attacking(&mut r, sub);
    assert!(r.pending_selection().is_none(), "nothing to play → no confirm");
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
}

#[test]
fn bt24_024_declining_keeps_the_once_per_turn() {
    let (mut r, sub) = submarimon_on_field(&["TS-TAMER-5"], 5);
    fire_when_attacking(&mut r, sub);
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().ok();
    assert_eq!(hand_ids(&r), vec!["TS-TAMER-5".to_string()]);
    // Not used: a later [When Attacking] the same turn offers it again.
    fire_when_attacking(&mut r, sub);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
}

#[test]
fn bt24_024_once_per_turn_lockout_and_reset() {
    let (mut r, sub) = submarimon_on_field(&["TS-TAMER-1", "TS-TAMER-1"], 5);
    fire_when_attacking(&mut r, sub);
    r.accept_optional_trigger().unwrap();
    let v = r.pending_selection_view().unwrap();
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    r.auto_resolve().ok();
    assert_eq!(hand_ids(&r).len(), 1);

    fire_when_attacking(&mut r, sub);
    assert!(r.pending_selection().is_none(), "[Once Per Turn]: locked out this turn");

    r.end_turn();
    r.end_turn();
    r.game.enter_main_phase();
    let sub = r.perm_handle(0, 0);
    fire_when_attacking(&mut r, sub);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "the lockout clears on your next turn"
    );
}

#[test]
fn bt24_024_real_attack_triggers_the_effect() {
    let mut r = base()
        .hand(0, &["TS-TAMER-5"])
        .security(1, &["DECK-PAD"; 3])
        .memory(5)
        .start();
    let sub = r.place_on_field(0, CARD_ID, Some(0));
    r.game.enter_main_phase();
    r.attack_player(sub, 1, false);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    r.accept_optional_trigger().unwrap();
    let v = r.pending_selection_view().unwrap();
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    r.auto_resolve().ok();
    assert!(field_ids(&r, 0).contains(&"TS-TAMER-5".to_string()));
}
