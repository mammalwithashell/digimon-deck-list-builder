//! Source: `data/card_bundles/ST23-03.md` (printed text and official Q&A).
//!
//! Standard digivolve cost circles:
//! Yellow Lv.3 / cost 2
//! Special digivolution condition:
//! [Digivolve] Lv.3 w/[Glowing Dawn] trait: Cost 2
//! Effect:
//! [On Play] [When Digivolving] Add your top security card to the hand. Then, ＜Recovery +1＞. (Place the top card of your deck as your top security card.) [Your Turn] When this Digimon would digivolve into a [Glowing Dawn] trait Digimon card, by trashing the bottom face-down card from under any of your Tamers, reduce the cost by 2.
//! Inherited Effect:
//! ＜Barrier＞ (When this Digimon would be deleted in battle, by trashing your top security card, it isn't deleted.)
//! Official Q&A:
//! Yes, you can. In such cases, you can't add a card from your security stack to your hand because you don't have any security cards, therefore you just perform .

use super::common::*;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PLAY_HAND_START};
use digimon_engine::selection::SelectionKind;

const CARD: &str = "ST23-03";

#[test]
fn on_play_and_when_digivolving_add_top_security_then_recover_even_with_empty_security() {
    for evolve in [false, true] {
        for empty in [false, true] {
            let security: &[&str] = if empty {
                &[]
            } else {
                &["TEXT-BOTTOM", "TEXT-TOP"]
            };
            let mut r = start(builder(CARD).hand(0, &[CARD]).security(0, security));
            let old_top = r.game.players[0].security.last().map(|c| c.handle());
            let deck = &r.game.players[0].deck;
            let recovered_card = deck[deck.len() - 1 - usize::from(evolve)].handle();
            if evolve {
                let base = r.place_on_field(0, "TEXT-3", Some(0));
                action(&mut r, 0, encode_digivolve(0, base.index as u16));
            } else {
                action(&mut r, 0, PLAY_HAND_START);
            }
            assert!(r.pending_selection().is_none());
            assert_eq!(r.security_count(0), if empty { 1 } else { 2 });
            if let Some(top) = old_top {
                assert!(r.game.players[0].hand.iter().any(|c| c.handle() == top));
            }
            let recovered = r.game.players[0].security.last().unwrap().handle();
            assert_eq!(recovered, recovered_card);
        }
    }
}

#[test]
fn your_turn_digivolve_glowing_dawn_reduces_cost_by_two_only_if_paid() {
    for (stocked, accept, target, paid) in [
        (true, true, "TEXT-5", 2),
        (true, false, "TEXT-5", 4),
        (false, false, "TEXT-5", 4),
        (true, false, "TEXT-OTHER", 4),
    ] {
        let mut r = start(builder(CARD).hand(0, &[target]));
        let base = r.place_on_field(0, CARD, Some(0));
        if stocked {
            stocked_tamer(&mut r, "TEXT-TAMER");
        }
        action(&mut r, 0, encode_digivolve(0, base.index as u16));
        if stocked && target == "TEXT-5" {
            r.assert_prompt(SelectionKind::EffectChoice, true, 0);
            if accept {
                r.execute_branch(0).unwrap();
                r.assert_prompt(SelectionKind::OwnField, false, 0);
                assert_eq!(candidates(&r), ["TEXT-TAMER"]);
                r.choose_prompt_card("TEXT-TAMER", None).unwrap();
            } else {
                r.decline_prompt().unwrap();
            }
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(10 - r.memory(), paid);
        assert_eq!(
            ids(&r, &r.game.players[0].trash),
            if accept { vec!["TEXT-BOTTOM"] } else { vec![] }
        );
    }
}

#[test]
fn inherited_barrier_may_trash_security_to_survive_battle() {
    for accept in [false, true] {
        let mut r = start(builder(CARD));
        let defender = r.place_stack(1, &[CARD, "TEXT-4"]);
        r.game.players[1].battle_area[defender.index as usize].is_suspended = true;
        let attacker = r.place_on_field(0, "TEXT-5", Some(0));
        action(
            &mut r,
            0,
            encode_attack(attacker.index as u16, defender.index as u16),
        );
        r.assert_prompt(SelectionKind::Replacement, true, 1);
        if accept {
            r.accept_optional_trigger().unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[1].battle_area.len(), usize::from(accept));
        assert_eq!(r.security_count(1), if accept { 4 } else { 5 });
    }
}
