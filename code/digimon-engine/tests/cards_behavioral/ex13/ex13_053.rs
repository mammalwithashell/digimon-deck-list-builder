//! EX13-053 Thundermon — Digimon, Lv.4, Black, DP 4000, Play Cost 4.
//! Traits: Mutant. Form: Champion. Attribute: Data.
//! Digivolve: Black Lv.3 / 2; Yellow Lv.3 / 2.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-053.md`;
//! per-card JSON `code/digimon-engine/cards/ex13/EX13-053.json`)
//!
//! [On Play] [On Deletion] You may return up to 3 Digimon cards with [Mamemon]
//! in their texts from your trash to the top of the deck. Then, delete 1 of
//! your opponent's Digimon with a play cost of 3 or less. For each one this
//! effect returned, add 1 to this effect's play cost maximum.
//! (Rule) Name: Treated as including [Mamemon].
//! Inherited: [On Deletion] ＜De-Digivolve 1＞ 1 of your opponent's Digimon.
//!
//! Official Q&A: "[X] in its text" covers name, traits, effects, inherited
//! effects, (Rule), digivolve requirements, ... (a broad text scan).
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_053.cs). The (Rule) name follows DCGO's
//! EX12_041 "Also treated as including [Mamemon]" (`cardNames[i] =
//! "Mamemon " + name`): an alias that CONTAINS [Mamemon] without making the
//! card a [Mamemon] by exact name.
//!
//! # Pattern rows
//! - A4-variant trash → deck top, up to 3, each pick a separate ordered choice
//! - dynamic play-cost cap (3 + returned count) delete
//! - inherited [On Deletion] De-Digivolve 1
//! - identity alias (Rule name)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_digivolve, ATTACK_START, PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-053";

fn digimon(id: &str, name: &str, level: u8, cost: u16, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.play_cost = cost;
    c.dp = Some(level as i32 * 1000);
    c.colors = vec![color];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-053 YAML loads from the embedded pack")
        .add_card(digimon("MAME-1", "Mamemon", 5, 7, CardColor::Black))
        .add_card(digimon("MAME-2", "MetalMamemon", 5, 8, CardColor::Black))
        .add_card({
            let mut c = digimon("MAME-3", "Text Holder", 4, 4, CardColor::Black);
            c.effect_text = "Play 1 [Mamemon] from your hand.".to_string();
            c
        })
        .add_card({
            let mut c = digimon("MAME-4", "Inherit Holder", 4, 4, CardColor::Black);
            c.inherited_text = "[When Attacking] if you have a [Mamemon] ...".to_string();
            c
        })
        .add_card({
            let mut c = make_test_card("MAME-OPT", "Mamemon Smash");
            c.card_kind = CardKind::Option;
            c.level = None;
            c.dp = None;
            c
        })
        .add_card(digimon("PLAIN", "Plain", 4, 4, CardColor::Black))
        .add_card(digimon("BLACK-L3", "Black Three", 3, 3, CardColor::Black))
        .add_card(digimon("YELLOW-L3", "Yellow Three", 3, 3, CardColor::Yellow))
        .add_card(digimon("RED-L3", "Red Three", 3, 3, CardColor::Red))
        .add_card(digimon("TOP-L5", "Top Five", 5, 7, CardColor::Black))
        .add_card(digimon("OPP-C3", "Opp Three", 3, 3, CardColor::Red))
        .add_card(digimon("OPP-C4", "Opp Four", 4, 4, CardColor::Red))
        .add_card(digimon("OPP-C5", "Opp Five", 4, 5, CardColor::Red))
        .add_card(digimon("OPP-C6", "Opp Six", 5, 6, CardColor::Red))
        .add_card(digimon("OPP-C7", "Opp Seven", 5, 7, CardColor::Red))
        .add_card(digimon("OPP-LV3", "Opp Lv3", 3, 3, CardColor::Red))
        .add_card(digimon("OPP-LV4", "Opp Lv4", 4, 4, CardColor::Red))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
}

fn ids(cards: &[digimon_engine::card_source::CardSource], runner: &DebugRunner) -> Vec<String> {
    cards
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

fn handle_of(runner: &DebugRunner, player: u8, id: &str) -> PermanentHandle {
    let idx = field_ids(runner, player)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} on player {player}'s field"));
    PermanentHandle { player, index: idx as u8 }
}

fn trash_pick(runner: &DebugRunner, card_id: &str) -> u16 {
    let idx = runner.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in trash"));
    TRASH_EFFECT_START + idx as u16
}

fn offered_trash_ids(runner: &DebugRunner) -> Vec<String> {
    let view = runner.pending_selection_view().expect("trash prompt");
    let trash = ids(&runner.game.players[0].trash, runner);
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(TRASH_EFFECT_START))
        .filter_map(|i| trash.get(i as usize).cloned())
        .collect();
    out.sort();
    out
}

fn assert_return_prompt(runner: &DebugRunner) {
    let view = runner.pending_selection_view().expect("return prompt");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. } | SelectionKind::Trash),
        "trash pick expected, got {:?}",
        view.kind
    );
    assert!(runner.pending_is_optional(), "\"up to 3\" — every pick is declinable");
}

fn opp_offered(runner: &DebugRunner) -> Vec<String> {
    let view = runner.pending_selection_view().expect("delete prompt");
    assert_eq!(view.kind, SelectionKind::OppField, "{view:?}");
    assert!(!view.valid_action_ids.contains(&PASS), "the delete is mandatory");
    let field = field_ids(runner, 1);
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter_map(|&a| a.checked_sub(ATTACK_START))
        .filter_map(|i| field.get(i as usize).cloned())
        .collect();
    out.sort();
    out
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

/// Thundermon in hand, `trash` injected, the opponent's whole cost ladder on field.
fn play_with_trash(trash: &[&str]) -> DebugRunner {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(10).start();
    runner.skip_mulligan();
    for id in trash {
        runner.inject_trash(0, id);
    }
    for id in ["OPP-C3", "OPP-C4", "OPP-C5", "OPP-C6", "OPP-C7"] {
        runner.place_on_field(1, id, Some(0));
    }
    runner.play(0, 0).expect("Thundermon plays");
    runner
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_053_printed_metadata_paths_and_rule_name() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(4000), Some(4)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.traits, vec!["Mutant".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2);
    assert!(digi.iter().all(|p| p.cost == Some(CompiledCost::Literal(2))));

    let data = runner
        .game
        .card_data
        .iter()
        .find(|d| d.card_id == CARD_ID)
        .expect("card data");
    assert_eq!(data.card_name, "Thundermon");
    assert!(
        data.also_treated_as.iter().any(|n| n.contains("Mamemon")),
        "(Rule) Name: Treated as including [Mamemon]"
    );
    assert!(
        !data.also_treated_as.iter().any(|n| n == "Mamemon"),
        "including — not exactly [Mamemon]"
    );
}

#[test]
fn ex13_053_clause_shapes() {
    let runner = builder().start();
    let t = triggered(&runner);
    let main = t
        .iter()
        .find(|x| {
            x.scope == CompiledScope::FaceUp
                && x.when.contains(&CompiledTiming::OnPlay)
                && x.when.contains(&CompiledTiming::OnDeletion)
        })
        .expect("[On Play][On Deletion] clause");
    assert!(!main.optional, "the delete is mandatory; only the return is a may");
    assert!(!main.once_per_turn);
    assert!(t.iter().any(|x| x.scope == CompiledScope::Inherited
        && x.when == vec![CompiledTiming::OnDeletion]));
}

// ─── Section 2 — Digivolve gating ────────────────────────────────────────────

#[test]
fn ex13_053_digivolves_from_black_or_yellow_lv3_for_2() {
    for base_id in ["BLACK-L3", "YELLOW-L3"] {
        let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
        runner.skip_mulligan();
        let base = runner.place_on_field(0, base_id, Some(0));
        runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
        let _ = runner.auto_resolve();
        assert_eq!(field_ids(&runner, 0), vec![CARD_ID.to_string()], "from {base_id}");
        assert_eq!(runner.memory(), 3);
    }
}

#[test]
fn ex13_053_cannot_digivolve_from_red_lv3() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(5).start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "RED-L3", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    assert_eq!(field_ids(&runner, 0), vec!["RED-L3".to_string()]);
}

// ─── Section 3 — [On Play] return-then-delete ────────────────────────────────

#[test]
fn ex13_053_only_mamemon_text_digimon_cards_are_returnable() {
    let runner = play_with_trash(&["MAME-1", "MAME-3", "MAME-4", "MAME-OPT", "PLAIN"]);
    assert_return_prompt(&runner);
    assert_eq!(
        offered_trash_ids(&runner),
        vec!["MAME-1".to_string(), "MAME-3".to_string(), "MAME-4".to_string()],
        "name / effect text / inherited text all count; the Option and the plain card don't"
    );
}

#[test]
fn ex13_053_returning_none_deletes_cost_3_or_less() {
    let mut runner = play_with_trash(&["MAME-1"]);
    runner.execute_action(0, PASS).expect("return none");
    assert_eq!(opp_offered(&runner), vec!["OPP-C3".to_string()]);
}

#[test]
fn ex13_053_no_eligible_trash_goes_straight_to_the_cost_3_delete() {
    let runner = play_with_trash(&["PLAIN"]);
    assert_eq!(opp_offered(&runner), vec!["OPP-C3".to_string()]);
}

#[test]
fn ex13_053_returning_one_raises_the_cap_to_4() {
    let mut runner = play_with_trash(&["MAME-1", "MAME-2"]);
    runner.execute_action(0, trash_pick(&runner, "MAME-1")).unwrap();
    assert_return_prompt(&runner);
    runner.execute_action(0, PASS).expect("stop at one");
    assert_eq!(opp_offered(&runner), vec!["OPP-C3".to_string(), "OPP-C4".to_string()]);
}

#[test]
fn ex13_053_returning_three_raises_the_cap_to_6_and_stacks_in_pick_order() {
    let mut runner = play_with_trash(&["MAME-1", "MAME-2", "MAME-3", "MAME-4"]);
    let deck0 = runner.deck_size(0);
    runner.execute_action(0, trash_pick(&runner, "MAME-2")).unwrap();
    runner.execute_action(0, trash_pick(&runner, "MAME-4")).unwrap();
    runner.execute_action(0, trash_pick(&runner, "MAME-1")).unwrap();
    assert_eq!(
        opp_offered(&runner),
        vec![
            "OPP-C3".to_string(),
            "OPP-C4".to_string(),
            "OPP-C5".to_string(),
            "OPP-C6".to_string()
        ],
        "3 + 3 = 6"
    );
    let c6 = handle_of(&runner, 1, "OPP-C6");
    runner.execute_action(0, ATTACK_START + c6.index as u16).unwrap();
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 1).contains(&"OPP-C6".to_string()));
    assert_eq!(runner.deck_size(0), deck0 + 3);
    let deck = ids(&runner.game.players[0].deck, &runner);
    let n = deck.len();
    assert_eq!(&deck[n - 3..], &["MAME-2", "MAME-4", "MAME-1"], "last pick is the new top");
    assert_eq!(ids(&runner.game.players[0].trash, &runner), vec!["MAME-3".to_string()]);
}

#[test]
fn ex13_053_at_most_three_returns() {
    let mut runner = play_with_trash(&["MAME-1", "MAME-2", "MAME-3", "MAME-4"]);
    for id in ["MAME-1", "MAME-2", "MAME-3"] {
        runner.execute_action(0, trash_pick(&runner, id)).unwrap();
    }
    // The next prompt is the delete, not a 4th return.
    let view = runner.pending_selection_view().expect("delete prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
}

#[test]
fn ex13_053_delete_with_no_opponent_digimon_is_skipped() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(10).start();
    runner.skip_mulligan();
    runner.inject_trash(0, "MAME-1");
    runner.play(0, 0).expect("play");
    runner.execute_action(0, trash_pick(&runner, "MAME-1")).unwrap();
    // Trash is now empty: no 2nd pick, and no opponent Digimon to delete.
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.trash_size(0), 0, "the return still happened");
}

// ─── Section 4 — [On Deletion] (face-up) ─────────────────────────────────────

#[test]
fn ex13_053_on_deletion_runs_the_same_body() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    runner.inject_trash(0, "MAME-1");
    let thunder = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-C4", Some(0));
    runner
        .game
        .delete_permanent_with_cause(thunder, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    assert_return_prompt(&runner);
    runner.execute_action(0, trash_pick(&runner, "MAME-1")).unwrap();
    runner.execute_action(0, PASS).unwrap();
    assert_eq!(opp_offered(&runner), vec!["OPP-C4".to_string()]);
}

// ─── Section 5 — Inherited [On Deletion] De-Digivolve 1 ──────────────────────

#[test]
fn ex13_053_inherited_on_deletion_de_digivolves_1() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-L5"]);
    let opp = runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("De-Digivolve target");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(view.selecting_player, ATTACK_START + opp.index as u16)
        .unwrap();
    let _ = runner.auto_resolve();
    let top = runner.game.players[1].battle_area[opp.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top, "OPP-LV3");
}

#[test]
fn ex13_053_inherited_does_not_run_the_face_up_return_body() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    runner.inject_trash(0, "MAME-1");
    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-L5"]);
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();
    assert!(
        ids(&runner.game.players[0].trash, &runner).contains(&"MAME-1".to_string()),
        "the face-up [On Deletion] return does not fire from under another Digimon"
    );
}

// ─── Section 6 — (Rule) name integration ─────────────────────────────────────

#[test]
fn ex13_053_counts_as_mamemon_named_for_princemamemons_aura() {
    let mut runner = builder()
        .dsl_card("EX13-063")
        .expect("EX13-063 loads")
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, "EX13-063", Some(0));
    let thunder = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(thunder, Keyword::Blocker),
        "Thundermon is a Digimon with [Mamemon] in its name"
    );
}
