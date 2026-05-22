//! EX4-060 Omnimon Alter-S — Digimon, Lv.7, White, DP 15000, Cost 15.
//! Traits: Holy Warrior.
//! Evo: Lv.6 Blue / Cost 6
//! DNA Digivolve: Lv.6 Blue + Lv.6 Red / Cost 0
//!
//! # Card text (cards.json — verbatim)
//!
//! ```text
//! [When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or
//!   less, and return 1 of your opponent's level 6 or higher Digimon to the
//!   bottom of its owner's deck.
//! [All Turns] When this Digimon would leave the battle area other than by
//!   one of your effects, play 1 [BlitzGreymon] and 1 [CresGarurumon] from
//!   this Digimon's digivolution cards without paying the costs. Then,
//!   place this Digimon at the bottom of your security stack face down.
//! ```
//!
//! Inherited / Security: (none)
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/EX4/White/EX4_060.cs`
//!
//! # Patterns this test covers
//! - Structural: 2 alt_paths (Digivolve Lv.6/Blue/cost-6 + DNA Blue+Red
//!   cost 0) + 1 triggered clause ([When Digivolving] mandatory delete +
//!   return-to-deck) + 1 declarative replacement clause ([All Turns]
//!   leave-other-than-own-effect → play BlitzGreymon + CresGarurumon free,
//!   then place self at security bottom face-down).
//! - F1-adjacent literal `dp_lte: 8000` predicate (sister tests:
//!   BT8-097, BT13-012, BT18-087, BT21-015, BT21-025).
//! - Sequential mandatory selects within one clause (BT15-101 idiom).
//! - F3 Replacement effect: `when_would_leave_battle_area` with
//!   `replacement_cause: own_effect` negative gate.
//! - Self-security-bottom placement via
//!   `place_permanent_on_security_and_handle_replacement`.
//!
//! # Sister cards (cross-reference)
//! - BT22-015 Omnimon — sister Decode form whose play-from-own-digivolution
//!   surfaces the SAME engine primitive (G-DECODE-PLAY-FROM-OWN-DIGIVOLUTION-
//!   SOURCES). EX4-060's clause is [All Turns] / not Decode-keyworded; both
//!   need the same engine substrate.
//! - BT17-078 Omnimon — sister DNA Greymon+Garurumon shape, but EX4-060
//!   uses color-only filters (Blue + Red) instead of name_contains.
//! - BT24-040 — sister `kind: replacement` with `replacement_cause` filter,
//!   but cancels the leave (place ANOTHER permanent at security bottom);
//!   EX4-060's clause does NOT cancel and reroutes the SELF subject.
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                              | Status         |
//! |----------------------------------------------------------------|----------------|
//! | Standard digivolve Lv.6 Blue / Cost 6                           | OK             |
//! | DNA digivolve Lv.6 Blue + Lv.6 Red / Cost 0                     | OK             |
//! | [When Digivolving] Delete 1 opp Digimon DP <= 8000               | OK             |
//! | [When Digivolving] Return 1 opp Lv >= 6 Digimon to bottom-deck   | OK             |
//! | [All Turns] On leave-other-than-own-effect, play BlitzGreymon + CresGarurumon free | OK (gaps G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES + G-PLACE-SELF-AT-SECURITY-BOTTOM closed 2026-05-08) |
//! | [All Turns] Then, place self at bottom of security face down     | OK             |

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

/// Production YAML for EX4-060, inlined at compile time from the canonical
/// location under `cards/ex4/`.
const YAML: &str = include_str!("../../../cards/ex4/EX4-060.yaml");

/// Compile EX4-060 from the production YAML and return the CompiledCard.
fn compiled_ex4_060() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("EX4-060.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("EX4-060.yaml compiles");
    registry
        .lookup("EX4-060")
        .expect("EX4-060 in registry")
        .clone()
}

// ─── Fixture helpers ────────────────────────────────────────────────────────

fn make_opp_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn make_named_digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card
}

fn battle_area_has_top_card(runner: &DebugRunner, player: PlayerId, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

fn place_ex4_on_field(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_on_field(player, "EX4-060", Some(0))
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions (parse / compile / topology)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex4_060_compiles_with_two_alt_paths_and_one_triggered() {
    let card = compiled_ex4_060();

    assert_eq!(
        card.alt_paths.len(),
        2,
        "expected exactly 2 alt_paths (Digivolve Lv.6/Blue/cost-6 + DnaDigivolve Blue+Red cost-0), got {}",
        card.alt_paths.len()
    );

    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        1,
        "expected exactly 1 triggered clause ([When Digivolving]); \
         the [All Turns] leave-replacement is a declarative Replacement, not counted here; got {}",
        triggered.len()
    );
}

/// Structural: the [All Turns] replacement clause compiles to a declarative
/// `Replacement` with trigger `when_would_leave_battle_area` (F3 pattern).
/// Its `none_of: replacement_cause: own_effect` active_when gate is what
/// enforces the "other than by one of your effects" restriction.
#[test]
fn ex4_060_has_declarative_replacement_when_would_leave_battle_area() {
    let card = compiled_ex4_060();

    let has_replacement = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                trigger,
                ..
            }) if trigger == "when_would_leave_battle_area"
        )
    });

    assert!(
        has_replacement,
        "EX4-060 must have a declarative `when_would_leave_battle_area` replacement clause \
         for the [All Turns] leave-other-than-own-effect text"
    );
}

#[test]
fn ex4_060_first_alt_path_is_digivolve_lv6_blue_cost6() {
    let card = compiled_ex4_060();
    let path = &card.alt_paths[0];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(6)),
        "standard digivolve must be Lv.6/Blue/cost-6 (printed cards.json evo_costs)"
    );
    assert!(
        !path.ignore_requirements,
        "standard digivolve must NOT ignore requirements"
    );
}

#[test]
fn ex4_060_second_alt_path_is_dna_digivolve_blue_red_cost0() {
    let card = compiled_ex4_060();
    let path = &card.alt_paths[1];
    assert_eq!(path.kind, CompiledAltPathKind::DnaDigivolve);
    assert_eq!(
        path.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(0)),
        "DNA digivolve must be cost 0 (printed dna_costs)"
    );
    let mats = &path.materials;
    assert_eq!(
        mats.len(),
        2,
        "DNA digivolve must have exactly 2 materials (Lv.6 Blue + Lv.6 Red), got {}",
        mats.len()
    );
}

#[test]
fn ex4_060_when_digivolving_clause_is_face_up_mandatory() {
    let card = compiled_ex4_060();
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if matches!(t.scope, CompiledScope::FaceUp)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have a face-up [When Digivolving] clause");

    assert!(
        !clause.optional,
        "[When Digivolving] clause is mandatory (printed text has no 'may')"
    );
    assert!(
        !clause.once_per_turn,
        "no [Once Per Turn] on the [When Digivolving] clause"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — [When Digivolving] delete-then-return behavioral
// ═══════════════════════════════════════════════════════════════════════════════

fn trigger_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Positive: [When Digivolving] with both arms having a single legal
/// target each. The delete-DP-<=8000 arm removes the low-DP Digimon, and
/// the return-Lv-6+ arm bottom-decks the Lv.6 Digimon.
#[test]
fn ex4_060_when_digivolving_deletes_low_dp_and_bottom_decks_lv6() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-060 YAML loads")
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 4, 5000))
        .add_card(make_opp_digimon("OPP-LV6", "OppLv6", 6, 11000))
        .memory(15)
        .start();

    let ex4 = place_ex4_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LOW", None);
    runner.place_on_field(1, "OPP-LV6", None);
    let opp_before = runner.battle_area_size(1);

    trigger_when_digivolving(&mut runner, ex4);
    let _ = runner.auto_resolve();

    let opp_after = runner.battle_area_size(1);
    assert!(
        opp_after <= opp_before.saturating_sub(1),
        "[When Digivolving] must remove at least the delete arm's target \
         (delete + return-to-deck both remove from battle area). Before: {}, after: {}.",
        opp_before,
        opp_after,
    );
}

/// Positive: with ONLY a legal delete target on the field (no Lv >= 6
/// opp Digimon), the delete arm fires and the return-to-deck arm
/// silently skips. Mirrors DCGO's per-arm HasMatchConditionPermanent
/// gate at lines 169 / 191.
#[test]
fn ex4_060_when_digivolving_only_low_dp_target_fires_delete_only() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-060 YAML loads")
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 4, 5000))
        .memory(15)
        .start();

    let ex4 = place_ex4_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LOW", None);
    let opp_before = runner.battle_area_size(1);

    trigger_when_digivolving(&mut runner, ex4);
    let _ = runner.auto_resolve();

    let opp_after = runner.battle_area_size(1);
    assert_eq!(
        opp_after,
        opp_before - 1,
        "delete arm must fire on the only legal DP-<=8000 target; return-to-deck arm silently skips"
    );
}

/// Positive: with ONLY a legal return-to-deck target on the field (no
/// DP <= 8000 opp Digimon), the delete arm silently skips and the
/// return-to-deck arm fires.
#[test]
fn ex4_060_when_digivolving_only_lv6_target_fires_return_only() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-060 YAML loads")
        .add_card(make_opp_digimon("OPP-LV6-HIGH", "OppLv6High", 6, 13000))
        .memory(15)
        .start();

    let ex4 = place_ex4_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LV6-HIGH", None);
    let opp_before = runner.battle_area_size(1);

    trigger_when_digivolving(&mut runner, ex4);
    let _ = runner.auto_resolve();

    let opp_after = runner.battle_area_size(1);
    assert_eq!(
        opp_after,
        opp_before - 1,
        "return-to-deck arm must fire on the only legal Lv >= 6 target; delete arm silently skips"
    );
}

/// Negative: with NO opp Digimon at all, the [When Digivolving] clause
/// must not panic and must leave the field unchanged (both arms silently
/// skip per per-arm existential guards).
#[test]
fn ex4_060_when_digivolving_no_opp_digimon_does_not_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-060 YAML loads")
        .memory(15)
        .start();

    let ex4 = place_ex4_on_field(&mut runner, 0);
    let opp_before = runner.battle_area_size(1);

    trigger_when_digivolving(&mut runner, ex4);
    // No assertion needed beyond non-panic and field unchanged.
    assert_eq!(runner.battle_area_size(1), opp_before);
}

/// Negative gate: with a legal candidate that fails BOTH filters
/// (e.g. an Lv.5 Digimon at 11000 DP — too high for delete, too low
/// for return-to-deck), neither arm fires and the field is unchanged.
#[test]
fn ex4_060_when_digivolving_filters_exclude_high_dp_low_level() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-060 YAML loads")
        .add_card(make_opp_digimon("OPP-MID", "OppMid", 5, 11000))
        .memory(15)
        .start();

    let ex4 = place_ex4_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-MID", None);
    let opp_before = runner.battle_area_size(1);

    trigger_when_digivolving(&mut runner, ex4);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "Lv.5 Digimon at 11000 DP fails both filters — neither arm should fire"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — Negative gating: target filter correctness on each arm
// ═══════════════════════════════════════════════════════════════════════════════

/// With opponent Digimon at DP 8001 only, the delete arm's candidate
/// list must be empty (boundary check on `dp_lte: 8000`). Acceptance
/// criterion: field unchanged.
#[test]
fn ex4_060_dp_filter_excludes_dp_8001() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-060 YAML loads")
        .add_card(make_opp_digimon("OPP-8001", "Opp8001", 5, 8001))
        .memory(15)
        .start();

    let ex4 = place_ex4_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-8001", None);
    let opp_before = runner.battle_area_size(1);

    trigger_when_digivolving(&mut runner, ex4);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "DP 8001 must be excluded by `dp_lte: 8000` (boundary), and Lv.5 fails the return arm too"
    );
}

/// With an opp Digimon at exactly DP 8000, the delete arm's candidate
/// list must include it (boundary check on `dp_lte: 8000`). Acceptance:
/// the Digimon is removed.
#[test]
fn ex4_060_dp_filter_includes_dp_8000_boundary() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-060 YAML loads")
        .add_card(make_opp_digimon("OPP-8000", "Opp8000", 5, 8000))
        .memory(15)
        .start();

    let ex4 = place_ex4_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-8000", None);
    let opp_before = runner.battle_area_size(1);

    trigger_when_digivolving(&mut runner, ex4);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "DP 8000 is INCLUSIVE on `dp_lte: 8000` — the delete arm must fire"
    );
}

/// With an opp Lv.5 Digimon at low DP (eligible for delete) and an opp
/// Lv.7 Digimon at high DP (eligible for return-to-deck), both arms
/// fire on their respective targets — order: delete first, then return.
#[test]
fn ex4_060_when_digivolving_both_arms_fire_independently_lv5_and_lv7() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-060 YAML loads")
        .add_card(make_opp_digimon("OPP-LV5-LOW", "OppLv5Low", 5, 4000))
        .add_card(make_opp_digimon("OPP-LV7-HIGH", "OppLv7High", 7, 14000))
        .memory(15)
        .start();

    let ex4 = place_ex4_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LV5-LOW", None);
    runner.place_on_field(1, "OPP-LV7-HIGH", None);

    trigger_when_digivolving(&mut runner, ex4);
    let _ = runner.auto_resolve();

    // Both should be removed. Delete consumes Lv.5/4000; return-to-deck
    // consumes Lv.7/14000. Field count drops by 2.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "both arms must fire independently when each has its own legal target"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4 — [All Turns] leave replacement
// ═══════════════════════════════════════════════════════════════════════════════

/// When EX4-060 would leave the battle area outside of own-effect cause,
/// its controller plays 1 [BlitzGreymon] source and 1 [CresGarurumon]
/// source without paying the costs, then EX4-060 is placed as bottom
/// security face-down.
#[test]
fn ex4_060_all_turns_offers_play_blitz_and_cres_from_own_digivolution_on_leave() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-060 YAML loads")
        .add_card(make_named_digimon("BLITZ", "BlitzGreymon"))
        .add_card(make_named_digimon("CRES", "CresGarurumon"))
        .add_card(make_named_digimon("OTHER", "OtherGreymon"))
        .memory(15)
        .start();
    let ex4 = runner.place_stack(0, &["BLITZ", "CRES", "OTHER", "EX4-060"]);

    runner
        .game
        .delete_permanent_with_cause(ex4, ReplacementCause::OpponentEffect);

    let blitz_pick = runner
        .pending_selection_view()
        .expect("EX4-060 should first ask for a [BlitzGreymon] source");
    assert_eq!(blitz_pick.kind, SelectionKind::Material);
    assert_eq!(
        blitz_pick.valid_action_ids.len(),
        1,
        "only the BlitzGreymon-named source should pass the first source filter"
    );
    runner
        .execute_action(0, blitz_pick.valid_action_ids[0])
        .expect("select BlitzGreymon source");

    let cres_pick = runner
        .pending_selection_view()
        .expect("EX4-060 should then ask for a [CresGarurumon] source");
    assert_eq!(cres_pick.kind, SelectionKind::Material);
    assert_eq!(
        cres_pick.valid_action_ids.len(),
        1,
        "only the CresGarurumon-named source should pass the second source filter"
    );
    runner
        .execute_action(0, cres_pick.valid_action_ids[0])
        .expect("select CresGarurumon source");

    assert!(
        battle_area_has_top_card(&runner, 0, "BLITZ"),
        "selected BlitzGreymon source should be played for free"
    );
    assert!(
        battle_area_has_top_card(&runner, 0, "CRES"),
        "selected CresGarurumon source should be played for free"
    );
    assert!(
        !battle_area_has_top_card(&runner, 0, "EX4-060"),
        "the original EX4-060 should leave the battle area"
    );
    let bottom_security = runner
        .game
        .players
        .first()
        .and_then(|player| player.security.first())
        .expect("EX4-060 should be placed as bottom security");
    assert_eq!(bottom_security.card_id(&runner.game.card_data), "EX4-060");
    assert!(
        !runner.game.players[0]
            .face_up_security
            .contains(&bottom_security.card_index),
        "EX4-060 places itself face-down at security bottom"
    );
}

/// After the play-from-stack arm resolves, the leaving EX4-060 is placed
/// face-down at the bottom of its controller's security stack. Any sources
/// left under it after the BlitzGreymon/CresGarurumon plays are trashed by
/// the normal stack-disposition rule.
#[test]
fn ex4_060_all_turns_places_self_at_security_bottom_face_down_after_leave() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-060 YAML loads")
        .add_card(make_named_digimon("BLITZ", "BlitzGreymon"))
        .add_card(make_named_digimon("CRES", "CresGarurumon"))
        .add_card(make_named_digimon("OTHER", "OtherGreymon"))
        .memory(15)
        .start();
    let ex4 = runner.place_stack(0, &["BLITZ", "CRES", "OTHER", "EX4-060"]);

    runner
        .game
        .delete_permanent_with_cause(ex4, ReplacementCause::OpponentEffect);
    while let Some(view) = runner.pending_selection_view() {
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("choose required EX4-060 source");
    }

    let bottom_security = runner
        .game
        .players
        .first()
        .and_then(|player| player.security.first())
        .expect("EX4-060 should be placed as bottom security");
    assert_eq!(bottom_security.card_id(&runner.game.card_data), "EX4-060");
    assert!(
        !runner.game.players[0]
            .face_up_security
            .contains(&bottom_security.card_index),
        "EX4-060 should be face-down in security"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "OTHER"),
        "unplayed leftover sources should be trashed when EX4-060 leaves"
    );
}

/// When EX4-060 would leave by its controller's own effect, the [All Turns]
/// clause must not fire (DCGO's `!IsByEffect(IsOwnerEffect)` gate). The
/// leaving Digimon goes to trash normally, not to security bottom.
#[test]
fn ex4_060_all_turns_does_not_fire_on_own_effect_leave() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-060 YAML loads")
        .add_card(make_named_digimon("BLITZ", "BlitzGreymon"))
        .add_card(make_named_digimon("CRES", "CresGarurumon"))
        .memory(15)
        .start();
    let ex4 = runner.place_stack(0, &["BLITZ", "CRES", "EX4-060"]);

    runner
        .game
        .delete_permanent_with_cause(ex4, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "own-effect leave must not install the EX4-060 source-play replacement"
    );
    assert!(
        runner.game.players[0].security.is_empty(),
        "own-effect leave must not place EX4-060 into security"
    );
    assert!(
        !battle_area_has_top_card(&runner, 0, "EX4-060"),
        "own-effect leave still deletes EX4-060 normally"
    );
}
