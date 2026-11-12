//! BT13-033 MirageGaogamon: Burst Mode — Digimon, Lv.7, Blue, DP 15000,
//! Cost 15, Beast Knight / Mega / Data.
//!
//! Printed (official DB bundle + card image):
//! `[When Digivolving] Return 1 of your opponent's Digimon to the hand. Then,`
//! `gain 1 memory for every 4 cards in your opponent's hand.`
//! `[When Attacking] If your opponent has 9 or more cards in their hand, by`
//! `choosing cards in your opponent's hand without looking and returning them`
//! `to the bottom of the deck so that 8 remain, unsuspend this Digimon.`
//! `[Burst Digivolve] 0 from [MirageGaogamon] by returning 1 [Thomas H.`
//! `Norstein] to hand; at the end of the burst turn trash the top card.`
//!
//! DCGO BT13_033.cs: the WD bounce is mandatory (canNoSelect false) and the
//! memory is `Enemy.HandCards.Count / 4` counted after the bounce; the WA is
//! optional, gated on >= 9 enemy hand cards, shuffles the enemy hand, and the
//! owner picks exactly hand-8 face-down cards (canEndNotMax false) for the
//! deck bottom, then unsuspends only if a card was returned.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD: &str = "BT13-033";
const BLIND_PROMPT: &str = "Choose cards in your opponent's hand without looking to return to the bottom of their deck so that 8 remain";

fn runner() -> DebugRunner {
    let mut digi = make_test_card("OPP-DIGI", "OppDigi");
    digi.card_kind = CardKind::Digimon;
    digi.dp = Some(1000);
    digi.level = Some(3);
    let mut other = make_test_card("OPP-DIGI-B", "OppDigiB");
    other.card_kind = CardKind::Digimon;
    other.dp = Some(2000);
    other.level = Some(4);
    let mut sec = make_test_card("SEC-FILLER", "SecurityFiller");
    sec.card_kind = CardKind::Tamer;
    sec.dp = None;
    sec.level = None;
    let mut r = DebugRunner::builder()
        .dsl_card(CARD)
        .expect("BT13-033 loads from the embedded DSL pack")
        .add_card(digi)
        .add_card(other)
        .add_card(sec)
        .add_card(make_test_card("OPP-HAND", "OppHandCard"))
        .add_card(make_test_card("OPP-DECK", "OppDeckCard"))
        .deck(0, &["OPP-DECK"; 5])
        .deck(1, &["OPP-DECK"; 5])
        .security(0, &["SEC-FILLER"; 5])
        .security(1, &["SEC-FILLER"; 5])
        .memory(3)
        .build();
    r.game.turn_count = 1;
    r
}

fn push_to_hand(r: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be registered"));
    let idx = r.game.next_card_index();
    r.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, idx));
}

fn give_opp_hand(r: &mut DebugRunner, n: usize) {
    for _ in 0..n {
        push_to_hand(r, 1, "OPP-HAND");
    }
}

fn ids(r: &DebugRunner, cards: &[CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| r.game.card_data[c.data_index].card_id.clone())
        .collect()
}

fn fire_when_digivolving(r: &mut DebugRunner) -> PermanentHandle {
    let carrier = r.place_on_field(0, CARD, None);
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(carrier));
    r.game.drain_effect_queue();
    carrier
}

// ─── [When Digivolving] ─────────────────────────────────────────────────────

/// The bounce is mandatory: a real OppField pick for player 0 with no PASS,
/// and only the chosen Digimon returns to its owner's hand.
#[test]
fn bt13_033_when_digivolving_bounce_is_a_mandatory_player_choice() {
    let mut r = runner();
    r.place_on_field(1, "OPP-DIGI", None);
    let b = r.place_on_field(1, "OPP-DIGI-B", None);
    fire_when_digivolving(&mut r);
    let v = r.pending_selection_view().expect("bounce target prompt");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert_eq!(v.selecting_player, 0);
    assert!(!v.valid_action_ids.contains(&PASS), "mandatory: no decline");
    assert_eq!(v.valid_action_ids.len(), 2, "both opponent Digimon are legal");
    // Choose the SECOND Digimon (OPP-DIGI-B).
    let pick = *v.valid_action_ids.iter().max().unwrap();
    r.execute_action(0, pick).unwrap();
    r.auto_resolve().unwrap();
    let field: Vec<String> = r.game.players[1]
        .battle_area
        .iter()
        .map(|p| r.game.card_data[p.top_card().data_index].card_id.clone())
        .collect();
    assert_eq!(field, vec!["OPP-DIGI".to_string()], "only the chosen Digimon left");
    assert!(ids(&r, &r.game.players[1].hand).contains(&"OPP-DIGI-B".to_string()));
}

/// Memory counts the opponent's hand AFTER the bounce: 3 + 1 bounced = 4 -> +1.
#[test]
fn bt13_033_when_digivolving_memory_counts_hand_after_bounce() {
    let mut r = runner();
    give_opp_hand(&mut r, 3);
    r.place_on_field(1, "OPP-DIGI", None);
    let before = r.memory();
    fire_when_digivolving(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].hand.len(), 4);
    assert_eq!(r.memory(), before + 1, "floor(4 / 4) = 1");
}

/// 7 + 1 bounced = 8 -> +2.
#[test]
fn bt13_033_when_digivolving_memory_eight_cards_gains_two() {
    let mut r = runner();
    give_opp_hand(&mut r, 7);
    r.place_on_field(1, "OPP-DIGI", None);
    let before = r.memory();
    fire_when_digivolving(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].hand.len(), 8);
    assert_eq!(r.memory(), before + 2, "floor(8 / 4) = 2");
}

/// 6 + 1 bounced = 7 -> floor(7/4) = 1 (rounds down).
#[test]
fn bt13_033_when_digivolving_memory_rounds_down() {
    let mut r = runner();
    give_opp_hand(&mut r, 6);
    r.place_on_field(1, "OPP-DIGI", None);
    let before = r.memory();
    fire_when_digivolving(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), before + 1, "floor(7 / 4) = 1");
}

/// No opponent Digimon: no prompt, the memory clause still resolves
/// (8 cards in hand -> +2).
#[test]
fn bt13_033_when_digivolving_no_target_still_gains_memory() {
    let mut r = runner();
    give_opp_hand(&mut r, 8);
    let before = r.memory();
    fire_when_digivolving(&mut r);
    assert!(r.pending_selection_view().is_none(), "nothing to bounce, no prompt");
    assert_eq!(r.memory(), before + 2);
}

/// Empty opponent hand and field: nothing happens (0 memory).
#[test]
fn bt13_033_when_digivolving_empty_opponent_gains_nothing() {
    let mut r = runner();
    let before = r.memory();
    fire_when_digivolving(&mut r);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.memory(), before, "floor(0 / 4) = 0");
}

// ─── [When Attacking] ───────────────────────────────────────────────────────

/// Attack a suspended 1000-DP opponent Digimon with BT13-033 while the
/// opponent holds `hand` cards. Returns (runner, attacker).
fn attack_with_opp_hand(hand: usize) -> (DebugRunner, PermanentHandle) {
    let mut r = runner();
    give_opp_hand(&mut r, hand);
    let attacker = r.place_on_field(0, CARD, Some(0));
    let target = r.place_on_field(1, "OPP-DIGI", None);
    r.game.players[1].battle_area[target.index as usize].is_suspended = true;
    r.attack_digimon(attacker, target, false);
    (r, attacker)
}

fn attacker_suspended(r: &DebugRunner) -> bool {
    let slot = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| r.game.card_data[p.top_card().data_index].card_id == CARD)
        .expect("BT13-033 still on the field");
    r.game.players[0].battle_area[slot].is_suspended
}

/// Accept the optional [When Attacking] activation prompt.
fn accept_activation(r: &mut DebugRunner) {
    assert!(r.pending_is_optional(), "the [When Attacking] effect is optional");
    let v = r.pending_selection_view().unwrap();
    assert_ne!(v.prompt, BLIND_PROMPT, "an activation prompt precedes the pick");
    r.accept_optional_trigger().unwrap();
}

/// Drive the blind pick: exactly `n` picks, each with no early finish.
fn blind_pick(r: &mut DebugRunner, n: usize) {
    for i in 0..n {
        let v = r.pending_selection_view().expect("blind hand pick pending");
        assert!(matches!(v.kind, SelectionKind::CountCappedMultiSelect { .. }), "{:?}", v.kind);
        if i == 0 {
            assert_eq!(v.prompt, BLIND_PROMPT);
        }
        assert_eq!(v.selecting_player, 0, "the attacker's controller chooses");
        assert_eq!(v.zone_owner, Some(1), "from the opponent's hand");
        assert!(
            !v.is_optional && !v.valid_action_ids.contains(&PASS),
            "pick {i} of {n}: cannot stop before exactly hand-8 cards are chosen"
        );
        let a = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("a hand card to pick");
        r.execute_action(0, a).unwrap();
    }
    if let Some(v) = r.pending_selection_view() {
        if matches!(v.kind, SelectionKind::CountCappedMultiSelect { .. }) {
            assert!(
                v.valid_action_ids.iter().all(|&a| a == PASS),
                "no extra pick beyond hand-8: {:?}",
                v.valid_action_ids
            );
            assert!(v.is_optional, "finishing is legal once hand-8 are chosen");
            r.execute_action(0, PASS).unwrap();
        }
    }
}

#[test]
fn bt13_033_when_attacking_nine_cards_returns_one_and_unsuspends() {
    let (mut r, _) = attack_with_opp_hand(9);
    let deck_before = r.game.players[1].deck.len();
    accept_activation(&mut r);
    blind_pick(&mut r, 1);
    assert_eq!(r.game.players[1].hand.len(), 8, "returned so that 8 remain");
    assert_eq!(r.game.players[1].deck.len(), deck_before + 1);
    assert_eq!(
        r.game.card_data[r.game.players[1].deck[0].data_index].card_id,
        "OPP-HAND",
        "the returned card is at the BOTTOM of the opponent's deck"
    );
    assert!(!attacker_suspended(&r), "unsuspended after returning");
    r.auto_resolve().unwrap();
}

#[test]
fn bt13_033_when_attacking_twelve_cards_returns_four() {
    let (mut r, _) = attack_with_opp_hand(12);
    let deck_before = r.game.players[1].deck.len();
    accept_activation(&mut r);
    blind_pick(&mut r, 4);
    assert_eq!(r.game.players[1].hand.len(), 8);
    assert_eq!(r.game.players[1].deck.len(), deck_before + 4);
    let bottom: Vec<String> = ids(&r, &r.game.players[1].deck[..4]);
    assert!(bottom.iter().all(|id| id == "OPP-HAND"), "4 hand cards on the bottom: {bottom:?}");
    assert!(!attacker_suspended(&r));
    r.auto_resolve().unwrap();
}

/// 8 cards is below the threshold: no activation, attacker stays suspended.
#[test]
fn bt13_033_when_attacking_eight_cards_does_nothing() {
    let (mut r, _) = attack_with_opp_hand(8);
    if let Some(v) = r.pending_selection_view() {
        assert!(
            !matches!(v.kind, SelectionKind::CountCappedMultiSelect { .. }),
            "no blind pick below 9 cards"
        );
        assert!(
            !(v.is_optional && v.selecting_player == 0),
            "no [When Attacking] activation prompt below 9 cards: {:?}",
            v.prompt
        );
    }
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].hand.len(), 8);
    assert!(attacker_suspended(&r), "not unsuspended");
}

/// Declining the optional effect returns nothing and keeps the attacker
/// suspended.
#[test]
fn bt13_033_when_attacking_decline_keeps_hand_and_suspension() {
    let (mut r, _) = attack_with_opp_hand(10);
    assert!(r.pending_is_optional());
    r.decline_optional_trigger().unwrap();
    assert!(
        r.pending_selection_view()
            .map_or(true, |v| !matches!(v.kind, SelectionKind::CountCappedMultiSelect { .. })),
        "no pick after declining"
    );
    assert_eq!(r.game.players[1].hand.len(), 10);
    assert!(attacker_suspended(&r));
    r.auto_resolve().unwrap();
}

/// The blind pick conserves cards: hand + deck of the opponent is unchanged
/// and nothing goes to the trash (a return, not a discard).
#[test]
fn bt13_033_when_attacking_return_is_not_a_discard() {
    let (mut r, _) = attack_with_opp_hand(11);
    let total = r.game.players[1].hand.len() + r.game.players[1].deck.len();
    accept_activation(&mut r);
    blind_pick(&mut r, 3);
    assert_eq!(r.game.players[1].hand.len() + r.game.players[1].deck.len(), total);
    // (The attacked Digimon is later deleted in battle; no HAND card is trashed.)
    assert!(
        !ids(&r, &r.game.players[1].trash).contains(&"OPP-HAND".to_string()),
        "no hand card trashed by the return"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// [Burst Digivolve] (CR 6-5-1-2-3, 8-3) through the real action surface.
// Shared scenario: `crate::burst_digivolve_common`.
// ─────────────────────────────────────────────────────────────────────────────

mod burst_digivolve {
    use crate::burst_digivolve_common::*;
    use digimon_engine::enums::CardColor;

    const SPEC: BurstSpec = BurstSpec {
        card: "BT13-033",
        host_name: "MirageGaogamon",
        host_color: CardColor::Blue,
        tamer_name: "Thomas H. Norstein",
    };

    /// 8-3-3: offered via the mask with [Thomas H. Norstein] on the field;
    /// returns the Tamer, pays 0, draws 1, stacks on the [MirageGaogamon].
    #[test]
    fn bt13_033_burst_offered_returns_tamer_costs_zero_draws_one() {
        burst_and_check(SPEC, Host::BurstOnly);
    }

    /// The burst cost can't be paid without a [Thomas H. Norstein].
    #[test]
    fn bt13_033_burst_not_offered_without_tamer() {
        assert_not_offered_without_tamer(SPEC);
    }

    /// 8-3-2-1: the top card is trashed at the end of the burst turn,
    /// tracked by permanent identity (the host shifted slots).
    #[test]
    fn bt13_033_burst_top_card_trashed_at_turn_end() {
        assert_turn_end_trash(SPEC, Host::BurstOnly);
    }

    /// Normal (cost 5) and burst routes both legal: a real player choice.
    #[test]
    fn bt13_033_burst_vs_normal_route_is_a_player_choice() {
        assert_route_choice(SPEC);
    }

    /// Choosing the burst route from the route prompt, then the turn-end trash.
    #[test]
    fn bt13_033_burst_chosen_from_route_prompt_then_trashed() {
        assert_turn_end_trash(SPEC, Host::AlsoNormal);
    }

    /// 8-3-2-2: an effect that digivolves can't burst digivolve.
    #[test]
    fn bt13_033_effect_digivolve_cannot_burst() {
        assert_effect_cannot_burst(SPEC);
    }

    /// Two qualifying Tamers: the player picks which returns.
    #[test]
    fn bt13_033_burst_two_tamers_is_a_player_choice() {
        assert_two_tamers_choice(SPEC);
    }
}
