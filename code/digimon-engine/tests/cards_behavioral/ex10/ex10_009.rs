//! EX10-009 Creepymon — Lv.6 Red/Purple, Demon Lord / Seven Great Demon Lords /
//! Fallen Angel ((Rule) trait).
//!
//! [When Digivolving] [On Deletion] Delete all of your opponent's lowest DP
//! Digimon. If this effect didn't delete, trash their deck's top 5 cards.
//! [When Attacking] If your opponent has 10 or more cards in their trash, you
//! may play 1 red or purple 5000 DP or lower Digimon card from your trash to
//! your empty breeding area without paying the cost.
//! [End of Your Turn] This Digimon may attack.
//!
//! DCGO: EX10/Red/EX10_009.cs. Synthetic boards only (no recorded games).
//!
//! Engine primitive covered here: `play_to_breeding_from_trash` /
//! `can_play_to_breeding_from_trash` (play a trash Digimon into the empty
//! breeding area; no memory, no [On Play]).

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, GamePhase, ModifierType, PlaySource};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use support::*;

const CARD_ID: &str = "EX10-009";

fn mon(id: &str, level: u8, dp: i32, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

fn setup() -> DebugRunner {
    let mut tamer = make_test_card("TAMER-R", "Red Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.colors = vec![CardColor::Red];
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX10-009 in embedded DSL pack")
        .add_card(mon("PAD", 3, 3000, CardColor::Yellow))
        .add_card(mon("BASE-RED5", 5, 8000, CardColor::Red))
        .add_card(mon("BASE-PURPLE5", 5, 8000, CardColor::Purple))
        .add_card(mon("BASE-BLUE5", 5, 8000, CardColor::Blue))
        .add_card(mon("OPP-LOW-A", 3, 3000, CardColor::Blue))
        .add_card(mon("OPP-LOW-B", 3, 3000, CardColor::Green))
        .add_card(mon("OPP-MID", 4, 5000, CardColor::Blue))
        .add_card(mon("OPP-HIGH", 5, 9000, CardColor::Blue))
        .add_card(mon("RED-5K", 3, 5000, CardColor::Red))
        .add_card(mon("PURPLE-5K", 3, 5000, CardColor::Purple))
        .add_card(mon("RED-6K", 4, 6000, CardColor::Red))
        .add_card(mon("BLUE-3K", 3, 3000, CardColor::Blue))
        .add_card(tamer)
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
        .security(1, &["PAD"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.current_phase = GamePhase::Main;
    r.game.turn_count = 3;
    r
}

fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) -> bool {
    let idx = r.add_to_hand(0, CARD_ID);
    r.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve)
}

fn fill_opp_trash(r: &mut DebugRunner, n: usize) {
    for _ in 0..n {
        push_trash(r, 1, "PAD");
    }
}

fn accept(r: &mut DebugRunner) {
    let a = non_pass(r)[0];
    let p = r.pending_selection().expect("pending").selecting_player;
    r.execute_action(p, a).unwrap();
}

// ─── Structural / identity ───────────────────────────────────────────────────

#[test]
fn ex10_009_metadata_and_clause_shape() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Creepymon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for t in ["Demon Lord", "Seven Great Demon Lords", "Fallen Angel"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t}");
    }
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 3, "WD/OD, WA, End of Your Turn");
    let shared = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("WD/OD");
    assert_eq!(shared.when.len(), 2);
    assert!(!shared.optional && !shared.once_per_turn);
    let wa = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("WA");
    assert!(wa.optional && wa.outer_prompt && !wa.once_per_turn);
    let eot = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::EndOfYourTurn))
        .expect("EOT");
    assert!(eot.optional && eot.outer_prompt && !eot.once_per_turn);
}

#[test]
fn ex10_009_digivolves_from_red_or_purple_lv5_for_4() {
    for (base_id, ok) in [("BASE-RED5", true), ("BASE-PURPLE5", true), ("BASE-BLUE5", false)] {
        let mut r = setup();
        let base = r.place_on_field(0, base_id, Some(0));
        assert_eq!(digivolve_onto(&mut r, base), ok, "{base_id}");
        if ok {
            assert_eq!(top_id(&r, base), CARD_ID);
            assert_eq!(r.memory(), 10 - 4, "{base_id}");
        }
    }
}

// ─── [When Digivolving] [On Deletion] ────────────────────────────────────────

#[test]
fn ex10_009_wd_deletes_all_lowest_dp_digimon_and_does_not_mill() {
    let mut r = setup();
    r.place_on_field(1, "OPP-LOW-A", Some(0));
    r.place_on_field(1, "OPP-LOW-B", Some(0));
    r.place_on_field(1, "OPP-MID", Some(0));
    let base = r.place_on_field(0, "BASE-RED5", Some(0));
    assert!(digivolve_onto(&mut r, base));
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none(), "no pick: it deletes ALL lowest");
    assert_eq!(field_ids(&r, 1), vec!["OPP-MID".to_string()], "both tied lowest are deleted");
    assert_eq!(deck_ids(&r, 1).len(), 20, "it deleted, so no mill");
}

#[test]
fn ex10_009_wd_with_no_opponent_digimon_mills_five() {
    let mut r = setup();
    let base = r.place_on_field(0, "BASE-RED5", Some(0));
    assert!(digivolve_onto(&mut r, base));
    let _ = r.auto_resolve();
    assert_eq!(deck_ids(&r, 1).len(), 15);
    assert_eq!(trash_ids(&r, 1).len(), 5);
    assert_eq!(deck_ids(&r, 0).len(), 20 - 1, "only the opponent's deck (own: the digivolve draw)");
}

#[test]
fn ex10_009_wd_when_the_deletion_is_prevented_it_didnt_delete_so_it_mills_five() {
    let mut r = setup();
    let protected = r.place_on_field(1, "OPP-LOW-A", Some(0));
    r.game.modifiers.add(
        protected,
        ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect, 0, Expiry::Permanent, 1),
    );
    let base = r.place_on_field(0, "BASE-RED5", Some(0));
    assert!(digivolve_onto(&mut r, base));
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-LOW-A".to_string()], "survived");
    assert_eq!(deck_ids(&r, 1).len(), 15, "nothing was deleted: mill 5");
}

#[test]
fn ex10_009_on_deletion_runs_the_same_body() {
    let mut r = setup();
    r.place_on_field(1, "OPP-LOW-A", Some(0));
    r.place_on_field(1, "OPP-HIGH", Some(0));
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-HIGH".to_string()]);
    assert_eq!(deck_ids(&r, 1).len(), 20);
}

// ─── [When Attacking] ────────────────────────────────────────────────────────

fn creepy_ready(trash_opp: usize) -> (DebugRunner, PermanentHandle) {
    let mut r = setup();
    fill_opp_trash(&mut r, trash_opp);
    for id in ["RED-5K", "PURPLE-5K", "RED-6K", "BLUE-3K"] {
        push_trash(&mut r, 0, id);
    }
    let h = r.place_on_field(0, CARD_ID, Some(0));
    (r, h)
}

#[test]
fn ex10_009_wa_needs_ten_cards_in_the_opponents_trash() {
    let (mut r, h) = creepy_ready(9);
    r.attack_player(h, 1, false);
    assert!(r.pending_selection().is_none(), "9 < 10: the effect doesn't activate");
    let _ = r.auto_resolve();
    assert!(r.game.players[0].breeding_area.is_none());
}

#[test]
fn ex10_009_wa_plays_a_red_or_purple_5000_dp_or_lower_digimon_into_the_empty_breeding_area() {
    let (mut r, h) = creepy_ready(10);
    r.attack_player(h, 1, false);
    assert!(r.pending_is_optional(), "outer 'you may' prompt");
    accept(&mut r);
    assert!(r.pending_is_optional(), "the pick is declinable too (DCGO canNoSelect)");
    // RED-5K and PURPLE-5K only: not 6000 DP, not blue.
    assert_eq!(non_pass(&r).len(), 2);
    pick_trash(&mut r, 0, "PURPLE-5K");
    let _ = r.auto_resolve();
    let breeding = r.game.players[0].breeding_area.as_ref().expect("placed in breeding");
    assert_eq!(breeding.top_card().card_id(&r.game.card_data), "PURPLE-5K");
    assert!(!trash_ids(&r, 0).contains(&"PURPLE-5K".to_string()));
    assert_eq!(r.memory(), 10, "without paying the cost");
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()], "not on the battle area");
}

#[test]
fn ex10_009_wa_can_be_declined_at_both_prompts() {
    let (mut r, h) = creepy_ready(12);
    r.attack_player(h, 1, false);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.game.players[0].breeding_area.is_none(), "declined the outer prompt");

    let (mut r, h) = creepy_ready(12);
    r.attack_player(h, 1, false);
    accept(&mut r);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.game.players[0].breeding_area.is_none(), "declined the pick");
    assert!(trash_ids(&r, 0).contains(&"RED-5K".to_string()));
}

#[test]
fn ex10_009_wa_needs_an_empty_breeding_area() {
    let (mut r, h) = creepy_ready(10);
    r.place_in_breeding(0, "PAD");
    r.attack_player(h, 1, false);
    assert!(r.pending_selection().is_none(), "occupied breeding area: not activatable");
    let _ = r.auto_resolve();
}

#[test]
fn ex10_009_wa_needs_a_qualifying_trash_card() {
    let mut r = setup();
    fill_opp_trash(&mut r, 10);
    push_trash(&mut r, 0, "RED-6K");
    push_trash(&mut r, 0, "BLUE-3K");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(h, 1, false);
    assert!(r.pending_selection().is_none(), "no candidate: not activatable");
    let _ = r.auto_resolve();
}

#[test]
fn ex10_009_wa_respects_the_cannot_play_from_trash_lock() {
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut r, h) = creepy_ready(10);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayFromTrash, 0, Expiry::Permanent, None, 1),
    );
    r.attack_player(h, 1, false);
    assert!(
        r.pending_selection().is_none(),
        "BT23-014's trash-play lock stops a trash play whatever the destination"
    );
    let _ = r.auto_resolve();
    assert!(r.game.players[0].breeding_area.is_none());
}

#[test]
fn ex10_009_wa_respects_the_cannot_play_digimon_by_effect_lock() {
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut r, h) = creepy_ready(10);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    r.attack_player(h, 1, false);
    assert!(r.pending_selection().is_none());
    let _ = r.auto_resolve();
    assert!(r.game.players[0].breeding_area.is_none());
}

// ─── [End of Your Turn] ──────────────────────────────────────────────────────

fn trigger_eot(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, digimon_engine::selection::TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

#[test]
fn ex10_009_end_of_turn_may_attack() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    trigger_eot(&mut r, h);
    assert!(r.pending_is_optional(), "outer 'may' prompt");
    accept(&mut r);
    assert!(r.pending_is_optional(), "the attack itself may be declined");
    let attack_player = encode_attack(h.index as u16, SECURITY_TARGET);
    assert!(non_pass(&r).contains(&attack_player));
    r.execute_action(0, attack_player).unwrap();
    let _ = r.auto_resolve();
    assert!(r.game.players[1].security.len() < 5, "the attack checked security");
}

#[test]
fn ex10_009_end_of_turn_decline_does_not_attack() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    trigger_eot(&mut r, h);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.game.players[1].security.len(), 5);
}

#[test]
fn ex10_009_end_of_turn_is_not_offered_when_it_cannot_attack() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    trigger_eot(&mut r, h);
    assert!(r.pending_selection().is_none(), "a suspended Digimon can't attack");
}

#[test]
fn ex10_009_end_of_turn_is_not_once_per_turn() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    trigger_eot(&mut r, h);
    accept(&mut r);
    let attack_player = encode_attack(h.index as u16, SECURITY_TARGET);
    r.execute_action(0, attack_player).unwrap();
    let _ = r.auto_resolve();
    r.game.players[0].battle_area[h.index as usize].is_suspended = false;
    trigger_eot(&mut r, h);
    assert!(r.pending_selection().is_some(), "no [Once Per Turn] printed");
}
