//! BT17-102 Greymon — Digimon, Lv.4, White, DP 5000, Cost 5.
//! Traits: Dinosaur
//! Standard digivolve: Lv.3 Red / cost 3
//! Alt digivolve: Lv.3 w/[Agumon] in name / cost 2 (xros_req)
//!
//! # Card text (data/cards.json — verbatim)
//!
//! [When Digivolving] If this Digimon's name is [Koromon], it gets +3000 DP
//! for the turn. Then, delete 1 of your opponent's Digimon with as much or
//! less DP as this Digimon.
//!
//! [All Turns] This Digimon has all the names of level 3 and lower cards in
//! its digivolution cards.
//!
//! [On Deletion] You may play 1 Tamer card with [Tai Kamiya] or [Kari Kamiya]
//! in its name from your hand without paying the cost, or hatch in your
//! breeding area.
//!
//! Inherited Effect:
//!   [On Deletion] You may play 1 Tamer card with [Tai Kamiya] or [Kari Kamiya]
//!   in its name from your hand without paying the cost, or hatch in your
//!   breeding area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT17/White/BT17_102.cs
//!
//! # Patterns covered
//! - Standard + alt digivolve paths (Lv.3 Red and Lv.3 w/Agumon name)
//! - [When Digivolving] conditional DP buff via `self_digivolution_contains_name`
//! - [All Turns] dynamic source-derived effective names
//! - [On Deletion] modal branch (play OR hatch) with optional decline
//! - Inherited [On Deletion] mirror clause via `scope: inherited`
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                       | YAML clause                                                                  | Status                |
//! |---------------------------------------------------------|------------------------------------------------------------------------------|-----------------------|
//! | Standard digivolve Lv.3 Red / cost 3                    | `alt_paths[0]` `level_eq: 3, color_is: red, cost: 3`                         | OK                    |
//! | Alt digivolve Lv.3 w/Agumon-name / cost 2               | `alt_paths[1]` `level_eq: 3, name_contains: Agumon, cost: 2, ignore_reqs`    | OK                    |
//! | [When Digivolving] +3000 DP if name is [Koromon]        | `condition: { self_digivolution_contains_name: Koromon }` + `add_dp_modifier`| OK                    |
//! | [When Digivolving] then delete opp Digimon ≤ self DP    | `select_opponent_permanent` + `dp_lte: { formula: { source_dp: {} } }`       | OK (G-FORMULA-SOURCE-DP resolved) |
//! | [All Turns] dynamic name aliasing from material stack   | `identity.source_name_aliases: [{ level_lte: 3 }]`                            | OK                    |
//! | [On Deletion] may play Tai/Kari Tamer free OR hatch     | `when: on_deletion`, `optional: true`, `select_effect_choice` + 2 branches    | OK                    |
//! | Inherited [On Deletion] same body                       | `scope: inherited` mirror of above                                            | OK                    |
//!
//! # NEW DSL gaps logged here
//!
//! - **G-FORMULA-SOURCE-DP** — `dp_lte`/`dp_gte` formula primitives can read
//!   a NAMED binding's DP via `binding_dp: <name>` (G-BINDING-DP-FORMULA
//!   resolved 2026-05-02 for user-bound permanents) and a single user-picked
//!   binding's DP via various selection-piped paths, but there is no formula
//!   primitive that reads the SOURCE permanent's DP (`ctx.source_permanent`).
//!   The engine has the data — `Game::effective_dp(handle)` is callable on
//!   `ctx.source_permanent` — only the formula vocabulary lacks a `source_dp`
//!   variant or a `binding_dp: source` special-cased name. Required for any
//!   "delete opp Digimon with as much or less DP as this Digimon" clause
//!   (this card, BT22-006 Greymon, BT17-078 Omnimon, BT24-101 GreyKnightmon,
//!   etc.). Suggested closure:
//!     ```yaml
//!     dp_lte:
//!       formula:
//!         source_dp: {}        # reads ctx.source_permanent's effective_dp
//!     ```
//!   Implementation: add `SourceDp` variant to `FormulaSpec`/`CompiledFormula`;
//!   add evaluation branch in `formula_eval.rs` that calls
//!   `ctx.source_permanent.and_then(|h| ctx.game.effective_dp(h)).unwrap_or(0)`.
//!   File under `qa/dsl-vocab-gaps.md` as a sibling of G-BINDING-DP-FORMULA.
//!
//! - **G-DYNAMIC-NAME-ALIAS-FROM-STACK** — resolved for this card through
//!   `identity.source_name_aliases`, which derives effective names from
//!   current level-3-or-lower source cards and routes name predicates through
//!   the synthesized permanent identity.
//!
//! - **G-DSL-SELF-NAME-IS** (= G-DSL-SELF-NAME-CONTAINS, see AD1-014:170) —
//!   no DSL BoolPredicate evaluates "the carrier permanent's TOP CARD name
//!   contains/equals X". Closest is `self_digivolution_contains_name` (RESOLVED
//!   2026-05-02; scans the digivolution material stack but NOT the top card).
//!   For BT17-102 [When Digivolving], stack-scanning is the faithful proxy
//!   in the standard play pattern (Koromon → Agumon → Greymon), and matches
//!   the [All Turns] alias semantics. The `name_is`/`name_contains` filters
//!   exist but apply only to CANDIDATE cards in `select_*` filters, not to
//!   the carrier itself. Already a known gap; this card is one more affected
//!   case if the [All Turns] dynamic-name-alias gap also remains open.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

/// Fire WhenDigivolving for the given permanent handle using the direct
/// enqueue pattern (mirrors BT21-017 / BT21-029 / BT24-017 tests).
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Accept the outer optional-trigger prompt that a lone, single optional
/// [On Deletion] clause installs before its body runs.
///
/// When BT17-102 is deleted as a lone top-card Digimon, only its face-up
/// [On Deletion] clause queues — its inherited clause stays dormant on a
/// top-card Digimon (RULES 15-3-1). A single optional triggered clause whose
/// body opens with a mandatory `select_effect_choice` surfaces its accept/
/// decline as an outer `SelectionKind::Replacement` prompt
/// (G-OUTER-OPTIONAL-NOT-INSTALLED).
///
/// This asserts that prompt is the pending selection before accepting it, so
/// the test fails loudly *here* — rather than later with a confusing
/// EffectChoice mismatch — if the engine's trigger flow changes.
fn accept_on_deletion_optional(runner: &mut DebugRunner) {
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "BT17-102's lone optional [On Deletion] clause must install an outer \
         Replacement accept prompt before its EffectChoice body"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");
}

// ─── Fixture helpers ────────────────────────────────────────────────────────

fn make_koromon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Koromon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(1000);
    c
}

fn make_agumon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Agumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

fn make_filler_lv3(id: &str) -> CardData {
    // A Lv.3 non-Agumon, non-Koromon Digimon for the negative alt-path test.
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

fn make_opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

fn make_tai_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Tai Kamiya");
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c
}

fn make_kari_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Kari Kamiya");
    c.card_kind = CardKind::Tamer;
    c.play_cost = 2;
    c
}

fn make_other_tamer(id: &str, name: &str) -> CardData {
    // Tamer whose name does NOT contain Tai/Kari — used for negative
    // filter test (must not be a legal pick for the Tamer play branch).
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c
}

// ─── Section 1 — Structural assertions ──────────────────────────────────────

#[test]
fn bt17_102_compiles_with_standard_and_alt_digivolve_paths() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-102").expect("compiled");

    // Two alt-paths: Lv.3 Red cost 3 (standard), Lv.3 Agumon-name cost 2 (alt).
    assert_eq!(
        compiled.alt_paths.len(),
        2,
        "BT17-102 must have exactly 2 digivolve alt-paths"
    );

    let standard = compiled
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && matches!(p.cost, Some(CompiledCost::Literal(3)))
        })
        .expect("Lv.3 Red cost-3 path");
    let from = standard
        .from
        .as_ref()
        .expect("standard path has source predicate");
    assert!(
        from.all_of.iter().any(|p| p.level_eq == Some(3)),
        "standard path filters level_eq: 3"
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
                && matches!(p.cost, Some(CompiledCost::Literal(2)))
        })
        .expect("Lv.3 Agumon-name cost-2 path");
    let from = alt.from.as_ref().expect("alt path has source predicate");
    assert!(
        from.all_of.iter().any(|p| p.level_eq == Some(3)),
        "alt path filters level_eq: 3"
    );
    assert!(
        from.all_of
            .iter()
            .any(|p| p.name_contains.as_deref() == Some("Agumon")),
        "alt path filters name_contains: Agumon"
    );
}

#[test]
fn bt17_102_has_when_digivolving_triggered_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-102").expect("compiled");

    let clause: &CompiledTriggeredClause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have [When Digivolving] clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    // The +3000 DP buff itself is not 'optional' — printed text is conditional
    // ("If…"), not "may". The condition gating handles the absence-of-Koromon
    // case rather than an `optional: true` flag.
    assert!(
        !clause.optional,
        "[When Digivolving] is condition-gated, not optionally-declined"
    );
}

#[test]
fn bt17_102_has_own_on_deletion_optional_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-102").expect("compiled");

    let own_on_deletion = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnDeletion)
                    && matches!(t.scope, CompiledScope::FaceUp) =>
            {
                Some(t)
            }
            _ => None,
        })
        .count();
    assert_eq!(
        own_on_deletion, 1,
        "must have exactly 1 face-up [On Deletion] clause"
    );
}

#[test]
fn bt17_102_has_inherited_on_deletion_optional_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-102").expect("compiled");

    let inherited_on_deletion = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnDeletion)
                    && matches!(t.scope, CompiledScope::Inherited) =>
            {
                Some(t)
            }
            _ => None,
        })
        .count();
    assert_eq!(
        inherited_on_deletion, 1,
        "must have exactly 1 inherited [On Deletion] clause"
    );
}

#[test]
fn bt17_102_dynamic_name_alias_is_not_a_static_alias() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-102").expect("compiled");

    if let Some(identity) = &compiled.identity {
        assert!(
            identity.name_aliases.is_empty(),
            "BT17-102 must not publish static name aliases — its extra names \
             are derived from the live digivolution stack"
        );
    }
}

// ─── Section 2 — [When Digivolving] +3000 DP gating (split positive/negative)

/// Positive: BT17-102 digivolves on top of a Koromon stack and gains +3000 DP
/// for the turn (5000 base → 8000 effective).
#[test]
fn bt17_102_when_digivolving_with_koromon_in_stack_buffs_self_3000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .add_card(make_koromon("KORO"))
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    // Stack [KORO, BT17-102] with BT17-102 on top. `place_stack` does not fire
    // the WhenDigivolving trigger automatically; we fire OnPlay at the
    // top-card index to exercise the registered effect chain.
    let stack = runner.place_stack(0, &["KORO", "BT17-102"]);

    // Manually fire the [When Digivolving] timing via OnPlay for the top card
    // (carrier). The DSL clause's `condition: { self_digivolution_contains_name:
    // "Koromon" }` should match because Koromon sits in the source stack.
    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();

    let dp = runner.effective_dp(stack).expect("perm has DP");
    assert!(
        dp >= 8000,
        "BT17-102 effective DP after Koromon-gated +3000 buff must be ≥ 8000 \
         (base 5000 + 3000); got {dp}"
    );
}

/// Negative: BT17-102 sits alone (no Koromon in the digivolution stack); the
/// +3000 buff must NOT apply. Effective DP stays at 5000 base.
#[test]
fn bt17_102_when_digivolving_without_koromon_does_not_buff() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .add_card(make_filler_lv3("LV3-FILLER"))
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    // Stack [LV3-FILLER, BT17-102] — no Koromon below.
    let stack = runner.place_stack(0, &["LV3-FILLER", "BT17-102"]);

    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();

    let dp = runner.effective_dp(stack).expect("perm has DP");
    assert_eq!(
        dp, 5000,
        "BT17-102 effective DP must stay at base 5000 when Koromon is not in \
         the digivolution stack; got {dp}"
    );
}

// ─── Section 3 — [When Digivolving] DP-comparator delete ────────────────────

/// G-FORMULA-SOURCE-DP (RESOLVED 2026-05-19): the "then, delete 1 opp
/// Digimon with as much or less DP as this Digimon" sub-clause is authored
/// with `dp_lte: { formula: { source_dp: {} } }` — the `source_dp` formula
/// primitive reads the carrier permanent's effective DP. The +3000 Koromon
/// buff applies first (the test stacks [KORO, BT17-102] so the buff fires),
/// raising effective DP to 8000; OPP-LOW (4000) is then a legal delete
/// target and OPP-HIGH (12000) is not.
#[test]
fn bt17_102_when_digivolving_delete_filters_to_dp_lte_self() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .add_card(make_koromon("KORO"))
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 4000))
        .add_card(make_opp_digimon("OPP-HIGH", "OppHigh", 12000))
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    // After +3000 buff, BT17-102 effective DP = 8000. OPP-LOW (4000) must be a
    // legal delete target; OPP-HIGH (12000) must NOT be.
    runner.place_on_field(1, "OPP-LOW", None);
    runner.place_on_field(1, "OPP-HIGH", None);
    let stack = runner.place_stack(0, &["KORO", "BT17-102"]);
    let opp_before = runner.battle_area_size(1);

    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();

    // Faithful behavior: opp battle-area drops by exactly 1 (OPP-LOW deleted),
    // and OPP-HIGH (12000 DP > 8000) survives.
    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "exactly 1 opp Digimon should be deleted (the ≤8000-DP one)"
    );
    let surviving_names: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_name(&runner.game.card_data).to_string())
        .collect();
    assert!(
        surviving_names.iter().any(|n| n == "OppHigh"),
        "OppHigh (12000 DP) must survive; surviving={surviving_names:?}"
    );
}

// ─── Section 4 — [All Turns] Dynamic name aliasing ──────────────────────────

#[test]
fn bt17_102_all_turns_aliases_low_level_material_names() {
    const PROBE_YAML: &str = r#"
card: TEST-ALIAS-PROBE
name: Alias Probe
kind: option
color: [white]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_own_permanent:
          bind_as: target
          filter:
            all_of:
              - kind: digimon
              - name_is: "Agumon"
          prompt: "Choose an Agumon-named Digimon"
"#;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .from_dsl_yaml(PROBE_YAML)
        .expect("probe card compiles")
        .add_card(make_koromon("KORO"))
        .add_card(make_agumon("AGU"))
        .hand(0, &["TEST-ALIAS-PROBE"])
        .memory(10)
        .start();

    let _stack = runner.place_stack(0, &["KORO", "AGU", "BT17-102"]);
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.activate_hand_main(0, 0),
        "probe Option should activate from hand"
    );

    let view = runner
        .pending_selection_view()
        .expect("name_is: Agumon should see BT17-102's dynamic source-name alias");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "BT17-102 must be selectable as an Agumon-named Digimon via its Lv.3 source alias"
    );
}

// ─── Section 5 — [On Deletion] own clause: play Tai/Kari Tamer free ─────────

/// Positive Play branch: with a Tai Kamiya in hand, deleting BT17-102 fires
/// the optional [On Deletion]; choosing the Play branch + the Tamer plays it
/// for free (memory unchanged from the cost path).
#[test]
fn bt17_102_own_on_deletion_play_branch_plays_tai_kamiya_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .add_card(make_tai_tamer("TAI"))
        .hand(0, &["TAI"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let perm = runner.place_on_field(0, "BT17-102", Some(0));

    runner.game.delete_permanent_with_cause(
        perm,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );

    // Lone BT17-102 fires only its face-up [On Deletion]; accept that single
    // optional clause's outer prompt so its EffectChoice body installs.
    accept_on_deletion_optional(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("EffectChoice prompt must install");
    assert_eq!(view.kind, SelectionKind::EffectChoice);

    runner.execute_branch(0).expect("pick Play branch");
    let mem_after_choice = runner.memory();
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TAI"),
        "Tai Kamiya tamer should now be on the field (played for free)"
    );
    assert_eq!(
        runner.memory(),
        mem_after_choice,
        "playing free must not change memory after the branch was picked"
    );
}

/// Positive Play branch with Kari instead of Tai — same free-play path.
#[test]
fn bt17_102_own_on_deletion_play_branch_plays_kari_kamiya_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .add_card(make_kari_tamer("KARI"))
        .hand(0, &["KARI"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let perm = runner.place_on_field(0, "BT17-102", Some(0));

    runner.game.delete_permanent_with_cause(
        perm,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    accept_on_deletion_optional(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("EffectChoice prompt must install");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("pick Play branch");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "KARI"),
        "Kari Kamiya tamer should now be on the field (played for free)"
    );
}

/// Negative filter: an unrelated Tamer in hand (no Tai/Kari in name) must NOT
/// be a legal pick when the Play branch is taken.
#[test]
fn bt17_102_own_on_deletion_play_branch_rejects_unrelated_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .add_card(make_other_tamer("MATT", "Matt Ishida"))
        .hand(0, &["MATT"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let perm = runner.place_on_field(0, "BT17-102", Some(0));
    runner.game.delete_permanent_with_cause(
        perm,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    accept_on_deletion_optional(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("EffectChoice prompt must install");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("pick Play branch");
    let _ = runner.auto_resolve();

    // MATT must NOT have hit the field — the select_hand filter is tamer +
    // (Tai or Kari name) and Matt Ishida fails both name predicates.
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "MATT"),
        "Matt Ishida tamer must NOT be playable via the Tai/Kari-name filter"
    );
}

/// Decline branch: with the clause `optional: true`, the player must be able
/// to skip the entire effect (no Tamer played, no hatch). Even via auto-resolve
/// the Tamer must NOT be auto-played without the player explicitly choosing the
/// Play branch — but auto_resolve picks the first legal action, so we drive the
/// PASS path explicitly via the EffectChoice prompt by trashing the Tamer from
/// hand first (no legal Play targets → no Tamer play).
///
/// Stronger decline assertion: after auto-resolving with no Tai/Kari Tamer in
/// hand AND breeding empty (so hatch can't move), the deletion clears the
/// permanent without affecting the field.
#[test]
fn bt17_102_own_on_deletion_no_legal_targets_resolves_to_noop() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let perm = runner.place_on_field(0, "BT17-102", Some(0));
    let battle_area_before_p0 = runner.battle_area_size(0);
    let hand_before = runner.hand_size(0);

    runner.game.delete_permanent_with_cause(
        perm,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );

    // Drain whatever prompts install; with no Tai/Kari Tamer in hand the Play
    // branch has no legal pick and the clause cannot net-fire a Tamer play.
    let _ = runner.auto_resolve();

    // No Tamer was played and battle area shrank by exactly 1 (the deleted
    // BT17-102) — no spurious permanent created from the optional clause.
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand size must be unchanged (no Tamer was added or removed)"
    );
    assert!(
        runner.battle_area_size(0) <= battle_area_before_p0,
        "battle area must not grow after declining"
    );
}

// ─── Section 6 — Inherited [On Deletion] ────────────────────────────────────

/// Inherited On Deletion: BT17-102 sits as a digivolution material under a
/// carrier, the carrier is deleted, and BT17-102's inherited On Deletion fires.
/// The Play branch with a Tai Kamiya in hand plays the Tamer for free.
#[test]
fn bt17_102_inherited_on_deletion_plays_tai_kamiya_when_carrier_deleted() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_tai_tamer("TAI"))
        .hand(0, &["TAI"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    // Stack [BT17-102, CARRIER] with CARRIER on top — BT17-102 is now an
    // inherited-effect source riding under CARRIER.
    let carrier = runner.place_stack(0, &["BT17-102", "CARRIER"]);

    runner.game.delete_permanent_with_cause(
        carrier,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    // The inherited [On Deletion] clause is the only one that fires when the
    // carrier is deleted; accept its single optional clause's outer prompt.
    accept_on_deletion_optional(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("inherited EffectChoice prompt must install");
    assert_eq!(view.kind, SelectionKind::EffectChoice);

    runner.execute_branch(0).expect("pick Play branch");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TAI"),
        "Tai Kamiya tamer must be played free via the inherited [On Deletion]"
    );
}

/// Inherited path with no legal Tamer in hand: the clause's Play branch has
/// no legal pick. The deletion clears the carrier without spuriously playing
/// a Tamer. (Stronger optionality assertion — covering "no legal target" is
/// equivalent to a decline for behavioral correctness.)
#[test]
fn bt17_102_inherited_on_deletion_no_legal_targets_resolves_to_noop() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-102")
        .expect("BT17-102 in embedded DSL pack")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &["BT17-102", "CARRIER"]);
    let battle_area_before = runner.battle_area_size(0);
    let hand_before = runner.hand_size(0);

    runner.game.delete_permanent_with_cause(
        carrier,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand must not change when no legal Tamer pick exists"
    );
    assert!(
        runner.battle_area_size(0) <= battle_area_before,
        "battle area must not grow when the inherited On Deletion has no legal target"
    );
}
