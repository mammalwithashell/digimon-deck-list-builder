//! P-209 Titamon — Lv.6 Purple/Green, Shaman/Titan/TS (+ Rule: [Demon]),
//! DP 11000, cost 11.
//!
//! <Alliance>
//! [On Play] [When Digivolving] By trashing 1 card in your hand, suspend 1 of
//! your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't
//! unsuspend until their turn ends.
//! [All Turns] [Once Per Turn] When your hand is trashed from, you may play 1
//! level 4 or lower [Demon] or [Titan] trait card from your trash without
//! paying the cost.
//! (Rule) Trait: Has [Demon] Type.
//!
//! DCGO: P/Purple/P_209.cs.
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

const CARD_ID: &str = "P-209";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-209")
        .from_dsl_yaml(HAND_TRASHER_YAML)
        .expect("trasher")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "DEMON4",
            "Demon Four",
            CardColor::Purple,
            4,
            5,
            &["Demon"],
        ))
        .add_card(digimon(
            "TITAN4",
            "Titan Four",
            CardColor::Purple,
            4,
            5,
            &["Titan"],
        ))
        .add_card(digimon(
            "TITAN5",
            "Titan Five",
            CardColor::Purple,
            5,
            7,
            &["Titan"],
        ))
        .add_card(digimon(
            "PLAIN4",
            "Plain Four",
            CardColor::Purple,
            4,
            5,
            &[],
        ))
        .add_card(digimon("TS5", "TS Five", CardColor::Purple, 5, 7, &["TS"]))
        .add_card(digimon("OPP-A", "Opp A", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("OPP-B", "Opp B", CardColor::Red, 5, 7, &[]))
        .add_card(tamer("OPP-T", CardColor::Red))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn p_209_structure_traits_alliance_alt_path() {
    let mut r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(c.level, Some(6));
    assert_eq!(c.cost, Some(11));
    assert_eq!(c.dp, Some(11000));
    for t in ["Shaman", "Titan", "TS", "Demon"] {
        assert!(
            c.traits.contains(&t.to_string()),
            "{t} (Demon via Rule trait)"
        );
    }
    let alt = format!("{:?}", c.alt_paths);
    assert!(
        alt.contains("Demon") && alt.contains("TS"),
        "Lv.5 w/[Demon]/[TS] alt digivolve"
    );
    let opwd = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("OP/WD clause");
    assert!(opwd.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!opwd.once_per_turn);
    let at = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDiscardHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("All Turns on_discard_hand clause");
    assert!(at.once_per_turn);
    assert!(at.optional, "'you may play'");
    let h = r.place_on_field(0, CARD_ID, None);
    assert!(r.game.has_keyword(h, Keyword::Alliance));
}

#[test]
fn p_209_on_play_trash_suspend_then_lock() {
    let mut r = setup();
    let opp_a = r.place_on_field(1, "OPP-A", Some(0));
    let opp_t = r.place_on_field(1, "OPP-T", Some(0));
    let ti = r.place_on_field(0, CARD_ID, None);
    push_hand(&mut r, 0, "PLAIN4");
    fire(&mut r, EffectTiming::OnPlay, ti);
    let v = r.pending_selection_view().expect("cost prompt");
    assert!(v.is_optional, "'By trashing' — declinable");
    pick_hand(&mut r, 0, "PLAIN4");
    let v = r.pending_selection_view().expect("suspend prompt");
    assert!(!v.is_optional);
    assert_eq!(non_pass(&r).len(), 2, "Digimon or Tamer");
    pick_side_field(&mut r, 0, opp_a);
    let v = r.pending_selection_view().expect("lock prompt");
    assert!(!v.is_optional);
    pick_side_field(&mut r, 0, opp_t);
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[opp_a.index as usize].is_suspended);
    assert!(r.modifiers().has(opp_t, ModifierType::CannotUnsuspend));
    assert!(!r.modifiers().has(opp_a, ModifierType::CannotUnsuspend));
    assert!(trash_ids(&r, 0).contains(&"PLAIN4".to_string()));
}

#[test]
fn p_209_when_digivolving_fires_too() {
    let mut r = setup();
    let opp_t = r.place_on_field(1, "OPP-T", Some(0));
    let ti = r.place_stack(0, &["TS5", CARD_ID]);
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::WhenDigivolving, ti);
    pick_hand(&mut r, 0, "FILLER");
    pick_side_field(&mut r, 0, opp_t);
    pick_side_field(&mut r, 0, opp_t);
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[opp_t.index as usize].is_suspended);
    assert!(r.modifiers().has(opp_t, ModifierType::CannotUnsuspend));
}

#[test]
fn p_209_decline_cost_does_nothing() {
    let mut r = setup();
    let opp_a = r.place_on_field(1, "OPP-A", Some(0));
    let ti = r.place_on_field(0, CARD_ID, None);
    push_hand(&mut r, 0, "PLAIN4");
    fire(&mut r, EffectTiming::OnPlay, ti);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!r.game.players[1].battle_area[opp_a.index as usize].is_suspended);
    assert!(!r.modifiers().has(opp_a, ModifierType::CannotUnsuspend));
    assert_eq!(hand_ids(&r, 0), vec!["PLAIN4".to_string()]);
}

#[test]
fn p_209_on_play_empty_hand_no_prompt() {
    let mut r = setup();
    r.place_on_field(1, "OPP-A", Some(0));
    let ti = r.place_on_field(0, CARD_ID, None);
    fire(&mut r, EffectTiming::OnPlay, ti);
    assert!(r.pending_selection_view().is_none());
}

#[test]
fn p_209_lock_survives_opponents_unsuspend_phase() {
    let mut r = setup();
    let opp_a = r.place_on_field(1, "OPP-A", Some(0));
    let ti = r.place_on_field(0, CARD_ID, None);
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, ti);
    pick_hand(&mut r, 0, "FILLER");
    pick_side_field(&mut r, 0, opp_a);
    pick_side_field(&mut r, 0, opp_a);
    let _ = r.auto_resolve();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1);
    assert!(
        r.game.players[1].battle_area[opp_a.index as usize].is_suspended,
        "can't unsuspend during their unsuspend phase"
    );
}

#[test]
fn p_209_all_turns_hand_trash_plays_demon_or_titan_from_trash() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "DEMON4");
    push_trash(&mut r, 0, "TITAN4");
    push_trash(&mut r, 0, "TITAN5");
    push_trash(&mut r, 0, "PLAIN4");
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    let mem = r.memory();
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    assert!(r.pending_is_optional(), "'you may play'");
    r.accept_optional_trigger().unwrap();
    assert!(trash_pickable(&r, 0, "DEMON4"));
    assert!(trash_pickable(&r, 0, "TITAN4"));
    assert!(!trash_pickable(&r, 0, "TITAN5"), "level 4 or lower");
    assert!(!trash_pickable(&r, 0, "PLAIN4"), "[Demon] or [Titan] only");
    pick_trash(&mut r, 0, "DEMON4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DEMON4".to_string()));
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn p_209_all_turns_once_per_turn() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN4");
    push_trash(&mut r, 0, "DEMON4");
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    r.accept_optional_trigger().unwrap();
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    assert!(r.pending_selection_view().is_none(), "OPT spent");
    assert!(trash_ids(&r, 0).contains(&"DEMON4".to_string()));
}

#[test]
fn p_209_all_turns_decline_keeps_trash() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN4");
    push_hand(&mut r, 0, "FILLER");
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&"TITAN4".to_string()));
}

#[test]
fn p_209_all_turns_ignores_opponent_hand_trash() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN4");
    push_hand(&mut r, 1, "FILLER");
    let t = r.place_on_field(1, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 1, "FILLER");
    assert!(r.pending_selection_view().is_none(), "only YOUR hand");
}

#[test]
fn p_209_own_on_play_cost_triggers_all_turns_revival() {
    let mut r = setup();
    let opp_a = r.place_on_field(1, "OPP-A", Some(0));
    let ti = r.place_on_field(0, CARD_ID, None);
    push_trash(&mut r, 0, "TITAN4");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, ti);
    pick_hand(&mut r, 0, "FILLER");
    // Finish the On Play body, then the hand-trash trigger resolves.
    let mut guard = 0;
    while let Some(v) = r.pending_selection_view() {
        guard += 1;
        assert!(guard < 10);
        if v.kind == SelectionKind::Replacement {
            r.accept_optional_trigger().unwrap();
        } else if trash_pickable(&r, 0, "TITAN4") {
            pick_trash(&mut r, 0, "TITAN4");
        } else {
            pick_side_field(&mut r, 0, opp_a);
        }
    }
    assert!(field_ids(&r, 0).contains(&"TITAN4".to_string()));
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
