//! Shared fixtures for the EX13 "orphan staples" slice tests
//! (EX13-002, 004, 005, 009, 010, 011, 012, 013, 024, 034, 042, 046, 047,
//! 051, 052, 056, 058, 070, 073, 075, 076).

#![allow(dead_code)]

use digimon_engine::action::space::{
    ATTACK_START, HAND_EFFECT_START, PASS, PLAY_HAND_START, SEL_REVEAL_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
pub(super) use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

/// Synthetic Digimon fixture (play cost 3).
pub(super) fn digimon(id: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.colors = vec![color];
    c.dp = Some(dp);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Same as [`digimon`] with an explicit name.
pub(super) fn named(id: &str, name: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut c = digimon(id, color, level, dp, &[]);
    c.card_name = name.to_string();
    c
}

pub(super) fn with_cost(mut c: CardData, cost: u16) -> CardData {
    c.play_cost = cost;
    c
}

pub(super) fn with_text(mut c: CardData, text: &str) -> CardData {
    c.effect_text = text.to_string();
    c
}

/// Adds a printed digivolve circle `from_color` Lv.`from_level` / `cost`.
pub(super) fn with_evo(mut c: CardData, from_color: CardColor, from_level: u8, cost: u8) -> CardData {
    c.evo_costs.push(EvoCost {
        card_color: from_color as u8,
        level: from_level,
        memory_cost: cost.into(),
    });
    c
}

/// Synthetic Tamer fixture.
pub(super) fn tamer(id: &str, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c.play_cost = cost;
    c
}

/// Synthetic Option fixture (no effect text).
pub(super) fn option(id: &str, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c.play_cost = cost;
    c
}

pub(super) fn ids(r: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

pub(super) fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

pub(super) fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].trash)
}

pub(super) fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

pub(super) fn handle_of(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let i = field_ids(r, p)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} on player {p}'s field: {:?}", field_ids(r, p)));
    PermanentHandle {
        player: p,
        index: i as u8,
    }
}

pub(super) fn try_handle_of(r: &DebugRunner, p: u8, id: &str) -> Option<PermanentHandle> {
    field_ids(r, p).iter().position(|c| c == id).map(|i| PermanentHandle {
        player: p,
        index: i as u8,
    })
}

pub(super) fn is_suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

pub(super) fn set_suspended(r: &mut DebugRunner, h: PermanentHandle, v: bool) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = v;
}

pub(super) fn source_count(r: &DebugRunner, h: PermanentHandle) -> usize {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .len()
        .saturating_sub(1)
}

pub(super) fn hand_index(r: &DebugRunner, p: u8, id: &str) -> usize {
    hand_ids(r, p)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} in player {p}'s hand: {:?}", hand_ids(r, p)))
}

/// Play `id` from player `p`'s hand through the normal play action.
pub(super) fn play_card(r: &mut DebugRunner, p: u8, id: &str) {
    let slot = hand_index(r, p, id);
    assert!(r.play(p, slot).is_some(), "{id} should be playable");
}

/// Legal non-PASS action ids at the pending prompt.
pub(super) fn legal(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default()
}

/// Ids of hand cards offered at a hand-pick prompt (any of the hand ranges).
pub(super) fn offered_hand_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("hand prompt pending");
    let p = view.selecting_player;
    let hand = hand_ids(r, p);
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| {
            [PLAY_HAND_START, HAND_EFFECT_START]
                .iter()
                .filter_map(|&s| a.checked_sub(s))
                .find(|&i| (i as usize) < hand.len() && i < 30)
        })
        .map(|i| hand[i as usize].clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Pick hand card `id` at a hand-pick prompt.
pub(super) fn pick_hand(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("hand prompt pending");
    let p = view.selecting_player;
    let slot = hand_index(r, p, id) as u16;
    let a = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} not selectable: {view:?}"));
    r.execute_action(p, a).expect("pick hand card");
}

/// Pick trash card `id` at a trash prompt.
pub(super) fn pick_trash(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("trash prompt pending");
    let p = view.selecting_player;
    let trash = trash_ids(r, p);
    let a = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(TRASH_EFFECT_START)
                .and_then(|i| trash.get(i as usize))
                .is_some_and(|c| c == id)
        })
        .unwrap_or_else(|| panic!("{id} not selectable in trash: {view:?}"));
    r.execute_action(p, a).expect("pick trash card");
}

pub(super) fn offered_reveal_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(SEL_REVEAL_START))
        .filter_map(|i| r.game.revealed_cards.get(i as usize))
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    out.sort();
    out
}

pub(super) fn pick_revealed(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let want = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(SEL_REVEAL_START)
                .and_then(|i| r.game.revealed_cards.get(i as usize))
                .is_some_and(|c| c.card_id(&r.game.card_data) == id)
        })
        .unwrap_or_else(|| panic!("{id} must be a legal reveal pick: {view:?}"));
    r.execute_action(view.selecting_player, want)
        .expect("pick revealed card");
}

/// Accept a pending yes/no optional-trigger prompt (first non-PASS action).
pub(super) fn accept(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("optional prompt pending");
    let a = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("an accept action");
    r.execute_action(view.selecting_player, a).expect("accept");
}

pub(super) fn decline(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("optional prompt pending");
    r.execute_action(view.selecting_player, PASS).expect("decline");
}

/// Deck with `top` on top (index 0 = first revealed) over `filler` fillers.
pub(super) fn deck_with_top<'a>(top: &[&'a str], filler: &'a str, n_filler: usize) -> Vec<&'a str> {
    let mut deck: Vec<&str> = vec![filler; n_filler];
    for c in top.iter().rev() {
        deck.push(c);
    }
    deck
}

pub(super) fn deck_bottom_ids(r: &DebugRunner, p: u8, n: usize) -> Vec<String> {
    ids(r, &r.game.players[p as usize].deck[..n])
}

/// Pick field permanent `h` at an Own/Opp/AnyField prompt.
pub(super) fn pick_field(r: &mut DebugRunner, h: PermanentHandle) {
    let view = r.pending_selection_view().expect("field prompt pending");
    let a = ATTACK_START + h.index as u16;
    assert!(view.valid_action_ids.contains(&a), "{h:?} not selectable: {view:?}");
    r.execute_action(view.selecting_player, a).expect("pick field permanent");
}

/// Whether field permanent `h` is offered at the pending field prompt.
pub(super) fn field_offered(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.pending_selection_view()
        .is_some_and(|v| v.valid_action_ids.contains(&(ATTACK_START + h.index as u16)))
}

/// Delete `h` as if by an opponent's effect and drain the trigger queue.
pub(super) fn delete_by_opponent_effect(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .delete_permanent_with_cause(h, digimon_engine::replacement::ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
}

/// Top card id of field permanent `h`.
pub(super) fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

/// Delete `h` with an explicit cause (no drain).
pub(super) fn delete_with(r: &mut DebugRunner, h: PermanentHandle, cause: digimon_engine::replacement::ReplacementCause) {
    r.game.delete_permanent_with_cause(h, cause);
}

/// Assert a Replacement accept/decline prompt is pending.
pub(super) fn assert_replacement_prompt(r: &DebugRunner) {
    let v = r.pending_selection_view().expect("replacement prompt pending");
    assert_eq!(v.kind, digimon_engine::selection::SelectionKind::Replacement, "{v:?}");
    assert!(v.is_optional, "replacement prompts are optional");
}

/// End the current turn, declining every optional prompt along the way (e.g.
/// <Engage> / end-of-turn offers) and auto-resolving mandatory ones.
pub(super) fn end_turn_declining(r: &mut DebugRunner) {
    let before = r.turn_player();
    r.end_turn();
    for _ in 0..32 {
        if r.pending_selection().is_none() {
            if r.turn_player() == before {
                // e.g. the <Engage> end-of-turn attack window: pass it.
                r.game.decode_action(PASS, before);
                continue;
            }
            break;
        }
        if r.pending_is_optional() {
            decline(r);
        } else {
            let _ = r.auto_resolve();
        }
    }
    assert_ne!(r.turn_player(), before, "turn passed");
}

/// Trash `owner`'s top security card by an effect controlled by `actor`, then
/// drain the queue (fires `on_own_security_removed` observers).
pub(super) fn trash_security_by(r: &mut DebugRunner, owner: u8, actor: u8) {
    let by = r.game.players[owner as usize].security[0].handle();
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, by, None, actor);
        assert!(ctx.trash_top_security(owner), "security card trashed");
    }
    r.game.drain_effect_queue();
}

/// Pass to the opponent, then end their turn leaving `my_memory` on my side,
/// so player 0's next turn starts with that memory (before start-of-turn
/// effects). Optional prompts along the way are declined.
pub(super) fn cycle_to_my_turn_with_memory(r: &mut DebugRunner, my_memory: i16) {
    end_turn_declining(r);
    // Opponent's perspective: my +N is their −N.
    r.game.memory = -my_memory;
    r.end_turn();
}
