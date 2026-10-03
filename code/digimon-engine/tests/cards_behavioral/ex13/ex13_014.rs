//! EX13-014 Jesmon — Digimon, Lv.6, Red/White, DP 12000, Cost 12.
//! Traits: Holy Warrior / Royal Knight. Attribute: Data.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-014.md`)
//!
//! [When Digivolving] [When Attacking] [Once Per Turn] You may use 1 use cost
//! 5 or lower [Huckmon] text Option card from your hand or this Digimon's
//! digivolution cards without paying the cost.
//! [All Turns] [Once Per Turn] When any of your Digimon are played, you may
//! delete 1 of your opponent's lowest DP Digimon. Then, if you don't have
//! [Atho, René & Por], you may play 1 [Atho, René & Por] Token.
//! (Digimon/White/6000 DP/＜Reboot＞ ＜Blocker＞ ＜Decoy(Red)/(Black)＞)
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all w/[Huckmon] in text
//! Digivolve: Red Lv.5 / cost 3; Lv.5 w/[Huckmon] in text: cost 3.
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_014.cs); printed text governs.
//!
//! # Patterns this test covers
//! - E2 OPT optional union-zone (hand|material) free Option use (BT25-085
//!   idiom), declining refunds the OPT.
//! - B3-adjacent [All Turns] own-Digimon-played trigger: optional lowest-DP
//!   delete + gated optional token play.
//! - Assembly with "[Huckmon] in text" per-level materials.

#![allow(dead_code, unused_imports)]

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::action::space::{
    ATTACK_START, PASS, PLAY_HAND_START, SOURCE_SELECT_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};
use digimon_engine::{CardEffect, Effect};

const CARD_ID: &str = "EX13-014";

struct OptionMainDraw;

impl CardEffect for OptionMainDraw {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("OptionMain draw 1")
            .process(|ctx: &mut EffectContext| {
                let owner = ctx.player;
                ctx.draw(owner, 1);
            })
            .build()]
    }
}

fn digimon(id: &str, level: u8, dp: i32, text: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.colors = vec![CardColor::Red];
    c.effect_text = text.to_string();
    c
}

fn option(id: &str, cost: u16, text: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![CardColor::Red];
    c.effect_text = text.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-014 YAML loads")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(option("HUCK-OPT", 5, "[Main] ... 1 [Huckmon] ..."))
        .add_card(option("HUCK-OPT-6", 6, "[Main] ... 1 [Huckmon] ..."))
        .add_card(option("PLAIN-OPT", 2, "[Main] Draw 1."))
        .add_card(digimon("H5", 5, 7000, "[On Play] ... [Huckmon] ..."))
        .add_card(digimon("H4", 4, 5000, "[On Play] ... [Huckmon] ..."))
        .add_card(digimon("H3", 3, 3000, "[On Play] ... [Huckmon] ..."))
        .add_card(digimon("ALLY", 3, 3000, ""))
        .add_card(digimon("OPP-LOW", 3, 2000, ""))
        .add_card(digimon("OPP-LOW2", 3, 2000, ""))
        .add_card(digimon("OPP-HIGH", 6, 11000, ""))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, perm: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in hand"))
}

fn trash_index(runner: &DebugRunner, player: u8, card_id: &str) -> u16 {
    runner.game.players[player as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in trash")) as u16
}

fn non_pass(runner: &DebugRunner) -> Vec<u16> {
    runner
        .pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default()
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn ex13_014_metadata_and_alt_paths() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(6), Some(12000), Some(12)));
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2);
    assert!(digi.iter().all(|p| p.cost == Some(CompiledCost::Literal(3))));
    let a = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly");
    assert_eq!(a.cost, Some(CompiledCost::Literal(5)));
    assert_eq!(a.materials.len(), 3);
}

#[test]
fn ex13_014_clause_shapes() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 2);
    let use_clause = t
        .iter()
        .find(|x| x.when == vec![CompiledTiming::WhenDigivolving, CompiledTiming::WhenAttacking])
        .expect("WD/WA");
    assert!(use_clause.once_per_turn);
    let played = t
        .iter()
        .find(|x| x.when == vec![CompiledTiming::OnAllyPlayed])
        .expect("played");
    assert!(played.once_per_turn);
}

// ─── Section 2/3/5 — WD/WA use Option ────────────────────────────────────────

fn use_runner(hand: &[&str], stack: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder().hand(0, hand).memory(3).start();
    for id in ["HUCK-OPT", "HUCK-OPT-6", "PLAIN-OPT"] {
        runner.register_effect(id, Arc::new(OptionMainDraw));
    }
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    for id in stack {
        runner.push_source(me, id);
    }
    (runner, me)
}

#[test]
fn ex13_014_wd_uses_a_huckmon_text_option_from_hand_free() {
    let (mut runner, me) = use_runner(&["HUCK-OPT", "HUCK-OPT-6", "PLAIN-OPT"], &[]);
    let mem0 = runner.memory();
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::MATERIAL
        })
    );
    assert!(runner.pending_is_optional(), "'you may use'");
    assert_eq!(non_pass(&runner).len(), 1, "cost 6 and non-[Huckmon] Options excluded");
    let a = non_pass(&runner)[0];
    runner.execute_action(0, a).expect("use");
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), mem0, "without paying the cost");
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "HUCK-OPT"));
    assert_eq!(runner.hand_size(0), 3, "2 left + 1 drawn by the Option");
}

#[test]
fn ex13_014_wa_uses_a_huckmon_text_option_from_digivolution_cards() {
    let (mut runner, me) = use_runner(&[], &["H5", "HUCK-OPT"]);
    fire(&mut runner, EffectTiming::WhenAttacking, me);
    let a = non_pass(&runner)
        .into_iter()
        .find(|&a| a >= SOURCE_SELECT_START)
        .expect("the stacked Option is offered");
    runner.execute_action(0, a).expect("use from sources");
    let _ = runner.auto_resolve();
    let stack = &runner.game.players[0].battle_area[me.index as usize].card_sources;
    assert!(
        !stack.iter().any(|c| c.card_id(&runner.game.card_data) == "HUCK-OPT"),
        "removed from the stack (Q&A)"
    );
    assert_eq!(runner.hand_size(0), 1, "its body drew 1");
}

#[test]
fn ex13_014_no_eligible_option_no_prompt() {
    let (mut runner, me) = use_runner(&["HUCK-OPT-6", "PLAIN-OPT"], &[]);
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    assert!(runner.pending_selection().is_none());
}

#[test]
fn ex13_014_decline_uses_nothing_and_keeps_the_opt() {
    let (mut runner, me) = use_runner(&["HUCK-OPT"], &[]);
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), 1);
    fire(&mut runner, EffectTiming::WhenAttacking, me);
    assert!(
        runner.pending_selection().is_some(),
        "declining did not spend the [Once Per Turn]"
    );
}

#[test]
fn ex13_014_use_is_once_per_turn() {
    let (mut runner, me) = use_runner(&["HUCK-OPT", "HUCK-OPT"], &[]);
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    let a = non_pass(&runner)[0];
    runner.execute_action(0, a).expect("use");
    let _ = runner.auto_resolve();
    fire(&mut runner, EffectTiming::WhenAttacking, me);
    assert!(runner.pending_selection().is_none(), "OPT spent");
}

// ─── Section 2/3/5 — [All Turns] Digimon played ──────────────────────────────

fn played_runner() -> DebugRunner {
    let mut runner = builder().hand(0, &["ALLY", "ALLY"]).memory(10).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner
}

#[test]
fn ex13_014_digimon_played_may_delete_lowest_dp_then_play_token() {
    let mut runner = played_runner();
    let low = runner.place_on_field(1, "OPP-LOW", Some(0));
    let high = runner.place_on_field(1, "OPP-HIGH", Some(0));
    let slot = hand_index(&runner, 0, "ALLY");
    runner.play(0, slot).expect("play ally");
    let view = runner.pending_selection_view().expect("delete prompt");
    assert!(view.is_optional, "'you may delete'");
    assert!(
        !view.valid_action_ids.contains(&(ATTACK_START + high.index as u16)),
        "only the lowest DP Digimon"
    );
    runner
        .execute_action(0, ATTACK_START + low.index as u16)
        .expect("delete low");
    // Token half: an explicit yes/no choice.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice), "'you may play' the token");
    runner.execute_branch(0).expect("play token");
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 1), vec!["OPP-HIGH".to_string()]);
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_name(&runner.game.card_data) == "Atho, René & Por"),
        "Atho, René & Por token played"
    );
}

#[test]
fn ex13_014_declining_delete_still_offers_the_token() {
    let mut runner = played_runner();
    runner.place_on_field(1, "OPP-LOW", Some(0));
    let slot = hand_index(&runner, 0, "ALLY");
    runner.play(0, slot).expect("play ally");
    runner.execute_action(0, PASS).expect("decline delete");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice), "token prompt still offered");
    runner.execute_branch(1).expect("decline token");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 1);
    assert_eq!(runner.battle_area_size(0), 2, "Jesmon + ally, no token");
}

#[test]
fn ex13_014_no_token_offer_while_you_already_have_atho() {
    let mut runner = played_runner();
    runner.place_on_field(0, "TOKEN_ATHO_RENE_POR", Some(0));
    runner.place_on_field(1, "OPP-LOW", Some(0));
    let slot = hand_index(&runner, 0, "ALLY");
    runner.play(0, slot).expect("play ally");
    runner.execute_action(0, PASS).expect("decline delete");
    assert!(runner.pending_selection().is_none(), "already have Atho → no token offer");
}

#[test]
fn ex13_014_played_trigger_is_once_per_turn() {
    let mut runner = played_runner();
    runner.place_on_field(1, "OPP-LOW", Some(0));
    runner.place_on_field(1, "OPP-LOW2", Some(0));
    let slot = hand_index(&runner, 0, "ALLY");
    runner.play(0, slot).expect("play ally");
    let _ = runner.auto_resolve();
    let before = runner.battle_area_size(1);
    let slot = hand_index(&runner, 0, "ALLY");
    runner.play(0, slot).expect("play ally 2");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), before, "OPT spent");
}

#[test]
fn ex13_014_opponent_play_does_not_trigger() {
    let mut runner = builder()
        .hand(1, &["ALLY"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-LOW", Some(0));
    runner.game.set_memory(-5);
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    let slot = hand_index(&runner, 1, "ALLY");
    runner.play(1, slot).expect("opp plays");
    assert!(runner.pending_selection().is_none(), "only YOUR Digimon");
}

// ─── Section 3 — Assembly ────────────────────────────────────────────────────

#[test]
fn ex13_014_assembly_huckmon_text_line_for_7() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(7).start();
    runner.skip_mulligan();
    for id in ["H5", "H4", "H3"] {
        runner.inject_trash(0, id);
    }
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    for id in ["H5", "H4", "H3"] {
        assert!(runner.game.pending_selection.is_some(), "Assembly element {id}");
        let idx = trash_index(&runner, 0, id);
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 4);
    assert_eq!(mem0 - runner.game.memory, 7);
}
