//! BT16-004 Minomon — Digi-Egg, Lv.2, Green.
//!
//! # Card text (code/digimon-engine/cards/bt16/BT16-004.json — verbatim)
//!
//! Inherited Effect [All Turns] [Once Per Turn]
//! When this Digimon deletes an opponent's Digimon in battle, if this Digimon
//! has 2 or more colors, gain 1 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Green/BT16_004.cs
//!
//! - Timing: `EffectTiming.OnDestroyedAnyone` (`on_any_deletion`), inherited
//!   (`SetIsInheritedEffect(true)`), OPT (`maxCount = 1`,
//!   `SetHashString("Memory+1_BT16_004")`).
//! - `CanUseCondition`:
//!     `CanTriggerOnPermanentDeleted` gated to opponent-battle-area deletions,
//!     then `CanTriggerWhenDeleteOpponentDigimonByBattle` with:
//!       - `WinnerCondition`: the permanent's `cardSources` contains this card
//!         (i.e. this card is somewhere in the winning permanent's stack —
//!         the DSL's `source_deleted_battle_opponent` idiom, gated by
//!         inherited scope so the "source" resolves to the carrying
//!         permanent's stack membership).
//!       - `WinnerRealCondition`: `permanent.TopCard.CardColors.Count >= 2`
//!         — the carrier's TOP card (not the full stack union) must have 2+
//!         colors. Maps to `self_color_count_gte: 2`.
//!       - `LoserCondition`: the deleted permanent belongs to the opponent.
//!       - `isOnlyWinnerSurvive: false` — the carrier does NOT need to
//!         survive the battle for this to fire (confirmed via DCGO source;
//!         contradicts a naive reading that "wins" implies "survives").
//! - `CanActivateCondition`: carrier still exists on the battle area (or, per
//!   the OnDestroyedAnyone timing, the trigger context still resolves).
//! - Body: `card.Owner.AddMemory(1, activateClass)` — gain 1 memory,
//!   unconditional once the gate passes.
//!
//! # Patterns this test covers (docs/RUST_DSL_TEST_API.md §4.3)
//! - G4 Inherited effect on a Digi-Egg (base-level carrier via digivolve stack)
//! - F5 OnAnyDeletion battle-delete observer (BT25-015 clause-B idiom)
//! - D-adjacent conditional gate: `self_color_count_gte` (P-117 / BT12-031 idiom)
//! - E-adjacent: mandatory effect (no "you may") — no PASS branch to expose
//! - Section 5: OPT enforcement (once per turn, resets after a turn cycle)
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                                          | YAML clause                                                                | Status |
//! |-----------------------------------------------------------------------------|-----------------------------------------------------------------------------|--------|
//! | "Inherited Effect"                                                          | `scope: inherited`                                                          | OK     |
//! | "[All Turns]"                                                               | `active_when: { all_turns: true }`                                          | OK     |
//! | "[Once Per Turn]"                                                           | `once_per_turn: true`                                                       | OK     |
//! | "When this Digimon deletes an opponent's Digimon in battle"                 | `when: [on_any_deletion]` + `condition: { source_deleted_battle_opponent: true }` | OK |
//! | "if this Digimon has 2 or more colors"                                      | `condition: { self_color_count_gte: 2 }` (top-card colors, DCGO-faithful)  | OK     |
//! | "gain 1 memory"                                                             | `gain_memory: 1`                                                            | OK     |
//! | Mandatory (no "you may")                                                    | no `optional: true` flag                                                    | OK     |
//! | `isOnlyWinnerSurvive: false` (winner need not survive)                      | `source_deleted_battle_opponent` fires from the trigger-context battle-cause check, not from live board presence — carrier survival is not required | OK |

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT16-004";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A two-color carrier (Green + Blue) placed on top of the Minomon source —
/// satisfies `self_color_count_gte: 2` (DCGO: `TopCard.CardColors.Count >= 2`).
fn make_dual_color_carrier(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 3);
    card.colors = vec![CardColor::Green, CardColor::Blue];
    card.dp = Some(2000);
    card
}

/// A mono-color carrier (Green only) — fails `self_color_count_gte: 2`.
fn make_mono_color_carrier(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 3);
    card.colors = vec![CardColor::Green];
    card.dp = Some(2000);
    card
}

/// A weak opponent Digimon guaranteed to lose a battle against a 2000 DP
/// carrier (used for the delete-and-gain-memory behavioral tests).
fn make_weak_opp(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 3);
    card.colors = vec![CardColor::Red];
    card.dp = Some(500);
    card
}

/// A very strong opponent Digimon that survives an attack from the 2000 DP
/// carrier — used for the negative "no deletion, no memory" cases.
fn make_tanky_opp(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 3);
    card.colors = vec![CardColor::Red];
    card.dp = Some(99_000);
    card
}

fn minomon_base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT16-004 YAML parses and compiles")
        .memory(10)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt16_004_yaml_has_printed_metadata() {
    let runner = minomon_base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT16-004 must be in the embedded DSL pack");

    assert_eq!(card.name, "Minomon");
    assert_eq!(card.level, Some(2));
}

#[test]
fn bt16_004_has_exactly_one_inherited_triggered_clause() {
    let runner = minomon_base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");

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
        1,
        "BT16-004 has exactly one triggered clause (the inherited battle-delete gain)"
    );
}

#[test]
fn bt16_004_clause_is_inherited_on_any_deletion_once_per_turn() {
    let runner = minomon_base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "printed text is under 'Inherited Effect'"
    );
    assert!(
        clause.when.contains(&CompiledTiming::OnAnyDeletion),
        "must fire on any-deletion timing (battle-delete gate is a condition, not a timing)"
    );
    assert!(
        clause.once_per_turn,
        "[Once Per Turn] must be encoded as once_per_turn: true"
    );
}

#[test]
fn bt16_004_clause_is_mandatory_not_optional() {
    let runner = minomon_base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert!(
        !clause.optional,
        "no 'you may' in the printed text — the gain is mandatory, no PASS branch"
    );
}

// ─── Section 2 — Condition gating (positive / negative, split per §11.3) ─────

/// Positive: carrier has 2+ colors AND deletes an opponent Digimon in battle
/// → the inherited trigger installs (no pending selection needed — mandatory,
/// unconditional body — so we assert via the memory delta after auto_resolve).
#[test]
fn bt16_004_condition_fires_when_carrier_has_two_colors_and_wins_battle() {
    let mut runner = minomon_base()
        .add_card(make_dual_color_carrier("CARRIER-DUAL"))
        .add_card(make_weak_opp("OPP-WEAK"))
        .memory(3)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-DUAL"]);
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));

    let memory_before = runner.memory();

    runner.attack_digimon(carrier, opp, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "2+ color carrier deleting an opponent Digimon in battle must gain 1 memory"
    );
}

/// Negative: carrier has only 1 color (mono-color) — the `self_color_count_gte:
/// 2` condition must block the gain even though the battle-delete happens.
#[test]
fn bt16_004_condition_blocks_when_carrier_has_only_one_color() {
    let mut runner = minomon_base()
        .add_card(make_mono_color_carrier("CARRIER-MONO"))
        .add_card(make_weak_opp("OPP-WEAK"))
        .memory(3)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-MONO"]);
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));

    let memory_before = runner.memory();

    runner.attack_digimon(carrier, opp, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "a mono-color carrier must NOT gain memory even after deleting an opponent Digimon in battle"
    );
}

/// Negative: carrier has 2+ colors but does NOT delete the opponent Digimon in
/// battle (opponent survives) — `source_deleted_battle_opponent` must not be
/// satisfied, so no memory gain.
#[test]
fn bt16_004_condition_blocks_when_opponent_survives_battle() {
    let mut runner = minomon_base()
        .add_card(make_dual_color_carrier("CARRIER-DUAL"))
        .add_card(make_tanky_opp("OPP-TANKY"))
        .memory(3)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-DUAL"]);
    let opp = runner.place_on_field(1, "OPP-TANKY", Some(0));

    let memory_before = runner.memory();

    runner.attack_digimon(carrier, opp, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "no deletion of an opponent Digimon in battle → no memory gain"
    );
}

// ─── Section 3 — Behavioral outcome / event-log assertions ──────────────────

/// The memory gain must be attributed to BT16-004 in the emitted
/// `GameEvent::MemoryChange` (cost-firing / side-effect event-log test,
/// docs/RUST_DSL_TEST_API.md §5 Section 4).
#[test]
fn bt16_004_memory_gain_emits_memory_change_event_attributed_to_minomon() {
    let mut runner = minomon_base()
        .add_card(make_dual_color_carrier("CARRIER-DUAL"))
        .add_card(make_weak_opp("OPP-WEAK"))
        .memory(3)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-DUAL"]);
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));

    let cp = runner.event_checkpoint();
    runner.attack_digimon(carrier, opp, false);
    let _ = runner.auto_resolve();

    let events = runner.events_since(cp);
    let memory_events: Vec<&GameEvent> = events
        .iter()
        .filter(|e| matches!(e, GameEvent::MemoryChange { .. }))
        .collect();

    assert!(
        !memory_events.is_empty(),
        "the battle-delete gain must emit at least one MemoryChange event"
    );

    let attributed = memory_events.iter().any(|e| {
        matches!(
            e,
            GameEvent::MemoryChange { source_card_id, delta, .. }
                if source_card_id.as_deref() == Some(CARD_ID) && *delta == 1
        )
    });
    assert!(
        attributed,
        "a +1 MemoryChange attributed to BT16-004 must be present; got {memory_events:?}"
    );
}

/// The opponent Digimon that lost the battle is actually removed from the
/// opponent's battle area — confirms the "deletes an opponent's Digimon in
/// battle" precondition really happened, not just an incidental memory bump.
#[test]
fn bt16_004_battle_win_actually_deletes_the_opponent_digimon() {
    let mut runner = minomon_base()
        .add_card(make_dual_color_carrier("CARRIER-DUAL"))
        .add_card(make_weak_opp("OPP-WEAK"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-DUAL"]);
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));
    let opp_before = runner.battle_area_size(1);

    runner.attack_digimon(carrier, opp, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "the weak opponent Digimon must be deleted by the battle"
    );
}

// ─── Section 4 — OPT enforcement ─────────────────────────────────────────────

/// OPT: a second battle-delete by the SAME carrier in the SAME turn must not
/// grant a second +1 memory.
#[test]
fn bt16_004_opt_blocks_second_gain_same_turn() {
    let mut runner = minomon_base()
        .add_card(make_dual_color_carrier("CARRIER-DUAL"))
        .add_card(make_weak_opp("OPP-WEAK-1"))
        .add_card(make_weak_opp("OPP-WEAK-2"))
        .memory(3)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-DUAL"]);
    let opp1 = runner.place_on_field(1, "OPP-WEAK-1", Some(0));
    let opp2 = runner.place_on_field(1, "OPP-WEAK-2", Some(0));

    let memory_before = runner.memory();

    // First attack: deletes OPP-WEAK-1, gains 1 memory.
    runner.attack_digimon(carrier, opp1, false);
    let _ = runner.auto_resolve();
    let memory_after_first = runner.memory();
    assert_eq!(
        memory_after_first,
        memory_before + 1,
        "first battle-delete this turn must gain 1 memory"
    );

    // Unsuspend the carrier so it can attack again this turn.
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;

    // Second attack, same turn: deletes OPP-WEAK-2, but OPT must block the
    // second +1 memory.
    runner.attack_digimon(carrier, opp2, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_after_first,
        "OPT must lock out the second memory gain within the same turn"
    );
}

/// OPT resets after a full turn cycle — a battle-delete on a fresh turn gains
/// memory again.
#[test]
fn bt16_004_opt_resets_after_turn_cycle() {
    let mut runner = minomon_base()
        .add_card(make_dual_color_carrier("CARRIER-DUAL"))
        .add_card(make_weak_opp("OPP-WEAK-1"))
        .add_card(make_weak_opp("OPP-WEAK-2"))
        .add_card(make_test_card_with_level("FILLER", "Filler", 3))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(0, &["FILLER"; 5])
        .security(1, &["FILLER"; 5])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-DUAL"]);
    let opp1 = runner.place_on_field(1, "OPP-WEAK-1", Some(0));
    let _opp2 = runner.place_on_field(1, "OPP-WEAK-2", Some(0));

    // First attack this turn: gains 1 memory.
    runner.attack_digimon(carrier, opp1, false);
    let _ = runner.auto_resolve();
    let memory_after_first = runner.memory();

    // Full turn cycle.
    runner.end_turn();
    runner.end_turn();

    // Unsuspend the carrier.
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;

    // Re-resolve opp2's handle in case the battle area shifted.
    let opp2_after = {
        let battle = &runner.game.players[1].battle_area;
        assert!(
            !battle.is_empty(),
            "opp2 must still be present into the next-turn assertion"
        );
        runner.perm_handle(1, battle.len() - 1)
    };

    // Second attack, fresh turn: OPT reset → gains another 1 memory.
    runner.attack_digimon(carrier, opp2_after, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_after_first + 1,
        "OPT must reset after a full turn cycle, allowing another +1 memory gain"
    );
}

// ─── Section 5 — [All Turns] scoping (fires during opponent's turn too) ─────

/// [All Turns] means the inherited trigger is not restricted to the
/// controller's own turn — the carrier deleting an opponent Digimon in
/// battle while defending (i.e. during the opponent's turn) must still gain
/// memory. We simulate the opponent's turn by advancing `turn_count`/turn
/// ownership and having the OPPONENT attack INTO the carrier, causing the
/// carrier to win the battle (the attacker dies to the defender's DP).
#[test]
fn bt16_004_fires_during_opponents_turn_all_turns_scope() {
    let mut runner = minomon_base()
        .add_card(make_dual_color_carrier("CARRIER-DUAL"))
        .add_card(make_weak_opp("OPP-WEAK"))
        .memory(0)
        .start();

    // Player 0's carrier (2000 DP) sits on defense; player 1's weak attacker
    // (500 DP) attacks into it and loses the battle — the carrier (player 0)
    // is the battle winner and deletes an opponent's Digimon during the
    // OPPONENT's turn (player 1's turn).
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-DUAL"]);
    let opp_attacker = runner.place_on_field(1, "OPP-WEAK", Some(0));

    // Advance to player 1's turn so the attack is opponent-turn-initiated.
    runner.end_turn();
    runner.game.players[1].battle_area[opp_attacker.index as usize].is_suspended = false;

    let cp = runner.event_checkpoint();

    runner.attack_digimon(opp_attacker, carrier, false);
    let _ = runner.auto_resolve();

    // The memory counter is stored from the CURRENT turn player's (player 1)
    // perspective (`Game::gain_memory_for_player_sourced`): a gain credited
    // to the non-turn-player (player 0, Minomon's controller) moves the
    // counter toward player 0's side, i.e. it DECREASES the raw seesaw
    // value. We assert via the attributed MemoryChange event instead of a
    // raw-value direction guess, so the test doesn't encode an assumption
    // about the seesaw sign convention.
    let events = runner.events_since(cp);
    let attributed = events.iter().any(|e| {
        matches!(
            e,
            GameEvent::MemoryChange { source_card_id, delta, .. }
                if source_card_id.as_deref() == Some(CARD_ID) && *delta != 0
        )
    });
    assert!(
        attributed,
        "[All Turns] must let the inherited gain fire (a non-zero MemoryChange \
         attributed to BT16-004) even during the opponent's turn; got {events:?}"
    );
}
