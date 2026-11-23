//! Shared fixtures for the BT27 card-text tests (real actions, explicit prompts).
#![allow(dead_code)]

use digimon_engine::action::space::{encode_attack, SECURITY_TARGET, SEL_REVEAL_START};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

pub fn mon(id: &str, name: &str, level: u8, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c.colors = vec![CardColor::Blue];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

pub fn evo(id: &str, name: &str, level: u8, cost: u16, evo_cost: u16) -> CardData {
    let mut c = mon(id, name, level, cost, &[]);
    c.evo_costs = [0u8, 1, 2, 3, 4, 5, 6]
        .into_iter()
        .map(|card_color| EvoCost { card_color, level: level - 1, memory_cost: evo_cost })
        .collect();
    c
}

pub fn tamer(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

pub fn hand_index(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in hand"))
}

pub fn play_hand(r: &mut DebugRunner, id: &str) {
    let i = hand_index(r, 0, id);
    assert!(r.play(0, i).is_some(), "{id} should be playable");
}

pub fn ids_of<'a>(r: &DebugRunner, cards: impl IntoIterator<Item = &'a digimon_engine::card_source::CardSource>) -> Vec<String> {
    cards.into_iter().map(|c| c.card_id(&r.game.card_data).to_string()).collect()
}

pub fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids_of(r, &r.game.players[p as usize].hand)
}

pub fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids_of(r, &r.game.players[p as usize].trash)
}

/// Digivolution cards (bottom first), excluding the top card.
pub fn source_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    let s = &r.game.players[h.player as usize].battle_area[h.index as usize].card_sources;
    ids_of(r, &s[..s.len() - 1])
}

pub fn face_down(r: &DebugRunner, h: PermanentHandle) -> Vec<bool> {
    let s = &r.game.players[h.player as usize].battle_area[h.index as usize].card_sources;
    s[..s.len() - 1].iter().map(|c| c.face_down).collect()
}

pub fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

pub fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|x| x.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

pub fn legal(r: &DebugRunner, a: u16) -> bool {
    let p = r.pending_selection_view().map(|v| v.selecting_player).unwrap_or(0);
    build_action_mask(&r.game, p)[a as usize] > 0.5
}

/// Attack an opposing suspended Digimon with `attacker` (index on player 0's field).
pub fn attack(r: &mut DebugRunner, attacker: u8, target: PermanentHandle) {
    r.game.players[1].battle_area[target.index as usize].is_suspended = true;
    let a = encode_attack(attacker as u16, target.index as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.5, "attack illegal");
    r.game.decode_action(a, 0);
}

pub fn reveal_action(r: &DebugRunner, id: &str) -> u16 {
    SEL_REVEAL_START
        + r.game
            .revealed_cards
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == id)
            .unwrap_or_else(|| panic!("{id} not revealed")) as u16
}

pub fn pick_reveal(r: &mut DebugRunner, id: &str) {
    let a = reveal_action(r, id);
    let v = r.pending_selection_view().expect("reveal pick pending");
    assert!(v.valid_action_ids.contains(&a), "{id} not pickable: {:?}", v.valid_action_ids);
    r.execute_action(v.selecting_player, a).unwrap();
}

/// Answer the pending effect choice with branch `i` (in printed label order).
pub fn choose_branch(r: &mut DebugRunner, i: usize) {
    r.execute_branch(i).unwrap();
}

/// Attack the opponent's security with `attacker` (index on player 0's field),
/// readying it first so repeat attacks in one turn exercise Once Per Turn.
pub fn attack_security(r: &mut DebugRunner, attacker: u8) {
    r.set_phase(digimon_engine::enums::GamePhase::Main);
    r.game.players[0].battle_area[attacker as usize].is_suspended = false;
    let a = encode_attack(attacker as u16, SECURITY_TARGET);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.5, "security attack illegal");
    r.game.decode_action(a, 0);
}

/// Digivolve hand card `id` onto `target` (player 0) through the real action.
pub fn digivolve_hand(r: &mut DebugRunner, id: &str, target: PermanentHandle) -> bool {
    let a = digimon_engine::action::space::encode_digivolve(hand_index(r, 0, id) as u16, target.index as u16);
    if build_action_mask(&r.game, 0)[a as usize] <= 0.5 {
        return false;
    }
    r.game.decode_action(a, 0);
    true
}


/// Attack a suspended 99000 DP opposing wall so `attacker` loses the battle.
pub fn lose_battle(r: &mut DebugRunner, attacker: u8) -> PermanentHandle {
    let wall = r.place_on_field(1, "WALL", Some(0));
    attack(r, attacker, wall);
    wall
}

pub fn wall() -> CardData {
    let mut c = mon("WALL", "Wall", 7, 12, &[]);
    c.dp = Some(99000);
    c
}

/// Return `mine` to hand by an opposing effect sourced from `opp_src`.
pub fn opponent_bounces(r: &mut DebugRunner, opp_src: PermanentHandle, mine: PermanentHandle) {
    use digimon_engine::effect_context::EffectContext;
    let card = r.top_card(opp_src);
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, card, Some(opp_src), 1);
        let _ = ctx.return_to_hand(mine);
    }
    r.game.set_effect_source_player_for_test(None);
    r.game.drain_effect_queue();
}
