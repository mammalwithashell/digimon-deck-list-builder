//! BT4-105 Tactical Retreat! — Option, Yellow, use cost 1.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT4-105.md)
//!
//! [Main] Place 1 of your Digimon on top of your security stack face down.
//! Security effect: [Security] ＜Recovery +1 (Deck)＞ (Place the top card of
//! your deck on top of your security stack.)
//!
//! The original printing (the local card image) also read "Trash all of the
//! digivolution cards of that Digimon."; the current official text drops the
//! sentence because Comprehensive Rules 4.3 §4-7-8 trashes the cards under a
//! card that leaves the field at the same time. DCGO (BT4/Yellow/BT4_105.cs,
//! IPutSecurityPermanent) does both.
//! Official Q&A: the Digimon is not deleted, so [On Deletion] does not activate.
//!
//! DCGO: mandatory own-Digimon pick (canNoSelect: false) gated on
//! CanAddSecurity → IPutSecurityPermanent(toTop: true); Security: IRecovery(1).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! Option [Main] mandatory own-permanent pick → place_on_security (permanent
//! source, face down, top); inherited [Security] recover 1.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT4-105";

/// Test-only Digimon whose [On Deletion] gains 3 memory (Q&A probe).
const ONDEL_YAML: &str = r#"
card: T-ONDEL
name: On Deletion Probe
kind: digimon
level: 4
color: [yellow]
cost: 4
dp: 4000
traits: []
effects:
  - when: on_deletion
    summary: "[On Deletion] Gain 3 memory"
    process:
      - gain_memory: 3
"#;

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = level as u16;
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Yellow];
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c
}

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT4-105 YAML")
        .from_dsl_yaml(ONDEL_YAML)
        .expect("probe")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(make_test_card("DECKTOP", "Deck Top"))
        .add_card(digimon("ROOKIE", 3))
        .add_card(digimon("CHAMP", 4))
        .add_card(digimon("ULT", 5))
        .add_card(tamer("TAMER"))
        .add_card(digimon("ATK", 5))
        .deck(0, &["FILL", "FILL", "DECKTOP"])
        .deck(1, &["FILL"; 4])
        .security(0, &["FILL", "FILL"])
        .security(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.set_first_player(0);
    r
}

fn data_idx(r: &DebugRunner, id: &str) -> usize {
    r.game.card_data.iter().position(|c| c.card_id == id).unwrap()
}

fn use_retreat(r: &mut DebugRunner) {
    let idx = data_idx(r, CARD_ID);
    let next = r.game.next_card_index();
    r.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    let h = r.game.players[0].hand.len() - 1;
    let res = r.game.play_option_from_hand(0, h);
    assert_ne!(format!("{res:?}"), "Invalid");
}

fn security_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .security
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .trash
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

fn own_action(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

// ─── Section 1: structure ────────────────────────────────────────────────────

#[test]
fn bt4_105_main_is_mandatory_and_security_is_inherited() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => Some(t),
            _ => None,
        })
        .expect("[Main]");
    assert!(!main.optional, "no 'you may' in [Main]");
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited
            && t.when.contains(&CompiledTiming::OnSecurity))));
}

// ─── Clause 1: [Main] ────────────────────────────────────────────────────────

#[test]
fn bt4_105_main_places_chosen_digimon_face_down_on_top_and_trashes_its_sources() {
    let mut r = setup();
    let stack = r.place_stack(0, &["ROOKIE", "CHAMP", "ULT"]);
    let other = r.place_on_field(0, "CHAMP", Some(0));
    use_retreat(&mut r);
    let v = r.pending_selection_view().expect("Digimon pick");
    assert!(!v.is_optional, "mandatory pick");
    assert!(v.valid_action_ids.contains(&own_action(stack)));
    assert!(v.valid_action_ids.contains(&own_action(other)));
    r.execute_action(0, own_action(stack)).unwrap();
    let _ = r.auto_resolve();

    let sec = &r.game.players[0].security;
    assert_eq!(sec.len(), 3);
    let top = sec.last().unwrap();
    assert_eq!(top.card_id(&r.game.card_data), "ULT", "the top card goes on top");
    assert!(
        !r.game.players[0].face_up_security.contains(&top.card_index),
        "face down"
    );
    assert_eq!(field_ids(&r, 0), vec!["CHAMP".to_string()]);
    let trash = trash_ids(&r, 0);
    assert!(trash.contains(&"ROOKIE".to_string()), "§4-7-8 sources trashed");
    assert!(trash.contains(&"CHAMP".to_string()));
    assert!(trash.contains(&CARD_ID.to_string()), "the used Option is trashed");
}

#[test]
fn bt4_105_main_offers_only_your_digimon() {
    let mut r = setup();
    let mine = r.place_on_field(0, "ROOKIE", Some(0));
    let tamer = r.place_on_field(0, "TAMER", Some(0));
    r.place_on_field(1, "CHAMP", Some(0));
    use_retreat(&mut r);
    let v = r.pending_selection_view().expect("Digimon pick");
    assert!(v.valid_action_ids.contains(&own_action(mine)));
    assert!(!v.valid_action_ids.contains(&own_action(tamer)), "a Tamer is not a Digimon");
    assert!(!v.valid_action_ids.contains(&PASS), "mandatory");
    assert_eq!(
        v.valid_action_ids.len(),
        1,
        "the opponent's Digimon is not offered: {:?}",
        v.valid_action_ids
    );
    r.execute_action(0, own_action(mine)).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(security_ids(&r, 0).last().map(String::as_str), Some("ROOKIE"));
    assert_eq!(field_ids(&r, 1), vec!["CHAMP".to_string()]);
}

#[test]
fn bt4_105_main_does_nothing_without_a_digimon() {
    let mut r = setup();
    r.place_on_field(0, "TAMER", Some(0));
    let mem = r.memory();
    use_retreat(&mut r);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(security_ids(&r, 0).len(), 2);
    assert_eq!(r.memory(), mem - 1, "use cost 1");
}

#[test]
fn bt4_105_placed_digimon_is_not_deleted() {
    // Q&A: it isn't deleted, so [On Deletion] effects don't activate.
    let mut r = setup();
    let probe = r.place_on_field(0, "T-ONDEL", Some(0));
    let mem = r.memory();
    use_retreat(&mut r);
    r.execute_action(0, own_action(probe)).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(security_ids(&r, 0).last().map(String::as_str), Some("T-ONDEL"));
    assert_eq!(r.memory(), mem - 1, "no [On Deletion] memory gain");
}

#[test]
fn bt4_105_main_offers_nothing_when_you_cannot_add_security() {
    // DCGO CanSelectPermanentCondition requires Owner.CanAddSecurity.
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = setup();
    r.place_on_field(0, "ROOKIE", Some(0));
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotAddSecurityByEffect, 1, Expiry::EndOfTurn, None, 1),
    );
    use_retreat(&mut r);
    assert!(r.game.pending_selection.is_none(), "no placeable Digimon → no pick");
    assert_eq!(field_ids(&r, 0), vec!["ROOKIE".to_string()]);
    assert_eq!(security_ids(&r, 0).len(), 2);
}

// ─── Clause 2: [Security] <Recovery +1 (Deck)> ───────────────────────────────

#[test]
fn bt4_105_security_recovers_the_deck_top_onto_security() {
    let mut r = setup();
    let atk = r.place_on_field(1, "ATK", Some(0));
    let idx = data_idx(&r, CARD_ID);
    let next = r.game.next_card_index();
    r.game.players[0]
        .security
        .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    let deck_top = r.game.players[0].deck.last().unwrap().card_id(&r.game.card_data).to_string();
    let deck_len = r.game.players[0].deck.len();
    r.set_first_player(1);
    r.attack_player(atk, 0, false);
    let _ = r.auto_resolve();
    // 3 security − 1 checked + 1 recovered.
    assert_eq!(r.game.players[0].security.len(), 3);
    assert_eq!(security_ids(&r, 0).last(), Some(&deck_top), "on top of the stack");
    assert_eq!(r.game.players[0].deck.len(), deck_len - 1);
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}
