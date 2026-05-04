//! BT23-018 Garurumon — Digimon, Lv.4, Blue, DP 5000, Cost 6.
//!
//! # Card text (cards.json)
//!
//! ＜Jamming＞ (This Digimon can't be deleted in battles against Security
//! Digimon.)
//! [Main] [Once Per Turn] By placing this Digimon's top stacked card as its
//! bottom digivolution card, you may play 1 [Agumon] or [Nokia Shiramine]
//! from your hand with the play cost reduced by 2.
//!
//! Inherited:
//! [Opponent's Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Blue/BT23_018.cs
//!
//! # Sister-card relationship
//! BT23-018 is the structural sister of BT23-008 (Greymon, Red). The same
//! [Main][OPT] cost-as-source-shuffle clause is BLOCKED on the same
//! G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM gap. This file mirrors the BT23-008 test
//! suite with the appropriate deltas: <Jamming> instead of <Raid>, the alt
//! lower-form is [Gabumon] (Garurumon's natural pre-evo) instead of [Agumon],
//! the inherited DP gate is [Opponent's Turn] instead of [Your Turn], and the
//! hand pick-list is [Agumon]/[Nokia Shiramine] instead of [Gabumon]/[Nokia].
//!
//! # Patterns this test covers
//! - H9 — face-up `<Jamming>` declarative grant_keyword
//! - D4 — declarative aura inherited DP modifier ([Opponent's Turn] +2000 DP)
//! - Alt-digivolve: Lv.3 with [Gabumon] in name OR [CS] trait, cost 2,
//!   ignoring color requirement
//!
//! # Clause status
//!
//! - `<Jamming>`                                            IMPLEMENTED
//!   (face-up grant_keyword; runtime install resolved by G-DECLARATIVE-KEYWORD
//!   2026-05-02 — Group 6 declarative-effect tick).
//! - [Main][OPT] place-top-as-bottom + play Agumon/Nokia   BLOCKED
//!   (G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM — no DSL verb to bind/place "the top
//!   stacked card of <permanent>" as that permanent's bottom digivolution
//!   card without a player-facing prompt). Tests in Section 4 are #[ignore]'d.
//! - [Inherited][Opponent's Turn] +2000 DP                  IMPLEMENTED
//!   (self-aura empty target + active_when opponents_turn; inverse gate of
//!   BT23-008 Clause 3).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

use crate::dsl_card_data::card_data_from_compiled;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn garurumon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-018")
        .expect("BT23-018 found in embedded DSL pack")
        .memory(10)
        .start()
}

/// Build a runner with BT23-018 (Garurumon) registered and stacked under a
/// Lv5 dummy Digimon on player 0's battle area.
///
/// Stack layout after this helper returns:
///   P0 battle_area[0]: [garurumon (source 0), carrier_lv5 (source 1/top)]
fn runner_with_garurumon_as_source() -> (DebugRunner, PermanentHandle) {
    let garurumon_data = card_data_from_compiled("BT23-018");
    let mut carrier = make_test_card("CARRIER-LV5", "CarrierLv5");
    carrier.level = Some(5);
    carrier.dp = Some(7000);

    let mut runner = DebugRunner::builder()
        .add_card(garurumon_data)
        .add_card(carrier)
        .memory(10)
        .start();

    // Place Garurumon on field as the base (source 0).
    let garurumon_handle = runner.place_on_field(0, "BT23-018", Some(0));

    // Push a Lv5 carrier on top of Garurumon by direct stack manipulation.
    let carrier_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "CARRIER-LV5")
        .expect("CARRIER-LV5 in card_data");
    let next_idx = runner.game.next_card_index();
    let carrier_source = CardSource::new(carrier_data_idx, 0, next_idx);
    let turn = runner.game.turn_count;
    runner.game.players[0].battle_area[garurumon_handle.index as usize]
        .digivolve(carrier_source, turn);

    (runner, garurumon_handle)
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn bt23_018_compiles_in_embedded_pack() {
    let runner = garurumon_runner();
    assert!(
        runner.compiled_card("BT23-018").is_some(),
        "BT23-018 must be present in embedded DSL pack"
    );
}

#[test]
fn bt23_018_card_metadata_matches_print() {
    let runner = garurumon_runner();
    let card = runner
        .compiled_card("BT23-018")
        .expect("BT23-018 compiles");

    assert_eq!(card.level, Some(4), "Garurumon is Lv.4");
    assert_eq!(card.dp, Some(5000), "Garurumon is DP 5000");
    assert_eq!(card.cost, Some(6), "Garurumon costs 6 to play");
}

/// Standard alt-digivolve: Lv.3 Blue / Cost 2 must be present.
#[test]
fn bt23_018_standard_digivolve_alt_path_present() {
    let runner = garurumon_runner();
    let card = runner
        .compiled_card("BT23-018")
        .expect("BT23-018 compiles");

    let std_path = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(2))
                && !p.ignore_requirements
        })
        .expect("BT23-018 must declare a standard Lv3 digivolve path with cost 2");

    let from = std_path
        .from
        .as_ref()
        .expect("standard digivolve must declare a `from:` predicate");
    assert_eq!(
        from.level_eq,
        Some(3),
        "standard digivolve must require level_eq: 3"
    );
}

/// Alt-digivolve: Lv.3 with [Gabumon] in name OR [CS] trait, cost 2,
/// ignoring color requirement.
#[test]
fn bt23_018_alt_digivolve_gabumon_or_cs_cost_2_ignores_requirements() {
    let runner = garurumon_runner();
    let card = runner
        .compiled_card("BT23-018")
        .expect("BT23-018 compiles");

    let alt = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(2))
                && p.ignore_requirements
        })
        .expect(
            "BT23-018 must declare an alt-digivolve path with cost 2 and ignore_requirements",
        );

    let from = alt
        .from
        .as_ref()
        .expect("alt digivolve must declare a `from:` predicate");

    // Recursive predicate walker — same shape as bt23_008's helper.
    fn pred_any<F: Fn(&digimon_dsl::compiled::CompiledPredicate) -> bool + Copy>(
        p: &digimon_dsl::compiled::CompiledPredicate,
        f: F,
    ) -> bool {
        if f(p) {
            return true;
        }
        if p.all_of.iter().any(|q| pred_any(q, f)) {
            return true;
        }
        if p.any_of.iter().any(|q| pred_any(q, f)) {
            return true;
        }
        if p.none_of.iter().any(|q| pred_any(q, f)) {
            return true;
        }
        if let Some(ref n) = p.not {
            if pred_any(n, f) {
                return true;
            }
        }
        false
    }

    let level_ok = pred_any(from, |p| p.level_eq == Some(3));
    assert!(
        level_ok,
        "alt-digivolve `from:` must require level_eq: 3 somewhere in the predicate"
    );

    let gabumon = pred_any(from, |p| p.name_contains.as_deref() == Some("Gabumon"));
    let cs = pred_any(from, |p| {
        p.trait_has
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("CS"))
            .unwrap_or(false)
    });
    assert!(
        gabumon,
        "alt-digivolve `from:` must allow name_contains: Gabumon \
         (Garurumon's natural pre-evo lower form)"
    );
    assert!(cs, "alt-digivolve `from:` must allow trait_has: CS");
}

/// Face-up `<Jamming>` grant_keyword declarative clause must be present.
#[test]
fn bt23_018_has_face_up_jamming_grant_keyword() {
    let runner = garurumon_runner();
    let card = runner
        .compiled_card("BT23-018")
        .expect("BT23-018 compiles");

    let face_up_jamming = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Jamming"
        )
    });
    assert!(
        face_up_jamming,
        "BT23-018 must declare a face-up GrantKeyword Jamming clause"
    );
}

/// The inherited [Opponent's Turn] +2000 DP self-aura must be present with
/// the right scope, dp_modifier, and active_when.
#[test]
fn bt23_018_has_inherited_opponents_turn_dp_aura() {
    let runner = garurumon_runner();
    let card = runner
        .compiled_card("BT23-018")
        .expect("BT23-018 compiles");

    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope,
            dp_modifier,
            active_when,
            ..
        }) => Some((*scope, *dp_modifier, active_when.clone())),
        _ => None,
    });

    let (scope, dp, active_when) =
        aura.expect("BT23-018 must have a Declarative::Aura clause (inherited +2000 DP)");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "the aura clause must have Inherited scope (active as a digivolution source)"
    );
    assert_eq!(dp, Some(2000), "the aura dp_modifier must be +2000 DP");
    let aw = active_when
        .expect("the aura must declare active_when (opponents_turn: true)");
    assert_eq!(
        aw.opponents_turn,
        Some(true),
        "the aura must be gated on opponents_turn: true"
    );
}

/// BT23-018's BLOCKED clause must be omitted: there is no [Main]/MainOnField
/// triggered clause (Clause 2 is BLOCKED — G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM).
#[test]
fn bt23_018_no_main_on_field_triggered_clause() {
    let runner = garurumon_runner();
    let card = runner
        .compiled_card("BT23-018")
        .expect("BT23-018 compiles");

    let main_clauses = card
        .effects
        .iter()
        .filter(|c| match c {
            CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::MainOnField),
            _ => false,
        })
        .count();
    assert_eq!(
        main_clauses, 0,
        "Clause 2 is BLOCKED — no MainOnField triggered clause should be present"
    );
}

// ─── Section 2: Behavioral — inherited DP gate ([Opponent's Turn]) ───────────
//
// The inverse polarity of BT23-008: BT23-018's +2000 DP must be ACTIVE on the
// opponent's turn and INACTIVE on the controller's own turn.

/// On the controller's own turn the [Opponent's Turn] gate must suppress the
/// DP buff.
#[test]
fn bt23_018_inherited_dp_inactive_on_your_turn() {
    let (runner, carrier_handle) = runner_with_garurumon_as_source();

    // Source index 0 = Garurumon (the base), index 1 = carrier (top card).
    let garurumon_src_idx = 0;

    assert_eq!(
        runner.turn_player(),
        0,
        "precondition: it is player 0's (the controller's) turn"
    );

    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, garurumon_src_idx);

    assert_eq!(
        contribution, 0,
        "Garurumon's inherited +2000 DP must NOT contribute on the controller's own turn \
         ([Opponent's Turn] gate); got {} instead",
        contribution
    );
}

/// On the opponent's turn the [Opponent's Turn] gate must light up the DP
/// buff for the Garurumon source.
#[test]
fn bt23_018_inherited_dp_active_on_opponents_turn() {
    let (mut runner, carrier_handle) = runner_with_garurumon_as_source();

    runner.end_turn();
    assert_eq!(
        runner.turn_player(),
        1,
        "precondition: it is now player 1's (the opponent's) turn"
    );

    let garurumon_src_idx = 0;
    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, garurumon_src_idx);

    assert_eq!(
        contribution, 2000,
        "Garurumon's inherited +2000 DP must contribute on the opponent's turn; \
         got {} instead",
        contribution
    );
}

/// Confirm the carrier's TOP card slot does NOT contribute the +2000 DP
/// (it is not the Garurumon source). Sample on the opponent's turn where the
/// gate is open so any leakage would surface.
#[test]
fn bt23_018_top_card_does_not_contribute_garurumon_dp() {
    let (mut runner, carrier_handle) = runner_with_garurumon_as_source();
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);

    let top_src_idx = 1;
    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, top_src_idx);

    assert_eq!(
        contribution, 0,
        "the carrier's own slot (top card) should not contribute Garurumon's buff"
    );
}

// ─── Section 3: Behavioral — face-up <Jamming> keyword install ───────────────
//
// Per G-DECLARATIVE-KEYWORD (RESOLVED 2026-05-02), face-up declarative
// grant_keyword clauses are now installed at runtime via the Group 6
// declarative-effect tick. Place BT23-018 on field and verify Jamming is
// installed.

#[test]
fn bt23_018_face_up_jamming_installs_on_field() {
    let mut runner = garurumon_runner();
    let garurumon = runner.place_on_field(0, "BT23-018", Some(0));

    // Run a tick so declarative effects materialize.
    runner.auto_resolve().ok();

    assert!(
        runner.game.has_keyword(garurumon, Keyword::Jamming),
        "face-up <Jamming> keyword must be installed on a face-up BT23-018 \
         (G-DECLARATIVE-KEYWORD resolved 2026-05-02)"
    );
}

// ─── Section 4: [Main][OPT] place-top-as-bottom + play — BLOCKED ─────────────
//
// Clause 2 cannot be expressed in the current DSL:
//   "By placing this Digimon's top stacked card as its bottom digivolution
//    card, you may play 1 [Agumon] or [Nokia Shiramine] from your hand with
//    the play cost reduced by 2."
//
// Same gap as the BT23-008 sister card. The DSL has `place_as_bottom_source
// { source, target }` for an arbitrary CardSource binding, but no verb that
// takes "the top stacked card of <permanent>" deterministically.
// `select_material` would expose a fake player-facing choice that the
// printed text and DCGO do not present (DCGO reads
// `card.PermanentOfThisCard().TopCard` directly with no prompt).
//
// Per skill rule §4 — Engine-gap vs DSL-vocab-gap discipline — the entire
// clause is OMITTED until a `place_top_source_as_bottom: { target: <perm> }`
// step lowers to:
//
//   let top = perm.top_card_source();
//   ctx.place_as_bottom_source(CardSourceRef::Material(perm, top_idx), perm);
//
// without a player-facing selection.
//
// Test stubs below outline the intended behavioral coverage once the gap
// closes. They are `#[ignore]`'d with the gap tag so they are mechanically
// unblockable.

/// Positive branch: with at least 1 stacked card and an [Agumon] in hand,
/// activating the [Main] OPT pays the cost (top stacked card → bottom),
/// presents the Agumon/Nokia hand prompt, and on accept plays Agumon at
/// printed cost - 2.
#[test]
#[ignore = "pending: G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM — no DSL verb for placing the top \
             stacked card of a permanent as its bottom digivolution card without a \
             player-facing prompt (see qa/dsl-vocab-gaps.md)"]
fn bt23_018_main_opt_with_agumon_pays_cost_and_plays_at_minus_2() {
    todo!("implement once DSL adds place_top_source_as_bottom step")
}

/// Negative gate: with empty stack (Garurumon just played, no carriers),
/// the Main OPT must not be activatable (DCGO: `StackCards.Count >= 1` gate).
#[test]
#[ignore = "pending: G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM"]
fn bt23_018_main_opt_blocked_when_stack_empty() {
    todo!("implement once DSL adds place_top_source_as_bottom step")
}

/// OPT lockout: after firing the [Main] OPT once, it cannot be activated a
/// second time on the same turn even if all other gates are satisfied.
#[test]
#[ignore = "pending: G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM"]
fn bt23_018_main_opt_once_per_turn_locks_out_second_activation() {
    todo!("implement once DSL adds place_top_source_as_bottom step")
}

/// Decline branch: paying the cost (top stacked card → bottom) but declining
/// the optional play of Agumon/Nokia must still consume the OPT lockout
/// (DCGO: cost is paid before the SelectHandEffect; the optional decline is
/// downstream of the cost). Hand and trash unchanged from the play step.
#[test]
#[ignore = "pending: G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM"]
fn bt23_018_main_opt_cost_paid_then_decline_play_consumes_opt() {
    todo!("implement once DSL adds place_top_source_as_bottom step")
}

/// Filter: the optional play prompt must allow only [Agumon]-named Digimon
/// or [Nokia Shiramine]-named Tamer cards from hand. An unrelated hand card
/// (e.g. a Gabumon) must not be selectable.
#[test]
#[ignore = "pending: G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM"]
fn bt23_018_main_opt_filters_hand_to_agumon_or_nokia_only() {
    todo!("implement once DSL adds place_top_source_as_bottom step")
}
