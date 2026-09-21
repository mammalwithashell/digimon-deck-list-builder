//! BT25-028 Dianamon — Digimon, Lv.6, Blue/White, DP 12000, Cost 12.
//! Traits: Shaman, Olympos XII, Iliad, TS. Attribute: Vaccine.
//!
//! # Card text (card image BT25-028 — authoritative for printed text)
//! When this card would be played, if your opponent has a level 6 or higher
//! Digimon, reduce the cost by 5.
//! [On Play] [When Digivolving] None of your opponent's Digimon with 1 or fewer
//! digivolution cards can suspend until their turn ends. Then, delete 1 of your
//! opponent's unsuspended Digimon.
//! [All Turns] [Once Per Turn] When any Digimon are played or digivolve, you may
//! trash any 4 digivolution cards from your opponent's Digimon. Then, 2 of your
//! Digimon may DNA digivolve into [GraceNovamon] in the hand.
//! Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon or
//! Tamers can't suspend until their turn ends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_028.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API 4.3)
//! - D2 conditional cost reduction; F7 cannot-suspend; mandatory delete
//! - G2 DNA digivolve into a hand card; inherited WA cannot-suspend (E2 OPT)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-028";

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-028 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("OPP-NOSRC", 5, 6000, &["Beast"]))
        .add_card(make_digimon("OPP-SRC", 4, 4000, &["Beast"]))
        .add_card(make_digimon("OPP-L6", 6, 11000, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

#[test]
fn bt25_028_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Dianamon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for t in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_028_has_cost_reduction() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    // "When THIS card would be played" — a self play-cost reducer (DCGO
    // `MandatorySelfPlayCostReduction`). `when_playing_this` is what lets the
    // engine collect it from the hand card being played; without it the
    // reduction never applied (DCGO exam BT25-028-effect1, 2026-09-18).
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                when_playing_this: true,
                amount: Some(5),
                ..
            })
        )),
        "self play-cost reduction clause present"
    );
}

/// Hard-playing Dianamon with an opponent Lv.6+ Digimon on the board pays
/// 12 - 5 = 7. Mandatory, no prompt (DCGO `MandatorySelfPlayCostReduction`).
#[test]
fn bt25_028_hard_play_pays_7_when_opponent_has_level_6() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(1, "OPP-L6", Some(0));
    runner.game.enter_main_phase();
    let mem_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(played.is_some(), "the play resolves with no reducer prompt");
    // The [On Play] body then asks its mandatory delete pick (OPP-L6 is
    // unsuspended) — resolve it so the memory read is post-play.
    let _ = runner.auto_resolve();
    assert_eq!(mem_before - runner.memory(), 7, "12 reduced by 5");
}

/// Without a Lv.6+ opponent Digimon the full 12 is paid.
#[test]
fn bt25_028_hard_play_pays_12_without_level_6_opponent() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(1, "OPP-NOSRC", Some(0));
    runner.game.enter_main_phase();
    let mem_before = runner.memory();
    assert_eq!(mem_before, 10);
    let played = runner.play(0, 0);
    assert!(played.is_some());
    let _ = runner.auto_resolve();
    // 10 - 12 = -2 crosses zero: the turn passes and the gauge is then read
    // from P1's side (+2). Un-flip it before subtracting.
    assert_eq!(runner.game.turn_player(), 1, "paying 12 from 10 ends the turn");
    let p0_memory = -runner.memory();
    assert_eq!(mem_before - p0_memory, 12, "no reduction: full 12");
}

#[test]
fn bt25_028_op_wd_clause_deletes_and_is_mandatory() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.process
                        .iter()
                        .any(|s| matches!(s, CompiledStep::DeletePermanent { .. })) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OP/WD delete clause present");
    assert!(!clause.optional, "the delete is mandatory (no 'may')");
}

#[test]
fn bt25_028_has_inherited_wa_cannot_suspend() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited WA clause present");
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddModifier { .. })),
        "inherited WA applies a CannotSuspend modifier"
    );
}

#[test]
fn bt25_028_has_all_turns_dna_observer() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if (t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    || t.when.contains(&CompiledTiming::OnDigivolve))
                    && t.optional
                    && t.once_per_turn =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[All Turns] DNA observer present");
    assert!(clause.optional && clause.once_per_turn);
}

// NOTE: cost-reduction is verified structurally by `bt25_028_has_cost_reduction`.
// The debug runner's `play()` path does not route the printed play cost through
// the `BeforePayCost` cost-reduction scan (cf. BT25-018, whose test also only
// asserts the cost_reduction clause structurally), so a memory-delta behavioral
// check is not reliable here.

#[test]
fn bt25_028_locks_low_source_opp_and_offers_delete() {
    let mut runner = base().memory(12).start();
    let nosrc = runner.place_on_field(1, "OPP-NOSRC", Some(0));
    let diana = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, diana.index as usize);

    assert!(
        runner.modifiers().has(nosrc, ModifierType::CannotSuspend),
        "opp Digimon with <=1 sources gains CannotSuspend"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "delete-1-unsuspended-opp prompt installs"
    );
}

#[test]
fn bt25_028_op_wd_no_delete_prompt_without_opp() {
    let mut runner = base().memory(12).start();
    let diana = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, diana.index as usize);
    assert!(
        runner.pending_selection().is_none(),
        "no opp Digimon -> OP/WD clause is a no-op"
    );
}

/// `[All Turns] [Once Per Turn] When any Digimon are played or digivolve, you
/// may trash any 4 digivolution cards from your opponent's Digimon.`
///
/// The DCGO exam measured this half as a no-op on our side:
/// `qa/dcgo-exams/BT25/BT25-028-effect3.yaml` (oracle run 2026-09-21, sidecar
/// `20260921T045227Z_15e1e879de…`) diffed
/// `p1.trash ours=[] dcgo=[ST2-01, ST2-02]` /
/// `p1.field[0].sources ours=[ST2-01, ST2-02] dcgo=[]` — DCGO trashes the
/// opponent's digivolution cards, we never even offer the pick. This test pins
/// the prompt: after the `optional: true` gate is accepted, a `SourceMulti`
/// selection over the OPPONENT's below-top sources must park
/// (`min 0 / max 4`, DCGO `SelectTrashDigivolutionCards(maxCount: 4,
/// canNoTrash: true)` — `BT25_028.cs:145-160`).
#[test]
fn bt25_028_all_turns_offers_opponent_source_pick() {
    let mut runner = base().memory(12).start();
    // Opponent carrier with TWO digivolution cards below its top card.
    runner.place_stack(1, &["PAD", "PAD", "OPP-SRC"]);
    let diana = runner.place_on_field(0, CARD_ID, Some(0));
    // Any Digimon entering the field is the trigger; use p0's own second play
    // so the observer's controller is unambiguous.
    let newcomer = runner.place_on_field(0, "OPP-NOSRC", Some(0));
    runner.fire_play_event_triggers(0, newcomer.index as usize, false, false);

    // The clause is `optional: true`, so our engine parks a Replacement gate
    // first. Accept it.
    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        let view = runner
            .pending_selection_view()
            .expect("gate view")
            .valid_action_ids[0];
        runner.execute_action(0, view).expect("accept the [All Turns] gate");
    }

    let view = runner
        .pending_selection_view()
        .expect("a selection must park after the gate is accepted");
    assert!(
        matches!(view.kind, SelectionKind::SourceMulti { .. }),
        "expected the opponent-source trash pick, got {:?}",
        view.kind
    );
}

/// Same clause, the shape the DCGO exam actually measured: the trigger is the
/// OPPONENT's Digimon entering the field (`[All Turns]` — "when ANY Digimon are
/// played or digivolve"), not p0's own play.
#[test]
fn bt25_028_all_turns_offers_opponent_source_pick_on_opponent_entry() {
    let mut runner = base().memory(12).start();
    runner.place_stack(1, &["PAD", "PAD", "OPP-SRC"]);
    let diana = runner.place_on_field(0, CARD_ID, Some(0));
    let newcomer = runner.place_on_field(1, "OPP-NOSRC", Some(0));
    runner.fire_play_event_triggers(1, newcomer.index as usize, false, false);

    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        let accept = runner
            .pending_selection_view()
            .expect("gate view")
            .valid_action_ids[0];
        runner
            .execute_action(0, accept)
            .expect("accept the [All Turns] gate");
    }

    let view = runner
        .pending_selection_view()
        .expect("a selection must park after the gate is accepted");
    assert!(
        matches!(view.kind, SelectionKind::SourceMulti { .. }),
        "expected the opponent-source trash pick, got {:?}",
        view.kind
    );
}

/// The exam's exact trigger: the opponent DIGIVOLVES (their stack grows to two
/// below-top sources as part of the same action) rather than playing. This is
/// the shape `BT25-028-effect3.yaml` drives on DCGO.
#[test]
fn bt25_028_all_turns_offers_opponent_source_pick_on_opponent_digivolve() {
    use digimon_engine::card_source::CardSource;
    use digimon_engine::enums::PlaySource;

    let mut opp_lv5 = make_digimon("OPP-LV5", 5, 7000, &["Beast"]);
    opp_lv5.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 4,
        memory_cost: 1,
    }];

    let mut runner = base()
        .add_card(opp_lv5)
        .memory(12)
        .start();
    runner.game.turn_count = 2;
    // The exam's line runs this trigger on the OPPONENT's turn.
    runner.game.turn_player_idx = 1;
    // p1 base: one below-top source already (the egg), top = OPP-SRC (Lv.4).
    let base_perm = runner.place_stack(1, &["PAD", "OPP-SRC"]);
    let _diana = runner.place_on_field(0, CARD_ID, Some(0));

    // p1 digivolves OPP-LV5 onto it -> two below-top sources.
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OPP-LV5")
        .expect("OPP-LV5 registered");
    let instance = runner.game.next_card_index();
    runner.game.players[1]
        .hand
        .push(CardSource::new(data_index, 1, instance));
    let hand_index = runner.game.players[1].hand.len() - 1;
    let ok = runner.game.digivolve_from_hand(
        1,
        hand_index,
        base_perm.index as usize,
        PlaySource::ByHand,
    );
    assert!(ok, "p1 digivolves onto its Lv.4");

    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        let accept = runner
            .pending_selection_view()
            .expect("gate view")
            .valid_action_ids[0];
        runner
            .execute_action(0, accept)
            .expect("accept the [All Turns] gate");
    }

    let view = runner
        .pending_selection_view()
        .expect("a selection must park after the gate is accepted");
    assert!(
        matches!(view.kind, SelectionKind::SourceMulti { .. }),
        "expected the opponent-source trash pick, got {:?}",
        view.kind
    );
}
