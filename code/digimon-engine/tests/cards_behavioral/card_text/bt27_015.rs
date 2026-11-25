//! Source: `data/card_bundles/BT27-015.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Red Lv.5 / cost 4
//!   - Black Lv.5 / cost 4
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [Hisyaryumon]/Lv.5 w/[Chronicle] trait: Cost 3
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Piercing＞ ＜Reboot＞ ＜Blocker＞ [On Play] [When Digivolving] Delete 1 of your opponent's Digimon with as much DP as 1 of your Digimon or less. Then, if during an attack, trash their top security card. [All Turns] [Once Per Turn] When security stacks are removed from, you may play or use 1 [Chronicle] trait Tamer or Option card from your hand without paying the cost.
//!
//! ### Special Play Condition
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all w/[Chronicle] trait
//!
//! ### Inherited Effect
//! [All Turns] [Once Per Turn] When security stacks are removed from, this [Alphamon: Ouryuken] may unsuspend.
//!
//! ## Official Q&A
//! - The [Security] effect is activated first. When a [Security] effect is checked, it is activated immediately without waiting. Activate the other triggered effects in order, starting with the turn player's effects.

const CARD: &str = "BT27-015";

use super::common::*;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::selection::SelectionKind;

#[test]
fn assembly_uses_three_different_chronicle_levels_for_a_five_cost_reduction() {
    let mut r = start(
        builder(CARD)
            .dsl_card("BT27-008")
            .unwrap()
            .dsl_card("BT27-009")
            .unwrap()
            .dsl_card("BT27-013")
            .unwrap()
            .hand(0, &[CARD]),
    );
    for id in ["BT27-008", "BT27-009", "BT27-013", "TEXT-4"] {
        r.inject_trash(0, id);
    }
    action(&mut r, 0, PLAY_HAND_START);
    for expected in ["BT27-013", "BT27-009", "BT27-008"] {
        assert_eq!(candidates(&r), [expected]);
        r.choose_prompt_card(expected, None).unwrap();
    }
    assert_eq!(r.memory(), 3);
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-4"]);
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), [CARD]);
    r.choose_prompt_card(CARD, None).unwrap();
    assert!(r.pending_selection().is_none());
}

#[test]
fn on_play_uses_the_chosen_digimons_dp_and_does_not_trash_security_outside_combat() {
    use digimon_engine::enums::Keyword;
    let mut r = start(builder(CARD).hand(0, &[CARD]).memory(10));
    r.place_on_field(0, "TEXT-4", Some(0));
    r.place_on_field(1, "TEXT-4", Some(0));
    r.place_on_field(1, "TEXT-5", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), [CARD, "TEXT-4"]);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-4"]);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    assert_eq!(r.security_count(1), 5);
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-4"]);
    let ouryumon = digimon_engine::permanent::PermanentHandle { player: 0, index: 1 };
    for keyword in [Keyword::Piercing, Keyword::Reboot, Keyword::Blocker] {
        assert!(r.game.has_keyword(ouryumon, keyword));
    }
}

#[test]
fn digivolving_during_an_attack_deletes_a_digimon_and_trashes_an_extra_security_card() {
    use digimon_engine::action::space::encode_attack;

    let mut r = start(
        builder(CARD)
            .dsl_card("BT27-013")
            .unwrap()
            .hand(0, &[CARD]),
    );
    let attacker = r.place_on_field(0, "BT27-013", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    action(&mut r, 0, encode_attack(attacker.index.into(), 14));
    assert_eq!(r.security_count(1), 4);
    assert_eq!(candidates(&r), [CARD]);
    r.choose_prompt_card(CARD, None).unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), [CARD]);
    r.choose_prompt_card(CARD, None).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(r.security_count(1), 3);
    assert!(r.pending_selection().is_none());
}

#[test]
fn removing_security_can_play_a_chronicle_tamer_without_paying_or_decline() {
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::enums::CardKind;

    for pay in [false, true] {
        let mut support = make_test_card("SUPPORT", "Chronicle support");
        support.card_kind = CardKind::Tamer;
        support.play_cost = 6;
        support.traits = vec!["Chronicle".into()];
        let mut r = start(builder(CARD).add_card(support).hand(0, &["SUPPORT", "TEXT-TAMER"]));
        let attacker = r.place_on_field(0, CARD, Some(0));
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        r.assert_prompt(SelectionKind::Hand, true, 0);
        assert_eq!(candidates(&r), ["SUPPORT"]);
        if pay {
            r.choose_prompt_card("SUPPORT", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.memory(), 10);
        assert_eq!(r.battle_area_size(0), 1 + usize::from(pay));
        assert_eq!(r.hand_size(0), 2 - usize::from(pay));
        assert_eq!(r.security_count(1), 4);
    }
}

#[test]
fn checked_security_effect_resolves_before_both_players_security_removal_triggers() {
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::enums::CardKind;

    let mut support = make_test_card("SUPPORT", "Chronicle support");
    support.card_kind = CardKind::Tamer;
    support.traits = vec!["Chronicle".into()];
    let mut r = start(
        builder(CARD)
            .dsl_card("BT27-099")
            .unwrap()
            .add_card(support)
            .hand(0, &["SUPPORT"])
            .hand(1, &["SUPPORT"])
            .security(1, &["BT27-099"]),
    );
    let attacker = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "TEXT-3", Some(0));
    r.place_on_field(1, CARD, Some(0));
    action(&mut r, 0, encode_attack(attacker.index.into(), 14));
    r.assert_prompt(SelectionKind::OwnField, true, 1);
    r.decline_prompt().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 1);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();

    for player in [0, 1] {
        r.assert_prompt(SelectionKind::Hand, true, player);
        assert_eq!(candidates(&r), ["SUPPORT"]);
        r.decline_prompt().unwrap();
    }
    assert!(r.pending_selection().is_none());
}

#[test]
fn removing_security_can_use_a_chronicle_option_without_paying() {
    use digimon_engine::action::space::encode_attack;

    let mut payment = digimon("PAYMENT", 3, 2000, false);
    payment.traits = vec!["Chronicle".into()];
    let mut r = start(
        builder(CARD)
            .dsl_card("P-204")
            .unwrap()
            .add_card(payment)
            .hand(0, &["P-204", "PAYMENT"]),
    );
    let attacker = r.place_on_field(0, CARD, Some(0));
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(candidates(&r), ["P-204"]);
    r.choose_prompt_card("P-204", None).unwrap();
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(candidates(&r), ["PAYMENT"]);
    r.choose_prompt_card("PAYMENT", None).unwrap();
    assert_eq!(r.memory(), 10);
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(ids(&r, &r.game.players[0].trash), ["PAYMENT"]);
    assert_eq!(r.battle_area_size(0), 2);
    assert_eq!(
        ids(&r, &r.game.players[0].battle_area[1].card_sources),
        ["P-204"],
    );
}

#[test]
fn inherited_unsuspend_requires_a_suspended_alphamon_ouryuken() {
    use digimon_engine::action::space::encode_attack;
    for (matching_name, accept) in [(false, false), (true, false), (true, true)] {
        let mut host = digimon("HOST", 7, 16000, false);
        if matching_name {
            host.card_name = "Alphamon: Ouryuken".into();
        }
        let mut r = start(builder(CARD).add_card(host));
        let attacker = r.place_stack(0, &[CARD, "HOST"]);
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        if matching_name {
            assert!(r.pending_is_optional());
            if accept {
                r.accept_optional_trigger().unwrap();
            } else {
                r.decline_prompt().unwrap();
            }
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].battle_area[0].is_suspended, !accept);
        assert_eq!(r.security_count(1), 4);
    }
}
