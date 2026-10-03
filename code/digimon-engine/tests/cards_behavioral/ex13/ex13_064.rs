//! EX13-064 LordKnightmon — Digimon, Lv.6, Purple/Black, DP 12000, Cost 12.
//! Traits: Holy Warrior / Royal Knight / CS. Attribute: Virus.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-064.md`)
//!
//! [When Digivolving] You may play or use 1 play or use cost 8 or lower
//! [Knightmon] text card from your hand or trash without paying the cost.
//! [Your Turn] All of your [Knightmon] text Digimon gain ＜Alliance＞ and
//! ＜Piercing＞.
//! [Your Turn] [Once Per Turn] When any of your other Digimon or Tamers are
//! played, 1 of your [Knightmon] text Digimon may gain ＜Rush＞ and
//! ＜Collision＞ for the turn and attack.
//! Digivolve: Purple Lv.5 / 4; Black Lv.5 / 4; Lv.5 w/[Knightmon] in text: 3;
//! [Digivolve] While you have 3 or fewer security cards, [Rie Kishibe]: Cost 5.
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_064.cs); printed text governs.
//!
//! # Patterns this test covers
//! - Conditional Tamer-sourced digivolve route (security-count gate).
//! - Union-zone (hand|trash) free play-or-use routed by card kind.
//! - D4 [Your Turn] keyword aura over "[Knightmon] text" Digimon.
//! - B3 other-Digimon/Tamer-played trigger → keyword grants + effect attack.

#![allow(dead_code, unused_imports)]

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_digivolve, ATTACK_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::{CardEffect, Effect};

const CARD_ID: &str = "EX13-064";

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

fn digimon(id: &str, name: &str, level: u8, cost: u16, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c.colors = vec![color];
    c
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Purple];
    c
}

fn option(id: &str, cost: u16, text: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![CardColor::Purple];
    c.effect_text = text.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-064 YAML loads")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(tamer("RIE", "Rie Kishibe"))
        .add_card(tamer("OTHER-TAMER", "Other Tamer"))
        .add_card(digimon("KNIGHT", "Knightmon", 5, 7, CardColor::Purple))
        .add_card(digimon("KNIGHT-9", "DarkKnightmon", 6, 9, CardColor::Purple))
        .add_card(digimon("PLAIN", "Plain", 4, 4, CardColor::Purple))
        .add_card(digimon("PURPLE5", "Purple Five", 5, 7, CardColor::Purple))
        .add_card(option("KNIGHT-OPT", 4, "[Main] ... [Knightmon] ..."))
        .add_card(digimon("OPP", "Opp", 4, 4, CardColor::Red))
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

fn non_pass(runner: &DebugRunner) -> Vec<u16> {
    runner
        .pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default()
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in hand"))
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn ex13_064_metadata_and_alt_paths() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(6), Some(12000), Some(12)));
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 4, "2 circles + Lv.5 [Knightmon] text + [Rie Kishibe]");
    let rie = digi
        .iter()
        .find(|p| p.from.as_ref().and_then(|f| f.name_is.as_deref()) == Some("Rie Kishibe"))
        .expect("[Rie Kishibe] route");
    assert_eq!(rie.cost, Some(CompiledCost::Literal(5)));
    assert!(rie.condition.is_some(), "gated on 3 or fewer security cards");
}

#[test]
fn ex13_064_clause_shapes() {
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
    assert!(t
        .iter()
        .any(|x| x.when == vec![CompiledTiming::OnAllyPlayed] && x.once_per_turn));
    assert_eq!(
        c.effects
            .iter()
            .filter(|c| matches!(c, CompiledClause::Declarative(_)))
            .count(),
        2,
        "two [Your Turn] keyword auras"
    );
}

// ─── Section 2/3 — [Rie Kishibe] route ───────────────────────────────────────

fn rie_runner(security: usize) -> (DebugRunner, PermanentHandle) {
    let sec = vec!["PAD"; security];
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .security(0, &sec)
        .memory(5)
        .start();
    let rie = runner.place_on_field(0, "RIE", Some(0));
    runner.game.current_phase = GamePhase::Main;
    (runner, rie)
}

#[test]
fn ex13_064_digivolves_onto_rie_kishibe_for_5_with_3_security() {
    let (mut runner, rie) = rie_runner(3);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[encode_digivolve(0, rie.index as u16) as usize], 1.0);
    let mem0 = runner.game.memory;
    assert!(runner
        .game
        .digivolve_from_hand(0, 0, rie.index as usize, PlaySource::ByHand));
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 0), vec![CARD_ID.to_string()]);
    assert_eq!(mem0 - runner.game.memory, 5);
}

#[test]
fn ex13_064_rie_kishibe_route_closed_with_4_security() {
    let (runner, rie) = rie_runner(4);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[encode_digivolve(0, rie.index as u16) as usize],
        0.0,
        "4 security cards → the [Rie Kishibe] route is closed"
    );
}

#[test]
fn ex13_064_other_tamers_are_not_a_base() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"; 2])
        .memory(5)
        .start();
    let t = runner.place_on_field(0, "OTHER-TAMER", Some(0));
    runner.game.current_phase = GamePhase::Main;
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[encode_digivolve(0, t.index as u16) as usize], 0.0);
}

// ─── Section 2/3 — [When Digivolving] ────────────────────────────────────────

#[test]
fn ex13_064_wd_plays_a_knightmon_text_digimon_from_trash_free() {
    let mut runner = builder().memory(3).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.inject_trash(0, "KNIGHT");
    runner.inject_trash(0, "KNIGHT-9");
    runner.inject_trash(0, "PLAIN");
    let mem0 = runner.memory();
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    assert!(runner.pending_is_optional(), "'you may'");
    assert_eq!(non_pass(&runner).len(), 1, "cost 9 and non-[Knightmon] excluded");
    let a = non_pass(&runner)[0];
    runner.execute_action(0, a).expect("play");
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"KNIGHT".to_string()));
    assert_eq!(runner.memory(), mem0, "free");
}

#[test]
fn ex13_064_wd_uses_a_knightmon_text_option_from_hand_free() {
    let mut runner = builder().hand(0, &["KNIGHT-OPT"]).memory(3).start();
    runner.register_effect("KNIGHT-OPT", Arc::new(OptionMainDraw));
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    let a = non_pass(&runner)[0];
    runner.execute_action(0, a).expect("use");
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), 1, "the Option's body drew 1");
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "KNIGHT-OPT"));
}

#[test]
fn ex13_064_wd_decline_plays_nothing() {
    let mut runner = builder().memory(3).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.inject_trash(0, "KNIGHT");
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 0), vec![CARD_ID.to_string()]);
}

// ─── Section 2/3 — [Your Turn] aura ──────────────────────────────────────────

#[test]
fn ex13_064_your_turn_knightmon_text_digimon_gain_alliance_and_piercing() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let knight = runner.place_on_field(0, "KNIGHT", Some(0));
    let plain = runner.place_on_field(0, "PLAIN", Some(0));
    runner.game.tick_declarative_effects();
    for h in [me, knight] {
        assert!(runner.game.has_keyword(h, Keyword::Alliance), "{h:?} Alliance");
        assert!(runner.game.has_keyword(h, Keyword::Piercing), "{h:?} Piercing");
    }
    assert!(!runner.game.has_keyword(plain, Keyword::Alliance));
    assert!(!runner.game.has_keyword(plain, Keyword::Piercing));
}

#[test]
fn ex13_064_aura_is_off_on_the_opponents_turn() {
    let mut runner = builder().start();
    let knight = runner.place_on_field(0, "KNIGHT", Some(0));
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.set_memory(-3);
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    runner.game.tick_declarative_effects();
    assert!(!runner.game.has_keyword(knight, Keyword::Alliance));
    assert!(!runner.game.has_keyword(knight, Keyword::Piercing));
}

// ─── Section 2/3/5 — other Digimon/Tamer played ──────────────────────────────

#[test]
fn ex13_064_other_play_grants_rush_collision_and_an_attack() {
    let mut runner = builder()
        .hand(0, &["OTHER-TAMER"])
        .security(1, &["PAD"; 3])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    // Knightmon played this turn: without Rush it could not attack.
    let knight = runner.place_on_field(0, "KNIGHT", None);
    let slot = hand_index(&runner, 0, "OTHER-TAMER");
    runner.play(0, slot).expect("tamer plays");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(runner.pending_is_optional(), "'may'");
    runner
        .execute_action(0, ATTACK_START + knight.index as u16)
        .expect("choose Knightmon");
    assert!(runner.game.has_keyword(knight, Keyword::Rush));
    assert!(runner.game.has_keyword(knight, Keyword::Collision));
    let sec0 = runner.security_count(1);
    let _ = runner.auto_resolve();
    assert!(
        runner.game.players[0].battle_area[knight.index as usize].is_suspended,
        "it attacked"
    );
    assert!(runner.security_count(1) < sec0);
}

#[test]
fn ex13_064_non_knightmon_digimon_is_not_eligible() {
    let mut runner = builder().hand(0, &["OTHER-TAMER"]).memory(10).start();
    runner.skip_mulligan();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let plain = runner.place_on_field(0, "PLAIN", Some(0));
    let slot = hand_index(&runner, 0, "OTHER-TAMER");
    runner.play(0, slot).expect("tamer plays");
    let v = runner.pending_selection_view().expect("prompt");
    assert!(!v.valid_action_ids.contains(&(ATTACK_START + plain.index as u16)));
    assert!(v.valid_action_ids.contains(&(ATTACK_START + me.index as u16)));
}

#[test]
fn ex13_064_own_play_does_not_trigger_itself() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(12).start();
    runner.skip_mulligan();
    runner.place_on_field(0, "KNIGHT", Some(0));
    runner.play(0, 0).expect("plays");
    assert!(runner.pending_selection().is_none(), "'other' Digimon or Tamers only");
}

#[test]
fn ex13_064_played_trigger_is_once_per_turn() {
    let mut runner = builder()
        .hand(0, &["OTHER-TAMER", "PLAIN"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    let slot = hand_index(&runner, 0, "OTHER-TAMER");
    runner.play(0, slot).expect("tamer plays");
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    let slot = hand_index(&runner, 0, "PLAIN");
    runner.play(0, slot).expect("plain plays");
    assert!(runner.pending_selection().is_none(), "OPT spent");
}
