//! EX8-071 Nightmare Soldiers — Option, Purple, use cost 2. Traits: [NSo].
//!
//! # Card text (official Bandai DB — data/card_bundles/EX8-071.md)
//!
//! While you have no face-up security cards, you may ignore this card's color
//! requirements.
//! {Security} [All Turns] All of your level 4 or higher Digimon with the [NSo]
//! trait gain ＜Scapegoat＞.
//! [Main] Add your bottom security card to the hand. Then, place this card face
//! up as the bottom security card.
//!
//! Security effect:
//! [Security] You may play 1 level 5 or lower Digimon card with the [NSo] trait
//! from your hand without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Purple/EX8_071.cs — IgnoreColorCondition
//! (0 face-up security), AddSkillClass(ScapegoatSelfEffect) over the owner's
//! battle-area Digimon whose top card is level >= 4 AND EqualsTraits("NSo"),
//! gated on this card face-up in security, ReplaceBottomSecurityWithFaceUp-
//! OptionMainEffect, and a `canNoSelect` hand pick (level <= 5, [NSo],
//! CanPlayAsNewPermanent payCost:false) → PlayPermanentCards free.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! D3 colour bypass (use_requirement), D4/D5 security keyword aura
//! (Scapegoat), C5 option Main (replace-bottom-security), F9 security play
//! from hand.

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};

use super::nso_support::{offered_ids, pick_hand};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "EX8-071";

fn nso(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.traits = vec!["NSo".to_string()];
    card.level = Some(level);
    card.dp = Some(1000 * level as i32);
    card.play_cost = u16::from(level);
    card
}

fn non_nso(id: &str, level: u8) -> CardData {
    let mut card = nso(id, level);
    card.traits = vec!["Other".to_string()];
    card
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-071 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(nso("N3", 3))
        .add_card(nso("N4", 4))
        .add_card(nso("N5", 5))
        .add_card(nso("N6", 6))
        .add_card(non_nso("X5", 5))
        .add_card(non_nso("X4", 4))
        .add_card({
            // Off-colour NSo Digimon: the Option's colour bypass must not
            // depend on the board; this is only a board filler.
            let mut c = nso("RED", 4);
            c.colors = vec![CardColor::Red];
            c
        })
        .add_card(non_nso("ATK", 4))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
}

fn data_idx(r: &DebugRunner, id: &str) -> usize {
    r.game.card_data.iter().position(|c| c.card_id == id).unwrap()
}

fn push_hand(r: &mut DebugRunner, p: u8, id: &str) -> usize {
    let idx = data_idx(r, id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .hand
        .push(digimon_engine::card_source::CardSource::new(idx, p, next));
    r.game.players[p as usize].hand.len() - 1
}

/// Seat Nightmare Soldiers in P0's security, face up or face down.
fn seat_in_security(r: &mut DebugRunner, face_up: bool) {
    let idx = data_idx(r, CARD_ID);
    let next = r.game.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(idx, 0, next);
    let ci = card.card_index;
    r.game.players[0].security.push(card);
    if face_up {
        r.game.players[0].face_up_security.insert(ci);
    }
    r.game.tick_declarative_effects();
}

fn ids(r: &DebugRunner, p: u8, zone: &str) -> Vec<String> {
    let pl = &r.game.players[p as usize];
    let v: Vec<&digimon_engine::card_source::CardSource> = match zone {
        "hand" => pl.hand.iter().collect(),
        "trash" => pl.trash.iter().collect(),
        "security" => pl.security.iter().collect(),
        _ => unreachable!(),
    };
    v.iter().map(|c| c.card_id(&r.game.card_data).to_string()).collect()
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn use_option(r: &mut DebugRunner) -> String {
    let idx = push_hand(r, 0, CARD_ID);
    format!("{:?}", r.game.play_option_from_hand(0, idx))
}

// ─── Structure ───────────────────────────────────────────────────────────────

#[test]
fn ex8_071_compiles_color_bypass_security_aura_main_and_security() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card.use_requirement.is_some(), "color-requirement bypass");
    let kw_auras = card
        .effects
        .iter()
        .filter(|c| {
            matches!(c, CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Security,
                grant_keyword: Some(_),
                ..
            }))
        })
        .count();
    assert_eq!(kw_auras, 1, "<Scapegoat> security aura");
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::MainFromHand])));
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited
            && t.when == vec![CompiledTiming::OnSecurity])));
}

// ─── Clause 1: color-requirement bypass ──────────────────────────────────────

#[test]
fn ex8_071_usable_off_color_with_no_face_up_security() {
    let mut r = builder().security(0, &["FILL", "FILL"]).start();
    r.place_on_field(0, "RED", Some(0)); // no purple anywhere
    let res = use_option(&mut r);
    assert_ne!(res, "Invalid", "no face-up security → color requirement ignored");
}

#[test]
fn ex8_071_off_color_is_unusable_with_a_face_up_security_card() {
    let mut r = builder().security(0, &["FILL", "FILL"]).start();
    r.place_on_field(0, "RED", Some(0));
    let ci = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(ci);
    let res = use_option(&mut r);
    assert_eq!(res, "Invalid", "a face-up security card closes the bypass");
}

#[test]
fn ex8_071_face_up_security_still_usable_when_the_color_matches() {
    let mut r = builder().security(0, &["FILL", "FILL"]).start();
    r.place_on_field(0, "N4", Some(0)); // purple
    let ci = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(ci);
    let res = use_option(&mut r);
    assert_ne!(res, "Invalid", "normal color matching still applies");
}

// ─── Clause 3: [Main] ────────────────────────────────────────────────────────

#[test]
fn ex8_071_main_replaces_the_bottom_security_card_with_this_card_face_up() {
    let mut r = builder().security(0, &["FILL", "N4"]).start();
    r.place_on_field(0, "N4", Some(0));
    let bottom_ci = r.game.players[0].security[0].card_index;
    let top_ci = r.game.players[0].security[1].card_index;
    let mem = r.memory();
    let res = use_option(&mut r);
    assert_ne!(res, "Invalid");

    // Bottom security card → hand; this card face up as the new bottom card.
    let sec = &r.game.players[0].security;
    assert_eq!(sec.len(), 2, "one out, one in");
    assert_eq!(sec[0].card_id(&r.game.card_data), CARD_ID);
    assert!(r.game.players[0].face_up_security.contains(&sec[0].card_index));
    assert_eq!(sec[1].card_index, top_ci, "the top security card is untouched");
    assert!(r.game.players[0].hand.iter().any(|c| c.card_index == bottom_ci));
    assert!(r.game.pending_selection.is_none(), "[Main] is mandatory and prompt-free");
    assert_eq!(r.memory(), mem - 2, "use cost 2");
}

#[test]
fn ex8_071_main_with_zero_security_just_places_this_card_face_up() {
    let mut r = builder().start();
    r.place_on_field(0, "N4", Some(0));
    assert!(r.game.players[0].security.is_empty());
    let hand_before = r.game.players[0].hand.len();
    let res = use_option(&mut r);
    assert_ne!(res, "Invalid", "usable with 0 security cards");
    assert_eq!(ids(&r, 0, "security"), vec![CARD_ID.to_string()]);
    let me = r.game.players[0].security[0].card_index;
    assert!(r.game.players[0].face_up_security.contains(&me), "face up");
    assert_eq!(r.game.players[0].hand.len(), hand_before, "nothing added to the hand");
    assert!(!ids(&r, 0, "trash").contains(&CARD_ID.to_string()));
}

// ─── Clause 2: {Security} [All Turns] Scapegoat aura ─────────────────────────

#[test]
fn ex8_071_face_up_in_security_grants_scapegoat_to_level_4_plus_nso() {
    let mut r = builder().start();
    let n4 = r.place_on_field(0, "N4", Some(0));
    let n6 = r.place_on_field(0, "N6", Some(0));
    let n3 = r.place_on_field(0, "N3", Some(0));
    let x5 = r.place_on_field(0, "X5", Some(0));
    seat_in_security(&mut r, true);
    assert!(r.game.has_keyword(n4, Keyword::Scapegoat), "level 4 [NSo]");
    assert!(r.game.has_keyword(n6, Keyword::Scapegoat), "level 6 [NSo]");
    assert!(!r.game.has_keyword(n3, Keyword::Scapegoat), "level 3 is below the floor");
    assert!(!r.game.has_keyword(x5, Keyword::Scapegoat), "no [NSo] trait");
    // [All Turns]: still on during the opponent's turn.
    r.end_turn();
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(n4, Keyword::Scapegoat));
}

#[test]
fn ex8_071_face_down_in_security_grants_nothing() {
    let mut r = builder().start();
    let n4 = r.place_on_field(0, "N4", Some(0));
    seat_in_security(&mut r, false);
    assert!(!r.game.has_keyword(n4, Keyword::Scapegoat));
}

#[test]
fn ex8_071_scapegoat_is_not_granted_to_the_opponents_digimon() {
    let mut r = builder().start();
    let theirs = r.place_on_field(1, "N5", Some(0));
    seat_in_security(&mut r, true);
    assert!(!r.game.has_keyword(theirs, Keyword::Scapegoat));
}

#[test]
fn ex8_071_granted_scapegoat_substitutes_another_digimon_for_the_deletion() {
    let mut r = builder().start();
    let n5 = r.place_on_field(0, "N5", Some(0));
    let _fodder = r.place_on_field(0, "X4", Some(0));
    seat_in_security(&mut r, true);
    r.game.delete_permanent_with_cause(n5, ReplacementCause::OpponentEffect);
    let v = r.pending_selection_view().expect("<Scapegoat> offer");
    assert!(v.is_optional, "<Scapegoat> is optional");
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    // Pick the other Digimon to delete instead.
    let slot = field_ids(&r, 0).iter().position(|s| s == "X4").unwrap() as u16;
    let action = digimon_engine::action::space::encode_attack(0, slot);
    let v = r.pending_selection_view().expect("Scapegoat pick");
    assert!(v.valid_action_ids.contains(&action), "{:?}", v.valid_action_ids);
    r.execute_action(0, action).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"N5".to_string()), "protected");
    assert!(ids(&r, 0, "trash").contains(&"X4".to_string()), "deleted instead");
}

#[test]
fn ex8_071_no_scapegoat_offer_for_a_level_3_nso_digimon() {
    let mut r = builder().start();
    let n3 = r.place_on_field(0, "N3", Some(0));
    r.place_on_field(0, "X4", Some(0));
    seat_in_security(&mut r, true);
    r.game.delete_permanent_with_cause(n3, ReplacementCause::OpponentEffect);
    assert!(r.game.pending_selection.is_none(), "no prevention offer");
    let _ = r.auto_resolve();
    assert!(ids(&r, 0, "trash").contains(&"N3".to_string()));
}

// ─── Clause 4: [Security] ────────────────────────────────────────────────────

fn check_option(r: &mut DebugRunner) {
    let atk = r.place_on_field(1, "ATK", Some(0));
    seat_in_security(r, false);
    r.set_first_player(1);
    r.attack_player(atk, 0, false);
}

#[test]
fn ex8_071_security_plays_a_level_5_or_lower_nso_digimon_from_hand_free() {
    let mut r = builder().start();
    push_hand(&mut r, 0, "N5");
    push_hand(&mut r, 0, "N6");
    push_hand(&mut r, 0, "X5");
    let mem = r.memory();
    check_option(&mut r);
    let v = r.pending_selection_view().expect("security play prompt");
    assert!(v.is_optional, "'you may play'");
    let offered = offered_ids(&r);
    assert_eq!(offered, vec!["N5".to_string()], "only the level 5 [NSo]; not level 6, not non-[NSo]");
    pick_hand(&mut r, 0, "N5");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"N5".to_string()));
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn ex8_071_security_play_is_optional() {
    let mut r = builder().start();
    push_hand(&mut r, 0, "N4");
    check_option(&mut r);
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert!(ids(&r, 0, "hand").contains(&"N4".to_string()));
}

#[test]
fn ex8_071_security_offers_nothing_when_no_hand_card_qualifies() {
    let mut r = builder().start();
    push_hand(&mut r, 0, "N6");
    push_hand(&mut r, 0, "X4");
    check_option(&mut r);
    assert!(r.pending_selection_view().is_none(), "no legal pick, no prompt");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
}

#[test]
fn ex8_071_security_does_not_play_from_the_trash() {
    let mut r = builder().start();
    let idx = data_idx(&r, "N4");
    let next = r.game.next_card_index();
    r.game.players[0]
        .trash
        .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    check_option(&mut r);
    assert!(r.pending_selection_view().is_none(), "hand only");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
}
