//! Shared fixtures for the BT26 DATA SQUAD (Rosemon / Ravemon) card tests.

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

pub fn digimon(
    id: &str,
    name: &str,
    color: CardColor,
    level: u8,
    cost: u16,
    traits: &[&str],
) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

pub fn tamer(id: &str, name: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

pub fn filler(id: &str) -> CardData {
    digimon(id, id, CardColor::Red, 3, 3, &[])
}

fn data_idx(r: &DebugRunner, card_id: &str) -> usize {
    r.game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"))
}

pub fn push_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = data_idx(r, card_id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .hand
        .push(CardSource::new(idx, p, next));
}

pub fn push_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = data_idx(r, card_id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .trash
        .push(CardSource::new(idx, p, next));
}

/// A Tamer (top = `tamer_id`) carrying `n` face-down FILLER sources.
pub fn tamer_with_face_down(
    r: &mut DebugRunner,
    p: u8,
    tamer_id: &str,
    n: usize,
) -> PermanentHandle {
    let mut ids: Vec<&str> = vec!["FILLER"; n];
    ids.push(tamer_id);
    let t = r.place_stack(p, &ids);
    let stack = &mut r.game.players[p as usize].battle_area[t.index as usize].card_sources;
    let top = stack.len() - 1;
    for s in stack[..top].iter_mut() {
        s.face_down = true;
    }
    t
}

pub fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

/// Digivolution-card count under a permanent (the stack minus its top card).
pub fn sources(r: &DebugRunner, h: PermanentHandle) -> usize {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .len()
        - 1
}

pub fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn deck_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .deck
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

/// Answer the current pending selection with the valid action that targets
/// the reveal/hand card whose id is `card_id` — falls back to panicking with
/// the prompt so a wrong-shape prompt fails loudly.
pub fn pick_first(r: &mut DebugRunner, p: u8) {
    let v = r.pending_selection_view().expect("a pending selection");
    r.execute_action(p, v.valid_action_ids[0]).unwrap();
}

pub fn pass(r: &mut DebugRunner, p: u8) {
    r.execute_action(p, digimon_engine::action::space::PASS)
        .expect("PASS legal");
}

/// Answer a reveal selection by picking the revealed card whose id is `card_id`.
pub fn pick_reveal(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not revealed"));
    let action = digimon_engine::action::space::SEL_REVEAL_START + idx as u16;
    let v = r
        .pending_selection_view()
        .expect("reveal selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{card_id} is not a legal pick here (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(p, action).unwrap();
}

/// Answer a hand selection by picking the hand card whose id is `card_id`.
pub fn pick_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"));
    let action = digimon_engine::action::space::PLAY_HAND_START + idx as u16;
    let v = r.pending_selection_view().expect("hand selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{card_id} is not a legal hand pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(p, action).unwrap();
}
