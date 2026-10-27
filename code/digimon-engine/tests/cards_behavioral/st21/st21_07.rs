//! ST21-07 Palmon — Digimon, Lv.3, Green, DP 1000, Cost 3.
//! Traits: Vegetation / ADVENTURE. Form: Rookie. Attribute: Data.
//!
//! # Card text (official bundle `data/card_bundles/ST21-07.md`)
//!
//! ```text
//! Digivolve: Green Lv.2 / cost 0.
//! [Digivolve] Lv.2 w/[ADVENTURE] trait: Cost 0
//!
//! [On Play] By trashing 1 card with the [ADVENTURE] trait in your hand,
//! <Draw 2> (Draw 2 cards from your deck.)
//!
//! Inherited: [All Turns] This Digimon gets +1000 DP.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Green/ST21_07.cs` — optional
//! activation, gated on an ADVENTURE card in hand; `SelectHandEffect(Discard,
//! canNoSelect: true, maxCount 1)` and only if a card was discarded,
//! `DrawClass(2)`. Inherited `ChangeSelfDPStaticEffect(+1000, null)`.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "ST21-07";

fn builder() -> DebugRunnerBuilder {
    let mut adv_digi = make_test_card("ADV-DIGI", "Adv Digimon");
    adv_digi.traits = vec!["ADVENTURE".to_string()];
    let mut adv_opt = make_test_card("ADV-OPTION", "Adv Option");
    adv_opt.card_kind = CardKind::Option;
    adv_opt.level = None;
    adv_opt.dp = None;
    adv_opt.traits = vec!["ADVENTURE".to_string()];
    let mut adv_lv2 = make_test_card("ADV-LV2", "Adv Baby");
    adv_lv2.level = Some(2);
    adv_lv2.colors = vec![CardColor::Black];
    adv_lv2.traits = vec!["ADVENTURE".to_string()];
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.level = Some(4);
    carrier.dp = Some(4000);
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-07 YAML parses, compiles and is in the embedded pack")
        .add_card(adv_digi)
        .add_card(adv_opt)
        .add_card(make_test_card("PLAIN", "Plain"))
        .add_card(adv_lv2)
        .add_card(carrier)
        .add_card(make_test_card("FILL", "Fill"))
}

fn ids(runner: &DebugRunner, zone: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    zone.iter().map(|c| c.card_id(&runner.game.card_data).to_string()).collect()
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in hand"))
}

fn hand_actions(runner: &DebugRunner, id: &str) -> [u16; 2] {
    let slot = hand_index(runner, id) as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
}

fn walk_to_hand_prompt(runner: &mut DebugRunner) -> Option<digimon_engine::selection::PendingSelectionView> {
    for _ in 0..4 {
        let view = runner.pending_selection_view()?;
        if view.kind == SelectionKind::Hand {
            return Some(view);
        }
        let action = view.valid_action_ids.iter().copied().find(|&a| a != PASS)?;
        runner.execute_action(view.selecting_player, action).expect("accept gate");
    }
    None
}

fn play_palmon(hand: &[&str]) -> DebugRunner {
    let mut all = vec![CARD_ID];
    all.extend_from_slice(hand);
    let mut runner = builder()
        .hand(0, &all)
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    let slot = hand_index(&runner, CARD_ID);
    runner.play(0, slot).expect("Palmon plays");
    runner
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st21_07_identity_paths_and_clauses() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((card.level, card.dp, card.cost), (Some(3), Some(1000), Some(3)));
    let paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(paths, 2, "Green Lv.2/0 circle + Lv.2 w/[ADVENTURE]/0 box");
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnPlay] && !t.once_per_turn
    )));
    let auras: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, dp_modifier, .. }) => {
                Some((*scope, *dp_modifier))
            }
            _ => None,
        })
        .collect();
    assert_eq!(auras, vec![(CompiledScope::Inherited, Some(1000))]);
}

// ─── Section 2/3 — [On Play] trash-cost → <Draw 2> ───────────────────────────

#[test]
fn st21_07_trashing_an_adventure_card_draws_two() {
    let mut runner = play_palmon(&["ADV-DIGI", "PLAIN"]);
    let view = walk_to_hand_prompt(&mut runner).expect("trash prompt");
    let action = hand_actions(&runner, "ADV-DIGI")
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .expect("ADV-DIGI is a legal cost");
    runner.execute_action(view.selecting_player, action).expect("trash");
    let _ = runner.auto_resolve();
    assert!(ids(&runner, &runner.game.players[0].trash).contains(&"ADV-DIGI".to_string()));
    let hand = ids(&runner, &runner.game.players[0].hand);
    assert_eq!(hand.iter().filter(|h| *h == "FILL").count(), 2, "<Draw 2>");
    assert!(hand.contains(&"PLAIN".to_string()));
}

#[test]
fn st21_07_adventure_option_is_a_legal_cost_but_plain_is_not() {
    let mut runner = play_palmon(&["ADV-OPTION", "PLAIN"]);
    let view = walk_to_hand_prompt(&mut runner).expect("trash prompt");
    assert!(hand_actions(&runner, "ADV-OPTION").iter().any(|a| view.valid_action_ids.contains(a)));
    assert!(!hand_actions(&runner, "PLAIN").iter().any(|a| view.valid_action_ids.contains(a)));
}

#[test]
fn st21_07_declining_the_cost_draws_nothing() {
    let mut runner = play_palmon(&["ADV-DIGI"]);
    for _ in 0..4 {
        let Some(view) = runner.pending_selection_view() else { break };
        assert!(view.is_optional, "the trash cost is optional");
        runner.execute_action(view.selecting_player, PASS).expect("decline");
    }
    assert_eq!(ids(&runner, &runner.game.players[0].hand), vec!["ADV-DIGI"]);
    assert!(runner.game.players[0].trash.is_empty());
    assert_eq!(runner.deck_size(0), 4);
}

#[test]
fn st21_07_without_an_adventure_card_in_hand_nothing_happens() {
    let runner = play_palmon(&["PLAIN"]);
    assert!(runner.pending_selection().is_none());
    assert_eq!(ids(&runner, &runner.game.players[0].hand), vec!["PLAIN"]);
}

// ─── Section 4 — digivolve box + inherited ───────────────────────────────────

#[test]
fn st21_07_digivolves_from_an_off_color_adventure_lv2_for_0() {
    let mut runner = builder().hand(0, &[CARD_ID]).deck(0, &["FILL", "FILL"]).memory(5).start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "ADV-LV2", Some(0));
    let before = runner.memory();
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    assert_eq!(runner.game.players[base.player as usize].battle_area[base.index as usize].top_card().card_id(&runner.game.card_data), CARD_ID);
    assert_eq!(before, runner.memory());
}

#[test]
fn st21_07_inherited_plus_1000_on_both_turns() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(runner.effective_dp(carrier), Some(5000));
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(carrier), Some(5000));
}
