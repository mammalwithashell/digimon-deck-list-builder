//! RB1-010 Siriusmon — Digimon, Lv.6, Red, DP 11000, Cost 11.
//! Traits: Light Dragon. Attribute: Vaccine.
//!
//! # Printed text (official Bandai DB — data/card_bundles/RB1-010.md)
//!
//! ```text
//! Digivolve: Red Lv.5 / cost 3
//! [Digivolve] 3 from Lv.5 w/[Gammamon] in text
//!
//! [When Digivolving] By placing 1 Digimon card with [Gammamon] in its text
//! from your hand as this Digimon's bottom digivolution card, delete 1 of your
//! opponent's Digimon with DP less than or equal to this Digimon's DP.
//! [Your Turn][Once Per Turn] When an opponent's Digimon is deleted, you may
//! unsuspend this Digimon.
//! ```
//!
//! Official Q&A: an inherited DP boost on the placed card counts — the
//! deletion threshold is this Digimon's DP AFTER the placement.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/RB1/Red/RB1_010.cs
//!
//! # DSL YAML
//! code/digimon-engine/cards/rb1/RB1-010.yaml
//!
//! # Patterns
//! - alt digivolve path (`level_eq` + `in_text_contains`)
//! - [When Digivolving] optional cost "By placing X from hand as bottom source"
//!   (`select_hand { optional, cost }` + `place_as_bottom_source`)
//! - self-relative DP deletion filter (`dp_lte: { formula: { source_dp } }`)
//! - [Your Turn][OPT] `on_any_deletion` opponent-Digimon observer → optional
//!   `unsuspend` self (gated on being suspended, DCGO `CanUnsuspend`)

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "RB1-010";

// ─── Test cards ──────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.dp = Some(dp);
    card.play_cost = 5;
    card
}

/// Lv.5 yellow Digimon with "[Gammamon]" in its effect text — the alt path.
fn gamma_text_lv5(id: &str) -> CardData {
    let mut card = digimon(id, "Yellow Five", CardColor::Yellow, 5, 7000);
    card.effect_text = "[On Play] Search for a card with [Gammamon] in its text.".to_string();
    card
}

/// Gammamon-text Digimon card in hand (name match).
fn gammamon_in_hand(id: &str) -> CardData {
    digimon(id, "Gammamon", CardColor::Red, 3, 3000)
}

/// A Tamer that has [Gammamon] in its text — not a Digimon card, never selectable.
fn gamma_tamer(id: &str) -> CardData {
    let mut card = make_test_card_with_level(id, "Hiro Amanokawa", 0);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.effect_text = "[Your Turn] Your Digimon with [Gammamon] in their texts get +1000 DP."
        .to_string();
    card
}

/// Test-only Gammamon with an inherited +2000 DP aura (Q&A fixture).
const BOOST_GAMMAMON_YAML: &str = r#"
card: T-GAMMA-BOOST
name: Gammamon
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
traits: []
effects:
  - scope: inherited
    kind: aura
    target: {}
    dp_modifier: 2000
    summary: "[Inherited] This Digimon gets +2000 DP"
"#;

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("RB1-010 in embedded DSL pack")
        .add_card(digimon("RED5", "Red Five", CardColor::Red, 5, 7000))
        .add_card(digimon("YEL5", "Plain Yellow Five", CardColor::Yellow, 5, 7000))
        .add_card(gamma_text_lv5("GAMMA5"))
        .add_card(gammamon_in_hand("GAMMA-HAND"))
        .add_card(digimon("PLAIN-HAND", "Agumon", CardColor::Red, 3, 3000))
        .add_card(gamma_tamer("GAMMA-TAMER"))
        .add_card(digimon("OPP-11K", "Opp Eleven", CardColor::Blue, 6, 11000))
        .add_card(digimon("OPP-12K", "Opp Twelve", CardColor::Blue, 6, 12000))
        .add_card(digimon("OPP-13K", "Opp Thirteen", CardColor::Blue, 6, 13000))
        .add_card(digimon("OPP-SMALL", "Opp Small", CardColor::Blue, 3, 3000))
        .add_card(digimon("FILL", "Filler", CardColor::Green, 3, 1000))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(10)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn hand_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

fn hand_action(runner: &DebugRunner, card_id: &str) -> Option<u16> {
    let view = runner.pending_selection_view()?;
    let slot = hand_index(runner, 0, card_id) as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn source_ids(runner: &DebugRunner, handle: PermanentHandle) -> Vec<String> {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .iter()
        .map(|s| s.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn opp_field_action(runner: &DebugRunner, card_id: &str) -> Option<u16> {
    let view = runner.pending_selection_view()?;
    let idx = field_ids(runner, 1).iter().position(|id| id == card_id)? as u16;
    let action = 100 + idx;
    view.valid_action_ids.contains(&action).then_some(action)
}

/// Place `base_id` on P0's field and digivolve Siriusmon (hand slot 0) onto
/// it through the real action pipeline. Returns the permanent handle, or
/// `None` when the digivolve action is illegal.
fn digivolve_onto(runner: &mut DebugRunner, base_id: &str, suspended: bool) -> Option<PermanentHandle> {
    let perm = runner.place_on_field(0, base_id, Some(0));
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = suspended;
    let slot = hand_index(runner, 0, CARD_ID) as u16;
    let action = encode_digivolve(slot, perm.index as u16);
    if build_action_mask(&runner.game, 0)[action as usize] != 1.0 {
        return None;
    }
    runner.game.decode_action(action, 0);
    assert_eq!(field_ids(runner, 0)[perm.index as usize], CARD_ID);
    Some(perm)
}

fn suspend(runner: &mut DebugRunner, h: PermanentHandle) {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn opp_handle(runner: &DebugRunner, card_id: &str) -> PermanentHandle {
    let idx = field_ids(runner, 1)
        .iter()
        .position(|id| id == card_id)
        .unwrap_or_else(|| panic!("{card_id} not on opponent field"));
    PermanentHandle {
        player: 1,
        index: idx as u8,
    }
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn rb1_010_structure_alt_path_and_two_triggers() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled RB1-010");
    assert_eq!(
        card.alt_paths.len(),
        2,
        "red Lv.5 circle + [Digivolve] Lv.5 w/[Gammamon] in text"
    );
    assert!(card
        .alt_paths
        .iter()
        .all(|p| p.kind == CompiledAltPathKind::Digivolve));

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2);
    let wd = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("[When Digivolving] clause");
    assert!(!wd.once_per_turn);
    let del = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnAnyDeletion))
        .expect("deletion observer clause");
    assert!(del.once_per_turn, "[Once Per Turn]");
    assert!(del.optional, "'you may unsuspend'");
    assert!(del.active_when.is_some(), "[Your Turn] gate");
}

// ─── Digivolution requirements ───────────────────────────────────────────────

#[test]
fn rb1_010_digivolves_from_red_lv5_for_3() {
    let mut runner = base().hand(0, &[CARD_ID]).start();
    let before = runner.memory();
    digivolve_onto(&mut runner, "RED5", false).expect("red Lv.5 circle legal");
    assert_eq!(before - runner.memory(), 3);
}

#[test]
fn rb1_010_alt_digivolves_from_lv5_with_gammamon_text_for_3() {
    let mut runner = base().hand(0, &[CARD_ID]).start();
    let before = runner.memory();
    digivolve_onto(&mut runner, "GAMMA5", false).expect("Gammamon-text Lv.5 alt path legal");
    assert_eq!(before - runner.memory(), 3);
}

#[test]
fn rb1_010_cannot_digivolve_from_non_red_lv5_without_gammamon_text() {
    let mut runner = base().hand(0, &[CARD_ID]).start();
    assert!(
        digivolve_onto(&mut runner, "YEL5", false).is_none(),
        "yellow Lv.5 without [Gammamon] in text has no digivolve path"
    );
}

// ─── [When Digivolving] ──────────────────────────────────────────────────────

#[test]
fn rb1_010_wd_place_gammamon_bottom_then_delete_dp_lte_self() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "GAMMA-HAND", "PLAIN-HAND", "GAMMA-TAMER"])
        .start();
    runner.place_on_field(1, "OPP-11K", None);
    runner.place_on_field(1, "OPP-12K", None);
    let sirius = digivolve_onto(&mut runner, "RED5", false).unwrap();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(runner.pending_is_optional(), "the placement cost is optional");
    assert!(hand_action(&runner, "PLAIN-HAND").is_none(), "non-Gammamon Digimon excluded");
    assert!(hand_action(&runner, "GAMMA-TAMER").is_none(), "non-Digimon card excluded");
    let pick = hand_action(&runner, "GAMMA-HAND").expect("Gammamon Digimon selectable");
    runner.execute_action(0, pick).unwrap();

    let sources = source_ids(&runner, sirius);
    assert_eq!(sources.first().map(String::as_str), Some("GAMMA-HAND"), "placed at the BOTTOM: {sources:?}");
    assert!(!hand_contains(&runner, 0, "GAMMA-HAND"));

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert!(opp_field_action(&runner, "OPP-12K").is_none(), "12000 DP > 11000 excluded");
    let target = opp_field_action(&runner, "OPP-11K").expect("11000 DP == 11000 selectable");
    runner.execute_action(0, target).unwrap();
    runner.auto_resolve().unwrap();

    assert_eq!(field_ids(&runner, 1), vec!["OPP-12K".to_string()]);
}

#[test]
fn rb1_010_wd_decline_placement_skips_delete() {
    let mut runner = base().hand(0, &[CARD_ID, "GAMMA-HAND"]).start();
    runner.place_on_field(1, "OPP-SMALL", None);
    let sirius = digivolve_onto(&mut runner, "RED5", false).unwrap();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    runner.execute_action(0, PASS).unwrap();

    assert!(runner.pending_selection().is_none(), "no delete prompt after declining the cost");
    assert_eq!(field_ids(&runner, 1), vec!["OPP-SMALL".to_string()]);
    assert!(hand_contains(&runner, 0, "GAMMA-HAND"));
    assert!(!source_ids(&runner, sirius).contains(&"GAMMA-HAND".to_string()));
}

#[test]
fn rb1_010_wd_no_gammamon_digimon_in_hand_no_prompt() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "PLAIN-HAND", "GAMMA-TAMER"])
        .start();
    runner.place_on_field(1, "OPP-SMALL", None);
    digivolve_onto(&mut runner, "RED5", false).unwrap();

    assert!(
        runner.pending_kind() != Some(SelectionKind::Hand)
            && runner.pending_kind() != Some(SelectionKind::OppField),
        "cost unpayable → effect does nothing: {:?}",
        runner.pending_kind()
    );
    runner.auto_resolve().unwrap();
    assert_eq!(field_ids(&runner, 1), vec!["OPP-SMALL".to_string()]);
}

#[test]
fn rb1_010_wd_places_even_when_no_valid_delete_target() {
    let mut runner = base().hand(0, &[CARD_ID, "GAMMA-HAND"]).start();
    runner.place_on_field(1, "OPP-12K", None);
    let sirius = digivolve_onto(&mut runner, "RED5", false).unwrap();

    let pick = hand_action(&runner, "GAMMA-HAND").unwrap();
    runner.execute_action(0, pick).unwrap();
    assert!(runner.pending_kind() != Some(SelectionKind::OppField));
    runner.auto_resolve().unwrap();

    assert_eq!(source_ids(&runner, sirius)[0], "GAMMA-HAND");
    assert_eq!(field_ids(&runner, 1), vec!["OPP-12K".to_string()]);
}

/// Official Q&A: the threshold is this Digimon's DP after the placed card's
/// inherited DP boost applies (11000 + 2000 = 13000).
#[test]
fn rb1_010_wd_dp_threshold_includes_placed_inherited_boost() {
    let mut runner = base()
        .from_dsl_yaml(BOOST_GAMMAMON_YAML)
        .expect("boost Gammamon compiles")
        .hand(0, &[CARD_ID, "T-GAMMA-BOOST"])
        .start();
    runner.place_on_field(1, "OPP-13K", None);
    digivolve_onto(&mut runner, "RED5", false).unwrap();

    let pick = hand_action(&runner, "T-GAMMA-BOOST").expect("boost Gammamon selectable");
    runner.execute_action(0, pick).unwrap();
    let target = opp_field_action(&runner, "OPP-13K")
        .expect("13000 DP deletable after the +2000 inherited boost");
    runner.execute_action(0, target).unwrap();
    runner.auto_resolve().unwrap();
    assert!(field_ids(&runner, 1).is_empty());
}

#[test]
fn rb1_010_wd_without_boost_13k_not_deletable() {
    let mut runner = base().hand(0, &[CARD_ID, "GAMMA-HAND"]).start();
    runner.place_on_field(1, "OPP-13K", None);
    digivolve_onto(&mut runner, "RED5", false).unwrap();
    let pick = hand_action(&runner, "GAMMA-HAND").unwrap();
    runner.execute_action(0, pick).unwrap();
    assert!(opp_field_action(&runner, "OPP-13K").is_none());
    runner.auto_resolve().unwrap();
    assert_eq!(field_ids(&runner, 1), vec!["OPP-13K".to_string()]);
}

// ─── [Your Turn][Once Per Turn] unsuspend ────────────────────────────────────

#[test]
fn rb1_010_unsuspends_when_opponent_digimon_deleted_on_your_turn() {
    let mut runner = base().start();
    let sirius = runner.place_on_field(0, CARD_ID, Some(0));
    suspend(&mut runner, sirius);
    let opp = runner.place_on_field(1, "OPP-SMALL", None);

    runner.game.delete_permanent_with_effects(opp);
    assert!(runner.pending_selection().is_some(), "optional unsuspend prompt");
    assert!(runner.pending_is_optional());
    runner.accept_optional_trigger().unwrap();
    runner.auto_resolve().unwrap();
    assert!(!is_suspended(&runner, sirius));
}

#[test]
fn rb1_010_unsuspend_decline_stays_suspended() {
    let mut runner = base().start();
    let sirius = runner.place_on_field(0, CARD_ID, Some(0));
    suspend(&mut runner, sirius);
    let opp = runner.place_on_field(1, "OPP-SMALL", None);

    runner.game.delete_permanent_with_effects(opp);
    assert!(runner.pending_is_optional());
    runner.decline_optional_trigger().unwrap();
    assert!(is_suspended(&runner, sirius));
}

#[test]
fn rb1_010_own_digimon_deleted_does_not_trigger() {
    let mut runner = base().start();
    let sirius = runner.place_on_field(0, CARD_ID, Some(0));
    suspend(&mut runner, sirius);
    let own = runner.place_on_field(0, "FILL", None);

    runner.game.delete_permanent_with_effects(own);
    assert!(runner.pending_selection().is_none());
    assert!(is_suspended(&runner, sirius));
}

#[test]
fn rb1_010_no_trigger_on_opponents_turn() {
    let mut runner = base().start();
    let sirius = runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn();
    runner.auto_resolve().ok();
    assert_eq!(runner.turn_player(), 1);
    suspend(&mut runner, sirius);
    let opp = runner.place_on_field(1, "OPP-SMALL", None);

    runner.game.delete_permanent_with_effects(opp);
    assert!(runner.pending_selection().is_none(), "[Your Turn] only");
    assert!(is_suspended(&runner, sirius));
}

#[test]
fn rb1_010_unsuspend_is_once_per_turn_and_resets() {
    let mut runner = base().start();
    let sirius = runner.place_on_field(0, CARD_ID, Some(0));
    suspend(&mut runner, sirius);
    runner.place_on_field(1, "OPP-SMALL", None);
    runner.place_on_field(1, "OPP-11K", None);
    runner.place_on_field(1, "OPP-12K", None);

    let a = opp_handle(&runner, "OPP-SMALL");
    runner.game.delete_permanent_with_effects(a);
    runner.accept_optional_trigger().unwrap();
    runner.auto_resolve().unwrap();
    assert!(!is_suspended(&runner, sirius));

    suspend(&mut runner, sirius);
    let b = opp_handle(&runner, "OPP-11K");
    runner.game.delete_permanent_with_effects(b);
    assert!(runner.pending_selection().is_none(), "OPT spent this turn");
    assert!(is_suspended(&runner, sirius));

    // Cycle to P0's next turn: OPT resets.
    runner.end_turn();
    runner.auto_resolve().ok();
    runner.end_turn();
    runner.auto_resolve().ok();
    assert_eq!(runner.turn_player(), 0);
    suspend(&mut runner, sirius);
    let c = opp_handle(&runner, "OPP-12K");
    runner.game.delete_permanent_with_effects(c);
    assert!(runner.pending_is_optional(), "OPT reset on the next own turn");
    runner.accept_optional_trigger().unwrap();
    runner.auto_resolve().unwrap();
    assert!(!is_suspended(&runner, sirius));
}

/// DCGO `CanActivateCondition` requires `CanUnsuspend`: an unsuspended
/// Siriusmon neither prompts nor spends its Once Per Turn.
#[test]
fn rb1_010_already_unsuspended_no_prompt_and_opt_not_spent() {
    let mut runner = base().start();
    let sirius = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-SMALL", None);
    runner.place_on_field(1, "OPP-11K", None);

    let a = opp_handle(&runner, "OPP-SMALL");
    runner.game.delete_permanent_with_effects(a);
    assert!(runner.pending_selection().is_none());

    suspend(&mut runner, sirius);
    let b = opp_handle(&runner, "OPP-11K");
    runner.game.delete_permanent_with_effects(b);
    assert!(runner.pending_is_optional(), "OPT still available");
    runner.accept_optional_trigger().unwrap();
    runner.auto_resolve().unwrap();
    assert!(!is_suspended(&runner, sirius));
}

// ─── Integrated: WD delete feeds the unsuspend observer ──────────────────────

#[test]
fn rb1_010_integrated_digivolve_on_suspended_delete_then_unsuspend() {
    let mut runner = base().hand(0, &[CARD_ID, "GAMMA-HAND"]).start();
    runner.place_on_field(1, "OPP-11K", None);
    let sirius = digivolve_onto(&mut runner, "GAMMA5", true).unwrap();
    assert!(is_suspended(&runner, sirius), "digivolving keeps the suspended state");

    let pick = hand_action(&runner, "GAMMA-HAND").unwrap();
    runner.execute_action(0, pick).unwrap();
    let target = opp_field_action(&runner, "OPP-11K").unwrap();
    runner.execute_action(0, target).unwrap();

    assert!(field_ids(&runner, 1).is_empty());
    assert!(runner.pending_is_optional(), "unsuspend observer fires off the WD deletion");
    runner.accept_optional_trigger().unwrap();
    runner.auto_resolve().unwrap();
    assert!(!is_suspended(&runner, sirius));
}
