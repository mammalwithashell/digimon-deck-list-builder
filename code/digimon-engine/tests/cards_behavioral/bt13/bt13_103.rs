//! BT13-103 Akihiro Kurata — Tamer, Purple, Cost 3 (no DP / no level).
//!
//! # Card text (data/cards.json `effect_description_eng`)
//!
//! **[Your Turn]** When you would play a card with [Belphemon] in its name, by
//! deleting 1 of your Digimon with [Gizmon] in its name, reduce the play cost
//! by the play cost of the deleted Digimon.
//!
//! **[End of Opponent's Turn] [Once Per Turn]** ＜Draw 1＞ (Draw 1 card from
//! your deck.) and trash 1 card in your hand. Then, by placing this Tamer under
//! 1 of your Digimon with [Belphemon] in its name as its bottom digivolution
//! card, delete 1 of your opponent's level 6 Digimon.
//!
//! **Inherited (Security):** Security Effect [Security] Play this card without
//! paying the cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT13/Purple/BT13_103.cs`
//!
//! # Clause decomposition (verified against DCGO)
//!
//! 1. **[Your Turn] BeforePayCost cost reduction** (DCGO `EffectTiming.BeforePayCost`
//!    + an AI-only `EffectTiming.None` mirror). When the controller would play a
//!    [Belphemon]-name card on their turn, they MAY (`canNoSelect: true`,
//!    `isOptional: true`) select 1 of their own [Gizmon]-name Digimon to delete;
//!    the play cost is then reduced by **the deleted Digimon's printed play
//!    cost** (`permanent.CostJustBeforeRemoveField`). The reduction amount is
//!    therefore determined by an *interactive selection made during the cost*.
//!
//!    ── IMPLEMENTED 2026-07-03 (G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST) ─
//!    The DSL `kind: cost_reduction` clause now expresses this via a `pay_cost`
//!    of `[select_own_permanent{Gizmon} + delete_for_cost_reduction]` with NO
//!    static `amount`: the interactive select parks on the resumable data VM
//!    (clone-safe, rule 28); `delete_for_cost_reduction` snapshots the deleted
//!    permanent's printed play cost (pre-removal, rule 25) into
//!    `Game::pending_cost_reduction_amount_override`; and the play-from-hand cost
//!    continuation credits it once the park resolves. The reducer's `condition`
//!    carries a pay-cost actionability guard (DCGO `CanActivateCondition`), so it
//!    is not offered when no Gizmon is in play. Substrate tests:
//!    `code/digimon-engine/tests/cost_hooks/pay_cost_play_delete_reducer.rs`.
//!    The Clause-1 behavioral test below is now active (no longer `#[ignore]`d).
//!
//! 2. **[End of Opponent's Turn] [Once Per Turn]** (DCGO `EffectTiming.OnEndTurn`,
//!    `IsOpponentTurn`, `SetUpActivateClass(..., 1, false, ...)` → OPT lockout,
//!    NOT optional). Mandatory: `<Draw 1>` then trash 1 card from hand
//!    (`canNoSelect: false`). THEN, by placing THIS Tamer under 1 of the
//!    controller's [Belphemon]-name Digimon as its bottom digivolution card
//!    (`canNoSelect: true` → optional "by placing"), delete 1 of the opponent's
//!    level-6 Digimon (`canNoSelect: false` once the placement happened —
//!    gated on `digivolutionCardAdded`). Each pick surfaces via
//!    `pending_selection`.
//!
//! 3. **[Security]** play this card without paying the cost (DCGO
//!    `EffectTiming.SecuritySkill` → `PlaySelfTamerSecurityEffect`). Mandatory.
//!
//! # Patterns this test covers
//! - `end_of_opponents_turn` triggered timing + `once_per_turn` (OPT lockout).
//! - mandatory draw + mandatory hand-trash.
//! - optional "by placing this Tamer under a [Belphemon] Digimon" via
//!   `place_as_bottom_source { source: source, target: <belphemon> }` (the
//!   inverse of P-215 / BT13-007 — places the CARRIER permanent itself).
//! - conditional opponent level-6 delete gated on the placement having occurred.
//! - `[Security] play_from_security` (structural; behavioral substrate covered
//!   by BT22-094 / BT21-015 sister tests).

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "BT13-103";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_filler(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

/// A purple level-6 [Belphemon]-name Digimon — the placement target for
/// Clause 2 and a play-target for Clause 1.
fn make_belphemon(card_id: &str, name: &str, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Purple];
    c.traits = vec!["Demon Lord".to_string()];
    c
}

/// A [Gizmon]-name Digimon — the Clause-1 deletion cost candidate.
fn make_gizmon(card_id: &str, name: &str, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Purple];
    c.traits = vec!["Machine".to_string()];
    c
}

/// A generic opponent level-6 Digimon — Clause 2 delete target.
fn make_opp_level6(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = 12;
    c.colors = vec![CardColor::Red];
    c
}

/// A generic opponent level-5 Digimon — must NOT be a Clause-2 delete target.
fn make_opp_level5(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Red];
    c
}

/// Drive every pending selection by accepting the first non-PASS action.
fn drive_accept(runner: &mut DebugRunner, player: u8) {
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(player, accept)
            .expect("execute pending selection");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT13-103 must compile from the embedded DSL pack as a PURPLE Tamer, cost 3.
#[test]
fn bt13_103_compiles_as_purple_tamer_cost_3() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(
        card.kind,
        CompiledCardKind::Tamer,
        "BT13-103 is a Tamer (card_kind 1)"
    );
    assert_eq!(card.cost, Some(3), "BT13-103 prints Cost 3");
    assert!(
        card.color.iter().any(|c| *c == CompiledColor::Purple),
        "BT13-103 is PURPLE (card_colors [6]); colors were {:?}",
        card.color
    );
    assert!(
        !card.color.iter().any(|c| *c == CompiledColor::Black),
        "BT13-103 must NOT be black — card_colors [6] is Purple"
    );
}

/// The [End of Opponent's Turn] clause must compile as a triggered clause with
/// `end_of_opponents_turn` timing and the [Once Per Turn] lockout.
#[test]
fn bt13_103_eot_clause_is_once_per_turn_and_mandatory() {
    let card = compiled(CARD_ID);
    let eot = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::EndOfOpponentsTurn])
        .expect("BT13-103 must have an end_of_opponents_turn triggered clause");

    assert!(
        eot.once_per_turn,
        "EOT clause carries [Once Per Turn] (DCGO SetUpActivateClass max-per-turn 1)"
    );
    assert!(
        !eot.optional,
        "EOT clause is NOT optional — the Draw 1 + trash 1 are mandatory \
         (DCGO isOptional false); only the inner 'by placing' is opt-in"
    );
    assert_eq!(
        eot.scope,
        CompiledScope::FaceUp,
        "EOT clause is own-field (FaceUp) scope"
    );
}

/// The [Security] clause must compile as a triggered on_security clause,
/// mandatory.
#[test]
fn bt13_103_security_clause_is_mandatory() {
    let card = compiled(CARD_ID);
    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("BT13-103 must have an on_security triggered clause");

    assert!(
        !security.optional,
        "[Security] effects are not opt-out (rules manual §16; DCGO \
         PlaySelfTamerSecurityEffect has no canNoSelect)"
    );
    assert!(
        !security.once_per_turn,
        "[Security] clause has no [Once Per Turn]"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause 2 [End of Opponent's Turn][OPT]
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: BT13-103 on field at end of opponent's turn → draw 1, trash 1
/// from hand, place THIS Tamer under the controller's Belphemon, then delete an
/// opponent level-6 Digimon.
#[test]
fn bt13_103_eot_draws_trashes_places_and_deletes_opp_level6() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-103 in embedded DSL pack")
        .add_card(make_belphemon("BELPH", "Belphemon: Sleep Mode", 11))
        .add_card(make_opp_level6("OPP6", "Opp Level6"))
        .add_card(make_filler("FILLER"))
        .add_card(make_filler("TRASHME"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["TRASHME"])
        .memory(0) // 0 memory → end_turn flips to P1's turn
        .start();

    // BT13-103 on P0's field; P0's own Belphemon on field; opponent level-6 on
    // P1's field.
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "BELPH", Some(1));
    runner.place_on_field(1, "OPP6", Some(0));

    // Pass to P1's turn, then end P1's turn → end_of_opponents_turn fires for P0.
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: P1's turn");
    runner.end_turn();

    // The EOT clause should have queued. Accept all prompts (draw is automatic;
    // trash 1, place under Belphemon, delete opp level-6).
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    // The mandatory "<Draw 1> and trash 1 card in your hand" must have trashed
    // the only pre-draw hand card (TRASHME). We assert on the trash pile rather
    // than the net hand count, because P0's own turn-start draw also fires when
    // play returns to P0 — so net hand size is not a stable signal for the EOT
    // clause's draw+trash alone.
    let trash_ids: Vec<String> = runner.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        trash_ids.contains(&"TRASHME".to_string()),
        "the mandatory hand-trash must have trashed a hand card; trash was {trash_ids:?}"
    );

    // BT13-103 must no longer be a standalone top card — it was placed as a
    // bottom digivolution source under BELPH.
    let kurata_top = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        !kurata_top,
        "BT13-103 must be tucked under BELPH (not a standalone top card) after placement"
    );
    let belph_has_kurata_source = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_id(&runner.game.card_data) == "BELPH")
        .any(|p| {
            p.card_sources
                .iter()
                .any(|s| s.card_id(&runner.game.card_data) == CARD_ID)
        });
    assert!(
        belph_has_kurata_source,
        "BT13-103 must be a digivolution source under BELPH after placement"
    );

    // The opponent's level-6 Digimon must be deleted.
    let opp6_present = runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP6");
    assert!(
        !opp6_present,
        "opponent's level-6 Digimon must be deleted after the placement"
    );
}

/// Negative: no Belphemon on field → the "by placing" cost can't be paid, so
/// the opponent level-6 delete must NOT happen (DCGO gates the delete on
/// `digivolutionCardAdded`). The mandatory Draw 1 + trash 1 still happen.
#[test]
fn bt13_103_eot_no_belphemon_no_delete() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-103 in embedded DSL pack")
        .add_card(make_opp_level6("OPP6", "Opp Level6"))
        .add_card(make_filler("FILLER"))
        .add_card(make_filler("TRASHME"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["TRASHME"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP6", Some(0));

    runner.end_turn();
    runner.end_turn();
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    // BT13-103 stays a standalone top card (no Belphemon to tuck under).
    let kurata_top = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        kurata_top,
        "BT13-103 must remain on field (no Belphemon to place it under)"
    );

    // Opponent level-6 must survive — the delete is gated on the placement.
    let opp6_present = runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP6");
    assert!(
        opp6_present,
        "opponent level-6 must survive — delete is gated on the placement having happened"
    );
}

/// Negative: opponent has only a level-5 Digimon → even with the placement
/// done, there is no legal delete target (level 6 only).
#[test]
fn bt13_103_eot_opp_level5_is_not_a_delete_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-103 in embedded DSL pack")
        .add_card(make_belphemon("BELPH", "Belphemon: Sleep Mode", 11))
        .add_card(make_opp_level5("OPP5", "Opp Level5"))
        .add_card(make_filler("FILLER"))
        .add_card(make_filler("TRASHME"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["TRASHME"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "BELPH", Some(1));
    runner.place_on_field(1, "OPP5", Some(0));

    runner.end_turn();
    runner.end_turn();
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    // The opponent level-5 Digimon must survive (not level 6).
    let opp5_present = runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP5");
    assert!(
        opp5_present,
        "opponent level-5 must survive — only level-6 Digimon are legal delete targets"
    );
}

/// OPT lockout: the EOT clause is [Once Per Turn]. It must fire at most once
/// per opponent's turn. We trigger one opponent turn and assert the placement
/// happened exactly once (BT13-103 tucked under BELPH, single source instance).
#[test]
fn bt13_103_eot_is_once_per_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-103 in embedded DSL pack")
        .add_card(make_belphemon("BELPH", "Belphemon: Sleep Mode", 11))
        .add_card(make_opp_level6("OPP6", "Opp Level6"))
        .add_card(make_filler("FILLER"))
        .add_card(make_filler("TRASHME"))
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["TRASHME"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "BELPH", Some(1));
    runner.place_on_field(1, "OPP6", Some(0));

    runner.end_turn();
    runner.end_turn();
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    // BT13-103 placed as exactly one source under BELPH (single activation).
    let kurata_source_count: usize = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_id(&runner.game.card_data) == "BELPH")
        .map(|p| {
            p.card_sources
                .iter()
                .filter(|s| s.card_id(&runner.game.card_data) == CARD_ID)
                .count()
        })
        .sum();
    assert_eq!(
        kurata_source_count, 1,
        "BT13-103 must be placed under BELPH exactly once (OPT lockout)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 1 [Your Turn] BeforePayCost cost reduction
//                          (BLOCKED — see header; ignored pending gap fix)
// ═══════════════════════════════════════════════════════════════════════════════

/// IMPLEMENTED (2026-07-03, G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST):
/// when the controller plays a [Belphemon]-name card on their turn, they may
/// delete 1 of their [Gizmon]-name Digimon to reduce the play cost by that
/// deleted Digimon's printed play cost. The interactive select parks on the
/// resumable data VM; the `delete_for_cost_reduction` step snapshots the deleted
/// permanent's cost and the play-cost continuation credits it. (Costs kept small
/// so the memory cap (10) does not clamp the observation.)
#[test]
fn bt13_103_belphemon_play_cost_reduced_by_deleted_gizmon_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-103 in embedded DSL pack")
        .add_card(make_belphemon("BELPH", "Belphemon: Rage Mode", 5))
        .add_card(make_gizmon("GIZMON", "Gizmon: AT", 3))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["BELPH"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "GIZMON", Some(1));

    let mem_before = runner.memory();
    let _ = runner.play(0, 0); // play the Belphemon-name card
    assert!(
        runner.pending_selection_view().is_some(),
        "playing a [Belphemon] card offers the optional delete-Gizmon reducer"
    );
    drive_accept(&mut runner, 0); // accept the optional delete-Gizmon cost
    let _ = runner.auto_resolve();

    // Belphemon cost 5 reduced by deleted Gizmon's cost 3 → effective 2.
    let cost_paid = mem_before - runner.memory();
    assert_eq!(
        cost_paid, 2,
        "Belphemon (cost 5) reduced by deleted Gizmon's cost (3) → 2; got {cost_paid}"
    );
    // The Gizmon must have been deleted as the cost.
    let gizmon_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "GIZMON");
    assert!(!gizmon_present, "GIZMON must be deleted as the cost");
}

/// Negative: with no [Gizmon] in play, the clause-1 reducer is NOT offered
/// (DCGO CanActivateCondition = HasMatchConditionPermanent) — the [Belphemon]
/// card plays at full cost.
#[test]
fn bt13_103_no_gizmon_no_reducer_offered() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-103 in embedded DSL pack")
        .add_card(make_belphemon("BELPH", "Belphemon: Rage Mode", 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["BELPH"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let _ = runner.play(0, 0);
    // No Gizmon → no reducer prompt; the play completes at full cost.
    let _ = runner.auto_resolve();
    assert_eq!(
        mem_before - runner.memory(),
        5,
        "no [Gizmon] in play → clause-1 reducer not offered, full cost 5 paid"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helpers (silence dead-code warnings across the test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_opp_level5("X", "X");
    let _ = make_gizmon("Y", "Y", 0);
}
