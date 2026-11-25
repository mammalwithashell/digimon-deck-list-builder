//! Source: `data/card_bundles/BT27-008.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Red Lv.2 / cost 1
//!   - Black Lv.2 / cost 1
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [Kyokyomon]/Black Lv.2 w/[X Antibody] trait: Cost 0
//!
//! ## Printed text (official)
//! ### Effect
//! [Start of Your Main Phase] By trashing 1 card with the [Chronicle] trait from your hand, ＜Draw 1＞ and gain 1 memory.
//!
//! ### Inherited Effect
//! ＜Raid＞ (When this Digimon attacks, you may change the attack target to 1 of your opponent's unsuspended Digimon with the highest DP.)

const CARD: &str = "BT27-008";

use super::common::*;
use digimon_engine::{
    action::space::{encode_attack, PASS},
    enums::GamePhase,
    selection::SelectionKind,
};

#[test]
fn start_of_main_only_offers_chronicle_and_draws_and_gains_memory_after_payment() {
    for pay in [false, true] {
        let mut chronicle = digimon("CHRONICLE", 3, 2000, false);
        chronicle.traits = vec!["Chronicle".into()];
        let mut r = builder(CARD)
            .add_card(chronicle)
            .hand(0, &["CHRONICLE", "TEXT-3"])
            .memory(5)
            .start();
        r.place_on_field(0, CARD, Some(0));
        r.set_phase(GamePhase::Breeding);
        let memory = r.memory();
        let hand = r.hand_size(0);
        assert_eq!(r.current_phase(), GamePhase::Breeding);
        action(&mut r, 0, PASS);
        r.assert_prompt(SelectionKind::Hand, true, 0);
        if pay {
            assert_eq!(candidates(&r), ["CHRONICLE"]);
            r.choose_prompt_card("CHRONICLE", None).unwrap();
            assert_eq!(ids(&r, &r.game.players[0].trash), ["CHRONICLE"]);
        } else {
            r.decline_prompt().unwrap();
            assert!(r.game.players[0].trash.is_empty());
        }
        assert_eq!(r.hand_size(0), hand);
        assert_eq!(r.memory(), memory + i16::from(pay));
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn inherited_raid_only_offers_the_highest_dp_unsuspended_digimon() {
    let mut r = start(builder(CARD));
    let attacker = r.place_stack(0, &[CARD, "TEXT-6"]);
    r.place_on_field(1, "TEXT-4", Some(0));
    r.place_on_field(1, "TEXT-5", Some(0));
    let suspended = r.place_on_field(1, "TEXT-6", Some(0));
    r.game.players[1].battle_area[suspended.index as usize].is_suspended = true;
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    assert_eq!(candidates(&r), ["TEXT-5"]);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert_eq!(r.battle_area_size(1), 2);
    assert_eq!(r.security_count(1), 5);
}
