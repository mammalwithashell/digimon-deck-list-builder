//! Source: `data/card_bundles/BT27-070.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Black Lv.5 / cost 4
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [Craniamon]: Cost 2
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Reboot＞ ＜Blocker＞ [All Turns] When any of your Digimon would leave the battle area, by suspending this Digimon, they don't leave. [All Turns] [Once Per Turn] When this Digimon suspends, you may delete all of your opponent's lowest DP Digimon. Then, by trashing 1 [Craniamon] or [X Antibody] from this Digimon's digivolution cards, activate 1 [Main] effect on this card's Option side.
//!
//! ### DUAL Effect
//! [Main] Until your opponent's turn ends, 1 of your Digimon with ＜Blocker＞ isn't affected by their Digimon effects and gets +5000 DP. Then, 1 of your Digimon may unsuspend.
//!
//! ### DUAL Rule
//! ＜Arts Digivolve＞ (Instead of trashing after use, your cards may digivolve into this card without paying the cost.)
//!
//! ## Official Q&A
//! - This effect triggers and can be activated when a Digimon specified by it leaves the battle area under the specified conditions. When activated, all Digimon matching its trigger conditions are prevented from leaving.

const CARD: &str = "BT27-070";

use super::common::*;
use digimon_engine::action::space::encode_attack;

#[test]
fn attacking_suspends_and_can_delete_all_tied_lowest_dp_digimon() {
    let mut r = start(builder(CARD));
    let attacker = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    r.place_on_field(1, "TEXT-4", Some(0));
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    let prompt = r.pending_selection_view().unwrap();
    let delete = prompt
        .effect_choices
        .unwrap()
        .into_iter()
        .find(|choice| choice.label == "Delete the lowest DP Digimon")
        .unwrap();
    action(&mut r, 0, delete.action_id);
    assert_eq!(
        ids(&r, &r.game.players[1].trash)
            .into_iter()
            .filter(|id| id == "TEXT-3")
            .count(),
        2
    );
    assert_eq!(r.battle_area_size(1), 1);
}

#[test]
fn suspending_can_skip_deletion_and_pay_x_antibody_to_protect_a_blocker() {
    use digimon_engine::enums::{EffectSourceKind, Keyword};
    use digimon_engine::selection::SelectionKind;

    for pay in [false, true] {
        let mut x = digimon("X", 4, 1000, false);
        x.card_name = "X Antibody".into();
        let mut r = start(builder(CARD).add_card(x));
        let attacker = r.place_stack(0, &["TEXT-4", "X", CARD]);
        r.place_on_field(0, "TEXT-3", Some(0));
        r.place_on_field(1, "TEXT-3", Some(0));
        assert!(r.game.has_keyword(attacker, Keyword::Reboot));
        assert!(r.game.has_keyword(attacker, Keyword::Blocker));
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        let skip = r.pending_selection_view().unwrap().effect_choices.unwrap()
            .into_iter().find(|choice| choice.label == "Do not delete").unwrap();
        action(&mut r, 0, skip.action_id);
        assert_eq!(candidates(&r), ["X"]);
        if pay {
            r.choose_prompt_card("X", None).unwrap();
            r.assert_prompt(SelectionKind::OwnField, false, 0);
            assert_eq!(candidates(&r), [CARD]);
            r.choose_prompt_card(CARD, None).unwrap();
            assert_eq!(r.effective_dp(attacker), Some(18000));
            assert!(!r.game.effect_can_affect(attacker, 1, EffectSourceKind::Digimon));
            assert!(r.game.effect_can_affect(attacker, 1, EffectSourceKind::Option));
            r.assert_prompt(SelectionKind::OwnField, true, 0);
            assert_eq!(candidates(&r), [CARD]);
            r.choose_prompt_card(CARD, None).unwrap();
            assert!(!r.game.players[0].battle_area[0].is_suspended);
            assert_eq!(ids(&r, &r.game.players[0].trash), ["X"]);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.effective_dp(attacker), Some(13000));
            assert!(r.game.players[0].battle_area[0].is_suspended);
            assert!(r.game.players[0].trash.is_empty());
        }
        assert_eq!(r.battle_area_size(1), 1);
    }
}

#[test]
fn craniamon_can_suspend_to_keep_another_digimon_from_leaving_or_decline() {
    use digimon_engine::action::space::PLAY_HAND_START;
    use digimon_engine::selection::SelectionKind;

    for protect in [false, true] {
        let mut r = start(
            builder(CARD)
                .dsl_card("BT27-103")
                .unwrap()
                .hand(1, &["BT27-103"]),
        );
        let craniamon = r.place_on_field(0, CARD, Some(0));
        r.place_on_field(0, "TEXT-3", Some(0));
        r.place_on_field(1, "TEXT-4", Some(0));
        next_turn(&mut r);
        r.game.set_memory(10);
        action(&mut r, 1, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::OppField, false, 1);
        assert_eq!(candidates(&r), [CARD, "TEXT-3"]);
        r.choose_prompt_card("TEXT-3", None).unwrap();
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        if protect {
            r.accept_optional_trigger().unwrap();
            r.assert_prompt(SelectionKind::EffectChoice, false, 0);
            let skip = r.pending_selection_view().unwrap().effect_choices.unwrap()
                .into_iter().find(|choice| choice.label == "Do not delete").unwrap();
            action(&mut r, 0, skip.action_id);
            assert!(r.game.players[0].battle_area[craniamon.index as usize].is_suspended);
            assert_eq!(r.battle_area_size(0), 2);
            assert!(r.game.players[0].trash.is_empty());
        } else {
            r.decline_prompt().unwrap();
            assert!(!r.game.players[0].battle_area[craniamon.index as usize].is_suspended);
            assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-3"]);
        }
    }
}

#[test]
fn one_suspension_protects_all_digimon_in_the_same_deletion_batch() {
    use digimon_engine::selection::SelectionKind;

    let mut ally = digimon("ALLY", 3, 2000, false);
    ally.play_cost = 5;
    let mut r = start(
        builder(CARD)
            .dsl_card("BT27-100")
            .unwrap()
            .add_card(ally)
            .security(1, &["BT27-100"]),
    );
    let craniamon = r.place_on_field(0, CARD, Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    action(&mut r, 0, encode_attack(ally.index.into(), 14));
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    let skip = r
        .pending_selection_view()
        .unwrap()
        .effect_choices
        .unwrap()
        .into_iter()
        .find(|choice| choice.label == "Do not delete")
        .unwrap();
    action(&mut r, 0, skip.action_id);
    assert!(r.game.players[0].battle_area[craniamon.index as usize].is_suspended);
    assert_eq!(r.battle_area_size(0), 2);
    assert!(r.game.players[0].trash.is_empty());
    assert_eq!(ids(&r, &r.game.players[1].hand), ["BT27-100"]);
    assert!(r.pending_selection().is_none());
}

#[test]
fn used_option_arts_evolves_craniamon_for_no_additional_memory() {
    use digimon_engine::action::space::PLAY_HAND_START;
    use digimon_engine::selection::SelectionKind;
    let mut base = digimon("BASE", 6, 12000, false);
    base.card_name = "Craniamon".into();
    let mut r = start(builder(CARD).add_card(base).hand(0, &[CARD]));
    let base = r.place_on_field(0, "BASE", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    assert_eq!(candidates(&r), ["BASE"]);
    action(&mut r, 0, encode_attack(0, base.index as u16));
    assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), ["BASE", CARD]);
    assert_eq!(r.memory(), 5);
}
