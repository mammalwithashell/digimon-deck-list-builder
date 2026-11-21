//! P-174 Boltmon — Digimon, Black/Purple, Lv.6, cost 11, 11000 DP. Traits:
//! Cyborg, NSo. Attribute: Data.
//!
//! # Card text (official Bandai DB — data/card_bundles/P-174.md)
//! [Digivolve] Black Lv.5 / Purple Lv.5: Cost 3
//! [Digivolve] Lv.5 w/[NSo] trait: Cost 3
//! [DNA Digivolve] Black/Yellow Lv.5 + Purple/Red Lv.5: Cost 0
//! When this card would be played, if [Nightmare Soldiers] is in your face-up
//! security cards, reduce the play cost by 4.
//! <Blocker>
//! [When Digivolving] [On Deletion] <De-Digivolve 1> 1 of your opponent's
//! Digimon. Then, delete 1 of their level 4 or lower Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Black/P_174.cs — see the YAML header.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, GamePhase, Keyword, PlaySource};
use digimon_engine::replacement::ReplacementCause;

#[path = "../ex8/nso_support.rs"]
mod nso_support;
use nso_support::*;

const CARD_ID: &str = "P-174";

fn nightmare_soldiers() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("NIGHTMARE", "Nightmare Soldiers");
    c.card_kind = digimon_engine::enums::CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = vec![CardColor::Purple];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-174 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(nightmare_soldiers())
        .add_card(nso("N5-RED", 5, 5000, 5, CardColor::Red))
        .add_card(plain("X5-BLACK", 5, 5000, CardColor::Black))
        .add_card(plain("X5-PURPLE", 5, 5000, CardColor::Purple))
        .add_card(plain("X5-BLUE", 5, 5000, CardColor::Blue))
        .add_card(plain("OPP-L4", 4, 4000, CardColor::Blue))
        .add_card(plain("OPP-L4B", 4, 4000, CardColor::Blue))
        .add_card(plain("OPP-L5", 5, 5000, CardColor::Blue))
        .add_card(plain("OPP-STACK-1", 3, 3000, CardColor::Blue))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(20)
}

fn digivolve_over(base: &str) -> (DebugRunner, usize) {
    let mut r = builder().hand(0, &[CARD_ID]).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, base, Some(0)).index as usize;
    (r, slot)
}

fn offered(r: &DebugRunner, slot: usize) -> bool {
    build_action_mask(&r.game, 0)[encode_digivolve(0, slot as u16) as usize] > 0.5
}

// ─── Digivolution routes ─────────────────────────────────────────────────────

#[test]
fn p_174_standard_circles_black_and_purple_level_5_cost_3() {
    for base in ["X5-BLACK", "X5-PURPLE"] {
        let (mut r, slot) = digivolve_over(base);
        assert!(offered(&r, slot), "{base}");
        let mem = r.memory();
        assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
        assert_eq!(mem - r.memory(), 3, "{base}");
    }
}

#[test]
fn p_174_alt_route_level_5_nso_costs_3_and_off_colour_non_nso_is_rejected() {
    let (mut r, slot) = digivolve_over("N5-RED");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 3);
    let (r, slot) = digivolve_over("X5-BLUE");
    assert!(!offered(&r, slot));
}

#[test]
fn p_174_dna_recipe_is_black_or_yellow_l5_plus_purple_or_red_l5_cost_0() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    let dna = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .expect("DNA alt-path");
    assert!(dna.stacks_unsuspended);
    assert_eq!(dna.materials.len(), 2);
    assert_eq!(dna.materials[0].filter.level_eq, Some(5));
    assert_eq!(dna.materials[1].filter.level_eq, Some(5));
    assert_eq!(dna.materials[0].filter.color_only.as_ref().map(|c| c.len()), Some(2));
    assert_eq!(dna.materials[1].filter.color_only.as_ref().map(|c| c.len()), Some(2));
}

// ─── Play-cost reduction ─────────────────────────────────────────────────────

fn play_cost_paid(face_up_nightmare: bool, face_up_other_card: bool) -> i16 {
    let mut r = builder().hand(0, &[CARD_ID]).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    if face_up_nightmare || face_up_other_card {
        let id = if face_up_nightmare { "NIGHTMARE" } else { "FILL" };
        let idx = data_idx(&r, id);
        let next = r.game.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(idx, 0, next);
        let ci = card.card_index;
        r.game.players[0].security.push(card);
        r.game.players[0].face_up_security.insert(ci);
    }
    let mem = r.memory();
    r.play(0, 0).expect("the Boltmon is played");
    let _ = r.auto_resolve();
    mem - r.memory()
}

#[test]
fn p_174_plays_for_7_while_nightmare_soldiers_is_face_up_in_security() {
    assert_eq!(play_cost_paid(true, false), 7, "11 - 4");
}

#[test]
fn p_174_plays_for_11_without_a_face_up_nightmare_soldiers() {
    assert_eq!(play_cost_paid(false, false), 11);
    assert_eq!(play_cost_paid(false, true), 11, "a face-up OTHER card does not reduce");
}

#[test]
fn p_174_a_face_down_nightmare_soldiers_does_not_reduce() {
    let mut r = builder().hand(0, &[CARD_ID]).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let idx = data_idx(&r, "NIGHTMARE");
    let next = r.game.next_card_index();
    r.game.players[0]
        .security
        .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    let mem = r.memory();
    r.play(0, 0).expect("the Boltmon is played");
    let _ = r.auto_resolve();
    assert_eq!(mem - r.memory(), 11);
}

// ─── <Blocker> + [When Digivolving]/[On Deletion] ────────────────────────────

#[test]
fn p_174_has_blocker_and_the_shared_when_digivolving_on_deletion_clause() {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(me, Keyword::Blocker));
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::OnDeletion))));
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. }))));
}

fn board() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_stack(1, &["OPP-STACK-1", "OPP-L5"]);
    r.place_on_field(1, "OPP-L4", Some(0));
    (r, me)
}

#[test]
fn p_174_on_deletion_de_digivolves_1_then_deletes_a_level_4_or_lower() {
    let (mut r, me) = board();
    r.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    let view = r.pending_selection_view().expect("de-digivolve pick");
    assert!(!view.is_optional);
    // De-Digivolve the level 5 stack: it drops to its level 3 bottom card.
    pick_field(&mut r, 1, "OPP-L5");
    declare_max(&mut r);
    // THEN delete 1 level 4 or lower: the de-digivolved level 3 is now legal.
    let _ = r.pending_selection_view().expect("delete pick");
    pick_field(&mut r, 1, "OPP-STACK-1");
    drain_declining(&mut r);
    assert!(trash_ids(&r, 1).contains(&"OPP-STACK-1".to_string()), "the level 3 was deleted");
    assert!(field_ids(&r, 1).contains(&"OPP-L4".to_string()), "the other Digimon is untouched");
}

#[test]
fn p_174_the_delete_is_evaluated_after_the_de_digivolve() {
    // Only a level 5 Digimon (stack of 2) and no other: after De-Digivolve 1 it is
    // level 3, so it is now the only legal deletion target.
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_stack(1, &["OPP-STACK-1", "OPP-L5"]);
    r.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    pick_field(&mut r, 1, "OPP-L5");
    declare_max(&mut r);
    if r.pending_selection_view().is_some() {
        pick_field(&mut r, 1, "OPP-STACK-1");
    }
    drain_declining(&mut r);
    assert!(field_ids(&r, 1).is_empty(), "de-digivolved to level 3, then deleted");
}

#[test]
fn p_174_when_digivolving_runs_the_same_body() {
    let (mut r, slot) = digivolve_over("N5-RED");
    r.place_on_field(1, "OPP-L4", Some(0));
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    let _ = r.pending_selection_view().expect("de-digivolve pick");
    pick_field(&mut r, 1, "OPP-L4");
    declare_max(&mut r);
    drain_declining(&mut r);
    assert!(field_ids(&r, 1).is_empty() || trash_ids(&r, 1).contains(&"OPP-L4".to_string()));
}
