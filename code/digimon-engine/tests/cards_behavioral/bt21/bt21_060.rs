//! BT21-060 Destromon — Digimon, Lv.5, Black, DP 10000, Cost 10.
//! Type/Traits: Unknown / LIBERATOR. Form: Ultimate.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT21-060.md)
//!
//! Digivolution: Black Lv.4 / Cost 5 (standard) · [Digivolve] [Vemmon]: Cost 6.
//!
//! [When Digivolving] Until your opponent's turn ends, their effects can't
//!   trash this Digimon's stacked cards. Then, to 1 of your opponent's Digimon,
//!   <De-Digivolve 1> for every 2 [Vemmon] in this Digimon's digivolution cards.
//! [All Turns] When this Digimon would leave the battle area, you may play 1
//!   [Vemmon] from its digivolution cards without paying the cost.
//!
//! Inherited Effect
//! [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon
//!   attacks, by returning 2 [Vemmon] from this Digimon's digivolution cards to
//!   the bottom of the deck, end that attack.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_060.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - F6 Effect immunity / IMMUNE_FROM_X (ImmuneFromStackTrashing)
//! - De-Digivolve N (count-scaled by floor(Vemmon sources / 2))
//! - F3-adjacent non-cancelling would-leave replacement (play source free)
//! - G4/inherited on-opponent-attack OPT (return 2 sources → end attack)
//! - Alt-digivolve paths (standard + xros_req name-gated)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT21-060";

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A synthetic "Vemmon" digivolution-source card (name matters for the
/// name_is: Vemmon filters on every clause).
fn vemmon() -> CardData {
    let mut c = make_test_card("VEMMON", "Vemmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

/// A non-Vemmon filler source (so the "for every 2 Vemmon" count excludes it).
fn filler_source() -> CardData {
    let mut c = make_test_card("FILLER-SRC", "FillerSource");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

/// An opponent Digimon at `level` with `play_cost`, used to build a stack whose
/// top card can be de-digivolved.
fn opp_stack_card(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = 4;
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-060 YAML parses and compiles")
        .add_card(vemmon())
        .add_card(filler_source())
        .add_card(opp_stack_card("OPP-LV5", 5))
        .add_card(opp_stack_card("OPP-LV4", 4))
        .add_card(opp_stack_card("OPP-LV3", 3))
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .deck(0, &["DECK-PAD"; 20])
        .deck(1, &["DECK-PAD"; 20])
}

/// Drive the [When Digivolving] clause by enqueuing the timing against the
/// carrier permanent (BT21-072 idiom). Returns after draining the queue.
fn fire_when_digivolving(runner: &mut DebugRunner, carrier: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
}

// ═════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_060_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert_eq!(card.name, "Destromon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(10000));
}

#[test]
fn bt21_060_has_when_digivolving_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let wd = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::FaceUp
                && t.when.contains(&CompiledTiming::WhenDigivolving)
    ));
    assert!(wd, "When Digivolving clause present (face-up scope)");
}

#[test]
fn bt21_060_would_leave_is_replacement() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
        )
    });
    assert!(
        has,
        "[All Turns] would-leave play-source is modeled as a Replacement clause"
    );
}

/// The inherited on-opponent-attack clause MUST keep its scope AND its timing
/// at compile — the exact mis-compile the prior implementation attempt hit.
#[test]
fn bt21_060_has_inherited_on_opponent_attack_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnOpponentAttack) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited on_opponent_attack clause survives compile with scope + timing intact");
    assert!(clause.once_per_turn, "[Once Per Turn]");
}

/// There must be exactly ONE inherited clause (no duplicate face-up copy from a
/// scope/when drop) and exactly ONE face-up When Digivolving clause.
#[test]
fn bt21_060_clause_scopes_are_distinct() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let inherited = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited))
        .count();
    let faceup_triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(t) if t.scope == CompiledScope::FaceUp))
        .count();
    assert_eq!(inherited, 1, "exactly one inherited triggered clause");
    assert_eq!(
        faceup_triggered, 1,
        "exactly one face-up triggered clause (When Digivolving) — no duplicate from a scope drop"
    );
}

#[test]
fn bt21_060_has_two_digivolve_alt_paths() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(
        digivolve_paths, 2,
        "standard Black Lv.4 cost-5 + [Digivolve] [Vemmon] cost-6"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Section 2 — [When Digivolving]: stack-trash immunity + scaled De-Digivolve
// ═════════════════════════════════════════════════════════════════════════════

/// The ImmuneFromStackTrashing modifier is granted to Destromon on WD.
#[test]
fn bt21_060_when_digivolving_grants_stack_trash_immunity() {
    let mut runner = base().memory(10).start();
    // Destromon over 2 Vemmon sources.
    let carrier = runner.place_stack(0, &["VEMMON", "VEMMON", CARD_ID]);
    // No opponent Digimon → the De-Digivolve select is skipped, but the
    // immunity (step 1) still applies.
    fire_when_digivolving(&mut runner, carrier);

    assert!(
        runner
            .modifiers()
            .has(carrier, ModifierType::ImmuneFromStackTrashing),
        "When Digivolving grants ImmuneFromStackTrashing to this Digimon"
    );
}

/// With an opponent Digimon present, the mandatory De-Digivolve target prompt
/// installs (an opp Digimon exists to receive it).
#[test]
fn bt21_060_when_digivolving_installs_de_digivolve_prompt() {
    let mut runner = base().memory(10).start();
    let carrier = runner.place_stack(0, &["VEMMON", "VEMMON", CARD_ID]);
    runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);

    fire_when_digivolving(&mut runner, carrier);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "mandatory De-Digivolve target select installs when an opp Digimon exists"
    );
    assert!(
        !runner.pending_is_optional(),
        "the De-Digivolve target pick is mandatory (DCGO canNoSelect: false)"
    );
}

/// No opponent Digimon → no De-Digivolve prompt installs (nothing to target),
/// but immunity still applied (asserted above).
#[test]
fn bt21_060_when_digivolving_no_prompt_without_opponent_digimon() {
    let mut runner = base().memory(10).start();
    let carrier = runner.place_stack(0, &["VEMMON", "VEMMON", CARD_ID]);
    // Opponent field is empty.
    fire_when_digivolving(&mut runner, carrier);

    assert!(
        runner.pending_selection().is_none(),
        "no De-Digivolve prompt when the opponent controls no Digimon"
    );
}

/// 4 Vemmon sources ⇒ floor(4/2) = 2 De-Digivolve on the chosen opp Digimon:
/// its 3-high stack (Lv3/Lv4/Lv5) loses its top 2 sources.
#[test]
fn bt21_060_de_digivolve_count_scales_two_per_two_vemmon() {
    let mut runner = base().memory(10).start();
    // 4 Vemmon beneath Destromon → floor(4/2) = 2 De-Digivolve.
    let carrier = runner.place_stack(0, &["VEMMON", "VEMMON", "VEMMON", "VEMMON", CARD_ID]);
    let opp = runner.place_stack(1, &["OPP-LV3", "OPP-LV4", "OPP-LV5"]);
    let sources_before = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();

    fire_when_digivolving(&mut runner, carrier);
    // Drive the mandatory target pick, then resolve the De-Digivolve.
    let view = runner
        .pending_selection_view()
        .expect("De-Digivolve target prompt");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the opponent Digimon");
    runner.auto_resolve().ok();

    let sources_after = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after,
        sources_before - 2,
        "floor(4 Vemmon / 2) = 2 De-Digivolve trashes the top 2 sources of the target"
    );
}

/// 2 Vemmon sources ⇒ floor(2/2) = 1 De-Digivolve.
#[test]
fn bt21_060_de_digivolve_count_is_one_with_two_vemmon() {
    let mut runner = base().memory(10).start();
    let carrier = runner.place_stack(0, &["VEMMON", "VEMMON", CARD_ID]);
    let opp = runner.place_stack(1, &["OPP-LV3", "OPP-LV4", "OPP-LV5"]);
    let sources_before = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();

    fire_when_digivolving(&mut runner, carrier);
    let view = runner
        .pending_selection_view()
        .expect("De-Digivolve target prompt");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the opponent Digimon");
    runner.auto_resolve().ok();

    let sources_after = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after,
        sources_before - 1,
        "floor(2 Vemmon / 2) = 1 De-Digivolve trashes exactly the top source"
    );
}

/// Only Vemmon sources count: 1 Vemmon + 1 non-Vemmon ⇒ floor(1/2) = 0, so a
/// target with a de-digivolvable stack is left untouched (no De-Digivolve
/// prompt installs because the scaled count is zero — but the target select
/// still installs; the De-Digivolve resolves to 0). Assert the opponent stack
/// is unchanged.
#[test]
fn bt21_060_only_vemmon_sources_count_toward_the_scale() {
    let mut runner = base().memory(10).start();
    // 1 Vemmon + 1 non-Vemmon beneath Destromon → floor(1/2) = 0.
    let carrier = runner.place_stack(0, &["FILLER-SRC", "VEMMON", CARD_ID]);
    let opp = runner.place_stack(1, &["OPP-LV3", "OPP-LV4", "OPP-LV5"]);
    let sources_before = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();

    fire_when_digivolving(&mut runner, carrier);
    // The mandatory target select still installs (an opp Digimon exists).
    if let Some(view) = runner.pending_selection_view() {
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("pick the opponent Digimon");
        runner.auto_resolve().ok();
    }

    let sources_after = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after, sources_before,
        "floor(1 Vemmon / 2) = 0 De-Digivolve: only Vemmon sources are counted"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns] would-leave: may play 1 [Vemmon] source free
// ═════════════════════════════════════════════════════════════════════════════

/// When Destromon would leave, the outer optional Replacement accept/decline
/// prompt installs first (per the replacement selection state machine).
#[test]
fn bt21_060_would_leave_installs_optional_replacement_prompt() {
    let mut runner = base().memory(10).start();
    let carrier = runner.place_stack(0, &["VEMMON", CARD_ID]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("would-leave offers an accept/decline replacement prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(
        runner.pending_is_optional(),
        "'you may play' → the replacement is declinable"
    );
}

/// Accept path: the Vemmon source is played to the battle area for free, and
/// Destromon STILL leaves (non-cancelling side-effect).
#[test]
fn bt21_060_would_leave_accept_plays_vemmon_and_still_leaves() {
    let mut runner = base().memory(10).start();
    let carrier = runner.place_stack(0, &["VEMMON", CARD_ID]);
    let field_before = runner.battle_area_size(0);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    // Accept the outer replacement.
    let view = runner
        .pending_selection_view()
        .expect("replacement accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept the would-leave play");

    // Inner: choose the Vemmon source to play (no auto-select — every pick is
    // surfaced). Then resolve.
    let inner = runner
        .pending_selection_view()
        .expect("Vemmon source select installs after accepting");
    runner
        .execute_action(0, inner.valid_action_ids[0])
        .expect("pick the Vemmon source to play");
    runner.auto_resolve().ok();

    // Destromon (BT21-060) has left the battle area…
    let destromon_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        !destromon_present,
        "the would-leave play does NOT cancel the leave — Destromon still leaves"
    );
    // …and a Vemmon is now a standing permanent in the battle area.
    let vemmon_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "VEMMON");
    assert!(
        vemmon_on_field,
        "the [Vemmon] source was played to the battle area for free"
    );
}

/// Decline path: PASS at the replacement prompt plays nothing; Destromon leaves.
#[test]
fn bt21_060_would_leave_decline_plays_nothing() {
    let mut runner = base().memory(10).start();
    let carrier = runner.place_stack(0, &["VEMMON", CARD_ID]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("replacement prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, PASS)
        .expect("decline the would-leave play");
    runner.auto_resolve().ok();

    let vemmon_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "VEMMON");
    assert!(
        !vemmon_on_field,
        "declining plays no Vemmon"
    );
}

/// No Vemmon among the sources → the would-leave play is never offered.
#[test]
fn bt21_060_would_leave_not_offered_without_vemmon_source() {
    let mut runner = base().memory(10).start();
    // Destromon over a non-Vemmon source only.
    let carrier = runner.place_stack(0, &["FILLER-SRC", CARD_ID]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "no [Vemmon] source ⇒ the would-leave play is never offered"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [Opponent's Turn][OPT]: return 2 Vemmon → end attack
// ═════════════════════════════════════════════════════════════════════════════

/// Fixture: BT21-060 sitting as a digivolution source under a top Digimon on
/// PLAYER 0's board, with N extra Vemmon sources beneath it. Player 1 (opponent)
/// gets an attacker. After `end_turn()` it is player 1's turn, so player 1's
/// attack drives the inherited [Opponent's Turn] clause through the real attack
/// flow (the dispatch path that scans player 0's below-top inherited sources —
/// `enqueue_triggered` on the attacker would NOT reach the defender's inherited
/// effects). Returns (runner, defending host [P0], attacker [P1]).
fn inherited_setup(vemmon_sources: usize) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut stack: Vec<&str> = Vec::new();
    for _ in 0..vemmon_sources {
        stack.push("VEMMON");
    }
    // BT21-060 as an inherited source, then a top Digimon over it.
    stack.push(CARD_ID);
    stack.push("OPP-LV5"); // a generic Lv5 shell as the evolved top card
    let mut runner = base()
        .memory(10)
        // Player 0 needs security so a player-attack has a check to end.
        .security(0, &["DECK-PAD", "DECK-PAD"])
        .start();
    runner.skip_mulligan();
    runner.set_first_player(0);
    let host = runner.place_stack(0, &stack);
    // Opponent's attacker on field (turn_played 0 so it can attack once it's
    // their turn; no summoning sickness).
    let attacker = runner.place_field_stack(1, &["OPP-LV4"], false, 0);
    // Advance to player 1's (opponent's) turn.
    runner.end_turn();
    assert_ne!(
        runner.turn_player(),
        0,
        "precondition: it must be the opponent's (player 1's) turn"
    );
    (runner, host, attacker)
}

#[test]
fn bt21_060_inherited_fires_on_opponent_attack_with_two_vemmon() {
    let (mut runner, _host, attacker) = inherited_setup(2);

    runner.attack_player(attacker, 0, false);

    // The inherited clause installs its Vemmon source-return selection.
    assert!(
        runner.pending_selection().is_some(),
        "inherited clause installs a Vemmon source-return selection on opp attack"
    );
}

/// Fewer than 2 Vemmon sources → the inherited clause never fires (DCGO
/// CanActivateCondition: DigivolutionCards.Count(Vemmon) >= 2).
#[test]
fn bt21_060_inherited_does_not_fire_with_one_vemmon() {
    let (mut runner, _host, attacker) = inherited_setup(1);

    runner.attack_player(attacker, 0, false);

    assert!(
        runner.pending_selection().is_none(),
        "with only 1 Vemmon source the inherited return-2 clause cannot fire"
    );
}

/// Full resolution: returning 2 Vemmon ends the attack. Both [Vemmon] leave the
/// digivolution stack and land in the deck (routed to deck, NOT trash), and the
/// security stack is untouched (the attack ended before the security check).
///
/// We assert the [Vemmon] *location* (deck vs battle-area sources vs trash)
/// rather than a raw deck-size delta — the latter couples to unrelated
/// turn/draw churn from the attack flow.
#[test]
fn bt21_060_inherited_returns_two_vemmon_to_deck_and_ends_attack() {
    let (mut runner, _host, attacker) = inherited_setup(2);
    let trash_before = runner.trash_size(0);
    let security_before = runner.security_count(0);

    let vemmon_in_deck = |r: &DebugRunner| {
        r.game.players[0]
            .deck
            .iter()
            .filter(|c| c.card_id(&r.game.card_data) == "VEMMON")
            .count()
    };
    let vemmon_in_sources = |r: &DebugRunner| {
        r.game.players[0]
            .battle_area
            .iter()
            .map(|p| {
                p.card_sources
                    .iter()
                    .filter(|c| c.card_id(&r.game.card_data) == "VEMMON")
                    .count()
            })
            .sum::<usize>()
    };
    assert_eq!(vemmon_in_deck(&runner), 0, "no Vemmon in the deck before");
    assert_eq!(vemmon_in_sources(&runner), 2, "2 Vemmon sources before");

    runner.attack_player(attacker, 0, false);
    // Drive both mandatory source picks + follow-ups.
    runner.auto_resolve().ok();

    assert_eq!(
        vemmon_in_deck(&runner),
        2,
        "both [Vemmon] sources were returned to the deck"
    );
    assert_eq!(
        vemmon_in_sources(&runner),
        0,
        "no [Vemmon] remains among the digivolution sources"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "the returned Vemmon go to the deck, NOT the trash"
    );
    assert_eq!(
        runner.security_count(0),
        security_before,
        "the attack ended before any security check (no security lost)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT enforcement (inherited clause)
// ═════════════════════════════════════════════════════════════════════════════

/// The inherited clause is [Once Per Turn]: after it fires and resolves once on
/// the opponent's turn, a second opponent attack the same turn does not re-arm
/// the return-2 selection.
#[test]
fn bt21_060_inherited_opt_locks_second_attack_same_turn() {
    // 4 Vemmon → still ≥2 after the first use consumes 2.
    let (mut runner, _host, attacker) = inherited_setup(4);

    runner.attack_player(attacker, 0, false);
    runner.auto_resolve().ok();
    assert!(
        runner.pending_selection().is_none(),
        "first activation resolved"
    );

    // Second opponent attack, same turn (the attacker unsuspends via a fresh
    // attack declaration in the test harness).
    runner.attack_player(attacker, 0, false);

    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] locks the inherited clause for the rest of the opponent's turn"
    );
}

