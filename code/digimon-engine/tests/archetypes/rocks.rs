//! Rocks — archetype interaction tests (exemplar).
//!
//! Model: `qa/archetype-qa/Rocks-model.md` (the digivolution-into-removal
//! payoff line). These exemplar tests pin the multi-card interaction pattern
//! and prove the `support.rs` fixtures; the `/archetype-interaction-test-author`
//! skill extends this file with the rest of the archetype's combos.
//!
//! # Combo under test — "Greymon removal, Koromon-enabled"
//!
//! BT17-102 Greymon, `[When Digivolving]`: *"If this Digimon has [Koromon] in
//! its digivolution cards, it gains +3000 DP for the turn. Then, delete 1 of
//! your opponent's Digimon with as much or less DP as this Digimon."*
//!
//! This is a genuine **three-card interaction**: the enabler (Koromon in the
//! stack) raises Greymon's effective DP from its base 5000 to 8000, which
//! widens the removal's DP window — so a 6000-DP opponent Digimon is deletable
//! *only when the enabler is present*. The payoff (Greymon) and the target
//! (the opponent Digimon) are the other two cards.
//!
//! - Card text: cards.json BT17-102.
//! - DCGO C# reference: `$BASE_DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_102.cs`.
//! - Rules basis: deletion semantics + DP comparison (`general_rule.pdf` §11 DP,
//!   §6-2-x deletion). The `[When Digivolving]` timing is the keyword window.
//!
//! The per-card behavioral coverage lives in
//! `tests/cards_behavioral/bt17/bt17_102.rs`; this file asserts the combo as a
//! *system* (enabler ⇒ flipped deletability), which no per-card test sees.

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use super::support::snapshot;

// ─── Combo-piece fixtures ────────────────────────────────────────────────────

/// Koromon — the digivolution-stack enabler whose presence grants Greymon the
/// +3000 buff (Greymon's clause condition is `self_digivolution_contains_name:
/// "Koromon"`).
fn make_koromon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Koromon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(1000);
    c
}

/// A Lv.3 non-Koromon stack base — same legal stack position, but it does NOT
/// satisfy Greymon's enabler condition.
fn make_lv3_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// An opponent Digimon target with a chosen DP.
fn make_opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// Fire Greymon's `[When Digivolving]` timing for `handle` (direct enqueue +
/// drain — the proven harness pattern; `place_stack` builds the stack without
/// firing the trigger, so we fire it explicitly to model the digivolve).
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

// ─── Combo: Greymon removal, Koromon-enabled ─────────────────────────────────

/// Happy path: with Koromon in the stack, Greymon's effective DP is 8000, so
/// its `[When Digivolving]` removal deletes the 6000-DP opponent Digimon while
/// the 12000-DP one survives the DP filter.
#[test]
fn greymon_removal_with_koromon_deletes_mid_dp_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 (Greymon) in embedded DSL pack")
        .add_card(make_koromon("KORO"))
        .add_card(make_opp_digimon("OPP-MID", "OppMid", 6000))
        .add_card(make_opp_digimon("OPP-HIGH", "OppHigh", 12000))
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "OPP-MID", None);
    runner.place_on_field(1, "OPP-HIGH", None);
    // Stack [Koromon, Greymon] — the enabler sits under the payoff.
    let stack = runner.place_stack(0, &["KORO", "BT17-102"]);

    let before = snapshot(&runner);
    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();
    let after = snapshot(&runner);

    // Exactly one opponent Digimon (the 6000-DP one) is removed: opp field −1,
    // opp trash +1.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "exactly one opponent Digimon should be deleted by the Koromon-enabled removal"
    );
    assert_eq!(
        after.trash[1],
        before.trash[1] + 1,
        "the deleted Digimon should land in the opponent's trash"
    );
    // The 12000-DP target is above Greymon's 8000 effective DP — it survives.
    let surviving: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_name(&runner.game.card_data).to_string())
        .collect();
    assert!(
        surviving.iter().any(|n| n == "OppHigh"),
        "the 12000-DP Digimon is outside the DP window and must survive; survivors={surviving:?}"
    );
}

/// Enabler-absent path: the SAME board without Koromon leaves Greymon at its
/// base 5000 DP, so the 6000-DP target is outside the removal's DP window and
/// survives. The combo's removal is gated on the enabler — exactly the
/// system-level fact a per-card test can't express.
#[test]
fn greymon_removal_without_koromon_spares_mid_dp_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 (Greymon) in embedded DSL pack")
        .add_card(make_lv3_filler("LV3-FILLER"))
        .add_card(make_opp_digimon("OPP-MID", "OppMid", 6000))
        .add_card(make_opp_digimon("OPP-HIGH", "OppHigh", 12000))
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "OPP-MID", None);
    runner.place_on_field(1, "OPP-HIGH", None);
    // Stack [Lv3 filler, Greymon] — no Koromon, so no +3000 buff.
    let stack = runner.place_stack(0, &["LV3-FILLER", "BT17-102"]);

    let before = snapshot(&runner);
    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();
    let after = snapshot(&runner);

    // Greymon is at base 5000; neither the 6000- nor the 12000-DP Digimon is a
    // legal target, so the opponent's board is untouched.
    assert_eq!(
        after.field[1], before.field[1],
        "with no enabler, Greymon's 5000 DP window deletes nothing — opp field unchanged"
    );
    assert_eq!(
        after.trash[1], before.trash[1],
        "no opponent Digimon should be trashed without the Koromon enabler"
    );
}
