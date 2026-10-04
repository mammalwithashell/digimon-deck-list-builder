//! BT21-098 Ragnarok Cannon — Option, Black, Cost 6, traits: [LIBERATOR].
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT21-098.md`)
//!
//! [Main] Delete 1 of your opponent's Digimon with the lowest play cost.
//! Then, place this card in the battle area.
//! [Your Turn] When one of your [Galacticmon] attacks, ＜Delay＞.
//! ・Delete 1 of your opponent's Digimon with the lowest play cost. If this
//! effect didn't delete, trash the top cards of your opponent's security stack
//! so that it has 1 card left.
//!
//! Security Effect:
//! [Security] You may play 1 card with [Vemmon] in its text and a play cost of
//! 6 or less from your hand or trash without paying the cost. Then, add this
//! card to the hand.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_098.cs`
//!
//! # Patterns
//! - [Main] mandatory lowest-play-cost delete (`lowest_play_cost` aggregate)
//!   + `place_self_as_delay_option` (BT23-096 shape).
//! - Event-gated `<Delay>` on `on_ally_attack` gated by attacker name
//!   (`event_target_name_contains: Galacticmon`), optional per 16-16-2.
//! - "If this effect didn't delete" → `effect_deleted_any_opponent_digimon:
//!   false` → `trash_top_security { leave: 1 }`.
//! - Option [Security] union-zone free play (`in_text_contains: Vemmon`,
//!   `play_cost_lte: 6`) then `add_this_option_to_hand` (BT25-096 shape).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT21-098";

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, name: &str, level: u8, cost: u16, dp: i32, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c.colors = vec![color];
    c
}

fn galacticmon() -> CardData {
    digimon("GALA", "Galacticmon", 6, 14, 14000, CardColor::Black)
}

fn not_galacticmon() -> CardData {
    digimon("OTHER-ATK", "Destromon", 6, 12, 14000, CardColor::Black)
}

/// Digimon card with [Vemmon] in its printed text (not named Vemmon).
fn vemmon_text_digimon(id: &str, cost: u16) -> CardData {
    let mut c = digimon(id, "Snatchmon", 4, cost, 6000, CardColor::Black);
    c.effect_text =
        "[On Play] Reveal the top 3 cards. Add 1 card with [Vemmon] in its text.".into();
    c
}

fn base_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-098 YAML parses and compiles")
        .add_card(galacticmon())
        .add_card(not_galacticmon())
        .add_card(digimon("LOW-A", "Low A", 3, 3, 3000, CardColor::Red))
        .add_card(digimon("LOW-B", "Low B", 3, 3, 4000, CardColor::Red))
        .add_card(digimon("HIGH", "High", 5, 7, 7000, CardColor::Red))
        .add_card(filler("FILL"))
        .add_card(filler("SEC"))
}

fn on_field(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| {
            !p.card_sources.is_empty() && p.top_card().card_id(&runner.game.card_data) == card_id
        })
}

fn in_trash(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

fn in_hand(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

fn cannon_state(runner: &DebugRunner) -> Option<OptionState> {
    runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .map(|p| p.option_state.clone())
}

/// Use BT21-098 from hand (index 0) and drive every prompt; returns once the
/// [Main] body + placement have resolved.
fn use_cannon(runner: &mut DebugRunner) {
    runner.game.enter_main_phase();
    let res = runner.game.play_option_from_hand(0, 0);
    assert_ne!(res, OptionPlayResult::Invalid, "BT21-098 must be usable");
    runner.auto_resolve().expect("resolve [Main]");
}

/// Advance two turns so the placing turn has passed and player 0 is turn
/// player again in its main phase.
fn advance_past_placing_turn(runner: &mut DebugRunner) {
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
}

// ═══ Section 1 — structure ═══════════════════════════════════════════════════

#[test]
fn bt21_098_structure() {
    let r = base_runner().build();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let data = r
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == CARD_ID)
        .expect("card data");
    assert_eq!(data.card_kind, CardKind::Option);
    assert_eq!(data.play_cost, 6);
    assert_eq!(data.colors, vec![CardColor::Black]);
    assert_eq!(data.traits, vec!["LIBERATOR".to_string()]);
    assert_eq!(card.effects.len(), 3, "[Main], <Delay>, [Security]");

    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::FaceUp
                && t.when.contains(&CompiledTiming::MainFromHand)
                && !t.optional
                && t.process.iter().any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption))
    )));
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. })
            if *trigger == CompiledTiming::OnAllyAttack
    )));
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnSecurity)
    )));
}

// ═══ Section 2/3 — [Main] ════════════════════════════════════════════════════

#[test]
fn bt21_098_main_deletes_only_the_lowest_play_cost_digimon_then_places_self() {
    let mut r = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.place_on_field(0, "GALA", Some(0)); // black anchor for color requirement
    let low = r.place_on_field(1, "LOW-A", Some(0));
    let high = r.place_on_field(1, "HIGH", Some(0));

    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, 0);
    let view = r
        .pending_selection_view()
        .expect("mandatory delete target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!r.pending_is_optional(), "the delete is mandatory");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the lowest play cost Digimon (cost 3) is a legal target"
    );
    r.auto_resolve().expect("resolve");

    assert!(in_trash(&r, 1, "LOW-A"), "lowest cost Digimon deleted");
    assert!(on_field(&r, 1, "HIGH"), "higher cost Digimon untouched");
    assert!(matches!(
        cannon_state(&r),
        Some(OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyAttack),
            ..
        })
    ));
}

#[test]
fn bt21_098_main_tied_lowest_cost_offers_both() {
    let mut r = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.place_on_field(0, "GALA", Some(0));
    r.place_on_field(1, "LOW-A", Some(0));
    r.place_on_field(1, "LOW-B", Some(0));
    r.place_on_field(1, "HIGH", Some(0));

    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, 0);
    let view = r.pending_selection_view().expect("delete prompt");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both cost-3 Digimon are legal"
    );
    let pick = view.valid_action_ids[1];
    r.execute_action(view.selecting_player, pick).expect("pick");
    r.auto_resolve().expect("resolve");
    let deleted = in_trash(&r, 1, "LOW-A") as u8 + in_trash(&r, 1, "LOW-B") as u8;
    assert_eq!(deleted, 1, "exactly one of the tied Digimon is deleted");
    assert!(on_field(&r, 1, "HIGH"));
}

#[test]
fn bt21_098_main_without_opponent_digimon_still_places_self() {
    let mut r = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.place_on_field(0, "GALA", Some(0));
    use_cannon(&mut r);
    assert!(cannon_state(&r).is_some(), "placed in battle area");
}

// ═══ Section 3 — <Delay> on Galacticmon attack ════════════════════════════════

#[test]
fn bt21_098_delay_galacticmon_attack_deletes_lowest_cost() {
    let mut r = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    let gala = r.place_on_field(0, "GALA", Some(0));
    use_cannon(&mut r);
    advance_past_placing_turn(&mut r);
    r.place_on_field(1, "LOW-A", Some(0));
    r.place_on_field(1, "HIGH", Some(0));
    let sec_before = r.security_count(1);

    r.attack_player(gala, 1, false);
    let view = r
        .pending_selection_view()
        .expect("<Delay> activation prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(r.pending_is_optional(), "<Delay> is optional (16-16-2)");
    r.execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
        .expect("accept");
    let view = r.pending_selection_view().expect("delete target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(view.valid_action_ids.len(), 1, "lowest cost only");
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick");
    r.auto_resolve().expect("finish the attack");
    assert!(in_trash(&r, 0, CARD_ID), "Delay cost trashes BT21-098");
    assert!(in_trash(&r, 1, "LOW-A"), "lowest cost Digimon deleted");
    assert!(on_field(&r, 1, "HIGH"));
    assert_eq!(
        r.security_count(1),
        sec_before - 1,
        "a successful delete does NOT trash security (only the attack's own check removes 1)"
    );
}

#[test]
fn bt21_098_delay_without_delete_trims_security_to_one() {
    let mut r = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .security(1, &["SEC", "SEC", "SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    let gala = r.place_on_field(0, "GALA", Some(0));
    use_cannon(&mut r);
    advance_past_placing_turn(&mut r);
    assert_eq!(r.security_count(1), 5);
    let checkpoint = r.event_checkpoint();

    r.attack_player(gala, 1, false);
    let view = r
        .pending_selection_view()
        .expect("<Delay> activation prompt");
    r.execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
        .expect("accept");
    r.auto_resolve().expect("finish the attack");
    assert!(in_trash(&r, 0, CARD_ID));
    // 5 security → Delay trims to exactly 1 (4 trashed) → the attack's own
    // check removes the last one. Had the Delay over-trashed to 0, the attack
    // would have been a direct hit and ended the game.
    assert!(!r.game_over(), "security was trimmed to 1, not 0");
    assert_eq!(r.security_count(1), 0);
    assert_eq!(
        r.game.players[1]
            .trash
            .iter()
            .filter(|c| c.card_id(&r.game.card_data) == "SEC")
            .count(),
        5,
        "4 trimmed by the Delay + 1 checked by the attack"
    );
    assert!(
        r.events_since(checkpoint)
            .iter()
            .any(|e| e.type_str() == "Trash"),
        "the Delay cost / security trim emit Trash events"
    );
}

#[test]
fn bt21_098_delay_decline_keeps_cannon_parked() {
    let mut r = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    let gala = r.place_on_field(0, "GALA", Some(0));
    use_cannon(&mut r);
    advance_past_placing_turn(&mut r);
    r.place_on_field(1, "LOW-A", Some(0));

    r.attack_player(gala, 1, false);
    let view = r
        .pending_selection_view()
        .expect("<Delay> activation prompt");
    r.execute_action(view.selecting_player, PASS)
        .expect("decline");
    assert!(cannon_state(&r).is_some(), "declined Delay stays parked");
    assert!(on_field(&r, 1, "LOW-A"), "no deletion when declined");
}

#[test]
fn bt21_098_delay_ignores_non_galacticmon_attacker() {
    let mut r = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    r.place_on_field(0, "GALA", Some(5)); // anchor that cannot attack
    use_cannon(&mut r);
    advance_past_placing_turn(&mut r);
    let other = r.place_on_field(0, "OTHER-ATK", Some(0));
    r.place_on_field(1, "LOW-A", Some(0));

    r.attack_player(other, 1, false);
    assert!(
        !matches!(r.pending_kind(), Some(SelectionKind::Replacement)),
        "a non-[Galacticmon] attack must not offer the Delay"
    );
    r.auto_resolve().expect("resolve attack");
    assert!(cannon_state(&r).is_some(), "Cannon stays parked");
    assert!(on_field(&r, 1, "LOW-A"));
}

#[test]
fn bt21_098_delay_does_not_fire_on_placing_turn() {
    let mut r = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    let gala = r.place_on_field(0, "GALA", Some(0));
    use_cannon(&mut r);
    r.place_on_field(1, "LOW-A", Some(0));
    r.attack_player(gala, 1, false);
    assert!(
        !matches!(r.pending_kind(), Some(SelectionKind::Replacement)),
        "Delay cannot activate on its placing turn"
    );
    r.auto_resolve().expect("resolve attack");
    assert!(cannon_state(&r).is_some());
    assert!(on_field(&r, 1, "LOW-A"));
}

// ═══ Section 3 — [Security] ══════════════════════════════════════════════════

#[test]
fn bt21_098_security_plays_vemmon_text_card_then_adds_self_to_hand() {
    let mut r = base_runner()
        .add_card(vemmon_text_digimon("VEM-6", 6))
        .add_card(vemmon_text_digimon("VEM-7", 7))
        .add_card(digimon("ATK", "Attacker", 4, 4, 4000, CardColor::Red))
        .hand(1, &["VEM-6", "VEM-7"])
        .security(1, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(0)
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);

    let view = r.pending_selection_view().expect("security union pick");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    assert!(r.pending_is_optional(), "'you may play'");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the cost<=6 [Vemmon]-text card is eligible"
    );
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("play VEM-6");
    r.auto_resolve().expect("resolve");
    assert!(on_field(&r, 1, "VEM-6"), "played without paying the cost");
    assert!(in_hand(&r, 1, "VEM-7"));
    assert!(in_hand(&r, 1, CARD_ID), "Ragnarok Cannon added to hand");
}

#[test]
fn bt21_098_security_trash_origin_and_decline_still_adds_to_hand() {
    let mut r = base_runner()
        .add_card(vemmon_text_digimon("VEM-6", 6))
        .add_card(digimon("ATK", "Attacker", 4, 4, 4000, CardColor::Red))
        .security(1, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(0)
        .start();
    r.inject_trash(1, "VEM-6");
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let view = r.pending_selection_view().expect("security union pick");
    assert_eq!(view.valid_action_ids.len(), 1, "trash candidate offered");
    r.execute_action(view.selecting_player, PASS)
        .expect("decline");
    r.auto_resolve().expect("resolve");
    assert!(in_trash(&r, 1, "VEM-6"), "declined: stays in trash");
    assert!(in_hand(&r, 1, CARD_ID), "Cannon still added to hand");
}
