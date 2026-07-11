//! BT24-010 Greymon — Digimon, Lv.4, Red/Black, DP 5000, Cost 5.
//! Traits: Dinosaur/Titan/TS. Attribute: Virus.
//!
//! # Card text (official Bandai DB bundle — data/card_bundles/BT24-010.md)
//!
//! ```text
//! <Blocker> (This Digimon can block in the blocker timing.)
//! [On Deletion] <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the
//!     top card. You can't trash past level 3 cards.)
//! ```
//!
//! Inherited Effect:
//! ```text
//! <Raid> (When this Digimon attacks, you may switch the target of attack to
//!     1 of your opponent's unsuspended Digimon with the highest DP.)
//! ```
//!
//! # Digivolution requirements (official Bandai DB — AUTHORITATIVE)
//! - Standard: Red Lv.3 / cost 3 (also present in cards.json evo_costs — auto-lowered)
//! - Standard: Black Lv.3 / cost 3 (DROPPED by cards.json's lossy multi-colour
//!   evo_costs ingest — cards.json only carries the Red circle. Must be
//!   authored explicitly as an alt_path per CLAUDE.md source-priority /
//!   the BT24-014-class fix pattern.)
//! - Alt-digivolve (xros_req): "[Digivolve] Lv.3 w/[Agumon] in name or
//!   w/[TS] trait: Cost 2"
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_010.cs
//!
//! DCGO cross-check:
//! - Alternative Digivolution Condition (EffectTiming.None):
//!   `AddSelfDigivolutionRequirementStaticEffect(permanentCondition: (level3
//!   && (ContainsCardName("Agumon") || HasTSTraits)), digivolutionCost: 2,
//!   ignoreDigivolutionRequirement: false)`.
//! - Blocker (EffectTiming.None): `BlockerSelfStaticEffect(isInheritedEffect:
//!   false)`.
//! - On Deletion (EffectTiming.OnDestroyedAnyone): `ActivateClass` with
//!   `SetUpActivateClass(CanActivateCondition, ActivateCoroutine, -1, false,
//!   ...)` — maxCountPerTurn -1 (NO OPT lockout), isOptional false
//!   (MANDATORY). `CanActivateCondition` = `CanActivateOnDeletion(...) &&
//!   HasMatchConditionPermanent(SelectableOpponentsDigimon)` — gates on at
//!   least 1 opponent Digimon existing; the pick itself is
//!   `SelectPermanentEffect` with `canNoSelect: false, maxCount: 1`, then
//!   `IDegeneration(permanent, 1, activateClass)` = De-Digivolve 1
//!   (stop_at_level 3 per the printed reminder text / keyword-semantics.md
//!   row `<De-Digivolve x>`: "stops at ... Lv.3", "trashing mandatory, can't
//!   choose 0").
//! - Raid (EffectTiming.OnAllyAttack): `RaidSelfEffect(isInheritedEffect:
//!   true)` — INHERITED ONLY (no own-face Raid grant on this card, unlike
//!   siblings such as BT24-011/BT24-017 which grant both own + inherited).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - H5  grant_keyword Blocker (own, declarative)
//! - H9  grant_keyword Raid (inherited only, declarative)
//! - D1-adj / F-adj: [On Deletion] mandatory single-target de_digivolve,
//!   no OPT, condition gated implicitly by `select_opponent_permanent`
//!   short-circuiting on zero candidates (matches DCGO
//!   HasMatchConditionPermanent outer gate) — the de_digivolve step is the
//!   WHOLE clause body (no dropped mandatory tail after it), so this is NOT
//!   the EX7-073/BT21-087-class "select short-circuits and drops a sibling
//!   tail" bug.
//! - structural — alt_paths: standard Red Lv.3/cost3 (auto from evo_costs),
//!   explicit Black Lv.3/cost3 (compensating for the cards.json multi-colour
//!   drop), and the Agumon-in-name OR TS-trait Lv.3/cost2 alt-digivolve
//!   (any_of composition).
//!
//! # Notes
//! - No [Once Per Turn] anywhere in printed text — no OPT lockout tests.
//! - De-Digivolve 1 stop_at_level: 3 — "You can't trash past level 3 cards"
//!   means the pop halts if the next-would-be-exposed source card is below
//!   level 3 (`de_digivolve_core`'s `nt_level < floor` break), matching the
//!   41 other `stop_at_level: 3` sibling authorings in the pool.
//! - The alt-digivolve condition is a genuine independent circle (DCGO
//!   `ignoreDigivolutionRequirement: false`) — `ignore_requirements` is
//!   omitted (defaults false), matching the BT24-011/BT24-034 sibling
//!   convention for standard "OR" alt-digivolve circles.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledPredicate, CompiledScope,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Flatten a `CompiledPredicate` tree (following `all_of` / `any_of`
/// recursively) into every reachable predicate node, so structural
/// assertions are robust to whichever composition shape the YAML uses for
/// the `any_of: [name_contains, trait_has]` alt-digivolve gate.
fn flatten_predicate_tree<'a>(pred: &'a CompiledPredicate, out: &mut Vec<&'a CompiledPredicate>) {
    out.push(pred);
    for p in &pred.all_of {
        flatten_predicate_tree(p, out);
    }
    for p in &pred.any_of {
        flatten_predicate_tree(p, out);
    }
}

fn predicate_tree_has_name_contains(pred: &CompiledPredicate, needle: &str) -> bool {
    let mut leaves = Vec::new();
    flatten_predicate_tree(pred, &mut leaves);
    leaves
        .iter()
        .any(|p| p.name_contains.as_deref() == Some(needle))
}

fn predicate_tree_has_trait_has(pred: &CompiledPredicate, needle: &str) -> bool {
    let mut leaves = Vec::new();
    flatten_predicate_tree(pred, &mut leaves);
    leaves
        .iter()
        .any(|p| p.trait_has.as_deref() == Some(needle))
}

/// Build a runner with BT24-010 (Greymon) loaded from the embedded DSL pack.
fn greymon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT24-010")
        .expect("BT24-010 must be in embedded DSL pack")
        .memory(15)
        .start()
}

/// Runner with Greymon + a generic opponent test Digimon registered.
fn greymon_with_opp() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT24-010")
        .expect("BT24-010 must be in embedded DSL pack")
        .add_card(make_test_card("OPP-DIG", "OppDigimon"))
        .memory(15)
        .start()
}

/// A synthetic Lv.3 card whose stack the opponent's Digimon sits on top of
/// (`OPP-DIG` as the top, `OPP-SRC-LV4` beneath) — enough depth that a
/// single De-Digivolve 1 pop lands on a level >= 3 source (does not halt).
fn opp_src_lv4() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("OPP-SRC-LV4", "OppSourceLv4");
    c.level = Some(4);
    c
}

/// A synthetic Lv.2 source card — placed directly beneath the opponent's top
/// card so a single De-Digivolve 1 pop would expose a level-2 card. Used to
/// prove `stop_at_level: 3` still allows exactly one pop when the CURRENT
/// top's own level isn't inspected (only the newly-exposed next-top is
/// checked against the floor); paired with `opp_target_stack_floor` below
/// this exercises the boundary the "can't trash past level 3" text guards.
fn opp_src_lv2() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("OPP-SRC-LV2", "OppSourceLv2");
    c.level = Some(2);
    c
}

/// Lv.3 Digimon named "Agumon" fixture — satisfies the alt-digivolve's
/// `name_contains: Agumon` branch (and is also Lv.3, satisfying the level
/// gate) but carries no TS trait, isolating the name-based leg.
fn agumon_named_lv3() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("AGU-LV3", "Agumon");
    c.level = Some(3);
    c.traits = Vec::new();
    c
}

/// Lv.3 Digimon with the TS trait but a name that does NOT contain "Agumon"
/// — isolates the trait-based leg of the alt-digivolve `any_of`.
fn ts_trait_lv3() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("TS-LV3", "Tsunomon");
    c.level = Some(3);
    c.traits = vec!["TS".to_string()];
    c
}

/// Lv.3 Digimon with neither "Agumon" in name nor TS trait — negative
/// control for the alt-digivolve gate.
fn plain_lv3() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("PLAIN-LV3", "Palmon");
    c.level = Some(3);
    c.traits = vec!["Plant".to_string()];
    c
}

/// A Black Lv.3 filler — used to prove the compensating Black standard
/// digivolve circle is present and payable at cost 3.
fn black_lv3() -> digimon_engine::card_data::CardData {
    use digimon_engine::enums::CardColor;
    let mut c = make_test_card("BLACK-LV3", "BlackFiller");
    c.level = Some(3);
    c.colors = vec![CardColor::Black];
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt24_010_compiles_and_is_in_pack() {
    let runner = greymon_runner();
    let compiled = runner.compiled_card("BT24-010");
    assert!(
        compiled.is_some(),
        "BT24-010 must be present in the embedded DSL pack"
    );
}

#[test]
fn bt24_010_has_printed_metadata() {
    let runner = greymon_runner();
    let card = runner
        .compiled_card("BT24-010")
        .expect("BT24-010 compiled card present");

    assert_eq!(card.name, "Greymon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(5000));
    assert!(card.traits.iter().any(|t| t == "Dinosaur"));
    assert!(card.traits.iter().any(|t| t == "Titan"));
    assert!(card.traits.iter().any(|t| t == "TS"));
}

/// Own-scope: exactly 1 declarative Blocker grant. No own-scope Raid (Raid
/// is inherited-only on this card, unlike BT24-011/BT24-017 siblings).
#[test]
fn bt24_010_own_blocker_grant_present() {
    let runner = greymon_runner();
    let compiled = runner.compiled_card("BT24-010").expect("compiles");

    let own_blocker = compiled.effects.iter().find(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Blocker"
        )
    });
    assert!(
        own_blocker.is_some(),
        "BT24-010 must have an own-scope (FaceUp) GrantKeyword Blocker clause"
    );
}

#[test]
fn bt24_010_inherited_raid_grant_present() {
    let runner = greymon_runner();
    let compiled = runner.compiled_card("BT24-010").expect("compiles");

    let inherited_raid = compiled.effects.iter().find(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword == "Raid"
        )
    });
    assert!(
        inherited_raid.is_some(),
        "BT24-010 must have an inherited-scope GrantKeyword Raid clause"
    );
}

#[test]
fn bt24_010_no_own_scope_raid_grant() {
    let runner = greymon_runner();
    let compiled = runner.compiled_card("BT24-010").expect("compiles");

    let own_raid = compiled.effects.iter().find(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Raid"
        )
    });
    assert!(
        own_raid.is_none(),
        "BT24-010 printed text has Raid ONLY as an inherited effect — no own-face Raid"
    );
}

#[test]
fn bt24_010_on_deletion_triggered_clause_present() {
    use digimon_dsl::compiled::CompiledTiming;

    let runner = greymon_runner();
    let compiled = runner.compiled_card("BT24-010").expect("compiles");

    let on_deletion = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDeletion) => Some(t),
        _ => None,
    });
    let clause = on_deletion.expect("BT24-010 must have an On Deletion triggered clause");
    assert!(
        !clause.optional,
        "De-Digivolve 1 target selection must be mandatory (DCGO canNoSelect: false / isOptional: false)"
    );
    assert!(
        !clause.once_per_turn,
        "DCGO SetUpActivateClass maxCountPerTurn is -1 — no OPT lockout"
    );
}

// ─── alt_paths structural assertions ──────────────────────────────────────

/// At least 3 Digivolve alt_paths: standard Red Lv3/3 (auto from evo_costs),
/// explicit Black Lv3/3 (compensating the cards.json multi-colour drop), and
/// the Agumon-name-OR-TS-trait Lv3/2 alt-digivolve.
#[test]
fn bt24_010_alt_paths_contain_black_standard_circle() {
    let runner = greymon_runner();
    let compiled = runner.compiled_card("BT24-010").expect("compiles");

    let digivolve_paths: Vec<&CompiledAltPath> = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();

    let black_circle = digivolve_paths.iter().find(|p| {
        p.from
            .as_ref()
            .map(|f| {
                f.level_eq == Some(3)
                    && f.color_is == Some(digimon_dsl::compiled::CompiledColor::Black)
            })
            .unwrap_or(false)
    });
    assert!(
        black_circle.is_some(),
        "BT24-010 must have an explicit Black Lv.3 standard digivolve alt_path \
         (cards.json evo_costs only carries the Red circle)"
    );
    if let Some(p) = black_circle {
        assert_eq!(
            p.cost,
            Some(CompiledCost::Literal(3)),
            "Black Lv.3 standard circle must cost 3 memory"
        );
    }
}

#[test]
fn bt24_010_alt_paths_contain_agumon_or_ts_lv3_cost2() {
    let runner = greymon_runner();
    let compiled = runner.compiled_card("BT24-010").expect("compiles");

    let digivolve_paths: Vec<&CompiledAltPath> = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();

    // Find the alt-digivolve path: level_eq 3, cost 2, and an `any_of` (or
    // direct field) that covers both name_contains: Agumon and trait_has: TS.
    let alt_path = digivolve_paths.iter().find(|p| {
        p.cost == Some(CompiledCost::Literal(2))
            && p.from
                .as_ref()
                .map(|f| {
                    f.level_eq == Some(3) || f.all_of.iter().any(|sub| sub.level_eq == Some(3))
                })
                .unwrap_or(false)
    });
    let alt_path = alt_path.expect(
        "BT24-010 must have a Lv.3-gated Digivolve alt_path costing 2 memory \
         (the Agumon-in-name OR TS-trait alt-digivolve)",
    );

    let from = alt_path.from.as_ref().expect("alt path has from filter");

    // Collect every predicate leaf reachable from `from` (following
    // all_of/any_of recursively) so this test is robust to whichever
    // composition shape the YAML uses (all_of[level_eq, any_of[...]] vs
    // any_of[{level_eq,name_contains}, {level_eq,trait_has}]).
    let mut leaves = Vec::new();
    flatten_predicate_tree(from, &mut leaves);

    assert!(
        leaves
            .iter()
            .any(|p| p.name_contains.as_deref() == Some("Agumon")),
        "alt-digivolve predicate tree must contain name_contains: Agumon"
    );
    assert!(
        leaves.iter().any(|p| p.trait_has.as_deref() == Some("TS")),
        "alt-digivolve predicate tree must contain trait_has: TS"
    );
    assert!(
        leaves.iter().any(|p| p.level_eq == Some(3)),
        "alt-digivolve predicate tree must gate on level_eq: 3"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Blocker keyword
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt24_010_blocker_keyword_active_on_field() {
    let mut runner = greymon_runner();
    let h = runner.place_on_field(0, "BT24-010", None);
    runner.game_mut().tick_declarative_effects();
    assert!(
        runner.game.has_keyword(h, Keyword::Blocker),
        "<Blocker> must be active while Greymon is face-up on the battle area"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: inherited Raid keyword
// ═══════════════════════════════════════════════════════════════════════════════

/// Raid is granted while Greymon is a digivolution source (inherited scope)
/// underneath another Digimon, not while it is the top (face-up) card.
#[test]
fn bt24_010_inherited_raid_active_when_buried() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-010")
        .expect("BT24-010 loads")
        .add_card({
            let mut c = make_test_card("HOST-LV5", "HostLv5");
            c.level = Some(5);
            c
        })
        .memory(15)
        .start();

    let host = runner.place_stack(0, &["BT24-010", "HOST-LV5"]);
    runner.game_mut().tick_declarative_effects();

    assert!(
        runner.game.has_keyword(host, Keyword::Raid),
        "<Raid> must be active on the host permanent via BT24-010's inherited grant"
    );
}

/// Negative: Greymon face-up (top card, not buried) does NOT grant Raid —
/// the card has no own-scope Raid clause (see bt24_010_no_own_scope_raid_grant).
#[test]
fn bt24_010_face_up_greymon_has_no_raid() {
    let mut runner = greymon_runner();
    let h = runner.place_on_field(0, "BT24-010", None);
    runner.game_mut().tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(h, Keyword::Raid),
        "Face-up Greymon (top of stack) must NOT have Raid — it's inherited-only"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Condition gating: On Deletion De-Digivolve target
// ═══════════════════════════════════════════════════════════════════════════════

/// Negative: no opponent Digimon on field → deleting Greymon must NOT open
/// any selection (mirrors DCGO HasMatchConditionPermanent outer gate; the
/// select_opponent_permanent step itself short-circuits on empty candidates
/// and there is no dropped sibling tail after it).
#[test]
fn bt24_010_on_deletion_no_opponent_digimon_no_prompt() {
    let mut runner = greymon_runner();
    let h = runner.place_on_field(0, "BT24-010", None);

    runner.game_mut().delete_permanent_with_effects(h);

    assert!(
        runner.pending_selection().is_none(),
        "No selection should install on deletion when opponent has no Digimon"
    );
}

/// Positive: opponent has exactly 1 Digimon → deleting Greymon opens a
/// mandatory OppField selection for the De-Digivolve target.
#[test]
fn bt24_010_on_deletion_opponent_digimon_prompts_mandatory_oppfield() {
    let mut runner = greymon_with_opp();
    runner.place_on_field(1, "OPP-DIG", None);
    let h = runner.place_on_field(0, "BT24-010", None);

    runner.game_mut().delete_permanent_with_effects(h);

    let kind = runner
        .pending_kind()
        .expect("Selection should install when opponent has a Digimon");
    assert_eq!(kind, SelectionKind::OppField);

    let view = runner
        .pending_selection_view()
        .expect("Selection installed");
    assert!(
        !view.is_optional,
        "De-Digivolve 1 target selection must be mandatory"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "Exactly 1 opponent Digimon should be a valid target"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Behavioral: On Deletion De-Digivolve 1 resolution
// ═══════════════════════════════════════════════════════════════════════════════

/// Resolving the mandatory target select pops exactly 1 digivolution source
/// off the opponent's chosen permanent into their trash (De-Digivolve 1).
#[test]
fn bt24_010_on_deletion_resolves_de_digivolve_one_source() {
    // Opponent's target: stack of [OPP-SRC-LV4 (source), OPP-DIG (top)].
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-010")
        .expect("BT24-010 loads")
        .add_card(opp_src_lv4())
        .add_card(make_test_card("OPP-DIG", "OppDigimon"))
        .memory(15)
        .start();

    let target = runner.place_stack(1, &["OPP-SRC-LV4", "OPP-DIG"]);
    let h = runner.place_on_field(0, "BT24-010", None);

    let stack_before = runner.game.player(1).battle_area[target.index as usize].stack_size();
    let trash_before = runner.trash_size(1);

    runner.game_mut().delete_permanent_with_effects(h);

    let view = runner
        .pending_selection_view()
        .expect("De-Digivolve target selection installed");
    let action_id = view.valid_action_ids[0];
    let sel_player = view.selecting_player;
    runner
        .game_mut()
        .resolve_selection(sel_player, action_id)
        .expect("target resolves");

    assert!(
        runner.pending_selection().is_none(),
        "De-Digivolve 1 resolves in a single mandatory pick with no further prompts"
    );

    let stack_after = runner.game.player(1).battle_area[target.index as usize].stack_size();
    assert_eq!(
        stack_after,
        stack_before - 1,
        "De-Digivolve 1 pops exactly 1 digivolution card"
    );
    assert_eq!(
        runner.trash_size(1),
        trash_before + 1,
        "the popped digivolution card lands in the opponent's trash"
    );
}

/// `stop_at_level: 3` halts the pop BEFORE it happens if the card that
/// would become the new top is below level 3 (`de_digivolve_core` checks
/// `next_top_level` — `stack[len-2]` — against the floor before popping).
/// With a stack of [OPP-SRC-LV2, OPP-DIG], popping OPP-DIG's only source
/// would expose OPP-SRC-LV2 (level 2) as the new top, which is below the
/// floor — so the printed "You can't trash past level 3 cards" blocks the
/// pop entirely: De-Digivolve 1 trashes ZERO cards here, matching the
/// mandatory-but-unpayable-target-shape row in keyword-semantics.md ("stops
/// at ... Lv.3").
#[test]
fn bt24_010_on_deletion_de_digivolve_blocked_when_next_top_below_level_3() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-010")
        .expect("BT24-010 loads")
        .add_card(opp_src_lv2())
        .add_card(make_test_card("OPP-DIG", "OppDigimon"))
        .memory(15)
        .start();

    // Stack: [OPP-SRC-LV2 (bottom source, level 2), OPP-DIG (top)].
    let target = runner.place_stack(1, &["OPP-SRC-LV2", "OPP-DIG"]);
    let h = runner.place_on_field(0, "BT24-010", None);

    let stack_before = runner.game.player(1).battle_area[target.index as usize].stack_size();
    let trash_before = runner.trash_size(1);

    runner.game_mut().delete_permanent_with_effects(h);
    let view = runner
        .pending_selection_view()
        .expect("De-Digivolve target selection installed");
    let action_id = view.valid_action_ids[0];
    let sel_player = view.selecting_player;
    runner
        .game_mut()
        .resolve_selection(sel_player, action_id)
        .expect("target resolves");

    let stack_after = runner.game.player(1).battle_area[target.index as usize].stack_size();
    assert_eq!(
        stack_after, stack_before,
        "stop_at_level: 3 blocks the pop when the next-would-be-exposed \
         source is below level 3 — the stack is untouched"
    );
    assert_eq!(
        runner.trash_size(1),
        trash_before,
        "no digivolution card is trashed when the pop is blocked by the level-3 floor"
    );
}

/// Multiple opponent Digimon → both are legal targets for the mandatory pick.
#[test]
fn bt24_010_on_deletion_multiple_opponent_targets_both_selectable() {
    let target2 = {
        let mut c = make_test_card("OPP-DIG-2", "OppDigimon2");
        c.card_id = "OPP-DIG-2".to_string();
        c
    };
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-010")
        .expect("BT24-010 loads")
        .add_card(make_test_card("OPP-DIG", "OppDigimon"))
        .add_card(target2)
        .memory(15)
        .start();
    runner.place_on_field(1, "OPP-DIG", None);
    runner.place_on_field(1, "OPP-DIG-2", None);
    let h = runner.place_on_field(0, "BT24-010", None);

    runner.game_mut().delete_permanent_with_effects(h);

    let view = runner
        .pending_selection_view()
        .expect("Selection installed");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both opponent Digimon must be valid De-Digivolve 1 targets"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Alt-digivolve behavioral: Agumon-name / TS-trait / negative
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: a Lv.3 Digimon named "Agumon" (no TS trait) satisfies the
/// alt-digivolve name leg — BT24-010 must be a legal digivolve target from
/// it for cost 2 (distinct from the standard Lv.3/cost3 circles).
#[test]
fn bt24_010_alt_digivolve_agumon_name_leg_offers_cost_2() {
    let runner = DebugRunner::builder()
        .dsl_card("BT24-010")
        .expect("BT24-010 loads")
        .add_card(agumon_named_lv3())
        .memory(15)
        .start();

    let compiled = runner.compiled_card("BT24-010").expect("compiles");
    let matches_agumon = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(2))
            && p.from
                .as_ref()
                .map(|f| predicate_tree_has_name_contains(f, "Agumon"))
                .unwrap_or(false)
    });
    assert!(
        matches_agumon,
        "alt-digivolve must expose a name_contains: Agumon leg at cost 2"
    );
}

/// Positive: a Lv.3 Digimon with the TS trait (name not containing
/// "Agumon") satisfies the alt-digivolve trait leg.
#[test]
fn bt24_010_alt_digivolve_ts_trait_leg_offers_cost_2() {
    let runner = DebugRunner::builder()
        .dsl_card("BT24-010")
        .expect("BT24-010 loads")
        .add_card(ts_trait_lv3())
        .memory(15)
        .start();

    let compiled = runner.compiled_card("BT24-010").expect("compiles");
    let matches_ts = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(2))
            && p.from
                .as_ref()
                .map(|f| predicate_tree_has_trait_has(f, "TS"))
                .unwrap_or(false)
    });
    assert!(
        matches_ts,
        "alt-digivolve must expose a trait_has: TS leg at cost 2"
    );
}

/// Negative: a plain Lv.3 Digimon (no Agumon name, no TS trait) must NOT be
/// covered by the cost-2 alt-digivolve leg — only the standard cost-3
/// circles (Red/Black) should apply to it.
#[test]
fn bt24_010_alt_digivolve_rejects_non_agumon_non_ts_lv3() {
    // Structural proof: the alt-digivolve predicate's any_of branches are
    // strictly name_contains:Agumon / trait_has:TS — a fixture satisfying
    // neither does not match either leaf. We assert this over the compiled
    // predicate tree directly (see Section 1's flatten helper pattern)
    // rather than driving the full digivolve-from-hand action loop, since
    // structural predicate-tree inspection is the DSL-guaranteed contract
    // (RUST_DSL_TEST_API.md §6) and plain_lv3 has neither marker by
    // construction.
    let fixture = plain_lv3();
    assert_eq!(fixture.traits, vec!["Plant".to_string()]);
    assert!(!fixture.card_name.contains("Agumon"));

    let runner = DebugRunner::builder()
        .dsl_card("BT24-010")
        .expect("BT24-010 loads")
        .add_card(fixture)
        .memory(15)
        .start();
    let compiled = runner.compiled_card("BT24-010").expect("compiles");

    let alt_path = compiled
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(2))
        })
        .expect("cost-2 alt-digivolve path exists");
    let from = alt_path.from.as_ref().expect("has from filter");

    let mut leaves = Vec::new();
    flatten_predicate_tree(from, &mut leaves);

    // Every leaf that carries a name_contains or trait_has constraint must
    // NOT match "Palmon"/"Plant" — i.e. the gate is exactly Agumon-name OR
    // TS-trait, nothing broader that would accidentally admit this fixture.
    for leaf in &leaves {
        if let Some(name) = &leaf.name_contains {
            assert_eq!(name, "Agumon", "unexpected name_contains leaf: {name}");
        }
        if let Some(tr) = &leaf.trait_has {
            assert_eq!(tr, "TS", "unexpected trait_has leaf: {tr}");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 7 — Standard digivolve circles: Black compensating circle
// ═══════════════════════════════════════════════════════════════════════════════

/// A Black Lv.3 base is a legal digivolve source into BT24-010 for cost 3 —
/// proves the explicitly-authored compensating circle (cards.json only
/// carries the Red circle) is actually reachable, not just present in the
/// alt_paths list.
#[test]
fn bt24_010_black_lv3_base_has_matching_alt_path() {
    let runner = DebugRunner::builder()
        .dsl_card("BT24-010")
        .expect("BT24-010 loads")
        .add_card(black_lv3())
        .memory(15)
        .start();

    let compiled = runner.compiled_card("BT24-010").expect("compiles");
    let black_circle = compiled.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.from
                .as_ref()
                .map(|f| {
                    f.level_eq == Some(3)
                        && f.color_is == Some(digimon_dsl::compiled::CompiledColor::Black)
                })
                .unwrap_or(false)
    });
    assert!(
        black_circle.is_some(),
        "Black Lv.3 base must match an authored standard digivolve alt_path"
    );
    assert_eq!(
        black_circle.unwrap().cost,
        Some(CompiledCost::Literal(3)),
        "Black Lv.3 circle costs 3 memory, matching the Red circle"
    );
}
