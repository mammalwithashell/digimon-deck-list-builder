//! BT4-086 Cerberusmon: Werewolf Mode — Lv.5 Purple, Wizard, DP 9000, cost 9.
//!
//! <Rush>
//! [On Play] You may delete 1 of your [Cerberusmon] to gain 9 memory.
//!
//! Official Q&A: [Cerberusmon] is a different name from [Cerberusmon:
//! Werewolf Mode] — this card can't delete itself.
//!
//! DCGO: BT4/Purple/BT4_086.cs.
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

const CARD_ID: &str = "BT4-086";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT4-086")
        .add_card(filler("FILLER"))
        .add_card(digimon("CERB", "Cerberusmon", CardColor::Purple, 5, 7, &[]))
        .add_card(digimon(
            "CERB-X",
            "Cerberusmon X-Antibody",
            CardColor::Purple,
            5,
            7,
            &[],
        ))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(0)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt4_086_structure_rush_and_single_on_play() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.name, "Cerberusmon: Werewolf Mode");
    assert_eq!(c.level, Some(5));
    assert_eq!(c.cost, Some(9));
    assert_eq!(c.dp, Some(9000));
    assert!(c.traits.contains(&"Wizard".to_string()));
    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0].when, vec![CompiledTiming::OnPlay]);
    assert!(!triggered[0].once_per_turn);
}

#[test]
fn bt4_086_has_rush() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, None);
    assert!(r.game.has_keyword(h, Keyword::Rush));
}

#[test]
fn bt4_086_played_from_hand_deletes_cerberusmon_and_gains_nine() {
    let mut r = setup();
    r.game.memory = 9;
    let cerb = r.place_on_field(0, "CERB", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    r.play(0, 0).expect("play Werewolf Mode");
    assert_eq!(r.memory(), 0, "paid 9");
    let v = r.pending_selection_view().expect("optional delete prompt");
    assert!(v.is_optional, "printed 'you may'");
    assert_eq!(
        non_pass(&r).len(),
        1,
        "only [Cerberusmon] — never this card itself"
    );
    pick_side_field(&mut r, 0, cerb);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
    assert!(trash_ids(&r, 0).contains(&"CERB".to_string()));
    assert_eq!(r.memory(), 9, "gain 9 memory");
}

#[test]
fn bt4_086_decline_keeps_cerberusmon_and_gains_nothing() {
    let mut r = setup();
    r.place_on_field(0, "CERB", Some(0));
    let wm = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, wm);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0).len(), 2);
    assert_eq!(r.memory(), 0);
}

#[test]
fn bt4_086_name_must_be_exactly_cerberusmon() {
    let mut r = setup();
    r.place_on_field(0, "CERB-X", Some(0));
    let wm = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, wm);
    assert!(
        r.pending_selection_view().is_none(),
        "no exact-name [Cerberusmon] (and not itself) → no prompt"
    );
    assert_eq!(r.memory(), 0);
}

#[test]
fn bt4_086_cannot_delete_another_werewolf_mode_or_itself() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    let wm = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, wm);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(field_ids(&r, 0).len(), 2);
}

#[test]
fn bt4_086_opponent_cerberusmon_is_not_a_target() {
    let mut r = setup();
    r.place_on_field(1, "CERB", Some(0));
    let wm = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, wm);
    assert!(
        r.pending_selection_view().is_none(),
        "\"1 of YOUR [Cerberusmon]\""
    );
    assert_eq!(field_ids(&r, 1), vec!["CERB".to_string()]);
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
