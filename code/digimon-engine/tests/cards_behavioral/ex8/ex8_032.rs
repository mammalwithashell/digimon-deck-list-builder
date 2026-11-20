//! EX8-032 Apemon — Digimon, Yellow, Lv.4, cost 3, 4000 DP. Traits: Beastkin,
//! NSo. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX8-032.md)
//! [Digivolve] Yellow Lv.3: Cost 2
//! [Digivolve] Lv.3 w/[NSo] trait: Cost 2
//! Inherited Effect
//! [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP
//! for the turn.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Yellow/EX8_032.cs — alt digivolve:
//! top card level 3 AND EqualsTraits("NSo"), cost 2; OnAllyAttack inherited
//! OPT, NOT optional: a mandatory pick (canNoSelect false) of 1 opponent
//! Digimon -2000 DP until end of turn.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, GamePhase, PlaySource};

use super::nso_support::*;

const CARD_ID: &str = "EX8-032";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-032 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(nso("N3-PURPLE", 3, 3000, 3, CardColor::Purple))
        .add_card(plain("Y3", 3, 3000, CardColor::Yellow))
        .add_card(plain("X3-BLUE", 3, 3000, CardColor::Blue))
        .add_card(nso("N4-PURPLE", 4, 4000, 4, CardColor::Purple))
        .add_card(plain("HOST-PLAIN", 4, 5000, CardColor::Yellow))
        .add_card(plain("OPP-A", 4, 6000, CardColor::Blue))
        .add_card(plain("OPP-B", 4, 7000, CardColor::Blue))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(10)
}

/// Apemon in hand over `base` on P0's field (Main phase, turn 1).
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
fn ex8_032_digivolves_from_a_yellow_level_3_for_2() {
    let (mut r, slot) = digivolve_over("Y3");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 2);
}

#[test]
fn ex8_032_digivolves_from_any_colour_nso_level_3_for_2() {
    let (mut r, slot) = digivolve_over("N3-PURPLE");
    assert!(offered(&r, slot), "[Digivolve] Lv.3 w/[NSo] trait: Cost 2");
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 2);
}

#[test]
fn ex8_032_rejects_an_off_colour_level_3_without_the_nso_trait() {
    let (mut r, slot) = digivolve_over("X3-BLUE");
    assert!(!offered(&r, slot));
    let mem = r.memory();
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(r.memory(), mem);
}

#[test]
fn ex8_032_the_nso_route_is_level_3_only() {
    let (mut r, slot) = digivolve_over("N4-PURPLE");
    assert!(!offered(&r, slot), "a level 4 [NSo] is not a legal base");
    let _ = &mut r;
}

#[test]
fn ex8_032_compiles_inherited_once_per_turn_when_attacking() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when == vec![CompiledTiming::WhenAttacking]
                && t.once_per_turn
                && !t.optional)));
}

/// P0 stacks Apemon's card UNDER a host so the inherited effect is live.
fn attack_board() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = builder().start();
    let attacker = r.place_stack(0, &[CARD_ID, "HOST-PLAIN"]);
    r.place_on_field(1, "OPP-A", Some(0));
    r.place_on_field(1, "OPP-B", Some(0));
    (r, attacker)
}

#[test]
fn ex8_032_inherited_when_attacking_gives_an_opponent_digimon_minus_2000() {
    let (mut r, attacker) = attack_board();
    r.attack_player(attacker, 1, false);
    let view = r.pending_selection_view().expect("the -2000 DP pick");
    assert!(!view.is_optional, "the pick is mandatory (canNoSelect: false)");
    pick_field(&mut r, 1, "OPP-B");
    let b = handle_of(&r, 1, "OPP-B");
    let a = handle_of(&r, 1, "OPP-A");
    assert_eq!(r.effective_dp(b), Some(5000), "7000 - 2000");
    assert_eq!(r.effective_dp(a), Some(6000), "the other Digimon is untouched");
}

#[test]
fn ex8_032_the_dp_reduction_lasts_only_for_the_turn() {
    let (mut r, attacker) = attack_board();
    r.attack_player(attacker, 1, false);
    pick_field(&mut r, 1, "OPP-A");
    drain_declining(&mut r);
    let a = handle_of(&r, 1, "OPP-A");
    assert_eq!(r.effective_dp(a), Some(4000));
    r.end_turn();
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(handle_of(&r, 1, "OPP-A")), Some(6000), "restored next turn");
}
