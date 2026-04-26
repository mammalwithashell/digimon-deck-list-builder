//! BT17-015 WarGreymon — sample behavioral test template.
//!
//! This file is the **template** the forthcoming `/batch-implement-cards-rust`
//! skill follows. Every hand-written Rust card effect under
//! `src/cards/<set>/<card_id>.rs` lands with a paired test file here that
//! covers the positive and negative branches of each conditional clause.
//!
//! TDD walkthrough (CLAUDE.md §18):
//!
//!   1. Read the card text verbatim. Note every conditional, optional
//!      branch, and numeric value.
//!   2. Write the failing behavioral tests in this file FIRST. One
//!      `#[test]` per distinct observable outcome — don't lump positive
//!      and negative cases into a single test.
//!   3. Implement the `CardEffect` struct in
//!      `src/cards/<set>/<card_id>.rs` and register it in the set's
//!      `mod.rs`.
//!   4. Remove the `#[ignore]` attributes.
//!   5. Run `cargo test --test cards_behavioral` — all tests must pass.
//!   6. Run `cargo test` — full suite must stay green. No existing test
//!      regresses.
//!
//! ---
//!
//! # Card text — BT17-015 WarGreymon
//!
//! > **[When Attacking]** If you have another Digimon in play with the
//! > \<Metal Empire\> or \<Virus Busters\> trait, this Digimon gets +2000
//! > DP for the turn.
//!
//! Engine primitives this card depends on (all currently gaps):
//!   - `WhenAttacking` timing firing (Tier-2 observer plumbing)
//!   - Trait-filter helper on `Permanent` / `CardSource`
//!   - Conditional DP modifier via `add_dp_modifier(_, _, Expiry::EndOfAttack)`
//!
//! Until the gaps land, both tests are `#[ignore]`d with the reason
//! below so `cargo test --test cards_behavioral` stays green while this
//! file documents the expected behavior. Run `cargo test --test
//! cards_behavioral -- --ignored` to see the failures once the gaps
//! close.

#![allow(dead_code, unused_imports)]

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

/// Happy path: WarGreymon declares an attack with a trait-matching ally
/// on the field. The +2000 DP buff must be live for the duration of the
/// attack and gone afterwards.
#[test]
#[ignore = "pending gaps: WhenAttacking timing firing + trait filter helper"]
fn bt17_015_gets_dp_boost_with_trait_ally() {
    // Test scaffolding will look like:
    //
    //   let mut runner = DebugRunner::new()
    //       .with_card(make_test_card("BT17-015", /* dp = */ 11000, /* level = */ 6))
    //       .with_card(make_test_card("TEST-METAL-EMPIRE-ALLY", /* dp = */ 5000, /* level = */ 4))
    //       .build();
    //   runner.play_to_field("BT17-015", P0);
    //   runner.play_to_field("TEST-METAL-EMPIRE-ALLY", P0);
    //   runner.declare_attack_on_player("BT17-015");
    //
    //   // During the attack resolution the tensor-readable DP is 11000 + 2000.
    //   assert_eq!(runner.effective_dp("BT17-015"), 13000);
    //
    //   // After the attack resolves the buff is gone.
    //   runner.resolve_pending_attack();
    //   assert_eq!(runner.effective_dp("BT17-015"), 11000);

    // TDD stub — will panic once the ignore attribute is removed and the
    // engine gap is closed.
    todo!("write scaffolding once WhenAttacking + trait filter land");
}

/// No-approximations companion: the conditional clause must NOT fire
/// when no matching ally is in play. This locks in the "don't
/// auto-activate" contract (CLAUDE.md §17) — every conditional card
/// needs both branches covered.
#[test]
#[ignore = "pending gaps: WhenAttacking timing firing + trait filter helper"]
fn bt17_015_no_boost_without_trait_ally() {
    // Test scaffolding will look like:
    //
    //   let mut runner = DebugRunner::new()
    //       .with_card(make_test_card("BT17-015", 11000, 6))
    //       // no trait-matching ally on the field
    //       .build();
    //   runner.play_to_field("BT17-015", P0);
    //   runner.declare_attack_on_player("BT17-015");
    //
    //   // No ally → no buff, DP stays at print.
    //   assert_eq!(runner.effective_dp("BT17-015"), 11000);

    todo!("write scaffolding once WhenAttacking + trait filter land");
}
