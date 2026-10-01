//! EX13-065 Sistermon Blanc (Awakened) — DUAL card (Digimon + Option).
//! Digimon face: Lv.3 White/Yellow, DP 5000, Puppet, Rookie, Vaccine. No play
//! cost (the Digimon face is only reached by digivolving / Arts digivolving).
//! Option face: White, use cost 5.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-065.md`;
//! per-card JSON `code/digimon-engine/cards/ex13/EX13-065.json`)
//!
//! DIGIMON FACE
//!   [Digivolve] [Sistermon Blanc]: Cost 0
//!   [Digivolve] Lv.2 w/[Huckmon] in text: Cost 1
//!   ＜Decode ([Sistermon Blanc])＞ (When this Digimon would leave the battle
//!   area other than in battle, you may play 1 specified Digimon card from its
//!   digivolution cards without paying the cost.)
//!   ＜Guard＞ (When any of your other Digimon would leave the battle area by
//!   your opponent's effects, by deleting this Digimon, they don't leave.)
//! OPTION FACE
//!   [Main] You may play 1 play cost 4 or lower card with [Sistermon] in its
//!   name from your hand or trash without paying the cost. Then, to 1 of your
//!   opponent's Digimon, give -3000 DP for the turn for each of your Digimon.
//!   ＜Arts Digivolve＞ (Instead of trashing after use, your cards may
//!   digivolve into this card without paying the cost.)
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_065.cs at b9a0638cd). Printed text +
//! general_rule.pdf (16-35 <Decode>, 16-45 <Guard>) govern.
//!
//! # Pattern rows
//! - G-DSL-DUAL-PER-FACE-EFFECTS, G-DSL-ARTS-DIGIVOLVE (ST23-09 / BT25-043 idiom)
//! - Name-narrowed <Decode> replacement (BT25-025 idiom)
//! - <Guard> static keyword
//! - Name / Lv.2 "in text" special digivolve conditions (no standard circles)
//! - A-union hand-or-trash free play (EX12-074 idiom) + per-ally DP formula

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledDeclarativeClause};
use digimon_engine::action::space::{
    encode_attack, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "EX13-065";

fn digimon(id: &str, name: &str, level: u8, cost: u16, dp: i32, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.play_cost = cost;
    c.dp = Some(dp);
    c.colors = vec![color];
    c.traits = vec!["Puppet".to_string()];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-065 YAML loads from the embedded pack")
        .add_card(digimon("SIS-BLANC", "Sistermon Blanc", 3, 3, 3000, CardColor::White))
        .add_card(digimon("SIS-CIEL", "Sistermon Ciel", 4, 4, 4000, CardColor::White))
        .add_card(digimon("SIS-BIG", "Sistermon Grande", 5, 5, 6000, CardColor::White))
        .add_card({
            let mut c = digimon("HUCK-L2", "Pupmon", 2, 0, 0, CardColor::Red);
            c.effect_text = "[Digivolve] into [Huckmon].".to_string();
            c
        })
        .add_card(digimon("PLAIN-L2", "Plainmon", 2, 0, 0, CardColor::Red))
        .add_card(digimon("WHITE-ALLY", "White Ally", 4, 4, 5000, CardColor::White))
        .add_card(digimon("OPP-A", "Opp A", 6, 10, 12000, CardColor::Blue))
        .add_card(digimon("FILL", "Filler", 3, 3, 3000, CardColor::Blue))
}

fn field_ids(r: &DebugRunner, player: usize) -> Vec<String> {
    r.game
        .player(player as u8)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_pos(r: &DebugRunner, player: usize, id: &str) -> usize {
    r.game
        .player(player as u8)
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in hand"))
}

fn trash_pos(r: &DebugRunner, player: usize, id: &str) -> usize {
    r.game
        .player(player as u8)
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in trash"))
}

// ─── Section 1 — structural ──────────────────────────────────────────────────

#[test]
fn ex13_065_is_dual_with_both_faces() {
    let r = builder().start();
    let compiled = r.compiled_card(CARD_ID).expect("compiled EX13-065");
    assert_eq!(compiled.kind, CompiledCardKind::Dual);
    let dual = compiled.dual.as_ref().expect("dual faces");
    assert_eq!(dual.digimon.level, 3);
    assert_eq!(dual.digimon.dp, 5000);
    assert_eq!(dual.option.use_cost, 5);
    // Digimon face: <Guard> grant + <Decode> replacement.
    assert_eq!(dual.digimon.effects.len(), 2);
    // Option face: one [Main] clause.
    assert_eq!(dual.option.effects.len(), 1);
    assert!(dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"));
}

#[test]
fn ex13_065_digimon_face_declares_guard() {
    let r = builder().start();
    let dual = r.compiled_card(CARD_ID).unwrap().dual.clone().unwrap();
    let grants: Vec<String> = dual
        .digimon
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) => Some(keyword.clone()),
            _ => None,
        })
        .collect();
    assert!(grants.contains(&"Guard".to_string()));
}

// ─── Section 2/3 — digivolution conditions ───────────────────────────────────

fn digivolve_runner(base: &str) -> (DebugRunner, usize) {
    let mut r = builder().hand(0, &[CARD_ID]).memory(5).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, base, Some(0)).index as usize;
    (r, slot)
}

#[test]
fn ex13_065_digivolves_from_sistermon_blanc_for_0() {
    let (mut r, slot) = digivolve_runner("SIS-BLANC");
    let mem = r.game.memory;
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(field_ids(&r, 0)[slot], CARD_ID);
    assert_eq!(mem - r.game.memory, 0, "[Sistermon Blanc]: Cost 0");
}

#[test]
fn ex13_065_digivolves_from_huckmon_text_lv2_for_1() {
    let (mut r, slot) = digivolve_runner("HUCK-L2");
    let mem = r.game.memory;
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(field_ids(&r, 0)[slot], CARD_ID);
    assert_eq!(mem - r.game.memory, 1, "Lv.2 w/[Huckmon] in text: Cost 1");
}

#[test]
fn ex13_065_cannot_digivolve_from_plain_lv2() {
    let (mut r, slot) = digivolve_runner("PLAIN-L2");
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(field_ids(&r, 0)[slot], "PLAIN-L2");
}

#[test]
fn ex13_065_cannot_digivolve_from_other_sistermon() {
    // [Sistermon Blanc] is a NAME condition, not "Sistermon in name".
    let (mut r, slot) = digivolve_runner("SIS-CIEL");
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(field_ids(&r, 0)[slot], "SIS-CIEL");
}

// ─── <Guard> ─────────────────────────────────────────────────────────────────

#[test]
fn ex13_065_has_guard_on_field() {
    let mut r = builder().start();
    let me = r.place_stack(0, &["SIS-BLANC", CARD_ID]);
    assert!(r.game.has_keyword(me, Keyword::Guard));
}

fn guard_saves_ally(stack: &[&str]) {
    let mut r = builder().deck(0, &["FILL"; 6]).start();
    r.skip_mulligan();
    r.place_stack(0, stack);
    let ally = r.place_on_field(0, "WHITE-ALLY", Some(0));
    r.game.tick_declarative_effects();
    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    let view = r.pending_selection_view().expect("<Guard> replacement offered");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("accept option");
    r.execute_action(view.selecting_player, accept).unwrap();
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    let mine = field_ids(&r, 0);
    assert!(mine.contains(&"WHITE-ALLY".to_string()), "ally doesn't leave");
    assert!(!mine.contains(&CARD_ID.to_string()), "the <Guard> carrier was deleted instead");
}

/// <Guard> with no [Sistermon Blanc] under the carrier.
#[test]
fn ex13_065_guard_saves_an_ally_from_an_opponent_effect() {
    guard_saves_ally(&["PLAIN-L2", CARD_ID]);
}

/// The card's signature line: <Guard> deletes this Digimon (other than in
/// battle) → its own <Decode> may play the [Sistermon Blanc] under it. The
/// Decode window opens INSIDE Guard's replacement process, i.e. a second
/// replacement parks while Guard's is still unresolved — the engine's
/// single-slot `parked_replacement` trips its nested-park debug_assert (and
/// would drop the outer outcome in release).
#[test]
#[ignore = "pending: G-NESTED-PARKED-REPLACEMENT from qa/archetype-qa/engine-gaps.md"]
fn ex13_065_guard_deletion_then_decode_plays_sistermon_blanc() {
    let mut r = builder().deck(0, &["FILL"; 6]).start();
    r.skip_mulligan();
    r.place_stack(0, &["SIS-BLANC", CARD_ID]);
    let ally = r.place_on_field(0, "WHITE-ALLY", Some(0));
    r.game.tick_declarative_effects();
    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    while let Some(v) = r.pending_selection_view() {
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(v.selecting_player, a).unwrap();
    }
    let mine = field_ids(&r, 0);
    assert!(mine.contains(&"WHITE-ALLY".to_string()), "ally doesn't leave");
    assert!(mine.contains(&"SIS-BLANC".to_string()), "<Decode> played [Sistermon Blanc]");
    assert!(!mine.contains(&CARD_ID.to_string()));
}

// ─── <Decode ([Sistermon Blanc])> ───────────────────────────────────────────

#[test]
fn ex13_065_decode_plays_sistermon_blanc_source_on_non_battle_leave() {
    let mut r = builder().memory(5).start();
    let me = r.place_stack(0, &["SIS-BLANC", CARD_ID]);
    r.game.return_to_hand_from_effect(me, 1);
    // Optional <Decode> window, then the source pick.
    let view = r.pending_selection_view().expect("<Decode> window opens");
    assert!(view.is_optional, "<Decode> is a 'you may'");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .unwrap();
    r.execute_action(view.selecting_player, accept).unwrap();
    while let Some(v) = r.pending_selection_view() {
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(v.selecting_player, a).unwrap();
    }
    assert!(field_ids(&r, 0).contains(&"SIS-BLANC".to_string()), "[Sistermon Blanc] played");
    assert_eq!(r.hand_size(0), 1, "the Awakened card still returns to hand");
}

#[test]
fn ex13_065_decode_does_not_play_a_non_matching_source() {
    let mut r = builder().memory(5).start();
    let me = r.place_stack(0, &["SIS-CIEL", CARD_ID]);
    r.game.return_to_hand_from_effect(me, 1);
    let _ = r.auto_resolve();
    assert!(
        !field_ids(&r, 0).contains(&"SIS-CIEL".to_string()),
        "[Sistermon Ciel] is not the specified [Sistermon Blanc]"
    );
}

#[test]
fn ex13_065_decode_does_not_fire_on_battle_deletion() {
    let mut r = builder().memory(5).start();
    let me = r.place_stack(0, &["SIS-BLANC", CARD_ID]);
    r.game.delete_permanent_with_cause(me, ReplacementCause::Battle);
    let _ = r.auto_resolve();
    assert!(
        !field_ids(&r, 0).contains(&"SIS-BLANC".to_string()),
        "<Decode> only works other than in battle"
    );
}

// ─── Option [Main] ───────────────────────────────────────────────────────────

/// Option use: a white Digimon on field pays the colour requirement.
fn option_runner(hand: &[&str], trash: &[&str]) -> DebugRunner {
    let mut r = builder().hand(0, hand).memory(8).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "WHITE-ALLY", Some(0));
    for id in trash {
        r.inject_trash(0, id);
    }
    r
}

#[test]
fn ex13_065_option_plays_sistermon_from_hand_then_dp_minus_per_ally() {
    let mut r = option_runner(&[CARD_ID, "SIS-CIEL"], &[]);
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    let idx = hand_pos(&r, 0, CARD_ID);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);

    let view = r.pending_selection_view().expect("union pick");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    assert!(view.is_optional, "'You may play'");
    let ciel = PLAY_HAND_START + hand_pos(&r, 0, "SIS-CIEL") as u16;
    assert!(view.valid_action_ids.contains(&ciel));
    r.execute_action(0, ciel).unwrap();
    assert!(field_ids(&r, 0).contains(&"SIS-CIEL".to_string()));

    // -3000 × 2 own Digimon (WHITE-ALLY + SIS-CIEL).
    let view = r.pending_selection_view().expect("DP target pick");
    assert_eq!(view.kind, SelectionKind::OppField);
    r.execute_action(0, encode_attack(0, opp.index as u16)).unwrap();
    assert_eq!(r.effective_dp(opp), Some(12000 - 6000));
}

#[test]
fn ex13_065_option_plays_sistermon_from_trash() {
    let mut r = option_runner(&[CARD_ID], &["SIS-BLANC"]);
    let idx = hand_pos(&r, 0, CARD_ID);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);
    let blanc = TRASH_EFFECT_START + trash_pos(&r, 0, "SIS-BLANC") as u16;
    let view = r.pending_selection_view().expect("union pick");
    assert!(view.valid_action_ids.contains(&blanc));
    r.execute_action(0, blanc).unwrap();
    assert!(field_ids(&r, 0).contains(&"SIS-BLANC".to_string()));
}

#[test]
fn ex13_065_option_excludes_cost_5_sistermon() {
    let mut r = option_runner(&[CARD_ID, "SIS-BIG"], &[]);
    let idx = hand_pos(&r, 0, CARD_ID);
    let big = PLAY_HAND_START + hand_pos(&r, 0, "SIS-BIG") as u16 - 1; // after the Option leaves hand
    // Nothing to do (no eligible card, no opp Digimon, no Arts base) → the
    // Option may resolve straight through; either way SIS-BIG is never offered.
    let _ = r.game.play_option_from_hand(0, idx);
    if let Some(view) = r.pending_selection_view() {
        if matches!(view.kind, SelectionKind::UnionZone { .. }) {
            assert!(!view.valid_action_ids.contains(&big), "play cost 5 is not 4 or lower");
        }
    }
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"SIS-BIG".to_string()));
}

#[test]
fn ex13_065_option_play_is_declinable_and_dp_still_applies() {
    let mut r = option_runner(&[CARD_ID, "SIS-CIEL"], &[]);
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    let idx = hand_pos(&r, 0, CARD_ID);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);
    r.execute_action(0, PASS).unwrap();
    assert!(!field_ids(&r, 0).contains(&"SIS-CIEL".to_string()));
    let view = r.pending_selection_view().expect("DP target pick");
    assert_eq!(view.kind, SelectionKind::OppField);
    r.execute_action(0, encode_attack(0, opp.index as u16)).unwrap();
    // Only WHITE-ALLY → -3000.
    assert_eq!(r.effective_dp(opp), Some(12000 - 3000));
}

// ─── <Arts Digivolve> ────────────────────────────────────────────────────────

#[test]
fn ex13_065_arts_digivolves_onto_sistermon_blanc_instead_of_trashing() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(8).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let base = r.place_on_field(0, "SIS-BLANC", Some(0));
    let idx = hand_pos(&r, 0, CARD_ID);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);
    let arts_action = encode_attack(0, base.index as u16);
    // Drive the [Main] prompts (decline the free play; no opp Digimon) until
    // the optional Arts prompt naming the base.
    loop {
        let v = r.pending_selection_view().expect("selection parked before Arts");
        if v.kind == SelectionKind::OwnField && v.is_optional && v.valid_action_ids.contains(&arts_action) {
            break;
        }
        let a = if v.is_optional { PASS } else { v.valid_action_ids[0] };
        r.execute_action(v.selecting_player, a).unwrap();
    }
    let trash_before = r.trash_size(0);
    let mem = r.game.memory;
    r.execute_action(0, arts_action).unwrap();
    let perm = &r.game.player(0).battle_area[base.index as usize];
    assert_eq!(perm.top_card().card_id(&r.game.card_data), CARD_ID);
    assert_eq!(perm.stack_size(), 2);
    assert_eq!(r.trash_size(0), trash_before, "Arts replaces the trash");
    assert_eq!(r.game.memory, mem, "without paying the cost");
}
