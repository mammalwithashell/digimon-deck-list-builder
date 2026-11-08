//! ST21-12 Joe Kido & Mimi Tachikawa — Tamer, Blue+Green, Cost 3,
//! traits: [ADVENTURE].
//!
//! # Card text (official bundle `data/card_bundles/ST21-12.md`)
//!
//! ```text
//! [Start of Your Main Phase] If you have a Digimon with the [ADVENTURE] trait,
//! gain 1 memory.
//! [Your Turn] When any Digimon cards with the [ADVENTURE] trait would be played
//! from your hand, by suspending this Tamer, reduce the play cost by 1.
//! Security Effect: [Security] Play this card without paying the cost.
//! ```
//!
//! Same printed text as ST20-12; DCGO `ST21/Blue/ST21_12.cs` is a line-for-line
//! copy of `ST20_12.cs`. The behavioural suite lives in `st20::st20_12`.

use digimon_dsl::compiled::CompiledColor;

use crate::st20::st20_12 as suite;

const CARD_ID: &str = "ST21-12";

#[test]
fn st21_12_identity() {
    suite::assert_adventure_tamer_identity(CARD_ID, [CompiledColor::Blue, CompiledColor::Green]);
}

#[test]
fn st21_12_start_of_main_gains_one_with_adventure_digimon() {
    suite::start_of_main_gains_one_with_adventure_digimon(CARD_ID);
}

#[test]
fn st21_12_reduction_accept_pays_one_less_and_suspends() {
    suite::reduction_accept_pays_one_less_and_suspends(CARD_ID);
}

#[test]
fn st21_12_reduction_decline_pays_full() {
    suite::reduction_decline_pays_full(CARD_ID);
}

#[test]
fn st21_12_reduction_only_for_adventure_digimon() {
    suite::reduction_only_for_adventure_digimon(CARD_ID);
}

#[test]
fn st21_12_reduction_not_offered_when_suspended() {
    suite::reduction_not_offered_when_suspended(CARD_ID);
}

#[test]
fn st21_12_reduction_not_on_opponents_turn() {
    suite::reduction_not_on_opponents_turn(CARD_ID);
}

/// Q&A (bundle): ST21-12 + ST20-12 reduce by a total of 2.
#[test]
fn st21_12_reduction_stacks_with_st20_12_qa() {
    suite::reduction_stacks_across_tamers(CARD_ID, "ST20-12");
}

#[test]
fn st21_12_reduction_each_tamer_is_its_own_choice() {
    suite::reduction_each_tamer_is_its_own_choice(CARD_ID, "ST20-12");
}

#[test]
fn st21_12_reduction_not_on_digivolve() {
    suite::reduction_not_on_digivolve(CARD_ID);
}

#[test]
fn st21_12_security_plays_self_free() {
    suite::security_plays_self_free(CARD_ID);
}

#[test]
fn st21_12_reduction_not_on_cost_paying_trash_play() {
    suite::reduction_not_on_cost_paying_trash_play(CARD_ID);
}

#[test]
fn st21_12_reduction_not_on_adventure_option() {
    suite::reduction_not_on_adventure_option(CARD_ID);
}
