//! EX13-019 Veedramon — Digimon, Lv.4, Blue, DP 6000, Cost 6.
//! Traits: Mythical Dragon, CS. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-019.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-019.md` agrees)
//!
//! ```text
//! Digivolve: Blue Lv.3 / cost 2.  [Digivolve] Lv.3 w/[CS] trait: Cost 2
//!
//! Effect:
//! <Jamming> (This Digimon can't be deleted in battles against Security Digimon.)
//! [When Attacking] [Once Per Turn] You may play 1 Tamer card with [Veedramon]
//! in its text from your hand with the cost reduced by 2.
//!
//! Inherited Effect:
//! <Jamming> (This Digimon can't be deleted in battles against Security Digimon.)
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Blue/EX13_019.cs
//! - Alt digivolve `HasCSTraits`, level 3, cost 2.
//! - `JammingSelfStaticEffect` face-up AND inherited.
//! - `OnAllyAttack` ActivateClass, maxCountPerTurn 1, skippable; activatable
//!   only with an eligible Tamer in hand; `SelectHandEffect(canNoSelect: true)`;
//!   nothing selected → `RemoveUse()` (OPT not consumed); play with
//!   `ChangeCostClass(-2)`, `payCost: true`.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - D keyword grant (face-up + inherited <Jamming>).
//! - E [When Attacking][OPT] optional hand pick → play with cost reduction,
//!   `refund_opt` on decline (BT25-085 idiom).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_digivolve, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-019";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn tamer(id: &str, text: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![CardColor::Blue];
    c.effect_text = text.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-019 YAML parses, compiles and is in the embedded pack")
        .add_card(tamer("VEE-TAMER", "[Start of Your Main Phase] If you have [Veedramon]...", 4))
        .add_card(tamer("PLAIN-TAMER", "", 4))
        .add_card({
            let mut c = make_test_card("VEE-DIGI", "Text Digimon");
            c.effect_text = "[Veedramon]".to_string();
            c
        })
        .add_card({
            let mut c = make_test_card("CS-L3", "CS Rookie");
            c.colors = vec![CardColor::Yellow];
            c.traits = vec!["CS".to_string()];
            c
        })
        .add_card({
            let mut c = make_test_card("CARRIER-L5", "Carrier Five");
            c.level = Some(5);
            c
        })
        .add_card(make_test_card("FILL", "Fill"))
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in hand"))
}

fn triggered(runner: &DebugRunner) -> Vec<CompiledTriggeredClause> {
    runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

/// Veedramon on the field (not summoning-sick), `hand` in hand, ready to attack.
fn attack_setup(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let veed = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, veed)
}

/// Walk to the hand prompt (accepting any yes/no gate) and return the view.
fn hand_prompt(runner: &mut DebugRunner) -> Option<digimon_engine::selection::PendingSelectionView> {
    for _ in 0..4 {
        let view = runner.pending_selection_view()?;
        if view.kind == SelectionKind::Hand {
            return Some(view);
        }
        let accept = view.valid_action_ids.iter().copied().find(|&a| a != PASS)?;
        runner.execute_action(view.selecting_player, accept).ok()?;
    }
    None
}

fn offered(runner: &DebugRunner, view: &digimon_engine::selection::PendingSelectionView, id: &str) -> bool {
    let slot = hand_index(runner, id) as u16;
    view.valid_action_ids.contains(&(PLAY_HAND_START + slot))
        || view
            .valid_action_ids
            .contains(&(digimon_engine::action::space::HAND_EFFECT_START + slot))
}

fn pick(runner: &mut DebugRunner, id: &str) {
    let view = hand_prompt(runner).expect("hand prompt");
    let slot = hand_index(runner, id) as u16;
    let action = [PLAY_HAND_START + slot, digimon_engine::action::space::HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} must be selectable: {view:?}"));
    runner.execute_action(view.selecting_player, action).expect("pick");
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_019_has_the_blue_circle_and_the_cs_trait_alt_digivolve() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(paths, 2, "Blue Lv.3/2 + Lv.3 w/[CS] trait/2");
}

#[test]
fn ex13_019_clause_shape_matches_printed_text() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let mut jamming_scopes: Vec<CompiledScope> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, keyword, .. })
                if keyword.eq_ignore_ascii_case("Jamming") =>
            {
                Some(*scope)
            }
            _ => None,
        })
        .collect();
    jamming_scopes.sort_by_key(|s| format!("{s:?}"));
    assert_eq!(jamming_scopes, vec![CompiledScope::FaceUp, CompiledScope::Inherited]);

    let t = triggered(&runner);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].when, vec![CompiledTiming::WhenAttacking]);
    assert!(t[0].once_per_turn, "[Once Per Turn]");
    assert_eq!(t[0].scope, CompiledScope::FaceUp);
}

#[test]
fn ex13_019_face_up_has_jamming() {
    let (mut runner, veed) = attack_setup(&[]);
    runner.game.tick_declarative_effects();
    assert!(runner.game.has_keyword(veed, Keyword::Jamming));
}

#[test]
fn ex13_019_inherited_jamming_reaches_the_carrier() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-L5"]);
    let plain = runner.place_on_field(0, "CARRIER-L5", Some(0));
    runner.game.tick_declarative_effects();
    assert!(runner.game.has_keyword(carrier, Keyword::Jamming));
    assert!(!runner.game.has_keyword(plain, Keyword::Jamming), "control");
}

#[test]
fn ex13_019_digivolves_from_a_cs_trait_lv3_for_2() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "CS-L3", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    assert_eq!(field_ids(&runner, 0), vec![CARD_ID]);
    assert_eq!(runner.memory(), 3, "Lv.3 w/[CS] trait: Cost 2");
}

// ─── Section 2/3/5 — [When Attacking][OPT] play a [Veedramon]-text Tamer ─────

#[test]
fn ex13_019_when_attacking_plays_a_veedramon_tamer_for_2_less() {
    let (mut runner, veed) = attack_setup(&["VEE-TAMER"]);
    let before = runner.memory();
    runner.attack_player(veed, 1, false);
    pick(&mut runner, "VEE-TAMER");
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"VEE-TAMER".to_string()), "Tamer played");
    assert_eq!(before - runner.memory(), 2, "cost 4 reduced by 2");
}

#[test]
fn ex13_019_tamer_without_veedramon_text_is_not_offered() {
    let (mut runner, veed) = attack_setup(&["VEE-TAMER", "PLAIN-TAMER", "VEE-DIGI"]);
    runner.attack_player(veed, 1, false);
    let view = hand_prompt(&mut runner).expect("hand prompt for the eligible Tamer");
    assert!(offered(&runner, &view, "VEE-TAMER"));
    assert!(!offered(&runner, &view, "PLAIN-TAMER"), "no [Veedramon] in its text");
    assert!(!offered(&runner, &view, "VEE-DIGI"), "not a Tamer card");
}

#[test]
fn ex13_019_no_eligible_tamer_installs_no_prompt() {
    let (mut runner, veed) = attack_setup(&["PLAIN-TAMER"]);
    runner.attack_player(veed, 1, false);
    assert!(
        runner
            .pending_selection_view()
            .map_or(true, |v| v.kind != SelectionKind::Hand),
        "nothing to play"
    );
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner, 0), vec!["PLAIN-TAMER"]);
}

#[test]
fn ex13_019_play_is_optional_and_declining_refunds_the_opt() {
    let (mut runner, veed) = attack_setup(&["VEE-TAMER"]);
    runner.attack_player(veed, 1, false);
    let view = hand_prompt(&mut runner).expect("hand prompt");
    assert!(view.is_optional, "\"you may\"");
    runner.execute_action(view.selecting_player, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner, 0), vec!["VEE-TAMER"]);

    // Declined → OPT not consumed: a second attack this turn re-offers it.
    runner.game.players[0].battle_area[veed.index as usize].is_suspended = false;
    runner.attack_player(veed, 1, false);
    assert!(hand_prompt(&mut runner).is_some(), "RemoveUse: OPT still available");
}

#[test]
fn ex13_019_when_attacking_is_once_per_turn() {
    let (mut runner, veed) = attack_setup(&["VEE-TAMER", "VEE-TAMER"]);
    runner.attack_player(veed, 1, false);
    pick(&mut runner, "VEE-TAMER");
    let _ = runner.auto_resolve();
    runner.game.players[0].battle_area[veed.index as usize].is_suspended = false;
    runner.attack_player(veed, 1, false);
    assert!(hand_prompt(&mut runner).is_none(), "[Once Per Turn] used");
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner, 0), vec!["VEE-TAMER"]);
}
