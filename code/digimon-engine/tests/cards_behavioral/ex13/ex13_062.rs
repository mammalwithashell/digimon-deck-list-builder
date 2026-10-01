//! EX13-062 Craniamon — Digimon, Lv.6, Black, DP 12000, Cost 12.
//! Traits: Holy Warrior / Royal Knight. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-062.md`)
//!
//! ＜Reboot＞ ＜Blocker＞
//! [On Play] [When Digivolving] Your opponent's effects don't affect this
//! Digimon until their turn ends.
//! [All Turns] [Once Per Turn] When this Digimon suspends, you may delete all
//! of your opponent's Digimon with the lowest play cost.
//! [All Turns] When this Digimon unsuspends, it gets +3000 DP until your turn
//! ends.
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all black w/＜Blocker＞
//! Digivolve: Black Lv.5 / cost 3.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Black/EX13_062.cs
//!
//! # Patterns this test covers
//! - H5 Blocker / H7 Reboot printed keywords.
//! - F6 effect immunity (opponent's effects, 4 source kinds).
//! - Self-suspend trigger → optional batched delete-all over a play-cost
//!   aggregate (OPT).
//! - Self-unsuspend trigger → DP buff until your turn ends (effect-driven
//!   unsuspension; phase / Reboot unsuspension is the logged engine gap
//!   G-ENGINE-PHASE-UNSUSPEND-NO-ONUNSUSPEND).
//! - Assembly with three per-level materials filtered on a face-up printed
//!   keyword (G-DSL-PREDICATE-PRINTED-KEYWORD).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming, CompiledZone,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-062";

fn digimon(id: &str, level: u8, cost: u16, color: CardColor, text: &str, inherited: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c.colors = vec![color];
    c.effect_text = text.to_string();
    c.inherited_text = inherited.to_string();
    c.keywords = digimon_engine::card_data::parse_printed_keywords(text, inherited, "");
    c
}

const BLOCKER: &str = "＜Blocker＞ (This Digimon can block in the blocker timing.)";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-062 YAML loads")
        .add_card(digimon("B5", 5, 7, CardColor::Black, BLOCKER, ""))
        .add_card(digimon("B4", 4, 5, CardColor::Black, BLOCKER, ""))
        .add_card(digimon("B3", 3, 3, CardColor::Black, BLOCKER, ""))
        .add_card(digimon("B4-INH", 4, 5, CardColor::Black, "", BLOCKER))
        .add_card(digimon("R4-BLK", 4, 5, CardColor::Red, BLOCKER, ""))
        .add_card(digimon("OPP-C3A", 3, 3, CardColor::Red, "", ""))
        .add_card(digimon("OPP-C3B", 3, 3, CardColor::Red, "", ""))
        .add_card(digimon("OPP-C5", 4, 5, CardColor::Red, "", ""))
        .add_card(digimon("FILL", 3, 3, CardColor::Red, "", ""))
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn stack_ids(runner: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
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
fn ex13_062_metadata_keywords_and_alt_paths() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.cost, Some(12));
    let circles: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(circles.len(), 1);
    assert_eq!(circles[0].cost, Some(CompiledCost::Literal(3)));
    let assembly = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly -5");
    assert_eq!(assembly.cost, Some(CompiledCost::Literal(5)));
    assert_eq!(assembly.materials.len(), 3, "Lv.5 × Lv.4 × Lv.3");
    for m in &assembly.materials {
        assert!(m.stack_under);
        assert!(m.zones.contains(&CompiledZone::Trash));
    }
}

#[test]
fn ex13_062_clause_shapes() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 3);
    let opwd = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OP/WD");
    assert_eq!(opwd.when, vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]);
    assert!(!opwd.optional && !opwd.once_per_turn);
    let susp = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSuspend])
        .expect("on suspend");
    assert!(susp.once_per_turn && susp.optional, "[OPT] 'you may delete'");
    let unsusp = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnUnsuspend])
        .expect("on unsuspend");
    assert!(!unsusp.once_per_turn && !unsusp.optional);
}

#[test]
fn ex13_062_carrier_has_reboot_and_blocker() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(runner.game.has_keyword(me, Keyword::Reboot));
    assert!(runner.game.has_keyword(me, Keyword::Blocker));
}

// ─── Section 3 — OP/WD immunity ──────────────────────────────────────────────

#[test]
fn ex13_062_on_play_grants_immunity_to_opponents_effects() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(12).start();
    runner.skip_mulligan();
    let field = runner.play(0, 0).expect("plays");
    let _ = runner.auto_resolve();
    let me = PermanentHandle { player: 0, index: field as u8 };
    for kind in [
        EffectSourceKind::Digimon,
        EffectSourceKind::Tamer,
        EffectSourceKind::Option,
    ] {
        assert!(
            runner.game.permanent_is_unaffected_by_effect(me, 1, kind),
            "opponent {kind:?} effects don't affect it"
        );
    }
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(me, 0, EffectSourceKind::Digimon),
        "your own effects still affect it"
    );
}

#[test]
fn ex13_062_without_on_play_there_is_no_immunity() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(!runner
        .game
        .permanent_is_unaffected_by_effect(me, 1, EffectSourceKind::Digimon));
}

#[test]
fn ex13_062_when_digivolving_grants_the_same_immunity() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    let _ = runner.auto_resolve();
    assert!(runner
        .game
        .permanent_is_unaffected_by_effect(me, 1, EffectSourceKind::Digimon));
}

// ─── Section 2/3/5 — When this suspends: may delete all lowest play cost ────

fn opp_board(runner: &mut DebugRunner) {
    runner.place_on_field(1, "OPP-C3A", Some(0));
    runner.place_on_field(1, "OPP-C5", Some(0));
    runner.place_on_field(1, "OPP-C3B", Some(0));
}

#[test]
fn ex13_062_suspending_may_delete_every_lowest_play_cost_opponent_digimon() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    opp_board(&mut runner);
    runner.game.suspend(me);
    assert!(runner.pending_is_optional(), "'you may delete'");
    runner.accept_optional_trigger().expect("accept");
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 1), vec!["OPP-C5".to_string()], "both cost-3 Digimon go");
}

#[test]
fn ex13_062_declining_deletes_nothing() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    opp_board(&mut runner);
    runner.game.suspend(me);
    runner.decline_optional_trigger().expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 3);
}

#[test]
fn ex13_062_another_digimon_suspending_does_not_trigger() {
    let mut runner = builder().start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "B5", Some(0));
    opp_board(&mut runner);
    runner.game.suspend(other);
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.battle_area_size(1), 3);
}

#[test]
fn ex13_062_suspend_delete_is_once_per_turn() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    opp_board(&mut runner);
    runner.game.suspend(me);
    runner.accept_optional_trigger().expect("accept");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 1);
    runner.place_on_field(1, "OPP-C3A", Some(0));
    runner.game.unsuspend(me);
    let _ = runner.auto_resolve();
    runner.game.suspend(me);
    assert!(runner.pending_selection().is_none(), "OPT used this turn");
    assert_eq!(runner.battle_area_size(1), 2);
}

#[test]
fn ex13_062_suspend_attack_triggers_the_delete() {
    let mut runner = builder()
        .security(1, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    opp_board(&mut runner);
    runner.attack_player(me, 1, false);
    assert!(runner.pending_is_optional(), "suspending to attack fires the trigger");
    runner.accept_optional_trigger().expect("accept");
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 1).iter().any(|c| c.starts_with("OPP-C3")));
}

// ─── Section 3 — When this unsuspends: +3000 until your turn ends ───────────

#[test]
fn ex13_062_effect_unsuspend_gives_3000_until_your_turn_ends() {
    let mut runner = builder()
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[me.index as usize].is_suspended = true;
    runner.game.unsuspend(me);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(me), Some(15000));
    runner.game.set_memory(-3);
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(runner.effective_dp(me), Some(12000), "expires at the end of your turn");
}

#[test]
fn ex13_062_another_digimon_unsuspending_does_not_buff() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "B5", Some(0));
    runner.game.players[0].battle_area[other.index as usize].is_suspended = true;
    runner.game.unsuspend(other);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(me), Some(12000));
}

// ─── Section 3 — Assembly ────────────────────────────────────────────────────

#[test]
fn ex13_062_assembly_takes_one_black_blocker_per_level_for_7() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(7).start();
    runner.skip_mulligan();
    runner.inject_trash(0, "B5");
    runner.inject_trash(0, "B4");
    runner.inject_trash(0, "B3");
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    for id in ["B5", "B4", "B3"] {
        assert!(runner.game.pending_selection.is_some(), "Assembly element {id}");
        let idx = trash_index(&runner, 0, id);
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    let _ = runner.auto_resolve();
    let perm = PermanentHandle { player: 0, index: 0 };
    let stack = stack_ids(&runner, perm);
    assert_eq!(stack.last().map(String::as_str), Some(CARD_ID));
    assert_eq!(stack.len(), 4);
    assert_eq!(mem0 - runner.game.memory, 7, "12 − 5 = 7");
}

#[test]
fn ex13_062_assembly_rejects_inherited_only_blocker_and_non_black_materials() {
    // B4-INH carries <Blocker> only as an inherited effect; R4-BLK is red.
    // Neither satisfies the Lv.4 element, so the Assembly is unavailable: the
    // play proceeds at full cost (12) with no materials stacked.
    let mut runner = builder().hand(0, &[CARD_ID]).memory(10).start();
    runner.skip_mulligan();
    runner.inject_trash(0, "B5");
    runner.inject_trash(0, "B4-INH");
    runner.inject_trash(0, "R4-BLK");
    runner.inject_trash(0, "B3");
    runner.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        runner.pending_selection().is_none(),
        "no Assembly prompt without a legal Lv.4 material"
    );
    let perm = PermanentHandle { player: 0, index: 0 };
    assert_eq!(stack_ids(&runner, perm), vec![CARD_ID.to_string()]);
    assert_eq!(runner.trash_size(0), 4, "no trash card consumed");
}
