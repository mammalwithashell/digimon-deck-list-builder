//! EX13-059 BigMamemon — Digimon, Lv.5, Black, DP 8000, Play Cost 8.
//! Traits: Mutant. Form: Ultimate. Attribute: Data.
//! Digivolve: Black Lv.4 / 3.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-059.md`;
//! per-card JSON `code/digimon-engine/cards/ex13/EX13-059.json`)
//!
//! [When Digivolving] [On Deletion] Reveal the top 3 cards of your deck. You
//! may play 1 play cost 7 or lower Digimon card with [Mamemon] in its name or
//! the [Mutant] trait among them without paying the cost. Trash the rest.
//! [End of Your Turn] [Once Per Turn] By deleting 1 of your Digimon with
//! [Mamemon] in its name, delete 1 of your opponent's Digimon with the lowest
//! play cost.
//! Inherited: [End of Your Turn] [Once Per Turn] By deleting 1 of your Digimon
//! with [Mamemon] in its name, delete all of your opponent's Digimon with the
//! lowest play cost.
//!
//! Official Q&A: "Yes, both have to be XX" — the play-cost-7 cap applies to
//! BOTH the [Mamemon]-name and the [Mutant]-trait alternatives.
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_059.cs). Printed text + general_rule.pdf
//! §15-7 (optional processing conditions: "By X, Y" — the player may decline
//! X, and then Y does not happen) govern.
//!
//! # Pattern rows
//! - A1/A3 reveal-3, optional single free play, trash the rest (EX8-050 idiom)
//! - E-OPT End of Your Turn optional processing condition (self-delete cost)
//! - lowest-play-cost single pick (selector) / delete-all sweep (BT23-058 idiom)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_digivolve, ATTACK_START, PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-059";

fn digimon(id: &str, name: &str, level: u8, cost: u16, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.play_cost = cost;
    c.dp = Some(dp);
    c.colors = vec![CardColor::Black];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-059 YAML loads from the embedded pack")
        .add_card(digimon("MAME-7", "Tiny Mamemon", 4, 7, 5000))
        .add_card(digimon("MAME-8", "Huge Mamemon", 5, 8, 8000))
        .add_card({
            let mut c = digimon("MUT-7", "Mutant Seven", 4, 7, 5000);
            c.traits = vec!["Mutant".to_string()];
            c
        })
        .add_card({
            let mut c = digimon("MUT-8", "Mutant Eight", 5, 8, 8000);
            c.traits = vec!["Mutant".to_string()];
            c
        })
        .add_card({
            let mut c = digimon("TEXT-MAME", "Text Only", 4, 4, 4000);
            c.effect_text = "Digivolve into [Mamemon].".to_string();
            c
        })
        .add_card({
            let mut c = make_test_card("MAME-OPT", "Mamemon Option");
            c.card_kind = CardKind::Option;
            c.level = None;
            c.dp = None;
            c.play_cost = 2;
            c
        })
        .add_card(digimon("PLAIN", "Plain", 4, 4, 4000))
        .add_card(digimon("BLACK-L4", "Black Four", 4, 4, 5000))
        .add_card(digimon("OWN-MAME", "Ally Mamemon", 4, 5, 6000))
        .add_card(digimon("OWN-OTHER", "Ally Other", 4, 5, 6000))
        .add_card(digimon("TOP-MAME6", "PrinceLike Mamemon", 6, 11, 12000))
        .add_card(digimon("OPP-C3", "Opp Cost Three", 3, 3, 3000))
        .add_card(digimon("OPP-C3B", "Opp Cost Three B", 3, 3, 3000))
        .add_card(digimon("OPP-C5", "Opp Cost Five", 4, 5, 5000))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(1, &["FILL"; 10])
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn handle_of(runner: &DebugRunner, player: u8, id: &str) -> PermanentHandle {
    let idx = field_ids(runner, player)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} on player {player}'s field"));
    PermanentHandle { player, index: idx as u8 }
}

fn offered_reveal_ids(runner: &DebugRunner) -> Vec<String> {
    let view = runner.pending_selection_view().expect("reveal prompt pending");
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(SEL_REVEAL_START))
        .filter_map(|i| runner.game.revealed_cards.get(i as usize))
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    ids.sort();
    ids
}

fn pick_revealed(runner: &mut DebugRunner, card_id: &str) {
    let view = runner.pending_selection_view().expect("reveal prompt pending");
    let want = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(SEL_REVEAL_START)
                .and_then(|i| runner.game.revealed_cards.get(i as usize))
                .is_some_and(|c| c.card_id(&runner.game.card_data) == card_id)
        })
        .unwrap_or_else(|| panic!("{card_id} must be a legal pick: {view:?}"));
    runner
        .execute_action(view.selecting_player, want)
        .expect("pick revealed card");
}

fn field_options(runner: &DebugRunner, kind: SelectionKind) -> Vec<u16> {
    let view = runner.pending_selection_view().expect("field prompt pending");
    assert_eq!(view.kind, kind, "{view:?}");
    view.valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .map(|a| a - ATTACK_START)
        .collect()
}

fn pick_field(runner: &mut DebugRunner, h: PermanentHandle) {
    let view = runner.pending_selection_view().expect("field prompt pending");
    let id = ATTACK_START + h.index as u16;
    assert!(view.valid_action_ids.contains(&id), "{h:?} not offered: {view:?}");
    runner.execute_action(view.selecting_player, id).expect("pick field");
}

fn pick_id(runner: &mut DebugRunner, player: u8, id: &str) {
    let h = handle_of(runner, player, id);
    pick_field(runner, h);
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

/// Digivolve BigMamemon onto a Black Lv.4. `top3_bottom_first` are the three
/// cards under the digivolve-draw card (the deck's top card is drawn by the
/// digivolution bonus before [When Digivolving] resolves).
fn digivolve_with_deck(top3_bottom_first: &[&str]) -> DebugRunner {
    let mut deck = vec!["FILL", "FILL"];
    deck.extend_from_slice(top3_bottom_first);
    deck.push("FILL");
    let mut runner = builder().hand(0, &[CARD_ID]).deck(0, &deck).memory(5).start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "BLACK-L4", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    runner
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_059_printed_metadata_and_digivolve_path() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(5), Some(8000), Some(8)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.traits, vec!["Mutant".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 1);
    assert_eq!(digi[0].cost, Some(CompiledCost::Literal(3)));
}

#[test]
fn ex13_059_clause_shapes() {
    let runner = builder().start();
    let t = triggered(&runner);
    assert_eq!(t.len(), 3);
    let reveal = t
        .iter()
        .find(|x| {
            x.when.contains(&CompiledTiming::WhenDigivolving)
                && x.when.contains(&CompiledTiming::OnDeletion)
        })
        .expect("[WD][OD] reveal clause");
    assert!(!reveal.optional, "the reveal/trash is mandatory; only the play is a may");
    assert_eq!(reveal.scope, CompiledScope::FaceUp);

    let eots: Vec<_> = t
        .iter()
        .filter(|x| x.when == vec![CompiledTiming::EndOfYourTurn])
        .collect();
    assert_eq!(eots.len(), 2);
    for e in &eots {
        assert!(e.once_per_turn, "[Once Per Turn]");
        assert!(e.optional, "\"By deleting ...\" is an optional processing condition");
    }
    assert!(eots.iter().any(|e| e.scope == CompiledScope::FaceUp));
    assert!(eots.iter().any(|e| e.scope == CompiledScope::Inherited));
}

// ─── Section 2 — [When Digivolving] reveal ───────────────────────────────────

#[test]
fn ex13_059_digivolves_from_black_lv4_for_3() {
    let runner = digivolve_with_deck(&["PLAIN", "PLAIN", "PLAIN"]);
    assert_eq!(field_ids(&runner, 0), vec![CARD_ID.to_string()]);
    assert_eq!(runner.memory(), 2, "cost 3");
}

#[test]
fn ex13_059_when_digivolving_offers_mamemon_name_or_mutant_at_cost_7_or_less() {
    let runner = digivolve_with_deck(&["MAME-7", "MUT-7", "PLAIN"]);
    assert_eq!(offered_reveal_ids(&runner), vec!["MAME-7".to_string(), "MUT-7".to_string()]);
    assert!(runner.pending_is_optional(), "\"You may play\"");
}

#[test]
fn ex13_059_when_digivolving_excludes_cost_8_text_only_and_non_digimon() {
    let runner = digivolve_with_deck(&["MAME-8", "TEXT-MAME", "MAME-OPT"]);
    if runner.pending_selection().is_some() {
        assert!(
            offered_reveal_ids(&runner).is_empty(),
            "cost 8 / [Mamemon] only in text / Option card are all ineligible"
        );
    }
}

#[test]
fn ex13_059_when_digivolving_excludes_cost_8_mutant() {
    let runner = digivolve_with_deck(&["MUT-8", "PLAIN", "PLAIN"]);
    if runner.pending_selection().is_some() {
        assert!(offered_reveal_ids(&runner).is_empty(), "the cap applies to [Mutant] too");
    }
}

#[test]
fn ex13_059_when_digivolving_plays_the_pick_free_and_trashes_the_rest() {
    let mut runner = digivolve_with_deck(&["PLAIN", "MAME-7", "MUT-8"]);
    let mem0 = runner.memory();
    pick_revealed(&mut runner, "MAME-7");
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"MAME-7".to_string()));
    assert_eq!(runner.memory(), mem0, "without paying the cost");
    let trash = trash_ids(&runner, 0);
    assert!(trash.contains(&"PLAIN".to_string()) && trash.contains(&"MUT-8".to_string()));
}

#[test]
fn ex13_059_when_digivolving_decline_trashes_all_three() {
    let mut runner = digivolve_with_deck(&["PLAIN", "MAME-7", "MUT-8"]);
    let trash0 = runner.trash_size(0);
    runner.execute_action(0, PASS).expect("decline the play");
    let _ = runner.auto_resolve();
    assert_eq!(runner.trash_size(0), trash0 + 3);
    assert!(!field_ids(&runner, 0).contains(&"MAME-7".to_string()));
}

// ─── Section 3 — [On Deletion] reveal ────────────────────────────────────────

#[test]
fn ex13_059_on_deletion_reveals_and_plays() {
    let mut runner = builder()
        .deck(0, &["FILL", "PLAIN", "PLAIN", "MUT-7"])
        .memory(0)
        .start();
    runner.skip_mulligan();
    let big = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .delete_permanent_with_cause(big, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    assert_eq!(offered_reveal_ids(&runner), vec!["MUT-7".to_string()]);
    pick_revealed(&mut runner, "MUT-7");
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 0), vec!["MUT-7".to_string()]);
}

#[test]
fn ex13_059_on_play_does_not_reveal() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "PLAIN", "PLAIN", "MUT-7"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.play(0, 0).expect("BigMamemon plays");
    assert!(runner.pending_selection().is_none(), "no [On Play] effect");
    assert_eq!(runner.deck_size(0), 4);
}

// ─── Section 4 — [End of Your Turn][OPT] (face-up) ───────────────────────────

fn eot_board(extra_own: &[&str], opp: &[&str]) -> DebugRunner {
    let mut runner = builder().deck(0, &["FILL"; 6]).memory(2).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    for id in extra_own {
        runner.place_on_field(0, id, Some(0));
    }
    for id in opp {
        runner.place_on_field(1, id, Some(0));
    }
    runner.pass_turn();
    runner
}

#[test]
fn ex13_059_eot_deleting_a_mamemon_deletes_1_lowest_play_cost_opp_digimon() {
    let mut runner = eot_board(&["OWN-MAME"], &["OPP-C3", "OPP-C3B", "OPP-C5"]);
    runner.accept_optional_trigger().expect("accept the optional processing");
    // Own pick: only [Mamemon]-named Digimon (BigMamemon itself + OWN-MAME).
    let own = field_options(&runner, SelectionKind::OwnField);
    assert_eq!(own.len(), 2);
    pick_id(&mut runner, 0, "OWN-MAME");
    // Opp pick: only the tied lowest play cost Digimon (cost 3) are offered.
    let opp = field_options(&runner, SelectionKind::OppField);
    let c5 = handle_of(&runner, 1, "OPP-C5").index as u16;
    assert_eq!(opp.len(), 2, "both cost-3 Digimon, the player chooses");
    assert!(!opp.contains(&c5));
    pick_id(&mut runner, 1, "OPP-C3B");
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 0).contains(&"OWN-MAME".to_string()));
    assert!(field_ids(&runner, 0).contains(&CARD_ID.to_string()));
    let opp_field = field_ids(&runner, 1);
    assert!(opp_field.contains(&"OPP-C3".to_string()));
    assert!(!opp_field.contains(&"OPP-C3B".to_string()));
    assert!(opp_field.contains(&"OPP-C5".to_string()));
}

#[test]
fn ex13_059_eot_non_mamemon_digimon_cannot_pay_the_cost() {
    let mut runner = eot_board(&["OWN-OTHER"], &["OPP-C3"]);
    runner.accept_optional_trigger().expect("accept");
    let own = field_options(&runner, SelectionKind::OwnField);
    let other = handle_of(&runner, 0, "OWN-OTHER").index as u16;
    assert!(!own.contains(&other), "no [Mamemon] in its name");
}

#[test]
fn ex13_059_eot_bigmamemon_may_delete_itself_as_the_cost() {
    let mut runner = eot_board(&[], &["OPP-C3", "OPP-C5"]);
    runner.accept_optional_trigger().expect("accept");
    pick_id(&mut runner, 0, CARD_ID);
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).is_empty());
    assert_eq!(field_ids(&runner, 1), vec!["OPP-C5".to_string()]);
}

#[test]
fn ex13_059_eot_decline_deletes_nothing() {
    let mut runner = eot_board(&["OWN-MAME"], &["OPP-C3"]);
    runner.decline_optional_trigger().expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 2);
    assert_eq!(runner.battle_area_size(1), 1);
}

#[test]
fn ex13_059_eot_cost_is_payable_without_an_opponent_digimon() {
    // §15-7-4: the player may execute the processing condition regardless of
    // whether the following content can be executed.
    let mut runner = eot_board(&["OWN-MAME"], &[]);
    runner.accept_optional_trigger().expect("accept");
    pick_id(&mut runner, 0, "OWN-MAME");
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 0).contains(&"OWN-MAME".to_string()));
}

#[test]
fn ex13_059_eot_is_once_per_turn_and_resets_next_turn() {
    let mut runner = builder().deck(0, &["FILL"; 6]).memory(2).start();
    runner.skip_mulligan();
    let big = runner.place_on_field(0, CARD_ID, Some(0));
    for id in ["OWN-MAME", "OWN-MAME", "OWN-MAME"] {
        runner.place_on_field(0, id, Some(0));
    }
    for id in ["OPP-C3", "OPP-C3B", "OPP-C5"] {
        runner.place_on_field(1, id, Some(0));
    }
    // First activation this turn (the [End of Your Turn] trigger fired once).
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(big));
    runner.game.drain_effect_queue();
    runner.accept_optional_trigger().expect("accept");
    pick_id(&mut runner, 0, "OWN-MAME");
    pick_id(&mut runner, 1, "OPP-C3");
    assert!(runner.pending_selection().is_none());
    // The same turn's end: [Once Per Turn] is spent — no prompt, nothing deleted.
    runner.pass_turn();
    assert!(runner.pending_selection().is_none(), "[Once Per Turn] lockout");
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(runner.battle_area_size(1), 2);
    // Opponent's turn passes; at the end of our next turn it is available again.
    runner.pass_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 0);
    runner.pass_turn();
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement), "lockout cleared");
}

#[test]
fn ex13_059_eot_does_not_fire_on_the_opponents_turn() {
    let mut runner = builder().deck(0, &["FILL"; 6]).memory(2).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-MAME", Some(0));
    runner.place_on_field(1, "OPP-C3", Some(0));
    runner.pass_turn();
    runner.decline_optional_trigger().expect("decline on own turn");
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    runner.pass_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 2, "no trigger at the end of the opponent's turn");
    assert_eq!(runner.battle_area_size(1), 1);
}

// ─── Section 5 — Inherited [End of Your Turn][OPT] delete all ────────────────

#[test]
fn ex13_059_inherited_eot_deletes_all_lowest_play_cost_opp_digimon() {
    let mut runner = builder().deck(0, &["FILL"; 6]).memory(2).start();
    runner.skip_mulligan();
    runner.place_stack(0, &[CARD_ID, "TOP-MAME6"]);
    runner.place_on_field(0, "OWN-MAME", Some(0));
    runner.place_on_field(1, "OPP-C3", Some(0));
    runner.place_on_field(1, "OPP-C3B", Some(0));
    runner.place_on_field(1, "OPP-C5", Some(0));
    runner.pass_turn();
    runner.accept_optional_trigger().expect("accept");
    pick_id(&mut runner, 0, "OWN-MAME");
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 1), vec!["OPP-C5".to_string()], "both cost-3 Digimon deleted");
    assert_eq!(field_ids(&runner, 0), vec!["TOP-MAME6".to_string()]);
}

#[test]
fn ex13_059_inherited_eot_does_not_apply_face_up() {
    // Face-up BigMamemon has only its own single-target EOT (not the delete-all).
    let mut runner = eot_board(&["OWN-MAME"], &["OPP-C3", "OPP-C3B"]);
    runner.accept_optional_trigger().expect("accept");
    pick_id(&mut runner, 0, "OWN-MAME");
    pick_id(&mut runner, 1, "OPP-C3");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 1, "only 1 deleted face-up");
}
