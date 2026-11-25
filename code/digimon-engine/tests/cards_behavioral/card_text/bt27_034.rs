//! Source: `data/card_bundles/BT27-034.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Blue Lv.6 / cost 6
//!   - Red Lv.6 / cost 6
//!   - Green Lv.6 / cost 6
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [Examon]: Cost 2
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Raid＞ ＜Piercing＞ ＜Security A.+1＞ ＜Blocker＞ ＜Evade＞ ＜Decode (Green Lv.5)＞ ＜Decode (Blue Lv.5)＞ [When Digivolving] [When Attacking] [Counter] [Once Per Turn] You may suspend up to 3 Digimon or Tamers. Then, by trashing 1 [Examon] or [X Antibody] from this Digimon's digivolution cards, activate 1 [Main] effect on this card's Option side.
//!
//! ### DUAL Effect
//! [Main] Delete up to 9000 DP's total worth of your opponent's Digimon. For each of their Digimon and Tamers, add 3000 to this effect's DP maximum. If this effect didn't delete, trash 1 of their Option cards in the battle area.
//!
//! ### DUAL Rule
//! ＜Arts Digivolve＞ (Instead of trashing after use, your cards may digivolve into this card without paying the cost.)
//!
//! ## Official Q&A
//! - A [Main] effect shown on the DUAL card's Option side will activate. Activated effects on the Option side are treated as Option card effects.

const CARD: &str = "BT27-034";

use super::common::*;
use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START};
use digimon_engine::selection::SelectionKind;

#[test]
fn option_dp_budget_grows_by_3000_for_each_opponents_digimon_or_tamer() {
    let mut red = digimon("RED", 3, 3000, false);
    red.colors = vec![digimon_engine::enums::CardColor::Red];
    let mut r = start(builder(CARD).add_card(red).hand(0, &[CARD]));
    r.place_on_field(0, "RED", Some(0));
    r.place_on_field(1, "TEXT-TAMER", Some(0));
    let target = r.place_on_field(1, "TEXT-6", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    let delete = encode_attack(0, target.index as u16);
    r.assert_prompt(
        SelectionKind::DpBudget {
            remaining_dp: 15000,
            picked: 0,
        },
        true,
        0,
    );
    assert_eq!(
        r.pending_selection().unwrap().valid_action_ids,
        [delete, PASS]
    );
    action(&mut r, 0, delete);
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-6"]);
}

#[test]
fn option_trashes_an_opponents_field_option_if_no_digimon_is_deleted() {
    use digimon_engine::enums::CardColor;
    let mut red = digimon("RED", 3, 3000, false);
    red.colors = vec![CardColor::Red];
    let mut r = start(
        builder(CARD)
            .add_card(red)
            .dsl_card("BT27-100")
            .unwrap()
            .dsl_card("BT23-059")
            .unwrap()
            .hand(0, &[CARD])
            .hand(1, &["BT27-100"]),
    );
    let red = r.place_on_field(0, "RED", Some(0));
    let digimon = r.place_on_field(1, "TEXT-6", Some(0));
    next_turn(&mut r);
    action(&mut r, 1, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 1);
    assert_eq!(candidates(&r), ["RED"]);
    r.choose_prompt_card("RED", None).unwrap();
    assert_eq!(r.battle_area_size(1), 2);
    next_turn(&mut r);
    r.assert_prompt(SelectionKind::Target, false, 0);
    assert_eq!(
        r.pending_selection().unwrap().valid_action_ids,
        [encode_attack(red.index as u16, 14)]
    );
    action(&mut r, 0, encode_attack(red.index as u16, 14));
    assert_eq!(r.battle_area_size(1), 2);
    let observer = r.place_on_field(0, "BT23-059", Some(0));
    r.game.players[0].battle_area[observer.index as usize].is_suspended = true;

    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(
        SelectionKind::DpBudget {
            remaining_dp: 12000,
            picked: 0,
        },
        true,
        0,
    );
    assert_eq!(
        r.pending_selection().unwrap().valid_action_ids,
        [encode_attack(0, digimon.index as u16), PASS]
    );
    r.decline_prompt().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["BT27-100"]);
    r.choose_prompt_card("BT27-100", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-FILLER", "BT27-100"]);
    assert_eq!(r.battle_area_size(1), 1);
    assert!(!r.game.players[0].battle_area[observer.index as usize].is_suspended);
}

#[test]
fn field_counter_can_skip_suspension_and_pay_x_antibody_to_remove_the_attacker() {
    use digimon_engine::enums::{EffectSourceKind, GamePhase};

    let mut x = digimon("X", 3, 1000, false);
    x.card_name = "X Antibody".into();
    let mut r = start(builder(CARD).add_card(x));
    let attacker = r.place_on_field(0, "TEXT-6", Some(0));
    let examon = r.place_stack(1, &["X", CARD]);
    action(&mut r, 0, encode_attack(attacker.index.into(), 14));
    assert_eq!(r.current_phase(), GamePhase::CounterTiming);
    r.assert_prompt(SelectionKind::Hand, true, 1);
    let counter = encode_attack(0, examon.index.into());
    assert_eq!(r.pending_selection().unwrap().valid_action_ids, [counter]);
    action(&mut r, 1, counter);
    r.assert_prompt(SelectionKind::AnyField, true, 1);
    assert_eq!(candidates(&r), [CARD]);
    r.decline_prompt().unwrap();
    assert_eq!(candidates(&r), ["X"]);
    r.choose_prompt_card("X", None).unwrap();
    r.assert_prompt(
        SelectionKind::DpBudget { remaining_dp: 12000, picked: 0 },
        true,
        1,
    );
    assert_eq!(r.pending_selection_view().unwrap().source_kind, EffectSourceKind::Option);
    action(&mut r, 1, encode_attack(0, attacker.index.into()));
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-6"]);
    assert_eq!(ids(&r, &r.game.players[1].trash), ["X"]);
    assert_eq!(r.security_count(1), 5);
}

#[test]
fn decode_plays_the_matching_green_or_blue_level_five_without_preventing_deletion() {
    use digimon_engine::enums::CardColor;

    for color in [CardColor::Green, CardColor::Blue] {
        let mut matching = digimon("MATCH", 5, 6000, false);
        matching.colors = vec![color];
        let mut off_color = digimon("OFF", 5, 6000, false);
        off_color.colors = vec![CardColor::Red];
        let mut off_level = digimon("LEVEL", 4, 4000, false);
        off_level.colors = vec![color];
        let mut r = start(
            builder(CARD)
                .dsl_card("BT27-103")
                .unwrap()
                .add_card(matching)
                .add_card(off_color)
                .add_card(off_level)
                .hand(1, &["BT27-103"]),
        );
        let examon = r.place_stack(0, &["MATCH", "OFF", "LEVEL", CARD]);
        r.game.players[0].battle_area[examon.index as usize].is_suspended = true;
        r.place_on_field(1, "TEXT-4", Some(0));
        next_turn(&mut r);
        action(&mut r, 1, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::OppField, false, 1);
        assert_eq!(candidates(&r), [CARD]);
        r.choose_prompt_card(CARD, None).unwrap();
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        r.accept_optional_trigger().unwrap();
        r.assert_prompt(SelectionKind::Material, false, 0);
        assert_eq!(candidates(&r), ["MATCH"]);
        r.choose_prompt_card("MATCH", None).unwrap();
        assert_eq!(ids(&r, &r.game.players[0].trash), ["OFF", "LEVEL", CARD]);
        assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), ["MATCH"]);
        assert_eq!(r.battle_area_size(0), 1);
    }
}

#[test]
fn attacking_security_checks_two_cards_without_an_eligible_option_payment() {
    let mut r = start(builder(CARD));
    let attacker = r.place_stack(0, &["TEXT-5", CARD]);
    action(&mut r, 0, encode_attack(attacker.index.into(), 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.security_count(1), 3);
    assert!(r.game.players[0].trash.is_empty());
}

#[test]
fn arts_digivolution_suspends_digimon_and_tamers_then_can_pay_examon_for_the_option_main() {
    use digimon_engine::enums::{CardColor, EffectSourceKind, Keyword};

    for pay in [false, true] {
        let mut examon = digimon("EXAMON", 7, 15000, false);
        examon.card_name = "Examon".into();
        examon.colors = vec![CardColor::Red];
        let mut r = start(builder(CARD).add_card(examon).hand(0, &[CARD]));
        let base = r.place_on_field(0, "EXAMON", Some(0));
        let ally = r.place_on_field(0, "TEXT-3", Some(0));
        let tamer = r.place_on_field(1, "TEXT-TAMER", Some(0));
        let victim = r.place_on_field(1, "TEXT-6", Some(0));
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(
            SelectionKind::DpBudget { remaining_dp: 15000, picked: 0 },
            true,
            0,
        );
        r.decline_prompt().unwrap();
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), ["EXAMON"]);
        action(&mut r, 0, encode_attack(0, base.index as u16));
        r.assert_prompt(SelectionKind::AnyField, true, 0);
        assert_eq!(
            r.pending_selection().unwrap().valid_action_ids,
            [
                encode_attack(0, base.index as u16),
                encode_attack(0, ally.index as u16),
                encode_attack(1, tamer.index as u16),
                encode_attack(1, victim.index as u16),
            ],
        );
        for target in [ally, tamer, victim] {
            action(&mut r, 0, encode_attack(target.player.into(), target.index.into()));
        }
        for target in [ally, tamer, victim] {
            assert!(r.game.players[target.player as usize].battle_area[target.index as usize].is_suspended);
        }
        assert_eq!(candidates(&r), ["EXAMON"]);
        if pay {
            r.choose_prompt_card("EXAMON", None).unwrap();
            r.assert_prompt(
                SelectionKind::DpBudget { remaining_dp: 15000, picked: 0 },
                true,
                0,
            );
            assert_eq!(r.pending_selection_view().unwrap().source_kind, EffectSourceKind::Option);
            action(&mut r, 0, encode_attack(0, victim.index as u16));
            assert_eq!(ids(&r, &r.game.players[0].trash), ["EXAMON"]);
            assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-6"]);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(ids(&r, &r.game.players[0].battle_area[base.index as usize].card_sources), ["EXAMON", CARD]);
            assert!(r.game.players[1].trash.is_empty());
        }
        for keyword in [Keyword::Raid, Keyword::Piercing, Keyword::Blocker, Keyword::Evade] {
            assert!(r.game.has_keyword(base, keyword));
        }
    }
}
