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
//! | Warp into WarGreymon (`direction: into`, cost 4)    | Behavioral — alt-path warp digivolve |
//! | Inherited <Reboot>                                  | H7 inherited grant_keyword Reboot (stack-walk) |
//!
//! # Resolved DSL gaps (warp-digivolve clause now authored)
//!
//! The three DSL gaps that previously blocked the [Your Turn]
//! warp-into-WarGreymon clause are all RESOLVED:
//!
//! - **G-ALT-PATH-DIRECTION-INTO** (RESOLVED) — `AltPathSpec` carries a
//!   `direction:` field. `direction: into` flips the semantic so the
//!   alt-path lives on the SOURCE card and `from:` filters the destination
//!   hand card. ST20-10's third alt_path uses it.
//! - **G-ALT-PATH-CONDITION** (RESOLVED 2026-05-15) — `AltPathSpec` carries
//!   a `condition:` field, consumed by the Digivolve route in
//!   `dna_digivolve.rs`. The "[Your Turn] + (opp 10000+ DP OR 3+ Tamer
//!   colours)" gate attaches here.
//! - **G-DSL-DISTINCT-TAMER-COLORS** (RESOLVED 2026-05-19) — the
//!   `distinct_tamer_colors_gte` predicate leaf counts distinct colors
//!   across the observer's battle-area Tamers. The Tamer-colour disjunct
//!   uses it. The 10000+ DP disjunct uses `dp_gte` inside an
//!   `any_permanent` existential.

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

/// The YAML declares THREE alt_paths:
///   1. Standard digivolve from Lv2 Black, cost 0 (printed evo_costs).
///   2. Alt-source from Lv2 with ADVENTURE or Hero trait, cost 0 (xros_req box).
///   3. Warp digivolve INTO WarGreymon in hand, cost 4, ignoring requirements
///      (`direction: into`). All three blocking DSL gaps are RESOLVED —
///      G-ALT-PATH-DIRECTION-INTO, G-ALT-PATH-CONDITION, and
///      G-DSL-DISTINCT-TAMER-COLORS.
#[test]
fn st20_10_has_three_alt_paths() {
    let card = compiled("ST20-10");
    assert_eq!(
        card.alt_paths.len(),
        3,
        "expected exactly 3 alt_paths (standard Lv2 Black + xros_req Lv2 \
         ADVENTURE/Hero + warp-into-WarGreymon); got {}",
        card.alt_paths.len()
    );
}

/// The third alt_path is the warp-into-WarGreymon clause: `direction: into`,
/// cost 4, `ignore_requirements: true`, with a `from:` filter on the
/// WarGreymon hand card and a `condition:` carrying the disjunctive gate.
#[test]
fn st20_10_third_alt_path_is_warp_into_wargreymon() {
    let card = compiled("ST20-10");
    let path = &card.alt_paths[2];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.direction,
        digimon_dsl::compiled::CompiledAltPathDirection::Into,
        "warp path must use direction: into (alt-path lives on the source card)"
    );
    assert_eq!(
        path.cost,
        Some(CompiledCost::Literal(4)),
        "warp digivolve cost is 4"
    );
    assert!(
        path.ignore_requirements,
        "warp digivolve ignores digivolution requirements"
    );
    let from = path
        .from
        .as_ref()
        .expect("warp path must carry a `from:` hand-card filter");
    assert_eq!(
        from.name_is.as_deref(),
        Some("WarGreymon"),
        "warp path `from:` must filter the WarGreymon hand card"
    );
    assert!(
        path.condition.is_some(),
        "warp path must carry a `condition:` (the [Your Turn] + disjunctive gate)"
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
// Section 4 — Warp-into-WarGreymon clause (G-ALT-PATH-DIRECTION-INTO +
//             G-ALT-PATH-CONDITION + G-DSL-DISTINCT-TAMER-COLORS all RESOLVED)
// ═══════════════════════════════════════════════════════════════════════════

/// Build a synthetic Lv.6 "WarGreymon"-named Digimon with EMPTY evo_costs so
/// the standard rules-based digivolve route cannot apply — a successful
/// digivolve onto a Lv.3 ST20-10 then proves the `direction: into` warp
/// alt-path took over. Mirrors the `group7_alt_path_registration.rs` idiom.
fn make_wargreymon(id: &str) -> CardData {
    let mut c = make_test_card(id, "WarGreymon");
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = 7;
    c.evo_costs = Vec::new();
    c
}

/// Build an opponent Digimon with an explicit DP value.
fn make_opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// Build a Tamer with a single explicit color.
fn make_colored_tamer(id: &str, color: digimon_engine::enums::CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = digimon_engine::enums::CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c
}

/// Positive: when ST20-10 is on field on its controller's turn, an opponent
/// has a Digimon with DP ≥ 10000, and a WarGreymon is in hand, the player
/// can digivolve ST20-10 into that WarGreymon for cost 4, ignoring
/// digivolution requirements.
#[test]
fn st20_10_warp_into_wargreymon_via_opp_dp_disjunct() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST20-10")
        .expect("ST20-10 in pack")
        .add_card(make_wargreymon("WARGREYMON"))
        .add_card(make_opp_digimon("BIG-DP", 10000))
        .hand(0, &["WARGREYMON"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "ST20-10", Some(0));
    runner.place_on_field(1, "BIG-DP", Some(0));
    let memory_before = runner.game.memory;

    // Warp digivolve ST20-10 → WarGreymon (hand index 0). The opp ≥10000-DP
    // disjunct of the condition is satisfied.
    assert!(
        runner.game.digivolve_from_hand(
            0,
            0,
            agumon.index as usize,
            digimon_engine::enums::PlaySource::ByHand,
        ),
        "warp digivolve must succeed when opponent has a Digimon with DP ≥ 10000"
    );
    let top = runner.game.players[0].battle_area[agumon.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "WARGREYMON",
        "ST20-10's slot top card is now WarGreymon"
    );
    assert_eq!(
        memory_before - runner.game.memory,
        4,
        "warp digivolve cost is 4"
    );
}

/// Positive: same as above, but using the Tamer-colour disjunct — 3 Tamers of
/// distinct colours on own field, opponent has no qualifying Digimon.
#[test]
fn st20_10_warp_into_wargreymon_via_tamer_colours_disjunct() {
    use digimon_engine::enums::CardColor;

    let mut runner = DebugRunner::builder()
        .dsl_card("ST20-10")
        .expect("ST20-10 in pack")
        .add_card(make_wargreymon("WARGREYMON"))
        .add_card(make_colored_tamer("TAMER-RED", CardColor::Red))
        .add_card(make_colored_tamer("TAMER-BLUE", CardColor::Blue))
        .add_card(make_colored_tamer("TAMER-GREEN", CardColor::Green))
        .hand(0, &["WARGREYMON"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "ST20-10", Some(0));
    // 3 Tamers of distinct colors → distinct_tamer_colors_gte: 3 satisfied.
    runner.place_on_field(0, "TAMER-RED", None);
    runner.place_on_field(0, "TAMER-BLUE", None);
    runner.place_on_field(0, "TAMER-GREEN", None);
    let memory_before = runner.game.memory;

    assert!(
        runner.game.digivolve_from_hand(
            0,
            0,
            agumon.index as usize,
            digimon_engine::enums::PlaySource::ByHand,
        ),
        "warp digivolve must succeed when your Tamers have 3+ distinct colors"
    );
    let top = runner.game.players[0].battle_area[agumon.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "WARGREYMON",
        "ST20-10's slot top card is now WarGreymon"
    );
    assert_eq!(memory_before - runner.game.memory, 4, "warp cost is 4");
}

/// Negative: when neither disjunct is satisfied (opponent's Digimon is below
/// 10000 DP AND fewer than 3 distinct Tamer colours), the warp path must NOT
/// be available — the digivolve attempt fails.
#[test]
fn st20_10_warp_into_wargreymon_blocked_when_neither_disjunct_satisfied() {
    use digimon_engine::enums::CardColor;

    let mut runner = DebugRunner::builder()
        .dsl_card("ST20-10")
        .expect("ST20-10 in pack")
        .add_card(make_wargreymon("WARGREYMON"))
        .add_card(make_opp_digimon("SMALL-DP", 5000))
        .add_card(make_colored_tamer("TAMER-RED", CardColor::Red))
        .add_card(make_colored_tamer("TAMER-BLUE", CardColor::Blue))
        .hand(0, &["WARGREYMON"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "ST20-10", Some(0));
    runner.place_on_field(1, "SMALL-DP", Some(0)); // < 10000 DP
                                                   // Only 2 distinct Tamer colors — below the 3-colour threshold.
    runner.place_on_field(0, "TAMER-RED", None);
    runner.place_on_field(0, "TAMER-BLUE", None);

    assert!(
        !runner.game.digivolve_from_hand(
            0,
            0,
            agumon.index as usize,
            digimon_engine::enums::PlaySource::ByHand,
        ),
        "warp digivolve must FAIL when neither disjunct is satisfied \
         (opp Digimon < 10000 DP AND < 3 distinct Tamer colors)"
    );
    let top = runner.game.players[0].battle_area[agumon.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "ST20-10",
        "ST20-10 must still be the top card — no warp digivolve occurred"
    );
}

/// Negative: the warp clause is gated on `[Your Turn]`. On the opponent's
/// turn the warp must NOT be available even if the opp-DP disjunct holds.
#[test]
fn st20_10_warp_into_wargreymon_blocked_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST20-10")
        .expect("ST20-10 in pack")
        .add_card(make_wargreymon("WARGREYMON"))
        .add_card(make_opp_digimon("BIG-DP", 10000))
        .hand(0, &["WARGREYMON"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "ST20-10", Some(0));
    runner.place_on_field(1, "BIG-DP", Some(0));

    // Flip to the opponent's turn — `your_turn: true` in the condition fails.
    runner.game.turn_player_idx = 1;

    assert!(
        !runner.game.digivolve_from_hand(
            0,
            0,
            agumon.index as usize,
            digimon_engine::enums::PlaySource::ByHand,
        ),
        "warp digivolve must FAIL on the opponent's turn ([Your Turn] gate)"
    );
}

/// Sanity: the warp path only resolves while ST20-10 is the digivolve base
/// permanent. A digivolve attempt against a NON-ST20-10 base must not pick up
/// the warp route — the `direction: into` path is keyed on the base's card id.
#[test]
fn st20_10_warp_into_wargreymon_only_applies_to_st20_10_base() {
    let plain_lv5 = {
        let mut c = make_test_card("PLAIN-LV5", "PlainLv5");
        c.level = Some(5);
        c.dp = Some(9000);
        c.evo_costs = Vec::new();
        c
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("ST20-10")
        .expect("ST20-10 in pack")
        .add_card(make_wargreymon("WARGREYMON"))
        .add_card(plain_lv5)
        .add_card(make_opp_digimon("BIG-DP", 10000))
        .hand(0, &["WARGREYMON"])
        .memory(20)
        .start();

    // Place a plain Lv.5 Digimon (NOT ST20-10) as the digivolve base.
    let plain = runner.place_on_field(0, "PLAIN-LV5", Some(0));
    runner.place_on_field(1, "BIG-DP", Some(0));

    assert!(
        !runner.game.digivolve_from_hand(
            0,
            0,
            plain.index as usize,
            digimon_engine::enums::PlaySource::ByHand,
        ),
        "the warp alt-path is registered on ST20-10 — a non-ST20-10 base \
         must not pick it up (WarGreymon has empty evo_costs so the rules \
         route also fails)"
    );
}
