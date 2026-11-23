//! Source: `data/card_bundles/BT27-043.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Yellow Lv.5 / cost 4
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [Kentaurosmon]: Cost 2
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Blocker＞ ＜Barrier＞ ＜Ascension＞ (When this Digimon is deleted, you may place this card as the top security card.) [When Digivolving] [When Attacking] [Once Per Turn] By trashing your top security card, this Digimon unsuspends. [All Turns] [Once Per Turn] When your security stack is removed from, by placing 1 [Kentaurosmon] or [X Antibody] from this Digimon's digivolution cards as the bottom security card, activate 1 [Main] effect on this card's Option side.
//!
//! ### DUAL Effect
//! [Main] 1 of your opponent's Digimon gets -20000 DP until their turn ends. For each card in both players' security stacks, add 1000 to this effect's DP reduction.
//!
//! ### DUAL Rule
//! ＜Arts Digivolve＞ (Instead of trashing after use, your cards may digivolve into this card without paying the cost.)
//!
//! ## Official Q&A
//! - A [Main] effect shown on the DUAL card's Option side will activate. Activated effects on the Option side are treated as Option card effects.

const CARD: &str = "BT27-043";

use super::common::*;
use digimon_engine::action::space::encode_attack;

#[test]
fn blocker_intercepts_barrier_can_save_it_and_ascension_can_move_it_to_security() {
    use digimon_engine::selection::SelectionKind;

    for (barrier, ascension) in [(true, false), (false, true), (false, false)] {
        let mut r = start(builder(CARD).add_card(digimon("GIANT", 7, 40000, false)));
        r.place_on_field(0, CARD, Some(0));
        let attacker = r.place_on_field(1, "GIANT", Some(0));
        next_turn(&mut r);
        action(&mut r, 1, encode_attack(attacker.index.into(), 14));
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), [CARD]);
        r.choose_prompt_card(CARD, None).unwrap();
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        if barrier {
            r.accept_optional_trigger().unwrap();
            assert_eq!(r.battle_area_size(0), 1);
            assert_eq!(r.security_count(0), 4);
        } else {
            r.decline_prompt().unwrap();
            r.assert_prompt(SelectionKind::EffectChoice, false, 0);
            r.execute_branch(usize::from(!ascension)).unwrap();
            assert_eq!(r.battle_area_size(0), 0);
            assert_eq!(r.security_count(0), 5 + usize::from(ascension));
            if ascension {
                assert_eq!(ids(&r, &r.game.players[0].security).last().unwrap(), CARD);
            } else {
                assert_eq!(ids(&r, &r.game.players[0].trash), [CARD]);
            }
        }
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn when_attacking_may_trash_own_security_to_unsuspend() {
    for pay in [false, true] {
        let mut r = start(builder(CARD));
        let attacker = r.place_on_field(0, CARD, Some(0));
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        if pay {
            r.accept_optional_trigger().unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.security_count(0), 5 - usize::from(pay));
        assert_eq!(r.game.players[0].battle_area[0].is_suspended, !pay);
    }
}

#[test]
fn security_payment_places_only_an_eligible_source_and_counts_both_security_stacks() {
    use digimon_engine::enums::{EffectSourceKind, Keyword};
    use digimon_engine::selection::SelectionKind;

    for pay_source in [false, true] {
        let mut alpha = digimon("X", 3, 1000, false);
        alpha.card_name = "X Antibody".into();
        let mut r = start(
            builder(CARD)
                .add_card(alpha)
                .add_card(digimon("GIANT", 7, 40000, false)),
        );
        let attacker = r.place_stack(0, &["TEXT-4", "X", CARD]);
        let victim = r.place_on_field(1, "GIANT", Some(0));
        for keyword in [Keyword::Blocker, Keyword::Barrier, Keyword::Ascension] {
            assert!(r.game.has_keyword(attacker, keyword));
        }
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        r.accept_optional_trigger().unwrap();
        assert_eq!(candidates(&r), ["X"]);
        if pay_source {
            r.choose_prompt_card("X", None).unwrap();
            r.assert_prompt(SelectionKind::OppField, false, 0);
            assert_eq!(r.pending_selection_view().unwrap().source_kind, EffectSourceKind::Option);
            r.choose_prompt_card("GIANT", None).unwrap();
            assert_eq!(r.effective_dp(victim), Some(10000));
            assert_eq!(r.game.players[0].security[0].card_id(&r.game.card_data), "X");
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.effective_dp(victim), Some(40000));
            assert_eq!(r.security_count(0), 4);
        }
    }
}

#[test]
fn option_can_arts_digivolve_into_kentaurosmon_after_its_main_effect() {
    use digimon_engine::action::space::PLAY_HAND_START;
    use digimon_engine::selection::SelectionKind;

    for accept in [false, true] {
        let mut kentaurosmon = digimon("KENTAURO", 6, 12000, false);
        kentaurosmon.card_name = "Kentaurosmon".into();
        let mut r = start(builder(CARD).add_card(kentaurosmon).hand(0, &[CARD]));
        let base = r.place_on_field(0, "KENTAURO", Some(0));
        r.place_on_field(1, "TEXT-3", Some(0));
        action(&mut r, 0, PLAY_HAND_START);
        r.choose_prompt_card("TEXT-3", None).unwrap();
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), ["KENTAURO"]);
        if accept {
            action(&mut r, 0, encode_attack(0, base.index as u16));
            r.decline_prompt().unwrap();
            assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), ["KENTAURO", CARD]);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(ids(&r, &r.game.players[0].trash), [CARD]);
        }
        assert_eq!(r.memory(), 5);
    }
}
