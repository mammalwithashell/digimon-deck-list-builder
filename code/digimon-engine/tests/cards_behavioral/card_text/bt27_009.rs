//! Source: `data/card_bundles/BT27-009.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Red Lv.3 / cost 3
//!   - Black Lv.3 / cost 3
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [Ryudamon]/Lv.3 w/[Chronicle] trait: Cost 2
//!
//! ## Printed text (official)
//! ### Effect
//! [On Play] [When Digivolving] 1 of your Digimon gets +3000 DP until your opponent's turn ends. [When Attacking] This Digimon may digivolve into a Digimon card with the [Chronicle] trait in the hand or trash.
//!
//! ### Inherited Effect
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! ## Official Q&A
//! - Yes, you pay the digivolution cost.

const CARD: &str = "BT27-009";

#[test]
fn official_qa_attack_digivolution_is_optional_and_pays_the_printed_cost_from_either_zone() {
    use digimon_engine::action::space::encode_attack;
    for zone in [None, Some(true), Some(false)] {
        let mut r = start(builder(CARD).dsl_card("BT27-013").unwrap());
        let attacker = r.place_on_field(0, CARD, Some(0));
        r.add_to_hand(0, "BT27-013");
        r.inject_trash(0, "BT27-013");
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        assert_eq!(candidates(&r), ["BT27-013", "BT27-013"]);
        if let Some(from_hand) = zone {
            let view = r.pending_selection_view().unwrap();
            let rows = view.card_choice.unwrap().rows;
            let row = rows
                .iter()
                .find(|row| {
                    use digimon_engine::owned_card_choice::CardChoiceZone;
                    match row.zone {
                        Some(CardChoiceZone::Hand) => from_hand,
                        Some(CardChoiceZone::Trash) => !from_hand,
                        _ => false,
                    }
                })
                .unwrap();
            action(&mut r, 0, row.action_id);
            assert_eq!(r.memory(), 7);
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                "BT27-013"
            );
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.memory(), 10);
        }
        assert!(r.pending_selection().is_none());
    }
}

use super::common::*;
use digimon_engine::action::space::PLAY_HAND_START;

#[test]
fn on_play_grants_chosen_digimon_3000_dp_through_the_opponents_turn() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    let ally = r.place_on_field(0, "TEXT-3", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), [CARD, "TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(r.effective_dp(ally), Some(5000));
    next_turn(&mut r);
    assert_eq!(r.effective_dp(ally), Some(5000));
    next_turn(&mut r);
    assert_eq!(r.effective_dp(ally), Some(2000));
}

#[test]
fn inherited_2000_dp_applies_only_during_your_turn() {
    let mut r = start(builder(CARD));
    let host = r.place_stack(0, &[CARD, "TEXT-4"]);
    assert_eq!(r.effective_dp(host), Some(6000));
    next_turn(&mut r);
    assert_eq!(r.effective_dp(host), Some(4000));
}

#[test]
fn when_digivolving_the_dp_bonus_can_target_ginryumon() {
    use digimon_engine::action::space::encode_digivolve;
    let mut ryudamon = digimon("RYUDA", 3, 2000, false);
    ryudamon.card_name = "Ryudamon".into();
    let mut r = start(builder(CARD).add_card(ryudamon).hand(0, &[CARD]));
    let base = r.place_on_field(0, "RYUDA", Some(0));
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert_eq!(candidates(&r), [CARD]);
    r.choose_prompt_card(CARD, None).unwrap();
    assert_eq!(r.effective_dp(base), Some(8000));
}
