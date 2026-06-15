//! BT13-075 Alphamon — Digimon, Lv.6, Black, DP 12000, Cost 12.
//! Traits: Holy Warrior / Royal Knight / X Antibody. Form: Mega. Vaccine.
//!
//! # Card text (cards.json / card image)
//!
//! ```text
//! [On Play] [When Digivolving] By placing 1 Digimon card with the [X Antibody]
//!   trait or the [Royal Knight] trait from your trash under this Digimon as
//!   its bottom digivolution card, all of your opponent's Digimon with a play
//!   cost of 10 or higher can't attack players until the end of your
//!   opponent's turn.
//! [All Turns] [Once Per Turn] When this Digimon would leave the battle area by
//!   an effect, by returning 1 Digimon card with the [X Antibody] trait or the
//!   [Royal Knight] trait from this Digimon's digivolution cards to the bottom
//!   of your deck, prevent it from leaving play.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Black/BT13_075.cs
//!
//! # Gap-routed (NEITHER clause implemented)
//! - [On Play][When Digivolving] place an [X Antibody]/[Royal Knight] source
//!   from trash, then opponent play-cost-10+ Digimon can't attack players —
//!   G-PLAY-COST-GTE-MODIFIER-AURA (a play-cost-threshold can't-attack-players
//!   aura keyed off a source-placement cost).
//! - [All Turns][OPT] would-leave self-protection: the printed COST is
//!   "by returning 1 [X Antibody]/[Royal Knight] card from this Digimon's
//!   digivolution cards to the BOTTOM OF YOUR DECK". The `kind: replacement`
//!   framework + `select_own_sources` exist, but there is NO DSL verb that
//!   returns a selected digivolution source to the deck bottom: the only
//!   source-return verb is `return_selected_sources_to_hand` (to hand). This
//!   is a NEW substrate gap — G-RETURN-SELECTED-SOURCE-TO-DECK-BOTTOM (sibling
//!   of `return_selected_sources_to_hand`). Until it lands, the replacement
//!   stays stubbed (returning a source to hand instead of deck would be an
//!   approximation and is disallowed).

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt13_075_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT13-075")
        .expect("load")
        .start();
}

#[ignore = "pending: G-PLAY-COST-GTE-MODIFIER-AURA — after placing a source, opponent play-cost 10+ Digimon cannot attack players"]
#[test]
fn bt13_075_places_source_then_blocks_high_play_cost_attacks() {}

#[ignore = "pending: G-RETURN-SELECTED-SOURCE-TO-DECK-BOTTOM — would-leave self-protection cost returns 1 [X Antibody]/[Royal Knight] digivolution source to the BOTTOM OF YOUR DECK; only `return_selected_sources_to_hand` exists today; tracker: qa/dsl-vocab-gaps.md"]
#[test]
fn bt13_075_would_leave_returns_source_to_deck_bottom_and_prevents() {}
