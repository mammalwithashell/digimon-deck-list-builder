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
//!   IMPLEMENTED. The suspend-cost selection is optional and filtered to opponent
//!   unsuspended Digimon/Tamers, then `binding_present` gates the own-unsuspend reward.
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
//! - binding-result conditional for cost-gated alternate effects
//! - G-IS-EFFECT-INITIATED: effect-initiated vs. player-action play/digivolve distinction

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, CostDelta, EffectTiming, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn dragon_mode_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT16-028")
        .expect("BT16-028 YAML must load")
}

fn dragon_mode_and_fighter_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    dragon_mode_builder()
        .dsl_card("BT16-027")
        .expect("BT16-027 YAML must load")
}

fn tamer_card(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

/// Standard fixture: Dragon Mode in hand, opponent has a Digimon and a Tamer on field.
/// Player 0 has Dragon Mode; player 1 has a digimon and a tamer.
fn dragon_mode_with_opp_field() -> DebugRunner {
    dragon_mode_builder()
        .hand(0, &["BT16-028"])
        .memory(14)
        .start()
}

fn fire_when_digivolving(runner: &mut DebugRunner, dragon: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(dragon),
    );
    runner.game.drain_effect_queue();
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

/// Verify basic metadata matches cards.json.
#[test]
fn bt16_028_metadata_name_level_cost_dp_color_trait() {
    let runner = dragon_mode_builder().start();
    let compiled = runner.compiled_card("BT16-028").expect("BT16-028 compiles");

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
    let compiled = runner.compiled_card("BT16-028").expect("BT16-028 compiles");

    let has_standard = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(4))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(5) && f.color_is == Some(CompiledColor::Blue))
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
    let compiled = runner.compiled_card("BT16-028").expect("BT16-028 compiles");

    let wd_clauses: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
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

/// Verify Clause 1 (on_enter_field_anyone / on_digivolve) carries the
/// effect-initiated event gate required by the printed "when an effect plays
/// or digivolves" condition.
#[test]
fn bt16_028_all_turns_observer_clause_has_effect_initiated_gate() {
    let runner = dragon_mode_builder().start();
    let compiled = runner.compiled_card("BT16-028").expect("BT16-028 compiles");

    let observer = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    && t.when.contains(&CompiledTiming::OnDigivolve) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("all-turns effect-play/digivolve observer must be authored");
    let condition = observer.condition.as_ref().expect("observer must be gated");
    assert!(
        condition
            .all_of
            .iter()
            .any(|p| p.event_is_effect_initiated == Some(true)),
        "observer condition must include event_is_effect_initiated: true; condition={condition:?}"
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
fn bt16_028_when_digivolving_opp_digimon_gets_cannot_unsuspend_modifier() {
    let mut runner = dragon_mode_builder().memory(14).start();

    // Place Dragon Mode for player 0; place an opponent Digimon (player 1).
    let dragon = runner.place_on_field(0, "BT16-028", None);
    let opp_digimon = runner.place_on_field(1, "BT16-028", None);

    // Fire the [When Digivolving] timing.
    fire_when_digivolving(&mut runner, dragon);

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
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select target");

    // Decline the optional by-suspending cost so the effect can finish.
    if let Some(suspend_cost) = runner.pending_selection_view() {
        runner
            .execute_action(suspend_cost.selecting_player, PASS)
            .expect("decline suspend cost");
    }
    runner.auto_resolve().expect("finish effect");

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
fn bt16_028_when_digivolving_opp_tamer_is_valid_cannot_unsuspend_target() {
    let mut runner = dragon_mode_builder()
        .add_card(tamer_card("OPP-TAMER", "Opponent Tamer"))
        .memory(14)
        .start();

    // Opponent has ONLY a Tamer on the field — no Digimon.
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", None);

    let dragon = runner.place_on_field(0, "BT16-028", None);
    fire_when_digivolving(&mut runner, dragon);

    let view = runner
        .pending_selection_view()
        .expect("selection must install with opp Tamer on field");
    assert_eq!(view.kind, SelectionKind::OppField);
    // The opponent Tamer must be the (only) valid CannotUnsuspend target.
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "OppField selection must include the opponent Tamer; got {} targets",
        view.valid_action_ids.len()
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select opponent Tamer");

    // Tamer is unsuspended, so it is also offered as the optional suspend cost;
    // decline it so the effect finishes.
    if let Some(suspend_cost) = runner.pending_selection_view() {
        runner
            .execute_action(suspend_cost.selecting_player, PASS)
            .expect("decline suspend cost");
    }
    runner.auto_resolve().expect("finish effect");

    assert!(
        runner
            .modifiers()
            .has(opp_tamer, ModifierType::CannotUnsuspend),
        "opponent Tamer must receive the CannotUnsuspend modifier"
    );
}

/// [When Digivolving] CannotUnsuspend expiry: modifier must be gone after
/// the opponent's turn ends.
///
/// This test drives two end-turn passes to cycle through opponent's turn end.
#[test]
fn bt16_028_cannot_unsuspend_modifier_expires_at_end_of_opp_turn() {
    // Both players need decks so `pass_turn` rotates cleanly (no deck-out park).
    let filler: Vec<&str> = vec!["FILL"; 12];
    let mut runner = dragon_mode_builder()
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(14)
        .start();

    let dragon = runner.place_on_field(0, "BT16-028", None);
    let opp_digimon = runner.place_on_field(1, "BT16-028", None);

    fire_when_digivolving(&mut runner, dragon);

    // Drive the selection.
    let view = runner
        .pending_selection_view()
        .expect("selection view must be present");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select target");

    // Decline the optional by-suspending cost so the effect finishes.
    if let Some(suspend_cost) = runner.pending_selection_view() {
        runner
            .execute_action(suspend_cost.selecting_player, PASS)
            .expect("decline suspend cost");
    }
    runner.auto_resolve().expect("finish effect");

    // Confirm modifier is present immediately.
    assert!(
        runner
            .modifiers()
            .has(opp_digimon, ModifierType::CannotUnsuspend),
        "modifier must be present before end of opponent's turn"
    );

    // The modifier was installed by player 0 with `end_of_opponents_turn`
    // expiry. Rotate turns until player 1's (the opponent's) turn ends.
    let installer = 0u8;
    for _ in 0..4 {
        let ending = runner.turn_player();
        runner.pass_turn();
        if ending != installer {
            // The opponent's turn just ended — the modifier must be gone.
            break;
        }
    }

    assert!(
        !runner
            .modifiers()
            .has(opp_digimon, ModifierType::CannotUnsuspend),
        "CannotUnsuspend modifier must expire at end of opponent's turn"
    );
}

// ─── Section 4: By-suspending branch behavioral tests ───────────────────────

/// [When Digivolving] Part B: "by suspending 1 opp Digimon/Tamer, unsuspend 1 of your Digimon".
///
/// The own-unsuspend reward is gated behind the optional opponent-suspend cost.
#[test]
fn bt16_028_when_digivolving_by_suspending_opp_unsuspend_own_digimon() {
    let mut runner = dragon_mode_builder().memory(14).start();
    let dragon = runner.place_on_field(0, "BT16-028", None);
    let own_suspended = runner.place_on_field(0, "BT16-028", None);
    let opp_unsuspended = runner.place_on_field(1, "BT16-028", None);
    runner.game.players[0].battle_area[own_suspended.index as usize].is_suspended = true;

    fire_when_digivolving(&mut runner, dragon);

    let lock = runner
        .pending_selection_view()
        .expect("first selection locks one opponent Digimon/Tamer");
    assert_eq!(lock.kind, SelectionKind::OppField);
    assert_eq!(lock.valid_action_ids.len(), 1);
    assert!(
        !runner.pending_is_optional(),
        "CannotUnsuspend target selection is mandatory when a target exists"
    );
    runner
        .execute_action(lock.selecting_player, lock.valid_action_ids[0])
        .expect("select CannotUnsuspend target");

    let suspend_cost = runner
        .pending_selection_view()
        .expect("second selection offers optional opponent suspend cost");
    assert_eq!(suspend_cost.kind, SelectionKind::OppField);
    assert_eq!(
        suspend_cost.valid_action_ids.len(),
        1,
        "only unsuspended opponent Digimon/Tamers are legal suspend-cost targets"
    );
    assert!(
        runner.pending_is_optional(),
        "by-suspending cost selection must be optional so the player can decline"
    );
    runner
        .execute_action(suspend_cost.selecting_player, suspend_cost.valid_action_ids[0])
        .expect("pay suspend cost");

    assert!(
        runner.game.players[1].battle_area[opp_unsuspended.index as usize].is_suspended,
        "selected opponent permanent must be suspended as the cost"
    );

    let unsuspend_reward = runner
        .pending_selection_view()
        .expect("paying the suspend cost should offer own suspended Digimon to unsuspend");
    assert_eq!(unsuspend_reward.kind, SelectionKind::OwnField);
    assert_eq!(
        unsuspend_reward.valid_action_ids.len(),
        1,
        "only own suspended Digimon should be legal for the reward"
    );
    assert!(
        !runner.pending_is_optional(),
        "reward selection is mandatory after the suspend cost is paid"
    );
    runner
        .execute_action(
            unsuspend_reward.selecting_player,
            unsuspend_reward.valid_action_ids[0],
        )
        .expect("select own suspended Digimon");
    runner.auto_resolve().expect("finish effect");

    assert!(
        !runner.game.players[0].battle_area[own_suspended.index as usize].is_suspended,
        "selected own Digimon must be unsuspended after paying the cost"
    );
}

/// [When Digivolving] Part B: if player declines the optional opp-suspend cost,
/// the unsuspend reward must NOT fire (no-selection → no unsuspend).
///
/// PASSing the optional opponent-suspend cost suppresses the reward branch.
#[test]
fn bt16_028_when_digivolving_declining_suspend_does_not_unsuspend_own() {
    let mut runner = dragon_mode_builder().memory(14).start();
    let dragon = runner.place_on_field(0, "BT16-028", None);
    let own_suspended = runner.place_on_field(0, "BT16-028", None);
    let opp_unsuspended = runner.place_on_field(1, "BT16-028", None);
    runner.game.players[0].battle_area[own_suspended.index as usize].is_suspended = true;

    fire_when_digivolving(&mut runner, dragon);

    let lock = runner
        .pending_selection_view()
        .expect("first selection locks one opponent Digimon/Tamer");
    runner
        .execute_action(lock.selecting_player, lock.valid_action_ids[0])
        .expect("select CannotUnsuspend target");

    let suspend_cost = runner
        .pending_selection_view()
        .expect("second selection offers optional opponent suspend cost");
    assert_eq!(suspend_cost.kind, SelectionKind::OppField);
    assert_eq!(suspend_cost.valid_action_ids.len(), 1);
    assert!(
        runner.pending_is_optional(),
        "by-suspending cost selection must expose a PASS decline"
    );
    runner
        .execute_action(suspend_cost.selecting_player, PASS)
        .expect("decline suspend cost");
    runner.auto_resolve().expect("finish declined branch");

    assert!(
        !runner.game.players[1].battle_area[opp_unsuspended.index as usize].is_suspended,
        "declining the branch must not suspend the opponent permanent"
    );
    assert!(
        runner.game.players[0].battle_area[own_suspended.index as usize].is_suspended,
        "declining the suspend cost must not install or resolve the unsuspend reward"
    );
    assert!(
        runner.pending_selection().is_none(),
        "declining the optional suspend cost should leave no own-unsuspend selection"
    );
}

/// [All Turns] When effect plays opp Digimon + own Tamer → may free-digivolve self
/// into [Imperialdramon: Fighter Mode] from hand.
///
#[test]
fn bt16_028_all_turns_effect_plays_opp_digimon_with_own_tamer_offers_free_digivolve() {
    let mut runner = dragon_mode_and_fighter_builder()
        .add_card(tamer_card("OWN-TAMER", "Own Tamer"))
        .add_card(make_test_card("OPP-EFFECT-PLAY", "Opponent Effect Play"))
        .hand(0, &["BT16-027"])
        .hand(1, &["OPP-EFFECT-PLAY"])
        .memory(0)
        .start();

    let dragon = runner.place_on_field(0, "BT16-028", None);
    runner.place_on_field(0, "OWN-TAMER", None);
    let played_by_effect = runner.game.players[1].hand[0].handle();

    assert_eq!(
        runner
            .game
            .play_card_from_effect_without_cost(1, played_by_effect),
        Some(digimon_engine::permanent::PermanentHandle {
            player: 1,
            index: 0,
        }),
        "opponent effect play precondition"
    );

    assert!(
        runner.pending_selection().is_some(),
        "effect-initiated opponent play should expose a follow-up choice"
    );
    for _ in 0..3 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        let action = view.valid_action_ids[0];
        runner
            .execute_action(view.selecting_player, action)
            .expect("execute pending Dragon Mode follow-up");
    }
    runner.auto_resolve().expect("finish free digivolve");

    let top_name = runner.game.players[0].battle_area[dragon.index as usize]
        .top_card()
        .card_name(&runner.game.card_data)
        .to_string();
    assert_eq!(
        top_name, "Imperialdramon: Fighter Mode",
        "Dragon Mode should free-digivolve into Fighter Mode after opponent effect play"
    );
}

/// [All Turns] Trigger does NOT fire when opponent's Digimon enters by player action
/// (not by an effect). This is the primary correctness test for the effect-initiated gate.
///
#[test]
fn bt16_028_all_turns_player_action_play_does_not_trigger() {
    let mut runner = dragon_mode_and_fighter_builder()
        .add_card(tamer_card("OWN-TAMER", "Own Tamer"))
        .add_card(make_test_card("OPP-NORMAL-PLAY", "Opponent Normal Play"))
        .hand(0, &["BT16-027"])
        .hand(1, &["OPP-NORMAL-PLAY"])
        .memory(0)
        .start();

    runner.place_on_field(0, "BT16-028", None);
    runner.place_on_field(0, "OWN-TAMER", None);

    assert_eq!(
        runner
            .game
            .play_from_hand_with_cost(1, 0, CostDelta::Free, PlaySource::ByHand),
        Some(0),
        "opponent normal play precondition"
    );
    assert!(
        runner.pending_selection().is_none(),
        "normal player-action play must not satisfy event_is_effect_initiated"
    );
}

/// [All Turns] Trigger does NOT fire when own Tamer is absent (Tamer condition fails).
///
/// The observer condition includes `any_permanent { of: you, kind: tamer }`; with
/// no Tamer on the player's field the clause must not install a selection even
/// though the effect-initiated opponent play occurred.
#[test]
fn bt16_028_all_turns_no_tamer_on_own_field_suppresses_digivolve_offer() {
    let mut runner = dragon_mode_and_fighter_builder()
        .add_card(make_test_card("OPP-EFFECT-PLAY", "Opponent Effect Play"))
        .hand(0, &["BT16-027"])
        .hand(1, &["OPP-EFFECT-PLAY"])
        .memory(0)
        .start();

    // Dragon Mode on own field, but NO Tamer.
    runner.place_on_field(0, "BT16-028", None);
    let played_by_effect = runner.game.players[1].hand[0].handle();

    assert!(
        runner
            .game
            .play_card_from_effect_without_cost(1, played_by_effect)
            .is_some(),
        "opponent effect play precondition"
    );

    assert!(
        runner.pending_selection().is_none(),
        "no Tamer on own field must suppress the free-digivolve offer"
    );
}

/// [All Turns] Trigger does NOT fire when [Imperialdramon: Fighter Mode] is not in hand.
///
/// The `select_hand` step filters for `Imperialdramon: Fighter Mode`; with none in
/// hand the clause produces no eligible selection and no digivolve occurs.
#[test]
fn bt16_028_all_turns_no_fighter_mode_in_hand_suppresses_digivolve_offer() {
    let mut runner = dragon_mode_builder()
        .add_card(tamer_card("OWN-TAMER", "Own Tamer"))
        .add_card(make_test_card("OPP-EFFECT-PLAY", "Opponent Effect Play"))
        .hand(1, &["OPP-EFFECT-PLAY"])
        .memory(0)
        .start();

    // Dragon Mode + Tamer on own field, but hand has NO Fighter Mode (BT16-027).
    let dragon = runner.place_on_field(0, "BT16-028", None);
    runner.place_on_field(0, "OWN-TAMER", None);
    let played_by_effect = runner.game.players[1].hand[0].handle();

    assert!(
        runner
            .game
            .play_card_from_effect_without_cost(1, played_by_effect)
            .is_some(),
        "opponent effect play precondition"
    );

    assert!(
        runner.pending_selection().is_none(),
        "no Fighter Mode in hand must suppress the free-digivolve offer"
    );
    // Dragon Mode is still on top of its own stack — no digivolve happened.
    let top_name = runner.game.players[0].battle_area[dragon.index as usize]
        .top_card()
        .card_name(&runner.game.card_data)
        .to_string();
    assert_eq!(
        top_name, "Imperialdramon: Dragon Mode",
        "Dragon Mode must not digivolve when no Fighter Mode is in hand"
    );
}

/// [All Turns] Trigger fires for effect-DIGIVOLVING opponent Digimon, not just effect-play.
///
/// The clause's `when` list includes `on_digivolve`; an effect-initiated digivolve
/// of an opponent's Digimon must satisfy the same effect-initiated gate.
#[test]
fn bt16_028_all_turns_effect_digivolves_opp_digimon_also_triggers() {
    // Opponent's evo card is a plain, effect-less Digimon so its digivolve does
    // not introduce unrelated triggers; only Dragon Mode's observer should fire.
    let mut runner = dragon_mode_and_fighter_builder()
        .add_card(tamer_card("OWN-TAMER", "Own Tamer"))
        .add_card(make_test_card("OPP-EVO", "Opponent Evo Digimon"))
        .hand(0, &["BT16-027"])
        .hand(1, &["OPP-EVO"])
        .memory(0)
        .start();

    let dragon = runner.place_on_field(0, "BT16-028", None);
    runner.place_on_field(0, "OWN-TAMER", None);

    // Opponent base Digimon to digivolve onto.
    let opp_base = runner.place_on_field(1, "BT16-028", None);

    // Effect-initiated digivolve of the opponent's Digimon (OPP-EVO from hand
    // index 0). `ignore_requirements` bypasses the level/color gate; what matters
    // is the digivolve is effect-initiated and targets an opponent Digimon.
    assert!(
        runner.game.effect_initiated_digivolve_ignore_requirements(
            1,
            0,
            opp_base,
            digimon_engine::enums::CostDelta::Free,
            PlaySource::ByEffect,
        ),
        "opponent effect digivolve precondition"
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "effect-initiated opponent digivolve should expose the free-digivolve choice"
    );
    for _ in 0..3 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("execute pending Dragon Mode follow-up");
    }
    runner.auto_resolve().expect("finish free digivolve");

    let top_name = runner.game.players[0].battle_area[dragon.index as usize]
        .top_card()
        .card_name(&runner.game.card_data)
        .to_string();
    assert_eq!(
        top_name, "Imperialdramon: Fighter Mode",
        "Dragon Mode should free-digivolve into Fighter Mode after opponent effect digivolve"
    );
}
