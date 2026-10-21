//! BT2-047 Argomon — Digimon, Lv.5, Green, DP 6000, Cost 8.
//! Traits: Mutant. Form: Ultimate. Attribute: Virus.
//! Digivolve: green Lv.4, cost 3.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT2-047.md; card image agrees)
//!
//! ```text
//! ＜Digisorption -3＞ (When one of your Digimon digivolves into this card from
//! your hand, you may suspend 1 of your Digimon to reduce the memory cost of
//! the digivolution by 3.)
//! Inherited: [When Attacking] You may play 1 level 3 green Digimon card from
//! your hand suspended without paying its memory cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT2/Green/BT2_047.cs
//!   - BeforePayCost "Digisorption -3" (optional), as BT3-054.
//!   - OnAllyAttack (inherited, optional): SelectHandEffect (canNoSelect)
//!     over own level-3 green Digimon cards that can be played without paying
//!     → PlayPermanentCards(payCost: false, isTapped: true, activateETB: true).
//!
//! # Rulings
//! - Q1018: the played Digimon's [On Play] effects activate.
//! - Entering play suspended is not "becoming suspended" (no suspend event).
//!
//! # Patterns this test covers
//! - H-row `<Digisorption -3>` (`grant_keyword` with `value`)
//! - F-row inherited `when_attacking`, optional `select_hand` +
//!   `play_from_hand_free { suspended: true }`

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_digivolve, ATTACK_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT2-047";

fn digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT2-047 in pack")
        .dsl_card("ST4-03")
        .expect("ST4-03 in pack")
        .dsl_card("BT10-053")
        .expect("BT10-053 in pack")
        .add_card(digimon("G-LV4", CardColor::Green, 4))
        .add_card(digimon("G-LV3", CardColor::Green, 3))
        .add_card(digimon("G-LV3-B", CardColor::Green, 3))
        .add_card(digimon("R-LV3", CardColor::Red, 3))
        .add_card(digimon("ALLY", CardColor::Blue, 3))
        .add_card(digimon("OPP", CardColor::Red, 4))
        .add_card(make_test_card("F1", "F1"))
        .deck(0, &["F1"; 10])
        .deck(1, &["F1"; 10])
        .security(1, &["F1"; 5])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.game.set_memory(5);
    r
}

fn hand_idx(r: &DebugRunner, id: &str) -> u16 {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in hand")) as u16
}

fn in_hand(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

fn find_on_field(r: &DebugRunner, id: &str) -> Option<PermanentHandle> {
    r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == id)
        .map(|i| PermanentHandle {
            player: 0,
            index: i as u8,
        })
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn hand_pick(r: &DebugRunner, id: &str) -> u16 {
    let a = PLAY_HAND_START + hand_idx(r, id);
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.valid_action_ids.contains(&a), "{id} offered: {:?}", v.valid_action_ids);
    a
}

/// Argomon stacked under a green Lv.4 top; attack the opponent's player.
fn attack_with_argomon_source(r: &mut DebugRunner) -> PermanentHandle {
    let carrier = r.place_stack(0, &[CARD_ID, "G-LV4"]);
    let _ = r.attack_player(carrier, 1, false);
    carrier
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt2_047_metadata_and_green_lv4_cost3_circle() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Argomon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(8));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.traits, vec!["Mutant".to_string()]);
    assert_eq!(card.attribute, ["Virus"]);
    assert_eq!(card.alt_paths.len(), 1);
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(3)));
}

#[test]
fn bt2_047_digisorption_3_and_optional_inherited_when_attacking_suspended_play() {
    let card = dsl_card_data::compiled(CARD_ID);
    let keywords: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                value,
                scope,
                ..
            }) => Some((keyword.clone(), *value, *scope)),
            _ => None,
        })
        .collect();
    assert_eq!(
        keywords,
        vec![("Digisorption".to_string(), Some(3), CompiledScope::FaceUp)]
    );
    let inh = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .expect("inherited trigger");
    assert_eq!(inh.when, vec![CompiledTiming::WhenAttacking]);
    assert!(inh.optional, "'You may'");
    assert!(!inh.once_per_turn);
    let process = format!("{:?}", inh.process);
    assert!(
        process.contains("PlayFromHandFree") && process.contains("suspended: true"),
        "{process}"
    );
}

// ─── SECTION 2 — <Digisorption -3> ─────────────────────────────────────────

fn digivolve_into_argomon(r: &mut DebugRunner, base: PermanentHandle) {
    r.add_to_hand(0, CARD_ID);
    let h = hand_idx(r, CARD_ID);
    r.game.decode_action(encode_digivolve(h, base.index as u16), 0);
}

#[test]
fn bt2_047_digisorption_suspend_reduces_cost_by_3() {
    let mut r = runner();
    let base = r.place_on_field(0, "G-LV4", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    digivolve_into_argomon(&mut r, base);
    let v = r.pending_selection_view().expect("<Digisorption> offered");
    assert!(v.is_optional && v.selecting_player == 0);
    r.accept_optional_trigger().unwrap();
    r.execute_action(0, ATTACK_START + ally.index as u16).unwrap();
    let _ = r.auto_resolve();
    assert!(find_on_field(&r, CARD_ID).is_some());
    assert!(suspended(&r, ally));
    assert_eq!(r.memory(), 5, "3 - 3 = 0");
}

#[test]
fn bt2_047_digisorption_declined_pays_full_cost() {
    let mut r = runner();
    let base = r.place_on_field(0, "G-LV4", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    digivolve_into_argomon(&mut r, base);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(find_on_field(&r, CARD_ID).is_some());
    assert!(!suspended(&r, ally));
    assert_eq!(r.memory(), 2);
}

// ─── SECTION 3 — Inherited [When Attacking] suspended free play ────────────

/// Accept → pick a level 3 green Digimon → it enters suspended, for free.
#[test]
fn bt2_047_inherited_plays_level3_green_from_hand_suspended_for_free() {
    let mut r = runner();
    r.add_to_hand(0, "G-LV3");
    attack_with_argomon_source(&mut r);
    let v = r.pending_selection_view().expect("[When Attacking] offered");
    assert!(v.is_optional, "'You may'");
    let a = hand_pick(&r, "G-LV3");
    let before = r.memory();
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    let played = find_on_field(&r, "G-LV3").expect("played");
    assert!(suspended(&r, played), "played suspended");
    assert_eq!(r.memory(), before, "without paying its memory cost");
    assert!(!in_hand(&r, "G-LV3"));
}

/// Only level 3 GREEN Digimon cards are offered.
#[test]
fn bt2_047_inherited_offers_only_level3_green_digimon() {
    let mut r = runner();
    r.add_to_hand(0, "G-LV3");
    r.add_to_hand(0, "R-LV3");
    r.add_to_hand(0, "G-LV4");
    attack_with_argomon_source(&mut r);
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.valid_action_ids.contains(&(PLAY_HAND_START + hand_idx(&r, "G-LV3"))));
    assert!(!v.valid_action_ids.contains(&(PLAY_HAND_START + hand_idx(&r, "R-LV3"))), "red");
    assert!(!v.valid_action_ids.contains(&(PLAY_HAND_START + hand_idx(&r, "G-LV4"))), "level 4");
}

/// "You may": declining plays nothing.
#[test]
fn bt2_047_inherited_decline_plays_nothing() {
    let mut r = runner();
    r.add_to_hand(0, "G-LV3");
    attack_with_argomon_source(&mut r);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert!(find_on_field(&r, "G-LV3").is_none());
    assert!(in_hand(&r, "G-LV3"));
}

/// With no qualifying card in the hand nothing is played.
#[test]
fn bt2_047_inherited_nothing_to_play_without_a_level3_green() {
    let mut r = runner();
    r.add_to_hand(0, "R-LV3");
    attack_with_argomon_source(&mut r);
    let _ = r.auto_resolve();
    assert!(find_on_field(&r, "R-LV3").is_none());
    assert!(in_hand(&r, "R-LV3"));
}

/// Inherited only: Argomon as the TOP card does not offer it.
#[test]
fn bt2_047_inherited_inactive_when_argomon_is_the_top_card() {
    let mut r = runner();
    r.add_to_hand(0, "G-LV3");
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let _ = r.attack_player(me, 1, false);
    let _ = r.auto_resolve();
    assert!(find_on_field(&r, "G-LV3").is_none(), "no inherited play from the top card");
}

/// Q1018: the played Digimon's [On Play] activates (ST4-03 reveals the top
/// card and adds it when it is a green Digimon).
#[test]
fn bt2_047_inherited_played_digimons_on_play_activates() {
    let mut r = runner();
    r.game.players[0].deck.clear();
    r.add_to_hand(0, "ST4-03");
    // Top of deck = a green Digimon for ST4-03 to add.
    let g = r.add_to_hand(0, "G-LV3-B");
    let card = r.game.players[0].hand.remove(g);
    r.game.players[0].deck.push(card);
    attack_with_argomon_source(&mut r);
    let a = hand_pick(&r, "ST4-03");
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    let st4 = find_on_field(&r, "ST4-03").expect("played");
    assert!(suspended(&r, st4));
    assert!(in_hand(&r, "G-LV3-B"), "ST4-03's [On Play] added the revealed card");
}

/// Entering play suspended is not "becoming suspended": BT10-053's inherited
/// "[Your Turn][OPT] When an effect suspends one of your Digimon, gain 1
/// memory" does not trigger.
#[test]
fn bt2_047_inherited_entering_suspended_is_not_a_suspend_event() {
    let mut r = runner();
    r.place_stack(0, &["BT10-053", "G-LV4"]);
    r.add_to_hand(0, "G-LV3");
    attack_with_argomon_source(&mut r);
    // Resolve Argomon's trigger first if a trigger-order prompt appears.
    let v = r.pending_selection_view().expect("[When Attacking]");
    assert!(v.is_optional);
    let a = hand_pick(&r, "G-LV3");
    let before = r.memory();
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert!(find_on_field(&r, "G-LV3").is_some());
    assert_eq!(r.memory(), before, "no suspend event → no +1 memory");
}
