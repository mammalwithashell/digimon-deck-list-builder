//! EX13-076 Imperialdramon: Paladin Mode — Digimon, Lv.7, White/Blue/Green,
//! DP 16000, Cost 16. Traits: Ancient Holy Warrior. Attribute: Vaccine;
//! (Rule) Has [Free] Attribute.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-076.md`)
//!
//! ＜Piercing＞ ＜Vortex＞ ＜Blocker＞ ＜Evade＞
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may
//! suspend 1 of your opponent's Digimon. Then, you may return all digivolution
//! cards of 1 of their Digimon to the bottom of the deck and have this Digimon
//! battle it. Compare the number of digivolution cards instead of DP in this
//! battle.
//! [All Turns] [Once Per Turn] When this Digimon wins a battle, you may return
//! 1 of your opponent's Digimon to the bottom of the deck. Then, this Digimon
//! may unsuspend.
//! Assembly -8: 6 [Free]/[Royal Knight] trait Digimon cards w/different names.
//! Digivolve: Blue Lv.6 / 6; Green Lv.6 / 6; Lv.6 w/[Free]/[Royal Knight]: 5.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/White/EX13_076.cs (DCGO trashes the
//! stripped sources; the printed text returns them to the deck bottom — the
//! official DB governs printed text).
//!
//! # Patterns this test covers
//! - H Piercing / Vortex / Blocker / Evade printed keywords.
//! - Effect battle comparing digivolution-card counts instead of DP
//!   (G-ENGINE-BATTLE-COMPARE-SOURCE-COUNT, `battle { compare:
//!   digivolution_cards }`) + `return_all_sources_to_deck`.
//! - OPT refund when nothing is done (G-OPT-REFUND-ON-DECLINE).
//! - Win-a-battle trigger → optional bottom-deck + optional self-unsuspend.
//! - Assembly `distinct_by: name` over a trait any-of.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDistinctBy, CompiledRepeat,
    CompiledTiming,
};
use digimon_engine::action::space::{ATTACK_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-076";

fn digimon(id: &str, name: &str, traits: &[&str], dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![CardColor::Blue];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-076 YAML loads")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(digimon("SRC", "Source", &[], 1000))
        .add_card(digimon("OPP-A", "Opp A", &[], 3000))
        .add_card(digimon("OPP-B", "Opp B", &[], 4000))
        .add_card(digimon("OPP-HUGE", "Opp Huge", &[], 30000))
        .add_card(digimon("RK1", "Knight One", &["Royal Knight"], 5000))
        .add_card(digimon("RK2", "Knight Two", &["Royal Knight"], 5000))
        .add_card(digimon("RK3", "Knight Three", &["Royal Knight"], 5000))
        .add_card(digimon("FR4", "Free Four", &["Free"], 5000))
        .add_card(digimon("FR5", "Free Five", &["Free"], 5000))
        .add_card(digimon("FR6", "Free Six", &["Free"], 5000))
        .add_card(digimon("RK1-DUP", "Knight One", &["Royal Knight"], 5000))
        .add_card(digimon("PLAIN", "Plain", &[], 5000))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

fn non_pass(runner: &DebugRunner) -> Vec<u16> {
    runner
        .pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default()
}

fn pick_field(runner: &mut DebugRunner, h: PermanentHandle) {
    let view = runner.pending_selection_view().expect("field prompt");
    let action = ATTACK_START + h.index as u16;
    assert!(view.valid_action_ids.contains(&action), "{h:?} selectable: {view:?}");
    runner.execute_action(view.selecting_player, action).expect("pick field");
}

fn pass(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("prompt to pass");
    assert!(view.is_optional, "PASS legal: {view:?}");
    runner.execute_action(view.selecting_player, PASS).expect("pass");
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, perm: PermanentHandle) {
    runner.game.enqueue_triggered(timing, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

fn field_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn deck_bottom_ids(runner: &DebugRunner, player: usize, n: usize) -> Vec<String> {
    runner.game.players[player]
        .deck
        .iter()
        .take(n)
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn trash_index(runner: &DebugRunner, player: usize, card_id: &str) -> u16 {
    runner.game.players[player]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in trash")) as u16
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn ex13_076_metadata_alt_paths_and_name_distinct_assembly() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(7), Some(16000), Some(16)));
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 3, "Blue Lv.6 / Green Lv.6 / Lv.6 Free-or-Royal-Knight");
    let costs: Vec<_> = digi.iter().map(|p| p.cost.clone()).collect();
    assert_eq!(
        costs,
        vec![
            Some(CompiledCost::Literal(6)),
            Some(CompiledCost::Literal(6)),
            Some(CompiledCost::Literal(5))
        ]
    );
    let asm = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly -8");
    assert_eq!(asm.cost, Some(CompiledCost::Literal(8)));
    let m = &asm.materials[0];
    assert_eq!(m.distinct_by, Some(CompiledDistinctBy::Name));
    assert_eq!(m.repeat, Some(CompiledRepeat::Range { min: 6, max: 6 }));
    assert!(m.stack_under);
}

#[test]
fn ex13_076_carrier_keywords() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    for k in [Keyword::Piercing, Keyword::Vortex, Keyword::Blocker, Keyword::Evade] {
        assert!(runner.game.has_keyword(me, k), "{k:?}");
    }
}

#[test]
fn ex13_076_clause_shapes() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let trig: Vec<_> = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 2);
    let shared = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OP/WD/WA clause");
    assert!(shared.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(shared.when.contains(&CompiledTiming::WhenAttacking));
    assert!(shared.once_per_turn);
    let win = trig
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnAllyWonBattle])
        .expect("win-battle clause");
    assert!(win.once_per_turn);
}

// ─── Section 2 — [OP][WD][WA] suspend + digivolution-card battle ──────────────

/// Paladin (with `own_sources` digivolution cards) vs opponent Digimon.
fn board(own_sources: usize) -> (DebugRunner, PermanentHandle, PermanentHandle, PermanentHandle) {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    for _ in 0..own_sources {
        runner.push_source(me, "SRC");
    }
    let a = runner.place_stack(1, &["SRC", "SRC", "OPP-HUGE"]);
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    (runner, me, a, b)
}

#[test]
fn ex13_076_on_play_suspends_then_strips_and_wins_on_digivolution_cards() {
    let (mut runner, me, huge, b) = board(1);
    fire(&mut runner, EffectTiming::OnPlay, me);

    // 1) "You may suspend 1 of your opponent's Digimon."
    let v = runner.pending_selection_view().expect("suspend prompt");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert!(v.is_optional, "'you may suspend'");
    pick_field(&mut runner, b);
    assert!(suspended(&runner, b), "OPP-B suspended");

    // 2) "you may return all digivolution cards of 1 of their Digimon …"
    let v = runner.pending_selection_view().expect("strip/battle prompt");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert!(v.is_optional);
    let opp_deck0 = runner.deck_size(1);
    pick_field(&mut runner, huge);
    // Its 2 sources went to the BOTTOM of the opponent's deck, then the battle
    // compared 1 (Paladin) vs 0 digivolution cards: 30000 DP is irrelevant.
    assert_eq!(runner.deck_size(1), opp_deck0 + 2, "both sources returned to deck");
    assert_eq!(deck_bottom_ids(&runner, 1, 2), vec!["SRC", "SRC"], "to the bottom");
    assert!(
        !field_ids(&runner, 1).contains(&"OPP-HUGE".to_string()),
        "OPP-HUGE lost the digivolution-card battle"
    );
    assert!(field_ids(&runner, 0).contains(&CARD_ID.to_string()), "Paladin survives");
}

#[test]
fn ex13_076_suspend_prompt_only_offers_unsuspended_digimon() {
    let (mut runner, me, huge, b) = board(1);
    runner.game.players[1].battle_area[b.index as usize].is_suspended = true;
    fire(&mut runner, EffectTiming::OnPlay, me);
    let v = runner.pending_selection_view().expect("suspend prompt");
    assert!(v.valid_action_ids.contains(&(ATTACK_START + huge.index as u16)));
    assert!(
        !v.valid_action_ids.contains(&(ATTACK_START + b.index as u16)),
        "an already-suspended Digimon is not a suspend target"
    );
}

#[test]
fn ex13_076_equal_digivolution_cards_delete_both() {
    // Paladin has 0 digivolution cards; the stripped target has 0 → tie.
    let (mut runner, me, huge, _b) = board(0);
    fire(&mut runner, EffectTiming::OnPlay, me);
    pass(&mut runner); // no suspend
    pick_field(&mut runner, huge);
    // Tie → both would be deleted; Paladin's <Evade> offers to suspend it
    // instead. Decline it.
    let v = runner.pending_selection_view().expect("<Evade> replacement prompt");
    assert_eq!(v.kind, SelectionKind::Replacement, "{v:?}");
    runner.execute_action(v.selecting_player, PASS).expect("decline <Evade>");
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 1).contains(&"OPP-HUGE".to_string()));
    assert!(
        !field_ids(&runner, 0).contains(&CARD_ID.to_string()),
        "0 vs 0 digivolution cards: both deleted (16000 DP irrelevant)"
    );
}

#[test]
fn ex13_076_tie_with_evade_accepted_still_deletes_the_target() {
    let (mut runner, me, huge, _b) = board(0);
    fire(&mut runner, EffectTiming::OnPlay, me);
    pass(&mut runner);
    pick_field(&mut runner, huge);
    let v = runner.pending_selection_view().expect("<Evade> replacement prompt");
    assert_eq!(v.kind, SelectionKind::Replacement, "{v:?}");
    let accept = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    runner.execute_action(v.selecting_player, accept).expect("accept <Evade>");
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&CARD_ID.to_string()), "<Evade> saved Paladin");
    assert!(suspended(&runner, me));
    assert!(!field_ids(&runner, 1).contains(&"OPP-HUGE".to_string()), "the tie still deletes the target");
}

#[test]
fn ex13_076_strip_battle_works_from_when_digivolving_and_when_attacking() {
    for timing in [EffectTiming::WhenDigivolving, EffectTiming::WhenAttacking] {
        let (mut runner, me, huge, _b) = board(2);
        fire(&mut runner, timing, me);
        pass(&mut runner);
        pick_field(&mut runner, huge);
            assert!(
            !field_ids(&runner, 1).contains(&"OPP-HUGE".to_string()),
            "{timing:?}: digivolution-card battle won"
        );
    }
}

#[test]
fn ex13_076_declining_both_keeps_the_once_per_turn() {
    let (mut runner, me, _huge, _b) = board(1);
    fire(&mut runner, EffectTiming::OnPlay, me);
    pass(&mut runner);
    pass(&mut runner);
    assert!(runner.pending_selection_view().is_none());
    assert_eq!(runner.battle_area_size(1), 2, "nothing happened");
    fire(&mut runner, EffectTiming::WhenAttacking, me);
    assert!(
        runner.pending_selection_view().is_some(),
        "declining everything did not spend the [Once Per Turn]"
    );
}

#[test]
fn ex13_076_using_it_spends_the_shared_once_per_turn() {
    let (mut runner, me, _huge, b) = board(1);
    fire(&mut runner, EffectTiming::OnPlay, me);
    pick_field(&mut runner, b); // suspend only
    pass(&mut runner);
    assert!(runner.pending_selection_view().is_none());
    fire(&mut runner, EffectTiming::WhenAttacking, me);
    assert!(
        runner.pending_selection_view().is_none(),
        "OP / WD / WA share one [Once Per Turn]"
    );
}

#[test]
fn ex13_076_no_opponent_digimon_means_no_activation() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    fire(&mut runner, EffectTiming::OnPlay, me);
    assert!(runner.pending_selection_view().is_none());
}

// ─── Section 3 — [All Turns][OPT] win a battle ───────────────────────────────

#[test]
fn ex13_076_winning_a_battle_bottom_decks_and_may_unsuspend() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[me.index as usize].is_suspended = true;
    let weak = runner.place_on_field(1, "OPP-A", Some(0));
    let other = runner.place_on_field(1, "OPP-B", Some(0));
    let _ = runner.battle_digimon(me, weak); // 16000 vs 3000 DP
    assert!(!field_ids(&runner, 1).contains(&"OPP-A".to_string()), "battle won");

    let v = runner.pending_selection_view().expect("bottom-deck prompt");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert!(v.is_optional, "'you may return'");
    // OPP-B shifted to index 0 after OPP-A's deletion.
    let other_now = PermanentHandle { player: 1, index: 0 };
    let _ = other;
    pick_field(&mut runner, other_now);
    assert_eq!(runner.battle_area_size(1), 0, "OPP-B returned to the deck");
    assert_eq!(deck_bottom_ids(&runner, 1, 1), vec!["OPP-B"]);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice), "may unsuspend");
    runner.execute_branch(0).expect("unsuspend");
    assert!(!suspended(&runner, me), "Paladin unsuspended");
}

#[test]
fn ex13_076_unsuspended_winner_is_not_offered_the_unsuspend() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let weak = runner.place_on_field(1, "OPP-A", Some(0));
    let _ = runner.battle_digimon(me, weak);
    // No opponent Digimon left and not suspended → nothing to do.
    assert!(runner.pending_selection_view().is_none());
}

#[test]
fn ex13_076_win_trigger_decline_keeps_the_opt_and_use_spends_it() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    for _ in 0..3 {
        runner.place_on_field(1, "OPP-A", Some(0));
    }
    let first = PermanentHandle { player: 1, index: 0 };
    let _ = runner.battle_digimon(me, first);
    pass(&mut runner); // decline the bottom-deck (not suspended → no unsuspend)
    assert!(runner.pending_selection_view().is_none());

    let second = PermanentHandle { player: 1, index: 0 };
    let _ = runner.battle_digimon(me, second);
    let v = runner.pending_selection_view().expect("OPT kept after a full decline");
    assert_eq!(v.kind, SelectionKind::OppField);
    pick_field(&mut runner, PermanentHandle { player: 1, index: 0 });
    assert!(runner.pending_selection_view().is_none());

    runner.place_on_field(1, "OPP-A", Some(0));
    let third = PermanentHandle { player: 1, index: 0 };
    let _ = runner.battle_digimon(me, third);
    assert!(runner.pending_selection_view().is_none(), "OPT spent");
}

#[test]
fn ex13_076_losing_a_battle_does_not_trigger() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let huge = runner.place_on_field(1, "OPP-HUGE", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    let _ = runner.battle_digimon(me, huge);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 2, "no bottom-deck after a loss");
}

// ─── Section 4 — Assembly ────────────────────────────────────────────────────

const SIX: [&str; 6] = ["RK1", "RK2", "RK3", "FR4", "FR5", "FR6"];

#[test]
fn ex13_076_assembly_six_free_or_royal_knight_names_for_8() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(8).start();
    runner.skip_mulligan();
    for id in SIX {
        runner.inject_trash(0, id);
    }
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    for id in SIX {
        let idx = trash_index(&runner, 0, id);
        assert!(non_pass(&runner).contains(&(TRASH_EFFECT_START + idx)), "pick {id}");
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 7);
    assert_eq!(mem0 - runner.game.memory, 8, "16 − 8");
}

#[test]
fn ex13_076_assembly_excludes_duplicate_names_and_untraited_cards() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(8).start();
    runner.skip_mulligan();
    for id in ["RK1", "RK1-DUP", "PLAIN", "RK2", "RK3", "FR4", "FR5", "FR6"] {
        runner.inject_trash(0, id);
    }
    runner.game.decode_action(PLAY_HAND_START, 0);
    let rk1 = trash_index(&runner, 0, "RK1");
    let plain = trash_index(&runner, 0, "PLAIN");
    assert!(
        !non_pass(&runner).contains(&(TRASH_EFFECT_START + plain)),
        "a card without [Free]/[Royal Knight] is not a material"
    );
    runner.game.decode_action(TRASH_EFFECT_START + rk1, 0);
    let dup = trash_index(&runner, 0, "RK1-DUP");
    assert!(
        !non_pass(&runner).contains(&(TRASH_EFFECT_START + dup)),
        "a second [Knight One] is excluded (different names)"
    );
}
