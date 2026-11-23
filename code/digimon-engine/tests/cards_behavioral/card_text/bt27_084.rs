//! Source: `data/card_bundles/BT27-084.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Purple Lv.5 / cost 4
//!   - Black Lv.5 / cost 4
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [LordKnightmon]: Cost 2
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Alliance＞ ＜Collision＞ ＜Piercing＞ ＜Blocker＞ [When Digivolving] You may play up to 8 play cost's total worth of cards with [Knightmon] in their texts from your hand without paying the cost. [All Turns] [Once Per Turn] When any of your other Digimon are played or deleted, by trashing 1 [LordKnightmon] or [X Antibody] from this Digimon's digivolution cards, activate 1 [Main] effect on this card's Option side.
//!
//! ### DUAL Effect
//! [Main] Delete 1 of your opponent's lowest level Digimon. If this effect didn't delete, until your turn ends, all of your Digimon with [Knightmon] in their texts gain ＜Blocker＞, your opponent's Digimon effects don't affect them and they get +3000 DP.
//!
//! ### DUAL Rule
//! ＜Arts Digivolve＞ (Instead of trashing after use, your cards may digivolve into this card without paying the cost.)
//!
//! ## Official Q&A
//! - It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, "a card with [Knightmon] in its text" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.

const CARD: &str = "BT27-084";

use super::common::*;
use digimon_engine::action::space::PLAY_HAND_START;

#[test]
fn option_deletes_a_lowest_level_digimon_regardless_of_dp() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_on_field(0, "TEXT-4", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    r.place_on_field(1, "TEXT-4", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    if r.pending_is_optional() {
        r.decline_prompt().unwrap();
    }
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-3"]);
    assert_eq!(r.battle_area_size(1), 1);
}

#[test]
fn option_without_a_deletion_buffs_only_knightmon_text_and_expires_at_turn_end() {
    use digimon_engine::enums::{EffectSourceKind, Keyword};

    let mut darkknightmon = digimon("DARK", 4, 4000, false);
    darkknightmon.card_name = "DarkKnightmon".into();
    let mut knightmon_text = digimon("TEXT", 4, 4000, false);
    knightmon_text.effect_text = "Play 1 [Knightmon].".into();
    let mut r = start(
        builder(CARD)
            .add_card(darkknightmon)
            .add_card(knightmon_text)
            .hand(0, &[CARD]),
    );
    let dark = r.place_on_field(0, "DARK", Some(0));
    let text = r.place_on_field(0, "TEXT", Some(0));
    let other = r.place_on_field(0, "TEXT-4", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    assert!(r.pending_selection().is_none());
    for target in [dark, text] {
        assert_eq!(r.effective_dp(target), Some(7000));
        assert!(r.game.has_keyword(target, Keyword::Blocker));
        assert!(!r.game.effect_can_affect(target, 1, EffectSourceKind::Digimon));
        assert!(r.game.effect_can_affect(target, 1, EffectSourceKind::Option));
    }
    assert_eq!(r.effective_dp(other), Some(4000));
    assert!(!r.game.has_keyword(other, Keyword::Blocker));
    next_turn(&mut r);
    for target in [dark, text] {
        assert_eq!(r.effective_dp(target), Some(4000));
        assert!(!r.game.has_keyword(target, Keyword::Blocker));
        assert!(r.game.effect_can_affect(target, 1, EffectSourceKind::Digimon));
    }
}

#[test]
fn digivolving_can_play_knightmon_text_cards_up_to_eight_total_or_decline() {
    use digimon_engine::action::space::encode_digivolve;
    use digimon_engine::enums::Keyword;
    use digimon_engine::selection::SelectionKind;

    for (play, pay) in [(false, false), (true, false), (true, true)] {
        let knight = |id: &str, cost| {
            let mut d = digimon(id, 4, 4000, false);
            d.effect_text = "Choose a [Knightmon].".into();
            d.play_cost = cost;
            d
        };
        let mut lordknightmon = digimon("BASE", 6, 12000, false);
        lordknightmon.card_name = "LordKnightmon".into();
        let mut r = start(
            builder(CARD)
                .add_card(lordknightmon)
                .add_card(knight("FIVE-A", 5))
                .add_card(knight("FIVE-B", 5))
                .add_card(knight("THREE", 3))
                .add_card(knight("NINE", 9))
                .hand(0, &[CARD, "FIVE-A", "FIVE-B", "THREE", "NINE", "TEXT-3"]),
        );
        let base = r.place_on_field(0, "BASE", Some(0));
        action(&mut r, 0, encode_digivolve(0, base.index as u16));
        for keyword in [Keyword::Alliance, Keyword::Collision, Keyword::Piercing, Keyword::Blocker] {
            assert!(r.game.has_keyword(base, keyword));
        }
        assert!(r.pending_is_optional());
        assert_eq!(candidates(&r), ["FIVE-A", "FIVE-B", "THREE"]);
        if play {
            r.choose_prompt_card("FIVE-A", None).unwrap();
            assert_eq!(candidates(&r), ["THREE"]);
            r.choose_prompt_card("THREE", None).unwrap();
            r.assert_prompt(SelectionKind::TriggerOrder, false, 0);
            let prompt = r.pending_selection_view().unwrap();
            assert_eq!(prompt.effect_choices.as_ref().unwrap().len(), 2);
            action(&mut r, 0, prompt.valid_action_ids[0]);
            assert_eq!(candidates(&r), ["BASE"]);
            if pay {
                r.choose_prompt_card("BASE", None).unwrap();
                assert_eq!(ids(&r, &r.game.players[0].trash), ["BASE"]);
                for i in [1, 2] {
                    let bonus = r.perm_handle(0, i);
                    assert_eq!(r.effective_dp(bonus), Some(7000));
                    assert!(r.game.has_keyword(bonus, Keyword::Blocker));
                }
            } else {
                r.decline_prompt().unwrap();
            }
            assert_eq!(r.battle_area_size(0), 3);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.battle_area_size(0), 1);
        }
        assert_eq!(r.memory(), 8);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn ally_deletion_on_the_opponents_turn_can_trash_an_x_antibody_source_to_refire_main() {
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::enums::EffectSourceKind;
    use digimon_engine::selection::SelectionKind;

    let mut x = digimon("X", 3, 2000, false);
    x.card_name = "X Antibody".into();
    for pay in [false, true] {
        let mut r = start(builder(CARD).add_card(x.clone()));
        r.place_stack(0, &["TEXT-4", "X", CARD]);
        let ally = r.place_on_field(0, "TEXT-3", Some(0));
        r.game.players[0].battle_area[ally.index as usize].is_suspended = true;
        let attacker = r.place_on_field(1, "TEXT-5", Some(0));
        r.place_on_field(1, "TEXT-4", Some(0));
        next_turn(&mut r);
        action(&mut r, 1, encode_attack(attacker.index.into(), ally.index.into()));
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), [CARD]);
        r.decline_prompt().unwrap();
        assert!(r.pending_is_optional());
        assert_eq!(candidates(&r), ["X"]);
        if pay {
            r.choose_prompt_card("X", None).unwrap();
            r.assert_prompt(SelectionKind::OppField, false, 0);
            assert_eq!(r.pending_selection_view().unwrap().source_kind, EffectSourceKind::Option);
            assert_eq!(candidates(&r), ["TEXT-4"]);
            r.choose_prompt_card("TEXT-4", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.battle_area_size(0), 1);
        assert_eq!(r.battle_area_size(1), 2 - usize::from(pay));
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn playing_then_losing_another_digimon_shares_one_activation_per_turn() {
    use digimon_engine::action::space::encode_attack;

    let mut x = digimon("X", 3, 2000, false);
    x.card_name = "X Antibody".into();
    let mut r = start(
        builder(CARD).add_card(x)
            .add_card(digimon("FATAL", 7, 20000, false))
            .hand(0, &["TEXT-4"])
            .security(1, &["FATAL"]),
    );
    r.place_stack(0, &["TEXT-5", "X", "X", CARD]);
    let ally = r.place_on_field(0, "TEXT-3", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), ["X", "X"]);
    let payment = r.pending_selection_view().unwrap().valid_action_ids[0];
    action(&mut r, 0, payment);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-3"]);

    action(&mut r, 0, encode_attack(ally.index.into(), 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(ids(&r, &r.game.players[0].trash), ["X", "TEXT-3"]);
    assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), ["TEXT-5", "X", CARD]);
}

#[test]
fn used_option_can_arts_digivolve_onto_lordknightmon_at_no_extra_cost() {
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::selection::SelectionKind;

    let mut lordknightmon = digimon("BASE", 6, 12000, false);
    lordknightmon.card_name = "LordKnightmon".into();
    let mut r = start(builder(CARD).add_card(lordknightmon).hand(0, &[CARD]));
    let base = r.place_on_field(0, "BASE", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    assert_eq!(candidates(&r), ["BASE"]);
    action(&mut r, 0, encode_attack(0, base.index.into()));
    assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), ["BASE", CARD]);
    assert_eq!(r.memory(), 5);
}
