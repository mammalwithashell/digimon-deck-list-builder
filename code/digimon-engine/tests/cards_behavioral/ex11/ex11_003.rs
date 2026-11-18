//! EX11-003 Puroromon — Digi-Egg, Lv.2, Green, Cost 0 (Larva, X Antibody, Royal
//! Base, LIBERATOR).
//!
//! Printed (official Bandai DB bundle + DCGO EX11_003.cs):
//! Inherited Effect [Your Turn] [Once Per Turn] When face-up [Royal Base] trait
//! cards are placed in your security stack, <Draw 1>.
//!
//! Official Q&A Q5788: flipping a face-down [Royal Base] security card face up
//! does NOT trigger it (only a placement does).
//!
//! DCGO QUIRK: EX11_003.cs builds a `SecurityCondition` filter (face-up AND
//! [Royal Base]) but never reads it, so DCGO draws on ANY card added to the
//! security stack. The printed text and Q5788 govern: this suite pins the
//! faithful behaviour (face-up + [Royal Base] only).
//!
//! The card had only a JSON stub before (hunt cluster
//! alt-digivolve-xros-req-cards). New substrate:
//! `event_card_face_up_in_security` (G-PRED-EVENT-CARD-FACE-UP-IN-SECURITY).
//! Synthetic boards only.

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, StackPosition};

const ID: &str = "EX11-003";

fn digimon(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 0;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// The CAUSE: an [On Play] effect that places the first card of its
/// controller's hand on the bottom of their security stack.
struct PlaceHandOnSecurity {
    face_up: bool,
}
impl CardEffect for PlaceHandOnSecurity {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let face_up = self.face_up;
        vec![Effect::on_play(card)
            .name("test placer: place a hand card in security")
            .process(move |ctx| {
                let p = ctx.player;
                assert!(ctx.place_on_security(p, CardSourceRef::Hand(p, 0), StackPosition::Bottom, face_up));
            })
            .build()]
    }
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX11-003 production YAML loads")
        .add_card(digimon("HOST", &["Beast"]))
        .add_card(digimon("PLACER", &["Beast"]))
        .add_card(digimon("RB", &["Royal Base"]))
        .add_card(digimon("PLAIN", &["Beast"]))
        .add_card(digimon("F", &["Beast"]))
        .deck(0, &["F"; 12])
        .deck(1, &["F"; 12])
        .memory(5)
}

/// Puroromon under a host on player 0's field, `hand` in player 0's hand.
fn setup(hand: &[&str], your_turn: bool) -> DebugRunner {
    let mut r = builder().hand(0, hand).start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.turn_player_idx = if your_turn { 0 } else { 1 };
    r.place_stack(0, &[ID, "HOST"]);
    r
}

fn place(r: &mut DebugRunner, face_up: bool) {
    r.register_effect("PLACER", Arc::new(PlaceHandOnSecurity { face_up }));
    let placer = r.place_on_field(0, "PLACER", Some(0));
    r.game.fire_on_play(0, placer.index as usize);
}

fn hand_after_placing(hand: &[&str], face_up: bool, your_turn: bool) -> usize {
    let mut r = setup(hand, your_turn);
    place(&mut r, face_up);
    r.hand_size(0)
}

#[test]
fn ex11_003_draws_1_when_a_face_up_royal_base_card_is_placed_in_your_security() {
    // 1 hand card is placed (-1), then <Draw 1> (+1).
    assert_eq!(hand_after_placing(&["RB", "PLAIN"], true, true), 2);
}

#[test]
fn ex11_003_a_face_down_placement_does_not_trigger_it() {
    // Printed text + Q5788: only FACE-UP [Royal Base] cards (DCGO's unused
    // filter would draw here — see the module header).
    assert_eq!(hand_after_placing(&["RB", "PLAIN"], false, true), 1);
}

#[test]
fn ex11_003_a_face_up_card_without_the_royal_base_trait_does_not_trigger_it() {
    assert_eq!(hand_after_placing(&["PLAIN", "RB"], true, true), 1);
}

#[test]
fn ex11_003_is_your_turn_only() {
    assert_eq!(hand_after_placing(&["RB", "PLAIN"], true, false), 1);
}

#[test]
fn ex11_003_is_once_per_turn() {
    let mut r = setup(&["RB", "RB", "PLAIN"], true);
    place(&mut r, true);
    assert_eq!(r.hand_size(0), 3, "first placement: -1 +1");
    place(&mut r, true);
    assert_eq!(r.hand_size(0), 2, "second placement: -1 and no further draw");
}
