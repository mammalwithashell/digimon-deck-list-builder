//! Shared fixtures for the EX13 "Chronicle" slice tests
//! (EX13-006 Dorimon, EX13-049 Dorumon, EX13-055 Raptordramon,
//! EX13-057 Grademon, EX13-072 Kota Domoto).

#![allow(dead_code)]

use digimon_engine::action::space::{
    ATTACK_START, HAND_EFFECT_START, PASS, PLAY_HAND_START, SEL_REVEAL_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
pub(super) use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

pub(super) fn color_id(c: CardColor) -> u8 {
    c as u8
}

/// Synthetic Digimon fixture.
pub(super) fn digimon(
    id: &str,
    color: CardColor,
    level: u8,
    dp: i32,
    traits: &[&str],
) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.colors = vec![color];
    c.dp = Some(dp);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Adds a printed digivolve circle `from_color` Lv.`from_level` / `cost`.
pub(super) fn with_evo(mut c: CardData, from_color: CardColor, from_level: u8, cost: u8) -> CardData {
    c.evo_costs.push(EvoCost {
        card_color: color_id(from_color),
        level: from_level,
        memory_cost: cost.into(),
    });
    c
}

/// Synthetic Option fixture (no effect text).
pub(super) fn option(id: &str, name: &str, color: CardColor, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

pub(super) fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

pub(super) fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

pub(super) fn deck_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

pub(super) fn hand_index(runner: &DebugRunner, player: u8, id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in player {player}'s hand"))
}

pub(super) fn trash_index(runner: &DebugRunner, player: u8, id: &str) -> usize {
    runner.game.players[player as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in player {player}'s trash"))
}

pub(super) fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

pub(super) fn suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

pub(super) fn set_suspended(runner: &mut DebugRunner, h: PermanentHandle, value: bool) {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = value;
}

pub(super) fn field_has(runner: &DebugRunner, player: u8, id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == id)
}

/// Push `ids` onto the top of `player`'s deck (the LAST id ends on top).
pub(super) fn stack_deck_top(runner: &mut DebugRunner, player: u8, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered"));
        let card_index = runner.game.next_card_index();
        runner.game.players[player as usize]
            .deck
            .push(CardSource::new(data_idx, player, card_index));
    }
}

pub(super) fn revealed_ids(runner: &DebugRunner) -> Vec<String> {
    runner
        .game
        .revealed_cards
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Select the revealed card `id` at a Reveal prompt.
pub(super) fn pick_revealed(runner: &mut DebugRunner, id: &str) {
    let view = runner.pending_selection_view().expect("reveal pick prompt");
    assert_eq!(view.kind, SelectionKind::Reveal, "{view:?}");
    let pos = revealed_ids(runner)
        .iter()
        .position(|r| r == id)
        .unwrap_or_else(|| panic!("{id} must be revealed"));
    let action = SEL_REVEAL_START + pos as u16;
    assert!(view.valid_action_ids.contains(&action), "{id} must be pickable: {view:?}");
    runner.execute_action(view.selecting_player, action).expect("pick revealed");
}

/// Is revealed card `id` a legal pick at the current Reveal prompt?
pub(super) fn reveal_offers(runner: &DebugRunner, id: &str) -> bool {
    let Some(view) = runner.pending_selection_view() else {
        return false;
    };
    if view.kind != SelectionKind::Reveal {
        return false;
    }
    match revealed_ids(runner).iter().position(|r| r == id) {
        Some(pos) => view.valid_action_ids.contains(&(SEL_REVEAL_START + pos as u16)),
        None => false,
    }
}

/// Answer the remaining OrderedPermutation prompts with their first legal action.
pub(super) fn finish_ordering(runner: &mut DebugRunner) {
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        if !matches!(view.kind, SelectionKind::OrderedPermutation { .. }) {
            return;
        }
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("order remainder");
    }
}

/// Pick 1 of `player`-relative field permanent `h` at an OwnField / OppField prompt.
pub(super) fn pick_field(runner: &mut DebugRunner, h: PermanentHandle) {
    let view = runner.pending_selection_view().expect("field prompt");
    assert!(
        matches!(
            view.kind,
            SelectionKind::OwnField | SelectionKind::OppField | SelectionKind::AnyField
        ),
        "{view:?}"
    );
    let action = ATTACK_START + h.index as u16;
    assert!(view.valid_action_ids.contains(&action), "{h:?} selectable: {view:?}");
    runner.execute_action(view.selecting_player, action).expect("pick field permanent");
}

pub(super) fn field_offered(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner
        .pending_selection_view()
        .map(|v| v.valid_action_ids.contains(&(ATTACK_START + h.index as u16)))
        .unwrap_or(false)
}

/// Pick hand card `id` at a Hand / UnionZone prompt.
pub(super) fn pick_hand(runner: &mut DebugRunner, player: u8, id: &str) {
    let view = runner.pending_selection_view().expect("hand prompt");
    let slot = hand_index(runner, player, id) as u16;
    let action = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} must be selectable from hand: {view:?}"));
    runner.execute_action(view.selecting_player, action).expect("pick hand");
}

pub(super) fn hand_offered(runner: &DebugRunner, player: u8, id: &str) -> bool {
    let Some(view) = runner.pending_selection_view() else {
        return false;
    };
    let Some(slot) = runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
    else {
        return false;
    };
    let slot = slot as u16;
    view.valid_action_ids.contains(&(PLAY_HAND_START + slot))
        || view.valid_action_ids.contains(&(HAND_EFFECT_START + slot))
}

/// Pick trash card `id` at a UnionZone prompt.
pub(super) fn pick_trash(runner: &mut DebugRunner, player: u8, id: &str) {
    let view = runner.pending_selection_view().expect("trash/union prompt");
    let action = TRASH_EFFECT_START + trash_index(runner, player, id) as u16;
    assert!(view.valid_action_ids.contains(&action), "{id} selectable from trash: {view:?}");
    runner.execute_action(view.selecting_player, action).expect("pick trash");
}

pub(super) fn trash_offered(runner: &DebugRunner, player: u8, id: &str) -> bool {
    let Some(view) = runner.pending_selection_view() else {
        return false;
    };
    match runner.game.players[player as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
    {
        Some(i) => view.valid_action_ids.contains(&(TRASH_EFFECT_START + i as u16)),
        None => false,
    }
}

/// Accept the current optional prompt with its first non-PASS action.
pub(super) fn accept(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("optional prompt pending");
    assert!(view.is_optional, "expected an optional prompt: {view:?}");
    let a = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .unwrap_or_else(|| panic!("no accept action: {view:?}"));
    runner.execute_action(view.selecting_player, a).expect("accept");
}

/// Decline (PASS) the current prompt.
pub(super) fn decline(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("prompt pending");
    assert!(view.is_optional, "expected a declinable prompt: {view:?}");
    runner.execute_action(view.selecting_player, PASS).expect("decline");
}

/// Drain every prompt by PASSing when legal, else the first legal action.
pub(super) fn decline_all(runner: &mut DebugRunner) {
    for _ in 0..64 {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        let action = if view.is_optional || view.valid_action_ids.contains(&PASS) {
            PASS
        } else {
            *view.valid_action_ids.first().expect("legal action")
        };
        runner.execute_action(view.selecting_player, action).expect("resolve");
    }
    panic!("decline_all did not converge");
}

pub(super) fn fire(runner: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    runner.game.enqueue_triggered(timing, TriggerSource::Permanent(h));
    runner.game.drain_effect_queue();
}

/// End the current turn so the NEXT player starts with 3 memory (the gauge is
/// set to 3 on the opponent's side first), then drain any prompts.
pub(super) fn next_turn(runner: &mut DebugRunner) {
    runner.game.memory = -3;
    runner.end_turn();
    decline_all(runner);
}

/// Digivolve hand card `hand_slot` onto `base`. When several routes with
/// different costs apply, the engine prompts (rule 17 — no auto-pick); answer
/// it with the route whose label names `cost`. Returns whether a cost-choice
/// prompt was shown.
pub(super) fn digivolve_choosing_cost(
    runner: &mut DebugRunner,
    hand_slot: u16,
    base: PermanentHandle,
    cost: u16,
) -> bool {
    runner
        .game
        .decode_action(digimon_engine::action::space::encode_digivolve(hand_slot, base.index as u16), 0);
    let Some(view) = runner.pending_selection_view() else {
        return false;
    };
    // A non-EffectChoice prompt is a post-digivolve trigger, not a route pick.
    if view.kind != SelectionKind::EffectChoice || view.effect_choices.is_none() {
        return false;
    }
    let choices = view.effect_choices.clone().expect("cost options");
    let pick = choices
        .iter()
        .find(|c| c.label.contains(&format!("{cost}")))
        .unwrap_or_else(|| {
            panic!(
                "no cost-{cost} route offered: {:?}",
                choices.iter().map(|c| c.label.clone()).collect::<Vec<_>>()
            )
        });
    runner.execute_action(view.selecting_player, pick.action_id).expect("pick route");
    true
}
