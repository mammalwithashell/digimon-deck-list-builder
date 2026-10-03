//! BT25-080 Witchmon — Lv.4 Purple, Wizard/Titan/TS, DP 6000, cost 5.
//!
//! [On Play] [When Attacking] [Once Per Turn] By trashing 1 card in your hand,
//! you may return 1 [Titan] trait card from your trash to the hand. After, if
//! played by an effect, delete 1 of your opponent's level 5 or lower Digimon.
//! Inherited: [All Turns] [Once Per Turn] When your hand is trashed from, if
//! this Digimon has the [Titan] trait, delete 1 of your opponent's level 4 or
//! lower Digimon.
//!
//! Official Q&A: without the hand trash, the "After, …" part is not processed.
//!
//! DCGO: BT25/Purple/BT25_080.cs.
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

const CARD_ID: &str = "BT25-080";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-080")
        .from_dsl_yaml(HAND_TRASHER_YAML)
        .expect("trasher")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "TITAN-CARD",
            "Titan Card",
            CardColor::Purple,
            3,
            3,
            &["Titan"],
        ))
        .add_card(digimon(
            "PLAIN-CARD",
            "Plain Card",
            CardColor::Purple,
            3,
            3,
            &[],
        ))
        .add_card(digimon(
            "TITAN-TOP",
            "Titan Top",
            CardColor::Purple,
            5,
            7,
            &["Titan"],
        ))
        .add_card(digimon(
            "PLAIN-TOP",
            "Plain Top",
            CardColor::Purple,
            5,
            7,
            &["TS"],
        ))
        .add_card(digimon("OPP4", "Opp4", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("OPP5", "Opp5", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("OPP6", "Opp6", CardColor::Red, 6, 9, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
        .start();
    r.set_first_player(0);
    r
}

fn op_wa_clause(r: &DebugRunner) -> &digimon_dsl::compiled::CompiledTriggeredClause {
    r.compiled_card(CARD_ID)
        .unwrap()
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("OP/WA clause")
}

#[test]
fn bt25_080_structure() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    for t in ["Wizard", "Titan", "TS"] {
        assert!(c.traits.contains(&t.to_string()), "{t}");
    }
    assert!(
        format!("{:?}", c.alt_paths).contains("TS"),
        "Lv.3 w/[TS] alt digivolve"
    );
    let opwa = op_wa_clause(&r);
    assert!(opwa.when.contains(&CompiledTiming::WhenAttacking));
    assert!(opwa.once_per_turn, "[Once Per Turn] shared by OP and WA");
    let inh = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .expect("inherited clause");
    assert!(inh.when.contains(&CompiledTiming::OnDiscardHand));
    assert!(inh.once_per_turn);
}

#[test]
fn bt25_080_normal_play_returns_titan_card_but_does_not_delete() {
    let mut r = setup();
    r.game.memory = 5;
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    push_trash(&mut r, 0, "TITAN-CARD");
    push_trash(&mut r, 0, "PLAIN-CARD");
    r.place_on_field(1, "OPP5", Some(0));
    r.play(0, 0).expect("play Witchmon from hand");
    let v = r.pending_selection_view().expect("hand-trash cost prompt");
    assert!(v.is_optional, "'By trashing' — declinable cost");
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("trash return prompt");
    assert!(v.is_optional, "'you may return'");
    assert!(trash_pickable(&r, 0, "TITAN-CARD"));
    assert!(!trash_pickable(&r, 0, "PLAIN-CARD"), "[Titan] trait only");
    pick_trash(&mut r, 0, "TITAN-CARD");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["TITAN-CARD".to_string()]);
    assert!(trash_ids(&r, 0).contains(&"FILLER".to_string()));
    assert_eq!(
        field_ids(&r, 1),
        vec!["OPP5".to_string()],
        "not played by an effect"
    );
}

#[test]
fn bt25_080_effect_play_deletes_level_five_or_lower() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    push_trash(&mut r, 0, "TITAN-CARD");
    let opp5 = r.place_on_field(1, "OPP5", Some(0));
    r.place_on_field(1, "OPP6", Some(0));
    let card = r.game.players[0].hand[0].handle();
    r.game
        .play_card_from_effect_without_cost(0, card)
        .expect("effect play");
    pick_hand(&mut r, 0, "FILLER");
    // Declining the optional return does not stop the "After" part.
    pass(&mut r, 0);
    let v = r.pending_selection_view().expect("delete prompt");
    assert!(!v.is_optional, "the deletion is mandatory");
    assert_eq!(
        non_pass(&r).len(),
        1,
        "only the level 5 — level 6 is too high"
    );
    pick_side_field(&mut r, 0, opp5);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP6".to_string()]);
    assert!(
        trash_ids(&r, 0).contains(&"TITAN-CARD".to_string()),
        "return was declined"
    );
}

#[test]
fn bt25_080_declining_cost_skips_everything_and_refunds_opt() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    push_trash(&mut r, 0, "TITAN-CARD");
    r.place_on_field(1, "OPP5", Some(0));
    let card = r.game.players[0].hand[0].handle();
    let w = r
        .game
        .play_card_from_effect_without_cost(0, card)
        .expect("effect play");
    pass(&mut r, 0);
    assert!(
        r.pending_selection_view().is_none(),
        "Q&A: no trash → no 'After' part"
    );
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
    assert_eq!(hand_ids(&r, 0), vec!["FILLER".to_string()]);
    // OPT refunded (DCGO RemoveUse): [When Attacking] can still use it.
    fire(&mut r, EffectTiming::WhenAttacking, w);
    assert!(
        r.pending_selection_view().is_some(),
        "declined cost does not consume [Once Per Turn]"
    );
}

#[test]
fn bt25_080_once_per_turn_shared_between_on_play_and_when_attacking() {
    let mut r = setup();
    let w = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, w);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1);
    fire(&mut r, EffectTiming::WhenAttacking, w);
    assert!(
        r.pending_selection_view().is_none(),
        "OPT already used this turn"
    );
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt25_080_when_attacking_returns_but_never_deletes() {
    let mut r = setup();
    let w = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FILLER");
    push_trash(&mut r, 0, "TITAN-CARD");
    r.place_on_field(1, "OPP4", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, w);
    pick_hand(&mut r, 0, "FILLER");
    pick_trash(&mut r, 0, "TITAN-CARD");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["TITAN-CARD".to_string()]);
    assert_eq!(
        field_ids(&r, 1),
        vec!["OPP4".to_string()],
        "an attack is not 'played by an effect'"
    );
}

#[test]
fn bt25_080_empty_hand_no_prompt() {
    let mut r = setup();
    let w = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN-CARD");
    fire(&mut r, EffectTiming::WhenAttacking, w);
    assert!(r.pending_selection_view().is_none());
    assert!(trash_ids(&r, 0).contains(&"TITAN-CARD".to_string()));
}

#[test]
fn bt25_080_played_by_invasion_of_the_titans_security_deletes() {
    // Real effect-play path: BT24-098's [Security] plays Witchmon from hand
    // without paying the cost → Witchmon's [On Play] sees "played by an effect".
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-080")
        .dsl_card("BT24-098")
        .expect("BT24-098")
        .add_card(filler("FILLER"))
        .add_card(digimon("ATK", "Attacker", CardColor::Red, 5, 7, &[]))
        .security(1, &["BT24-098"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    push_hand(&mut r, 1, CARD_ID);
    push_hand(&mut r, 1, "FILLER");
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    // BT24-098 [Security]: play Witchmon from hand.
    let a = non_pass(&r)[0];
    let sel = r.pending_selection_view().unwrap().selecting_player;
    r.execute_action(sel, a).unwrap();
    // Witchmon [On Play]: trash FILLER, decline the return, delete ATK (Lv.5).
    pick_hand(&mut r, 1, "FILLER");
    if r.pending_selection_view()
        .map(|v| v.kind == SelectionKind::Trash)
        .unwrap_or(false)
    {
        pass(&mut r, 1);
    }
    let v = r
        .pending_selection_view()
        .expect("effect-played → delete prompt");
    assert!(!v.is_optional);
    pick_side_field(&mut r, 1, atk);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&CARD_ID.to_string()));
    assert!(
        !field_ids(&r, 0).contains(&"ATK".to_string()),
        "level 5 attacker deleted"
    );
}

// ─── Inherited ──────────────────────────────────────────────────────────────

#[test]
fn bt25_080_inherited_titan_host_deletes_level_four_on_own_hand_trash() {
    let mut r = setup();
    r.place_stack(0, &[CARD_ID, "TITAN-TOP"]);
    let opp4 = r.place_on_field(1, "OPP4", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("inherited delete prompt");
    assert!(!v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "level 4 or lower only");
    pick_side_field(&mut r, 0, opp4);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
}

#[test]
fn bt25_080_inherited_needs_titan_host() {
    let mut r = setup();
    r.place_stack(0, &[CARD_ID, "PLAIN-TOP"]);
    r.place_on_field(1, "OPP4", Some(0));
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert_eq!(
        field_ids(&r, 1),
        vec!["OPP4".to_string()],
        "host lacks [Titan]"
    );
}

#[test]
fn bt25_080_inherited_once_per_turn() {
    let mut r = setup();
    r.place_stack(0, &[CARD_ID, "TITAN-TOP"]);
    let a = r.place_on_field(1, "OPP4", Some(0));
    r.place_on_field(1, "OPP4", Some(0));
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    pick_side_field(&mut r, 0, a);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1).len(), 1);
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert_eq!(
        field_ids(&r, 1).len(),
        1,
        "second hand trash this turn: OPT spent"
    );
}

#[test]
fn bt25_080_inherited_ignores_opponent_hand_trash() {
    let mut r = setup();
    r.place_stack(0, &[CARD_ID, "TITAN-TOP"]);
    r.place_on_field(1, "OPP4", Some(0));
    push_hand(&mut r, 1, "FILLER");
    let t = r.place_on_field(1, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 1, "FILLER");
    let _ = r.auto_resolve();
    assert!(
        field_ids(&r, 1).contains(&"OPP4".to_string()),
        "only YOUR hand"
    );
}

#[test]
fn bt25_080_inherited_fires_on_opponents_turn() {
    let mut r = setup();
    r.place_stack(0, &[CARD_ID, "TITAN-TOP"]);
    let opp4 = r.place_on_field(1, "OPP4", Some(0));
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1);
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    if r.pending_selection_view().is_some() {
        pick_side_field(&mut r, 0, opp4);
    }
    let _ = r.auto_resolve();
    assert!(
        !field_ids(&r, 1).contains(&"OPP4".to_string()),
        "[All Turns]"
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
