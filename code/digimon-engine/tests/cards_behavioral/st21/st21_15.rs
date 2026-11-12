//! ST21-15 Gennai's House — Option, White, Cost 2, traits: [ADVENTURE].
//!
//! Printed (official Bandai DB, data/card_bundles/ST21-15.md):
//!   While you have no face-up [Gennai's House] security cards, you can ignore
//!   this card's color requirements.
//!   [Security] [Your Turn] All of your level 3 or higher Digimon get +3000 DP.
//!   [Main] Add your bottom security card to the hand. Then, place this card
//!   face up as the bottom security card.
//!   Security Effect: [Security] You may play 1 level 3 Digimon card from your
//!   trash without paying the cost.
//!
//! DCGO: ST21/White/ST21_15.cs — IgnoreColorConditionClass(card == self)
//! while `!Owner.SecurityCards.Any(EqualsCardName("Gennai's House") &&
//! !IsFlipped)`; ChangeDPStaticEffect(+3000, own battle-area Digimon Lv>=3)
//! gated on IsExistInSecurity(card, false) && IsOwnerTurn;
//! ReplaceBottomSecurityWithFaceUpOptionMainEffect; SecuritySkill optional
//! (canNoSelect true) SelectCardEffect Root.Trash, IsDigimon && Level == 3 →
//! PlayPermanentCards(payCost: false).
//! Twin of ST20-15 Island of Adventure (top → bottom, All Turns +2000 →
//! Your Turn +3000, Tamer-from-hand → Lv.3-from-trash).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::OptionPlayResult;

const CARD_ID: &str = "ST21-15";

fn digimon(id: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![color];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-15 YAML in embedded pack")
        .add_card(digimon("LV3", CardColor::Red, 3, 3000))
        .add_card(digimon("LV3-B", CardColor::Blue, 3, 3000))
        .add_card(digimon("LV2", CardColor::Red, 2, 2000))
        .add_card(digimon("LV4", CardColor::Red, 4, 4000))
        .add_card(digimon("ATTACKER", CardColor::Red, 6, 12000))
        .add_card({
            let mut c = make_test_card("ISLAND-NAMESAKE", "Gennai's House");
            c.card_kind = CardKind::Option;
            c
        })
        .add_card(make_test_card("BOTTOM", "Bottom"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("FILL", "Fill"))
}

fn push_trash(r: &mut DebugRunner, player: u8, id: &str) {
    let idx = r.game.card_data.iter().position(|c| c.card_id == id).unwrap();
    let iid = r.game.next_card_index();
    r.game.players[player as usize].trash.push(CardSource::new(idx, player, iid));
}

fn on_field(r: &DebugRunner, player: u8, id: &str) -> bool {
    r.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

fn face_up_security(r: &mut DebugRunner, player: u8, id: &str) {
    let idx = r.game.players[player as usize]
        .security
        .iter()
        .find(|c| c.card_id(&r.game.card_data) == id)
        .expect("card in security")
        .card_index;
    r.game.players[player as usize].face_up_security.insert(idx);
}

// ─── Structure ───────────────────────────────────────────────────────────────

#[test]
fn st21_15_identity_and_clauses() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.name, "Gennai's House");
    assert_eq!(card.cost, Some(2));
    assert!(card.traits.iter().any(|t| t == "ADVENTURE"));
    let timings: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.when.clone()),
            _ => None,
        })
        .collect();
    assert!(timings.contains(&vec![CompiledTiming::MainFromHand]));
    assert!(timings.contains(&vec![CompiledTiming::OnSecurity]));
}

// ─── Colour-requirement waiver ──────────────────────────────────────────────

#[test]
fn st21_15_colour_waiver_open_without_face_up_gennais_house() {
    let r = builder()
        .hand(0, &[CARD_ID])
        .security(0, &["FILL", "FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[0], 1.0, "no white source on field, but no face-up [Gennai's House] in security");
}

#[test]
fn st21_15_colour_waiver_closed_by_face_up_gennais_house_but_not_face_down() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .security(0, &[CARD_ID, "FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    // Face-down copy: gate stays open.
    assert_eq!(build_action_mask(&r.game, 0)[0], 1.0, "face-down copy does not close the waiver");
    face_up_security(&mut r, 0, CARD_ID);
    assert_eq!(
        build_action_mask(&r.game, 0)[0],
        0.0,
        "a face-up [Gennai's House] re-asserts the colour requirement"
    );
}

#[test]
fn st21_15_colour_waiver_matches_by_name() {
    // A different card NAMED "Gennai's House" face-up also closes the waiver
    // (DCGO EqualsCardName).
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .security(0, &["ISLAND-NAMESAKE", "FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    face_up_security(&mut r, 0, "ISLAND-NAMESAKE");
    assert_eq!(build_action_mask(&r.game, 0)[0], 0.0);
}

// ─── [Security] [Your Turn] +3000 DP aura ────────────────────────────────────

#[test]
fn st21_15_security_aura_buffs_own_lv3_plus_on_your_turn_only() {
    let mut r = builder()
        .security(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .start();
    face_up_security(&mut r, 0, CARD_ID);
    let lv3 = r.place_on_field(0, "LV3", None);
    let lv4 = r.place_on_field(0, "LV4", None);
    let lv2 = r.place_on_field(0, "LV2", None);
    let opp = r.place_on_field(1, "LV3-B", None);
    assert_eq!(r.turn_player(), 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(lv3), Some(6000));
    assert_eq!(r.effective_dp(lv4), Some(7000));
    assert_eq!(r.effective_dp(lv2), Some(2000), "below level 3");
    assert_eq!(r.effective_dp(opp), Some(3000), "opponent's Digimon unaffected");

    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(lv3), Some(3000), "[Your Turn] — not on the opponent's turn");
}

#[test]
fn st21_15_security_aura_inactive_face_down() {
    let mut r = builder()
        .security(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();
    let lv3 = r.place_on_field(0, "LV3", None);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(lv3), Some(3000));
}

// ─── [Main] ──────────────────────────────────────────────────────────────────

#[test]
fn st21_15_main_swaps_bottom_security_with_self_face_up() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .security(0, &["BOTTOM", "TOP"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let mem = r.memory();
    let result = r.game.play_option_from_hand(0, 0);
    assert_ne!(result, OptionPlayResult::Invalid);
    let _ = r.auto_resolve();
    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "BOTTOM"));
    let sec = &r.game.players[0].security;
    assert_eq!(sec.len(), 2);
    assert_eq!(sec[0].card_id(&r.game.card_data), CARD_ID, "placed as the BOTTOM card");
    assert_eq!(sec[1].card_id(&r.game.card_data), "TOP", "top card untouched");
    assert!(r.game.players[0].face_up_security.contains(&sec[0].card_index), "face up");
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == CARD_ID),
        "the Option is not trashed"
    );
    assert_eq!(r.memory(), mem - 2, "cost 2");
}

// ─── [Security] Security Effect ──────────────────────────────────────────────

fn security_runner() -> DebugRunner {
    let mut r = builder()
        .security(1, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    push_trash(&mut r, 1, "LV3");
    push_trash(&mut r, 1, "LV2");
    push_trash(&mut r, 1, "LV4");
    r
}

#[test]
fn st21_15_security_effect_plays_lv3_from_trash_free() {
    let mut r = security_runner();
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let mem = r.memory();
    let _ = r.attack_player(attacker, 1, false);
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional, "'you may'");
    let candidates: Vec<_> = v.valid_action_ids.iter().copied().filter(|a| *a != PASS).collect();
    assert_eq!(candidates.len(), 1, "only the level 3 Digimon card is offered");
    r.execute_action(v.selecting_player, candidates[0]).expect("play");
    let _ = r.auto_resolve();
    assert!(on_field(&r, 1, "LV3"));
    assert!(!on_field(&r, 1, "LV2") && !on_field(&r, 1, "LV4"));
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn st21_15_security_effect_can_be_declined() {
    let mut r = security_runner();
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let _ = r.attack_player(attacker, 1, false);
    let v = r.pending_selection_view().expect("trash pick");
    r.execute_action(v.selecting_player, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert!(!on_field(&r, 1, "LV3"));
    assert!(r.game.players[1]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "LV3"));
}

/// [Main] with an EMPTY security stack: there is no bottom card to add, but
/// "Then, place this card face up as the bottom security card" still happens
/// (DCGO ReplaceBottomSecurityWithFaceUpOptionEffect: the add-to-hand half is
/// gated on SecurityCards.Count >= 1, the placement is not).
#[test]
fn st21_15_main_with_empty_security_still_places_self_face_up() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    assert_eq!(r.security_count(0), 0);
    let hand_before = r.game.players[0].hand.len();
    let result = r.game.play_option_from_hand(0, 0);
    assert_ne!(result, OptionPlayResult::Invalid);
    let _ = r.auto_resolve();
    let sec = &r.game.players[0].security;
    assert_eq!(sec.len(), 1, "the Option becomes the only security card");
    assert_eq!(sec[0].card_id(&r.game.card_data), CARD_ID);
    assert!(r.game.players[0].face_up_security.contains(&sec[0].card_index), "face up");
    assert_eq!(r.game.players[0].hand.len(), hand_before - 1, "nothing was added to hand");
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == CARD_ID),
        "the Option is not trashed"
    );
}

/// [Security] effect with no level 3 Digimon card in trash: no card can be
/// chosen, nothing is played, and the attack resolves normally.
#[test]
fn st21_15_security_effect_with_no_lv3_in_trash_plays_nothing() {
    let mut r = builder()
        .security(1, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    push_trash(&mut r, 1, "LV2");
    push_trash(&mut r, 1, "LV4");
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let _ = r.attack_player(attacker, 1, false);
    if let Some(v) = r.pending_selection_view() {
        assert!(
            v.valid_action_ids.iter().all(|a| *a == PASS),
            "no level 3 Digimon card may be offered: {v:?}"
        );
        r.execute_action(v.selecting_player, PASS).expect("pass");
    }
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area.is_empty(), "nothing was played");
    assert!(r.game.players[1]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "LV2"));
    assert!(r.game.players[1]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "LV4"));
    assert_eq!(r.security_count(1), 0, "the security card was checked");
}
