//! ST20-10 Agumon — Lv.3, Black, Reptile / ADVENTURE / Hero, DP 1000, Cost 3.
//!
//! # Card text (data/cards.json)
//!
//! ```text
//! [Your Turn] While your opponent has a Digimon with 10000 DP or more, or
//! your Tamers have 3 or more total colors, this Digimon can digivolve into
//! [WarGreymon] in the hand for a digivolution cost of 4, ignoring
//! digivolution requirements.
//!
//! Inherited:  ＜Reboot＞ (Unsuspend this Digimon during your opponent's
//!                        unsuspend phase.)
//!
//! [Digivolve box] Lv.2 w/[ADVENTURE] / [Hero] trait: Cost 0
//! [evo_costs]     Lv.2 Black: Cost 0
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST20/Black/ST20_10.cs
//!
//! DCGO declares three EffectTiming.None static effects:
//!   1. AddSelfDigivolutionRequirementStaticEffect — alt-source: Lv.2 with
//!      HasAdventureTraits || HasHeroTraits, cost 0, requirement-respecting.
//!   2. AddSelfDigivolutionRequirementStaticEffect — "Warp Digivolution":
//!      ownerTurn + on-field gate, opp-DP-≥-10000 OR own-tamer-colours-≥-3
//!      gate, source = self, target hand card name = "WarGreymon", cost 4,
//!      ignore_requirements = true.
//!   3. RebootSelfStaticEffect — inherited <Reboot>.
//!
//! # Patterns this test file covers
//!
//! | Clause                                              | Pattern (RUST_DSL_TEST_API.md §4.3) |
//! |-----------------------------------------------------|--------------------------------------|
//! | Standard digivolve (Lv2 Black / cost 0)             | Structural — alt_paths kind: digivolve |
//! | Alt-source xros_req (Lv2 ADVENTURE/Hero / cost 0)   | Structural — alt_paths kind: digivolve |
//! | Inherited <Reboot>                                  | H7 inherited grant_keyword Reboot (stack-walk) |
//!
//! # Known gaps blocking the warp-digivolve clause
//!
//! Three independent DSL gaps prevent the [Your Turn] warp-into-WarGreymon
//! clause from being expressed today; the YAML's clause-2 comment block
//! contains the full discussion. Tests covering the warp behaviour are
//! `#[ignore]`-tagged below.
//!
//! - **G-ALT-PATH-DIRECTION-INTO** (NEW) — `AltPathSpec` describes sources
//!   that can digivolve INTO the carrier card. There is no inverse form for
//!   "this card grants ITSELF the ability to digivolve into card X in hand."
//! - ~~**G-ALT-PATH-CONDITION**~~ RESOLVED 2026-05-15 — `AltPathSpec`
//!   now carries a `condition:` field (consumed by the Digivolve route
//!   in `dna_digivolve.rs`). The "while opp has 10000+ DP Digimon OR
//!   your Tamers have 3+ colours" gate can now be attached, but the
//!   inverse-direction blocker below still prevents authoring the
//!   warp-into clause as a whole.
//! - **G-DSL-DISTINCT-TAMER-COLORS** (NEW; sibling of
//!   G-DSL-DISTINCT-TAMER-COLORS-FORMULA filed for BT21-102) — there is no
//!   BoolPredicate leaf for "you have N or more distinct Tamer colours on
//!   field"; `distinct_colors_count` is only available inside `FormulaSpec::per`.
//!   The Tamer-colour disjunct of the gate cannot be expressed in any
//!   predicate. (The 10000+ DP disjunct is also impacted by the existing
//!   G-PRED-DP-LTE engine gap — `dp_gte` is parsed but not evaluated;
//!   precedent: EX10-010.)

#![allow(dead_code, unused_imports, unused_variables)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;

use dsl_card_data::compiled;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a runner with ST20-10 (Agumon) loaded from the embedded DSL pack.
fn agumon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST20-10")
        .expect("ST20-10 must be in embedded DSL pack")
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Card identity / structural assertions
// ═══════════════════════════════════════════════════════════════════════════

/// ST20-10 is a Lv.3 black Digimon, DP 1000, cost 3, Reptile / ADVENTURE / Hero.
#[test]
fn st20_10_identity_matches_printed_card() {
    let card = compiled("ST20-10");
    assert_eq!(card.card, "ST20-10");
    assert_eq!(card.name, "Agumon");
    assert_eq!(card.level, Some(3), "level must be 3");
    assert_eq!(card.dp, Some(1000), "DP must be 1000");
    assert_eq!(card.cost, Some(3), "play cost must be 3");
    let traits_lower: Vec<String> = card.traits.iter().map(|t| t.to_lowercase()).collect();
    assert!(
        traits_lower.iter().any(|t| t == "reptile"),
        "must have Reptile trait, got {:?}",
        card.traits
    );
    assert!(
        traits_lower.iter().any(|t| t == "adventure"),
        "must have ADVENTURE trait, got {:?}",
        card.traits
    );
    assert!(
        traits_lower.iter().any(|t| t == "hero"),
        "must have Hero trait, got {:?}",
        card.traits
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Alt-path structure (standard + xros_req)
// ═══════════════════════════════════════════════════════════════════════════

/// The YAML must declare exactly TWO alt_paths today:
///   1. Standard digivolve from Lv2 Black, cost 0 (printed evo_costs).
///   2. Alt-source from Lv2 with ADVENTURE or Hero trait, cost 0 (xros_req box).
///
/// A third, "warp into WarGreymon" alt_path is BLOCKED on stacked DSL gaps
/// (see file header). When G-ALT-PATH-DIRECTION-INTO +
/// G-DSL-DISTINCT-TAMER-COLORS both close, a third alt_path will be added
/// and this assertion bumped to 3. (G-ALT-PATH-CONDITION was RESOLVED
/// 2026-05-15 — no longer a blocker.)
#[test]
fn st20_10_has_two_alt_paths_today() {
    let card = compiled("ST20-10");
    assert_eq!(
        card.alt_paths.len(),
        2,
        "expected exactly 2 alt_paths (standard Lv2 Black + xros_req Lv2 ADVENTURE/Hero) today; \
         got {}. Note: warp-into-WarGreymon clause is blocked on \
         G-ALT-PATH-DIRECTION-INTO / G-DSL-DISTINCT-TAMER-COLORS (G-ALT-PATH-CONDITION resolved 2026-05-15).",
        card.alt_paths.len()
    );
}

/// First alt_path is the standard digivolve from Lv.2 Black at cost 0.
#[test]
fn st20_10_first_alt_path_is_lv2_black_cost0_digivolve() {
    let card = compiled("ST20-10");
    let path = &card.alt_paths[0];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.cost,
        Some(CompiledCost::Literal(0)),
        "standard digivolve path must be cost 0"
    );
    assert!(
        !path.ignore_requirements,
        "standard digivolve path must respect digivolution requirements"
    );
    let from = path
        .from
        .as_ref()
        .expect("standard digivolve path must carry a `from:` predicate");
    assert_eq!(from.level_eq, Some(2), "standard path must be from Lv.2");
}

/// Second alt_path is the xros_req box: Lv.2 with ADVENTURE OR Hero trait, cost 0.
/// Per DCGO ST20_10.cs region "Alternate Digivolution Requirement", this path
/// respects digivolution requirements (`ignoreDigivolutionRequirement: false`),
/// only the trait/level filter replaces the standard colour gate.
#[test]
fn st20_10_second_alt_path_is_xros_req_lv2_adventure_or_hero_cost0() {
    let card = compiled("ST20-10");
    let path = &card.alt_paths[1];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.cost,
        Some(CompiledCost::Literal(0)),
        "xros_req alt-source path must be cost 0"
    );
    assert!(
        !path.ignore_requirements,
        "xros_req alt-source path must respect digivolution requirements per DCGO"
    );
    // The `from:` filter must accept Lv.2 ADVENTURE and Lv.2 Hero (any colour).
    // We don't assert the inner any_of shape exhaustively — both branches must
    // be present in the predicate tree. Sibling pattern: AD1-001 alt-digivolve.
    let from = path
        .from
        .as_ref()
        .expect("xros_req path must carry a `from:` predicate");
    let pretty = format!("{from:?}");
    assert!(
        pretty.contains("ADVENTURE")
            || pretty.contains("Adventure")
            || pretty.contains("adventure"),
        "xros_req `from:` must mention ADVENTURE trait somewhere; got {pretty}"
    );
    assert!(
        pretty.contains("Hero") || pretty.contains("hero"),
        "xros_req `from:` must mention Hero trait somewhere; got {pretty}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Inherited <Reboot> (declarative grant_keyword)
// ═══════════════════════════════════════════════════════════════════════════

/// Inherited Reboot grant_keyword clause must be present with scope: inherited.
#[test]
fn st20_10_has_inherited_reboot_grant_keyword() {
    let card = compiled("ST20-10");
    let inherited_reboot = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword.eq_ignore_ascii_case("Reboot")
        )
    });
    assert!(
        inherited_reboot,
        "ST20-10 must declare an inherited-scope GrantKeyword Reboot clause"
    );
}

/// Stack-walk runtime check: when ST20-10 sits as a digivolution source under
/// a higher-level carrier, the carrier's `has_keyword(Reboot)` must be true.
///
/// Mirrors the ST1-07 pattern (st1_07_inherited_security_attack_plus_grants_to_carrier_via_stack):
/// place ST20-10, snapshot its CardSource, then place a generic Lv.4+ carrier
/// at the same slot and inject ST20-10's CardSource beneath it. The carrier
/// must inherit Reboot via game.has_keyword stack-walk (Batch 7 mechanism).
#[test]
fn st20_10_inherited_reboot_grants_to_carrier_via_stack() {
    let lv4_card = make_test_card("PLAIN-LV4", "PlainLv4");

    let mut runner = DebugRunner::builder()
        .dsl_card("ST20-10")
        .expect("ST20-10 in pack")
        .add_card(lv4_card)
        .memory(10)
        .start();

    // Place ST20-10 at slot 0 first.
    let _agumon = runner.place_on_field(0, "ST20-10", Some(0));

    // Snapshot ST20-10's CardSource before overwriting the slot.
    let agumon_source = {
        let game = runner.game_mut();
        game.players[0].battle_area[0].top_card().clone()
    };

    // Place PLAIN-LV4 at slot 0 (replaces ST20-10).
    let carrier = runner.place_on_field(0, "PLAIN-LV4", Some(0));

    // Inject ST20-10's CardSource as a digivolution source under the carrier.
    {
        let game = runner.game_mut();
        game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .insert(0, agumon_source);
    }

    // Carrier must inherit Reboot from ST20-10 in its stack.
    assert!(
        runner.game.has_keyword(carrier, Keyword::Reboot),
        "PLAIN-LV4 with ST20-10 as a digivolution source must inherit <Reboot>"
    );
}

/// Negative: a carrier without ST20-10 in its stack must NOT have Reboot.
#[test]
fn st20_10_inherited_reboot_absent_without_source() {
    let lv4_card = make_test_card("PLAIN-LV4", "PlainLv4");

    let mut runner = DebugRunner::builder()
        .dsl_card("ST20-10")
        .expect("ST20-10 in pack")
        .add_card(lv4_card)
        .memory(10)
        .start();

    // Place ONLY the carrier — no ST20-10 in its sources.
    let carrier = runner.place_on_field(0, "PLAIN-LV4", Some(0));

    assert!(
        !runner.game.has_keyword(carrier, Keyword::Reboot),
        "A carrier with no ST20-10 in its digivolution stack must NOT have Reboot"
    );
}

/// Mirror ST1-07 quirk: the engine's current stack-walk semantics treat
/// `scope: inherited` clauses as applying ONLY to a higher carrier above
/// the source, NOT to the source itself when face-up on top. This matches
/// the existing ST1-07 precedent (st1_07_top_card_inherited_security_attack_plus_does_not_apply_to_itself).
///
/// Note: real Digimon TCG rules read inherited effects as ALSO applying to
/// the printed-card itself when face-up. The own-scope runtime install path
/// is gated by G-DECLARATIVE-KEYWORD (EffectTiming::Declarative not yet
/// fired) — see ST1-07 comment block. For ST20-10, this means the
/// face-up-only Reboot grant on ST20-10 itself is currently a known
/// behavioural divergence shared with every other inherited-keyword card.
#[test]
fn st20_10_top_card_inherited_reboot_does_not_apply_to_itself_today() {
    let mut runner = agumon_runner();
    let handle = runner.place_on_field(0, "ST20-10", Some(0));

    assert!(
        !runner.game.has_keyword(handle, Keyword::Reboot),
        "ST20-10's inherited Reboot clause must not apply to itself while it is the top card \
         (matches ST1-07 precedent; tracked under G-DECLARATIVE-KEYWORD for face-up own-scope)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Warp-into-WarGreymon clause (BLOCKED on stacked DSL gaps)
// ═══════════════════════════════════════════════════════════════════════════
//
// All tests in this section are `#[ignore]`-tagged with the gap markers that
// must close before the warp clause can be authored on ST20-10's YAML.
// Scaffolding is left as comments; replace with real assertions once the
// gaps land.

/// Positive: when ST20-10 is on field on its controller's turn, an opponent
/// has a Digimon with DP ≥ 10000, and a WarGreymon is in hand, the player
/// can digivolve ST20-10 into that WarGreymon for cost 4, ignoring
/// digivolution requirements.
#[test]
#[ignore = "pending: G-ALT-PATH-DIRECTION-INTO + G-PRED-DP-LTE — \
            warp-into-WarGreymon path cannot be authored on ST20-10's YAML today"]
fn st20_10_warp_into_wargreymon_via_opp_dp_disjunct() {
    // Scaffolding (to be filled once gaps close):
    //
    //   let mut runner = DebugRunner::builder()
    //       .dsl_card("ST20-10")
    //       .dsl_card("ST20-11")          // WarGreymon target
    //       .add_card(make_opp_digimon("BIG-DP", 10000))
    //       .memory(20)
    //       .hand(0, &["ST20-11"])
    //       .start();
    //   let agumon = runner.place_on_field(0, "ST20-10", Some(0));
    //   let _opp_big = runner.place_on_field(1, "BIG-DP", Some(0));
    //   let memory_before = runner.memory();
    //
    //   // Action: activated_digivolve from ST20-10 INTO hand[ST20-11], cost 4.
    //   runner.activated_digivolve(agumon, /* hand_idx = */ 0)
    //       .expect("warp digivolve succeeds (opp ≥10000 DP gate satisfied)");
    //
    //   // Top card of ST20-10's slot is now WarGreymon; cost was 4.
    //   let top = runner.game.players[0].battle_area[0].top_card();
    //   assert_eq!(top.card_id, "ST20-11", "top card is WarGreymon");
    //   assert_eq!(memory_before - runner.memory(), 4, "warp cost was 4");
    todo!("G-ALT-PATH-DIRECTION-INTO + G-PRED-DP-LTE must land first");
}

/// Positive: same as above, but using the Tamer-colour disjunct (3+ distinct
/// Tamer colours on own field, opponent has no big Digimon).
#[test]
#[ignore = "pending: G-ALT-PATH-DIRECTION-INTO + G-DSL-DISTINCT-TAMER-COLORS"]
fn st20_10_warp_into_wargreymon_via_tamer_colours_disjunct() {
    // Scaffolding: place ST20-10 + 3 Tamers of distinct colours; assert warp
    // succeeds. Requires the 3-tamer-colour BoolPredicate (G-DSL-DISTINCT-TAMER-COLORS).
    todo!("G-ALT-PATH-DIRECTION-INTO + G-DSL-DISTINCT-TAMER-COLORS must land first");
}

/// Negative: when neither disjunct is satisfied (no opp ≥10000 Digimon AND
/// fewer than 3 Tamer colours), the warp path must NOT be available.
#[test]
#[ignore = "pending: G-ALT-PATH-DIRECTION-INTO"]
fn st20_10_warp_into_wargreymon_blocked_when_neither_disjunct_satisfied() {
    // Scaffolding: ST20-10 + WarGreymon in hand + no qualifying opponent or
    // Tamers. Assert the activated_digivolve action is NOT in the action mask.
    todo!("G-ALT-PATH-DIRECTION-INTO must land first");
}

/// Negative: warp clause is gated on `[Your Turn]`. On the opponent's turn
/// the warp must NOT be available even if both disjuncts are satisfied.
#[test]
#[ignore = "pending: G-ALT-PATH-DIRECTION-INTO"]
fn st20_10_warp_into_wargreymon_blocked_on_opponents_turn() {
    todo!("G-ALT-PATH-DIRECTION-INTO must land first");
}

/// Negative: warp clause requires self to be on field (DCGO
/// `IsExistOnBattleArea`). When ST20-10 is in hand (not on field), the warp
/// alt-path must not be exposed via has-card-in-hand alone.
#[test]
#[ignore = "pending: G-ALT-PATH-DIRECTION-INTO"]
fn st20_10_warp_into_wargreymon_requires_self_on_field() {
    todo!("G-ALT-PATH-DIRECTION-INTO must land first");
}
