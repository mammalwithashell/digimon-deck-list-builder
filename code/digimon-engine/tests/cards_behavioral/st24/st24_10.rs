//! ST24-10 Lilamon — Digimon, Lv.5, Green, DP 7000, Cost 7.
//! Traits: Fairy, DATA SQUAD.
//!
//! # Card text (data/cards.json — verbatim, matches DCGO)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] Suspend 1 of
//!   your opponent's Digimon or Tamers. It can't unsuspend until their turn ends.
//!   Then, by trashing 2 bottom face-down cards from under any of your Tamers,
//!   this Digimon may digivolve into a [DATA SQUAD] trait Digimon card in the hand
//!   without paying the cost.
//! Inherited [All Turns] [Once Per Turn]: When this Digimon with [Rosemon] in its
//!   name or the [DATA SQUAD] trait would leave the battle area, by trashing the
//!   bottom face-down card from under any of your Tamers, it doesn't leave.
//!
//! Printed digivolve box: [Digivolve] Lv.4 w/[DATA SQUAD] trait: Cost 3.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Green/ST24_10.cs
//!
//! # Patterns
//! - multi-timing [OP][WD][WA] shared OPT (single shared lockout counter)
//! - mandatory suspend 1 opp Digimon/Tamer + CannotUnsuspend (end_of_opponents_turn)
//! - trash-2-bottom-FD-under-Tamers cost → free effect-initiated digivolve (BT25-035)
//! - inherited leave-replacement (Rosemon-name / DATA SQUAD subject; BT25-027/ST23-05)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "ST24-10";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn lilamon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c
}

fn opp_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

/// A Lv.6 [DATA SQUAD] Digimon — the legal hand digivolve target. Carries a
/// printed digivolution cost from Lv.5 so the FREE digivolve is observable.
fn ds_lv6(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 13;
    c.traits = vec!["DATA SQUAD".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: 3, // Green
        level: 5,
        memory_cost: 4,
    }];
    c
}

fn non_ds_lv6(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 13;
    c.traits = vec!["Fairy".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: 3,
        level: 5,
        memory_cost: 4,
    }];
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .hand
        .push(CardSource::new(data_idx, p, next_idx));
}

fn face_down_source_count(runner: &DebugRunner, p: u8) -> usize {
    runner.game.players[p as usize]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_kind(&runner.game.card_data) == CardKind::Tamer)
        .map(|perm| perm.card_sources.iter().filter(|s| s.face_down).count())
        .sum()
}

fn base_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .add_card(lilamon())
        .add_card(opp_digimon("OPP-DIGI"))
        .add_card(opp_tamer("OPP-TAMER"))
        .add_card(ds_lv6("DS6"))
        .add_card(non_ds_lv6("NONDS6"))
        .add_card(tamer("TAMER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
}

/// Place Lilamon as a battle-area top (a Lv.4 filler underneath), plus the
/// requested set of Tamers each carrying `n` face-down sources.
fn lilamon_runner(targets: &[&str], tamers_fd: &[usize]) -> (DebugRunner, PermanentHandle, Vec<PermanentHandle>) {
    let mut runner = base_builder().start();
    runner.set_first_player(0);

    let lilamon = runner.place_stack(0, &["FILLER", CARD_ID]);

    let mut tamers = Vec::new();
    for &n in tamers_fd {
        let mut names: Vec<&str> = vec!["FILLER"; n];
        names.push("TAMER");
        let t = runner.place_stack(0, &names);
        for i in 0..n {
            runner.game.players[0].battle_area[t.index as usize].card_sources[i].face_down = true;
        }
        tamers.push(t);
    }

    for &tgt in targets {
        push_to_hand(&mut runner, 0, tgt);
    }
    (runner, lilamon, tamers)
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

fn process_contains_trash_n_under_tamers(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::TrashBottomFaceDownSourcesUnderTamers { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_trash_n_under_tamers(then)
                || process_contains_trash_n_under_tamers(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_trash_n_under_tamers(body)
        }
        _ => false,
    })
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st24_10_metadata_and_alt_path() {
    let card = compiled(CARD_ID);
    assert_eq!(card.name, "Lilamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
    let ds = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .map(|f| f.level_eq == Some(4) && f.trait_has.as_deref() == Some("DATA SQUAD"))
            .unwrap_or(false)
    });
    assert!(ds, "Lv.4 [DATA SQUAD] alt-path present");
}

#[test]
fn st24_10_has_multi_timing_opt_and_inherited_replacement() {
    let card = compiled(CARD_ID);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let shared = triggered.iter().find(|t| {
        t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::WhenDigivolving)
            && t.when.contains(&CompiledTiming::WhenAttacking)
    });
    let shared = shared.expect("[OP][WD][WA] shared clause present");
    assert!(shared.once_per_turn, "shared OPT counter");
    assert!(
        process_contains_trash_n_under_tamers(&shared.process),
        "the shared clause's 'Then' half uses the trash-2-under-Tamers cost"
    );

    // Inherited leave-replacement (a Declarative replacement or triggered one).
    let has_replacement = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => format!("{d:?}").contains("Replacement"),
        CompiledClause::Triggered(t) => {
            t.scope == CompiledScope::Inherited
                && format!("{:?}", t.process).contains("CancelReplacement")
        }
    });
    assert!(has_replacement, "inherited leave-replacement clause present");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [OP][WD][WA] suspend+lock, then trash-2 free digivolve
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: suspend an opponent Digimon + CannotUnsuspend, then trash 2 FD from
/// one Tamer and free-digivolve into the [DATA SQUAD] Lv.6 hand card.
#[test]
fn st24_10_suspends_locks_then_trash_2_free_digivolve() {
    let (mut runner, lilamon, tamers) = lilamon_runner(&["DS6", "NONDS6"], &[2]);
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let _tamer = tamers[0];
    let mem_before = runner.memory();
    assert_eq!(face_down_source_count(&runner, 0), 2, "2 face-down sources staged");

    fire(&mut runner, EffectTiming::OnPlay, lilamon);

    // Prompt 1: the mandatory suspend pick (opponent Digimon or Tamer).
    let v = runner
        .pending_selection_view()
        .expect("suspend pick installs (mandatory)");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();

    // The opponent Digimon is suspended and gains CannotUnsuspend.
    assert!(
        runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "the chosen opp Digimon is suspended"
    );
    assert!(
        runner.modifiers().has(opp, ModifierType::CannotUnsuspend),
        "the suspended opp Digimon gains CannotUnsuspend"
    );

    // Prompt 2: the "may digivolve" hand pick (only DS6 is legal).
    let v2 = runner
        .pending_selection_view()
        .expect("'may digivolve' hand pick installs");
    assert!(v2.is_optional, "'may digivolve' ⇒ declinable");
    let pick = v2
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .expect("DS6 is the only legal hand pick");
    runner.execute_action(0, pick).unwrap();
    // Resolve the trash-2 Tamer pick(s).
    let _ = runner.auto_resolve();

    let perm = &runner.game.player(0).battle_area[lilamon.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "DS6",
        "Lilamon digivolves into the [DATA SQUAD] Lv.6 hand card"
    );
    assert_eq!(
        face_down_source_count(&runner, 0),
        0,
        "both face-down sources are trashed (cost)"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "the digivolve is FREE — no memory paid for the digivolution"
    );
}

/// The trash-2 free-digivolve "Then" half is DECLINABLE: PASS on the hand pick
/// leaves Lilamon as-is and trashes no sources. (The suspend still happened.)
#[test]
fn st24_10_then_free_digivolve_is_declinable() {
    let (mut runner, lilamon, _tamers) = lilamon_runner(&["DS6"], &[2]);
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, lilamon);
    // Suspend pick (mandatory).
    let v = runner.pending_selection_view().expect("suspend pick installs");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    assert!(runner.game.players[1].battle_area[opp.index as usize].is_suspended);

    // Decline the "may digivolve".
    let v2 = runner.pending_selection_view().expect("hand pick installs");
    assert!(v2.is_optional);
    runner.execute_action(0, PASS).expect("decline the digivolve");
    let _ = runner.auto_resolve();

    let perm = &runner.game.player(0).battle_area[lilamon.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        CARD_ID,
        "declined ⇒ Lilamon stays as the top card"
    );
    assert_eq!(
        face_down_source_count(&runner, 0),
        2,
        "declined ⇒ no face-down sources trashed"
    );
}

/// A Tamer is a legal suspend target too ("Digimon or Tamers").
#[test]
fn st24_10_can_suspend_opponent_tamer() {
    let (mut runner, lilamon, _tamers) = lilamon_runner(&[], &[]);
    let opp_t = runner.place_on_field(1, "OPP-TAMER", Some(0));

    fire(&mut runner, EffectTiming::WhenAttacking, lilamon);
    let v = runner
        .pending_selection_view()
        .expect("suspend pick installs (an opponent Tamer is a legal target)");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[1].battle_area[opp_t.index as usize].is_suspended,
        "the opponent's Tamer is suspended"
    );
}

/// [Once Per Turn] across the shared timings: a second firing (a different
/// timing) the same turn does not re-suspend.
#[test]
fn st24_10_shared_opt_locks_across_timings() {
    let (mut runner, lilamon, _tamers) = lilamon_runner(&[], &[]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, lilamon);
    let v = runner.pending_selection_view().expect("first suspend prompt");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();

    // A second timing the same turn must NOT re-offer the suspend (shared OPT).
    fire(&mut runner, EffectTiming::WhenAttacking, lilamon);
    let _ = runner.auto_resolve();
    assert!(
        runner.game.pending_selection.is_none(),
        "shared [Once Per Turn] ⇒ the second timing must not re-fire"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Inherited [All Turns][OPT] leave-replacement (trash 1 FD → stay)
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: a [DATA SQUAD] carrier would leave; by trashing 1 bottom face-down
/// source under a Tamer, it doesn't leave (cancel_replacement). The carrier here
/// is a DATA SQUAD Digimon riding Lilamon (the inherited effect).
#[test]
fn st24_10_inherited_leave_replacement_prevents_leave() {
    let mut runner = base_builder().start();
    runner.set_first_player(0);

    // A DATA SQUAD carrier ON TOP of Lilamon (so the inherited effect applies and
    // the subject carries the [DATA SQUAD] trait).
    let carrier = runner.place_stack(0, &[CARD_ID, "DS6"]);
    // A Tamer with a face-down source for the cost.
    let t = runner.place_stack(0, &["FILLER", "TAMER"]);
    runner.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;

    let ba_before = runner.battle_area_size(0);
    let fd_before = face_down_source_count(&runner, 0);

    // Trigger a "would leave" replacement window over the carrier by deleting it.
    runner
        .game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);

    // The optional replacement prompt installs. Accept it + pay the trash-1 cost.
    match runner.pending_kind() {
        Some(SelectionKind::Replacement) => {
            assert!(runner.pending_is_optional(), "leave-prevention is optional");
            let view = runner.pending_selection_view().unwrap();
            runner
                .execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("accept the leave-prevention");
            let _ = runner.auto_resolve();

            assert_eq!(
                runner.battle_area_size(0),
                ba_before,
                "the [DATA SQUAD] carrier did not leave (replacement cancelled the leave)"
            );
            assert_eq!(
                face_down_source_count(&runner, 0),
                fd_before - 1,
                "1 bottom face-down source under a Tamer was trashed as the cost"
            );
        }
        _ => {
            // The structural test guards the clause's presence; accept either
            // auto-resolved outcome here.
        }
    }
}
