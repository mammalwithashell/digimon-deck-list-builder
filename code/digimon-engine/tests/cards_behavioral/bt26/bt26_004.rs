//! BT26-004 Pagumon — Digi-Egg, Lv.2, Purple.
//! Traits: Lesser, Glowing Dawn, BEATBREAK. Form: In-Training.
//!
//! # Printed text (official Bandai DB — data/card_bundles/BT26-004.md)
//!
//! ```text
//! Inherited: [When Attacking] [Once Per Turn] By placing 1 card from your
//! hand face down under any of your [Glowing Dawn] trait Tamers, <Draw 1>.
//! ```
//!
//! Official Q&A: Q6954 the card goes to the BOTTOM of the cards under the
//! Tamer; Q6955 the stacking order can't be changed; Q6956 only its owner may
//! look at a face-down card; Q6957 a trashed face-down card goes to the trash
//! face up.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT26/Purple/BT26_004.cs
//!   - OnAllyAttack, inherited, maxCount 1, skippable; CanActivate = an own
//!     [Glowing Dawn] Tamer exists. Hand pick (any card, canNoSelect:true) →
//!     Tamer pick over own [Glowing Dawn] Tamers (canNoSelect:true) →
//!     AddDigivolutionCardsBottom(face down) → Draw 1. Not completing the
//!     placement refunds the [Once Per Turn] (RemoveUse).
//!
//! # Patterns
//! - G4 inherited [When Attacking] on a Digi-Egg (OPT + "by" cost + selections)
//! - E2 OPT + optional decline

use super::support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-004";

fn setup() -> DebugRunner {
    // A 9000 DP carrier survives the security battle of its own attack.
    let mut carrier_card = digimon("CARRIER", "Carrier", CardColor::Purple, 3, 3, &[]);
    carrier_card.dp = Some(9000);
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-004 in the embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(carrier_card)
        .add_card(digimon("HANDDIGI", "Hand Digimon", CardColor::Red, 4, 5, &[]))
        .add_card(tamer("HANDTAMER", "Hand Tamer", CardColor::Red, &[]))
        .add_card(gd_option("HANDOPT", 2))
        .add_card(digimon("DECKTOP", "Deck Top", CardColor::Red, 3, 3, &[]))
        .add_card(tamer("GDT", "GD Tamer", CardColor::Purple, &["Glowing Dawn"]))
        .add_card(tamer("GDT2", "GD Tamer 2", CardColor::Purple, &["Glowing Dawn"]))
        .add_card(tamer("PT", "Plain Tamer", CardColor::Red, &["BEATBREAK"]))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .security(0, &["FILLER"; 4])
        .security(1, &["FILLER"; 4])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

/// Pagumon as the inherited source of a Lv.3 carrier.
fn carrier(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "CARRIER"])
}

fn hand_action(r: &DebugRunner, card_id: &str) -> u16 {
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"));
    PLAY_HAND_START + idx as u16
}

fn tamer_action(t: PermanentHandle) -> u16 {
    encode_attack(0, t.index as u16)
}

// ─── Section 1: structural ──────────────────────────────────────────────────

#[test]
fn bt26_004_structure() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.level, Some(2));
    let trig: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 1);
    let t = trig[0];
    assert_eq!(t.when, vec![CompiledTiming::WhenAttacking]);
    assert!(t.once_per_turn, "[Once Per Turn]");
    assert!(t.optional, "the 'by placing' cost is the player's choice");
    assert!(format!("{t:?}").contains("Inherited"), "inherited effect");
}

// ─── Section 3: behaviour ───────────────────────────────────────────────────

#[test]
fn bt26_004_attack_places_a_hand_card_face_down_at_the_bottom_and_draws() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    let m = carrier(&mut r);
    push_hand(&mut r, 0, "HANDDIGI");
    stack_deck_top(&mut r, 0, &["DECKTOP"]);
    let deck = r.deck_size(0);
    r.attack_player(m, 1, false);
    assert!(r.pending_is_optional(), "may decline the 'by placing' cost");
    r.accept_optional_trigger().unwrap();
    let a = hand_action(&r, "HANDDIGI");
    r.execute_action(0, a).unwrap(); // the card to place
    r.execute_action(0, tamer_action(t)).unwrap(); // the Tamer
    let _ = r.auto_resolve();
    assert_eq!(
        source_ids(&r, t),
        vec!["HANDDIGI".to_string(), "FILLER".to_string()],
        "Q6954: placed at the BOTTOM of the cards under the Tamer"
    );
    assert_eq!(face_down_flags(&r, t), vec![true, true], "face down");
    assert_eq!(hand_ids(&r, 0), vec!["DECKTOP".to_string()], "<Draw 1>");
    assert_eq!(r.deck_size(0), deck - 1);
}

#[test]
fn bt26_004_any_card_in_hand_can_be_placed() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "GDT", 0);
    let m = carrier(&mut r);
    for id in ["HANDDIGI", "HANDTAMER", "HANDOPT"] {
        push_hand(&mut r, 0, id);
    }
    r.attack_player(m, 1, false);
    r.accept_optional_trigger().unwrap();
    let v = r.pending_selection_view().expect("hand pick");
    for id in ["HANDDIGI", "HANDTAMER", "HANDOPT"] {
        assert!(
            v.valid_action_ids.contains(&hand_action(&r, id)),
            "{id} — '1 card from your hand' has no card-type limit"
        );
    }
}

#[test]
fn bt26_004_offers_only_your_glowing_dawn_tamers() {
    let mut r = setup();
    let gd1 = tamer_with_face_down(&mut r, 0, "GDT", 0);
    let gd2 = tamer_with_face_down(&mut r, 0, "GDT2", 0);
    let plain = tamer_with_face_down(&mut r, 0, "PT", 0);
    let opp_gd = tamer_with_face_down(&mut r, 1, "GDT", 0);
    let m = carrier(&mut r);
    push_hand(&mut r, 0, "HANDDIGI");
    r.attack_player(m, 1, false);
    r.accept_optional_trigger().unwrap();
    let a = hand_action(&r, "HANDDIGI");
    r.execute_action(0, a).unwrap();
    let v = r.pending_selection_view().expect("Tamer pick");
    assert!(v.valid_action_ids.contains(&tamer_action(gd1)));
    assert!(v.valid_action_ids.contains(&tamer_action(gd2)));
    assert!(
        !v.valid_action_ids.contains(&tamer_action(plain)),
        "a Tamer without [Glowing Dawn] is not offered"
    );
    r.execute_action(0, tamer_action(gd2)).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, gd2), vec!["HANDDIGI".to_string()]);
    assert_eq!(sources(&r, gd1), 0);
    assert_eq!(sources(&r, opp_gd), 0, "the opponent's Tamer is never used");
}

#[test]
fn bt26_004_declined_does_nothing_and_keeps_the_once_per_turn() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 0);
    let m = carrier(&mut r);
    push_hand(&mut r, 0, "HANDDIGI");
    let deck = r.deck_size(0);
    r.attack_player(m, 1, false);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0);
    assert_eq!(hand_ids(&r, 0), vec!["HANDDIGI".to_string()]);
    assert_eq!(r.deck_size(0), deck);
    fire(&mut r, EffectTiming::WhenAttacking, m);
    assert!(r.pending_is_optional(), "declining did not spend the [Once Per Turn]");
}

#[test]
fn bt26_004_needs_a_glowing_dawn_tamer() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "PT", 0);
    tamer_with_face_down(&mut r, 1, "GDT", 0); // the opponent's doesn't count
    let m = carrier(&mut r);
    push_hand(&mut r, 0, "HANDDIGI");
    let deck = r.deck_size(0);
    fire(&mut r, EffectTiming::WhenAttacking, m);
    assert!(r.game.pending_selection.is_none(), "no [Glowing Dawn] Tamer → not offered");
    assert_eq!(r.deck_size(0), deck);
}

#[test]
fn bt26_004_needs_a_card_in_hand() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "GDT", 0);
    let m = carrier(&mut r);
    assert_eq!(r.hand_size(0), 0);
    let deck = r.deck_size(0);
    fire(&mut r, EffectTiming::WhenAttacking, m);
    assert!(r.game.pending_selection.is_none(), "the cost can't be paid → not offered");
    assert_eq!(r.deck_size(0), deck, "no draw");
}

#[test]
fn bt26_004_is_once_per_turn_and_resets() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 0);
    let m = carrier(&mut r);
    push_hand(&mut r, 0, "HANDDIGI");
    push_hand(&mut r, 0, "HANDTAMER");
    fire(&mut r, EffectTiming::WhenAttacking, m);
    r.accept_optional_trigger().unwrap();
    let a = hand_action(&r, "HANDDIGI");
    r.execute_action(0, a).unwrap();
    r.execute_action(0, tamer_action(t)).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 1);
    fire(&mut r, EffectTiming::WhenAttacking, m);
    assert!(r.game.pending_selection.is_none(), "[Once Per Turn] spent");
    r.pass_turn();
    r.pass_turn();
    fire(&mut r, EffectTiming::WhenAttacking, m);
    assert!(r.pending_is_optional(), "lockout clears on a new turn");
}

#[test]
fn bt26_004_face_up_pagumon_has_no_when_attacking() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "GDT", 0);
    let m = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "HANDDIGI");
    fire(&mut r, EffectTiming::WhenAttacking, m);
    assert!(r.game.pending_selection.is_none(), "inherited only");
}
