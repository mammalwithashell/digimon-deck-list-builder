//! EX10-028 Golemon — Digimon, Black, Lv.4, Cost 4, DP 4000.
//! Traits: [Mineral, LIBERATOR]. Attribute: Virus.
//!
//! # Card text (data/cards.json — verbatim)
//!
//! **[On Play] [When Digivolving]** By trashing any 1 card with the [Mineral]
//! or [Rock] trait from your Digimon's digivolution cards, 1 of your Digimon
//! with the [Mineral] or [Rock] trait gains ＜Reboot＞, ＜Blocker＞ and +3000
//! DP until your opponent's turn ends.
//!
//! **Inherited:** When effects trash this card from a [Mineral] or [Rock] trait
//! Digimon's digivolution cards, delete 1 of your opponent's Digimon with a
//! play cost of 4 or less.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Black/EX10_028.cs
//!
//! # Patterns this test covers
//! - D1  DP buff +3000 with end_of_opponents_turn expiry (EX8-070 exemplar)
//! - H5  Blocker keyword grant (procedural, temporary, EoOT expiry)
//! - H7  Reboot keyword grant (procedural, temporary, EoOT expiry)
//! - E2  "By trashing" cost gate — select_own_sources (Mineral/Rock filter)
//!       → select_own_permanent (buff target), THEN trash_selected_sources
//! - Multi-timing: [On Play] AND [When Digivolving] trigger Clause 1
//! - Inherited on_digivolution_card_trashed: delete 1 opp Digimon cost ≤4
//! - Expiry: buffs persist through own turn end, expire after opponent's turn
//! - Negative: no Mineral/Rock source → no selection, no buffs
//! - Negative: no Mineral/Rock Digimon target → source selection still fires,
//!   but buff steps silently skip (no target)
//! - Negative: inherited clause not triggered when source card is not EX10-028
//! - Negative: inherited clause not triggered from a non-Mineral/non-Rock host

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::encode_source_select;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A Lv.3 Digimon source with Mineral trait — eligible cost card.
fn make_mineral_source(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} MineralSrc"));
    c.traits.push("Mineral".to_string());
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// A Lv.3 Digimon source with Rock trait — eligible cost card.
fn make_rock_source(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} RockSrc"));
    c.traits.push("Rock".to_string());
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// A Lv.4 Digimon with Mineral trait — eligible buff target.
fn make_mineral_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} MineralDigi"));
    c.traits.push("Mineral".to_string());
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

/// A Lv.4 Digimon with Rock trait — eligible buff target.
fn make_rock_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} RockDigi"));
    c.traits.push("Rock".to_string());
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

/// A plain filler card (no special traits).
fn make_filler(id: &str) -> CardData {
    make_test_card(id, &format!("{id} Filler"))
}

/// Base runner: EX10-028 loaded, memory 10 for cost-free play testing.
fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .memory(10)
        .build()
}

/// Accept clause 1's outer accept/decline prompt.
///
/// §15-7-1: "Optional processing conditions include text such as 'by X, Y.'" —
/// EX10-028's "By trashing any 1 card with the [Mineral] or [Rock] trait ...,
/// 1 of your Digimon ... gains ＜Reboot＞, ＜Blocker＞ and +3000 DP" is exactly
/// that shape, and §15-7-4 gives the player the choice of whether to execute
/// it. So clause 1 installs a `SelectionKind::Replacement` accept/decline
/// prompt BEFORE the SourceMulti cost selection. DCGO does the same:
/// EX10_028.cs:20 ([On Play]) and :159 ([When Digivolving]) both pass
/// `isOptional: true` to `SetUpActivateClass`.
fn accept_optional_cost(runner: &mut DebugRunner) {
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "the optional processing condition must surface an accept/decline prompt first (rule 17); got {:?}",
        runner.pending_kind()
    );
    runner
        .accept_optional_trigger()
        .expect("accepting the optional processing condition must be legal");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX10-028 must have exactly two triggered clauses:
/// - Clause 1: on_play + when_digivolving (face-up scope)
/// - Clause 2: on_digivolution_card_trashed (inherited scope)
#[test]
fn ex10_028_has_two_triggered_clauses() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX10-028")
        .expect("EX10-028 in embedded pack");

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
        2,
        "EX10-028 must have exactly 2 triggered clauses (face-up on_play/when_digivolving + inherited); got {}",
        triggered.len()
    );
}

/// Clause 1 fires on OnPlay timing.
#[test]
fn ex10_028_clause1_has_on_play_timing() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX10-028")
        .expect("EX10-028 in embedded pack");

    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )),
        "EX10-028 must have an OnPlay timing in Clause 1"
    );
}

/// Clause 1 fires on WhenDigivolving timing.
#[test]
fn ex10_028_clause1_has_when_digivolving_timing() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX10-028")
        .expect("EX10-028 in embedded pack");

    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving)
        )),
        "EX10-028 must have a WhenDigivolving timing in Clause 1"
    );
}

/// Clause 1 has both OnPlay and WhenDigivolving on the same triggered clause.
#[test]
fn ex10_028_clause1_has_both_on_play_and_when_digivolving_on_same_clause() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX10-028")
        .expect("EX10-028 in embedded pack");

    let dual_timing_clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving) =>
        {
            Some(t)
        }
        _ => None,
    });

    assert!(
        dual_timing_clause.is_some(),
        "EX10-028 Clause 1 must carry both OnPlay and WhenDigivolving timings"
    );
}

/// Clause 1 is face-up (own) scope — not inherited, not security.
#[test]
fn ex10_028_clause1_is_face_up_scope() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX10-028")
        .expect("EX10-028 in embedded pack");

    let clause1 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OnPlay clause must exist");

    assert_eq!(
        clause1.scope,
        CompiledScope::FaceUp,
        "Clause 1 must be FaceUp scope"
    );
}

/// Clause 2 is inherited scope with OnDigivolutionCardTrashed timing.
#[test]
fn ex10_028_has_inherited_on_digivolution_card_trashed_clause() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX10-028")
        .expect("EX10-028 in embedded pack");

    let inherited = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when == vec![CompiledTiming::OnDigivolutionCardTrashed] =>
        {
            Some(t)
        }
        _ => None,
    });

    assert!(
        inherited.is_some(),
        "EX10-028 must have an inherited OnDigivolutionCardTrashed clause"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1 positive: [On Play] path
//
// When EX10-028 is played with a Mineral/Rock Digimon that has a Mineral/Rock
// source card, the effect chain is:
//   1. SourceMulti { min:1, max:1 } — pick the source to trash
//   2. OwnField                     — pick the Mineral/Rock Digimon to buff
// After both are resolved, the target gains Reboot, Blocker, and +3000 DP.
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive [On Play]: with a Mineral Digimon that has a Mineral source,
/// playing EX10-028 from hand prompts SourceMulti(1,1) first.
#[test]
fn ex10_028_on_play_prompts_source_multi_with_mineral_source() {
    let mineral_digi = make_mineral_digimon("MINERAL-DIGI");
    let mineral_src = make_mineral_source("MINERAL-SRC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(mineral_src)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "MINERAL-DIGI", None);
    runner.push_source(host, "MINERAL-SRC");

    runner.play(0, 0).expect("play EX10-028 from hand");

    // §15-7-1/§15-7-4: the "By trashing ..." cost is an optional processing
    // condition, so the accept/decline prompt precedes the cost selection.
    // This assertion previously read `pending_kind()` straight after the play
    // and expected SourceMulti — pinning the auto-pay bug, in which the source
    // trash was forced on the controller with no reachable decline.
    accept_optional_cost(&mut runner);

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 1, max: 1, .. })
        ),
        "after accepting, EX10-028 with a Mineral/Rock source must prompt SourceMulti(1,1); got {:?}",
        runner.pending_kind()
    );
}

/// Positive [On Play]: with a Rock Digimon that has a Rock source,
/// playing EX10-028 from hand prompts SourceMulti(1,1) first.
#[test]
fn ex10_028_on_play_prompts_source_multi_with_rock_source() {
    let rock_digi = make_rock_digimon("ROCK-DIGI");
    let rock_src = make_rock_source("ROCK-SRC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(rock_digi)
        .add_card(rock_src)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "ROCK-DIGI", None);
    runner.push_source(host, "ROCK-SRC");

    runner.play(0, 0).expect("play EX10-028 from hand");

    // §15-7-1/§15-7-4: accept/decline precedes the cost selection (see
    // `accept_optional_cost`). This assertion used to expect SourceMulti here.
    accept_optional_cost(&mut runner);

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 1, max: 1, .. })
        ),
        "after accepting, EX10-028 with a Rock source must prompt SourceMulti(1,1); got {:?}",
        runner.pending_kind()
    );
}

/// Positive [On Play]: after manually picking the Mineral source card,
/// the next pending selection must be OwnField for the buff target.
#[test]
fn ex10_028_on_play_after_source_selection_prompts_own_field() {
    let mineral_digi = make_mineral_digimon("M-DIGI-TGT");
    let mineral_src = make_mineral_source("M-SRC-TGT");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(mineral_src)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "M-DIGI-TGT", None);
    runner.push_source(host, "M-SRC-TGT");

    runner.play(0, 0).expect("play EX10-028 from hand");

    // Step 0: accept the optional processing condition (§15-7-1/§15-7-4).
    accept_optional_cost(&mut runner);

    // Step 1: SourceMulti(1,1) — manually pick the source card at slot 0
    // (the pushed Mineral source is the first/only slot in the digivolution stack).
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 1, max: 1, .. })
        ),
        "SourceMulti(1,1) must be pending before source pick; got {:?}",
        runner.pending_kind()
    );
    let source_action =
        encode_source_select(host.index as u16, 0).expect("encode source slot 0 action");
    runner
        .execute_action(0, source_action)
        .expect("pick source at slot 0");

    // Step 2: OwnField — pick Mineral/Rock Digimon to buff
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "after source pick, EX10-028 must prompt OwnField for the buff target; got {:?}",
        runner.pending_kind()
    );
}

/// Positive [On Play]: full resolution — target Digimon gains Reboot.
#[test]
fn ex10_028_on_play_grants_reboot_to_target() {
    let mineral_digi = make_mineral_digimon("M-RBT");
    let mineral_src = make_mineral_source("SRC-RBT");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(mineral_src)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let target = runner.place_on_field(0, "M-RBT", None);
    runner.push_source(target, "SRC-RBT");

    runner.play(0, 0).expect("play EX10-028");
    runner.auto_resolve().expect("source selection");
    runner.auto_resolve().expect("buff target selection");

    assert!(
        runner.game.has_keyword(target, Keyword::Reboot),
        "EX10-028 must grant Reboot to the selected Mineral/Rock Digimon"
    );
}

/// Positive [On Play]: full resolution — target Digimon gains Blocker.
#[test]
fn ex10_028_on_play_grants_blocker_to_target() {
    let mineral_digi = make_mineral_digimon("M-BLK");
    let mineral_src = make_mineral_source("SRC-BLK");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(mineral_src)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let target = runner.place_on_field(0, "M-BLK", None);
    runner.push_source(target, "SRC-BLK");

    runner.play(0, 0).expect("play EX10-028");
    runner.auto_resolve().expect("source selection");
    runner.auto_resolve().expect("buff target selection");

    assert!(
        runner.game.has_keyword(target, Keyword::Blocker),
        "EX10-028 must grant Blocker to the selected Mineral/Rock Digimon"
    );
}

/// Positive [On Play]: full resolution — target Digimon gets +3000 DP.
#[test]
fn ex10_028_on_play_grants_plus_3000_dp_to_target() {
    let mineral_digi = make_mineral_digimon("M-DP3K");
    let mineral_src = make_mineral_source("SRC-DP3K");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(mineral_src)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let target = runner.place_on_field(0, "M-DP3K", None);
    runner.push_source(target, "SRC-DP3K");

    let dp_before = runner.dp_of(target).unwrap_or(0);

    runner.play(0, 0).expect("play EX10-028");
    runner.auto_resolve().expect("source selection");
    runner.auto_resolve().expect("buff target selection");

    let dp_after = runner.dp_of(target).unwrap_or(0);
    assert_eq!(
        dp_after - dp_before,
        3000,
        "EX10-028 must give +3000 DP to the selected Digimon (before: {dp_before}, after: {dp_after})"
    );
}

/// Positive [On Play]: the cost source card is moved to trash after selection.
#[test]
fn ex10_028_on_play_trashes_selected_source() {
    let mineral_digi = make_mineral_digimon("M-TRASH");
    let mineral_src = make_mineral_source("SRC-TRASH");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(mineral_src)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "M-TRASH", None);
    runner.push_source(host, "SRC-TRASH");

    let sources_before = runner.game.player(0).battle_area[host.index as usize]
        .card_sources
        .len();
    assert!(
        sources_before >= 2,
        "host must have at least 1 digivolution source before the effect"
    );
    let trash_before = runner.trash_size(0);

    runner.play(0, 0).expect("play EX10-028");
    runner.auto_resolve().expect("source selection");
    runner.auto_resolve().expect("buff target selection");

    let sources_after = runner.game.player(0).battle_area[host.index as usize]
        .card_sources
        .len();
    let trash_after = runner.trash_size(0);

    assert_eq!(
        sources_after,
        sources_before - 1,
        "one digivolution source must be removed after the cost is paid"
    );
    assert!(
        trash_after > trash_before,
        "the trashed source must land in P0's trash (before: {trash_before}, after: {trash_after})"
    );
}

/// Positive [On Play]: buffing a different Digimon than the source host
/// is allowed — the buff target is independently selected from any Mineral/Rock
/// Digimon on the field.
#[test]
fn ex10_028_on_play_can_buff_different_digimon_than_source_host() {
    let host_digi = make_mineral_digimon("HOST-DIGI");
    let mineral_src = make_mineral_source("HOST-SRC");
    let target_digi = make_rock_digimon("OTHER-TGT");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(host_digi)
        .add_card(mineral_src)
        .add_card(target_digi)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "HOST-DIGI", None);
    runner.push_source(host, "HOST-SRC");
    let other = runner.place_on_field(0, "OTHER-TGT", None);

    runner.play(0, 0).expect("play EX10-028");

    // Step 0: accept the optional processing condition (§15-7-1/§15-7-4).
    accept_optional_cost(&mut runner);

    // Step 1: SourceMulti(1,1) — manually pick the source from HOST-DIGI's stack.
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 1, max: 1, .. })
        ),
        "SourceMulti(1,1) should be pending; got {:?}",
        runner.pending_kind()
    );
    let source_action = encode_source_select(host.index as u16, 0).expect("encode source slot 0");
    runner
        .execute_action(0, source_action)
        .expect("pick source at slot 0");

    // Step 2: OwnField — lets us pick any Mineral/Rock Digimon.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "OwnField should be pending after source selection; got {:?}",
        runner.pending_kind()
    );
    assert!(
        runner.pending_action_count() >= 2,
        "both HOST-DIGI (Mineral) and OTHER-TGT (Rock) should be selectable"
    );

    // Choose OTHER-TGT (the second action).
    let view = runner
        .pending_selection_view()
        .expect("selection view present");
    let player = view.selecting_player;
    let action = view.valid_action_ids[1]; // pick the other target
    runner
        .game
        .resolve_selection(player, action)
        .expect("pick OTHER-TGT");

    // other (OTHER-TGT) must receive the buffs.
    assert!(
        runner.game.has_keyword(other, Keyword::Reboot),
        "the selected Rock Digimon (OTHER-TGT) must gain Reboot"
    );
    assert!(
        runner.game.has_keyword(other, Keyword::Blocker),
        "the selected Rock Digimon (OTHER-TGT) must gain Blocker"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 1 multi-timing: [When Digivolving] path
// ═══════════════════════════════════════════════════════════════════════════════

/// [When Digivolving]: manually firing WhenDigivolving on EX10-028 with a
/// Mineral source available prompts SourceMulti(1,1).
#[test]
fn ex10_028_when_digivolving_prompts_source_multi() {
    let mineral_digi = make_mineral_digimon("WD-DIGI");
    let mineral_src = make_mineral_source("WD-SRC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(mineral_src)
        .memory(10)
        .start();

    let golemon = runner.place_on_field(0, "EX10-028", None);
    let digi = runner.place_on_field(0, "WD-DIGI", None);
    runner.push_source(digi, "WD-SRC");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(golemon),
    );
    runner.game.drain_effect_queue();

    // §15-7-1/§15-7-4: accept/decline precedes the cost selection on the
    // [When Digivolving] timing too — DCGO's own WhenDigivolving ActivateClass
    // (EX10_028.cs:159) likewise passes isOptional=true.
    accept_optional_cost(&mut runner);

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 1, max: 1, .. })
        ),
        "WhenDigivolving on EX10-028 with a Mineral source must prompt SourceMulti(1,1) after accepting; got {:?}",
        runner.pending_kind()
    );
}

/// [When Digivolving]: after full resolution, a Mineral/Rock Digimon gains
/// Reboot + Blocker + +3000 DP.
///
/// EX10-028 (Golemon) itself has the Mineral trait, so it is a valid buff
/// target. We set up golemon as the only Mineral/Rock Digimon on the field
/// with a Mineral source under it. auto_resolve picks golemon as the source
/// (SourceMulti) and as the buff target (OwnField).
#[test]
fn ex10_028_when_digivolving_grants_all_buffs() {
    let mineral_src = make_mineral_source("WD-MSRC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_src)
        .memory(10)
        .start();

    // Golemon (Mineral) on the field with one Mineral source.
    let golemon = runner.place_on_field(0, "EX10-028", None);
    runner.push_source(golemon, "WD-MSRC");

    let dp_before = runner.dp_of(golemon).unwrap_or(0);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(golemon),
    );
    runner.game.drain_effect_queue();

    // auto_resolve drives both selections (SourceMulti then OwnField) in one call
    // since it loops until no pending_selection remains.
    assert!(
        runner.pending_selection().is_some(),
        "a selection must be pending after WhenDigivolving fires"
    );
    runner.auto_resolve().expect("all selections resolve");

    // Golemon (Mineral trait) is the only valid buff target on the field.
    assert!(
        runner.game.has_keyword(golemon, Keyword::Reboot),
        "WhenDigivolving must grant Reboot to the selected Mineral Digimon (golemon)"
    );
    assert!(
        runner.game.has_keyword(golemon, Keyword::Blocker),
        "WhenDigivolving must grant Blocker to the selected Mineral Digimon (golemon)"
    );
    let dp_after = runner.dp_of(golemon).unwrap_or(0);
    assert_eq!(
        dp_after - dp_before,
        3000,
        "WhenDigivolving must give +3000 DP (before: {dp_before}, after: {dp_after})"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Negative: Clause 1 silent-skip conditions
// ═══════════════════════════════════════════════════════════════════════════════

/// Negative: no Mineral/Rock source available → the cost cannot be paid, so
/// no cost selection installs and no buffs are granted.
///
/// The accept/decline prompt itself still installs: §15-7-4 gives the player
/// the choice "regardless of whether or not the content of the conditions can
/// be executed", and the lowering deliberately attaches no candidate guard to
/// a `select_own_sources` first step (see `first_step_candidate_guard`'s NOTE
/// in `lower_triggered.rs` — the designed behavior is to fire and silently
/// skip the trash). Sibling EX10-034 is tested to the same shape.
///
/// This assertion previously read `pending_selection().is_none()` directly
/// after the play — correct only while the clause was mandatory (the auto-pay
/// bug). Corrected: accept, then assert the cost selection never installs.
#[test]
fn ex10_028_on_play_no_selection_when_no_mineral_rock_source() {
    let plain_digi = make_mineral_digimon("PLAIN-HOST");
    // No Mineral/Rock source is pushed — the host has only its plain top card.

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(plain_digi)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    runner.place_on_field(0, "PLAIN-HOST", None);

    runner.play(0, 0).expect("play EX10-028");

    accept_optional_cost(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "no cost selection should install when no Mineral/Rock digivolution source is available"
    );
}

/// Negative: source host has only a plain (non-Mineral/non-Rock) source
/// → the filter excludes it, SourceMulti silently skips.
///
/// As above, the §15-7-4 accept/decline prompt still installs (no candidate
/// guard exists for a `select_own_sources` first step); the assertion is that
/// nothing follows the accept.
#[test]
fn ex10_028_on_play_no_selection_when_source_has_wrong_trait() {
    let mineral_digi = make_mineral_digimon("M-WRONGSRC");
    let plain_src = make_filler("PLAIN-SRC"); // no Mineral or Rock trait

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(plain_src)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "M-WRONGSRC", None);
    runner.push_source(host, "PLAIN-SRC");

    runner.play(0, 0).expect("play EX10-028");

    accept_optional_cost(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "no SourceMulti should install when the only source lacks Mineral/Rock trait"
    );
}

/// Negative: no Mineral/Rock Digimon on field (only non-Mineral/Rock Digimon exist
/// AND a Mineral source exists) → SourceMulti installs (source is available), but
/// after paying the source cost, OwnField silently skips (no buff target).
/// Buffs must not be granted.
#[test]
fn ex10_028_on_play_source_selected_but_no_buff_target_means_no_keywords() {
    let mut plain_host = make_test_card("PLAIN-HOST2", "Plain Digimon Host");
    plain_host.level = Some(4);
    plain_host.dp = Some(5000);
    // no Mineral or Rock trait → cannot be buff target
    let mineral_src = make_mineral_source("MSRC-NOTGT");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(plain_host)
        .add_card(mineral_src)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "PLAIN-HOST2", None);
    runner.push_source(host, "MSRC-NOTGT");

    runner.play(0, 0).expect("play EX10-028");

    // Step 0: accept the optional processing condition (§15-7-1/§15-7-4).
    accept_optional_cost(&mut runner);

    // SourceMulti must appear — the source IS Mineral.
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 1, max: 1, .. })
        ),
        "SourceMulti should install when a Mineral/Rock source exists"
    );
    runner.auto_resolve().expect("source selection resolves");

    // After trashing the source, OwnField silently skips (no Mineral/Rock Digimon).
    assert!(
        runner.pending_selection().is_none(),
        "no OwnField selection should install when no Mineral/Rock Digimon is present"
    );

    // Plain host must NOT have received Reboot or Blocker.
    assert!(
        !runner.game.has_keyword(host, Keyword::Reboot),
        "plain Digimon must NOT gain Reboot (filter excludes non-Mineral/Rock)"
    );
    assert!(
        !runner.game.has_keyword(host, Keyword::Blocker),
        "plain Digimon must NOT gain Blocker (filter excludes non-Mineral/Rock)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Expiry: buffs persist through own turn end, expire after opponent's
// ═══════════════════════════════════════════════════════════════════════════════

/// Buffs (Reboot + Blocker + +3000 DP) must persist at the start of the
/// opponent's turn (i.e., they survive own-turn end).
#[test]
fn ex10_028_buffs_persist_through_own_turn_end() {
    let mineral_digi = make_mineral_digimon("EXP1-DIGI");
    let mineral_src = make_mineral_source("EXP1-SRC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(mineral_src)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let target = runner.place_on_field(0, "EXP1-DIGI", None);
    runner.push_source(target, "EXP1-SRC");

    runner.play(0, 0).expect("play EX10-028");
    runner.auto_resolve().expect("source selection");
    runner.auto_resolve().expect("buff target selection");

    // Confirm buffs are active right after resolution.
    assert!(runner.game.has_keyword(target, Keyword::Reboot));
    assert!(runner.game.has_keyword(target, Keyword::Blocker));

    // End P0's turn → now it is P1's turn.
    runner.end_turn();

    // Buffs must still be active during P1's (opponent's) turn.
    assert!(
        runner.game.has_keyword(target, Keyword::Reboot),
        "Reboot must still be active at start of opponent's turn (expiry is end_of_opponents_turn)"
    );
    assert!(
        runner.game.has_keyword(target, Keyword::Blocker),
        "Blocker must still be active at start of opponent's turn"
    );
}

/// Buffs must expire after the opponent's turn ends (EoOT expiry).
/// P1 filler deck prevents deckout that would abort the turn before expire
/// hooks fire.
#[test]
fn ex10_028_buffs_expire_after_opponents_turn_ends() {
    let mineral_digi = make_mineral_digimon("EXP2-DIGI");
    let mineral_src = make_mineral_source("EXP2-SRC");
    let p1_filler = make_filler("EXP2-PAD");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(mineral_src)
        .add_card(p1_filler)
        .hand(0, &["EX10-028"])
        .deck(1, &["EXP2-PAD", "EXP2-PAD", "EXP2-PAD"])
        .memory(10)
        .start();

    let target = runner.place_on_field(0, "EXP2-DIGI", None);
    runner.push_source(target, "EXP2-SRC");

    let dp_base = runner.dp_of(target).unwrap_or(0);

    runner.play(0, 0).expect("play EX10-028");
    runner.auto_resolve().expect("source selection");
    runner.auto_resolve().expect("buff target selection");

    // End P0's turn → P1's turn starts.
    runner.end_turn();
    // End P1's turn → back to P0; modifiers should be expired.
    runner.end_turn();

    assert!(
        !runner.game.has_keyword(target, Keyword::Reboot),
        "Reboot must expire after opponent's turn ends"
    );
    assert!(
        !runner.game.has_keyword(target, Keyword::Blocker),
        "Blocker must expire after opponent's turn ends"
    );
    let dp_after = runner.dp_of(target).unwrap_or(0);
    assert_eq!(
        dp_after, dp_base,
        "+3000 DP must expire after opponent's turn ends (base: {dp_base}, got: {dp_after})"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Clause 2 inherited: delete ≤4-cost opponent Digimon
// ═══════════════════════════════════════════════════════════════════════════════

/// Inherited: when EX10-028 is trashed as a source from a Mineral host,
/// the player must select from opponent's Digimon cost ≤4 (OppField selection).
#[test]
fn ex10_028_inherited_prompts_opp_field_selection_when_trashed_from_mineral_host() {
    let mut host = make_test_card("INH-HOST-M", "Mineral Host");
    host.traits.push("Mineral".to_string());
    let mut cheap = make_test_card("INH-CHEAP", "Cost 4 Digi");
    cheap.play_cost = 4;
    let mut expensive = make_test_card("INH-EXP", "Cost 5 Digi");
    expensive.play_cost = 5;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(host)
        .add_card(cheap)
        .add_card(expensive)
        .build();

    let host_perm = runner.place_on_field(0, "INH-HOST-M", None);
    let source_slot = runner.push_source(host_perm, "EX10-028");
    runner.place_on_field(1, "INH-CHEAP", None);
    runner.place_on_field(1, "INH-EXP", None);

    let top = runner.top_card(host_perm);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host_perm), 0);
        ctx.trash_card_source(host_perm, source_slot);
    }

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "inherited clause must prompt OppField after EX10-028 is trashed from Mineral host"
    );
}

/// Inherited: only the cost-4-or-less Digimon is selectable (cost-5+ excluded).
#[test]
fn ex10_028_inherited_only_cost_4_or_less_selectable() {
    let mut host = make_test_card("INH-HOST-R", "Rock Host");
    host.traits.push("Rock".to_string());
    let mut cheap = make_test_card("INH-CHEAP2", "Cost 3 Digi");
    cheap.play_cost = 3;
    let mut expensive = make_test_card("INH-EXP2", "Cost 5 Digi");
    expensive.play_cost = 5;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(host)
        .add_card(cheap)
        .add_card(expensive)
        .build();

    let host_perm = runner.place_on_field(0, "INH-HOST-R", None);
    let source_slot = runner.push_source(host_perm, "EX10-028");
    runner.place_on_field(1, "INH-CHEAP2", None);
    runner.place_on_field(1, "INH-EXP2", None);

    let top = runner.top_card(host_perm);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host_perm), 0);
        ctx.trash_card_source(host_perm, source_slot);
    }

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "OppField selection must be pending"
    );
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only the cost-4-or-less Digimon should be selectable (cost-5 excluded)"
    );
}

/// Inherited: after selecting the target, the cost-4-or-less Digimon is deleted
/// and the cost-5+ Digimon survives.
#[test]
fn ex10_028_inherited_deletes_cost_4_or_less_digimon() {
    let mut host = make_test_card("INH-HOST-DEL", "Mineral Host Del");
    host.traits.push("Mineral".to_string());
    let mut cheap = make_test_card("INH-DEL-C4", "Cost 4");
    cheap.play_cost = 4;
    let mut pricey = make_test_card("INH-DEL-C6", "Cost 6");
    pricey.play_cost = 6;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(host)
        .add_card(cheap)
        .add_card(pricey)
        .build();

    let host_perm = runner.place_on_field(0, "INH-HOST-DEL", None);
    let source_slot = runner.push_source(host_perm, "EX10-028");
    runner.place_on_field(1, "INH-DEL-C4", None);
    runner.place_on_field(1, "INH-DEL-C6", None);

    let top = runner.top_card(host_perm);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host_perm), 0);
        ctx.trash_card_source(host_perm, source_slot);
    }

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner.auto_resolve().expect("inherited delete resolves");

    let remaining: Vec<&str> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data))
        .collect();
    assert_eq!(
        remaining,
        vec!["INH-DEL-C6"],
        "the cost-4 Digimon must be deleted; cost-6 must survive"
    );
}

/// Inherited: when opponent has no Digimon with cost ≤4, the condition
/// `any_permanent: { play_cost_lte: 4 }` is false → no selection installs.
#[test]
fn ex10_028_inherited_no_selection_when_no_opponent_digimon_cost_4_or_less() {
    let mut host = make_test_card("INH-HOST-NONE", "Mineral Host None");
    host.traits.push("Mineral".to_string());
    let mut big = make_test_card("INH-BIG", "Cost 5");
    big.play_cost = 5;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(host)
        .add_card(big)
        .build();

    let host_perm = runner.place_on_field(0, "INH-HOST-NONE", None);
    let source_slot = runner.push_source(host_perm, "EX10-028");
    runner.place_on_field(1, "INH-BIG", None);

    let top = runner.top_card(host_perm);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host_perm), 0);
        ctx.trash_card_source(host_perm, source_slot);
    }

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when opponent has no Digimon with cost ≤4"
    );
}

/// Inherited (negative): trashing EX10-028 from a non-Mineral/non-Rock host
/// must NOT trigger the inherited delete clause.
#[test]
fn ex10_028_inherited_does_not_fire_from_non_mineral_rock_host() {
    let plain_host = make_test_card("PLAIN-HOST-INH", "Plain Host");
    // no Mineral or Rock trait
    let mut cheap = make_test_card("CHEAP-INH-NEG", "Cost 3");
    cheap.play_cost = 3;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(plain_host)
        .add_card(cheap)
        .build();

    let host_perm = runner.place_on_field(0, "PLAIN-HOST-INH", None);
    let source_slot = runner.push_source(host_perm, "EX10-028");
    runner.place_on_field(1, "CHEAP-INH-NEG", None);

    let field_before = runner.battle_area_size(1);

    let top = runner.top_card(host_perm);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host_perm), 0);
        ctx.trash_card_source(host_perm, source_slot);
    }

    assert!(
        runner.pending_selection().is_none(),
        "inherited clause must NOT fire when the host lacks Mineral or Rock trait"
    );
    assert_eq!(
        runner.battle_area_size(1),
        field_before,
        "opponent's Digimon must survive when host lacks Mineral/Rock trait"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 7 — YAML compilation smoke test
// ═══════════════════════════════════════════════════════════════════════════════

/// EX10-028 YAML must parse, compile, and appear in the embedded DSL pack.
#[test]
fn ex10_028_yaml_compiles_without_error() {
    let runner = base_runner();
    assert!(
        runner.compiled_card("EX10-028").is_some(),
        "EX10-028 must be present in the embedded DSL pack (YAML must parse + compile)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 8 — Clause 1: the §15-7-4 decline branch
// ═══════════════════════════════════════════════════════════════════════════════

/// §15-7-4: "A player can choose whether or not to execute the content of
/// optional processing conditions, regardless of whether or not the content of
/// the conditions can be executed." Declining "By trashing any 1 card with the
/// [Mineral] or [Rock] trait from your Digimon's digivolution cards" must leave
/// BOTH halves undone — the source stays on the stack AND no Digimon gains
/// ＜Reboot＞ / ＜Blocker＞ / +3000 DP — because §15-7-2 says that if the
/// optional condition's content isn't executed, "the processing after the
/// conditions can't be executed".
///
/// DCGO agrees: EX10_028.cs:20 passes `isOptional: true` to
/// `SetUpActivateClass`, and its inner `SelectTrashDigivolutionCards` uses
/// `canNoTrash: false` — i.e. the outer Yes/No is the ONLY place the player
/// can say no, exactly what `outer_prompt: true` models here.
///
/// This is the branch the engine had no way to reach: clause 1 fired
/// unconditionally, so the source trash was auto-paid.
#[test]
fn ex10_028_on_play_optional_cost_may_be_declined() {
    let mineral_digi = make_mineral_digimon("DECLINE-DIGI");
    let mineral_src = make_mineral_source("DECLINE-SRC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-028")
        .expect("EX10-028 YAML parses and compiles")
        .add_card(mineral_digi)
        .add_card(mineral_src)
        .hand(0, &["EX10-028"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "DECLINE-DIGI", None);
    runner.push_source(host, "DECLINE-SRC");

    let sources_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    let trash_before = runner.trash_size(0);
    let dp_before = runner.dp_of(host).expect("host DP readable");

    runner.play(0, 0).expect("play EX10-028 from hand");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "the optional processing condition must surface an accept/decline prompt (rule 17); got {:?}",
        runner.pending_kind()
    );
    runner
        .decline_optional_trigger()
        .expect("declining must be reachable from the action space");
    let _ = runner.auto_resolve();

    // §15-7-2, half 1: the cost was NOT paid.
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        sources_before,
        "declining must not trash the Mineral digivolution card"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "declining must not move any source card to the trash"
    );

    // §15-7-2, half 2: the processing after the condition did NOT happen.
    assert!(
        !runner.game.has_keyword(host, Keyword::Reboot),
        "declining must not grant Reboot"
    );
    assert!(
        !runner.game.has_keyword(host, Keyword::Blocker),
        "declining must not grant Blocker"
    );
    assert_eq!(
        runner.dp_of(host).expect("host DP readable"),
        dp_before,
        "declining must not grant +3000 DP"
    );
}
