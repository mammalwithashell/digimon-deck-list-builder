//! Source: `data/card_bundles/BT27-060.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Black Lv.3 / cost 2
//!   - Yellow Lv.3 / cost 2
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [Thundermon]: Cost 0
//!
//! ## Printed text (official)
//! ### Effect
//! [On Play] [When Digivolving] [On Deletion] Until your opponent's turn ends, 1 of your Digimon with [Mamemon] in its name gains ＜Blocker＞ and their effects can't return it to hands or decks. (Rule) Name: Treated as including [Mamemon].
//!
//! ### Inherited Effect
//! ＜Blocker＞ (This Digimon can block in the blocker timing.)
//!
//! ## Official Q&A
//! - No, it is not treated as [Mamemon]. The name of this card is [Thundermon (X Antibody)], but the (Rule) allows it to be referenced as a card that has [Mamemon] in its name.

const CARD: &str = "BT27-060";

#[test]
fn inherited_blocker_can_intercept_a_security_attack() {
    use digimon_engine::action::space::encode_attack;
    let mut r = start(builder(CARD));
    r.place_stack(0, &[CARD, "TEXT-6"]);
    let attacker = r.place_on_field(1, "TEXT-3", Some(0));
    next_turn(&mut r);
    action(&mut r, 1, encode_attack(attacker.index as u16, 14));
    assert_eq!(candidates(&r), ["TEXT-6"]);
    r.choose_prompt_card("TEXT-6", None).unwrap();
    assert_eq!(r.security_count(0), 5);
    assert_eq!(r.battle_area_size(1), 0);
}

use super::common::*;
use digimon_engine::{action::space::PLAY_HAND_START, enums::Keyword};

#[test]
fn on_play_can_choose_itself_by_its_mamemon_name_rule() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_on_field(0, "TEXT-3", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), [CARD]);
    r.choose_prompt_card(CARD, None).unwrap();
    assert!(r.game.has_keyword(
        digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 1
        },
        Keyword::Blocker
    ));
}

#[test]
fn digivolving_and_then_being_deleted_in_battle_can_protect_another_mamemon() {
    use digimon_engine::action::space::{encode_attack, encode_digivolve};
    use digimon_engine::selection::SelectionKind;

    let mut base = digimon("BASE", 4, 4000, false);
    base.card_name = "Thundermon".into();
    let mut mamemon = digimon("MAME", 5, 7000, false);
    mamemon.card_name = "Mamemon".into();
    let mut r = start(
        builder(CARD)
            .add_card(base)
            .add_card(mamemon)
            .add_card(digimon("FATAL", 7, 20000, false))
            .hand(0, &[CARD])
            .security(1, &["FATAL", "FATAL"]),
    );
    let base = r.place_on_field(0, "BASE", Some(0));
    r.place_on_field(0, "MAME", Some(0));
    action(&mut r, 0, encode_digivolve(0, base.index.into()));
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), [CARD, "MAME"]);
    r.choose_prompt_card(CARD, None).unwrap();
    assert_eq!(r.memory(), 10);
    action(&mut r, 0, encode_attack(base.index.into(), 14));
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), ["MAME"]);
    r.choose_prompt_card("MAME", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[0].trash), ["BASE", CARD]);
    assert!(r.game.has_keyword(r.perm_handle(0, 0), Keyword::Blocker));
}

#[test]
fn mamemon_protection_blocks_opponent_hand_and_deck_returns_but_expires() {
    for to_hand in [false, true] {
        let mut mamemon = digimon("MAME", 5, 7000, false);
        mamemon.card_name = "Mamemon".into();
        let mut r = start(builder(CARD).add_card(mamemon).hand(0, &[CARD]));
        let protected = r.place_on_field(0, "MAME", Some(0));
        let other = r.place_on_field(0, "TEXT-4", Some(0));
        action(&mut r, 0, PLAY_HAND_START);
        assert_eq!(candidates(&r), [CARD, "MAME"]);
        r.choose_prompt_card("MAME", None).unwrap();
        assert!(r.game.has_keyword(protected, Keyword::Blocker));
        assert!(!r.game.has_keyword(other, Keyword::Blocker));
        let attempt = |r: &mut digimon_engine::debug_runner::DebugRunner| {
            if to_hand {
                r.game.return_to_hand_from_effect(protected, 1)
            } else {
                r.game.return_to_deck_from_effect(protected, 1)
            }
        };
        assert!(!attempt(&mut r));
        next_turn(&mut r);
        assert!(!attempt(&mut r));
        next_turn(&mut r);
        assert!(!r.game.has_keyword(protected, Keyword::Blocker));
        assert!(attempt(&mut r));
    }
}
