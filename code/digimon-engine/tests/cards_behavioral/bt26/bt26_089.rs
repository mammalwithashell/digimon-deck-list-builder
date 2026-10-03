//! BT26-089 Kyo Sawashiro — Tamer, Yellow, Cost 3. Traits: Glowing Dawn, BEATBREAK.
//!
//! [Start of Your Main Phase] By placing 1 [BEATBREAK] trait card from your hand
//! face down under this Tamer, <Draw 1> and gain 1 memory.
//! [All Turns] When your security stack is removed from, by suspending this
//! Tamer, place the top card of your deck face down under this Tamer. After, if
//! removed from by effects, give 1 of your opponent's Digimon <Security A. -1>
//! until their turn ends.
//! [Security] Play this card without paying the cost.
//! Official Q&A: the card is placed on the bottom of the cards under the Tamer.
//!
//! DCGO: BT26/Yellow/BT26_089.cs. Effect-caused removal is driven by the
//! test-only `SEC_TRASHER_YAML` ([On Play] trash your top security card);
//! the non-effect removal is a real security check from an opponent attack.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, ModifierType};

const CARD_ID: &str = "BT26-089";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-089")
        .from_dsl_yaml(SEC_TRASHER_YAML)
        .expect("trasher")
        .add_card(filler("FILLER"))
        .add_card(tamer("SECT", "SecTamer", CardColor::Red, &[]))
        .add_card(digimon(
            "BB",
            "Beatbreak",
            CardColor::Yellow,
            3,
            3,
            &["BEATBREAK"],
        ))
        .add_card(digimon("PLAIN", "Plain", CardColor::Yellow, 3, 3, &[]))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("DECKTOP", "DeckTop", CardColor::Red, 3, 3, &[]))
        .deck(0, &["FILLER", "FILLER", "DECKTOP"])
        .deck(1, &["FILLER"; 6])
        .security(0, &["SECT", "SECT", "SECT"])
        .security(1, &["SECT", "SECT", "SECT"])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn sa_change(r: &DebugRunner, h: digimon_engine::permanent::PermanentHandle) -> i32 {
    r.modifiers().sum(h, ModifierType::SecurityAttackChange)
}

// ─── [Start of Your Main Phase] ─────────────────────────────────────────────

#[test]
fn bt26_089_somp_places_beatbreak_card_draws_and_gains_memory() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BB");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, k);
    let v = r.pending_selection_view().expect("optional hand pick");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "BB");
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, k), vec!["BB".to_string()]);
    assert_eq!(face_down_flags(&r, k), vec![true]);
    assert_eq!(hand_ids(&r, 0), vec!["DECKTOP".to_string()], "<Draw 1>");
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn bt26_089_somp_declined_does_nothing() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BB");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, k);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, k), 0);
    assert_eq!(hand_ids(&r, 0), vec!["BB".to_string()]);
    assert_eq!(r.memory(), mem);
}

#[test]
fn bt26_089_somp_needs_a_beatbreak_card_in_hand() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "PLAIN");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, k);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(sources(&r, k), 0);
    assert_eq!(r.hand_size(0), 1);
}

// ─── [All Turns] When your security stack is removed from ───────────────────

#[test]
fn bt26_089_effect_removal_places_deck_top_and_gives_security_minus_one() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    let t = r.place_on_field(0, "T-SECTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    assert_eq!(r.security_count(0), 2, "trasher removed 1 security");
    r.accept_optional_trigger()
        .expect("Kyo triggers on own security removal");
    assert!(
        r.game.players[0].battle_area[k.index as usize].is_suspended,
        "suspend cost paid"
    );
    // removed BY AN EFFECT ⇒ mandatory pick of 1 opponent Digimon.
    let v = r
        .pending_selection_view()
        .expect("Security A. -1 target pick");
    assert!(!v.is_optional);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, k), vec!["DECKTOP".to_string()]);
    assert_eq!(face_down_flags(&r, k), vec![true]);
    assert_eq!(sa_change(&r, opp), -1, "<Security A. -1>");
}

#[test]
fn bt26_089_battle_removal_places_deck_top_without_security_debuff() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    r.pass_turn(); // → P1's turn
    let atk = r.place_on_field(1, "OPP", Some(0));
    r.attack_player(atk, 0, false);
    r.accept_optional_trigger()
        .expect("Kyo triggers on the security-check removal");
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[k.index as usize].is_suspended);
    assert_eq!(source_ids(&r, k), vec!["DECKTOP".to_string()]);
    assert_eq!(face_down_flags(&r, k), vec![true]);
    assert_eq!(
        sa_change(&r, atk),
        0,
        "not removed by an effect ⇒ no <Security A. -1>"
    );
}

#[test]
fn bt26_089_trigger_is_declinable() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    let t = r.place_on_field(0, "T-SECTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[k.index as usize].is_suspended);
    assert_eq!(sources(&r, k), 0);
    assert_eq!(sa_change(&r, opp), 0);
}

#[test]
fn bt26_089_suspended_kyo_cannot_pay_the_cost() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[k.index as usize].is_suspended = true;
    let t = r.place_on_field(0, "T-SECTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(sources(&r, k), 0);
}

#[test]
fn bt26_089_opponent_security_removal_does_not_trigger() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let t = r.place_on_field(1, "T-SECTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 2);
    assert!(!r.game.players[0].battle_area[k.index as usize].is_suspended);
    assert_eq!(sources(&r, k), 0);
}

#[test]
fn bt26_089_security_effect_plays_itself() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    let dbg = format!("{:?}", card.effects);
    assert!(dbg.contains("OnSecurity") && dbg.contains("PlayFromSecurity"));
}
