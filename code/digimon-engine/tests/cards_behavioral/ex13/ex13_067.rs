//! EX13-067 Nokia Shiramine — Tamer, Red/Blue, Cost 4. Traits: CS.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-067.md`;
//! `data/card_overrides.json` carries the "... or 1 [Agumon]" correction over
//! the per-card JSON. The corpus files the [Security] line under
//! `inherited_effect_description_eng`.)
//!
//! ```text
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [Your Turn] When any of your Digimon digivolve, if you have 1 or fewer
//! Digimon, by suspending this Tamer, you may play 1 [Gabumon] if that Digimon
//! has [Greymon] in its name or 1 [Agumon] if it has [Garurumon] in its name
//! from your hand or trash without paying the cost.
//!
//! [Security] Play this card without paying the cost.
//! ```
//!
//! Official Q&A: two copies — the moment the first copy's [Your Turn] effect
//! resolves you no longer have 1 or fewer Digimon, so the second copy can't
//! activate (the condition is re-checked at resolution).
//!
//! # DCGO C# reference
//! None — DCGO (b9a0638cd) ships no EX13_067.cs. Authored from the official
//! printed text + Q&A; the shape mirrors the closest DCGO idioms
//! (`CanActivateSuspendCostEffect`, `PlayPermanentCards(payCost: false)` over a
//! hand-or-trash union pick, `PlaySelfTamerSecurityEffect`).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - Start-of-main memory gain gated on the opponent's board.
//! - F own-Digimon-digivolved observer (BT24-082 `on_digivolve` idiom) with an
//!   own-Digimon-count gate and a suspend-self cost → union hand/trash free play
//!   whose eligible name depends on the digivolved Digimon's name.
//! - Tamer [Security] play self.

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledClause, CompiledTiming, CompiledTriggeredClause};
use digimon_engine::action::space::{
    encode_digivolve, PASS, PLAY_HAND_END, PLAY_HAND_START, TRASH_EFFECT_END, TRASH_EFFECT_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{PendingSelectionView, SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-067";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.colors = vec![CardColor::Red];
    c
}

/// Lv.4 red that digivolves from a red Lv.3 for 2.
fn evo(id: &str, name: &str) -> CardData {
    let mut c = digimon(id, name, 4);
    c.play_cost = 5;
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 2,
    }];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-067 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("BASE-L3", "Base Three", 3))
        .add_card(digimon("GABUMON", "Gabumon", 3))
        .add_card(digimon("AGUMON", "Agumon", 3))
        .add_card(digimon("GABUMON-X", "Gabumon X", 3))
        .add_card(evo("GREYMON-EVO", "Greymon"))
        .add_card(evo("GARURUMON-EVO", "Garurumon"))
        .add_card(evo("PLAIN-EVO", "Plain Evo"))
        .add_card(digimon("FILL", "Fill", 3))
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
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

/// Nokia (unsuspended) + one own red Lv.3, `hand` in hand, `trash` in trash.
fn setup(hand: &[&str], trash: &[&str]) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    runner.skip_mulligan();
    for id in trash {
        runner.inject_trash(0, id);
    }
    let nokia = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "BASE-L3", Some(0));
    (runner, nokia, base)
}

fn digivolve(runner: &mut DebugRunner, evo_id: &str, base: PermanentHandle) {
    let slot = hand_index(runner, evo_id) as u16;
    runner.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
}

fn accept_confirm(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("pre-cost confirm pending");
    assert!(view.is_optional, "\"by suspending this Tamer, you may\": {view:?}");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("accept");
    runner.execute_action(view.selecting_player, accept).expect("accept");
}

/// Ids offered at the hand/trash union prompt, in a stable order.
fn union_offered(runner: &DebugRunner) -> Vec<String> {
    let view: PendingSelectionView = runner.pending_selection_view().expect("union prompt");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }), "{view:?}");
    let p = &runner.game.players[0];
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter_map(|&a| {
            if (PLAY_HAND_START..PLAY_HAND_END).contains(&a) {
                p.hand.get((a - PLAY_HAND_START) as usize)
            } else if (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&a) {
                p.trash.get((a - TRASH_EFFECT_START) as usize)
            } else {
                None
            }
        })
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    ids.sort();
    ids
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_067_clause_shape_matches_printed_text() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.traits, vec!["CS".to_string()]);
    let t = triggered(&runner);
    assert_eq!(t.len(), 3, "start-of-main + digivolve observer + [Security]");
    let obs = t
        .iter()
        .find(|c| c.when == vec![CompiledTiming::OnDigivolve])
        .expect("digivolve observer");
    assert!(obs.optional);
    assert!(!obs.once_per_turn, "no [Once Per Turn] printed");
    assert!(t.iter().any(|c| c.when == vec![CompiledTiming::StartOfYourMainPhase]));
    assert!(t.iter().any(|c| c.when == vec![CompiledTiming::OnSecurity]));
}

// ─── Section 2/3 — [Start of Your Main Phase] ────────────────────────────────

fn fire_start_of_main(runner: &mut DebugRunner, nokia: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(nokia));
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();
}

#[test]
fn ex13_067_start_of_main_gains_1_when_the_opponent_has_a_digimon() {
    let (mut runner, nokia, _) = setup(&[], &[]);
    runner.place_on_field(1, "FILL", Some(0));
    let before = runner.memory();
    fire_start_of_main(&mut runner, nokia);
    assert_eq!(runner.memory(), before + 1);
}

#[test]
fn ex13_067_start_of_main_no_gain_when_the_opponent_has_no_digimon() {
    let (mut runner, nokia, _) = setup(&[], &[]);
    let before = runner.memory();
    fire_start_of_main(&mut runner, nokia);
    assert_eq!(runner.memory(), before);
}

// ─── Section 2/3 — [Your Turn] digivolve observer ────────────────────────────

#[test]
fn ex13_067_greymon_digivolve_plays_gabumon_from_hand_free() {
    let (mut runner, nokia, base) = setup(&["GREYMON-EVO", "GABUMON"], &[]);
    digivolve(&mut runner, "GREYMON-EVO", base);
    let before = runner.memory();
    accept_confirm(&mut runner);
    assert!(suspended(&runner, nokia), "cost: this Tamer suspended");
    assert_eq!(union_offered(&runner), vec!["GABUMON"]);
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"GABUMON".to_string()));
    assert_eq!(runner.memory(), before, "without paying the cost");
}

#[test]
fn ex13_067_greymon_digivolve_can_play_gabumon_from_trash() {
    let (mut runner, _nokia, base) = setup(&["GREYMON-EVO"], &["GABUMON"]);
    digivolve(&mut runner, "GREYMON-EVO", base);
    accept_confirm(&mut runner);
    assert_eq!(union_offered(&runner), vec!["GABUMON"]);
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"GABUMON".to_string()));
    assert_eq!(runner.trash_size(0), 0);
}

#[test]
fn ex13_067_greymon_branch_does_not_offer_agumon_or_name_variants() {
    let (mut runner, _nokia, base) = setup(&["GREYMON-EVO", "AGUMON", "GABUMON-X", "GABUMON"], &[]);
    digivolve(&mut runner, "GREYMON-EVO", base);
    accept_confirm(&mut runner);
    assert_eq!(union_offered(&runner), vec!["GABUMON"], "exactly [Gabumon]");
}

#[test]
fn ex13_067_garurumon_digivolve_plays_agumon() {
    let (mut runner, _nokia, base) = setup(&["GARURUMON-EVO", "AGUMON", "GABUMON"], &[]);
    digivolve(&mut runner, "GARURUMON-EVO", base);
    accept_confirm(&mut runner);
    assert_eq!(union_offered(&runner), vec!["AGUMON"]);
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"AGUMON".to_string()));
}

#[test]
fn ex13_067_other_names_do_not_trigger() {
    let (mut runner, nokia, base) = setup(&["PLAIN-EVO", "AGUMON", "GABUMON"], &[]);
    digivolve(&mut runner, "PLAIN-EVO", base);
    assert!(runner.pending_selection().is_none(), "neither [Greymon] nor [Garurumon]");
    assert!(!suspended(&runner, nokia));
}

#[test]
fn ex13_067_two_digimon_do_not_trigger() {
    let (mut runner, nokia, base) = setup(&["GREYMON-EVO", "GABUMON"], &[]);
    runner.place_on_field(0, "FILL", Some(0));
    digivolve(&mut runner, "GREYMON-EVO", base);
    assert!(runner.pending_selection().is_none(), "needs 1 or fewer Digimon");
    assert!(!suspended(&runner, nokia));
}

#[test]
fn ex13_067_no_playable_card_does_not_trigger() {
    let (mut runner, nokia, base) = setup(&["GREYMON-EVO", "AGUMON"], &[]);
    digivolve(&mut runner, "GREYMON-EVO", base);
    assert!(runner.pending_selection().is_none(), "no [Gabumon] in hand or trash");
    assert!(!suspended(&runner, nokia));
}

#[test]
fn ex13_067_suspended_nokia_cannot_pay_the_cost() {
    let (mut runner, nokia, base) = setup(&["GREYMON-EVO", "GABUMON"], &[]);
    runner.game.players[0].battle_area[nokia.index as usize].is_suspended = true;
    digivolve(&mut runner, "GREYMON-EVO", base);
    assert!(runner.pending_selection().is_none());
    assert_eq!(hand_ids(&runner).iter().filter(|h| *h == "GABUMON").count(), 1);
}

#[test]
fn ex13_067_declining_leaves_nokia_unsuspended() {
    let (mut runner, nokia, base) = setup(&["GREYMON-EVO", "GABUMON"], &[]);
    digivolve(&mut runner, "GREYMON-EVO", base);
    let view = runner.pending_selection_view().expect("confirm");
    runner.execute_action(view.selecting_player, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert!(!suspended(&runner, nokia));
    assert!(!field_ids(&runner, 0).contains(&"GABUMON".to_string()));
}

#[test]
fn ex13_067_second_copy_fails_the_1_or_fewer_recheck() {
    // Official Q&A: two Nokias; after the first plays Gabumon you have 2 Digimon,
    // so the second copy's effect can't activate.
    let (mut runner, _nokia, base) = setup(&["GREYMON-EVO", "GABUMON", "GABUMON"], &[]);
    let nokia2 = runner.place_on_field(0, CARD_ID, Some(0));
    digivolve(&mut runner, "GREYMON-EVO", base);
    // Resolve whichever copy goes first (accepting any trigger-order prompt).
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else { break };
        if matches!(view.kind, SelectionKind::UnionZone { .. }) {
            let _ = runner.auto_resolve();
            break;
        }
        let accept = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        runner.execute_action(view.selecting_player, accept).unwrap();
    }
    let _ = runner.auto_resolve();
    let gabumons = field_ids(&runner, 0).iter().filter(|f| *f == "GABUMON").count();
    assert_eq!(gabumons, 1, "only one copy resolves");
    let nokias_suspended = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID && p.is_suspended)
        .count();
    assert_eq!(nokias_suspended, 1, "the second copy never paid its cost");
    let _ = nokia2;
}

#[test]
fn ex13_067_does_not_trigger_on_the_opponents_turn() {
    let (mut runner, nokia, base) = setup(&["GREYMON-EVO", "GABUMON"], &[]);
    runner.end_turn();
    let _ = runner.auto_resolve();
    // Effect-initiated digivolve of our Digimon during the opponent's turn.
    let slot = hand_index(&runner, "GREYMON-EVO");
    runner.game.effect_initiated_digivolve(
        0,
        slot,
        base,
        digimon_engine::enums::CostDelta::Free,
        false,
        digimon_engine::enums::PlaySource::ByEffect,
    );
    runner.game.drain_effect_queue();
    assert!(runner.pending_selection().is_none(), "[Your Turn]");
    assert!(!suspended(&runner, nokia));
}

// ─── [Security] ──────────────────────────────────────────────────────────────

#[test]
fn ex13_067_security_plays_itself_for_free() {
    let mut runner = builder()
        .deck(0, &["FILL"; 3])
        .deck(1, &["FILL"; 3])
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    runner.skip_mulligan();
    let attacker = runner.place_on_field(0, "GREYMON-EVO", Some(0));
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 1), vec![CARD_ID], "Nokia played from security");
    assert_eq!(runner.memory(), 3, "without paying the cost");
}
