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

/// Multi-colour Digimon test card (BT26 Iliad slice).
pub fn digimon_colors(
    id: &str,
    level: u8,
    cost: u16,
    colors: &[CardColor],
    traits: &[&str],
) -> CardData {
    let mut c = digimon(id, id, colors[0], level, cost, traits);
    c.colors = colors.to_vec();
    c
}

pub fn security_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .security
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

/// Test-only [On Play] "return 1 card from your trash to the bottom of the
/// deck" Digimon — an own-effect deck add that fires `on_add_to_deck`
/// (G-ENGINE-ON-ADD-TO-DECK). Fire it with `fire(r, OnPlay, handle)`.
pub const DECKER_YAML: &str = r#"
card: T-DECKER
name: Decker
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Return 1 card from your trash to the bottom of the deck"
    process:
      - select_trash:
          of: you
          bind_as: back
          filter: {}
          prompt: "Return 1 card from your trash to the bottom of the deck"
      - return_trash_list_to_deck_bottom: { of: you, cards: back }
"#;

/// Test-only [On Play] "reveal the top card, put it back at the bottom" —
/// a reveal round-trip that must NOT fire `on_add_to_deck`.
pub const REVEALER_YAML: &str = r#"
card: T-REVEALER
name: Revealer
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Reveal the top card of your deck; place it at the bottom"
    process:
      - reveal_top_deck: { of: you, count: 1 }
      - place_remainder_on_deck: { of: you, position: bottom }
"#;

/// Auto-answer every pending selection with its first non-PASS action.
pub fn drain_first(r: &mut DebugRunner) {
    while let Some(v) = r.pending_selection_view() {
        let a = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != digimon_engine::action::space::PASS)
            .unwrap_or(digimon_engine::action::space::PASS);
        r.execute_action(v.selecting_player, a).unwrap();
    }
}

/// Test-only [On Play] "trash your top security card" Digimon — an
/// EFFECT-caused security removal (Glowing Dawn slice: BT26-089 Kyo's
/// "if removed from by effects" branch). Fire it with `fire(r, OnPlay, h)`.
pub const SEC_TRASHER_YAML: &str = r#"
card: T-SECTRASH
name: SecTrasher
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Trash your top security card"
    process:
      - trash_top_security: { of: you }
"#;

/// Facedown-ness of each digivolution card under a permanent, bottom first.
pub fn face_down_flags(r: &DebugRunner, h: PermanentHandle) -> Vec<bool> {
    let stack = &r.game.players[h.player as usize].battle_area[h.index as usize].card_sources;
    stack[..stack.len() - 1]
        .iter()
        .map(|s| s.face_down)
        .collect()
}

/// Card ids of the digivolution cards under a permanent, bottom first.
pub fn source_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    let stack = &r.game.players[h.player as usize].battle_area[h.index as usize].card_sources;
    stack[..stack.len() - 1]
        .iter()
        .map(|s| s.card_id(&r.game.card_data).to_string())
        .collect()
}

/// Answer a trash selection by picking `p`'s trash card whose id is `card_id`
/// (`TRASH_EFFECT_START + trash index`).
pub fn pick_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r.game.players[p as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in trash"));
    let action = digimon_engine::action::space::TRASH_EFFECT_START + idx as u16;
    let v = r.pending_selection_view().expect("trash selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{card_id} is not a legal trash pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(p, action).unwrap();
}

/// Non-PASS valid actions of the current pending selection.
pub fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| {
            v.valid_action_ids
                .into_iter()
                .filter(|&a| a != digimon_engine::action::space::PASS)
                .collect()
        })
        .unwrap_or_default()
}

/// A vanilla Yellow [Glowing Dawn] Option card with use cost `cost`
/// (Glowing Dawn slice — "use 1 [Glowing Dawn] Option ... cost reduced").
pub fn gd_option(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = digimon_engine::enums::CardKind::Option;
    c.colors = vec![CardColor::Yellow];
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// Test-only [On Play] "trash 1 card in your hand" Digimon — an own-effect
/// hand trash that fires `on_discard_hand` (Titan slice: BT26-066 / 069 /
/// 059 "when your hand is trashed from"). Fire it with `fire(r, OnPlay, h)`.
pub const HAND_TRASHER_YAML: &str = r#"
card: T-HANDTRASH
name: HandTrasher
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Trash 1 card in your hand"
    process:
      - select_hand:
          of: you
          bind_as: gone
          filter: {}
          prompt: "Trash 1 card in your hand"
      - trash_from_hand_by_index: { of: you, hand_index: gone }
"#;

/// The card id on top of permanent `h`.
pub fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

/// A Digimon test card with one printed digivolve circle (`color` Lv.`from_level` / `evo_cost`).
pub fn digimon_evo(
    id: &str,
    name: &str,
    color: CardColor,
    level: u8,
    cost: u16,
    traits: &[&str],
    from_level: u8,
    evo_cost: u16,
) -> CardData {
    let mut c = digimon(id, name, color, level, cost, traits);
    c.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: color as u8,
        level: from_level,
        memory_cost: evo_cost,
    }];
    c
}

/// Test-only Yellow [Glowing Dawn] Option (use cost 5): "[Main] <Draw 1>" —
/// observable proof the Option was USED (Glowing Dawn slice).
pub const GD_DRAW_OPTION_YAML: &str = r#"
card: T-GDOPT
name: GD Draw Option
kind: option
color: [yellow]
cost: 5
traits: [Glowing Dawn]
effects:
  - when: main
    summary: "[Main] <Draw 1>"
    process:
      - draw: { of: you, count: 1 }
"#;

/// Answer an `AnyField` (both battle areas) permanent selection by picking
/// `h` — encoded `encode_attack(h.player, h.index)`.
pub fn pick_any_field(r: &mut DebugRunner, selector: u8, h: PermanentHandle) {
    let action = digimon_engine::action::space::encode_attack(h.player as u16, h.index as u16);
    let v = r.pending_selection_view().expect("field selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{h:?} is not a legal pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(selector, action).unwrap();
}

/// Test-only purple [Titan] Option, use cost 4: "[Main] <Draw 1>"
/// (Titan slice — BT26-074 "use 1 [Titan] Option from your trash").
pub const TITAN_OPTION_YAML: &str = r#"
card: T-TITANOPT
name: Titan Art
kind: option
color: [purple]
cost: 4
traits: [Titan]
effects:
  - when: main_from_hand
    summary: "[Main] <Draw 1>"
    process:
      - draw: { of: you, count: 1 }
"#;

/// Answer an `OwnField` / `OppField` permanent selection by picking `h` —
/// encoded `encode_attack(0, h.index)` (the side is implied by the kind).
pub fn pick_side_field(r: &mut DebugRunner, selector: u8, h: PermanentHandle) {
    let action = digimon_engine::action::space::encode_attack(0, h.index as u16);
    let v = r.pending_selection_view().expect("field selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{h:?} is not a legal pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(selector, action).unwrap();
}

// ── Appmon / Seven Code slice helpers (BT26 Appmon Dantemon slice) ──────────

/// Declare the §6-5-1-4 from-hand `<Link>` of `p`'s hand card `card_id`
/// (the `HAND_EFFECT` bit), then answer the host prompt with its first legal
/// host. Leaves any follow-on (e.g. `[When Linking]`) prompt pending.
pub fn link_from_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"));
    assert!(
        r.game.hand_link_available(p, idx),
        "{card_id} has no legal from-hand link"
    );
    let action = digimon_engine::action::space::HAND_EFFECT_START + idx as u16;
    r.game.decode_action(action, p);
    if r.pending_selection().is_some() {
        pick_first(r, p);
    }
}

/// Link ids attached to `h`, in order.
pub fn linked_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .linked_cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

/// Push `card_id` (owned by the host's controller) as a link card onto `host`
/// and dispatch `OnLink` for both players, then drain — fires the host's
/// `when_card_linked_to_this` / "when this Digimon gets linked" triggers
/// (BT25-052 `fire_link_onto_host` idiom). Returns the linked card handle.
pub fn push_link_and_fire(
    r: &mut DebugRunner,
    host: PermanentHandle,
    card_id: &str,
) -> digimon_engine::card_source::CardHandle {
    let linked = r.push_linked_owned(host, card_id, host.player);
    for pid in 0..2u8 {
        r.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid,
                host,
                card: linked,
            },
        );
    }
    r.game.drain_effect_queue();
    linked
}

/// Push `ids` onto `p`'s deck in order — the LAST id ends up on top.
pub fn stack_deck_top(r: &mut DebugRunner, p: u8, ids: &[&str]) {
    for id in ids {
        let idx = data_idx(r, id);
        let next = r.game.next_card_index();
        r.game.players[p as usize]
            .deck
            .push(CardSource::new(idx, p, next));
    }
}

/// Put `card_id` on TOP of `host`'s stack (a cost-free stand-in for digivolving
/// onto the SAME permanent, so per-permanent state such as once-per-turn
/// counters carries over). No triggers fire.
pub fn put_on_top(r: &mut DebugRunner, host: PermanentHandle, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("put_on_top: unknown card_id {card_id}"));
    let next_idx = r.game.next_card_index();
    let card = CardSource::new(data_idx, host.player, next_idx);
    r.game.player_mut(host.player).battle_area[host.index as usize]
        .card_sources
        .push(card);
}
