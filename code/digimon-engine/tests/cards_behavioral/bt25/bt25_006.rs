//! BT25-006 Dorimon — Digimon, Lv.2, Purple, Cost 0.
//! Traits: Lesser, X Antibody, Titan, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! Inherited Effect [Opponent's Turn] [Once Per Turn] When one of your
//! opponent's Digimon attacks, by trashing 1 card in your hand, 1 of your
//! [Titan] trait Digimon unsuspends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Purple/BT25_006.cs
//!
//! OnAllyAttack inherited ActivateClass, OPT, isOptional: true, gated by
//! opponent-Digimon-attacks + IsOpponentTurn; CanActivate requires
//! HandCards >= 1. Body: SelectHand Discard (cost) → SelectPermanent UnTap on
//! own [Titan] Digimon (mandatory).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - B (inherited triggered clause, opponent-turn gated, optional, OPT)
//! - cost-firing clause (trash 1 hand card) with event-log assertion
//! - C (mandatory own-permanent select → unsuspend)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-006";

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A Lv.4 Titan Digimon we can place on P0's field as the unsuspend target.
fn titan_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = vec!["Titan".to_string(), "TS".to_string()];
    card
}

/// A non-Titan Digimon — never a legal unsuspend target.
fn plain_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = vec!["Beast".to_string()];
    card
}

/// An opponent attacker.
fn opp_attacker(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.traits = vec!["Beast".to_string()];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-006 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(titan_digimon("TITAN-A"))
        .add_card(titan_digimon("TITAN-B"))
        .add_card(plain_digimon("PLAIN"))
        .add_card(opp_attacker("OPP-ATK"))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
}

/// Place a carrier Digimon on P0's field with Dorimon as a digivolution source
/// underneath (so Dorimon's *inherited* effect is live). Returns the carrier
/// handle and the host carrier built from `TITAN-A` (so the carrier itself is
/// a legal unsuspend target).
fn carrier_with_dorimon_source(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_stack(0, &[CARD_ID, "TITAN-A"])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_006_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Dorimon");
    assert_eq!(card.level, Some(2));
    assert_eq!(card.cost, Some(0));
    for t in ["Lesser", "X Antibody", "Titan", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "missing trait {t}");
    }
}

#[test]
fn bt25_006_inherited_clause_is_optional_opt_and_opp_turn_gated() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnOpponentAttack) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have an OnOpponentAttack triggered clause");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "the whole card is an inherited effect"
    );
    assert!(clause.optional, "DCGO isOptional: true → optional");
    assert!(clause.once_per_turn, "[Once Per Turn] → once_per_turn");
    let active = clause
        .active_when
        .as_ref()
        .expect("must be opponents_turn gated");
    assert!(
        active.opponents_turn == Some(true)
            || active.all_of.iter().any(|p| p.opponents_turn == Some(true)),
        "[Opponent's Turn] must compile to an opponents_turn gate"
    );
}

#[test]
fn bt25_006_clause_trashes_hand_then_unsuspends() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnOpponentAttack) => {
                Some(t)
            }
            _ => None,
        })
        .expect("clause present");

    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::TrashFromHandByIndex { .. })),
        "must trash a hand card (the printed cost)"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::Unsuspend { .. })),
        "must unsuspend a Titan Digimon"
    );
}

// ─── Section 2 — Behavior ────────────────────────────────────────────────────

/// Positive: opponent's turn, opponent Digimon attacks, you have a hand card
/// and a suspended Titan → accepting the trigger trashes 1 hand card and
/// unsuspends the chosen Titan.
#[test]
fn bt25_006_opp_attack_trash_to_unsuspend_titan() {
    let mut runner = base().hand(0, &["PAD"]).start();
    let carrier = carrier_with_dorimon_source(&mut runner);
    // Suspend the carrier (the Titan we will unsuspend).
    runner.game.suspend(carrier);
    assert!(
        runner.game.players[0].battle_area[carrier.index as usize].is_suspended,
        "precondition: Titan carrier is suspended"
    );

    let opp = runner.place_on_field(1, "OPP-ATK", Some(0));

    // Opponent's turn.
    runner.game.turn_player_idx = 1;
    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);

    // Opponent attacks P0's security → fires "opponent Digimon attacks".
    runner.attack_player(opp, 0, false);

    assert!(
        runner.pending_selection().is_some(),
        "inherited trigger must install an optional accept/decline prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the optional trigger");
    runner.auto_resolve().expect("resolve trash + unsuspend");

    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "1 hand card trashed as the cost"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the trashed hand card is now in trash"
    );
    assert!(
        !runner.game.players[0].battle_area[carrier.index as usize].is_suspended,
        "the chosen Titan must be unsuspended"
    );
}

/// Negative: empty hand → the cost cannot be paid, clause does not fire.
#[test]
fn bt25_006_no_fire_with_empty_hand() {
    let mut runner = base().start();
    let carrier = carrier_with_dorimon_source(&mut runner);
    runner.game.suspend(carrier);
    let opp = runner.place_on_field(1, "OPP-ATK", Some(0));

    runner.game.turn_player_idx = 1;
    assert_eq!(runner.hand_size(0), 0, "precondition: empty hand");

    runner.attack_player(opp, 0, false);
    assert!(
        runner.pending_selection().is_none(),
        "no hand card → cost unpayable → clause must not fire"
    );
}

/// Negative: it is YOUR turn (you attack) → [Opponent's Turn] gate blocks it.
#[test]
fn bt25_006_no_fire_on_your_own_turn() {
    let mut runner = base().hand(0, &["PAD"]).start();
    let carrier = carrier_with_dorimon_source(&mut runner);
    runner.game.suspend(carrier);
    // P0 attacker on its own turn.
    let p0_atk = runner.place_on_field(0, "TITAN-B", Some(0));
    let opp = runner.place_on_field(1, "OPP-ATK", Some(0));

    runner.game.turn_player_idx = 0; // your turn
    runner.attack_digimon(p0_atk, opp, false);

    assert!(
        runner.pending_selection().is_none(),
        "[Opponent's Turn] gate → must not fire when you attack on your turn"
    );
}

/// Optional decline keeps the hand and leaves the Titan suspended.
#[test]
fn bt25_006_decline_keeps_hand_and_suspension() {
    let mut runner = base().hand(0, &["PAD"]).start();
    let carrier = carrier_with_dorimon_source(&mut runner);
    runner.game.suspend(carrier);
    let opp = runner.place_on_field(1, "OPP-ATK", Some(0));

    runner.game.turn_player_idx = 1;
    let hand_before = runner.hand_size(0);

    runner.attack_player(opp, 0, false);
    assert!(runner.pending_selection().is_some(), "prompt installed");
    runner
        .decline_optional_trigger()
        .expect("declining the optional trigger is legal");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "no card trashed on decline"
    );
    assert!(
        runner.game.players[0].battle_area[carrier.index as usize].is_suspended,
        "declining leaves the Titan suspended"
    );
}
