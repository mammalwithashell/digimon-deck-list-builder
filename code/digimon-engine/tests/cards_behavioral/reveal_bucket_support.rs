//! Shared fixtures for the reveal-bucket / delay-option card tests added by the
//! DCGO PvP replay hunt (round 2, lane `reveal-bucket-cards`): BT11-046,
//! BT11-040, BT13-065, BT14-034, EX5-054, EX5-045, P-230, P-232, P-181,
//! BT21-094, BT8-110. Every board is hand-built; no recording data.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::permanent::PermanentHandle;

/// A bare synthetic card of `kind` with `colors` (play cost 2).
pub(crate) fn card(id: &str, name: &str, kind: CardKind, colors: Vec<CardColor>) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = kind;
    c.colors = colors;
    c.play_cost = 2;
    if matches!(kind, CardKind::Tamer | CardKind::Option) {
        c.level = None;
        c.dp = None;
    }
    c
}

/// A synthetic Digimon with a level, DP, play cost and trait list.
pub(crate) fn digimon(
    id: &str,
    name: &str,
    level: u8,
    dp: i32,
    play_cost: u16,
    colors: Vec<CardColor>,
    traits: &[&str],
) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = play_cost;
    c.colors = colors;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

pub(crate) fn with_evo(mut c: CardData, color: CardColor, from_level: u8, cost: u16) -> CardData {
    c.evo_costs.push(EvoCost {
        card_color: color as u8,
        level: from_level,
        memory_cost: cost,
    });
    c
}

/// Put `r` in `seat`'s Main phase with `memory` and a non-trivial turn count.
pub(crate) fn into_main(mut r: DebugRunner, seat: u8, memory: i16) -> DebugRunner {
    r.game.turn_player_idx = seat as usize;
    r.game.memory_pair = (seat, 1 - seat);
    r.game.memory = memory;
    r.game.turn_count = 3;
    r.game.current_phase = GamePhase::Main;
    r
}

fn ids_of(r: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

pub(crate) fn hand_ids(r: &DebugRunner, seat: u8) -> Vec<String> {
    ids_of(r, &r.game.players[seat as usize].hand)
}

pub(crate) fn trash_ids(r: &DebugRunner, seat: u8) -> Vec<String> {
    ids_of(r, &r.game.players[seat as usize].trash)
}

/// The deck listed TOP FIRST (the engine stores the top at the vector end).
pub(crate) fn deck_top_first(r: &DebugRunner, seat: u8) -> Vec<String> {
    let mut ids = ids_of(r, &r.game.players[seat as usize].deck);
    ids.reverse();
    ids
}

/// Security stack listed TOP FIRST.
pub(crate) fn security_top_first(r: &DebugRunner, seat: u8) -> Vec<String> {
    let mut ids = ids_of(r, &r.game.players[seat as usize].security);
    ids.reverse();
    ids
}

pub(crate) fn field_ids(r: &DebugRunner, seat: u8) -> Vec<String> {
    r.game.players[seat as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

pub(crate) fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

/// Choose the revealed card `id` in the open reveal-bucket / reveal-order
/// prompt (asserts the choice is legal first).
pub(crate) fn pick_reveal(r: &mut DebugRunner, seat: u8, id: &str) {
    let index = r
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} is not in the revealed pool"));
    let action = SEL_REVEAL_START + index as u16;
    assert!(
        r.pending_selection_view()
            .expect("a reveal prompt is open")
            .valid_action_ids
            .contains(&action),
        "revealing {id} must be a legal choice"
    );
    r.execute_action(seat, action).expect("legal reveal choice");
}

/// Reorder `deck` (top first) into the vector order the builder expects
/// (top at the end).
pub(crate) fn top_first<'a>(deck: &[&'a str]) -> Vec<&'a str> {
    deck.iter().rev().copied().collect()
}

/// Perform a legal action as `actor` (asserts the mask allows it first).
pub(crate) fn act(r: &mut DebugRunner, actor: u8, id: u16) {
    use digimon_engine::action::mask::build_action_mask;
    assert!(r.pending_selection().is_none(), "no prompt may be open");
    assert_eq!(
        build_action_mask(&r.game, actor)[id as usize],
        1.0,
        "action {id} must be legal"
    );
    r.game.decode_action(id, actor);
}

/// Digivolve `host` into `hand_card` (added to `seat`'s hand) via the real
/// digivolve action.
pub(crate) fn digivolve_via_action(
    r: &mut DebugRunner,
    seat: u8,
    hand_card: &str,
    host: PermanentHandle,
) {
    let hand = r.add_to_hand(seat, hand_card);
    let id = digimon_engine::action::space::encode_digivolve(hand as u16, host.index as u16);
    act(r, seat, id);
}

/// Push `card_id` onto the TOP of `seat`'s security stack, optionally face up.
pub(crate) fn put_in_security(r: &mut DebugRunner, seat: u8, card_id: &str, face_up: bool) {
    use digimon_engine::card_source::CardSource;
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} is not registered"));
    let next = r.game.next_card_index();
    let source = CardSource::new(idx, seat, next);
    let card_index = source.card_index;
    r.game.players[seat as usize].security.push(source);
    if face_up {
        r.game.players[seat as usize].face_up_security.insert(card_index);
    }
}
