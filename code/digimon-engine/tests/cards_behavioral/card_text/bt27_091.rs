//! Source: `data/card_bundles/BT27-091.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! ## Printed text (official)
//! ### Effect
//! [Start of Your Main Phase] By placing 1 [BEATBREAK] trait card from your hand face down under this Tamer, ＜Draw 1＞ and gain 1 memory. [All Turns] When any Digimon suspend, by suspending this Tamer, you may place the top card of your deck face down under this Tamer.
//!
//! ### Security Effect
//! [Security] Play this card without paying the cost.
//!
//! ## Official Q&A
//! - The card is placed on the bottom of the cards under the Tamer.

use super::common::{action, builder, ids, start};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn tomoro_stashes_the_decks_top_card_when_a_digimon_suspends() {
    let mut r = start(builder("BT27-091"));
    let tamer = r.place_on_field(0, "BT27-091", Some(0));
    let attacker = r.place_on_field(0, "TEXT-3", Some(0));
    let deck = r.game.players[0].deck.len();
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    r.execute_branch(0).unwrap();
    let permanent = &r.game.players[0].battle_area[tamer.index as usize];
    assert!(permanent.is_suspended);
    assert_eq!(
        ids(&r, &permanent.card_sources),
        ["TEXT-FILLER", "BT27-091"]
    );
    assert!(permanent.card_sources[0].face_down);
    assert_eq!(r.game.players[0].deck.len(), deck - 1);
}

#[test]
fn main_phase_buries_a_beatbreak_card_under_existing_sources_then_draws_and_gains_memory() {
    use digimon_engine::debug_runner::DebugRunner;
    for accept in [false, true] {
        let mut r: DebugRunner = builder("BT27-091")
            .memory(4)
            .hand(0, &["TEXT-3", "TEXT-OTHER"])
            .start();
        let t = super::common::stocked_tamer(&mut r, "BT27-091");
        let deck = r.game.players[0].deck.len();
        let memory = r.memory();
        r.game.enter_main_phase();
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        if accept {
            r.accept_optional_trigger().unwrap();
            r.assert_prompt(SelectionKind::Hand, false, 0);
            assert_eq!(super::common::candidates(&r), ["TEXT-3"]);
            r.choose_prompt_card("TEXT-3", None).unwrap();
        } else {
            r.decline_optional_trigger().unwrap();
        }
        let stack = &r.game.players[0].battle_area[t.index as usize].card_sources;
        assert_eq!(stack.len(), 3 + usize::from(accept));
        if accept {
            assert!(stack[0].face_down);
            assert_eq!(
                ids(&r, stack),
                ["TEXT-3", "TEXT-BOTTOM", "TEXT-TOP", "BT27-091"]
            );
        }
        assert_eq!(r.game.players[0].deck.len(), deck - usize::from(accept));
        assert_eq!(r.memory(), memory + i16::from(accept));
    }
}

#[test]
fn security_plays_the_tamer_without_cost() {
    super::common::security_tamer_play("BT27-091");
}
