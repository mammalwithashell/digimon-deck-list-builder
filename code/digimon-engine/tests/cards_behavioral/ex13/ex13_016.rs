//! EX13-016 Omnimon — Digimon, Lv.7, Red/White/Blue, DP 15000, Cost 15.
//! Traits: Holy Warrior / Royal Knight / CS. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-016.md`)
//!
//! ```text
//! Digivolve: Red Lv.6 / 5; Blue Lv.6 / 5; [Digivolve] Lv.6 w/[CS] trait: Cost 5
//! [DNA Digivolve] Lv.6 w/[Greymon] in name + Lv.6 w/[Garurumon] in name: Cost 0
//! Assembly -7: [WarGreymon]×[MetalGarurumon]×[Agumon]×[Gabumon]
//! <Raid> <Blocker>
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] 2 of your
//! opponent's Digimon or Tamers can't suspend until their turn ends.
//! [On Play] [When Digivolving] [Counter] [Once Per Turn] You may delete 1 of
//! your opponent's Digimon with as many digivolution cards as this Digimon or
//! fewer.
//! [All Turns] When this Digimon would leave the battle area, by trashing 2
//! same-level cards from its digivolution cards, it doesn't leave.
//! ```
//!
//! # DCGO C# reference
//! None — no `EX13_016.cs` at `b9a0638cd`.
//!
//! # Verdict — IMPLEMENTED
//! The would-leave save uses `select_materials { min: 2, max: 2, same_by:
//! level }` + `trash_selected_materials` (G-ENGINE-SAME-LEVEL-SOURCE-PAIR-
//! SELECTION, RESOLVED 2026-10-01); the replacement preflight only offers it
//! when a same-level pair exists.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use super::orphan_support::{assert_replacement_prompt, delete_with};

const CARD_ID: &str = "EX13-016";

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 3;
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-016 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("L4A", 4))
        .add_card(digimon("L4B", 4))
        .add_card(digimon("L5", 5))
        .add_card(digimon("L6", 6))
        .add_card(digimon("L6B", 6))
        .add_card(digimon("OPP", 5))
        .add_card(tamer("TAMER"))
        .add_card(digimon("FILL", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start()
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn source_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    let p = &r.game.players[h.player as usize].battle_area[h.index as usize];
    let n = p.card_sources.len() - 1;
    p.card_sources[..n]
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn cannot_suspend(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.modifiers.has(h, ModifierType::CannotSuspend)
}

/// [On Play] queues the lock AND the delete offer together: resolve the
/// simultaneous-trigger order prompt (first entry = the lock clause).
fn order_first(r: &mut DebugRunner) {
    while r.pending_kind() == Some(SelectionKind::TriggerOrder) {
        let v = r.pending_selection_view().unwrap();
        r.execute_action(v.selecting_player, v.valid_action_ids[0]).unwrap();
    }
}

fn pick(r: &mut DebugRunner, idx: usize) {
    let v = r.pending_selection_view().expect("pending");
    r.execute_action(v.selecting_player, v.valid_action_ids[idx])
        .expect("pick");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_016_identity_alt_paths_and_keywords() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(7));
    assert_eq!(card.dp, Some(15000));
    let count = |k: CompiledAltPathKind| card.alt_paths.iter().filter(|p| p.kind == k).count();
    assert_eq!(count(CompiledAltPathKind::Digivolve), 3, "red Lv.6, blue Lv.6, Lv.6 [CS]");
    assert_eq!(count(CompiledAltPathKind::DnaDigivolve), 1);
    assert_eq!(count(CompiledAltPathKind::Assembly), 1);
    let keywords: Vec<&str> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();
    assert!(keywords.contains(&"Raid") && keywords.contains(&"Blocker"), "{keywords:?}");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [OP][WD][WA][OPT] 2 opponent Digimon/Tamers can't suspend
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_016_on_play_locks_two_of_three_opponent_digimon_or_tamers() {
    let mut r = runner();
    let o1 = r.place_on_field(1, "OPP", Some(0));
    let o2 = r.place_on_field(1, "TAMER", Some(0));
    let o3 = r.place_on_field(1, "OPP", Some(0));
    let omni = r.place_stack(0, &["L6", CARD_ID]);
    r.fire_on_play(0, omni.index as usize);
    order_first(&mut r);
    // The lock is mandatory; with 3 candidates the controller picks 2.
    let v = r.pending_selection_view().expect("lock pick");
    assert_eq!(v.valid_action_ids.len(), 3, "2 Digimon + 1 Tamer offered");
    assert_eq!(v.selecting_player, 0);
    pick(&mut r, 0);
    pick(&mut r, 0);
    // Resolve whatever follows (the optional delete offer: decline).
    while let Some(v) = r.pending_selection_view() {
        if v.is_optional {
            r.decline_optional_trigger().unwrap();
        } else {
            pick(&mut r, 0);
        }
    }
    let locked = [o1, o2, o3].iter().filter(|h| cannot_suspend(&r, **h)).count();
    assert_eq!(locked, 2, "exactly 2 of the opponent's permanents can't suspend");
}

#[test]
fn ex13_016_lock_takes_every_target_when_fewer_than_two() {
    let mut r = runner();
    let o1 = r.place_on_field(1, "TAMER", Some(0));
    let omni = r.place_stack(0, &["L6", CARD_ID]);
    r.fire_on_play(0, omni.index as usize);
    while let Some(v) = r.pending_selection_view() {
        if v.is_optional {
            r.decline_optional_trigger().unwrap();
        } else {
            pick(&mut r, 0);
        }
    }
    assert!(cannot_suspend(&r, o1), "the only opponent Tamer can't suspend");
}

#[test]
fn ex13_016_lock_is_once_per_turn_across_on_play_and_when_attacking() {
    let mut r = runner();
    let o1 = r.place_on_field(1, "OPP", Some(0));
    let omni = r.place_stack(0, &["L6", CARD_ID]);
    r.fire_on_play(0, omni.index as usize);
    while let Some(v) = r.pending_selection_view() {
        if v.is_optional {
            r.decline_optional_trigger().unwrap();
        } else {
            pick(&mut r, 0);
        }
    }
    assert!(cannot_suspend(&r, o1));
    // A fresh opponent Digimon arrives; a re-fired lock would catch it.
    let o2 = r.place_on_field(1, "OPP", Some(0));
    r.attack_player(omni, 1, false);
    // [When Attacking]: no second lock this turn (OPT). Decline anything else.
    while let Some(v) = r.pending_selection_view() {
        if v.is_optional {
            r.decline_optional_trigger().unwrap();
        } else {
            pick(&mut r, 0);
        }
    }
    assert!(!cannot_suspend(&r, o2), "[Once Per Turn] — the lock didn't re-fire");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [OP][WD][Counter][OPT] may delete ≤ sources
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_016_on_play_may_delete_opponent_digimon_with_no_more_sources() {
    let mut r = runner();
    // Omnimon has 2 digivolution cards; the opponent Digimon with 3 can't be
    // chosen, the one with 2 can.
    let big = r.place_stack(1, &["FILL", "FILL", "FILL", "OPP"]);
    let small = r.place_stack(1, &["FILL", "FILL", "OPP"]);
    let omni = r.place_stack(0, &["L5", "L6", CARD_ID]);
    // Lock resolves first (mandatory); decline nothing there.
    r.fire_on_play(0, omni.index as usize);
    order_first(&mut r);
    // Lock pick (2 opp Digimon): take both.
    let v = r.pending_selection_view().expect("lock pick");
    assert!(!v.is_optional);
    pick(&mut r, 0);
    // (second pick, if still pending as a lock pick)
    if r
        .pending_selection_view()
        .is_some_and(|v| !v.is_optional && v.valid_action_ids.len() == 1)
    {
        pick(&mut r, 0);
    }
    // Optional delete offer.
    assert!(r.pending_is_optional(), "'You may delete' is offered");
    r.accept_optional_trigger().unwrap();
    let v = r.pending_selection_view().expect("delete target pick");
    assert_eq!(v.valid_action_ids.len(), 1, "only the ≤2-source Digimon is eligible");
    pick(&mut r, 0);
    let _ = r.auto_resolve();
    let ids: Vec<String> = r.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.card_sources.len().to_string())
        .collect();
    assert_eq!(ids, vec!["4"], "the 3-source Digimon stays; the 2-source one was deleted");
    let _ = (big, small);
}

#[test]
fn ex13_016_delete_not_offered_without_an_eligible_target() {
    let mut r = runner();
    let _big = r.place_stack(1, &["FILL", "FILL", "OPP"]);
    let omni = r.place_stack(0, &["L6", CARD_ID]);
    r.fire_on_play(0, omni.index as usize);
    // The lock picks its single target; no optional delete prompt follows.
    while let Some(v) = r.pending_selection_view() {
        assert!(
            !v.is_optional,
            "no 'may delete' offer: the opponent Digimon has more sources than Omnimon"
        );
        pick(&mut r, 0);
    }
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — [All Turns] would-leave save (same-level pair)
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_016_save_trashes_a_same_level_pair_and_stays() {
    let mut r = runner();
    // Sources (bottom→top): L4A, L5, L4B — the only same-level pair is 4/4.
    let omni = r.place_stack(0, &["L4A", "L5", "L4B", CARD_ID]);
    delete_with(&mut r, omni, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    r.accept_optional_trigger().expect("accept the save");
    let v = r.pending_selection_view().expect("first pair pick");
    assert_eq!(v.valid_action_ids.len(), 2, "only the two level-4 cards start a pair");
    assert!(!v.is_optional, "exactly 2 must be trashed");
    pick(&mut r, 0);
    let v = r.pending_selection_view().expect("second pair pick");
    assert_eq!(v.valid_action_ids.len(), 1, "only the other level-4 card");
    pick(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.game.players[0].battle_area.len(), 1, "it doesn't leave");
    assert_eq!(top_id(&r, omni), CARD_ID);
    assert_eq!(source_ids(&r, omni), vec!["L5"], "the level-4 pair was trashed");
    let trash: Vec<String> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert!(trash.contains(&"L4A".to_string()) && trash.contains(&"L4B".to_string()));
}

#[test]
fn ex13_016_save_not_offered_without_a_same_level_pair() {
    let mut r = runner();
    // Levels 4, 5, 6 + a level-less Tamer card: no pair → no offer.
    let omni = r.place_stack(0, &["L4A", "TAMER", "L5", "L6", CARD_ID]);
    delete_with(&mut r, omni, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area.is_empty(), "deleted — the save was unpayable");
}

#[test]
fn ex13_016_save_decline_lets_it_leave() {
    let mut r = runner();
    let omni = r.place_stack(0, &["L6", "L6B", CARD_ID]);
    delete_with(&mut r, omni, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area.is_empty());
}

#[test]
fn ex13_016_save_is_not_once_per_turn() {
    let mut r = runner();
    let omni = r.place_stack(0, &["L4A", "L4B", "L6", "L6B", CARD_ID]);
    for round in 0..2 {
        delete_with(&mut r, omni, ReplacementCause::OpponentEffect);
        assert_replacement_prompt(&r);
        r.accept_optional_trigger().unwrap();
        pick(&mut r, 0);
        pick(&mut r, 0);
        let _ = r.auto_resolve();
        assert_eq!(r.game.players[0].battle_area.len(), 1, "saved in round {round}");
    }
    assert!(source_ids(&r, omni).is_empty(), "both pairs were spent");
    // No pair left → the third removal goes through.
    delete_with(&mut r, omni, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area.is_empty());
}
