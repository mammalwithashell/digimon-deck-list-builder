//! BT4-111 Jack Raid — Option, Purple, cost 0.
//!
//! [Main] Gain 1 memory for every 10 cards in your trash.
//! [Security] Gain 2 memory.
//!
//! Official Q&A: the Option is not yet in the trash when its [Main] counts —
//! 9 cards in trash ⇒ gain 0.
//!
//! DCGO: BT4/Purple/BT4_111.cs.
#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{self, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardColor, CardKind, DelayTrigger, EffectTiming, Keyword, ModifierType,
};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

const CARD_ID: &str = "BT4-111";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT4-111")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "PURPLE",
            "Purple Guy",
            CardColor::Purple,
            3,
            3,
            &[],
        ))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

/// Put `n` cards in player 0's trash, a purple Digimon on the field (colour
/// requirement), Jack Raid in hand, then use it. Returns the memory delta.
fn use_with_trash(n: usize) -> (DebugRunner, i16) {
    let mut r = setup();
    for _ in 0..n {
        push_trash(&mut r, 0, "FILLER");
    }
    r.place_on_field(0, "PURPLE", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    r.game.enter_main_phase();
    let before = r.memory();
    let idx = r.game.players[0].hand.len() - 1;
    let res = r.game.play_option_from_hand(0, idx);
    assert!(
        !matches!(res, OptionPlayResult::Invalid),
        "Jack Raid must be usable with a purple Digimon on the field"
    );
    let _ = r.auto_resolve();
    let delta = r.memory() - before;
    (r, delta)
}

#[test]
fn bt4_111_structure_main_and_security() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.name, "Jack Raid");
    assert_eq!(c.cost, Some(0));
    let main = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("[Main] clause");
    assert!(!main.optional);
    let sec = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("[Security] clause");
    assert_eq!(sec.scope, CompiledScope::FaceUp);
    assert!(!sec.optional);
}

#[test]
fn bt4_111_main_nine_in_trash_gains_nothing() {
    let (r, delta) = use_with_trash(9);
    assert_eq!(delta, 0, "Q&A: Jack Raid itself is not counted");
    assert_eq!(
        r.game.players[0].trash.len(),
        10,
        "Jack Raid then goes to trash"
    );
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt4_111_main_ten_in_trash_gains_one() {
    let (_r, delta) = use_with_trash(10);
    assert_eq!(delta, 1);
}

#[test]
fn bt4_111_main_twenty_five_in_trash_gains_two() {
    let (_r, delta) = use_with_trash(25);
    assert_eq!(delta, 2, "floor(25 / 10)");
}

#[test]
fn bt4_111_main_counts_only_own_trash() {
    let mut r = setup();
    for _ in 0..20 {
        push_trash(&mut r, 1, "FILLER");
    }
    r.place_on_field(0, "PURPLE", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    r.game.enter_main_phase();
    let before = r.memory();
    let res = r.game.play_option_from_hand(0, 0);
    assert!(!matches!(res, OptionPlayResult::Invalid));
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), before, "opponent's trash is not \"your trash\"");
}

#[test]
fn bt4_111_security_gains_two_for_defender() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT4-111")
        .add_card(filler("FILLER"))
        .add_card(digimon("ATK", "Attacker", CardColor::Red, 3, 3, &[]))
        .security(1, &[CARD_ID])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 0);
    assert_eq!(
        r.memory(),
        1,
        "the defender gains 2 memory → turn player's gauge 3 → 1"
    );
}

// ─── Local fixtures (this set has no shared `support` module) ───────────────

fn digimon(
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

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

fn filler(id: &str) -> CardData {
    digimon(id, id, CardColor::Red, 3, 3, &[])
}

fn data_idx(r: &DebugRunner, card_id: &str) -> usize {
    r.game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"))
}

fn push_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = data_idx(r, card_id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .hand
        .push(CardSource::new(idx, p, next));
}

fn push_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = data_idx(r, card_id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .trash
        .push(CardSource::new(idx, p, next));
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn ids(r: &DebugRunner, cards: &[CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].trash)
}

fn pass(r: &mut DebugRunner, p: u8) {
    r.execute_action(p, PASS).expect("PASS legal");
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| {
            v.valid_action_ids
                .into_iter()
                .filter(|&a| a != PASS)
                .collect()
        })
        .unwrap_or_default()
}

fn pick_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"));
    let action = space::PLAY_HAND_START + idx as u16;
    let v = r.pending_selection_view().expect("hand selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{card_id} not a legal hand pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(p, action).unwrap();
}

fn pick_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r.game.players[p as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in trash"));
    let action = space::TRASH_EFFECT_START + idx as u16;
    let v = r.pending_selection_view().expect("trash selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{card_id} not a legal trash pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(p, action).unwrap();
}

/// `true` when `card_id` (in `p`'s trash) is a legal pick of the pending prompt.
fn trash_pickable(r: &DebugRunner, p: u8, card_id: &str) -> bool {
    let Some(idx) = r.game.players[p as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
    else {
        return false;
    };
    non_pass(r).contains(&(space::TRASH_EFFECT_START + idx as u16))
}

/// Pick a permanent on one side of the field (OwnField / OppField prompt).
fn pick_side_field(r: &mut DebugRunner, selector: u8, h: PermanentHandle) {
    let action = space::encode_attack(0, h.index as u16);
    let v = r.pending_selection_view().expect("field selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{h:?} not a legal pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(selector, action).unwrap();
}

/// Test-only [On Play] "trash 1 card in your hand" Digimon — an own-effect
/// hand trash that fires `on_discard_hand`.
const HAND_TRASHER_YAML: &str = r#"
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
