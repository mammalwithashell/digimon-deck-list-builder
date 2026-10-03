//! EX13-036 Kentaurosmon — Digimon, Lv.6, Yellow, DP 12000, Cost 12.
//! Traits: Holy Warrior / Royal Knight / DATA SQUAD. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-036.md`)
//!
//! [Security] [On Play] 1 of your opponent's Digimon gets -7000 DP for the
//! turn. If there are 6 or fewer total cards in both players' security
//! stacks, instead all of their Digimon get -7000 DP for the turn.
//! [When Digivolving] By trashing the top security card of 1 player with the
//! most security cards, you may activate 1 of this Digimon's [Security]
//! effects.
//! [When Digivolving] [End of Attack] [Counter] [Once Per Turn] You may place
//! 1 of each player's Digimon as the top security cards.
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all yellow w/[Holy Beast] trait
//! Digivolve: Yellow Lv.5 / cost 3; Lv.5 w/[Holy Beast]/[DATA SQUAD] trait: 3.
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_036.cs); printed text governs.
//!
//! # Patterns this test covers
//! - "instead" branch replacement gated on total security count.
//! - Most-security trash cost (explicit player choice) → re-run of the
//!   [Security] body.
//! - Optional OPT place-on-security of 1 Digimon per player (top, face down).
//! - Assembly with colour + trait per-level materials.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::action::space::{ATTACK_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-036";

fn digimon(id: &str, level: u8, dp: i32, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-036 YAML loads")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(digimon("OPP-A", 6, 12000, CardColor::Red, &[]))
        .add_card(digimon("OPP-B", 6, 12000, CardColor::Red, &[]))
        .add_card(digimon("ALLY", 4, 5000, CardColor::Yellow, &[]))
        .add_card(digimon("Y5", 5, 7000, CardColor::Yellow, &["Holy Beast"]))
        .add_card(digimon("Y4", 4, 5000, CardColor::Yellow, &["Holy Beast"]))
        .add_card(digimon("Y3", 3, 3000, CardColor::Yellow, &["Holy Beast"]))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, perm: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

fn pick_field(runner: &mut DebugRunner, target: PermanentHandle) {
    let view = runner.pending_selection_view().expect("field prompt");
    let id = ATTACK_START + target.index as u16;
    assert!(view.valid_action_ids.contains(&id), "{target:?} not selectable: {view:?}");
    runner.execute_action(view.selecting_player, id).expect("pick");
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Decline every remaining prompt (e.g. the optional place-on-security
/// clause that the same [When Digivolving] also queues).
fn decline_rest(runner: &mut DebugRunner) {
    while let Some(v) = runner.pending_selection_view() {
        if !v.is_optional {
            break;
        }
        runner.execute_action(v.selecting_player, PASS).unwrap();
    }
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
fn ex13_036_metadata_and_alt_paths() {
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
    assert_eq!(a.materials.len(), 3);
}

#[test]
fn ex13_036_clause_shapes() {
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
    assert_eq!(t.len(), 3);
    assert!(t
        .iter()
        .any(|x| x.when == vec![CompiledTiming::OnSecurity, CompiledTiming::OnPlay]));
    let place = t
        .iter()
        .find(|x| x.when.contains(&CompiledTiming::EndOfAttack))
        .expect("place clause");
    assert_eq!(
        place.when,
        vec![
            CompiledTiming::WhenDigivolving,
            CompiledTiming::EndOfAttack,
            CompiledTiming::Counter
        ]
    );
    assert!(place.once_per_turn && place.optional);
}

// ─── Section 2/3 — [Security][On Play] ───────────────────────────────────────

#[test]
fn ex13_036_on_play_with_more_than_6_security_debuffs_one() {
    // Kentaurosmon costs 12 (> the 10-memory cap), so drive [On Play] on a
    // placed copy rather than letting the play pass the turn and expire the
    // "for the turn" debuff.
    let mut runner = builder()
        .security(0, &["PAD"; 4])
        .security(1, &["PAD"; 3])
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    fire(&mut runner, EffectTiming::OnPlay, me);
    pick_field(&mut runner, a);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(a), Some(5000));
    assert_eq!(runner.effective_dp(b), Some(12000));
}

#[test]
fn ex13_036_on_play_with_6_or_fewer_security_instead_debuffs_all() {
    let mut runner = builder()
        .security(0, &["PAD"; 3])
        .security(1, &["PAD"; 3])
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    fire(&mut runner, EffectTiming::OnPlay, me);
    assert!(
        runner.pending_selection().is_none(),
        "'instead' — no single-target pick"
    );
    assert_eq!(runner.effective_dp(a), Some(5000));
    assert_eq!(runner.effective_dp(b), Some(5000));
}

#[test]
fn ex13_036_debuff_lasts_for_the_turn_only() {
    let mut runner = builder().security(0, &["PAD"; 2]).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    fire(&mut runner, EffectTiming::OnPlay, me);
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 0);
    assert_eq!(runner.effective_dp(a), Some(5000));
    runner.game.set_memory(-3);
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(a), Some(12000));
}

// ─── Section 2/3 — [When Digivolving] trash most-security → re-run ──────────

#[test]
fn ex13_036_wd_trashing_the_opponents_top_security_activates_the_security_effect() {
    let mut runner = builder()
        .security(0, &["PAD"; 3])
        .security(1, &["PAD"; 5])
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    // Two WD clauses may queue a TriggerOrder; drive to the 3-way choice.
    for _ in 0..4 {
        match runner.pending_kind() {
            Some(SelectionKind::EffectChoice) => break,
            Some(_) => {
                let v = runner.pending_selection_view().unwrap();
                let first = v.valid_action_ids[0];
                runner.execute_action(v.selecting_player, first).unwrap();
            }
            None => break,
        }
    }
    let labels = runner
        .pending_selection_view()
        .and_then(|v| v.effect_choices)
        .expect("3-way trash choice");
    assert_eq!(labels.len(), 3);
    runner.execute_branch(1).expect("trash opponent's (5 > 3)");
    assert_eq!(runner.security_count(1), 4);
    // Total 7 → single-target branch.
    pick_field(&mut runner, a);
    decline_rest(&mut runner);
    assert_eq!(runner.effective_dp(a), Some(5000));
}

#[test]
fn ex13_036_wd_choosing_the_player_with_fewer_security_does_nothing() {
    let mut runner = builder()
        .security(0, &["PAD"; 2])
        .security(1, &["PAD"; 5])
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    for _ in 0..4 {
        match runner.pending_kind() {
            Some(SelectionKind::EffectChoice) => break,
            Some(_) => {
                let v = runner.pending_selection_view().unwrap();
                let first = v.valid_action_ids[0];
                runner.execute_action(v.selecting_player, first).unwrap();
            }
            None => break,
        }
    }
    runner.execute_branch(0).expect("try to trash own (2 < 5)");
    decline_rest(&mut runner);
    assert_eq!(runner.security_count(0), 2, "not the player with the most");
    assert_eq!(runner.effective_dp(a), Some(12000), "no activation");
}

#[test]
fn ex13_036_wd_declining_trashes_nothing() {
    let mut runner = builder()
        .security(0, &["PAD"; 3])
        .security(1, &["PAD"; 3])
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    for _ in 0..4 {
        match runner.pending_kind() {
            Some(SelectionKind::EffectChoice) => break,
            Some(_) => {
                let v = runner.pending_selection_view().unwrap();
                let first = v.valid_action_ids[0];
                runner.execute_action(v.selecting_player, first).unwrap();
            }
            None => break,
        }
    }
    runner.execute_branch(2).expect("don't trash");
    // The optional place-on-security clause may still prompt: decline it.
    while let Some(v) = runner.pending_selection_view() {
        runner.execute_action(v.selecting_player, PASS).unwrap();
    }
    assert_eq!(runner.security_count(0) + runner.security_count(1), 6);
    assert_eq!(runner.effective_dp(a), Some(12000));
}

// ─── Section 2/3/5 — place 1 of each player's Digimon on security ────────────

#[test]
fn ex13_036_counter_places_one_of_each_players_digimon_on_top_of_security() {
    let mut runner = builder()
        .security(0, &["PAD"; 2])
        .security(1, &["PAD"; 2])
        .start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    fire(&mut runner, EffectTiming::CounterEffect, me);
    assert!(runner.pending_is_optional(), "'you may place'");
    runner.accept_optional_trigger().expect("accept");
    pick_field(&mut runner, ally);
    pick_field(&mut runner, a);
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 0), vec![CARD_ID.to_string()]);
    assert!(field_ids(&runner, 1).is_empty());
    assert_eq!(runner.security_count(0), 3);
    assert_eq!(runner.security_count(1), 3);
    assert_eq!(
        runner.game.players[0].security.last().map(|c| c.card_id(&runner.game.card_data).to_string()),
        Some("ALLY".to_string()),
        "placed as the TOP security card"
    );
    assert_eq!(
        runner.game.players[1].security.last().map(|c| c.card_id(&runner.game.card_data).to_string()),
        Some("OPP-A".to_string())
    );
}

#[test]
fn ex13_036_place_declined_moves_nothing() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    fire(&mut runner, EffectTiming::CounterEffect, me);
    runner.decline_optional_trigger().expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 1);
}

#[test]
fn ex13_036_place_is_once_per_turn() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.place_on_field(1, "OPP-B", Some(0));
    fire(&mut runner, EffectTiming::CounterEffect, me);
    runner.accept_optional_trigger().expect("accept");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 1);
    fire(&mut runner, EffectTiming::CounterEffect, me);
    assert!(runner.pending_selection().is_none(), "OPT spent");
}

// ─── Section 3 — Assembly ────────────────────────────────────────────────────

#[test]
fn ex13_036_assembly_yellow_holy_beast_line_for_7() {
    let mut runner = builder().hand(0, &[CARD_ID]).security(0, &["PAD"; 5]).memory(7).start();
    runner.skip_mulligan();
    for id in ["Y5", "Y4", "Y3"] {
        runner.inject_trash(0, id);
    }
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    for id in ["Y5", "Y4", "Y3"] {
        assert!(runner.game.pending_selection.is_some(), "Assembly element {id}");
        let idx = trash_index(&runner, 0, id);
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 4);
    assert_eq!(mem0 - runner.game.memory, 7);
}
