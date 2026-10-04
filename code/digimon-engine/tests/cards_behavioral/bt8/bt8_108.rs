//! BT8-108 Mist Memory Boost! — Option, Purple, Cost 3.
//!
//! Printed text (official Bandai DB, `data/card_bundles/BT8-108.md`):
//!   [Main] Trash the top 2 cards of your deck and <Draw 1>. Then, place this
//!     card in the battle area.
//!   [Main] <Delay> ・Gain 2 memory.
//!   [Security] Place this card in the battle area.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT8/Purple/BT8_108.cs
//!   - OptionSkill: IAddTrashCardsFromLibraryTop(2) → DrawClass(1) →
//!     PlaceDelayOptionCards(card).
//!   - OnDeclaration (<Delay>): trash self, on success Owner.AddMemory(2);
//!     not declarable the turn it entered play.
//!   - SecuritySkill: CardEffectFactory.PlaceSelfDelayOptionSecurityEffect.
//!
//! Patterns: Memory Boost delay option (mirrors LM-033 shape); trash_from_top
//! + draw; standard <Delay> declarative clause; security place-self.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

const CARD_ID: &str = "BT8-108";

fn purple_tamer() -> CardData {
    let mut c = make_test_card("PURPLE-TAMER", "Purple Tamer");
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Purple];
    c
}

fn attacker() -> CardData {
    let mut c = make_test_card("ATTACKER", "Attacker");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Red];
    c
}

fn named(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Deck (bottom → top): BOTTOM, DRAWN, MILL-2, MILL-1.
fn main_runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT8-108 DSL card")
        .add_card(purple_tamer())
        .add_card(named("BOTTOM"))
        .add_card(named("DRAWN"))
        .add_card(named("MILL-2"))
        .add_card(named("MILL-1"))
        .add_card(named("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["BOTTOM", "DRAWN", "MILL-2", "MILL-1"])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.place_on_field(0, "PURPLE-TAMER", Some(0));
    r.game.enter_main_phase();
    r
}

fn delayed_index(r: &DebugRunner, player: u8) -> Option<usize> {
    r.game.player(player).battle_area.iter().position(|p| {
        p.top_card().card_id(&r.game.card_data) == CARD_ID
            && matches!(
                p.option_state,
                OptionState::Delayed {
                    trigger: DelayTrigger::MainPhaseActivated,
                    ..
                }
            )
    })
}

// ── Section 1: structure ────────────────────────────────────────────────────

#[test]
fn bt8_108_structure() {
    let r = main_runner();
    let card = r.compiled_card(CARD_ID).expect("compiles");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.effects.len(), 3, "Main, Delay, Security");

    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("[Main] clause");
    assert!(!main.optional, "[Main] is mandatory");

    let delay = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                process,
                ..
            }) => Some((trigger, process)),
            _ => None,
        })
        .expect("standard <Delay> clause");
    assert_eq!(*delay.0, CompiledTiming::Delayed);
    assert!(delay
        .1
        .iter()
        .any(|s| matches!(s, CompiledStep::GainMemory(2))));

    let sec = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("[Security] clause");
    assert_eq!(sec.process, vec![CompiledStep::PlaceSelfAsDelayOption]);
}

// ── Section 3: [Main] behaviour ────────────────────────────────────────────

#[test]
fn bt8_108_main_trashes_top_two_then_draws_one() {
    let mut r = main_runner();
    let hand_before = r.hand_size(0); // includes BT8-108
    assert_ne!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid
    );
    let _ = r.auto_resolve();

    let trash_ids: Vec<String> = r
        .game
        .player(0)
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert!(trash_ids.contains(&"MILL-1".to_string()));
    assert!(trash_ids.contains(&"MILL-2".to_string()));
    assert_eq!(trash_ids.len(), 2, "only the 2 milled cards go to trash");
    assert!(
        r.game
            .player(0)
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "DRAWN"),
        "draw happens after the mill, drawing the 3rd card"
    );
    assert_eq!(r.hand_size(0), hand_before, "-1 option +1 draw");
    assert_eq!(r.deck_size(0), 1);
    assert_eq!(r.memory(), 7, "cost 3 paid");
}

#[test]
fn bt8_108_main_places_self_as_delay_option() {
    let mut r = main_runner();
    r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert!(
        delayed_index(&r, 0).is_some(),
        "BT8-108 must be placed in the battle area as a <Delay> option"
    );
    assert!(
        !r.game
            .player(0)
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == CARD_ID),
        "BT8-108 is not trashed after [Main]"
    );
}

#[test]
fn bt8_108_main_with_short_deck_still_places_self() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT8-108 DSL card")
        .add_card(purple_tamer())
        .add_card(named("MILL-1"))
        .add_card(named("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["MILL-1"])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.place_on_field(0, "PURPLE-TAMER", Some(0));
    r.game.enter_main_phase();
    r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), 0);
    assert!(delayed_index(&r, 0).is_some());
}

// ── Section 3/5: <Delay> ───────────────────────────────────────────────────

#[test]
fn bt8_108_delay_not_activatable_on_placing_turn() {
    let mut r = main_runner();
    r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    let idx = delayed_index(&r, 0).expect("placed");
    let bit = (FIELD_EFFECT_START + idx as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN)
        as usize;
    assert_eq!(build_action_mask(&r.game, 0)[bit], 0.0);
}

#[test]
fn bt8_108_delay_activation_next_turn_gains_2_memory_and_trashes_self() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT8-108 DSL card")
        .add_card(purple_tamer())
        .add_card(named("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    r.place_on_field(0, "PURPLE-TAMER", Some(0));
    r.game.enter_main_phase();
    r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();

    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
    r.game.set_memory(0);

    let idx = delayed_index(&r, 0).expect("still parked");
    let bit = (FIELD_EFFECT_START + idx as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN)
        as usize;
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[bit], 1.0, "<Delay> is a legal action on a later turn");
    assert_eq!(mask[PASS as usize], 1.0, "declining stays legal");

    r.game.decode_action(bit as u16, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 2, "<Delay> gains 2 memory");
    assert!(
        delayed_index(&r, 0).is_none(),
        "trashed as the <Delay> cost"
    );
    assert!(r
        .game
        .player(0)
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == CARD_ID));
}

// ── Section 3: [Security] ──────────────────────────────────────────────────

#[test]
fn bt8_108_security_places_self_in_battle_area() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT8-108 DSL card")
        .add_card(attacker())
        .add_card(named("FILL"))
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"; 3])
        .deck(1, &["FILL"; 3])
        .memory(3)
        .start();
    let atk = r.place_on_field(0, "ATTACKER", Some(0));
    let res = r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(res, AttackResult::SecurityCheckSurvived);
    assert_eq!(r.security_count(1), 0);
    assert_eq!(r.trash_size(1), 0, "placed, not trashed");
    assert!(delayed_index(&r, 1).is_some(), "parked as a <Delay> option");
}
