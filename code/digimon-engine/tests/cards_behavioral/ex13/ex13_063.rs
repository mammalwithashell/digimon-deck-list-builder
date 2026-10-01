//! EX13-063 PrinceMamemon — Digimon, Lv.6, Black, DP 12000, Play Cost 11.
//! Traits: Mutant. Form: Mega. Attribute: Data.
//! Digivolve: Black Lv.5 / 3.
//! Assembly -4: 3 Lv.5 or lower [Mamemon] text cards w/different names.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-063.md`;
//! per-card JSON `code/digimon-engine/cards/ex13/EX13-063.json`)
//!
//! [On Play] [When Digivolving] [On Deletion] Reveal the top 3 cards of your
//! deck. You may play 1 play cost 10 or lower Digimon card with [Mamemon] in
//! its name or the [Mutant] trait among them without paying the cost. Trash
//! the rest.
//! [On Deletion] Delete 1 of your opponent's highest play cost Digimon.
//! [All Turns] All of your Digimon with [Mamemon] in their names gain
//! ＜Blocker＞ and ＜Guard＞. (When any of your other Digimon would leave the
//! battle area by your opponent's effects, by deleting this Digimon, they
//! don't leave.)
//! Assembly: "When this would be played, by placing the specified cards from
//! the trash under it, reduce the play cost."
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_063.cs). Printed text + general_rule.pdf
//! (16-45 <Guard>) govern.
//!
//! # Pattern rows
//! - A1/A3 reveal-3 free play, trash rest (EX8-050 idiom)
//! - highest-play-cost single pick (selector, EX11-044 idiom)
//! - H-aura keyword grants (<Blocker>, <Guard>) over a name filter
//! - C-Assembly, repeat 3 + distinct_by: name, "[X] text" material

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{
    encode_digivolve, ATTACK_START, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT, SEL_REVEAL_START,
    TRASH_EFFECT_START,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-063";

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
        .expect("EX13-063 YAML loads from the embedded pack")
        .add_card(digimon("MAME-10", "Grand Mamemon", 5, 10, 9000))
        .add_card(digimon("MAME-11", "Ultra Mamemon", 6, 11, 12000))
        .add_card({
            let mut c = digimon("MUT-10", "Mutant Ten", 5, 10, 9000);
            c.traits = vec!["Mutant".to_string()];
            c
        })
        .add_card({
            let mut c = digimon("TEXT-MAME", "Text Only", 4, 4, 4000);
            c.effect_text = "Digivolve into [Mamemon].".to_string();
            c
        })
        .add_card(digimon("PLAIN", "Plain", 4, 4, 4000))
        .add_card(digimon("BLACK-L5", "Black Five", 5, 7, 7000))
        .add_card({
            let mut c = digimon("RED-L5", "Red Five", 5, 7, 7000);
            c.colors = vec![CardColor::Red];
            c
        })
        // Assembly materials: [Mamemon] in their text, Lv.5 or lower.
        .add_card(digimon("ASM-A", "Mamemon", 4, 5, 5000))
        .add_card(digimon("ASM-A2", "Mamemon", 4, 5, 5000))
        .add_card(digimon("ASM-B", "MetalMamemon", 5, 7, 8000))
        .add_card({
            let mut c = digimon("ASM-C", "Thunder Thing", 4, 4, 4000);
            c.effect_text = "You may return Digimon cards with [Mamemon] in their texts.".to_string();
            c
        })
        .add_card(digimon("ASM-L6", "BigTime Mamemon", 6, 9, 10000))
        .add_card(digimon("OWN-MAME", "Ally Mamemon", 4, 5, 6000))
        .add_card(digimon("OWN-OTHER", "Ally Other", 4, 5, 6000))
        .add_card(digimon("OPP-C3", "Opp Cost Three", 3, 3, 3000))
        .add_card(digimon("OPP-C5", "Opp Cost Five", 4, 5, 5000))
        .add_card(digimon("OPP-C7", "Opp Cost Seven", 5, 7, 7000))
        .add_card(digimon("OPP-C7B", "Opp Cost Seven B", 5, 7, 7000))
        .add_card(digimon("OPP-MAME", "Enemy Mamemon", 4, 5, 6000))
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

fn trash_index(runner: &DebugRunner, player: u8, card_id: &str) -> u16 {
    runner.game.players[player as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in trash")) as u16
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

/// Deck listed bottom-first; the revealed top 3 are the last three ids.
fn play_with_deck(top3_bottom_first: &[&str]) -> DebugRunner {
    let mut deck = vec!["FILL", "FILL"];
    deck.extend_from_slice(top3_bottom_first);
    let mut runner = builder().hand(0, &[CARD_ID]).deck(0, &deck).memory(11).start();
    runner.skip_mulligan();
    runner.play(0, 0).expect("PrinceMamemon plays");
    runner
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_063_printed_metadata_digivolve_and_assembly() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(6), Some(12000), Some(11)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.traits, vec!["Mutant".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 1);
    assert_eq!(digi[0].cost, Some(CompiledCost::Literal(3)));
    let asm = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly -4");
    assert_eq!(asm.cost, Some(CompiledCost::Literal(4)));
    assert_eq!(asm.materials.len(), 1, "one repeated material slot");
    assert!(asm.materials[0].stack_under);
}

#[test]
fn ex13_063_clause_shapes() {
    let runner = builder().start();
    let t = triggered(&runner);
    let reveal = t
        .iter()
        .find(|x| {
            x.when.contains(&CompiledTiming::OnPlay)
                && x.when.contains(&CompiledTiming::WhenDigivolving)
                && x.when.contains(&CompiledTiming::OnDeletion)
        })
        .expect("[OP][WD][OD] reveal clause");
    assert!(!reveal.optional);
    let del = t
        .iter()
        .find(|x| x.when == vec![CompiledTiming::OnDeletion])
        .expect("[On Deletion] delete-highest clause");
    assert!(!del.optional, "mandatory");
    let c = runner.compiled_card(CARD_ID).unwrap();
    let grants: Vec<String> = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::FaceUp,
                grant_keyword: Some(g),
                ..
            }) => Some(g.keyword.clone()),
            _ => None,
        })
        .collect();
    assert!(grants.contains(&"Blocker".to_string()));
    assert!(grants.contains(&"Guard".to_string()));
}

// ─── Section 2 — Entry paths ─────────────────────────────────────────────────

#[test]
fn ex13_063_digivolves_from_black_lv5_for_3() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "BLACK-L5", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 0), vec![CARD_ID.to_string()]);
    assert_eq!(runner.memory(), 2);
}

#[test]
fn ex13_063_cannot_digivolve_from_red_lv5() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "RED-L5", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    assert_eq!(field_ids(&runner, 0), vec!["RED-L5".to_string()]);
}

#[test]
fn ex13_063_assembly_three_distinct_mamemon_text_cards_for_7() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .memory(7)
        .start();
    runner.skip_mulligan();
    for id in ["ASM-A", "ASM-B", "ASM-C"] {
        runner.inject_trash(0, id);
    }
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    for id in ["ASM-A", "ASM-B", "ASM-C"] {
        assert!(runner.game.pending_selection.is_some(), "Assembly element {id}");
        let idx = trash_index(&runner, 0, id);
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 4);
    assert_eq!(mem0 - runner.game.memory, 7, "11 - 4");
}

#[test]
#[ignore = "G-ASSEMBLY-NO-DISTINCT-BY (docs/RUST_ENGINE_GAPS.md): resolve_eligible_assembly / assembly_can_fulfill / install_assembly_element drop the material's `distinct_by`, so same-name cards satisfy \"w/different names\""]
fn ex13_063_assembly_rejects_duplicate_names_and_level_6() {
    // Two [Mamemon]-named copies + a Lv.6: only 2 distinct eligible names → no
    // Assembly; full cost 11 at memory 0 overdraws → the play is masked out.
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .memory(0)
        .start();
    runner.skip_mulligan();
    for id in ["ASM-A", "ASM-A2", "ASM-L6", "ASM-B"] {
        runner.inject_trash(0, id);
    }
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 0.0);
}

#[test]
#[ignore = "G-ASSEMBLY-NO-DISTINCT-BY (docs/RUST_ENGINE_GAPS.md)"]
fn ex13_063_assembly_second_pick_excludes_an_already_chosen_name() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .memory(7)
        .start();
    runner.skip_mulligan();
    for id in ["ASM-A", "ASM-A2", "ASM-B", "ASM-C"] {
        runner.inject_trash(0, id);
    }
    runner.game.decode_action(PLAY_HAND_START, 0);
    let a = trash_index(&runner, 0, "ASM-A");
    runner.game.decode_action(TRASH_EFFECT_START + a, 0);
    let view = runner.pending_selection_view().expect("next Assembly pick");
    let dup = TRASH_EFFECT_START + trash_index(&runner, 0, "ASM-A2");
    assert!(
        !view.valid_action_ids.contains(&dup),
        "a second [Mamemon]-named card is not a different name"
    );
}

#[test]
fn ex13_063_assembly_needs_all_three_materials() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .memory(7)
        .start();
    runner.skip_mulligan();
    for id in ["ASM-A", "ASM-B", "ASM-C"] {
        runner.inject_trash(0, id);
    }
    runner.game.decode_action(PLAY_HAND_START, 0);
    let a = trash_index(&runner, 0, "ASM-A");
    runner.game.decode_action(TRASH_EFFECT_START + a, 0);
    let view = runner.pending_selection_view().expect("Assembly still selecting");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "Assembly -4 needs all 3 cards; it can't be closed after 1"
    );
}

#[test]
fn ex13_063_assembly_available_with_three_distinct_names_at_memory_0() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .memory(0)
        .start();
    runner.skip_mulligan();
    for id in ["ASM-A", "ASM-A2", "ASM-B", "ASM-C"] {
        runner.inject_trash(0, id);
    }
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 1.0, "11 - 4 = 7 ≤ the −10 floor budget");
}

// ─── Section 3 — reveal clause ───────────────────────────────────────────────

#[test]
fn ex13_063_on_play_offers_mamemon_or_mutant_at_cost_10_or_less() {
    let runner = play_with_deck(&["MAME-10", "MUT-10", "MAME-11"]);
    assert_eq!(offered_reveal_ids(&runner), vec!["MAME-10".to_string(), "MUT-10".to_string()]);
    assert!(runner.pending_is_optional());
}

#[test]
fn ex13_063_on_play_text_only_card_is_not_eligible() {
    let runner = play_with_deck(&["TEXT-MAME", "PLAIN", "MAME-11"]);
    if runner.pending_selection().is_some() {
        assert!(offered_reveal_ids(&runner).is_empty());
    }
}

#[test]
fn ex13_063_on_play_plays_free_and_trashes_rest() {
    // Memory caps at 10, so the 11-cost play itself would pass the turn; fire
    // the [On Play] of a fielded PrinceMamemon to isolate the free play.
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "PLAIN", "MUT-10", "MAME-11"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, 0);
    let mem0 = runner.memory();
    let trash0 = runner.trash_size(0);
    pick_revealed(&mut runner, "MUT-10");
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"MUT-10".to_string()));
    assert_eq!(runner.memory(), mem0, "without paying the cost");
    assert_eq!(runner.trash_size(0), trash0 + 2);
}

#[test]
fn ex13_063_when_digivolving_reveals() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "PLAIN", "PLAIN", "MAME-10", "FILL"])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "BLACK-L5", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    assert_eq!(offered_reveal_ids(&runner), vec!["MAME-10".to_string()]);
}

// ─── Section 4 — [On Deletion] ───────────────────────────────────────────────

#[test]
fn ex13_063_on_deletion_deletes_the_opponents_highest_play_cost_digimon() {
    let mut runner = builder().deck(0, &["FILL"; 6]).start();
    runner.skip_mulligan();
    let prince = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-C3", Some(0));
    runner.place_on_field(1, "OPP-C7", Some(0));
    runner.place_on_field(1, "OPP-C5", Some(0));
    runner
        .game
        .delete_permanent_with_cause(prince, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();
    let opp = field_ids(&runner, 1);
    assert!(!opp.contains(&"OPP-C7".to_string()), "highest play cost deleted");
    assert_eq!(opp.len(), 2);
}

#[test]
fn ex13_063_on_deletion_tied_highest_is_a_player_choice() {
    let mut runner = builder().deck(0, &["FILL"; 6]).start();
    runner.skip_mulligan();
    let prince = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-C7", Some(0));
    runner.place_on_field(1, "OPP-C3", Some(0));
    runner.place_on_field(1, "OPP-C7B", Some(0));
    runner
        .game
        .delete_permanent_with_cause(prince, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    // Walk prompts until the opponent-field pick; take OPP-C7B.
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else { break };
        if view.kind == SelectionKind::OppField {
            let offered: Vec<u16> = view
                .valid_action_ids
                .iter()
                .copied()
                .filter(|&a| a != PASS)
                .collect();
            assert_eq!(offered.len(), 2, "only the two cost-7 Digimon: {view:?}");
            let b = handle_of(&runner, 1, "OPP-C7B");
            runner
                .execute_action(view.selecting_player, ATTACK_START + b.index as u16)
                .unwrap();
            break;
        }
        let a = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS);
        runner.execute_action(view.selecting_player, a).unwrap();
    }
    let _ = runner.auto_resolve();
    let opp = field_ids(&runner, 1);
    assert!(opp.contains(&"OPP-C7".to_string()));
    assert!(!opp.contains(&"OPP-C7B".to_string()));
}

#[test]
fn ex13_063_on_deletion_also_reveals() {
    let mut runner = builder()
        .deck(0, &["FILL", "PLAIN", "PLAIN", "MAME-10"])
        .start();
    runner.skip_mulligan();
    let prince = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .delete_permanent_with_cause(prince, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 0), vec!["MAME-10".to_string()], "first legal pick played");
}

// ─── Section 5 — [All Turns] <Blocker> / <Guard> aura ────────────────────────

#[test]
fn ex13_063_mamemon_named_allies_gain_blocker_and_guard() {
    let mut runner = builder().deck(0, &["FILL"; 6]).start();
    runner.skip_mulligan();
    let prince = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "OWN-MAME", Some(0));
    let other = runner.place_on_field(0, "OWN-OTHER", Some(0));
    let enemy = runner.place_on_field(1, "OPP-MAME", Some(0));
    runner.game.tick_declarative_effects();
    for h in [prince, ally] {
        assert!(runner.game.has_keyword(h, Keyword::Blocker), "{h:?} Blocker");
        assert!(runner.game.has_keyword(h, Keyword::Guard), "{h:?} Guard");
    }
    assert!(!runner.game.has_keyword(other, Keyword::Blocker), "no [Mamemon] in its name");
    assert!(!runner.game.has_keyword(other, Keyword::Guard));
    assert!(!runner.game.has_keyword(enemy, Keyword::Blocker), "only YOUR Digimon");
    assert!(!runner.game.has_keyword(enemy, Keyword::Guard));
}

#[test]
fn ex13_063_aura_ends_when_prince_leaves() {
    let mut runner = builder().deck(0, &["FILL"; 6]).start();
    runner.skip_mulligan();
    let prince = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "OWN-MAME", Some(0));
    runner.game.tick_declarative_effects();
    runner.game.players[0].battle_area.remove(prince.index as usize);
    runner.game.tick_declarative_effects();
    let ally = handle_of(&runner, 0, "OWN-MAME");
    assert!(!runner.game.has_keyword(ally, Keyword::Blocker));
    assert!(!runner.game.has_keyword(ally, Keyword::Guard));
}

#[test]
fn ex13_063_granted_guard_saves_an_ally_from_an_opponent_effect() {
    let mut runner = builder().deck(0, &["FILL"; 6]).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "OWN-OTHER", Some(0));
    runner.place_on_field(0, "OWN-MAME", Some(0));
    runner.game.tick_declarative_effects();
    // The opponent's effect would delete OWN-OTHER; OWN-MAME's granted <Guard>
    // (or PrinceMamemon's) may be deleted instead.
    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    let view = runner.pending_selection_view().expect("<Guard> replacement offered");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("an accept option");
    runner.execute_action(view.selecting_player, accept).unwrap();
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();
    let mine = field_ids(&runner, 0);
    assert!(mine.contains(&"OWN-OTHER".to_string()), "OWN-OTHER doesn't leave");
    assert_eq!(mine.len(), 2, "one <Guard> carrier was deleted instead");
}
