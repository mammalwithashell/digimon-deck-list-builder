//! `G-ENGINE-PLAY-MASK-IGNORES-WHEN-PLAYING-REDUCTION` follow-up (3):
//! `BeforePayCostObserve` "gain N memory" effects run automatically (no
//! choice) after the cost is fixed and BEFORE it is paid, so declare-then-pay
//! affordability (rule 1-3-11-1) must add the gain to the gauge — capped at
//! the gauge maximum, exactly as `gain_memory` caps it.
//!
//! The pool's only gain-memory observers (BT12-022 / BT12-050) fire on DNA
//! digivolution, whose mask has no memory gate; this synthetic Tamer pins the
//! play-path contract the mask now shares with the payment path.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use std::sync::Arc;

fn plain_digimon(card_id: &str, play_cost: u16) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.play_cost = play_cost;
    card
}

fn tamer(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

/// "When you would play a card, before paying the cost, gain 2 memory."
struct GainTwoBeforePaying;
impl CardEffect for GainTwoBeforePaying {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost_observe(card)
            .name("gain 2 memory before paying")
            .before_pay_cost_gain_memory(2)
            .build()]
    }
}

fn hand_play_offered(r: &DebugRunner) -> bool {
    let mask = digimon_engine::action::mask::build_action_mask(&r.game, 0);
    mask[digimon_engine::action::space::PLAY_HAND_START as usize] == 1.0
}

fn runner(play_cost: u16) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TARGET", play_cost))
        .add_card(tamer("OBSERVER"))
        .hand(0, &["TARGET"])
        .start();
    r.register_effect("OBSERVER", Arc::new(GainTwoBeforePaying));
    r
}

#[test]
fn observer_memory_gain_makes_an_otherwise_unaffordable_play_declarable() {
    let mut r = runner(5);
    // 5 from -7 → -12 (illegal) without the observer.
    r.game.set_memory(-7);
    assert!(!hand_play_offered(&r), "no observer → unaffordable");

    r.place_on_field(0, "OBSERVER", Some(0));
    assert!(
        hand_play_offered(&r),
        "-7 + 2 = -5, then paying 5 lands exactly on the -10 floor"
    );
    let events_before = r.game.events.len();
    r.play(0, 0);
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "TARGET"),
        "the play resolved: the payment path gains before paying, like the mask"
    );
    let net: i16 = r.game.events[events_before..]
        .iter()
        .filter_map(|e| match e {
            digimon_engine::events::GameEvent::MemoryChange { delta, .. } => Some(*delta),
            _ => None,
        })
        .sum();
    assert_eq!(net, -3, "+2 gained, 5 paid");

    // From -8 the gain is not enough (-6 - 5 = -11).
    let mut r = runner(5);
    r.place_on_field(0, "OBSERVER", Some(0));
    r.game.set_memory(-8);
    assert!(!hand_play_offered(&r));
}

#[test]
fn observer_memory_gain_is_capped_at_the_gauge_maximum() {
    // 21 from 10 → -11. The +2 cannot lift the gauge past 10, so the play
    // stays unaffordable.
    let mut r = runner(21);
    r.place_on_field(0, "OBSERVER", Some(0));
    r.game.set_memory(10);
    assert!(!hand_play_offered(&r), "gain capped at the gauge maximum");
    // 20 from 10 → -10: affordable without any gain.
    let mut r = runner(20);
    r.place_on_field(0, "OBSERVER", Some(0));
    r.game.set_memory(10);
    assert!(hand_play_offered(&r));
}
