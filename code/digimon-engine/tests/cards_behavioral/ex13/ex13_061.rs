//! EX13-061 Gankoomon — Digimon, Lv.6, Black/White, DP 13000, Cost 13.
//! Traits: Holy Warrior / Royal Knight. Attribute: Data.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-061.md`)
//!
//! ＜Reboot＞ ＜Blocker＞
//! [On Play] [When Digivolving] You may play 1 [Hinukamuy] Token.
//! (Digimon/White/6000 DP/＜Alliance＞ ＜Reboot＞ ＜Blocker＞) Then, until your
//! opponent's turn ends, their Digimon effects don't affect 1 of your white
//! Digimon.
//! [All Turns] [Once Per Turn] When any of your white Digimon suspend, you may
//! use 1 use cost 5 or lower Option card with [Huckmon] in its text from your
//! hand or this Digimon's digivolution cards without paying the cost.
//! Assembly -5: 3 [Huckmon] text Digimon cards w/different names
//! Digivolve: Black Lv.5 / cost 5; Lv.5 w/[Huckmon] in text: cost 4.
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_061.cs); printed text governs.
//!
//! # Patterns this test covers
//! - H7 Reboot / H5 Blocker.
//! - Mid-clause optional token play (explicit yes/no) + mandatory F6 immunity
//!   grant (opponent Digimon effects) on a chosen own white Digimon.
//! - Suspend observer over own white Digimon → optional union-zone free
//!   Option use, OPT refunded on decline.
//! - Assembly with 3 distinct-name "[Huckmon] text" materials.

#![allow(dead_code, unused_imports)]

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::action::space::{ATTACK_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};
use digimon_engine::{CardEffect, Effect};

const CARD_ID: &str = "EX13-061";

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

fn digimon(id: &str, name: &str, level: u8, color: CardColor, text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = level as u16;
    c.colors = vec![color];
    c.effect_text = text.to_string();
    c
}

fn option(id: &str, cost: u16, text: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![CardColor::White];
    c.effect_text = text.to_string();
    c
}

const HUCK_TEXT: &str = "[On Play] ... [Huckmon] ...";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-061 YAML loads")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(option("HUCK-OPT", 5, "[Main] ... [Huckmon] ..."))
        .add_card(option("PLAIN-OPT", 1, "[Main] Draw 1."))
        .add_card(digimon("WHITE", "White Ally", 4, CardColor::White, ""))
        .add_card(digimon("RED", "Red Ally", 4, CardColor::Red, ""))
        .add_card(digimon("HA", "Huck A", 3, CardColor::White, HUCK_TEXT))
        .add_card(digimon("HB", "Huck B", 4, CardColor::White, HUCK_TEXT))
        .add_card(digimon("HC", "Huck C", 5, CardColor::White, HUCK_TEXT))
        .add_card(digimon("HA2", "Huck A", 4, CardColor::White, HUCK_TEXT))
        .add_card(digimon("OPP", "Opp", 4, CardColor::Purple, ""))
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

fn trash_index(runner: &DebugRunner, player: u8, card_id: &str) -> u16 {
    runner.game.players[player as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in trash")) as u16
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn ex13_061_metadata_and_alt_paths() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(6), Some(13000), Some(13)));
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2);
    let a = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly");
    assert_eq!(a.cost, Some(CompiledCost::Literal(5)));
    assert_eq!(a.materials.len(), 1, "one repeated, name-distinct filter entry");
}

#[test]
fn ex13_061_keywords_and_clauses() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(runner.game.has_keyword(me, Keyword::Reboot));
    assert!(runner.game.has_keyword(me, Keyword::Blocker));
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
    let opwd = t
        .iter()
        .find(|x| x.when == vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving])
        .expect("OP/WD");
    assert!(!opwd.optional, "the immunity half is mandatory");
    let susp = t.iter().find(|x| x.when == vec![CompiledTiming::OnSuspend]).expect("suspend");
    assert!(susp.once_per_turn);
}

// ─── Section 2/3 — OP/WD token + immunity ────────────────────────────────────

#[test]
fn ex13_061_on_play_token_then_immunity_on_a_chosen_white_digimon() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(13).start();
    runner.skip_mulligan();
    let white = runner.place_on_field(0, "WHITE", Some(0));
    runner.play(0, 0).expect("plays");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice), "'you may play' the token");
    runner.execute_branch(0).expect("play token");
    assert!(field_ids(&runner, 0).contains(&"TOKEN_HINUKAMUY".to_string()));
    let view = runner.pending_selection_view().expect("immunity target prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(!view.is_optional, "the immunity target is mandatory");
    runner
        .execute_action(0, ATTACK_START + white.index as u16)
        .expect("shield WHITE");
    let _ = runner.auto_resolve();
    assert!(runner
        .game
        .permanent_is_unaffected_by_effect(white, 1, EffectSourceKind::Digimon));
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(white, 1, EffectSourceKind::Option),
        "only their DIGIMON effects"
    );
}

#[test]
fn ex13_061_declining_the_token_still_grants_immunity_and_red_is_not_eligible() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let red = runner.place_on_field(0, "RED", Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    runner.execute_branch(1).expect("decline token");
    assert!(!field_ids(&runner, 0).contains(&"TOKEN_HINUKAMUY".to_string()));
    let view = runner.pending_selection_view().expect("immunity target prompt");
    assert!(
        !view.valid_action_ids.contains(&(ATTACK_START + red.index as u16)),
        "a red Digimon is not white"
    );
    runner
        .execute_action(0, ATTACK_START + me.index as u16)
        .expect("shield Gankoomon (white)");
    let _ = runner.auto_resolve();
    assert!(runner
        .game
        .permanent_is_unaffected_by_effect(me, 1, EffectSourceKind::Digimon));
    assert!(!runner
        .game
        .permanent_is_unaffected_by_effect(red, 1, EffectSourceKind::Digimon));
}

// ─── Section 2/3/5 — white Digimon suspends → use Option ─────────────────────

fn suspend_runner(hand: &[&str]) -> (DebugRunner, PermanentHandle, PermanentHandle, PermanentHandle) {
    let mut runner = builder().hand(0, hand).memory(3).start();
    runner.register_effect("HUCK-OPT", Arc::new(OptionMainDraw));
    runner.register_effect("PLAIN-OPT", Arc::new(OptionMainDraw));
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let white = runner.place_on_field(0, "WHITE", Some(0));
    let red = runner.place_on_field(0, "RED", Some(0));
    (runner, me, white, red)
}

#[test]
fn ex13_061_white_digimon_suspending_offers_a_free_huckmon_option() {
    let (mut runner, _me, white, _red) = suspend_runner(&["HUCK-OPT", "PLAIN-OPT"]);
    let mem0 = runner.memory();
    runner.game.suspend(white);
    assert!(matches!(runner.pending_kind(), Some(SelectionKind::UnionZone { .. })));
    assert!(runner.pending_is_optional());
    assert_eq!(non_pass(&runner).len(), 1, "PLAIN-OPT has no [Huckmon] text");
    let a = non_pass(&runner)[0];
    runner.execute_action(0, a).expect("use");
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), mem0);
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "HUCK-OPT"));
}

#[test]
fn ex13_061_gankoomon_itself_suspending_also_triggers() {
    let (mut runner, me, _white, _red) = suspend_runner(&["HUCK-OPT"]);
    runner.game.suspend(me);
    assert!(matches!(runner.pending_kind(), Some(SelectionKind::UnionZone { .. })));
}

#[test]
fn ex13_061_non_white_digimon_suspending_does_not_trigger() {
    let (mut runner, _me, _white, red) = suspend_runner(&["HUCK-OPT"]);
    runner.game.suspend(red);
    assert!(runner.pending_selection().is_none());
}

#[test]
fn ex13_061_opponent_white_digimon_suspending_does_not_trigger() {
    let (mut runner, _me, _white, _red) = suspend_runner(&["HUCK-OPT"]);
    let opp = runner.place_on_field(1, "WHITE", Some(0));
    runner.game.suspend(opp);
    assert!(runner.pending_selection().is_none());
}

#[test]
fn ex13_061_decline_keeps_the_opt_and_use_spends_it() {
    let (mut runner, me, white, _red) = suspend_runner(&["HUCK-OPT", "HUCK-OPT"]);
    runner.game.suspend(white);
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    runner.game.suspend(me);
    assert!(runner.pending_selection().is_some(), "declining did not spend the OPT");
    let a = non_pass(&runner)[0];
    runner.execute_action(0, a).expect("use");
    let _ = runner.auto_resolve();
    runner.game.unsuspend(white);
    runner.game.suspend(white);
    assert!(runner.pending_selection().is_none(), "OPT spent");
}

// ─── Section 3 — Assembly ────────────────────────────────────────────────────

#[test]
fn ex13_061_assembly_three_differently_named_huckmon_text_cards_for_8() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(8).start();
    runner.skip_mulligan();
    for id in ["HA", "HB", "HC"] {
        runner.inject_trash(0, id);
    }
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    for id in ["HA", "HB", "HC"] {
        assert!(runner.game.pending_selection.is_some(), "Assembly pick {id}");
        let idx = trash_index(&runner, 0, id);
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    // [On Play]: decline the token; shield Gankoomon.
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 4);
    assert_eq!(mem0 - runner.game.memory, 8, "13 − 5 = 8");
}

#[test]
fn ex13_061_assembly_unavailable_with_only_two_distinct_names() {
    // Huck A ×2 + Huck B: only 2 distinct names → no legal Assembly, so the
    // play proceeds at full cost with no materials (G-ASSEMBLY-NO-DISTINCT-BY).
    let mut runner = builder().hand(0, &[CARD_ID]).memory(10).start();
    runner.skip_mulligan();
    for id in ["HA", "HA2", "HB"] {
        runner.inject_trash(0, id);
    }
    runner.game.decode_action(PLAY_HAND_START, 0);
    assert_ne!(
        runner.pending_selection_view().map(|v| v.valid_action_ids.iter().any(|&a| a >= TRASH_EFFECT_START && a < TRASH_EFFECT_START + 45)),
        Some(true),
        "no Assembly trash pick without 3 distinct names"
    );
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 1);
    assert_eq!(runner.trash_size(0), 3);
}

#[test]
fn ex13_061_assembly_masks_a_second_card_with_an_already_chosen_name() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(8).start();
    runner.skip_mulligan();
    for id in ["HA", "HA2", "HB", "HC"] {
        runner.inject_trash(0, id);
    }
    runner.game.decode_action(PLAY_HAND_START, 0);
    let ha = trash_index(&runner, 0, "HA");
    let ha2 = trash_index(&runner, 0, "HA2");
    assert!(non_pass(&runner).contains(&(TRASH_EFFECT_START + ha2)));
    runner.game.decode_action(TRASH_EFFECT_START + ha, 0);
    assert!(
        !non_pass(&runner).contains(&(TRASH_EFFECT_START + ha2)),
        "a second [Huck A] is masked once one is chosen"
    );
}
