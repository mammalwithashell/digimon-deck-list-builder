//! BT6-102 Tropical Venom — Option, Green, Cost 0.
//! Traits: (none printed).
//!
//! # Card text (per-card JSON / card image — verbatim)
//!
//! [Main] 1 of your opponent's Digimon gains "[On Deletion] Lose 2 memory"
//! until the end of their next turn.
//!
//! Security Effect: — (none printed; the card back shows a blank Security
//! Effect box, confirmed by the card image and the empty
//! `security_effect_description_eng` field).
//!
//! # Official Q&A (data/card_bundles/BT6-102.md)
//! "Your opponent loses 2 memory, which means you gain 2 memory. If you use
//! this card to give '[On Deletion] Lose 2 memory' to an opponent's Digimon,
//! you gain 2 memory when that Digimon is deleted."
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT6/Green/BT6_102.cs`
//!
//! DCGO `EffectTiming.OptionSkill` (= `[Main]`):
//!   - `CardEffectCommons.CanTriggerOptionMainEffect` gates the outer
//!     activation (mandatory — no `canNoSelect` argument on the outer
//!     `ActivateClass`).
//!   - Gated on `CardEffectCommons.HasMatchConditionPermanent` (an eligible
//!     opponent Digimon exists) before installing the picker.
//!   - `SelectPermanentEffect` with `canNoSelect: false`, `maxCount: 1`,
//!     `Mode.Custom` over opponent battle-area Digimon — a single MANDATORY
//!     target pick (no decline).
//!   - On pick: `CardEffectCommons.AddEffectToPermanent(targetPermanent:
//!     selectedPermanent, effectDuration: EffectDuration.UntilOpponentTurnEnd,
//!     ..., timing: EffectTiming.OnDestroyedAnyone)` installs a grant that
//!     fires when the carrier is deleted, expiring at the end of the
//!     opponent's (carrier's controller's) next turn.
//!   - The granted body: `CanActivateCondition1` requires
//!     `IsTopCardInTrashOnDeletion` (fires AFTER the carrier's top card has
//!     moved to trash — matches the engine's post-trash OnDeletion
//!     contract, rule 25/`Game::delete_permanents_batch`).
//!   - `ActivateCoroutine1`: `_topCard.Owner.AddMemory(-2, activateClass1)`
//!     — `_topCard.Owner` is the CARRIER's controller (the opponent), so the
//!     opponent loses 2 memory (== the caster gains 2 memory, confirmed by
//!     the Official Q&A above).
//!
//! # DSL mapping
//! `select_opponent_permanent` (mandatory, filter: kind digimon) binds the
//! target, then `grant_triggered_effect { target: <binding>, timing:
//! on_deletion, expiry: end_of_opponents_next_turn, body: [lose_memory: 2] }`
//! installs the granted clause. This is the exact BT3-109 idiom
//! (`select_own_permanent` → `grant_triggered_effect { timing: on_deletion,
//! ... }`) with the opponent-scoped select swapped in — and the EX1-068
//! "twin" card (same DCGO family, `grant_triggered_effect` to ALL opponent
//! Digimon at `when_attacking` instead of ONE opponent Digimon at
//! `on_deletion`). The granted body runs with `ctx.player = carrier.player`
//! (`Game::fire_granted_triggered_effects` / the queue-routed dispatch in
//! `enqueue_from_permanent`), so `lose_memory: 2` inside the body correctly
//! charges the carrier's controller (the opponent) — no extra `of:` needed.
//!
//! # Patterns this test covers (docs/RUST_DSL_TEST_API.md §4.3)
//! - select_opponent_permanent mandatory single-target pick (no decline)
//! - grant_triggered_effect (Track H) with `on_deletion` timing + turn-scoped
//!   expiry (`end_of_opponents_next_turn`)
//! - F5 OnDeletion → memory swing (lose_memory in a granted body)
//! - Track-A/rule-25 post-trash OnDeletion firing (asserted via trash
//!   membership check ordering is implicit in the engine's batched-deletion
//!   contract; this card's own test focuses on the memory-swing outcome)
//!
//! # Per-clause status
//! | Clause | Status |
//! |--------|--------|
//! | [Main] select 1 opp Digimon, grant "[On Deletion] Lose 2 memory" until end of their next turn | IMPLEMENTED |

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledModifierTarget, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::events::GameEvent;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT6-102";

fn opp_digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card_with_level(id, name, 4);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(5000);
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Player 0 holds Tropical Venom; player 1 has one eligible Digimon on
/// their battle area.
fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT6-102 YAML must be present in the embedded DSL pack")
        .add_card(opp_digimon("OPP-DIGI", "OppDigi"))
        .add_card(filler("PAD"))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
        .memory(5)
        .start()
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt6_102_compiles_and_is_in_pack() {
    let r = runner();
    assert!(
        r.compiled_card(CARD_ID).is_some(),
        "BT6-102 must be present in the embedded DSL pack"
    );
}

#[test]
fn bt6_102_is_option_green_cost_0() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("BT6-102 compiled");

    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "BT6-102 must be an Option card"
    );
    assert_eq!(card.cost, Some(0), "BT6-102 prints Cost 0");
    assert!(
        card.color
            .iter()
            .any(|c| matches!(c, digimon_dsl::compiled::CompiledColor::Green)),
        "BT6-102 must be a Green Option (printed card_colors=[3]); got {:?}",
        card.color
    );
}

/// Exactly one clause: the [Main] grant. No separate Security clause —
/// the card image + `security_effect_description_eng` are both blank.
#[test]
fn bt6_102_has_exactly_one_clause() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("BT6-102 compiled");
    assert_eq!(
        card.effects.len(),
        1,
        "expected exactly 1 clause ([Main] grant only, no [Security] text); got {} clauses",
        card.effects.len()
    );
}

/// The [Main] clause is faithfully shaped: `MainFromHand` triggered clause,
/// mandatory (DCGO outer `ActivateClass` has no `canNoSelect` — printed text
/// has no "you may"), whose process contains a `GrantTriggeredEffect` with
/// `timing = on_deletion`, `expiry = end_of_opponents_next_turn`, and a body
/// that loses 2 memory.
#[test]
fn bt6_102_main_from_hand_grant_clause_shape() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("BT6-102 compiled");

    let main_clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("BT6-102 must carry a MainFromHand [Main] clause");

    assert!(
        !main_clause.optional,
        "[Main] clause is mandatory — printed text has no 'you may', and DCGO's outer \
         ActivateClass has no canNoSelect"
    );
    assert!(
        !main_clause.once_per_turn,
        "no [Once Per Turn] marker in the printed text"
    );

    let grant = main_clause
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::GrantTriggeredEffect {
                target,
                timing,
                expiry,
                body,
            } => Some((target, timing, expiry, body)),
            _ => None,
        })
        .expect("[Main] process must contain a grant_triggered_effect step");

    let (target, timing, expiry, body) = grant;
    assert_eq!(
        timing, "on_deletion",
        "granted trigger fires '[On Deletion]' per printed text"
    );
    assert_eq!(
        expiry, "end_of_opponents_next_turn",
        "grant lasts 'until the end of their next turn'"
    );
    assert!(
        matches!(target, CompiledModifierTarget::Binding(_)),
        "target must be the previously-selected single opponent Digimon (a binding), \
         got {target:?}"
    );
    assert!(
        body.iter()
            .any(|s| matches!(s, CompiledStep::LoseMemory(2))),
        "granted body must lose 2 memory; got {body:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (positive / negative split, per §11.3)
// ════════════════════════════════════════════════════════════════════════════

/// Negative: with NO opponent Digimon on the battle area, [Main] fires but
/// the mandatory select is a no-op (DCGO: `HasMatchConditionPermanent` gates
/// the picker install; the DSL `select_opponent_permanent` step silently
/// no-ops with an empty candidate set — BT23-047 precedent).
#[test]
fn bt6_102_no_selection_installs_with_no_opponent_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT6-102 YAML loads")
        .add_card(filler("PAD"))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(5)
        .start();

    let fired = r.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "BT6-102 [Main] must fire (mandatory, no condition gate on the option itself)"
    );
    let _ = r.auto_resolve();

    assert!(
        r.pending_selection().is_none(),
        "no selection should remain pending when there is no opponent Digimon to target"
    );
}

/// Positive: with an eligible opponent Digimon present, [Main] installs a
/// MANDATORY (no PASS) single-target OppField-style selection.
#[test]
fn bt6_102_selection_installs_with_eligible_opponent_digimon() {
    let mut r = runner();
    let _opp = r.place_on_field(1, "OPP-DIGI", Some(0));

    let fired = r.game.activate_hand_main(0, 0);
    assert!(fired, "BT6-102 [Main] must fire");

    let pending = r
        .pending_selection()
        .expect("[Main] must install a target selection when an opponent Digimon exists");
    assert!(
        !pending.is_optional,
        "the target pick is mandatory — no PASS / decline (DCGO canNoSelect: false)"
    );
    assert!(
        !pending.valid_action_ids.contains(&PASS),
        "PASS must not be a legal action for this mandatory pick"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: grant installs, then fires on deletion
// ════════════════════════════════════════════════════════════════════════════

/// Core scenario: select the opponent's Digimon, grant it "[On Deletion]
/// Lose 2 memory", delete it, and confirm the memory swing fires exactly
/// once and is attributed to BT6-102 in the event log.
#[test]
fn bt6_102_grant_fires_lose_2_memory_on_deletion() {
    let mut r = runner();
    let target = r.place_on_field(1, "OPP-DIGI", Some(0));

    let fired = r.game.activate_hand_main(0, 0);
    assert!(fired, "BT6-102 [Main] must fire");

    let pending = r
        .pending_selection()
        .expect("target selection must install");
    let pick = *pending
        .valid_action_ids
        .first()
        .expect("the opponent Digimon must be a selectable target");
    r.execute_action(0, pick).expect("target pick resolves");
    let _ = r.auto_resolve();

    // Granting does not remove or otherwise affect the target permanent.
    assert_eq!(
        r.battle_area_size(1),
        1,
        "granting the [On Deletion] clause must not move or delete the target"
    );

    let cp = r.event_checkpoint();
    let memory_before = r.memory();

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::Battle);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "the carrier must be deleted");

    let events = r.events_since(cp);
    let memory_changes: Vec<&GameEvent> = events
        .iter()
        .filter(|e| matches!(e, GameEvent::MemoryChange { .. }))
        .collect();
    assert_eq!(
        memory_changes.len(),
        1,
        "the granted [On Deletion] body must fire exactly once on the carrier's deletion; \
         got {} MemoryChange events: {:?}",
        memory_changes.len(),
        memory_changes
    );

    match memory_changes[0] {
        GameEvent::MemoryChange {
            player,
            delta,
            source_card_id,
            ..
        } => {
            assert_eq!(
                *player, 1,
                "the memory loss is attributed to the carrier's controller (the opponent, \
                 player 1), matching DCGO's `_topCard.Owner.AddMemory(-2, ...)`"
            );
            assert_eq!(
                delta.abs(),
                2,
                "the granted body loses exactly 2 memory; got delta {delta}"
            );
            assert_eq!(
                source_card_id.as_deref(),
                Some(CARD_ID),
                "the memory change must be attributed to BT6-102 (the grantor's source card)"
            );
        }
        other => panic!("expected MemoryChange, got {other:?}"),
    }

    // Official Q&A: "you gain 2 memory when that Digimon is deleted" — the
    // caster (player 0, the turn player at deletion time) benefits.
    let memory_after = r.memory();
    assert_ne!(
        memory_before, memory_after,
        "the shared memory gauge must move when the granted body fires"
    );
}

/// The granted clause is a genuine trigger (not a one-shot inline effect):
/// suspending/unsuspending or otherwise leaving the carrier alive across a
/// turn boundary must NOT fire it — only an actual deletion does.
#[test]
fn bt6_102_grant_does_not_fire_without_deletion() {
    let mut r = runner();
    let target = r.place_on_field(1, "OPP-DIGI", Some(0));

    let fired = r.game.activate_hand_main(0, 0);
    assert!(fired, "BT6-102 [Main] must fire");
    let pending = r.pending_selection().expect("target selection installs");
    let pick = *pending.valid_action_ids.first().expect("target exists");
    r.execute_action(0, pick).expect("target pick resolves");
    let _ = r.auto_resolve();

    let memory_before = r.memory();

    // Advance through a full round trip without deleting the carrier.
    r.end_turn(); // -> player 1's turn
    r.end_turn(); // -> back to player 0's turn

    assert_eq!(
        r.memory(),
        memory_before,
        "no memory change should occur while the carrier merely survives across turns"
    );
    assert_eq!(
        r.battle_area_size(1),
        1,
        "the carrier remains on the field (not deleted)"
    );
}

/// Selection targeting: with two opponent Digimon, exactly one is targeted
/// and the OTHER one's later deletion does NOT trigger the granted memory
/// loss — the grant is carrier-specific, not "all opponent Digimon" (unlike
/// the EX1-068 twin, which grants to ALL opponent Digimon).
#[test]
fn bt6_102_grant_is_scoped_to_the_selected_carrier_only() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT6-102 YAML loads")
        .add_card(opp_digimon("OPP-A", "OppA"))
        .add_card(opp_digimon("OPP-B", "OppB"))
        .add_card(filler("PAD"))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
        .memory(5)
        .start();

    let a = r.place_on_field(1, "OPP-A", Some(0));
    let b = r.place_on_field(1, "OPP-B", Some(0));

    let fired = r.game.activate_hand_main(0, 0);
    assert!(fired, "BT6-102 [Main] must fire");
    let pending = r.pending_selection().expect("target selection installs");
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "both opponent Digimon must be legal targets for the single mandatory pick"
    );

    // Target A specifically. `select_opponent_permanent` enumerates the
    // opponent's battle area in index order, so the first legal action id
    // corresponds to A (placed at index 0); B is at index 1.
    let pick = pending.valid_action_ids[0];
    r.execute_action(0, pick).expect("target A pick resolves");
    let _ = r.auto_resolve();

    let memory_before = r.memory();

    // Delete B (the ungranted Digimon) — must NOT trigger a memory swing.
    r.game
        .delete_permanent_with_cause(b, ReplacementCause::Battle);
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        memory_before,
        "deleting the UNGRANTED opponent Digimon (B) must not lose memory"
    );

    // Now delete A (the granted carrier) — must trigger the memory swing.
    r.game
        .delete_permanent_with_cause(a, ReplacementCause::Battle);
    let _ = r.auto_resolve();

    assert_ne!(
        r.memory(),
        memory_before,
        "deleting the GRANTED carrier (A) must trigger the memory swing"
    );
}
