//! BT21-040 Agumon — Digimon, Lv.3, Yellow/Red, DP 1000, Cost 3.
//! Traits: Dinosaur/Hero. Attribute: Vaccine.
//!
//! # Card text (data/card_bundles/BT21-040.md — official Bandai DB; confirmed
//! against the card image)
//!
//! [Digivolve box / xros_req] [Koromon]/Lv.2 w/[Hero] trait: Cost 0
//!
//! [Your Turn] While your opponent has a level 6 or higher Digimon or you
//! have 3 or more [Hero] trait Tamers with different names, this Digimon may
//! digivolve into [ShineGreymon] in the hand for a digivolution cost of 4,
//! ignoring digivolution requirements.
//!
//! Inherited Effect:
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! Standard evo_costs: Yellow Lv.2, cost 1 (cards.json).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Yellow/BT21_040.cs
//!
//! # Clause decomposition / DCGO mapping
//!
//! Clause 1 — Alt digivolution xros_req box (DCGO EffectTiming.None,
//!   region "Alternative Digivolution Condition",
//!   AddSelfDigivolutionRequirementStaticEffect):
//!   PermanentCondition = TopCard.EqualsCardName("Koromon") ||
//!     (TopCard.HasHeroTraits && TopCard.IsLevel2);
//!   digivolutionCost: 0, ignoreDigivolutionRequirement: false.
//!   -> `alt_paths: kind: digivolve, from: { any_of: [name_is: Koromon,
//!   all_of: [level_eq: 2, trait_has: Hero]] }, cost: 0` (BT22-008 idiom,
//!   Koromon-OR-Lv2-trait, swap CS -> Hero).
//!
//! Clause 2 — [Your Turn] warp into [ShineGreymon] (DCGO EffectTiming.None,
//!   region "Your Turn", AddSelfDigivolutionRequirementStaticEffect):
//!   Condition = IsOwnerTurn && self-on-field &&
//!     (opp has Lv6+ Digimon || own Hero-Tamer-count >= 3).
//!   PermanentCondition = target == card.PermanentOfThisCard() (source=self).
//!   CardCondition = hand card named "ShineGreymon".
//!   digivolutionCost: 4, ignoreDigivolutionRequirement: true.
//!
//!   IMPORTANT (source priority: printed text overrides DCGO): DCGO's
//!   Hero-Tamer disjunct is a bare `Count(...) >= 3` — it does NOT check for
//!   distinct names at all. The PRINTED text requires "3 or more [Hero] trait
//!   Tamers with DIFFERENT NAMES". We follow the printed text and use
//!   `distinct_named_count_gte` (G-DSL-DISTINCT-NAMED-PERMANENT-COUNT,
//!   authored with BT21-040 as its driver card) — same-named duplicate Hero
//!   Tamers must NOT count toward the threshold. DCGO's incidental
//!   `LibraryCards.Count >= 1` guard has no basis in the printed card text
//!   and is not replicated.
//!
//!   -> `alt_paths: kind: digivolve, direction: into` (ST20-10 precedent),
//!   `from: { zone: [hand], of: you, kind: digimon, name_is: "ShineGreymon" }`,
//!   `cost: 4`, `ignore_requirements: true`,
//!   `condition: all_of [your_turn: true, any_of [
//!     any_permanent { of: opponent, zone: [battle_area], kind: digimon,
//!       level_gte: 6 },
//!     distinct_named_count_gte { of: you, n: 3,
//!       filter: { kind: tamer, trait_has: Hero } } ] ]`.
//!
//! Clause 3 — Inherited [Your Turn] +2000 DP self-aura (DCGO EffectTiming.
//!   None, region "Inherit", ChangeSelfDPStaticEffect(changeValue: 2000,
//!   isInheritedEffect: true, condition: IsOwnerTurn)). -> `scope: inherited
//!   / kind: aura / active_when: { your_turn: true } / target: {} /
//!   dp_modifier: 2000` (BT21-042 / EX9-012 idiom).
//!
//! # Patterns this test covers (hybrid §11 + positive-rules checklist)
//! - Structural: alt_paths shape (xros_req box + warp-into path) + inherited
//!   aura clause presence.
//! - Behavioral: warp digivolve via EACH disjunct independently, negative
//!   when neither disjunct holds, negative off-turn, negative wrong hand-card
//!   name, distinct-name dedup (printed text vs DCGO's under-implementation),
//!   event-log cost assertion (memory delta), inherited DP on/off turn.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathDirection, CompiledAltPathKind, CompiledClause, CompiledCost,
    CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, PlaySource, PlayerId};
use digimon_engine::permanent::PermanentHandle;

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// Build a runner with BT21-040 loaded from the embedded DSL pack.
fn agumon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 must be in embedded DSL pack")
        .memory(20)
        .start()
}

/// A Digimon named "ShineGreymon" in hand with EMPTY evo_costs so the
/// standard rules-based digivolve route cannot apply — a successful
/// digivolve onto BT21-040 then proves the `direction: into` warp alt-path
/// took over. Mirrors ST20-10's `make_wargreymon` idiom.
fn make_shinegreymon(id: &str) -> CardData {
    let mut c = make_test_card(id, "ShineGreymon");
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = 7;
    c.evo_costs = Vec::new();
    c
}

/// An opponent Digimon with an explicit level.
fn make_opp_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.dp = Some(3000);
    c
}

/// A Tamer with the Hero trait and an explicit distinct name.
fn make_hero_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.traits = vec!["Hero".to_string()];
    c
}

/// A non-Hero Tamer (must not count toward the threshold).
fn make_plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

/// Build a runner with BT21-040 registered and stacked under a Lv.5 dummy
/// Digimon on player 0's battle area. Used by inherited DP tests.
fn runner_with_bt21_040_as_source() -> (DebugRunner, PermanentHandle) {
    // Use the real compiled BT21-040 registered under its real card_id so the
    // inherited aura clause (scoped by card) is exercised faithfully.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card({
            let mut carrier = make_test_card("CARRIER-LV5", "CarrierLv5");
            carrier.level = Some(5);
            carrier.dp = Some(9000);
            carrier
        })
        .memory(10)
        .start();

    let bt21_handle = runner.place_on_field(0, "BT21-040", Some(0));
    let carrier_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "CARRIER-LV5")
        .expect("CARRIER-LV5 in card_data");
    let next_idx = runner.game.next_card_index();
    let carrier_source = CardSource::new(carrier_data_idx, 0, next_idx);
    let turn = runner.game.turn_count;
    runner.game.players[0].battle_area[bt21_handle.index as usize]
        .digivolve(carrier_source, turn);

    (runner, bt21_handle)
}

/// Push a CardSource referencing `card_id` into player `p`'s hand.
fn push_to_hand(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

/// Find a card's index in a player's hand by card_id.
fn find_hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> Option<usize> {
    let data = &runner.game.card_data;
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(data) == card_id)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_040_compiles_in_embedded_pack() {
    let runner = agumon_runner();
    assert!(
        runner.compiled_card("BT21-040").is_some(),
        "BT21-040 must be present in embedded DSL pack"
    );
}

#[test]
fn bt21_040_card_metadata_matches_print() {
    let runner = agumon_runner();
    let card = runner.compiled_card("BT21-040").expect("BT21-040 compiles");

    assert_eq!(card.level, Some(3), "Agumon is Lv.3");
    assert_eq!(card.dp, Some(1000), "BT21-040 is DP 1000");
    assert_eq!(card.cost, Some(3), "BT21-040 costs 3 to play");
    let traits_lower: Vec<String> = card.traits.iter().map(|t| t.to_lowercase()).collect();
    assert!(
        traits_lower.iter().any(|t| t == "dinosaur"),
        "must have Dinosaur trait, got {:?}",
        card.traits
    );
    assert!(
        traits_lower.iter().any(|t| t == "hero"),
        "must have Hero trait, got {:?}",
        card.traits
    );
}

/// The YAML declares exactly TWO alt_paths: the xros_req box
/// (Koromon-OR-Lv2-Hero, cost 0) and the warp-into-ShineGreymon path
/// (`direction: into`, cost 4, ignore_requirements: true).
#[test]
fn bt21_040_has_two_alt_paths() {
    let card = agumon_runner()
        .compiled_card("BT21-040")
        .expect("BT21-040 compiles")
        .clone();
    assert_eq!(
        card.alt_paths.len(),
        2,
        "expected exactly 2 alt_paths (xros_req Koromon/Lv2-Hero + \
         warp-into-ShineGreymon); got {}",
        card.alt_paths.len()
    );
}

/// First alt_path: xros_req box — Koromon (any level/color) OR Lv.2 Hero
/// trait, cost 0, requirement-respecting (no ignore_requirements).
#[test]
fn bt21_040_first_alt_path_is_xros_req_koromon_or_lv2_hero_cost0() {
    let card = agumon_runner()
        .compiled_card("BT21-040")
        .expect("BT21-040 compiles")
        .clone();
    let path = &card.alt_paths[0];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.cost,
        Some(CompiledCost::Literal(0)),
        "xros_req alt-source path must be cost 0"
    );
    assert!(
        !path.ignore_requirements,
        "xros_req alt-source path must respect digivolution requirements per DCGO \
         (ignoreDigivolutionRequirement: false)"
    );
    let from = path
        .from
        .as_ref()
        .expect("xros_req path must carry a `from:` predicate");
    let pretty = format!("{from:?}");
    assert!(
        pretty.contains("Koromon"),
        "xros_req `from:` must mention Koromon somewhere; got {pretty}"
    );
    assert!(
        pretty.contains("Hero") || pretty.contains("hero"),
        "xros_req `from:` must mention Hero trait somewhere; got {pretty}"
    );
}

/// Second alt_path: the warp-into-ShineGreymon clause: `direction: into`,
/// cost 4, `ignore_requirements: true`, `from:` filters a ShineGreymon hand
/// card, and a `condition:` carrying the disjunctive [Your Turn] gate.
#[test]
fn bt21_040_second_alt_path_is_warp_into_shinegreymon() {
    let card = agumon_runner()
        .compiled_card("BT21-040")
        .expect("BT21-040 compiles")
        .clone();
    let path = &card.alt_paths[1];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.direction,
        CompiledAltPathDirection::Into,
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
        Some("ShineGreymon"),
        "warp path `from:` must filter the ShineGreymon hand card"
    );
    assert!(
        path.condition.is_some(),
        "warp path must carry a `condition:` (the [Your Turn] + disjunctive gate)"
    );
}

/// The condition on the warp alt_path must carry `distinct_named_count_gte`
/// (NOT a bare count) — this is the printed-text-over-DCGO faithfulness
/// requirement: "3 or more [Hero] trait Tamers with DIFFERENT NAMES".
#[test]
fn bt21_040_warp_condition_uses_distinct_named_count_not_bare_count() {
    let card = agumon_runner()
        .compiled_card("BT21-040")
        .expect("BT21-040 compiles")
        .clone();
    let path = &card.alt_paths[1];
    let condition = path
        .condition
        .as_ref()
        .expect("warp path must carry a condition");
    let pretty = format!("{condition:?}");
    assert!(
        pretty.contains("DistinctNamedCount") || pretty.contains("distinct_named_count"),
        "warp condition must use the distinct_named_count_gte predicate (printed text says \
         \"with different names\"; DCGO's bare Count(...) >= 3 under-implements this); got {pretty}"
    );
}

/// The inherited [Your Turn] +2000 DP self-aura must be present with the
/// right scope, dp_modifier, and active_when.
#[test]
fn bt21_040_has_inherited_your_turn_dp_aura() {
    let card = agumon_runner()
        .compiled_card("BT21-040")
        .expect("BT21-040 compiles")
        .clone();

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
        aura.expect("BT21-040 must have a Declarative::Aura clause (inherited +2000 DP)");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "the aura clause must have Inherited scope"
    );
    assert_eq!(dp, Some(2000), "the aura dp_modifier must be +2000 DP");
    assert!(
        active_when.is_some(),
        "the aura must declare active_when (your_turn: true)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: warp-into-ShineGreymon — opponent Lv6+ disjunct
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: on the controller's turn, with a Lv.6+ opponent Digimon on
/// field and a ShineGreymon in hand, the warp digivolve succeeds for cost 4,
/// ignoring digivolution requirements.
#[test]
fn bt21_040_warp_into_shinegreymon_via_opp_level6_disjunct() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(make_shinegreymon("SHINEGREY"))
        .add_card(make_opp_digimon("BIG-LVL", 6))
        .hand(0, &["SHINEGREY"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "BT21-040", Some(0));
    runner.place_on_field(1, "BIG-LVL", Some(0));
    let memory_before = runner.game.memory;

    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, agumon.index as usize, PlaySource::ByHand,),
        "warp digivolve must succeed when opponent has a Lv.6+ Digimon"
    );
    let top = runner.game.players[0].battle_area[agumon.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "SHINEGREY",
        "BT21-040's slot top card is now ShineGreymon"
    );
    assert_eq!(
        memory_before - runner.game.memory,
        4,
        "warp digivolve cost is 4"
    );
}

/// Negative: opponent Digimon below Lv.6 does not satisfy the level disjunct
/// on its own (paired with too few Hero Tamers below).
#[test]
fn bt21_040_warp_into_shinegreymon_blocked_when_opp_below_level6() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(make_shinegreymon("SHINEGREY"))
        .add_card(make_opp_digimon("SMALL-LVL", 5))
        .hand(0, &["SHINEGREY"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "BT21-040", Some(0));
    runner.place_on_field(1, "SMALL-LVL", Some(0));

    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, agumon.index as usize, PlaySource::ByHand,),
        "warp digivolve must FAIL when opponent's Digimon is below Lv.6 and no Hero-Tamer \
         disjunct is satisfied"
    );
    let top = runner.game.players[0].battle_area[agumon.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "BT21-040",
        "BT21-040 must still be the top card — no warp digivolve occurred"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: warp-into-ShineGreymon — distinct-named Hero-Tamer
//              disjunct (printed text vs DCGO under-implementation)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: 3 DISTINCT-named Hero Tamers on own field (no qualifying
/// opponent Digimon) satisfies the Tamer disjunct.
#[test]
fn bt21_040_warp_into_shinegreymon_via_three_distinct_hero_tamers() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(make_shinegreymon("SHINEGREY"))
        .add_card(make_hero_tamer("HERO-A", "Marcus"))
        .add_card(make_hero_tamer("HERO-B", "Thomas"))
        .add_card(make_hero_tamer("HERO-C", "Keenan"))
        .hand(0, &["SHINEGREY"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "BT21-040", Some(0));
    runner.place_on_field(0, "HERO-A", None);
    runner.place_on_field(0, "HERO-B", None);
    runner.place_on_field(0, "HERO-C", None);
    let memory_before = runner.game.memory;

    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, agumon.index as usize, PlaySource::ByHand,),
        "warp digivolve must succeed with 3 distinct-named Hero Tamers on field"
    );
    let top = runner.game.players[0].battle_area[agumon.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "SHINEGREY",
        "BT21-040's slot top card is now ShineGreymon"
    );
    assert_eq!(memory_before - runner.game.memory, 4, "warp cost is 4");
}

/// CRITICAL faithfulness test: this is the exact case DCGO gets WRONG. Two
/// Hero Tamer PERMANENTS that share the SAME name only count as ONE distinct
/// name under the printed text ("with different names"), even though DCGO's
/// bare `Count(...) >= 3` would count them as 2 separate Tamers. With only 2
/// distinct Hero-Tamer names on field (even if backed by 3 permanents) and no
/// qualifying opponent Digimon, the warp must FAIL.
#[test]
fn bt21_040_warp_blocked_when_hero_tamers_share_names_dcgo_undercounts() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(make_shinegreymon("SHINEGREY"))
        .add_card(make_hero_tamer("HERO-DUP-1", "Marcus"))
        .add_card(make_hero_tamer("HERO-DUP-2", "Marcus"))
        .add_card(make_hero_tamer("HERO-B", "Thomas"))
        .hand(0, &["SHINEGREY"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "BT21-040", Some(0));
    // 3 Hero Tamer PERMANENTS, but only 2 DISTINCT names (Marcus x2, Thomas).
    runner.place_on_field(0, "HERO-DUP-1", None);
    runner.place_on_field(0, "HERO-DUP-2", None);
    runner.place_on_field(0, "HERO-B", None);

    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, agumon.index as usize, PlaySource::ByHand,),
        "warp digivolve must FAIL: 3 Hero Tamer permanents but only 2 DISTINCT names \
         (printed text requires \"3 or more ... with different names\"; DCGO's bare count \
         would wrongly allow this)"
    );
    let top = runner.game.players[0].battle_area[agumon.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "BT21-040",
        "BT21-040 must still be the top card — no warp digivolve occurred"
    );
}

/// Negative: a non-Hero Tamer must not count toward the threshold, even
/// alongside 2 distinct Hero Tamers.
#[test]
fn bt21_040_warp_blocked_non_hero_tamer_does_not_count() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(make_shinegreymon("SHINEGREY"))
        .add_card(make_hero_tamer("HERO-A", "Marcus"))
        .add_card(make_hero_tamer("HERO-B", "Thomas"))
        .add_card(make_plain_tamer("PLAIN-T"))
        .hand(0, &["SHINEGREY"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "BT21-040", Some(0));
    runner.place_on_field(0, "HERO-A", None);
    runner.place_on_field(0, "HERO-B", None);
    runner.place_on_field(0, "PLAIN-T", None);

    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, agumon.index as usize, PlaySource::ByHand,),
        "warp digivolve must FAIL: only 2 distinct Hero Tamers + 1 non-Hero Tamer \
         (non-Hero Tamer must not count toward the threshold)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Behavioral: warp negative gates (neither disjunct / off-turn /
//              wrong hand-card name / non-BT21-040 base)
// ═══════════════════════════════════════════════════════════════════════════

/// Negative: neither disjunct satisfied at all (low-level opponent Digimon,
/// zero Hero Tamers) — the warp must fail entirely.
#[test]
fn bt21_040_warp_blocked_when_neither_disjunct_satisfied() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(make_shinegreymon("SHINEGREY"))
        .add_card(make_opp_digimon("SMALL-LVL", 3))
        .hand(0, &["SHINEGREY"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "BT21-040", Some(0));
    runner.place_on_field(1, "SMALL-LVL", Some(0));

    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, agumon.index as usize, PlaySource::ByHand,),
        "warp digivolve must FAIL when neither disjunct is satisfied"
    );
}

/// Negative: the warp clause is gated on `[Your Turn]`. On the opponent's
/// turn the warp must NOT be available even if the opp-level disjunct holds.
#[test]
fn bt21_040_warp_blocked_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(make_shinegreymon("SHINEGREY"))
        .add_card(make_opp_digimon("BIG-LVL", 6))
        .hand(0, &["SHINEGREY"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "BT21-040", Some(0));
    runner.place_on_field(1, "BIG-LVL", Some(0));

    // Flip to the opponent's turn — `your_turn: true` in the condition fails.
    runner.game.turn_player_idx = 1;

    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, agumon.index as usize, PlaySource::ByHand,),
        "warp digivolve must FAIL on the opponent's turn ([Your Turn] gate)"
    );
}

/// Negative: a hand card that is NOT named "ShineGreymon" must not be picked
/// up by the warp path even when the disjunctive gate is satisfied.
#[test]
fn bt21_040_warp_blocked_for_non_shinegreymon_hand_card() {
    let mut other = make_test_card("OTHER-CARD", "SomeOtherDigimon");
    other.level = Some(6);
    other.dp = Some(10000);
    other.evo_costs = Vec::new();

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(other)
        .add_card(make_opp_digimon("BIG-LVL", 6))
        .hand(0, &["OTHER-CARD"])
        .memory(20)
        .start();

    let agumon = runner.place_on_field(0, "BT21-040", Some(0));
    runner.place_on_field(1, "BIG-LVL", Some(0));

    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, agumon.index as usize, PlaySource::ByHand,),
        "warp digivolve must FAIL: hand card is not named ShineGreymon"
    );
}

/// Sanity: the warp path only resolves while BT21-040 is the digivolve base
/// permanent (`direction: into` is keyed on the base card's identity).
#[test]
fn bt21_040_warp_only_applies_to_bt21_040_base() {
    let plain_lv5 = {
        let mut c = make_test_card("PLAIN-LV5", "PlainLv5");
        c.level = Some(5);
        c.dp = Some(9000);
        c.evo_costs = Vec::new();
        c
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(make_shinegreymon("SHINEGREY"))
        .add_card(plain_lv5)
        .add_card(make_opp_digimon("BIG-LVL", 6))
        .hand(0, &["SHINEGREY"])
        .memory(20)
        .start();

    // Place a plain Lv.5 Digimon (NOT BT21-040) as the digivolve base.
    let plain = runner.place_on_field(0, "PLAIN-LV5", Some(0));
    runner.place_on_field(1, "BIG-LVL", Some(0));

    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, plain.index as usize, PlaySource::ByHand,),
        "the warp alt-path is registered on BT21-040 — a non-BT21-040 base must not \
         pick it up (ShineGreymon has empty evo_costs so the rules route also fails)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Behavioral: xros_req box (Koromon-OR-Lv2-Hero, cost 0)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: a card literally named "Koromon" (any level/color, empty
/// evo_costs so the standard route can't apply) can digivolve into BT21-040
/// for cost 0 via the xros_req alt-path.
#[test]
fn bt21_040_xros_req_from_koromon_named_card() {
    let mut koromon = make_test_card("KOROMON-CARD", "Koromon");
    koromon.level = Some(2);
    koromon.dp = None;
    koromon.card_kind = CardKind::Digimon;
    koromon.evo_costs = Vec::new();

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(koromon)
        .hand(0, &["BT21-040"])
        .memory(20)
        .start();

    let base = runner.place_on_field(0, "KOROMON-CARD", Some(0));
    let memory_before = runner.game.memory;

    let hand_idx = find_hand_index(&runner, 0, "BT21-040").expect("BT21-040 in hand");
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand,),
        "BT21-040 must be able to digivolve from a card named Koromon via the xros_req path"
    );
    assert_eq!(
        memory_before - runner.game.memory,
        0,
        "xros_req digivolve from Koromon costs 0"
    );
}

/// Positive: a Lv.2 Hero-trait Digimon (any color, NOT named Koromon, empty
/// evo_costs) can also digivolve into BT21-040 for cost 0.
#[test]
fn bt21_040_xros_req_from_lv2_hero_trait_card() {
    let mut hero_lv2 = make_test_card("HERO-LV2", "SomeHeroRookie");
    hero_lv2.level = Some(2);
    hero_lv2.dp = Some(500);
    hero_lv2.card_kind = CardKind::Digimon;
    hero_lv2.traits = vec!["Hero".to_string()];
    hero_lv2.evo_costs = Vec::new();

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(hero_lv2)
        .hand(0, &["BT21-040"])
        .memory(20)
        .start();

    let base = runner.place_on_field(0, "HERO-LV2", Some(0));
    let memory_before = runner.game.memory;

    let hand_idx = find_hand_index(&runner, 0, "BT21-040").expect("BT21-040 in hand");
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand,),
        "BT21-040 must be able to digivolve from any Lv.2 Hero-trait Digimon via the xros_req path"
    );
    assert_eq!(
        memory_before - runner.game.memory,
        0,
        "xros_req digivolve from Lv.2 Hero-trait card costs 0"
    );
}

/// Negative: a Lv.2 card WITHOUT the Hero trait and NOT named Koromon must
/// not satisfy the xros_req path (and has empty evo_costs so no other route
/// applies either).
#[test]
fn bt21_040_xros_req_blocked_for_non_hero_non_koromon_lv2() {
    let mut plain_lv2 = make_test_card("PLAIN-LV2", "PlainRookie");
    plain_lv2.level = Some(2);
    plain_lv2.dp = Some(500);
    plain_lv2.card_kind = CardKind::Digimon;
    plain_lv2.evo_costs = Vec::new();

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-040")
        .expect("BT21-040 in pack")
        .add_card(plain_lv2)
        .hand(0, &["BT21-040"])
        .memory(20)
        .start();

    let base = runner.place_on_field(0, "PLAIN-LV2", Some(0));

    let hand_idx = find_hand_index(&runner, 0, "BT21-040").expect("BT21-040 in hand");
    assert!(
        !runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand,),
        "BT21-040 must NOT digivolve from a plain non-Hero, non-Koromon Lv.2 card \
         with empty evo_costs"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — Inherited [Your Turn] +2000 DP
// ═══════════════════════════════════════════════════════════════════════════

/// When BT21-040 is a digivolution source under a carrier on the
/// controller's own turn, `source_dp_contribution` must return +2000 for its
/// slot.
#[test]
fn bt21_040_inherited_dp_active_on_your_turn() {
    let (runner, carrier_handle) = runner_with_bt21_040_as_source();
    let bt21_src_idx = 0;

    assert_eq!(runner.turn_player(), 0, "precondition: it is P0's turn");

    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, bt21_src_idx);

    assert_eq!(
        contribution, 2000,
        "BT21-040's inherited +2000 DP must contribute on the controller's own turn; got {}",
        contribution
    );
}

/// On the opponent's turn, the [Your Turn] gate must suppress the DP buff.
#[test]
fn bt21_040_inherited_dp_inactive_on_opponents_turn() {
    let (mut runner, carrier_handle) = runner_with_bt21_040_as_source();
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: it is now P1's turn");

    let bt21_src_idx = 0;
    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, bt21_src_idx);

    assert_eq!(
        contribution, 0,
        "BT21-040's inherited +2000 DP must NOT contribute on opponent's turn; got {}",
        contribution
    );
}
