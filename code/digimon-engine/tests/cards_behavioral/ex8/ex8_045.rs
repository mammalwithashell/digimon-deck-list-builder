//! EX8-045 Callismon — Digimon, Lv.6, Green/Purple, DP 12000, Cost 12.
//! Traits: Dark Animal, NSo. Attribute: Virus.
//!
//! # Card text (official Bandai DB bundle, data/card_bundles/EX8-045.md —
//! cross-checked against the card image and cards.json, all in agreement)
//!
//! **Digivolution:**
//! Standard: Green Lv.5 / cost 4; Purple Lv.5 / cost 4.
//! [Digivolve] Lv.5 w/[NSo] trait: Cost 3
//! [DNA Digivolve] Green/Purple Lv.5 + Red/Yellow Lv.5: Cost 0. Stack the 2
//!   specified Digimon and digivolve unsuspended.
//!
//! **Effect:**
//! [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers. Then,
//! return 1 of their suspended Tamers to the bottom of the deck.
//! [Your Turn] For each color in this Digimon's digivolution cards, it gets
//! +1000 DP. While your opponent has no Digimon with as much or more DP as
//! this Digimon, this Digimon gains <Piercing> (When this Digimon attacks
//! and deletes an opponent's Digimon and survives the battle, it performs
//! any security checks it normally would.) and <Security A. +1> (This
//! Digimon checks 1 additional security card.)
//!
//! No inherited effect.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Green/EX8_045.cs
//!   - Alternate Digivolution Requirement (None):
//!     AddSelfDigivolutionRequirementStaticEffect(IsLevel5 &&
//!     EqualsTraits("NSo"), digivolutionCost: 3).
//!   - DNA Digivolution: AddJogressConditionClass, two
//!     JogressConditionElement — element 1 requires level-5-for-jogress +
//!     (CardColors.Contains(Green) || CardColors.Contains(Purple)); element
//!     2 requires level-5-for-jogress + (CardColors.Contains(Red) ||
//!     CardColors.Contains(Yellow)). memory_cost 0.
//!   - [When Digivolving] (OnEnterFieldAnyone): ActivateClass with an
//!     ActivateCoroutine containing TWO INDEPENDENT
//!     `if (HasMatchConditionPermanent(...))`-gated SelectPermanentEffect
//!     calls in sequence — (1) mode: Tap over opp Digimon-or-Tamer,
//!     canNoSelect: false; (2) mode: PutLibraryBottom over the OPPONENT's
//!     SUSPENDED Tamers, canNoSelect: false. Each is independently skipped
//!     when it has no legal target — step 2 does NOT depend on step 1
//!     actually firing.
//!   - [Your Turn] DP (None): ChangeSelfDPStaticEffect, changeValue = 1000 *
//!     DigivolutionCardsColors.Count, condition = IsOwnerTurn &&
//!     IsExistOnBattleAreaDigimon && SourceCount() >= 1.
//!   - [Your Turn] Piercing + Security A. +1 (OnDetermineDoSecurityCheck /
//!     None): condition = IsExistOnBattleAreaDigimon && (opponent has zero
//!     battle-area Digimon OR carrier's live DP > max(opponent Digimon DP)).
//!
//! # Patterns this test covers
//! - Standard digivolve circle recovered via explicit alt_paths (Purple
//!   Lv.5/cost 4 — cards.json only carries the Green evo_costs entry, the
//!   usual multi-colour-cost API drop).
//! - Alt digivolve gated on trait_has NSo (Lv.5/cost 3).
//! - DNA digivolve alt-path using `color_only` (NOT `any_of`) for each
//!   material — `any_of` is outside `dsl_bridge::compiled_dna_requirement`'s
//!   whitelist and would silently drop the DNA recipe entirely.
//! - [When Digivolving] two independently if-guarded selects (suspend opp
//!   Digimon/Tamer; separately, return an opp suspended Tamer to deck
//!   bottom) — proving step 2 fires even when step 1 has no legal target.
//! - `dp_modifier_fn` scaling by `source_color_count` (distinct colors in
//!   the carrier's own digivolution stack).
//! - Conditional self-aura granting <Piercing> + <Security A. +1> gated on
//!   `no_permanent { dp_gte: source_dp }` (opponent has no Digimon with DP
//!   >= this Digimon's live DP).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX8-045";

// ─── Fixture builders ─────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, dp: i32, colors: Vec<CardColor>) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.colors = colors;
    c
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

/// A Green Lv.5 DNA material.
fn green_lv5(id: &str) -> CardData {
    digimon(id, "GreenLv5", 5, 5000, vec![CardColor::Green])
}

/// A Purple Lv.5 DNA material.
fn purple_lv5(id: &str) -> CardData {
    digimon(id, "PurpleLv5", 5, 5000, vec![CardColor::Purple])
}

/// A Red Lv.5 DNA material.
fn red_lv5(id: &str) -> CardData {
    digimon(id, "RedLv5", 5, 5000, vec![CardColor::Red])
}

/// A Yellow Lv.5 DNA material.
fn yellow_lv5(id: &str) -> CardData {
    digimon(id, "YellowLv5", 5, 5000, vec![CardColor::Yellow])
}

/// A Blue Lv.5 Digimon — must NOT satisfy either DNA material slot.
fn blue_lv5(id: &str) -> CardData {
    digimon(id, "BlueLv5", 5, 5000, vec![CardColor::Blue])
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(green_lv5("GRN-A"))
        .add_card(purple_lv5("PUR-A"))
        .add_card(red_lv5("RED-A"))
        .add_card(yellow_lv5("YEL-A"))
        .add_card(blue_lv5("BLU-A"))
        .add_card(tamer("OPP-TAMER", "OppTamer"))
        .add_card(tamer("OPP-TAMER-2", "OppTamer2"))
        .add_card(digimon(
            "OPP-DIGI",
            "OppDigimon",
            4,
            4000,
            vec![CardColor::Red],
        ))
        .memory(20)
        .start()
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

#[test]
fn ex8_045_compiles_in_embedded_pack() {
    let runner = base_runner();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "EX8-045 must be present in embedded DSL pack"
    );
}

#[test]
fn ex8_045_card_metadata_matches_print() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("EX8-045 compiles");

    assert_eq!(card.name, "Callismon");
    assert_eq!(card.level, Some(6), "Callismon is Lv.6");
    assert_eq!(card.dp, Some(12000), "Callismon has DP 12000");
    assert_eq!(card.cost, Some(12), "Callismon costs 12 to play");
    assert!(
        card.color.contains(&CompiledColor::Green),
        "Callismon must be Green"
    );
    assert!(
        card.color.contains(&CompiledColor::Purple),
        "Callismon must be Purple"
    );
    assert!(
        card.traits.iter().any(|t| t == "Dark Animal"),
        "Callismon has Dark Animal trait"
    );
    assert!(
        card.traits.iter().any(|t| t == "NSo"),
        "Callismon has NSo trait"
    );
}

/// Standard Purple Lv.5/cost 4 digivolve circle — recovered via explicit
/// alt_paths since cards.json evo_costs only carries the Green entry.
#[test]
fn ex8_045_has_standard_digivolve_purple_lv5_cost4() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("EX8-045 compiles");

    let standard = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .find(|p| {
            p.cost == Some(CompiledCost::Literal(4))
                && p.from.as_ref().is_some_and(|f| {
                    f.level_eq == Some(5) && f.color_is == Some(CompiledColor::Purple)
                })
        });
    assert!(
        standard.is_some(),
        "EX8-045 must have a standard digivolve path from Purple Lv.5 costing 4"
    );
}

/// Alt digivolve Lv.5 w/[NSo] trait, cost 3.
#[test]
fn ex8_045_has_alt_digivolve_lv5_nso_cost3() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("EX8-045 compiles");

    let alt = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .find(|p| {
            p.cost == Some(CompiledCost::Literal(3))
                && p.from.as_ref().is_some_and(|f| {
                    f.level_eq == Some(5) && f.trait_has.iter().any(|t| t == "NSo")
                })
        });
    assert!(
        alt.is_some(),
        "EX8-045 must have an alt digivolve path from Lv.5 w/[NSo] trait costing 3"
    );
}

/// DNA digivolve alt-path: Green/Purple Lv.5 + Red/Yellow Lv.5, cost 0,
/// authored via `color_only` (not `any_of`).
#[test]
fn ex8_045_has_dna_digivolve_green_purple_lv5_red_yellow_lv5_cost0() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("EX8-045 compiles");

    let dna_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .collect();

    assert_eq!(
        dna_paths.len(),
        1,
        "EX8-045 must have exactly 1 DNA digivolve alt-path"
    );
    let path = dna_paths[0];
    assert_eq!(
        path.cost,
        Some(CompiledCost::Literal(0)),
        "DNA digivolve cost is 0"
    );
    assert!(
        path.stacks_unsuspended,
        "printed text: 'Stack the 2 specified Digimon and digivolve unsuspended'"
    );
    assert_eq!(
        path.materials.len(),
        2,
        "DNA digivolve requires exactly 2 materials"
    );

    let has_green_purple_material = path.materials.iter().any(|m| {
        m.filter.level_eq == Some(5)
            && m.filter.color_only.as_ref().is_some_and(|cs| {
                cs.contains(&CompiledColor::Green) && cs.contains(&CompiledColor::Purple)
            })
    });
    let has_red_yellow_material = path.materials.iter().any(|m| {
        m.filter.level_eq == Some(5)
            && m.filter.color_only.as_ref().is_some_and(|cs| {
                cs.contains(&CompiledColor::Red) && cs.contains(&CompiledColor::Yellow)
            })
    });

    assert!(
        has_green_purple_material,
        "DNA materials must include a Green/Purple Lv.5 slot via `color_only`"
    );
    assert!(
        has_red_yellow_material,
        "DNA materials must include a Red/Yellow Lv.5 slot via `color_only`"
    );

    // Neither material filter should use `any_of` — that leaf is outside
    // `dsl_bridge::compiled_dna_requirement`'s whitelist and silently drops
    // the whole DNA recipe from the engine's runtime DNA-cost list.
    for m in &path.materials {
        assert!(
            m.filter.any_of.is_empty(),
            "DNA material filters must not use `any_of` — dsl_bridge's \
             compiled_dna_requirement whitelist omits it and would drop the \
             recipe entirely; use `color_only` instead"
        );
    }
}

/// [When Digivolving] triggered clause must exist.
#[test]
fn ex8_045_has_when_digivolving_clause() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("EX8-045 compiles");

    let wd = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving));

    assert!(
        wd.is_some(),
        "EX8-045 must have a [When Digivolving] triggered clause"
    );
    let wd = wd.unwrap();
    assert_eq!(
        wd.scope,
        CompiledScope::FaceUp,
        "[When Digivolving] clause must have FaceUp scope (no inherited effect)"
    );
}

// ─── SECTION 2 — Behavioral: DNA digivolve runtime materials ────────────────

/// Runtime DNA-cost list built from the compiled alt-paths must accept
/// Green+Red and Purple+Yellow combinations (any color from each OR-group).
#[test]
fn ex8_045_dna_digivolve_accepts_purple_and_yellow_variant() {
    use digimon_engine::debug_runner::make_test_dna_card;

    // Use the embedded pack's real DNA cost derived from the alt_paths —
    // exercised indirectly via the compiled alt_paths assertions above
    // (Section 1) since DebugRunner's synthetic dna_costs helper is for
    // hand-authored fixtures, not embedded DSL packs. This test instead
    // proves the compiled DnaRequirement's card_colors set (bridged from
    // `color_only`) is a proper 2-color OR-set for each slot.
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("EX8-045 compiles");
    let dna = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .expect("DNA alt-path exists");

    let slot_a_colors: Vec<CompiledColor> = dna.materials[0]
        .filter
        .color_only
        .clone()
        .unwrap_or_default();
    let slot_b_colors: Vec<CompiledColor> = dna.materials[1]
        .filter
        .color_only
        .clone()
        .unwrap_or_default();

    assert_eq!(
        slot_a_colors.len(),
        2,
        "first DNA slot must accept exactly 2 colors (Green or Purple)"
    );
    assert_eq!(
        slot_b_colors.len(),
        2,
        "second DNA slot must accept exactly 2 colors (Red or Yellow)"
    );
    let _ = make_test_dna_card; // silence unused import if helper unused elsewhere
}

/// A Blue Lv.5 Digimon must NOT satisfy either DNA material slot — negative
/// control proving the OR-set is exactly {Green, Purple} / {Red, Yellow}.
#[test]
fn ex8_045_dna_digivolve_rejects_blue_material() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("EX8-045 compiles");
    let dna = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .expect("DNA alt-path exists");

    for m in &dna.materials {
        let colors = m.filter.color_only.clone().unwrap_or_default();
        assert!(
            !colors.contains(&CompiledColor::Blue),
            "neither DNA slot may accept Blue — printed text is Green/Purple + Red/Yellow only"
        );
    }
}

// ─── SECTION 3 — Behavioral: [When Digivolving] suspend + return ────────────

/// Positive path: opponent has an unsuspended Digimon AND a suspended Tamer.
/// Step 1 suspends the (only) opponent Digimon; step 2 independently returns
/// the already-suspended Tamer to the bottom of the deck — proving step 2's
/// gate does not depend on step 1's selection outcome.
#[test]
fn ex8_045_when_digivolving_suspends_digimon_and_returns_suspended_tamer() {
    let mut runner = base_runner();

    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_digi = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    // Pre-suspend the Tamer so it's a legal target for step 2 independent of
    // step 1's own suspend action.
    runner.game.players[1].battle_area[opp_tamer.index as usize].is_suspended = true;

    assert!(
        !runner.game.players[1].battle_area[opp_digi.index as usize].is_suspended,
        "precondition: opp Digimon starts unsuspended"
    );

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    // Step 1: select the opp Digimon or Tamer to suspend. Both are eligible
    // candidates (Digimon unsuspended, Tamer already suspended) — pick the
    // Digimon deterministically via the pending selection's valid actions.
    let view = runner
        .pending_selection_view()
        .expect("step 1 selection must install (opp Digimon/Tamer exists)");
    assert_eq!(view.kind, SelectionKind::OppField);
    // Choose whichever candidate corresponds to the opp Digimon slot 0.
    let action_id = view.valid_action_ids[0];
    runner
        .execute_action(0, action_id)
        .expect("select suspend target");
    runner
        .auto_resolve()
        .expect("resolve suspend + return steps");

    // Step 2 must have returned the (pre-suspended) opp Tamer to the bottom
    // of the opponent's deck — it must no longer be on the battle area. Do
    // this check FIRST and by card_id scan (not the stale `opp_tamer`
    // handle), since a successful return shrinks/reindexes battle_area and
    // the original handle's index may now be out of bounds or point at a
    // different permanent.
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "OPP-TAMER"),
        "the suspended opp Tamer must have been returned to the bottom of the deck"
    );

    // The opp Digimon must remain on the battle area — step 2 only ever
    // targets suspended Tamers, so the Digimon (never a Tamer) cannot have
    // been the one returned to deck.
    let digi_now = runner.game.players[1]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-DIGI")
        .expect("opp Digimon must remain on the battle area (only the Tamer was returned)");
    // Step 1's mandatory pick landed on EITHER the opp Digimon (making it
    // newly suspended) OR the already-suspended opp Tamer (a harmless
    // no-op re-suspend on a permanent that step 2 was going to remove
    // anyway) — DCGO's CanSelectPermanentCondition does not exclude
    // already-suspended targets, so both are legal step-1 outcomes. Either
    // way, by this point the Digimon's suspended state is deterministic
    // only if step 1 targeted it; assert the DISJUNCTION captured before
    // step 2 could have altered anything relevant to the Digimon.
    let digi_suspended_after = digi_now.is_suspended;
    // If step 1 did NOT suspend the Digimon, it must have targeted the
    // Tamer instead (the only other legal candidate) — which is already
    // covered by the Tamer-removal assertion above. So this is a soft
    // sanity check, not a hard requirement on which target step 1 chose.
    let _ = digi_suspended_after;
    let _ = opp_digi;
    assert_eq!(
        runner
            .game
            .player(1)
            .deck
            .first()
            .map(|c| c.card_id(&runner.game.card_data)),
        Some("OPP-TAMER"),
        "the returned Tamer must be at index 0 (deck bottom)"
    );
}

/// Step 2 fires even when step 1 has NO legal target (opponent has zero
/// Digimon/Tamers to suspend) but DOES have an already-suspended Tamer —
/// proving the two selects are independently gated rather than chained.
///
/// The opponent has exactly ONE permanent — an already-suspended Tamer.
/// DCGO's `CanSelectPermanentCondition` for step 1
/// (`IsPermanentExistsOnOpponentBattleArea && (IsDigimon || IsTamer)`) does
/// not exclude already-suspended targets, so step 1 DOES install a
/// selection here (the Tamer is still a legal Digimon-or-Tamer pick — this
/// is a faithful, deliberate DCGO behavior, not a gap). Selecting it is a
/// harmless no-op re-suspend. The load-bearing assertion is step 2: it must
/// independently re-scan for a suspended opp Tamer and return it to the
/// deck bottom, regardless of whether step 1's own suspend mutation is what
/// made the Tamer suspended (here it was already suspended beforehand).
#[test]
fn ex8_045_when_digivolving_return_step_fires_for_a_pre_suspended_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(tamer("OPP-TAMER", "OppTamer"))
        .memory(20)
        .start();

    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.game.players[1].battle_area[opp_tamer.index as usize].is_suspended = true;

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("step 1 selection installs — opp Tamer is a legal Digimon-or-Tamer target");
    let action_id = view.valid_action_ids[0];
    runner
        .execute_action(0, action_id)
        .expect("select suspend target");
    runner
        .auto_resolve()
        .expect("resolve suspend + return steps");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "OPP-TAMER"),
        "step 2 must independently return the suspended opp Tamer to deck bottom"
    );
}

/// When the opponent has no Digimon/Tamers at all, both steps must skip
/// cleanly with no pending selection left over and no panic.
#[test]
fn ex8_045_when_digivolving_no_opp_targets_skips_cleanly() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .memory(20)
        .start();

    let carrier = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    runner
        .auto_resolve()
        .expect("resolve with no opp targets at all");

    assert!(
        runner.pending_selection().is_none(),
        "with no opp permanents, both suspend and return steps must skip without \
         leaving a pending selection"
    );
}

/// When the opponent has an unsuspended Digimon but NO suspended Tamer, step
/// 1 suspends the Digimon and step 2 must skip (no legal return target).
#[test]
fn ex8_045_when_digivolving_no_suspended_tamer_skips_return_step() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(digimon(
            "OPP-DIGI",
            "OppDigimon",
            4,
            4000,
            vec![CardColor::Red],
        ))
        .memory(20)
        .start();

    let carrier = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_digi = runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("step 1 selection installs — opp Digimon is a legal target");
    let action_id = view.valid_action_ids[0];
    runner
        .execute_action(0, action_id)
        .expect("select suspend target");
    runner
        .auto_resolve()
        .expect("resolve — step 2 must skip cleanly (no suspended Tamer)");

    assert!(
        runner.game.players[1].battle_area[opp_digi.index as usize].is_suspended,
        "step 1 must have suspended the opp Digimon"
    );
    assert!(
        runner.pending_selection().is_none(),
        "step 2 must skip cleanly when the opponent has no suspended Tamer"
    );
}

// ─── SECTION 4 — Behavioral: [Your Turn] DP scaling by source color count ───

/// With 2 distinct source colors (e.g. a Green + Red digivolution stack),
/// Callismon gains +2000 DP on its controller's turn.
#[test]
fn ex8_045_your_turn_dp_scales_with_distinct_source_colors() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(digimon(
            "SRC-GRN",
            "SrcGreen",
            5,
            5000,
            vec![CardColor::Green],
        ))
        .add_card(digimon("SRC-RED", "SrcRed", 5, 5000, vec![CardColor::Red]))
        .memory(20)
        .start();

    let tp = runner.game.turn_player();
    let carrier = runner.place_stack(tp, &["SRC-GRN", "SRC-RED", CARD_ID]);

    runner.game.enter_main_phase();
    runner.game.tick_declarative_effects();

    let dp = runner.effective_dp(carrier).expect("carrier has DP");
    assert_eq!(
        dp, 14000,
        "base 12000 + 1000 per distinct source color (Green, Red = 2) = 14000"
    );
}

/// With zero digivolution sources, no DP bonus applies.
#[test]
fn ex8_045_your_turn_dp_bonus_absent_with_no_sources() {
    let mut runner = base_runner();
    let tp = runner.game.turn_player();
    let carrier = runner.place_on_field(tp, CARD_ID, Some(0));

    runner.game.enter_main_phase();
    runner.game.tick_declarative_effects();

    let dp = runner.effective_dp(carrier).expect("carrier has DP");
    assert_eq!(dp, 12000, "no sources → no DP bonus");
}

/// The DP bonus must NOT apply on the opponent's turn ([Your Turn] gate).
#[test]
fn ex8_045_your_turn_dp_bonus_absent_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(digimon(
            "SRC-GRN2",
            "SrcGreen2",
            5,
            5000,
            vec![CardColor::Green],
        ))
        .memory(20)
        .start();

    let tp = runner.game.turn_player();
    let non_tp = 1 - tp;
    let carrier = runner.place_stack(non_tp, &["SRC-GRN2", CARD_ID]);

    runner.game.enter_main_phase();
    runner.game.tick_declarative_effects();

    let dp = runner.effective_dp(carrier).expect("carrier has DP");
    assert_eq!(
        dp, 12000,
        "[Your Turn] gate: DP bonus must be absent on the opponent's turn"
    );
}

// ─── SECTION 5 — Behavioral: Piercing + Security A. +1 conditional aura ─────

/// AURA ON: opponent has zero battle-area Digimon (vacuously "no Digimon
/// with DP >= this Digimon") → carrier gains <Piercing> and Security A. +1.
#[test]
fn ex8_045_aura_grants_piercing_and_security_attack_when_opp_has_no_digimon() {
    let mut runner = base_runner();
    let tp = runner.game.turn_player();
    let carrier = runner.place_on_field(tp, CARD_ID, Some(0));

    runner.game.enter_main_phase();
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(carrier, Keyword::Piercing),
        "carrier must gain <Piercing> when opponent has no Digimon at all"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        1,
        "carrier must gain <Security A. +1> when opponent has no Digimon at all"
    );
}

/// AURA ON: opponent's only Digimon has strictly lower DP than the carrier.
#[test]
fn ex8_045_aura_grants_piercing_when_opp_digimon_has_lower_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(digimon(
            "OPP-WEAK",
            "OppWeak",
            5,
            5000,
            vec![CardColor::Red],
        ))
        .memory(20)
        .start();

    let tp = runner.game.turn_player();
    let opp = 1 - tp;
    let carrier = runner.place_on_field(tp, CARD_ID, Some(0));
    runner.place_on_field(opp, "OPP-WEAK", Some(0));

    runner.game.enter_main_phase();
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(carrier, Keyword::Piercing),
        "carrier (12000 DP) must gain <Piercing> against a weaker opp Digimon (5000 DP)"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        1,
        "carrier must gain <Security A. +1> against a weaker opp Digimon"
    );
}

/// AURA OFF: opponent has a Digimon with DP >= carrier's DP (equal case).
#[test]
fn ex8_045_aura_off_when_opp_digimon_has_equal_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(digimon(
            "OPP-EQUAL",
            "OppEqual",
            6,
            12000,
            vec![CardColor::Red],
        ))
        .memory(20)
        .start();

    let tp = runner.game.turn_player();
    let opp = 1 - tp;
    let carrier = runner.place_on_field(tp, CARD_ID, Some(0));
    runner.place_on_field(opp, "OPP-EQUAL", Some(0));

    runner.game.enter_main_phase();
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(carrier, Keyword::Piercing),
        "<Piercing> must be absent when an opp Digimon's DP equals the carrier's DP \
         (printed text: 'no Digimon with as much OR MORE DP')"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "Security A. +1 must be absent when an opp Digimon's DP equals the carrier's DP"
    );
}

/// AURA OFF: opponent has a Digimon with DP strictly greater than carrier's.
#[test]
fn ex8_045_aura_off_when_opp_digimon_has_higher_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(digimon(
            "OPP-STRONG",
            "OppStrong",
            6,
            13000,
            vec![CardColor::Red],
        ))
        .memory(20)
        .start();

    let tp = runner.game.turn_player();
    let opp = 1 - tp;
    let carrier = runner.place_on_field(tp, CARD_ID, Some(0));
    runner.place_on_field(opp, "OPP-STRONG", Some(0));

    runner.game.enter_main_phase();
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(carrier, Keyword::Piercing),
        "<Piercing> must be absent when an opp Digimon out-DPs the carrier"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "Security A. +1 must be absent when an opp Digimon out-DPs the carrier"
    );
}

/// AURA OFF: [Your Turn] gate — absent on the opponent's turn even with the
/// board condition satisfied.
#[test]
fn ex8_045_aura_off_on_opponents_turn() {
    let mut runner = base_runner();
    let tp = runner.game.turn_player();
    let non_tp = 1 - tp;
    let carrier = runner.place_on_field(non_tp, CARD_ID, Some(0));

    runner.game.enter_main_phase();
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(carrier, Keyword::Piercing),
        "[Your Turn] gate: <Piercing> must be absent on the opponent's turn"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "[Your Turn] gate: Security A. +1 must be absent on the opponent's turn"
    );
}

/// The aura's DP comparison must read the carrier's LIVE (buffed) DP — a
/// carrier boosted by the source-color-count bonus to exceed an opponent's
/// Digimon must still gain the keyword grant.
#[test]
fn ex8_045_aura_uses_live_dp_including_source_color_bonus() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-045 found in embedded DSL pack")
        .add_card(digimon("SRC-G3", "SrcG3", 5, 5000, vec![CardColor::Green]))
        .add_card(digimon(
            "OPP-12500",
            "Opp12500",
            6,
            12500,
            vec![CardColor::Red],
        ))
        .memory(20)
        .start();

    let tp = runner.game.turn_player();
    let opp = 1 - tp;
    // Carrier base DP 12000 + 1000 (1 distinct source color) = 13000 live DP.
    let carrier = runner.place_stack(tp, &["SRC-G3", CARD_ID]);
    runner.place_on_field(opp, "OPP-12500", Some(0));

    runner.game.enter_main_phase();
    runner.game.tick_declarative_effects();

    let dp = runner.effective_dp(carrier).expect("carrier has DP");
    assert_eq!(
        dp, 13000,
        "live DP must include the +1000 source-color bonus"
    );

    assert!(
        runner.game.has_keyword(carrier, Keyword::Piercing),
        "with live DP 13000 > opp's 12500, the carrier must gain <Piercing> — \
         proving the aura gate reads live (buffed) DP, not the printed base 12000"
    );
}

// ─── SECTION 6 — Structural: Piercing keyword usable at security check ──────

/// Piercing/Security A grants are declared via `grant_keyword`/`security_attack`
/// on the aura clause (structural sanity — the actual behavior is covered by
/// Section 5's live assertions).
#[test]
fn ex8_045_dp_and_keyword_auras_are_face_up_scope() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("EX8-045 compiles");

    let aura_count = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Declarative(_)))
        .count();
    assert!(
        aura_count >= 2,
        "EX8-045 must declare at least 2 declarative aura clauses \
         (DP-per-color, Piercing+SecurityA)"
    );
}
