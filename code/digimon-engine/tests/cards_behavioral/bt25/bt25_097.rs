//! BT25-097 Guardian Palace — Option, Yellow/Purple, use cost 3. Traits: Iliad, TS.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT25-097.md)
//!
//! While you have no face-up security cards, you can ignore this card's color
//! requirements.
//! {Security} [All Turns] All of your yellow or purple [TS] trait Digimon gain
//! ＜Alliance＞. While you have a Digimon with [Junomon] in its name, they also
//! gain ＜Scapegoat＞.
//! [Main] Add your bottom security card to the hand and place this card face up
//! as the bottom security card. Then, you may play 1 yellow or purple [TS] trait
//! Digimon card from your hand with the cost reduced by 3.
//!
//! Security effect:
//! [Security] You may play 1 level 4 or lower yellow or purple [TS] trait
//! Digimon card from your hand or trash without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_097.cs — IgnoreColorCondition
//! (0 face-up security), AllianceStaticEffect + AddSkillClass(ScapegoatSelfEffect)
//! over own Digimon whose top card is yellow OR purple AND [TS], gated on this
//! card face-up in security (+ an own [Junomon]-named permanent for Scapegoat),
//! ReplaceBottomSecurityWithFaceUpOptionEffect + PlayByEffect(-3), Security
//! PlayByEffect free from hand or trash.
//!
//! Prior UNREVIEWED draft: 38d596b56 (re-verified clause by clause; its
//! structure is kept, these tests are new).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! D3 color bypass (use_requirement), D4/D5 security keyword aura + conditional
//! replacement-keyword aura (Scapegoat), C5/B2 option Main
//! (replace-bottom-security + reduced play), F9 security play from hand/trash.

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT25-097";

fn ts_digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.traits = vec!["TS".to_string()];
    card.level = Some(level);
    card.dp = Some(1000 * level as i32);
    card.play_cost = u16::from(level);
    card
}

fn plain_digimon(id: &str, color: CardColor) -> CardData {
    let mut card = ts_digimon(id, color, 4);
    card.traits = vec![];
    card
}

fn named(id: &str, name: &str, color: CardColor) -> CardData {
    let mut card = ts_digimon(id, color, 6);
    card.card_name = name.to_string();
    card
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-097 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(ts_digimon("Y-TS", CardColor::Yellow, 4))
        .add_card(ts_digimon("P-TS", CardColor::Purple, 4))
        .add_card(ts_digimon("R-TS", CardColor::Red, 4))
        .add_card(ts_digimon("Y-TS6", CardColor::Yellow, 6))
        .add_card(ts_digimon("Y-TS5", CardColor::Yellow, 5))
        .add_card(plain_digimon("Y-PLAIN", CardColor::Yellow))
        .add_card(plain_digimon("RED", CardColor::Red))
        .add_card(named("JUNO", "Junomon", CardColor::Purple))
        .add_card(named("NOTJUNO", "Minervamon", CardColor::Purple))
        .add_card({
            // A [Junomon]-named Digimon that does NOT itself gain the aura
            // (red, no [TS]) — isolates the Q6463 probe to one Scapegoat.
            let mut c = plain_digimon("JUNO-R", CardColor::Red);
            c.card_name = "Junomon".to_string();
            c
        })
        .add_card(plain_digimon("ATK", CardColor::Blue))
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

fn push_trash(r: &mut DebugRunner, p: u8, id: &str) {
    let idx = data_idx(r, id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .trash
        .push(digimon_engine::card_source::CardSource::new(idx, p, next));
}

/// Seat Guardian Palace in P0's security, face up or face down.
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

fn hand_action(r: &DebugRunner, p: u8, id: &str) -> u16 {
    let i = r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap();
    PLAY_HAND_START + i as u16
}

fn use_palace(r: &mut DebugRunner) -> String {
    let idx = push_hand(r, 0, CARD_ID);
    format!("{:?}", r.game.play_option_from_hand(0, idx))
}

// ─── Section 1: structure ────────────────────────────────────────────────────

#[test]
fn bt25_097_compiles_color_bypass_two_security_auras_main_and_security() {
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
    assert_eq!(kw_auras, 2, "<Alliance> + conditional <Scapegoat>");
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::MainFromHand])));
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited
            && t.when == vec![CompiledTiming::OnSecurity])));
}

// ─── Clause 1: color-requirement bypass ──────────────────────────────────────

#[test]
fn bt25_097_usable_off_color_with_no_face_up_security() {
    let mut r = builder().security(0, &["FILL", "FILL"]).start();
    r.place_on_field(0, "RED", Some(0)); // no yellow/purple anywhere
    let res = use_palace(&mut r);
    assert_ne!(res, "Invalid", "no face-up security → color requirement ignored");
}

#[test]
fn bt25_097_off_color_is_unusable_with_a_face_up_security_card() {
    let mut r = builder().security(0, &["FILL", "FILL"]).start();
    r.place_on_field(0, "RED", Some(0));
    let ci = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(ci);
    let res = use_palace(&mut r);
    assert_eq!(res, "Invalid", "a face-up security card closes the bypass");
}

#[test]
fn bt25_097_face_up_security_still_usable_when_colors_match() {
    // §4-22-3: a multicolour Option needs every one of its colours.
    let mut r = builder().security(0, &["FILL", "FILL"]).start();
    r.place_on_field(0, "Y-TS", Some(0));
    let ci = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(ci);
    assert_eq!(use_palace(&mut r), "Invalid", "yellow alone is not enough");
    r.game.players[0].hand.pop();
    r.place_on_field(0, "P-TS", Some(0));
    let res = use_palace(&mut r);
    assert_ne!(res, "Invalid", "normal color matching still applies");
}

#[test]
fn bt25_097_bypass_does_not_unlock_other_options() {
    // The bypass is THIS card's; another off-colour Option stays locked.
    let mut other = make_test_card("OTHER-OPT", "Other Option");
    other.card_kind = CardKind::Option;
    other.colors = vec![CardColor::Green];
    other.level = None;
    other.dp = None;
    other.play_cost = 1;
    let mut r = builder()
        .add_card(other)
        .security(0, &["FILL", "FILL"])
        .start();
    r.place_on_field(0, "RED", Some(0));
    push_hand(&mut r, 0, CARD_ID); // Palace in hand, no face-up security
    let idx = push_hand(&mut r, 0, "OTHER-OPT");
    let res = format!("{:?}", r.game.play_option_from_hand(0, idx));
    assert_eq!(res, "Invalid");
}

// ─── Clause 3: [Main] ────────────────────────────────────────────────────────

#[test]
fn bt25_097_main_replaces_bottom_security_and_plays_yellow_or_purple_ts_reduced() {
    let mut r = builder().security(0, &["FILL", "FILL"]).start();
    let bottom_ci = r.game.players[0].security[0].card_index;
    push_hand(&mut r, 0, "Y-TS6");
    push_hand(&mut r, 0, "P-TS");
    push_hand(&mut r, 0, "R-TS");
    push_hand(&mut r, 0, "Y-PLAIN");
    let mem = r.memory();
    let res = use_palace(&mut r);
    assert_ne!(res, "Invalid");

    // Bottom security card → hand; Palace face up as the new bottom card.
    let sec = &r.game.players[0].security;
    assert_eq!(sec.len(), 2);
    assert_eq!(sec[0].card_id(&r.game.card_data), CARD_ID);
    assert!(r.game.players[0].face_up_security.contains(&sec[0].card_index));
    assert!(r.game.players[0].hand.iter().any(|c| c.card_index == bottom_ci));

    let v = r.pending_selection_view().expect("reduced-play prompt");
    assert!(v.is_optional, "'you may play'");
    assert!(v.valid_action_ids.contains(&hand_action(&r, 0, "Y-TS6")));
    assert!(v.valid_action_ids.contains(&hand_action(&r, 0, "P-TS")));
    assert!(!v.valid_action_ids.contains(&hand_action(&r, 0, "R-TS")), "red");
    assert!(!v.valid_action_ids.contains(&hand_action(&r, 0, "Y-PLAIN")), "not [TS]");
    let a = hand_action(&r, 0, "Y-TS6");
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"Y-TS6".to_string()));
    assert_eq!(r.memory(), mem - 3 - (6 - 3), "use 3 + play 6 reduced by 3");
}

#[test]
fn bt25_097_main_play_is_optional() {
    let mut r = builder().security(0, &["FILL"]).start();
    push_hand(&mut r, 0, "Y-TS");
    let mem = r.memory();
    use_palace(&mut r);
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(r.memory(), mem - 3);
    assert_eq!(ids(&r, 0, "security"), vec![CARD_ID.to_string()]);
}

#[test]
fn bt25_097_main_with_zero_security_just_places_this_card_face_up() {
    // Q6458: usable with 0 security cards — nothing is added to the hand; the
    // card is only placed face up in the security stack.
    let mut r = builder().start();
    assert!(r.game.players[0].security.is_empty());
    push_hand(&mut r, 0, "Y-TS");
    let hand_before = r.game.players[0].hand.len(); // Y-TS only
    let res = use_palace(&mut r);
    assert_ne!(res, "Invalid", "usable with 0 security cards");
    assert_eq!(ids(&r, 0, "security"), vec![CARD_ID.to_string()]);
    let palace = r.game.players[0].security[0].card_index;
    assert!(r.game.players[0].face_up_security.contains(&palace), "face up");
    // The play is still offered; decline it.
    let v = r.pending_selection_view().expect("reduced-play prompt");
    assert!(v.is_optional);
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.game.players[0].hand.len(), hand_before, "no card added to the hand");
    assert!(!ids(&r, 0, "trash").contains(&CARD_ID.to_string()));
}

// ─── Clause 2: {Security} [All Turns] auras ──────────────────────────────────

#[test]
fn bt25_097_face_up_in_security_grants_alliance_to_yellow_or_purple_ts() {
    let mut r = builder().start();
    let y = r.place_on_field(0, "Y-TS", Some(0));
    let p = r.place_on_field(0, "P-TS", Some(0));
    let red = r.place_on_field(0, "R-TS", Some(0));
    let plain = r.place_on_field(0, "Y-PLAIN", Some(0));
    seat_in_security(&mut r, true);
    assert!(r.game.has_keyword(y, Keyword::Alliance));
    assert!(r.game.has_keyword(p, Keyword::Alliance));
    assert!(!r.game.has_keyword(red, Keyword::Alliance), "red");
    assert!(!r.game.has_keyword(plain, Keyword::Alliance), "not [TS]");
    // [All Turns]: still on during the opponent's turn.
    r.end_turn();
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(y, Keyword::Alliance));
}

#[test]
fn bt25_097_face_down_in_security_grants_nothing() {
    let mut r = builder().start();
    let y = r.place_on_field(0, "Y-TS", Some(0));
    r.place_on_field(0, "JUNO", Some(0));
    seat_in_security(&mut r, false);
    assert!(!r.game.has_keyword(y, Keyword::Alliance));
    assert!(!r.game.has_keyword(y, Keyword::Scapegoat));
}

#[test]
fn bt25_097_scapegoat_only_while_you_have_a_junomon() {
    let mut r = builder().start();
    let y = r.place_on_field(0, "Y-TS", Some(0));
    r.place_on_field(0, "NOTJUNO", Some(0));
    seat_in_security(&mut r, true);
    assert!(!r.game.has_keyword(y, Keyword::Scapegoat), "no [Junomon] → no Scapegoat");
    r.place_on_field(0, "JUNO", Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(y, Keyword::Scapegoat));
}

#[test]
fn bt25_097_granted_scapegoat_substitutes_another_digimon_for_the_deletion() {
    let mut r = builder().start();
    let y = r.place_on_field(0, "Y-TS", Some(0));
    let _juno = r.place_on_field(0, "JUNO", Some(0));
    let _fodder = r.place_on_field(0, "RED", Some(0));
    seat_in_security(&mut r, true);
    r.game.delete_permanent_with_cause(y, ReplacementCause::OpponentEffect);
    let v = r.pending_selection_view().expect("<Scapegoat> offer");
    assert!(v.is_optional, "<Scapegoat> is optional");
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    // Pick the RED Digimon to delete instead.
    let red_slot = field_ids(&r, 0).iter().position(|s| s == "RED").unwrap() as u16;
    let action = digimon_engine::action::space::encode_attack(0, red_slot);
    let v = r.pending_selection_view().expect("Scapegoat pick");
    assert!(v.valid_action_ids.contains(&action), "{:?}", v.valid_action_ids);
    r.execute_action(0, action).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"Y-TS".to_string()), "protected");
    assert!(ids(&r, 0, "trash").contains(&"RED".to_string()), "deleted instead");
}

#[test]
fn bt25_097_scapegoat_still_resolves_when_the_junomon_is_deleted_in_the_same_batch() {
    // Q6463: the [Junomon] and a yellow/purple [TS] Digimon would be deleted
    // at the same time; the granted <Scapegoat> activates and the [Junomon] is
    // deleted, so <Scapegoat> is lost — but processing that has started is
    // performed to the end, so the Digimon is still prevented from deletion.
    let mut r = builder().start();
    let y = r.place_on_field(0, "Y-TS", Some(0));
    let juno = r.place_on_field(0, "JUNO-R", Some(0));
    let _fodder = r.place_on_field(0, "RED", Some(0));
    seat_in_security(&mut r, true);
    assert!(r.game.has_keyword(y, Keyword::Scapegoat));
    let _ = r
        .game
        .delete_permanents_batch(vec![y, juno], ReplacementCause::OpponentEffect);
    let v = r.pending_selection_view().expect("<Scapegoat> offer");
    assert!(v.is_optional);
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    let red_slot = field_ids(&r, 0).iter().position(|s| s == "RED").unwrap() as u16;
    let action = digimon_engine::action::space::encode_attack(0, red_slot);
    let v = r.pending_selection_view().expect("Scapegoat pick");
    assert!(v.valid_action_ids.contains(&action), "{:?}", v.valid_action_ids);
    r.execute_action(0, action).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["Y-TS".to_string()], "protected; the others are gone");
    let trash = ids(&r, 0, "trash");
    assert!(trash.contains(&"JUNO-R".to_string()), "the [Junomon] was deleted");
    assert!(trash.contains(&"RED".to_string()), "deleted instead");
    assert!(!trash.contains(&"Y-TS".to_string()));
    let y_now = PermanentHandle { player: 0, index: 0 };
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(y_now, Keyword::Scapegoat), "no [Junomon] left → Scapegoat lost");
}

#[test]
fn bt25_097_no_scapegoat_offer_without_junomon() {
    let mut r = builder().start();
    let y = r.place_on_field(0, "Y-TS", Some(0));
    r.place_on_field(0, "RED", Some(0));
    seat_in_security(&mut r, true);
    r.game.delete_permanent_with_cause(y, ReplacementCause::OpponentEffect);
    assert!(r.game.pending_selection.is_none(), "no prevention offer");
    let _ = r.auto_resolve();
    assert!(ids(&r, 0, "trash").contains(&"Y-TS".to_string()));
}

#[test]
fn bt25_097_scapegoat_not_granted_to_red_ts() {
    let mut r = builder().start();
    let red = r.place_on_field(0, "R-TS", Some(0));
    r.place_on_field(0, "JUNO", Some(0));
    seat_in_security(&mut r, true);
    assert!(!r.game.has_keyword(red, Keyword::Scapegoat));
}

// ─── Clause 4: [Security] ────────────────────────────────────────────────────

fn check_palace(r: &mut DebugRunner) {
    let atk: PermanentHandle = r.place_on_field(1, "ATK", Some(0));
    seat_in_security(r, false);
    r.set_first_player(1);
    r.attack_player(atk, 0, false);
}

#[test]
fn bt25_097_security_plays_level_four_yellow_or_purple_ts_from_hand_free() {
    let mut r = builder().start();
    push_hand(&mut r, 0, "Y-TS");
    push_hand(&mut r, 0, "Y-TS5");
    push_hand(&mut r, 0, "R-TS");
    push_hand(&mut r, 0, "Y-PLAIN");
    let mem = r.memory();
    check_palace(&mut r);
    let v = r.pending_selection_view().expect("security play prompt");
    assert!(v.is_optional, "'you may play'");
    let rows = physical_choice::commits(&r.game);
    assert_eq!(rows.len(), 1, "level 5, red, and non-[TS] cards are excluded");
    assert_eq!(rows[0].card_id.as_deref(), Some("Y-TS"));
    assert!(matches!(rows[0].zone, Some(digimon_engine::owned_card_choice::CardChoiceZone::Hand)));
    assert!(v.valid_action_ids.contains(&PASS), "explicit optional decline");
    let a = physical_choice::card_action(&r.game, "Y-TS", digimon_engine::owned_card_choice::CardChoiceZone::Hand);
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"Y-TS".to_string()));
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn bt25_097_security_can_play_from_trash() {
    let mut r = builder().start();
    push_trash(&mut r, 0, "P-TS");
    check_palace(&mut r);
    let v = r.pending_selection_view().expect("security play prompt");
    assert!(v.is_optional);
    assert!(v.valid_action_ids.contains(&PASS));
    let a = physical_choice::card_action(&r.game, "P-TS", digimon_engine::owned_card_choice::CardChoiceZone::Trash);
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"P-TS".to_string()));
    assert!(!ids(&r, 0, "trash").contains(&"P-TS".to_string()));
}

#[test]
fn bt25_097_security_play_is_optional() {
    let mut r = builder().start();
    push_hand(&mut r, 0, "Y-TS");
    check_palace(&mut r);
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert!(ids(&r, 0, "hand").contains(&"Y-TS".to_string()));
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_choice;
