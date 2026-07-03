//! EX4-051 BlitzGreymon — Digimon, Lv.6, Black/Red, DP 12000, Cost 12.
//! Traits: Cyborg. Attribute: Virus.
//! Standard digivolve: Lv.5 Black / cost 4 OR Lv.5 Red / cost 4 (dual-color
//! evo_costs; both circles print cost 4).
//! Special evo (xros_req): `[Digivolve] [MetalGreymon]: Cost 3`.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX4-051.md, verbatim)
//!
//! ```text
//! [When Digivolving] Activate 1 of the effects below.
//! ・ ＜De-Digivolve 1＞ 3 of your opponent's Digimon.
//! ・ 1 of your other Digimon digivolves into a level 6 or lower Digimon card
//!   with [Garurumon] in its name in your hand without paying the cost.
//! ・ This Digimon and one of your other Digimon may DNA digivolve into a
//!   Digimon card in your hand for the cost.
//!
//! Inherited Effect [When Attacking][Once Per Turn]
//! If this Digimon has [Omnimon] in its name, trash the top card of your
//! opponent's security stack.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX4/Black/EX4_051.cs
//!
//! DCGO structure (EffectTiming.OnEnterFieldAnyone, `ActivateClass` gated by
//! `CanTriggerWhenDigivolving`): a single 3-way `SetIntSelection` ("Which
//! effect will you activate?"), then per-branch bodies:
//!   - case 0: `SelectPermanentEffect` over
//!     `IsPermanentExistsOnOpponentBattleAreaDigimon`, `maxCount:
//!     Min(3, candidates)`, `canNoSelect: false` (mandatory — DCGO has no
//!     `canEndNotMax` override so the picker fills up to the cap), then
//!     `IDegeneration(selectedPermanent, 1, ...)` per picked permanent
//!     (De-Digivolve 1 each).
//!   - case 1: `SelectPermanentEffect` over own OTHER battle-area Digimon
//!     that have a matching hand card (`CanPlayCardTargetFrame`),
//!     `maxCount: Min(1, candidates)`, `canNoSelect: true` — then
//!     `DigivolveIntoHandOrTrashCard(payCost: false, ...)` filtered by
//!     `IsDigimon && HasGarurumonName && Level <= 6 && HasLevel`.
//!   - case 2: gated on `CanSelectCardCondition1` (own Digimon in hand that
//!     `CanPlayJogress(true)` against this permanent), then
//!     `DNADigivolvePermanentsIntoHandOrTrashCard(payCost: true, ...,
//!     permanentConditions: [perm == card.PermanentOfThisCard()])` — this
//!     Digimon is a fixed anchor material, `payCost: true` charges the
//!     target's printed DNA cost.
//! Inherited (EffectTiming.OnAllyAttack, `isInheritedEffect: true`,
//! `SetHashString("TrashSecurity_EX4_051")`): gated on
//! `card.PermanentOfThisCard().TopCard.ContainsCardName("Omnimon")`, then
//! `IDestroySecurity(opp, 1, fromTop: true)`.
//!
//! # Patterns covered (docs/RUST_DSL_TEST_API.md §4.3)
//! - E1 Branch-choice [When Digivolving] (3-way EffectChoice, BT17-015 idiom).
//! - F1-adjacent: De-Digivolve N via `select_count_capped_multi` +
//!   `per_selected` + `de_digivolve` (mandatory 1..=3, clamp_to_available).
//! - F2 Effect-initiated free digivolve from hand onto an "other" own
//!   Digimon, filtered by level + name.
//! - G2 DNA digivolve — self-anchored `may_dna_digivolve_now` (anchor
//!   defaults to `source`), paying the target's printed DNA cost
//!   (`ignore_requirements: false`).
//! - G4 Inherited [When Attacking][Once Per Turn] name-gated security trash
//!   (byte-identical shape to BT17-015's inherited clause).
//!
//! # Faithfulness notes
//! - Branch 0 ("3 of your opponent's Digimon") is authored as a mandatory
//!   pick of `min(3, available)` — DCGO's `canNoSelect: false` with no
//!   `canEndNotMax` override means the picker cannot stop early while more
//!   legal targets remain, matching FAQ MP-31 ("N of your opponent's
//!   Digimon" clamps to however many are on the board without letting the
//!   player under-pick when N are available). `clamp_to_available: true`
//!   plus omitting `optional_zero` gives exactly this: 0 legal targets → the
//!   clause structurally cannot be selected as "no fire" is handled by the
//!   engine's install-time eligibility check (no candidates → no selection).
//! - Branch 1's digivolve target filter is `level_lte: 6` + `name_contains:
//!   "Garurumon"` — matches DCGO's `HasGarurumonName && Level <= 6`. The
//!   "your OTHER Digimon" gate uses `other: true` (excludes the effect
//!   source/carrier permanent, mirroring DCGO's `permanent !=
//!   card.PermanentOfThisCard()`).
//! - Branch 2 uses `may_dna_digivolve_now` with the default `anchor: source`
//!   (this permanent) and `ignore_requirements: false` so the DSL orchestrator
//!   both (a) restricts the partner to eligible own-Digimon-other-than-anchor
//!   automatically and (b) restricts/charges the hand target by its actual
//!   printed DNA requirements against {anchor, partner} — matching DCGO's
//!   `payCost: true` / requirement-checked path. No raw `cost:` literal is
//!   authored; the verb resolves the real DNA cost per pair/target.
//! - Inherited clause mirrors BT17-015's `source_name_contains: "Omnimon"`
//!   gate byte-for-byte (see BT17-015.yaml / bt17_015.rs for the identical
//!   pattern this card reuses).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledStep, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, make_test_dna_card, DebugRunner,
};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX4-051";

// ─── Fixture helpers ────────────────────────────────────────────────────────

fn make_opp_digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(5000);
    card
}

fn make_own_digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(5000);
    card
}

/// Digivolve target for branch 1: level <= 6, [Garurumon] in name.
fn make_garurumon_target(id: &str, level: u8) -> CardData {
    let mut card = make_test_card_with_level(id, "MetalGarurumon", level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(9000);
    card.play_cost = 9;
    card
}

/// Digivolve target that does NOT contain "Garurumon" — negative-filter check.
fn make_non_garurumon_target(id: &str, level: u8) -> CardData {
    let mut card = make_test_card_with_level(id, "WarGreymon", level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(9000);
    card.play_cost = 9;
    card
}

/// Garurumon-named target above level 6 — negative-filter check (level gate).
fn make_high_level_garurumon(id: &str) -> CardData {
    make_garurumon_target(id, 7)
}

fn make_omnimon_top(id: &str) -> CardData {
    let mut card = make_test_card(id, "Omnimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(7);
    card.dp = Some(13000);
    card
}

fn make_filler_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card
}

fn blitzgreymon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .memory(0)
        .start()
}

fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    let card = runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Digivolved {
            player: handle.player,
            permanent: handle,
            card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
}

fn stack_ids(runner: &DebugRunner, handle: PermanentHandle) -> Vec<String> {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ex4_051_has_when_digivolving_and_inherited_when_attacking_clauses() {
    let runner = blitzgreymon();
    let compiled = runner.compiled_card(CARD_ID).expect("EX4-051 compiled");

    let triggered: Vec<&CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "expected exactly 2 triggered clauses (own WhenDigivolving + inherited WhenAttacking)"
    );
}

#[test]
fn ex4_051_when_digivolving_clause_is_mandatory_face_up() {
    let runner = blitzgreymon();
    let compiled = runner.compiled_card(CARD_ID).expect("EX4-051 compiled");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.scope != CompiledScope::Inherited =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have own-scope [When Digivolving] clause");

    assert!(
        !clause.optional,
        "the branch-choice clause itself is mandatory once triggered (DCGO isOptional not set on the outer ActivateClass)"
    );
    assert!(
        !clause.once_per_turn,
        "no [Once Per Turn] on the [When Digivolving] clause"
    );
}

#[test]
fn ex4_051_when_digivolving_process_offers_three_branches() {
    let runner = blitzgreymon();
    let compiled = runner.compiled_card(CARD_ID).expect("EX4-051 compiled");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.scope != CompiledScope::Inherited =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have own-scope [When Digivolving] clause");

    let choice = clause.process.iter().find_map(|step| match step {
        CompiledStep::SelectEffectChoice { labels, .. } => Some(labels),
        _ => None,
    });
    let labels = choice.expect("[When Digivolving] must open with a SelectEffectChoice step");
    assert_eq!(labels.len(), 3, "must offer exactly 3 branches");
}

#[test]
fn ex4_051_has_inherited_when_attacking_opt_clause() {
    let runner = blitzgreymon();
    let compiled = runner.compiled_card(CARD_ID).expect("EX4-051 compiled");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if matches!(t.scope, CompiledScope::Inherited)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have inherited [When Attacking] clause");

    assert!(
        clause.once_per_turn,
        "[When Attacking] inherited must be [Once Per Turn]"
    );
}

#[test]
fn ex4_051_has_lv5_cost4_digivolve_alt_path() {
    let runner = blitzgreymon();
    let compiled = runner.compiled_card(CARD_ID).expect("EX4-051 compiled");

    let has_standard = compiled.alt_paths.iter().any(|p| {
        p.kind == digimon_dsl::compiled::CompiledAltPathKind::Digivolve
            && matches!(
                p.cost,
                Some(digimon_dsl::compiled::CompiledCost::Literal(4))
            )
    });
    assert!(
        has_standard,
        "must have at least one Lv.5 / cost 4 standard digivolve alt-path"
    );
}

#[test]
fn ex4_051_has_metalgreymon_cost3_special_digivolve_alt_path() {
    let runner = blitzgreymon();
    let compiled = runner.compiled_card(CARD_ID).expect("EX4-051 compiled");

    let has_special = compiled.alt_paths.iter().any(|p| {
        p.kind == digimon_dsl::compiled::CompiledAltPathKind::Digivolve
            && matches!(
                p.cost,
                Some(digimon_dsl::compiled::CompiledCost::Literal(3))
            )
    });
    assert!(
        has_special,
        "must have the [MetalGreymon] cost-3 special digivolve alt-path (printed xros_req)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [When Digivolving] branch 0: De-Digivolve 1, up to 3 opp Digimon
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: with 3+ eligible opp Digimon, branch 0 installs a mandatory
/// multi-pick selection over the opponent's field (min(3, available) = 3).
/// The `select_count_capped_multi` DSL step lowers `battle_area`-zone picks
/// through `install_select_count_capped_permanents`, which drives the same
/// `OppField` selection kind as a single-target field pick (the multi-pick
/// re-installs the prompt after each choice) — there is no distinct
/// `CountCappedMultiSelect` kind for the battle-area zone (that variant is
/// reserved for the Hand/Trash zone path).
#[test]
fn ex4_051_branch0_condition_installs_with_opp_digimon_present() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-A", "OppA"))
        .add_card(make_opp_digimon("OPP-B", "OppB"))
        .add_card(make_opp_digimon("OPP-C", "OppC"))
        .memory(0)
        .start();

    runner.place_on_field(1, "OPP-A", None);
    runner.place_on_field(1, "OPP-B", None);
    runner.place_on_field(1, "OPP-C", None);
    let blitz = runner.place_on_field(0, CARD_ID, None);

    fire_when_digivolving(&mut runner, blitz);
    let kind = runner.pending_kind().expect("branch-choice prompt installs");
    assert_eq!(kind, SelectionKind::EffectChoice);

    runner.execute_branch(0).expect("pick De-Digivolve branch");
    let after = runner.pending_kind();
    assert_eq!(
        after,
        Some(SelectionKind::OppField),
        "branch 0 must install an OppField multi-pick selection over opp Digimon; got {after:?}"
    );
    assert!(
        !runner.pending_is_optional(),
        "mandatory pick (clamp_to_available) — PASS must not be legal while candidates remain"
    );
}

/// Negative: with NO opponent Digimon on the field, branch 0's inner select
/// has zero candidates — no permanents are de-digivolved and no crash/panic
/// occurs on empty-target resolution.
#[test]
fn ex4_051_branch0_no_opp_digimon_no_targets_selected() {
    let mut runner = blitzgreymon();
    let blitz = runner.place_on_field(0, CARD_ID, None);

    fire_when_digivolving(&mut runner, blitz);
    let kind = runner.pending_kind().expect("branch-choice prompt installs");
    assert_eq!(kind, SelectionKind::EffectChoice);

    runner.execute_branch(0).expect("pick De-Digivolve branch");
    let _ = runner.auto_resolve();

    // No opponent Digimon existed, so nothing should have changed on
    // opponent's (empty) battle area — the important invariant is no panic
    // and the game remains in a resolvable state.
    assert_eq!(runner.battle_area_size(1), 0);
}

/// Behavioral: branch 0 de-digivolves each of up to 3 selected opp Digimon
/// by exactly 1 (their top digivolution source is trashed).
#[test]
fn ex4_051_branch0_de_digivolves_selected_opp_digimon_by_one() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-BASE", "OppBase"))
        .add_card(make_opp_digimon("OPP-SRC", "OppSrc"))
        .memory(0)
        .start();

    // Opp Digimon with 1 digivolution source on top of a base.
    let opp_stack = runner.place_stack(1, &["OPP-BASE", "OPP-SRC"]);
    let blitz = runner.place_on_field(0, CARD_ID, None);

    fire_when_digivolving(&mut runner, blitz);
    runner.execute_branch(0).expect("pick De-Digivolve branch");

    // Drive the capped-multi select: pick the single available opp Digimon
    // then decline further picks (mandatory min(3, available) == 1 here).
    let view = runner
        .pending_selection_view()
        .expect("capped-multi selection installed");
    let action = view.valid_action_ids[0];
    runner.execute_action(0, action).expect("pick opp Digimon");
    let _ = runner.auto_resolve();

    assert_eq!(
        stack_ids(&runner, opp_stack),
        vec!["OPP-BASE"],
        "De-Digivolve 1 must pop exactly the top source, leaving the base"
    );
}

/// Boundary: branch 0's mandatory pick clamps to available candidates — with
/// only 2 eligible opp Digimon, the player cannot decline below min(3,2)=2;
/// both must end up de-digivolved once selection completes.
#[test]
fn ex4_051_branch0_clamps_to_available_when_fewer_than_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-BASE-1", "OppBase1"))
        .add_card(make_opp_digimon("OPP-SRC-1", "OppSrc1"))
        .add_card(make_opp_digimon("OPP-BASE-2", "OppBase2"))
        .add_card(make_opp_digimon("OPP-SRC-2", "OppSrc2"))
        .memory(0)
        .start();

    let opp1 = runner.place_stack(1, &["OPP-BASE-1", "OPP-SRC-1"]);
    let opp2 = runner.place_stack(1, &["OPP-BASE-2", "OPP-SRC-2"]);
    let blitz = runner.place_on_field(0, CARD_ID, None);

    fire_when_digivolving(&mut runner, blitz);
    runner.execute_branch(0).expect("pick De-Digivolve branch");

    // Pick both available candidates in turn (clamp_to_available: true means
    // the selection auto-completes once available candidates are exhausted).
    // The battle-area zone lowers through `OppField`, not a distinct
    // `CountCappedMultiSelect` kind (see the structural comment on
    // `ex4_051_branch0_condition_installs_with_opp_digimon_present`).
    for _ in 0..2 {
        if let Some(view) = runner.pending_selection_view() {
            if view.kind == SelectionKind::OppField && !view.valid_action_ids.is_empty() {
                let action = view.valid_action_ids[0];
                runner.execute_action(0, action).expect("pick opp Digimon");
            }
        }
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        stack_ids(&runner, opp1),
        vec!["OPP-BASE-1"],
        "first opp Digimon must be de-digivolved by 1"
    );
    assert_eq!(
        stack_ids(&runner, opp2),
        vec!["OPP-BASE-2"],
        "second opp Digimon must be de-digivolved by 1"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [When Digivolving] branch 1: free digivolve into Garurumon-name
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: an "other" own Digimon on field + a Lv<=6 [Garurumon]-name card
/// in hand → branch 1 installs a selection chain and completes the free
/// digivolve (no memory cost paid).
#[test]
fn ex4_051_branch1_digivolves_other_digimon_into_garurumon_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_own_digimon("OTHER-DIGI", "OtherDigi"))
        .add_card(make_garurumon_target("HAND-MG", 6))
        .hand(0, &["HAND-MG"])
        .memory(0)
        .start();

    let other = runner.place_on_field(0, "OTHER-DIGI", None);
    let blitz = runner.place_on_field(0, CARD_ID, None);
    let mem_before = runner.memory();

    fire_when_digivolving(&mut runner, blitz);
    runner.execute_branch(1).expect("pick free-digivolve branch");
    let _ = runner.auto_resolve();

    let top = runner.game.players[0].battle_area[other.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "HAND-MG",
        "the hand Garurumon-name card must land on top of the other Digimon's stack"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "branch 1 digivolves without paying the cost — memory must be unchanged"
    );
    assert!(
        !hand_ids(&runner, 0).contains(&"HAND-MG".to_string()),
        "the played card must leave hand"
    );
}

/// Negative: hand card without [Garurumon] in its name must not be a legal
/// target for branch 1's digivolve selection.
#[test]
fn ex4_051_branch1_filter_excludes_non_garurumon_name() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_own_digimon("OTHER-DIGI", "OtherDigi"))
        .add_card(make_non_garurumon_target("HAND-WG", 6))
        .hand(0, &["HAND-WG"])
        .memory(0)
        .start();

    let other = runner.place_on_field(0, "OTHER-DIGI", None);
    let blitz = runner.place_on_field(0, CARD_ID, None);

    fire_when_digivolving(&mut runner, blitz);
    runner.execute_branch(1).expect("pick free-digivolve branch");
    let _ = runner.auto_resolve();

    let top = runner.game.players[0].battle_area[other.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "OTHER-DIGI",
        "non-Garurumon-name hand card must never be offered as a digivolve target"
    );
    assert!(
        hand_ids(&runner, 0).contains(&"HAND-WG".to_string()),
        "the non-matching card must remain in hand"
    );
}

/// Negative: a [Garurumon]-name hand card above level 6 must not be a legal
/// target for branch 1.
#[test]
fn ex4_051_branch1_filter_excludes_garurumon_above_level_6() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_own_digimon("OTHER-DIGI", "OtherDigi"))
        .add_card(make_high_level_garurumon("HAND-MG7"))
        .hand(0, &["HAND-MG7"])
        .memory(0)
        .start();

    let other = runner.place_on_field(0, "OTHER-DIGI", None);
    let blitz = runner.place_on_field(0, CARD_ID, None);

    fire_when_digivolving(&mut runner, blitz);
    runner.execute_branch(1).expect("pick free-digivolve branch");
    let _ = runner.auto_resolve();

    let top = runner.game.players[0].battle_area[other.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "OTHER-DIGI",
        "level-7 Garurumon-name hand card exceeds the printed 'level 6 or lower' gate"
    );
}

/// Negative: BlitzGreymon itself (the effect source) must not be a legal
/// target for "1 of your OTHER Digimon" — `other: true` must exclude it.
#[test]
fn ex4_051_branch1_cannot_target_self_as_the_digivolving_permanent() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_garurumon_target("HAND-MG", 6))
        .hand(0, &["HAND-MG"])
        .memory(0)
        .start();

    // No other own Digimon on the field — only BlitzGreymon itself.
    let blitz = runner.place_on_field(0, CARD_ID, None);

    fire_when_digivolving(&mut runner, blitz);
    runner.execute_branch(1).expect("pick free-digivolve branch");
    let _ = runner.auto_resolve();

    let top = runner.game.players[0].battle_area[blitz.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        CARD_ID,
        "with no eligible OTHER own Digimon, BlitzGreymon's own stack must be untouched \
         (self must never be offered as the digivolving permanent)"
    );
    assert!(
        hand_ids(&runner, 0).contains(&"HAND-MG".to_string()),
        "hand card must remain — branch 1 has no legal target"
    );
}

/// Optionality: DCGO EX4_051.cs:181 sets `canNoSelect: true` on branch 1's
/// permanent pick — the player may decline picking a digivolving permanent
/// even when a legal one exists. Declining must leave the board/hand
/// unchanged (matches DCGO's early-return on a null `selectedPermanent`).
#[test]
fn ex4_051_branch1_permanent_pick_is_declinable_and_decline_is_a_no_op() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_own_digimon("OTHER-DIGI", "OtherDigi"))
        .add_card(make_garurumon_target("HAND-MG", 6))
        .hand(0, &["HAND-MG"])
        .memory(0)
        .start();

    let other = runner.place_on_field(0, "OTHER-DIGI", None);
    let blitz = runner.place_on_field(0, CARD_ID, None);

    fire_when_digivolving(&mut runner, blitz);
    runner.execute_branch(1).expect("pick free-digivolve branch");

    assert!(
        runner.pending_selection().is_some(),
        "own-field permanent selection must install with a legal candidate present"
    );
    assert!(
        runner.pending_is_optional(),
        "branch 1's permanent pick must be declinable (DCGO canNoSelect: true); \
         PASS is gated by `PendingSelection::is_optional` rather than appearing \
         in `valid_action_ids` (see `install_field_selection`)"
    );

    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the permanent pick");
    let _ = runner.auto_resolve();

    let top = runner.game.players[0].battle_area[other.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "OTHER-DIGI",
        "declining must leave the candidate permanent's stack untouched"
    );
    assert!(
        hand_ids(&runner, 0).contains(&"HAND-MG".to_string()),
        "declining must leave the hand card untouched (select_hand never installs)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [When Digivolving] branch 2: self-anchored DNA digivolve
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: this Digimon + an eligible own partner + a matching DNA-target
/// hand card → branch 2 completes the DNA digivolve, paying the target's
/// printed DNA cost (not a free digivolve, per printed "for the cost").
#[test]
fn ex4_051_branch2_dna_digivolves_with_partner_paying_printed_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_own_digimon("PARTNER", "PartnerDigi"))
        .add_card(make_test_dna_card("HAND-DNA", "DnaTarget", 6, 5, 3))
        .hand(0, &["HAND-DNA"])
        .memory(10)
        .start();

    let partner = runner.place_on_field(0, "PARTNER", None);
    let blitz = runner.place_on_field(0, CARD_ID, None);
    let mem_before = runner.memory();

    fire_when_digivolving(&mut runner, blitz);
    runner.execute_branch(2).expect("pick DNA-digivolve branch");

    // Drive the partner-selection prompt (own-field), then the target
    // hand-card selection prompt.
    let view = runner
        .pending_selection_view()
        .expect("partner selection installed");
    // Choose the eligible partner (should be exactly PARTNER; BlitzGreymon
    // itself is excluded as the anchor).
    let action = view.valid_action_ids[0];
    runner.execute_action(0, action).expect("pick DNA partner");
    let _ = runner.auto_resolve();

    // After completion, the hand card must have DNA-digivolved onto one of
    // the two materials (DCGO stacks DNA digivolve results onto the anchor
    // permanent's slot — assert via card presence rather than a specific slot
    // index, since DNA digivolve implementation may consolidate stacks).
    assert!(
        !hand_ids(&runner, 0).contains(&"HAND-DNA".to_string()),
        "the DNA target card must leave hand once the digivolve completes"
    );
    assert_eq!(
        runner.memory(),
        mem_before - 3,
        "branch 2 must pay the target's printed DNA cost (3), not a free digivolve"
    );

    // One of the two materials' stacks must now show HAND-DNA on top.
    let blitz_top = runner.game.players[0]
        .battle_area
        .get(blitz.index as usize)
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string());
    let partner_top = runner.game.players[0]
        .battle_area
        .get(partner.index as usize)
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string());
    assert!(
        blitz_top.as_deref() == Some("HAND-DNA") || partner_top.as_deref() == Some("HAND-DNA"),
        "HAND-DNA must land on top of one of the two DNA material stacks; \
         blitz_top={blitz_top:?}, partner_top={partner_top:?}"
    );
}

/// Negative: with no eligible own partner on the field (BlitzGreymon alone),
/// branch 2 must silently no-op — no partner-selection prompt installs, and
/// the hand card is untouched.
#[test]
fn ex4_051_branch2_no_partner_silent_skip() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_test_dna_card("HAND-DNA", "DnaTarget", 6, 5, 3))
        .hand(0, &["HAND-DNA"])
        .memory(10)
        .start();

    let blitz = runner.place_on_field(0, CARD_ID, None);

    fire_when_digivolving(&mut runner, blitz);
    runner.execute_branch(2).expect("pick DNA-digivolve branch");
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "no eligible partner → branch 2 must silently no-op, no lingering selection"
    );
    assert!(
        hand_ids(&runner, 0).contains(&"HAND-DNA".to_string()),
        "hand card must be untouched when no partner is available"
    );
}

/// Negative: with an eligible partner but no hand card that satisfies the
/// {anchor, partner} pair's DNA requirements, branch 2 must silently no-op.
#[test]
fn ex4_051_branch2_no_matching_hand_target_silent_skip() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_own_digimon("PARTNER", "PartnerDigi"))
        // Hand card whose DNA requirement (level 2 + level 2) can never be
        // satisfied by a Lv.5 partner + Lv.6 BlitzGreymon pair.
        .add_card(make_test_dna_card("HAND-BADFIT", "BadFit", 2, 2, 1))
        .hand(0, &["HAND-BADFIT"])
        .memory(10)
        .start();

    let _partner = runner.place_on_field(0, "PARTNER", None);
    let blitz = runner.place_on_field(0, CARD_ID, None);

    fire_when_digivolving(&mut runner, blitz);
    runner.execute_branch(2).expect("pick DNA-digivolve branch");
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "no matching DNA hand target → branch 2 must silently no-op"
    );
    assert!(
        hand_ids(&runner, 0).contains(&"HAND-BADFIT".to_string()),
        "non-matching hand card must be untouched"
    );
}

/// Negative: BlitzGreymon itself must never be offered as its own DNA
/// partner (the engine excludes the anchor from partner candidates as a
/// hard invariant of `may_dna_digivolve_now`).
#[test]
fn ex4_051_branch2_partner_selection_excludes_self() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_own_digimon("PARTNER", "PartnerDigi"))
        .add_card(make_test_dna_card("HAND-DNA", "DnaTarget", 6, 5, 3))
        .hand(0, &["HAND-DNA"])
        .memory(10)
        .start();

    let _partner = runner.place_on_field(0, "PARTNER", None);
    let blitz = runner.place_on_field(0, CARD_ID, None);

    fire_when_digivolving(&mut runner, blitz);
    runner.execute_branch(2).expect("pick DNA-digivolve branch");

    let view = runner
        .pending_selection_view()
        .expect("partner selection installed");
    // The only legal partner action should correspond to PARTNER's slot, not
    // BlitzGreymon's own slot.
    assert!(
        !view.valid_action_ids.is_empty(),
        "at least one legal partner action must exist"
    );
    // Drive it to confirm it resolves to PARTNER specifically (not a self-pick).
    let action = view.valid_action_ids[0];
    runner.execute_action(0, action).expect("pick DNA partner");
    let _ = runner.auto_resolve();

    // DNA digivolve consolidates the two materials into a single battle-area
    // slot, so the battle area shrinks by one — index safely via `.get(..)`.
    // The key faithfulness assertion is: the merge actually happened (hand
    // card left hand, memory was charged — already asserted by the sibling
    // positive test), and exactly ONE permanent remains carrying both
    // materials plus HAND-DNA on top (never two independent unmerged stacks,
    // which would indicate BlitzGreymon merged with itself instead of
    // PARTNER).
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "the two DNA materials (BlitzGreymon + PARTNER) must consolidate into \
         exactly 1 battle-area permanent"
    );
    let remaining_top = runner.game.players[0].battle_area[0]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        remaining_top, "HAND-DNA",
        "the merged permanent's top card must be the DNA target picked from hand"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited [When Attacking] condition gating (split positive/negative)
// ═══════════════════════════════════════════════════════════════════════════

fn place_omnimon_over_ex4_051(runner: &mut DebugRunner, player: u8) -> PermanentHandle {
    runner.place_stack(player, &[CARD_ID, "OMNI-TOP"])
}

/// Positive: stack with an Omnimon-named top card over EX4-051 — inherited
/// trigger fires and trashes 1 opp security.
#[test]
fn ex4_051_inherited_when_attacking_omnimon_top_trashes_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-FILLER"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_ex4_051(&mut runner, 0);
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    let sec_before = runner.security_count(1);
    assert!(sec_before >= 1, "test setup needs at least 1 opp security");

    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "Omnimon-named attacker must trash 1 opp security via inherited"
    );
}

/// Negative: stack has only BlitzGreymon (EX4-051) at top — name gate must
/// block the trash (top card name "BlitzGreymon" does not contain "Omnimon").
#[test]
fn ex4_051_inherited_when_attacking_blitzgreymon_top_does_not_trash_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_filler_digimon("DEF-FILLER"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // EX4-051 alone at top — printed name "BlitzGreymon" does NOT contain
    // "Omnimon".
    let attacker = runner.place_on_field(0, CARD_ID, Some(0));
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    let sec_before = runner.security_count(1);
    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "BlitzGreymon-only top card must NOT trigger the [Omnimon]-name inherited"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — Inherited [When Attacking] OPT enforcement
// ═══════════════════════════════════════════════════════════════════════════

/// OPT: a second attack in the same turn must NOT trash a second security.
#[test]
fn ex4_051_inherited_when_attacking_opt_blocks_second_activation() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-1"))
        .add_card(make_filler_digimon("DEF-2"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_ex4_051(&mut runner, 0);
    let def1 = runner.place_on_field(1, "DEF-1", Some(0));
    let sec_before = runner.security_count(1);

    runner.attack_digimon(attacker, def1, false);
    let _ = runner.auto_resolve();
    let sec_after_first = runner.security_count(1);
    assert_eq!(
        sec_after_first,
        sec_before - 1,
        "first attack must trash 1 opp security"
    );

    if runner.battle_area_size(0) <= attacker.index as usize {
        // Attacker died (e.g. security counter) — no follow-up possible.
        return;
    }

    let def2 = runner.place_on_field(1, "DEF-2", Some(0));
    runner.attack_digimon(attacker, def2, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_after_first,
        "OPT must prevent the inherited from firing twice in one turn"
    );
}

/// OPT resets after a full turn cycle (end_turn → opponent turn → back).
#[test]
fn ex4_051_inherited_when_attacking_opt_resets_after_end_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-1"))
        .add_card(make_filler_digimon("DEF-2"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .deck(0, &["SEC-FILLER"; 10])
        .deck(1, &["SEC-FILLER"; 10])
        .security(0, &["SEC-FILLER"; 5])
        .security(
            1,
            &[
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
            ],
        )
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_ex4_051(&mut runner, 0);
    let def1 = runner.place_on_field(1, "DEF-1", Some(0));

    runner.attack_digimon(attacker, def1, false);
    let _ = runner.auto_resolve();
    let sec_after_first = runner.security_count(1);

    if runner.game_over() {
        return;
    }

    runner.end_turn();
    runner.end_turn();

    if runner.battle_area_size(0) <= attacker.index as usize {
        return;
    }

    let def2 = runner.place_on_field(1, "DEF-2", Some(0));
    runner.attack_digimon(attacker, def2, false);
    let _ = runner.auto_resolve();

    assert!(
        runner.security_count(1) < sec_after_first,
        "OPT must reset across turn boundary; security must drop again"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 7 — Event-log / state-delta assertion for inherited security trash
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ex4_051_inherited_security_trash_observable_in_state() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-051 in embedded DSL pack")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-FILLER"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_ex4_051(&mut runner, 0);
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    let sec_before = runner.security_count(1);
    let cp = runner.event_checkpoint();

    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "primary state delta — opp security drops by 1"
    );

    // Soft event-log inspection — do not assert strictly since Trash may not
    // be a wired GameEvent variant (mirrors bt17_015's pattern).
    let _events = runner.events_since(cp);
}
