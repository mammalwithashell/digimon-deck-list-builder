//! Source: `data/card_bundles/BT27-068.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Black Lv.5 / cost 4
//!   - Red Lv.5 / cost 4
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [Gankoomon]: Cost 2
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Collision＞ ＜Reboot＞ ＜Blocker＞ [All Turns] When this Digimon would leave the battle area, by returning 1 of your other Digimon or 1 [Gankoomon] or [X Antibody] from this Digimon's digivolution cards to the bottom of the deck, it doesn't leave. [All Turns] [Once Per Turn] When any of your Digimon leave the battle area, you may activate 1 [Main] effect on this card's Option side.
//!
//! ### DUAL Effect
//! ＜Use Req. ([Huckmon] in text)＞ (Specified conditions meet color requirements.) [Main] Return 1 of your opponent's 12000 DP or higher Digimon to the top of the deck. If this effect didn't return, delete 1 of your opponent's lowest play cost Digimon.
//!
//! ### DUAL Rule
//! ＜Arts Digivolve＞ (Instead of trashing after use, your cards may digivolve into this card without paying the cost.)
//!
//! ## Official Q&A
//! - It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, "a card with [Knightmon] in its text" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.

const CARD: &str = "BT27-068";

use super::common::*;
use digimon_engine::{
    action::space::{encode_attack, HAND_EFFECT_START, PLAY_HAND_START},
    build_action_mask,
    selection::SelectionKind,
};

#[test]
fn leave_replacement_without_an_eligible_source_only_offers_another_digimon_or_decline() {
    for pay in [false, true] {
        let fatal_security = digimon("FATAL-SECURITY", 7, 17000, false);
        let mut r = start(
            builder(CARD)
                .from_dsl_yaml(include_str!("../../../cards/bt27/BT27-068.yaml"))
                .unwrap()
                .add_card(fatal_security)
                .security(1, &["FATAL-SECURITY"]),
        );
        let attacker = r.place_stack(0, &["TEXT-5", CARD]);
        r.place_on_field(0, "TEXT-3", Some(0));
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        r.accept_optional_trigger().unwrap();
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        let pick = r.pending_selection_view().unwrap();
        assert_eq!(
            pick.effect_choices
                .unwrap()
                .iter()
                .map(|choice| (choice.action_id, choice.label.as_str()))
                .collect::<Vec<_>>(),
            [
                (HAND_EFFECT_START, "Return another Digimon"),
                (HAND_EFFECT_START + 2, "Do not return a card")
            ]
        );
        assert_eq!(
            build_action_mask(&r.game, 0)[(HAND_EFFECT_START + 1) as usize],
            0.0
        );
        if pay {
            action(&mut r, 0, HAND_EFFECT_START);
            r.assert_prompt(SelectionKind::OwnField, false, 0);
            assert_eq!(candidates(&r), ["TEXT-3"]);
            r.choose_prompt_card("TEXT-3", None).unwrap();
            r.assert_prompt(SelectionKind::Replacement, true, 0);
            r.decline_prompt().unwrap();
            assert_eq!(
                r.game.players[0].deck[0].card_id(&r.game.card_data),
                "TEXT-3"
            );
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                CARD
            );
        } else {
            action(&mut r, 0, HAND_EFFECT_START + 2);
            assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-5", CARD]);
        }
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn leave_replacement_can_return_an_x_antibody_source_without_returning_the_carrier() {
    let fatal_security = digimon("FATAL-SECURITY", 7, 17000, false);
    let mut x = digimon("X", 3, 2000, false);
    x.card_name = "X Antibody".into();
    let mut r = start(
        builder(CARD)
            .add_card(x)
            .add_card(fatal_security)
            .security(1, &["FATAL-SECURITY"]),
    );
    let attacker = r.place_stack(0, &["TEXT-5", "X", CARD]);
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    assert_eq!(
        r.pending_selection().unwrap().valid_action_ids,
        [HAND_EFFECT_START + 1, HAND_EFFECT_START + 2],
    );
    action(&mut r, 0, HAND_EFFECT_START + 1);
    assert_eq!(candidates(&r), ["X"]);
    r.choose_prompt_card("X", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), ["TEXT-5", CARD]);
    assert_eq!(r.game.players[0].deck[0].card_id(&r.game.card_data), "X");
    assert!(r.pending_selection().is_none());
}

#[test]
fn another_digimon_leaving_can_refire_the_option_only_once_per_turn() {
    let fatal_security = digimon("FATAL-SECURITY", 7, 17000, false);
    let mut r = start(
        builder(CARD)
            .add_card(fatal_security)
            .security(1, &["FATAL-SECURITY", "FATAL-SECURITY"]),
    );
    let host = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "TEXT-3", Some(0));
    r.place_on_field(0, "TEXT-4", Some(0));
    r.place_on_field(1, "TEXT-6", Some(0));
    r.place_on_field(1, "TEXT-5", Some(0));
    action(&mut r, 0, encode_attack(1, 14));
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-6"]);
    r.choose_prompt_card("TEXT-6", None).unwrap();
    assert_eq!(r.game.players[1].deck.last().unwrap().card_id(&r.game.card_data), "TEXT-6");
    action(&mut r, 0, encode_attack(1, 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-3", "TEXT-4"]);
    assert_eq!(r.battle_area_size(1), 1);
    for keyword in [
        digimon_engine::enums::Keyword::Collision,
        digimon_engine::enums::Keyword::Reboot,
        digimon_engine::enums::Keyword::Blocker,
    ] {
        assert!(r.game.has_keyword(host, keyword));
    }
}

#[test]
fn option_returns_a_12000_dp_target_to_deck_instead_of_deleting_the_lower_cost_target() {
    let mut huckmon = digimon("HUCKMON", 3, 2000, false);
    huckmon.effect_text = "[Huckmon]".into();
    let mut r = start(builder(CARD).add_card(huckmon).hand(0, &[CARD]));
    r.place_on_field(0, "HUCKMON", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    r.place_on_field(1, "TEXT-6", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), ["TEXT-6"]);
    r.choose_prompt_card("TEXT-6", None).unwrap();
    if r.pending_is_optional() {
        r.decline_prompt().unwrap();
    }
    assert_eq!(r.battle_area_size(1), 1);
    assert!(r.game.players[1].trash.is_empty());
    assert_eq!(
        r.game.players[1]
            .deck
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        "TEXT-6"
    );
}

#[test]
fn option_without_a_12000_dp_target_deletes_lowest_cost_then_offers_arts() {
    for accept in [false, true] {
        let mut gankoomon = digimon("GANK", 6, 12000, false);
        gankoomon.card_name = "Gankoomon".into();
        gankoomon.effect_text = "Choose 1 [Huckmon].".into();
        let mut cheap = digimon("CHEAP", 5, 11000, false);
        cheap.play_cost = 2;
        let mut r = start(builder(CARD).add_card(gankoomon).add_card(cheap).hand(0, &[CARD]));
        let base = r.place_on_field(0, "GANK", Some(0));
        r.place_on_field(1, "TEXT-3", Some(0));
        r.place_on_field(1, "CHEAP", Some(0));
        action(&mut r, 0, PLAY_HAND_START);
        assert_eq!(candidates(&r), ["CHEAP"]);
        r.choose_prompt_card("CHEAP", None).unwrap();
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), ["GANK"]);
        if accept {
            action(&mut r, 0, encode_attack(0, base.index as u16));
            assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), ["GANK", CARD]);
            for keyword in [digimon_engine::enums::Keyword::Collision, digimon_engine::enums::Keyword::Reboot, digimon_engine::enums::Keyword::Blocker] {
                assert!(r.game.has_keyword(base, keyword));
            }
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(ids(&r, &r.game.players[0].trash), [CARD]);
        }
        assert_eq!(ids(&r, &r.game.players[1].trash), ["CHEAP"]);
        assert_eq!(r.memory(), 5);
        assert!(r.pending_selection().is_none());
    }
}
