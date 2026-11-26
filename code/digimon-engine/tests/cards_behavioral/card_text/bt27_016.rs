//! Source: `data/card_bundles/BT27-016.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Red Lv.5 / cost 3
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [Jesmon]: Cost 1
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Progress＞＜Piercing＞ ＜Engage＞ (At the end of your turn, this Digimon may attack.) [When Digivolving] [When Attacking] [Once Per Turn] This Digimon gets +6000 DP until your opponent's turn ends. Then, by returning 1 of your other Digimon or 1 [Jesmon] or [X Antibody] from this Digimon's digivolution cards to the bottom of the deck, activate 1 [Main] effect on this card's Option side.
//!
//! ### DUAL Effect
//! ＜Use Req. ([Huckmon] in text)＞ (Specified conditions meet color requirements.) [Main] By placing 1 of your opponent's lowest DP Digimon as the bottom security card, trash their top security card.
//!
//! ### DUAL Rule
//! ＜Arts Digivolve＞ (Instead of trashing after use, your cards may digivolve into this card without paying the cost.)
//!
//! ## Official Q&A
//! - It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, "a card with [Knightmon] in its text" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.

const CARD: &str = "BT27-016";

use super::common::*;
use digimon_engine::{
    action::space::{encode_attack, encode_digivolve, HAND_EFFECT_START},
    build_action_mask,
    debug_runner::make_test_card,
    enums::EffectSourceKind,
    selection::SelectionKind,
};

#[test]
fn attack_boosts_by_6000_and_returning_a_jesmon_source_uses_the_option_face() {
    let jesmon = make_test_card("JESMON", "Jesmon");
    let mut r = start(
        builder(CARD)
            .from_dsl_yaml(include_str!("../../../cards/bt27/BT27-016.yaml"))
            .unwrap()
            .add_card(jesmon),
    );
    let attacker = r.place_stack(0, &["JESMON", CARD]);
    r.place_on_field(1, "TEXT-3", Some(0));
    r.place_on_field(1, "TEXT-4", Some(0));
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    assert_eq!(r.effective_dp(attacker), Some(18000));
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    let pick = r.pending_selection_view().unwrap();
    let choices = pick.effect_choices.unwrap();
    assert_eq!(
        choices
            .iter()
            .map(|choice| (choice.action_id, choice.label.as_str()))
            .collect::<Vec<_>>(),
        [
            (HAND_EFFECT_START + 1, "Return a digivolution card"),
            (HAND_EFFECT_START + 2, "Don't return a card")
        ]
    );
    assert_eq!(
        build_action_mask(&r.game, 0)[HAND_EFFECT_START as usize],
        0.0
    );
    action(&mut r, 0, HAND_EFFECT_START + 1);
    r.choose_prompt_card("JESMON", None).unwrap();
    let option = r.pending_selection_view().unwrap();
    assert_eq!(option.source_kind, EffectSourceKind::Option);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(
        r.game.players[1].security[0].card_id(&r.game.card_data),
        "TEXT-3"
    );
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
}

#[test]
fn digivolving_boosts_dp_even_without_payment_and_attacking_cannot_boost_again() {
    use digimon_engine::enums::CardColor;

    let mut base = digimon("RED-BASE", 5, 7000, false);
    base.colors = vec![CardColor::Red];
    let card_data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cards.json");
    let printed = digimon_engine::CardData::load_from_file(&card_data)
        .unwrap()
        .remove(CARD)
        .unwrap();
    let mut r = start(builder(CARD).add_card(printed).add_card(base).hand(0, &[CARD]));
    let jesmon = r.place_on_field(0, "RED-BASE", Some(0));
    action(&mut r, 0, encode_digivolve(0, jesmon.index as u16));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.effective_dp(jesmon), Some(18000));

    action(&mut r, 0, encode_attack(jesmon.index as u16, 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.effective_dp(jesmon), Some(18000));
    assert_eq!(r.security_count(1), 4);
    next_turn(&mut r);
    assert_eq!(r.effective_dp(jesmon), Some(18000));
    next_turn(&mut r);
    assert_eq!(r.effective_dp(jesmon), Some(12000));
}

#[test]
fn used_option_arts_digivolves_onto_jesmon_and_can_return_another_digimon() {
    use digimon_engine::action::space::PLAY_HAND_START;
    use digimon_engine::enums::Keyword;

    let mut base = digimon("BASE", 6, 11000, false);
    base.card_name = "Jesmon".into();
    let mut ally = digimon("ALLY", 3, 2000, false);
    ally.effect_text = "Choose 1 [Huckmon].".into();
    let mut r = start(builder(CARD).add_card(base).add_card(ally).hand(0, &[CARD]));
    let jesmon = r.place_on_field(0, "BASE", Some(0));
    r.place_on_field(0, "ALLY", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    assert_eq!(candidates(&r), ["BASE"]);
    action(&mut r, 0, encode_attack(0, jesmon.index.into()));
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    action(&mut r, 0, HAND_EFFECT_START);
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), ["ALLY"]);
    r.choose_prompt_card("ALLY", None).unwrap();
    assert_eq!(r.game.players[0].deck[0].card_id(&r.game.card_data), "ALLY");
    assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), ["BASE", CARD]);
    assert_eq!(r.effective_dp(jesmon), Some(18000));
    assert_eq!(r.memory(), 5);
    for keyword in [Keyword::Progress, Keyword::Piercing, Keyword::Engage] {
        assert!(r.game.has_keyword(jesmon, keyword));
    }
}

#[test]
fn huckmon_text_on_a_non_matching_color_permanent_satisfies_the_use_requirement() {
    use digimon_engine::action::space::PLAY_HAND_START;
    use digimon_engine::enums::CardColor;
    for huckmon_text in [false, true] {
        let mut anchor = digimon("ANCHOR", 3, 2000, false);
        anchor.colors = vec![CardColor::Black];
        if huckmon_text {
            anchor.effect_text = "Choose 1 [Huckmon].".into();
        }
        let mut r = start(builder(CARD).add_card(anchor).hand(0, &[CARD]));
        r.place_on_field(0, "ANCHOR", Some(0));
        assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.5, huckmon_text);
    }
}
