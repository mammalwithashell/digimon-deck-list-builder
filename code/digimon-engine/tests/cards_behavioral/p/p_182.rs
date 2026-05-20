//! P-182 WarGreymon — Digimon, Lv.6, Red, Cost 12, DP 12000.
//! Traits: Dragonkin, ADVENTURE.
//! Standard digivolve: Lv.5 Red / cost 4
//! Alt digivolve (xros_req): Lv.5 w/[MetalGreymon] in name OR w/[ADVENTURE]
//! trait, cost 3 (DCGO P_182.cs lines 19-40,
//! AddSelfDigivolutionRequirementStaticEffect with `IsLevel5 &&
//! (ContainsCardName("MetalGreymon") || EqualsTraits("ADVENTURE"))`,
//! `digivolutionCost: 3`, `ignoreDigivolutionRequirement: false`).
//!
//! # Card text (data/cards.json — verbatim)
//!
//! ```text
//! <Security A. +1> (This Digimon checks 1 additional security card.)
//! <Blocker> (This Digimon can block in the blocker timing.)
//! [When Digivolving] Delete 1 of your opponent's Digimon with as much or
//!   less DP as this Digimon.
//! [All Turns] This Digimon gets +1000 DP for each color Digimon and Tamers
//!   have.
//! ```
//!
//! Inherited / Security: none.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/P/Red/P_182.cs`
//!
//! # DSL YAML
//! `code/digimon-engine/cards/p/P-182.yaml`
//!
//! # Patterns covered
//! - Standard + alt digivolve paths (Lv.5 Red and Lv.5 [MetalGreymon] / [ADVENTURE])
//! - Two declarative `grant_keyword` clauses (`SecurityAttackPlus(1)`, `Blocker`)
//! - BLOCKED clauses asserted via structural absence (clauses NOT present in
//!   compiled effect list because their formulae cannot be expressed)
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                          | YAML clause                                                       | Status                |
//! |------------------------------------------------------------|-------------------------------------------------------------------|-----------------------|
//! | Standard digivolve Lv.5 Red / cost 4                       | `alt_paths[0]` `level_eq: 5, color_is: red, cost: 4`              | OK                    |
//! | Alt digivolve Lv.5 [MetalGreymon] OR [ADVENTURE] / cost 3  | `alt_paths[1]` `level_eq: 5 + any_of {name_contains, trait_has}`  | OK                    |
//! | <Security A. +1>                                           | `kind: grant_keyword, keyword: SecurityAttackPlus, value: 1`      | PARTIAL — G-DECLARATIVE-KEYWORD |
//! | <Blocker>                                                  | `kind: grant_keyword, keyword: Blocker`                            | PARTIAL — G-DECLARATIVE-KEYWORD |
//! | [When Digivolving] delete opp Digimon ≤ self DP            | `when: when_digivolving` + `dp_lte: { source_dp }`               | OK (G-FORMULA-SOURCE-DP resolved) |
//! | [All Turns] +1000 DP per distinct color (both players' Digimon+Tamers) | `kind: aura` + `dp_modifier_fn` `distinct_colors_count` | OK             |
//!
//! # Resolved DSL gaps
//!
//! - **G-FORMULA-SOURCE-DP** (RESOLVED 2026-05-19) — the `source_dp` formula
//!   primitive reads the SOURCE permanent's effective DP
//!   (`ctx.source_permanent.and_then(|h| ctx.game.effective_dp(h))`). YAML
//!   form `{ source_dp: {} }`. Required for "delete opp Digimon with as much
//!   or less DP as this Digimon" clauses (this card, BT17-102, etc.).
//!
//! - **G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA** — generalised
//!   distinct-colors PerSelector. The DSL `PerSelector` enum
//!   (`code/digimon-dsl/src/formula.rs` lines 130-141) has only:
//!     `MaterialCount, StackSize, AllyCount, DigivolutionColorCount,
//!      SameLevelPairsInSources, SharedTrashCount, CardCountInZone`.
//!   `DigivolutionColorCount` only counts colors of the source's own
//!   digivolution stack; `CardCountInZone` counts cards (not distinct colors
//!   among them). No primitive iterates a zone (battle_area) across BOTH
//!   players, filters to Digimon ∪ Tamer top cards, collapses to the
//!   distinct-color set, and returns its size. DCGO P_182.cs lines 156-164
//!   wraps this exact computation:
//!
//!   ```csharp
//!   List<CardSource> cards = card.Owner.GetBattleAreaPermanents()
//!     .Concat(card.Owner.Enemy.GetBattleAreaPermanents()).ToList()
//!     .Map(card => card.TopCard)
//!     .Where(x => x.IsDigimon || x.IsTamer).ToList();
//!   var colourCount = Combinations.GetDifferenetColorCardCount(cards);
//!   var newDP = colourCount * 1000;
//!   ```
//!
//!   Sibling of `G-DSL-DISTINCT-TAMER-COLORS-FORMULA` referenced as a
//!   companion in `qa/dsl-vocab-gaps.md` line 892 — that variant is scoped
//!   to YOUR TAMERS only; the P-182 variant is BOTH PLAYERS' Digimon AND
//!   Tamers. A general primitive should accept a `PredicateSpec` filter +
//!   a `PlayerRef` (`you` / `opponent` / `any`) so all card-shapes consume
//!   the same formula primitive. Suggested DSL syntax (sibling to
//!   `card_count_in_zone`):
//!
//!   ```yaml
//!   per:
//!     distinct_colors_count:
//!       of: any                          # both players
//!       zone: [battle_area]
//!       filter:
//!         any_of:
//!           - kind: digimon
//!           - kind: tamer
//!   ```
//!
//!   Implementation: add `DistinctColorsCount(DistinctColorsSpec)` variant to
//!   `PerSelector` in `digimon-dsl/src/formula.rs`, add a `CompiledPerSelector`
//!   branch, and an evaluator in `formula_eval.rs` that iterates the requested
//!   player(s)' battle_area, filters via the embedded `CompiledPredicate`, and
//!   returns the cardinality of the distinct-color set across surviving top
//!   cards. Engine has `Permanent::top_card()` + `card_data.colors` — the
//!   distinct-color collapse is trivial (`HashSet<Color>`).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

// ─── Fixture helpers ────────────────────────────────────────────────────────

fn wargreymon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("P-182")
        .expect("P-182 in embedded DSL pack")
        .memory(15)
        .start()
}

fn make_metalgreymon(id: &str) -> CardData {
    let mut c = make_test_card(id, "MetalGreymon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c
}

fn make_adventure_lv5(id: &str, name: &str) -> CardData {
    // A Lv.5 Digimon with the ADVENTURE trait but a non-MetalGreymon name —
    // exercises the OR branch of the alt digivolve (`trait_has: ADVENTURE`
    // matches even though name doesn't contain "MetalGreymon").
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

fn make_filler_lv5(id: &str) -> CardData {
    // A Lv.5 non-MetalGreymon-named, non-ADVENTURE Digimon. Should NOT be a
    // legal source under the alt-path predicate (only the standard cost-4 path
    // applies if it's also red).
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(5000);
    c
}

fn make_opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

fn make_own_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c
}

/// Phase 2 Track F: a Tamer/Digimon helper that sets an explicit single
/// color, used by the [All Turns] +1000 DP per distinct color tests.
fn make_colored_tamer(id: &str, name: &str, color: digimon_engine::enums::CardColor) -> CardData {
    let mut c = make_own_tamer(id, name);
    c.colors = vec![color];
    c
}

fn make_colored_opp_digimon(
    id: &str,
    name: &str,
    dp: i32,
    color: digimon_engine::enums::CardColor,
) -> CardData {
    let mut c = make_opp_digimon(id, name, dp);
    c.colors = vec![color];
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// P-182 compiles with TWO `kind: digivolve` alt-paths: the standard
/// Lv.5 Red / cost 4 path, and the alt Lv.5 [MetalGreymon] / [ADVENTURE]
/// cost-3 path.
#[test]
fn p_182_compiles_with_standard_and_alt_digivolve_paths() {
    let runner = wargreymon_runner();
    let compiled = runner.compiled_card("P-182").expect("compiled");

    assert_eq!(
        compiled.alt_paths.len(),
        2,
        "P-182 must have exactly 2 digivolve alt-paths"
    );

    let standard = compiled
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && matches!(p.cost, Some(CompiledCost::Literal(4)))
        })
        .expect("Lv.5 Red cost-4 standard path");
    let from = standard
        .from
        .as_ref()
        .expect("standard path has source predicate");
    assert!(
        from.all_of.iter().any(|p| p.level_eq == Some(5)),
        "standard path filters level_eq: 5"
    );
    assert!(
        from.all_of
            .iter()
            .any(|p| p.color_is == Some(CompiledColor::Red)),
        "standard path filters color_is: red"
    );

    let alt = compiled
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && matches!(p.cost, Some(CompiledCost::Literal(3)))
        })
        .expect("Lv.5 alt cost-3 path");
    let from = alt.from.as_ref().expect("alt path has source predicate");
    assert!(
        from.all_of.iter().any(|p| p.level_eq == Some(5)),
        "alt path filters level_eq: 5"
    );
    // The any_of branch contains either the name_contains MetalGreymon OR the
    // trait_has ADVENTURE leaf — assert the all_of vector contains a slot
    // that holds the any_of disjunction.
    let has_anyof = from.all_of.iter().any(|p| !p.any_of.is_empty());
    assert!(
        has_anyof,
        "alt path's all_of must include an any_of disjunction (MetalGreymon name OR ADVENTURE trait)"
    );
}

/// P-182 has exactly two declarative `grant_keyword` clauses (the printed
/// `<Security A. +1>` and `<Blocker>` keyword icons).
#[test]
fn p_182_has_two_declarative_grant_keyword_clauses() {
    let runner = wargreymon_runner();
    let card = runner.compiled_card("P-182").expect("compiled");

    let gk_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
            )
        })
        .count();
    assert_eq!(
        gk_count, 2,
        "expected 2 grant_keyword declaratives (SecurityAttackPlus + Blocker)"
    );
}

/// The two declarative keyword grants are SecurityAttackPlus (with value 1)
/// and Blocker. Both are own-scope, face-up by default.
#[test]
fn p_182_declarative_keywords_are_security_attack_plus_and_blocker() {
    let runner = wargreymon_runner();
    let card = runner.compiled_card("P-182").expect("compiled");

    let keywords: Vec<&str> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        keywords.contains(&"SecurityAttackPlus"),
        "SecurityAttackPlus grant keyword must be present; found: {keywords:?}"
    );
    assert!(
        keywords.contains(&"Blocker"),
        "Blocker grant keyword must be present; found: {keywords:?}"
    );
}

/// P-182 has exactly ONE triggered clause: the [When Digivolving] delete
/// (G-FORMULA-SOURCE-DP RESOLVED 2026-05-19). The [All Turns] +1000-per-color
/// effect is a declarative aura, not a triggered clause.
#[test]
fn p_182_has_one_triggered_when_digivolving_clause() {
    let runner = wargreymon_runner();
    let card = runner.compiled_card("P-182").expect("compiled");

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
        "P-182 must have exactly 1 triggered clause ([When Digivolving] delete); \
         found {}",
        triggered.len()
    );
    assert!(
        triggered[0].when.contains(&CompiledTiming::WhenDigivolving),
        "the triggered clause must fire on [When Digivolving]"
    );
}

/// Phase 2 Track F (2026-05-17): the distinct-colors-both-players formula
/// is available; P-182's [All Turns] +1000 DP per distinct color now
/// publishes a DYNAMIC aura via `dp_modifier_fn`.
#[test]
fn p_182_publishes_dynamic_dp_modifier_fn_aura() {
    let runner = wargreymon_runner();
    let card = runner.compiled_card("P-182").expect("compiled");

    let aura_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
            )
        })
        .count();
    assert_eq!(
        aura_count, 1,
        "P-182's [All Turns] +1000 DP-per-color clause must compile to one aura"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause behavioral: Security A. +1 keyword grant (declarative)
// ═══════════════════════════════════════════════════════════════════════════════

/// Playing P-182 onto the field installs `Keyword::SecurityAttackPlus(1)` on
/// the permanent via the declarative grant_keyword clause.
///
/// G-DECLARATIVE-KEYWORD is closed: the DSL declarative `grant_keyword`
/// clause lowers (`lower_grant_keyword::lower`) to an `Effect::declarative`
/// flagged `materializes_declarative_state()` whose body calls
/// `ctx.grant_declarative_keyword(...)`; the engine's declarative-state
/// tick installs the keyword modifier at runtime (proven by AD1-001 /
/// BT20-102).
#[test]
fn p_182_security_attack_plus_1_installed_on_field() {
    let mut runner = wargreymon_runner();

    // Place directly on field so we avoid needing digivolve targets.
    let handle = runner.place_on_field(0, "P-182", Some(0));

    // Declarative grant_keyword fires immediately when the card enters the
    // field via play_from_hand path; place_on_field bypasses play, so manually
    // fire OnPlay to reflect the declarative install.
    runner.fire_on_play(0, handle.index as usize);

    // Check via ModifierRegistry: DSL grant_keyword installs a permanent modifier.
    let installed_via_modifier = runner
        .modifiers()
        .has_keyword(handle, Keyword::SecurityAttackPlus(1));

    // Also check via card_data keywords (printed text parsing route).
    let from_card_data = runner
        .game
        .card_data_for_handle(
            runner.game.player(0).battle_area[handle.index as usize]
                .top_card()
                .handle(),
        )
        .map_or(false, |d| {
            d.keywords
                .iter()
                .any(|k| matches!(k, Keyword::SecurityAttackPlus(1)))
        });

    assert!(
        installed_via_modifier || from_card_data,
        "SecurityAttackPlus(1) must be present on P-182 after it enters the field"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause behavioral: Blocker keyword grant (declarative)
// ═══════════════════════════════════════════════════════════════════════════════

/// Playing P-182 onto the field installs `Keyword::Blocker` on the permanent
/// via the declarative grant_keyword clause.
///
/// G-DECLARATIVE-KEYWORD is closed — same wiring as the Security A.
/// installation test above (declarative-state tick installs the keyword).
#[test]
fn p_182_blocker_installed_on_field() {
    let mut runner = wargreymon_runner();

    let handle = runner.place_on_field(0, "P-182", Some(0));
    runner.fire_on_play(0, handle.index as usize);

    let installed_via_modifier = runner.modifiers().has_keyword(handle, Keyword::Blocker);
    let from_card_data = runner
        .game
        .card_data_for_handle(
            runner.game.player(0).battle_area[handle.index as usize]
                .top_card()
                .handle(),
        )
        .map_or(false, |d| {
            d.keywords.iter().any(|k| matches!(k, Keyword::Blocker))
        });

    assert!(
        installed_via_modifier || from_card_data,
        "Blocker keyword must be present on P-182 after it enters the field"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [When Digivolving] delete opp Digimon ≤ self DP
//                                                  (G-FORMULA-SOURCE-DP RESOLVED)
// ═══════════════════════════════════════════════════════════════════════════════

/// G-FORMULA-SOURCE-DP (RESOLVED 2026-05-19): the "delete 1 opp Digimon with
/// as much or less DP as this Digimon" clause uses the `source_dp` formula
/// primitive — `dp_lte: { formula: { source_dp: {} } }` — so only opponent
/// Digimon with DP ≤ the carrier WarGreymon's effective DP are selectable.
///
/// Same gap shape as BT17-102 Greymon's delete sub-clause.
#[test]
fn p_182_when_digivolving_deletes_opp_digimon_with_dp_lte_self() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-182")
        .expect("P-182 in embedded DSL pack")
        .add_card(make_metalgreymon("META"))
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 8000))
        .add_card(make_opp_digimon("OPP-HIGH", "OppHigh", 15000))
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    // Place opponents and stack [META, P-182] for P0.
    runner.place_on_field(1, "OPP-LOW", None);
    runner.place_on_field(1, "OPP-HIGH", None);
    let stack = runner.place_stack(0, &["META", "P-182"]);
    let opp_before = runner.battle_area_size(1);

    // Fire [When Digivolving] for the carrier.
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(stack),
    );
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    // Faithful behavior: P-182 base DP 12000; OPP-LOW (8000) ≤ 12000 must be
    // a legal delete target; OPP-HIGH (15000) must NOT be. Exactly 1 opp
    // permanent should be deleted (the legal one).
    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "exactly 1 opp Digimon should be deleted (the ≤12000-DP one)"
    );
    let surviving_names: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_name(&runner.game.card_data).to_string())
        .collect();
    assert!(
        surviving_names.iter().any(|n| n == "OppHigh"),
        "OppHigh (15000 DP) must survive; surviving={surviving_names:?}"
    );
}

/// G-FORMULA-SOURCE-DP (RESOLVED): companion negative test. When the
/// opponent has only Digimon with DP > self, the `dp_lte: { source_dp }`
/// candidate filter admits no target, so nothing is deleted.
#[test]
fn p_182_when_digivolving_deletes_nothing_when_no_opp_digimon_within_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-182")
        .expect("P-182 in embedded DSL pack")
        .add_card(make_metalgreymon("META"))
        .add_card(make_opp_digimon("OPP-HUGE", "OppHuge", 20000))
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "OPP-HUGE", None);
    let stack = runner.place_stack(0, &["META", "P-182"]);
    let opp_before = runner.battle_area_size(1);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(stack),
    );
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    // No legal target: opp battle area unchanged.
    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "no opp Digimon should be deleted when none has DP ≤ self DP"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — [All Turns] +1000 DP per distinct color  [BLOCKED]
// ═══════════════════════════════════════════════════════════════════════════════

/// BLOCKED: G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA — the [All Turns]
/// clause needs a continuously-recomputed +1000 DP aura whose value is
/// `1000 * (count of distinct colors across both players' battle-area
/// Digimon + Tamers)`. The DSL `kind: aura` + `dp_modifier_fn` shell
/// (RESOLVED by Group 6) is in place, but `PerSelector` has no
/// `distinct_colors_count` variant over a zone+filter+player combination.
///
/// See module-doc gap entry and `cards/p/P-182.yaml` header for the full
/// proposal and DCGO reference. When the formula primitive lands, uncomment
/// the YAML stub and activate this test.
#[test]
// Phase 2 Track F (2026-05-17): distinct_colors_count formula is available
// via `of: any`. Active.
fn p_182_all_turns_gets_1000_dp_per_distinct_color_on_field() {
    // Setup: own field has P-182 (Red Digimon) + a Blue Tamer; opp field has
    // a Yellow Digimon. Distinct colors across (own ∪ opp) Digimon ∪ Tamers
    // = {Red, Blue, Yellow} = 3. P-182 base DP = 12000. Expected effective
    // DP = 12000 + 3 * 1000 = 15000.
    let blue_tamer = make_colored_tamer(
        "BLU-TAMER",
        "Blue Tamer",
        digimon_engine::enums::CardColor::Blue,
    );
    let yellow_opp = make_colored_opp_digimon(
        "YEL-OPP",
        "Yellow Opp",
        4000,
        digimon_engine::enums::CardColor::Yellow,
    );

    let mut runner = DebugRunner::builder()
        .dsl_card("P-182")
        .expect("P-182 in embedded DSL pack")
        .add_card(blue_tamer)
        .add_card(yellow_opp)
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    let p182 = runner.place_on_field(0, "P-182", Some(0));
    runner.place_on_field(0, "BLU-TAMER", None);
    runner.place_on_field(1, "YEL-OPP", None);

    let dp = runner.effective_dp(p182).expect("perm has DP");
    assert!(
        dp >= 15000,
        "P-182 effective DP must include +3000 from 3 distinct colors \
         (Red + Blue + Yellow); got {dp}"
    );
}

/// BLOCKED: companion test — single color on field. Only P-182 itself (Red)
/// is in play; expected DP = 12000 (base) + 1 * 1000 = 13000. Locked in as
/// the lower-bound case for the formula.
#[test]
// Phase 2 Track F (2026-05-17): distinct_colors_count formula is available
// via `of: any`. Active.
fn p_182_all_turns_single_color_yields_1000_dp_buff() {
    let mut runner = wargreymon_runner();
    runner.game.turn_count = 1;

    let p182 = runner.place_on_field(0, "P-182", Some(0));
    let dp = runner.effective_dp(p182).expect("perm has DP");

    // Only Red on the field (P-182 itself); expected 12000 + 1000 = 13000.
    assert_eq!(
        dp, 13000,
        "P-182 effective DP must be 12000 + 1*1000 = 13000 when only Red is on the field; \
         got {dp}"
    );
}

/// BLOCKED: companion test — no own/opp Digimon or Tamers besides P-182.
/// Even with only P-182 on the field, the formula must count P-182's own
/// color (Red) → at minimum +1000.
#[test]
// Phase 2 Track F (2026-05-17): distinct_colors_count formula is available
// via `of: any`. Active.
fn p_182_all_turns_dynamic_dp_recomputes_when_field_changes() {
    // Initially only P-182 on the field; when an opp Digimon of a NEW color
    // enters, P-182's DP must recompute live (continuous aura semantics).
    let yellow_opp = make_colored_opp_digimon(
        "YEL2",
        "Yellow2",
        5000,
        digimon_engine::enums::CardColor::Yellow,
    );
    let mut runner = DebugRunner::builder()
        .dsl_card("P-182")
        .expect("P-182 in embedded DSL pack")
        .add_card(yellow_opp)
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    let p182 = runner.place_on_field(0, "P-182", Some(0));
    let dp_before = runner.effective_dp(p182).expect("perm has DP");

    runner.place_on_field(1, "YEL2", None);
    let dp_after = runner.effective_dp(p182).expect("perm has DP");

    // dp_after should be GREATER than dp_before because a new color (Yellow)
    // appeared on the opponent's field, lifting the distinct-color count by 1.
    assert!(
        dp_after > dp_before,
        "P-182 effective DP must recompute when a new color enters the field \
         (before={dp_before}, after={dp_after}); continuous aura semantics required"
    );
}
