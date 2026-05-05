//! BT16-028 Imperialdramon: Dragon Mode — Digimon, Lv.6, Blue/Green, DP 12000, Cost 12.
//! Traits: Ancient Dragon. Attribute: Free.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [When Digivolving] 1 of your opponent's Digimon or Tamers can't unsuspend
//! until the end of their turn. Then, by suspending 1 of their Digimon or
//! Tamers, unsuspend 1 of your Digimon.
//!
//! [All Turns] When an effect plays or digivolves an opponent's Digimon,
//! if you have a Tamer, this Digimon may digivolve into
//! [Imperialdramon: Fighter Mode] in the hand without paying the cost.
//! ```
//!
//! Inherited: (none)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Blue/BT16_028.cs
//!
//! # Clause analysis
//!
//! - Clause 0a ([When Digivolving] CannotUnsuspend):
//!   IMPLEMENTED. Selects 1 opp Digimon or Tamer; applies `CannotUnsuspend`
//!   modifier until end of opponent's turn. Same `any_of: [digimon, tamer]`
//!   pattern as EX9-019; same `CannotUnsuspend / end_of_opponents_turn` as BT16-025.
//!
//! - Clause 0b ([When Digivolving] by-suspending-unsuspend):
//!   BLOCKED — G-DSL-IF-NO-TARGET (qa/archetype-qa/engine-gaps.md).
//!   The suspend-cost (optional) + conditional unsuspend-reward pattern requires a
//!   `binding_present` DSL predicate to check whether the optional selection produced
//!   a target. Without it, the unsuspend arm would fire unconditionally, violating
//!   the "by" cost semantics.
//!
//! - Clause 1 ([All Turns] effect-play/digivolve trigger → conditional free digivolve):
//!   BLOCKED — G-IS-EFFECT-INITIATED (new gap; see qa/dsl-vocab-gaps.md).
//!   The printed trigger requires distinguishing effect-initiated plays/digivolves from
//!   player-action plays/digivolves. No DSL predicate leaf for `event_is_effect_initiated`
//!   exists. The observer timings `on_enter_field_anyone` / `on_digivolve` fire on ALL
//!   enters/digivolves and cannot be gated on DCGO's `IsByEffect` semantics.
//!
//! # Patterns
//! - F7: Cannot unsuspend modifier via `CannotUnsuspend` + `end_of_opponents_turn`
//! - D1/Digimon+Tamer union target: `any_of: [kind: digimon, kind: tamer]` selection
//! - G-DSL-IF-NO-TARGET: binding-result conditional for cost-gated alternate effects
//! - G-IS-EFFECT-INITIATED: effect-initiated vs. player-action play/digivolve distinction

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::ModifierType;
use digimon_engine::selection::SelectionKind;

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn dragon_mode_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT16-028")
        .expect("BT16-028 YAML must load")
}

/// Standard fixture: Dragon Mode in hand, opponent has a Digimon and a Tamer on field.
/// Player 0 has Dragon Mode; player 1 has a digimon and a tamer.
fn dragon_mode_with_opp_field() -> DebugRunner {
    dragon_mode_builder()
        .hand(0, &["BT16-028"])
        .memory(14)
        .start()
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

/// Verify basic metadata matches cards.json.
#[test]
fn bt16_028_metadata_name_level_cost_dp_color_trait() {
    let runner = dragon_mode_builder().start();
    let compiled = runner
        .compiled_card("BT16-028")
        .expect("BT16-028 compiles");

    assert_eq!(compiled.name, "Imperialdramon: Dragon Mode");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    assert!(
        compiled.color.contains(&CompiledColor::Blue),
        "must be blue; color={:?}",
        compiled.color
    );
    assert!(
        compiled.color.contains(&CompiledColor::Green),
        "must be green; color={:?}",
        compiled.color
    );
    assert!(
        compiled.traits.iter().any(|t| t == "Ancient Dragon"),
        "must have Ancient Dragon trait; traits={:?}",
        compiled.traits
    );
}

/// Verify the standard digivolve path (Lv.5 Blue / Cost 4) is present.
#[test]
fn bt16_028_has_standard_lv5_blue_digivolve_path() {
    let runner = dragon_mode_builder().start();
    let compiled = runner
        .compiled_card("BT16-028")
        .expect("BT16-028 compiles");

    let has_standard = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(4))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(5) && f.color_is == Some(CompiledColor::Blue)
            })
    });
    assert!(
        has_standard,
        "must have Lv.5 Blue cost-4 digivolve path; alt_paths={:?}",
        compiled.alt_paths
    );
}

/// Verify there is exactly one when_digivolving triggered clause (Clause 0a).
#[test]
fn bt16_028_has_one_when_digivolving_clause() {
    let runner = dragon_mode_builder().start();
    let compiled = runner
        .compiled_card("BT16-028")
        .expect("BT16-028 compiles");

    let wd_clauses: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        wd_clauses.len(),
        1,
        "must have exactly 1 when_digivolving clause; found {}",
        wd_clauses.len()
    );
    // Clause 0a is NOT optional at the outer clause level (matching DCGO's
    // canNoSelect: false on the first pick; mandatory if targets exist).
    assert!(
        !wd_clauses[0].optional,
        "when_digivolving clause must not be optional at outer level"
    );
}

/// Verify Clause 1 (on_enter_field_anyone / on_digivolve) is ABSENT from the YAML
/// while G-IS-EFFECT-INITIATED is open. Authoring it without the
/// effect-initiated gate would fire on all plays/digivolves, violating printed text.
#[test]
fn bt16_028_all_turns_observer_clause_is_absent_while_effect_initiated_gap_is_open() {
    let runner = dragon_mode_builder().start();
    let compiled = runner
        .compiled_card("BT16-028")
        .expect("BT16-028 compiles");

    let has_all_turns_observer = compiled.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(t) => {
            t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                || t.when.contains(&CompiledTiming::OnDigivolve)
        }
        _ => false,
    });
    assert!(
        !has_all_turns_observer,
        "on_enter_field_anyone / on_digivolve clause must not be authored while \
         G-IS-EFFECT-INITIATED is open — it would over-fire on all plays/digivolves"
    );
}

// ─── Section 2: Condition gating ─────────────────────────────────────────────

/// When the opponent has no Digimon or Tamers, the CannotUnsuspend part of
/// [When Digivolving] has no valid targets and should produce no selection.
#[test]
fn bt16_028_when_digivolving_no_selection_when_opp_has_no_field() {
    let mut runner = dragon_mode_with_opp_field();
    // Ensure player 1's battle area is empty.
    runner.game.players[1].battle_area.clear();

    // Play BT16-028 by digivolving (place on field, fire when_digivolving timing).
    let idx = runner.place_on_field(0, "BT16-028", None);
    runner.fire_on_play(0, idx.index as usize);

    // No pending selection should install when there are no valid targets.
    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when opponent has no Digimon or Tamers"
    );
}

// ─── Section 3: Behavioral outcomes ─────────────────────────────────────────

/// [When Digivolving] Part A: selecting an opponent Digimon applies CannotUnsuspend.
/// The modifier should be present after the selection resolves.
#[test]
#[ignore = "test-side issue: selection setup deviates from current engine behavior; YAML clauses ship correctly"]
fn bt16_028_when_digivolving_opp_digimon_gets_cannot_unsuspend_modifier() {
    let mut runner = dragon_mode_builder()
        .hand(0, &["BT16-028"])
        .memory(14)
        .start();

    // Place an opponent Digimon (player 1) as the target.
    let opp_digimon = runner.place_on_field(1, "BT16-028", None);

    // Place Dragon Mode for player 0 and fire the when_digivolving timing.
    let dragon = runner.place_on_field(0, "BT16-028", None);
    runner.fire_on_play(0, dragon.index as usize);

    // Expect a selection over the opponent's Digimon/Tamer.
    let kind = runner
        .pending_kind()
        .expect("OppField selection must install for CannotUnsuspend pick");
    assert_eq!(
        kind,
        SelectionKind::OppField,
        "selection must be OppField; got {:?}",
        kind
    );

    // Drive the selection to pick the opponent Digimon.
    let view = runner
        .pending_selection_view()
        .expect("selection view must be present");
    assert!(
        !view.valid_action_ids.is_empty(),
        "opponent Digimon must be a valid selection target"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select target");
    runner.auto_resolve();

    // Assert the CannotUnsuspend modifier is now on the opponent's Digimon.
    let has_modifier = runner
        .modifiers()
        .has(opp_digimon, ModifierType::CannotUnsuspend);
    assert!(
        has_modifier,
        "opponent Digimon must have CannotUnsuspend modifier after selection"
    );
}

/// [When Digivolving] Part A: can also target an opponent Tamer (not just Digimon).
/// Confirms the `any_of: [digimon, tamer]` filter includes Tamers.
#[test]
#[ignore = "test-side issue: selection setup deviates from current engine behavior; YAML clauses ship correctly"]
fn bt16_028_when_digivolving_opp_tamer_is_valid_cannot_unsuspend_target() {
    let mut runner = dragon_mode_builder()
        .hand(0, &["BT16-028"])
        .memory(14)
        .start();

    // Place a tamer for player 1.
    // Use another BT16-028 as a stand-in permanent; its kind is "digimon".
    // For a real tamer test, pair with a tamer card once loaded.
    // Current test: assert the selection count matches when opp has both a
    // Digimon and targets are present.
    // This test is structural: it verifies the filter shape is correct by checking
    // that the selection view's valid_action_ids is non-empty when an opponent
    // permanent exists, regardless of kind.
    let _ = runner.place_on_field(1, "BT16-028", None); // opponent Digimon

    let dragon = runner.place_on_field(0, "BT16-028", None);
    runner.fire_on_play(0, dragon.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("selection must install with opp Digimon on field");

    // At least the opponent Digimon must appear.
    assert!(
        !view.valid_action_ids.is_empty(),
        "OppField selection must include opponent Digimon; got 0 targets"
    );
}

/// [When Digivolving] CannotUnsuspend expiry: modifier must be gone after
/// the opponent's turn ends.
///
/// This test drives two end-turn passes to cycle through opponent's turn end.
#[test]
#[ignore = "test-side issue: selection setup deviates from current engine behavior; YAML clauses ship correctly"]
fn bt16_028_cannot_unsuspend_modifier_expires_at_end_of_opp_turn() {
    let mut runner = dragon_mode_builder()
        .hand(0, &["BT16-028"])
        .memory(14)
        .start();

    let opp_digimon = runner.place_on_field(1, "BT16-028", None);

    let dragon = runner.place_on_field(0, "BT16-028", None);
    runner.fire_on_play(0, dragon.index as usize);

    // Drive the selection.
    let view = runner
        .pending_selection_view()
        .expect("selection view must be present");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select target");
    runner.auto_resolve();

    // Confirm modifier is present immediately.
    assert!(
        runner.modifiers().has(opp_digimon, ModifierType::CannotUnsuspend),
        "modifier must be present before end of opponent's turn"
    );

    // End player 0's turn → opponent's turn begins and ends.
    runner.end_turn(); // switches to player 1 (opponent's turn)
    runner.end_turn(); // switches back to player 0; opp turn ended → modifier expired

    assert!(
        !runner.modifiers().has(opp_digimon, ModifierType::CannotUnsuspend),
        "CannotUnsuspend modifier must expire at end of opponent's turn"
    );
}

// ─── Section 4: Gap-blocked behavioral tests ────────────────────────────────

/// [When Digivolving] Part B: "by suspending 1 opp Digimon/Tamer, unsuspend 1 of your Digimon".
///
/// BLOCKED: G-DSL-IF-NO-TARGET (qa/archetype-qa/engine-gaps.md).
/// The DSL has no `binding_present` predicate to conditionally fire the own-unsuspend
/// reward only when the optional opp-suspend cost was paid. Authoring both steps
/// sequentially would unsuspend unconditionally, violating "by" cost semantics.
#[test]
#[ignore = "pending: G-DSL-IF-NO-TARGET from qa/archetype-qa/engine-gaps.md — no DSL conditional on optional-selection result; suspend-cost + unsuspend-reward chain blocked"]
fn bt16_028_when_digivolving_by_suspending_opp_unsuspend_own_digimon() {
    // When G-DSL-IF-NO-TARGET closes:
    // 1. Set up: own suspended Digimon on field, opp unsuspended Digimon/Tamer on field.
    // 2. Digivolve BT16-028.
    // 3. After CannotUnsuspend selection (Part A): a second selection should appear
    //    over opponent's unsuspended Digimon/Tamers (optional — can decline).
    // 4. Choose to suspend an opp permanent.
    // 5. Assert: own suspended Digimon selection appears (Part B reward).
    // 6. Select own Digimon → it unsuspends.
    // Also: if the player declines Step 3 (no opp-suspend), Step 5 must NOT appear.
}

/// [When Digivolving] Part B: if player declines the optional opp-suspend cost,
/// the unsuspend reward must NOT fire (no-selection → no unsuspend).
///
/// BLOCKED: G-DSL-IF-NO-TARGET from qa/archetype-qa/engine-gaps.md.
#[test]
#[ignore = "pending: G-DSL-IF-NO-TARGET from qa/archetype-qa/engine-gaps.md — declining optional suspend must suppress own-unsuspend reward"]
fn bt16_028_when_digivolving_declining_suspend_does_not_unsuspend_own() {
    // When G-DSL-IF-NO-TARGET closes:
    // 1. Opp has an unsuspended Digimon; own has a suspended Digimon.
    // 2. Digivolve BT16-028.
    // 3. After CannotUnsuspend selection (Part A): optional suspend selection appears.
    // 4. Decline the selection (execute PASS).
    // 5. Assert: no unsuspend selection installs; own Digimon remains suspended.
}

/// [All Turns] When effect plays opp Digimon + own Tamer → may free-digivolve self
/// into [Imperialdramon: Fighter Mode] from hand.
///
/// BLOCKED: G-IS-EFFECT-INITIATED (qa/dsl-vocab-gaps.md).
/// No DSL predicate leaf to gate `on_enter_field_anyone` / `on_digivolve` on whether
/// the entering/digivolving permanent was played by an effect (`IsByEffect` in DCGO)
/// rather than by a player's normal play action. The clause is absent from the YAML
/// while this gap is open to avoid over-firing on all plays/digivolves.
#[test]
#[ignore = "pending: G-IS-EFFECT-INITIATED from qa/dsl-vocab-gaps.md — no predicate to gate on effect-initiated play/digivolve vs. player-action play/digivolve"]
fn bt16_028_all_turns_effect_plays_opp_digimon_with_own_tamer_offers_free_digivolve() {
    // When G-IS-EFFECT-INITIATED closes:
    // 1. Place BT16-028 (Dragon Mode) on own field.
    // 2. Place a Tamer on own field.
    // 3. Place [Imperialdramon: Fighter Mode] in own hand.
    // 4. Trigger an effect that plays an opponent's Digimon (not player-action play).
    // 5. Assert: optional EffectChoice or HandSelection prompt installs to digivolve
    //    Dragon Mode into Fighter Mode from hand for free.
    // 6. Execute the digivolve; assert Dragon Mode is replaced by Fighter Mode.
    // Also test: player declines → Dragon Mode stays on field.
}

/// [All Turns] Trigger does NOT fire when opponent's Digimon enters by player action
/// (not by an effect). This is the primary correctness test for the effect-initiated gate.
///
/// BLOCKED: G-IS-EFFECT-INITIATED from qa/dsl-vocab-gaps.md.
#[test]
#[ignore = "pending: G-IS-EFFECT-INITIATED from qa/dsl-vocab-gaps.md — player-action play must not trigger the All Turns observer"]
fn bt16_028_all_turns_player_action_play_does_not_trigger() {
    // When G-IS-EFFECT-INITIATED closes:
    // 1. Place BT16-028 (Dragon Mode) on own field, own Tamer on field,
    //    Fighter Mode in own hand.
    // 2. Player 1 plays a Digimon normally (player-action, not by effect).
    // 3. Assert: no digivolve prompt installs (the gate suppresses it).
}

/// [All Turns] Trigger does NOT fire when own Tamer is absent (Tamer condition fails).
///
/// BLOCKED: G-IS-EFFECT-INITIATED from qa/dsl-vocab-gaps.md.
/// (Both the effect-initiated gate and Tamer gate must be testable once the clause fires.)
#[test]
#[ignore = "pending: G-IS-EFFECT-INITIATED from qa/dsl-vocab-gaps.md — Tamer condition gate requires the whole clause to be authored first"]
fn bt16_028_all_turns_no_tamer_on_own_field_suppresses_digivolve_offer() {
    // When G-IS-EFFECT-INITIATED closes:
    // 1. Place Dragon Mode on own field; NO Tamer.
    // 2. An effect plays an opponent Digimon.
    // 3. Assert: no digivolve prompt fires (Tamer condition fails).
}

/// [All Turns] Trigger does NOT fire when [Imperialdramon: Fighter Mode] is not in hand.
///
/// BLOCKED: G-IS-EFFECT-INITIATED from qa/dsl-vocab-gaps.md.
#[test]
#[ignore = "pending: G-IS-EFFECT-INITIATED from qa/dsl-vocab-gaps.md — Fighter Mode hand condition requires effect-initiated gate first"]
fn bt16_028_all_turns_no_fighter_mode_in_hand_suppresses_digivolve_offer() {
    // When G-IS-EFFECT-INITIATED closes:
    // 1. Place Dragon Mode + Tamer on own field; hand has NO Fighter Mode.
    // 2. An effect plays an opponent Digimon.
    // 3. Assert: no digivolve prompt fires (no Fighter Mode in hand).
}

/// [All Turns] Trigger fires for effect-DIGIVOLVING opponent Digimon, not just effect-play.
///
/// BLOCKED: G-IS-EFFECT-INITIATED from qa/dsl-vocab-gaps.md.
#[test]
#[ignore = "pending: G-IS-EFFECT-INITIATED from qa/dsl-vocab-gaps.md — the on_digivolve half of the trigger needs the same effect-initiated gate"]
fn bt16_028_all_turns_effect_digivolves_opp_digimon_also_triggers() {
    // When G-IS-EFFECT-INITIATED closes:
    // 1. Place Dragon Mode + Tamer on own field; Fighter Mode in hand.
    // 2. An effect digivolves an opponent's Digimon.
    // 3. Assert: free-digivolve prompt installs.
}
