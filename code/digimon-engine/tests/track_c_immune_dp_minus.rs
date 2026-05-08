//! Track C / D consult-site test for `ImmuneFromDPMinus` (2026-05-08).
//!
//! Pinned at the canonical DP calc — `Game::effective_dp`. When the
//! target carries `ImmuneFromDPMinus`, negative `ChangeDp` modifiers
//! are filtered out before the sum; positive `ChangeDp` and the
//! dynamic-aura bonus path remain untouched. Distinct from
//! `CannotBeAffected`, which suppresses the entire mutation across
//! the broader effect surface.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;

fn lv4_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

#[test]
fn baseline_negative_change_dp_lowers_effective_dp() {
    // Sanity baseline so the next test's delta is attributable.
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("VICTIM", 5000))
        .start();
    let h = r.place_on_field(0, "VICTIM", Some(0));

    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeDp, -2000, Expiry::Permanent, 1),
    );

    assert_eq!(r.game.effective_dp(h), Some(3000));
}

#[test]
fn immune_from_dp_minus_filters_negative_change_dp() {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("VICTIM", 5000))
        .start();
    let h = r.place_on_field(0, "VICTIM", Some(0));

    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeDp, -2000, Expiry::Permanent, 1),
    );
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ImmuneFromDPMinus, 0, Expiry::Permanent, 0),
    );

    assert_eq!(
        r.game.effective_dp(h),
        Some(5000),
        "ImmuneFromDPMinus must filter the -2000 ChangeDp entry"
    );
}

#[test]
fn immune_from_dp_minus_keeps_positive_change_dp() {
    // Narrow protection: only DP-minus is filtered. Positive ChangeDp
    // entries continue to apply normally.
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("VICTIM", 5000))
        .start();
    let h = r.place_on_field(0, "VICTIM", Some(0));

    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeDp, 3000, Expiry::Permanent, 0),
    );
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeDp, -1500, Expiry::Permanent, 1),
    );
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ImmuneFromDPMinus, 0, Expiry::Permanent, 0),
    );

    assert_eq!(
        r.game.effective_dp(h),
        Some(8000),
        "positive +3000 must still apply; negative -1500 must be filtered"
    );
}

#[test]
fn immune_from_dp_minus_does_not_affect_other_permanents() {
    // The modifier is permanent-scoped; an `ImmuneFromDPMinus` on one
    // permanent must not protect another permanent on the same field.
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("PROTECTED", 5000))
        .add_card(lv4_digimon("OTHER", 5000))
        .start();
    let protected = r.place_on_field(0, "PROTECTED", Some(0));
    let other = r.place_on_field(0, "OTHER", Some(0));

    r.game.modifiers.add(
        protected,
        ModifierEntry::simple(ModifierType::ImmuneFromDPMinus, 0, Expiry::Permanent, 0),
    );
    r.game.modifiers.add(
        other,
        ModifierEntry::simple(ModifierType::ChangeDp, -3000, Expiry::Permanent, 1),
    );

    assert_eq!(r.game.effective_dp(other), Some(2000));
    assert_eq!(r.game.effective_dp(protected), Some(5000));
}
