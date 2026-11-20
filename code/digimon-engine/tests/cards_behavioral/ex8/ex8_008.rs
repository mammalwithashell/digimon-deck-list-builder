//! EX8-008 Candlemon — Digimon, Red, Lv.3, cost 3, 2000 DP. Traits: Flame, NSo.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX8-008.md)
//! [Digivolve] Red Lv.2: Cost 0
//! [Digivolve] Lv.2 w/[NSo] trait: Cost 0
//! [On Deletion] Gain 1 memory.
//! Inherited Effect
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Red/EX8_008.cs — alt digivolve (level 2 +
//! EqualsTraits("NSo"), cost 0), OnDestroyedAnyone AddMemory(1) (also gated on
//! `CanAddMemory`), inherited ChangeSelfDPStaticEffect(+2000) on the owner's turn.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, GamePhase, PlaySource};
use digimon_engine::replacement::ReplacementCause;

use super::nso_support::*;

const CARD_ID: &str = "EX8-008";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-008 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(nso("N2-BLUE", 2, 0, 0, CardColor::Blue))
        .add_card(plain("X2-RED", 2, 0, CardColor::Red))
        .add_card(plain("X2-BLUE", 2, 0, CardColor::Blue))
        .add_card(plain("HOST-L4", 4, 4000, CardColor::Purple))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
}

fn digivolve_over(base: &str) -> (DebugRunner, usize) {
    let mut r = builder().hand(0, &[CARD_ID]).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, base, Some(0)).index as usize;
    (r, slot)
}

fn offered(r: &DebugRunner, slot: usize) -> bool {
    build_action_mask(&r.game, 0)[encode_digivolve(0, slot as u16) as usize] > 0.5
}

#[test]
fn ex8_008_digivolves_from_a_red_or_nso_level_2_for_free() {
    for base in ["X2-RED", "N2-BLUE"] {
        let (mut r, slot) = digivolve_over(base);
        assert!(offered(&r, slot), "{base}");
        let mem = r.memory();
        assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
        assert_eq!(mem - r.memory(), 0, "{base}");
    }
}

#[test]
fn ex8_008_rejects_an_off_colour_level_2_without_the_nso_trait() {
    let (r, slot) = digivolve_over("X2-BLUE");
    assert!(!offered(&r, slot));
}

#[test]
fn ex8_008_on_deletion_gains_1_memory() {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let mem = r.memory();
    r.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    drain_declining(&mut r);
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn ex8_008_inherited_gives_plus_2000_dp_on_your_turn_only() {
    let mut r = builder().start();
    let host = r.place_stack(0, &[CARD_ID, "HOST-L4"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.game.turn_player(), 0);
    assert_eq!(r.effective_dp(host), Some(6000), "4000 + 2000 on your turn");
    r.end_turn();
    r.game.tick_declarative_effects();
    let host = handle_of(&r, 0, "HOST-L4");
    assert_eq!(r.effective_dp(host), Some(4000), "no bonus on the opponent's turn");
}

#[test]
fn ex8_008_the_dp_bonus_is_inherited_only() {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(
        r.effective_dp(me),
        Some(2000),
        "an inherited effect does not apply to the card as the top card"
    );
}
