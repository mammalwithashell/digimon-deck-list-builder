//! BT25-077 Bacchusmon — Digimon, Lv.6, Black/Green, DP 12000, Cost 12.
//! Traits: Shaman, Olympos XII, Iliad, TS. Attribute: Virus.
//!
//! # Card text (card image BT25-077 — authoritative for printed text)
//! When this card would be played, if there are 12 or more levels' total worth
//! of Digimon, reduce the cost by 5.
//! [On Play] [When Digivolving] You may play 1 [TS] trait Digimon card with 6000
//! DP or less from your hand without paying the cost.
//! [All Turns] [Once Per Turn] When any Digimon are played or digivolve, you may
//! suspend 1 Digimon. Then, if played or digivolved by an effect, delete 1 of
//! your opponent's lowest DP Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_077.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API 4.3)
//! - C1 play-from-hand free; observer suspend; effect-gated lowest-DP delete
//! - B-cost self play-cost reduction gated on a board-wide level sum
//!   (`level_sum_gte`, both players' Digimon)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, CostDelta, PlaySource, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-077";

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
        .expect("BT25-077 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("TS-LOW", 4, 5000, &["TS"]))
        .add_card(make_digimon("TS-BIG", 5, 9000, &["TS"]))
        .add_card(make_digimon("OWN-PLAY", 4, 4000, &["Beast"]))
        .add_card(make_digimon("OPP-SMALL", 3, 2000, &["Beast"]))
        .add_card(make_digimon("OPP-BIG", 5, 9000, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

#[test]
fn bt25_077_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Bacchusmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for t in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_077_has_op_wd_play_free_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OP/WD play-free clause present");
    assert!(clause.optional, "printed 'you may' -> optional");
    assert!(clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromHandFree { .. })));
}

#[test]
fn bt25_077_has_all_turns_suspend_delete_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if (t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    || t.when.contains(&CompiledTiming::OnDigivolve))
                    && t.once_per_turn =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[All Turns] suspend/delete clause present");
    // DCGO optional:false — only the suspend is "you may" (in-body optional
    // select); the by-effect delete is mandatory, so the clause itself must
    // NOT be skippable as a whole.
    assert!(
        !clause.optional,
        "clause-level optional would let the player skip the mandatory delete"
    );
    fn has_refund(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| match s {
            CompiledStep::RefundOpt { .. } => true,
            CompiledStep::If {
                then, else_branch, ..
            } => has_refund(then) || has_refund(else_branch),
            _ => false,
        })
    }
    assert!(
        has_refund(&clause.process),
        "DCGO RemoveUse (not by effect AND nothing suspended) → refund_opt"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::Suspend { .. })),
        "clause has a top-level suspend step"
    );
    // The lowest-DP delete is nested inside the `if event_is_effect_initiated`
    // gate, so it appears as a CompiledStep::If at the clause top level.
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::If { .. })),
        "clause has an effect-initiated gate (If) wrapping the lowest-DP delete"
    );
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn colored(mut card: CardData, color: CardColor) -> CardData {
    card.colors = vec![color];
    card
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn field_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Action id that targets permanent `h` in the currently pending field prompt.
fn pick_id(runner: &DebugRunner, h: PermanentHandle) -> u16 {
    match runner.pending_kind() {
        Some(SelectionKind::AnyField) => encode_attack(h.player as u16, h.index as u16),
        _ => encode_attack(0, h.index as u16),
    }
}

fn pick(runner: &mut DebugRunner, h: PermanentHandle) {
    let id = pick_id(runner, h);
    let player = runner.pending_selection().unwrap().selecting_player;
    runner.execute_action(player, id).expect("pick target");
}

fn pass(runner: &mut DebugRunner) {
    let player = runner.pending_selection().unwrap().selecting_player;
    runner.execute_action(player, PASS).expect("PASS");
}

/// Resolve whatever is pending: accept outer optional prompts, pick the first
/// non-PASS candidate in a Hand prompt (the [TS] play), PASS optional field
/// prompts (the "you may suspend"), and take the first legal id otherwise.
fn drive_default(runner: &mut DebugRunner) {
    for _ in 0..32 {
        let Some(v) = runner.pending_selection_view() else {
            return;
        };
        let player = v.selecting_player;
        let id = match v.kind {
            SelectionKind::Hand | SelectionKind::Replacement => v
                .valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .unwrap_or(PASS),
            SelectionKind::AnyField | SelectionKind::OwnField | SelectionKind::OppField
                if v.is_optional =>
            {
                PASS
            }
            _ => v.valid_action_ids.first().copied().unwrap_or(PASS),
        };
        runner.execute_action(player, id).expect("drive");
    }
    panic!("selection loop did not terminate");
}

// ─── Behavior: [OP][WD] play a TS <=6000 Digimon from hand free ─────────────

#[test]
fn bt25_077_on_play_plays_ts_low_from_hand_free() {
    let mut runner = base().hand(0, &["TS-LOW"]).memory(5).start();
    let bac = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bac.index as usize);

    // Outer "you may" (if installed) → accept.
    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        runner.accept_optional_trigger().expect("accept OP");
    }
    let v = runner.pending_selection_view().expect("hand pick installs");
    assert_eq!(v.kind, SelectionKind::Hand, "OP offers a hand pick");
    assert!(v.is_optional, "the hand pick is 'you may' (PASS legal)");
    let pick_hand = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("TS-LOW is a candidate");
    runner.execute_action(0, pick_hand).expect("pick TS-LOW");
    // TS-LOW's entry (by effect) fires the [All Turns] clause: pass the
    // suspend; the opponent has no Digimon, so the delete has no target.
    drive_default(&mut runner);

    assert!(
        field_ids(&runner, 0).contains(&"TS-LOW".to_string()),
        "TS-LOW must be played onto the field"
    );
    assert_eq!(runner.hand_size(0), 0, "TS-LOW left the hand");
    assert_eq!(runner.memory(), 5, "played without paying the cost");
}

#[test]
fn bt25_077_on_play_no_prompt_without_eligible_ts_card() {
    let mut runner = base()
        .hand(0, &["TS-BIG", "OWN-PLAY"]) // [TS] 9000 DP; non-[TS] 4000 DP
        .memory(12)
        .start();
    let bac = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bac.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "no eligible [TS] <=6000 Digimon -> the optional play clause installs no prompt"
    );
    assert_eq!(runner.hand_size(0), 2);
}

#[test]
fn bt25_077_when_digivolving_plays_ts_low_from_hand_free() {
    let mut runner = base()
        .add_card(colored(
            make_digimon("BLACK-5", 5, 7000, &[]),
            CardColor::Black,
        ))
        .hand(0, &[CARD_ID, "TS-LOW"])
        .memory(10)
        .start();
    let host = runner.place_on_field(0, "BLACK-5", Some(0));
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, host.index as usize, PlaySource::ByHand),
        "Lv.5 Black → Bacchusmon (standard circle, cost 4)"
    );
    assert_eq!(runner.memory(), 6, "standard black circle costs 4");
    let mut saw_hand_prompt = false;
    for _ in 0..32 {
        let Some(v) = runner.pending_selection_view() else {
            break;
        };
        if v.kind == SelectionKind::Hand {
            saw_hand_prompt = true;
        }
        drive_default(&mut runner);
    }
    assert!(saw_hand_prompt, "[WD] offers the [TS] hand play");
    assert!(
        field_ids(&runner, 0).contains(&"TS-LOW".to_string()),
        "WD plays TS-LOW from hand for free"
    );
    assert_eq!(runner.memory(), 6, "the WD play is free");
}

// ─── Behavior: [All Turns] ───────────────────────────────────────────────────

/// Effect play: pick the suspend target explicitly, then the mandatory delete
/// offers ONLY the opponent's lowest-DP Digimon and deletes it.
#[test]
fn bt25_077_all_turns_effect_play_suspends_choice_then_deletes_lowest() {
    let mut runner = base().hand(0, &["OWN-PLAY"]).memory(12).start();
    let _bac = runner.place_on_field(0, CARD_ID, Some(0));
    let small = runner.place_on_field(1, "OPP-SMALL", Some(0));
    let big = runner.place_on_field(1, "OPP-BIG", Some(0));

    let played = runner.game.players[0].hand[0].handle();
    runner
        .game
        .play_card_from_effect_without_cost(0, played)
        .expect("effect play");

    assert!(runner.pending_is_optional(), "the suspend is 'you may'");
    assert!(runner
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .contains(&pick_id(&runner, big)));
    pick(&mut runner, big);
    assert!(is_suspended(&runner, big), "chosen Digimon is suspended");
    assert!(!is_suspended(&runner, small));

    let v = runner
        .pending_selection_view()
        .expect("mandatory delete prompt installs");
    assert!(!v.is_optional, "the by-effect delete is mandatory");
    let small_id = pick_id(&runner, small);
    let big_id = pick_id(&runner, big);
    assert!(
        v.valid_action_ids.contains(&small_id),
        "lowest-DP target offered"
    );
    assert!(
        !v.valid_action_ids.contains(&big_id),
        "a non-lowest Digimon must not be offered"
    );
    runner.execute_action(0, small_id).expect("delete lowest");
    runner.auto_resolve().ok();

    assert_eq!(field_ids(&runner, 1), vec!["OPP-BIG".to_string()]);
    assert_eq!(runner.trash_size(1), 1, "OPP-SMALL deleted to trash");
}

/// Effect play with a PASS on the suspend still deletes (the delete is not
/// gated on the suspend).
#[test]
fn bt25_077_all_turns_effect_play_pass_suspend_still_deletes() {
    let mut runner = base().hand(0, &["OWN-PLAY"]).memory(12).start();
    let _bac = runner.place_on_field(0, CARD_ID, Some(0));
    let small = runner.place_on_field(1, "OPP-SMALL", Some(0));
    let _big = runner.place_on_field(1, "OPP-BIG", Some(0));

    let played = runner.game.players[0].hand[0].handle();
    runner.game.play_card_from_effect_without_cost(0, played);
    pass(&mut runner);

    let v = runner
        .pending_selection_view()
        .expect("delete prompt still installs after a declined suspend");
    assert!(!v.is_optional, "delete is mandatory");
    let small_id = pick_id(&runner, small);
    runner.execute_action(0, small_id).expect("delete lowest");
    runner.auto_resolve().ok();

    assert_eq!(field_ids(&runner, 1), vec!["OPP-BIG".to_string()]);
}

/// Normal hand play (NOT by effect): suspend is offered, chosen target is
/// suspended, and NO delete prompt follows.
#[test]
fn bt25_077_all_turns_hand_play_suspends_but_never_deletes() {
    let mut runner = base().hand(0, &["OWN-PLAY"]).memory(12).start();
    let _bac = runner.place_on_field(0, CARD_ID, Some(0));
    let small = runner.place_on_field(1, "OPP-SMALL", Some(0));
    let big = runner.place_on_field(1, "OPP-BIG", Some(0));

    runner.play(0, 0).expect("hand play OWN-PLAY");
    assert!(
        runner.pending_is_optional(),
        "[All Turns] offers the optional suspend on a hand play"
    );
    pick(&mut runner, small);
    assert!(is_suspended(&runner, small), "chosen Digimon suspended");
    assert!(
        runner.pending_selection().is_none(),
        "not played by an effect → no delete prompt"
    );
    assert_eq!(
        field_ids(&runner, 1),
        vec!["OPP-SMALL".to_string(), "OPP-BIG".to_string()],
        "both opponent Digimon remain"
    );
    assert!(!is_suspended(&runner, big));
}

/// [Once Per Turn]: after the clause resolves on an effect play, a second
/// effect play the same turn gets no prompt.
#[test]
fn bt25_077_all_turns_once_per_turn_lockout() {
    let mut runner = base().hand(0, &["OWN-PLAY", "OWN-PLAY"]).memory(12).start();
    let _bac = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));

    let first = runner.game.players[0].hand[0].handle();
    runner.game.play_card_from_effect_without_cost(0, first);
    pass(&mut runner);
    runner.auto_resolve().expect("mandatory delete");
    assert_eq!(runner.battle_area_size(1), 2);

    let second = runner.game.players[0].hand[0].handle();
    runner.game.play_card_from_effect_without_cost(0, second);
    assert!(
        runner.pending_selection().is_none(),
        "OPT spent → the second effect play gets no prompt"
    );
    assert_eq!(runner.battle_area_size(1), 2, "no second delete");
}

/// Refund (DCGO RemoveUse): a non-effect play + PASS on the suspend does not
/// consume the once-per-turn, so the next (effect) play still triggers.
#[test]
fn bt25_077_all_turns_hand_play_pass_refunds_opt() {
    let mut runner = base().hand(0, &["OWN-PLAY", "OWN-PLAY"]).memory(12).start();
    let _bac = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));

    runner.play(0, 0).expect("hand play");
    assert!(runner.pending_selection().is_some(), "suspend offered");
    pass(&mut runner);
    assert!(runner.pending_selection().is_none());

    let second = runner.game.players[0].hand[0].handle();
    runner.game.play_card_from_effect_without_cost(0, second);
    assert!(
        runner.pending_selection().is_some(),
        "OPT was refunded → the effect play triggers the clause again"
    );
    pass(&mut runner);
    runner.auto_resolve().expect("mandatory delete");
    assert_eq!(field_ids(&runner, 1), vec!["OPP-BIG".to_string()]);
}

/// Negative refund: a non-effect play where a Digimon WAS suspended consumes
/// the once-per-turn.
#[test]
fn bt25_077_all_turns_hand_play_with_suspend_spends_opt() {
    let mut runner = base().hand(0, &["OWN-PLAY", "OWN-PLAY"]).memory(12).start();
    let _bac = runner.place_on_field(0, CARD_ID, Some(0));
    let small = runner.place_on_field(1, "OPP-SMALL", Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0));

    runner.play(0, 0).expect("hand play");
    pick(&mut runner, small);
    assert!(runner.pending_selection().is_none());

    let second = runner.game.players[0].hand[0].handle();
    runner.game.play_card_from_effect_without_cost(0, second);
    assert!(
        runner.pending_selection().is_none(),
        "the suspend executed → OPT spent"
    );
    assert_eq!(runner.battle_area_size(1), 2);
}

/// The clause triggers when a Digimon digivolves (not only on play).
#[test]
fn bt25_077_all_turns_triggers_on_digivolve() {
    let mut evo = make_digimon("EVO-4", 4, 5000, &["Beast"]);
    evo.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 2,
    }];
    let mut runner = base()
        .add_card(make_digimon("BASE-3", 3, 3000, &["Beast"]))
        .add_card(evo)
        .hand(0, &["EVO-4"])
        .memory(12)
        .start();
    let _bac = runner.place_on_field(0, CARD_ID, Some(0));
    let host = runner.place_on_field(0, "BASE-3", Some(0));
    let opp = runner.place_on_field(1, "OPP-SMALL", Some(0));

    assert!(runner
        .game
        .digivolve_from_hand(0, 0, host.index as usize, PlaySource::ByHand));
    assert!(
        runner.pending_is_optional(),
        "[All Turns] offers the optional suspend when a Digimon digivolves"
    );
    pick(&mut runner, opp);
    assert!(is_suspended(&runner, opp));
    assert!(
        runner.pending_selection().is_none(),
        "hand digivolve is not by effect → no delete"
    );
    assert_eq!(runner.battle_area_size(1), 1);
}

// ─── Alt digivolve: Lv.5 [TS] (any color) / Cost 3 ──────────────────────────

#[test]
fn bt25_077_digivolves_from_red_lv5_ts_for_cost_3() {
    let mut runner = base()
        .add_card(make_digimon("RED-TS-5", 5, 7000, &["TS"])) // Red by default
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let host = runner.place_on_field(0, "RED-TS-5", Some(0));
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, host.index as usize, PlaySource::ByHand),
        "a Red Lv.5 [TS] base qualifies via the [TS] alt route"
    );
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID
    );
    assert_eq!(runner.memory(), 7, "[TS] route costs 3 (10 - 3)");
}

#[test]
fn bt25_077_cannot_digivolve_from_red_lv5_without_ts() {
    let mut runner = base()
        .add_card(make_digimon("RED-5", 5, 7000, &["Beast"]))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let host = runner.place_on_field(0, "RED-5", Some(0));
    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, host.index as usize, PlaySource::ByHand),
        "a Red non-[TS] Lv.5 matches no route"
    );
    assert_eq!(runner.memory(), 10);
}

// ─── Cost reduction: 12+ total levels of Digimon (both players) → -5 ─────────

#[test]
fn bt25_077_has_level_sum_cost_reduction_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { condition: Some(p), .. })
            if p.level_sum_gte.is_some())
    });
    assert!(has, "cost reduction must be gated on level_sum_gte");
}

/// Positive: OWN Lv.5 + OPP Lv.4 + OPP Lv.3 = 12 levels across BOTH players
/// (DCGO sums every battle-area Digimon) → play cost 12 - 5 = 7.
#[test]
fn bt25_077_cost_reduced_with_twelve_levels_across_both_players() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(0, "TS-BIG", Some(0)); // Lv.5 (own)
    runner.place_on_field(1, "OWN-PLAY", Some(0)); // Lv.4 (opponent)
    runner.place_on_field(1, "OPP-SMALL", Some(0)); // Lv.3 (opponent)
    runner.play(0, 0).expect("play Bacchusmon");
    assert_eq!(
        runner.memory(),
        3,
        "12 levels on board: paid 7 (12 - 5) from 10"
    );
}

/// Negative: 9 total levels (< 12) → no reduction, full cost 12.
#[test]
fn bt25_077_cost_not_reduced_below_twelve_levels() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(0, "TS-BIG", Some(0)); // Lv.5
    runner.place_on_field(1, "OWN-PLAY", Some(0)); // Lv.4
    runner.play(0, 0).expect("play Bacchusmon");
    assert_eq!(
        runner.memory(),
        -2,
        "9 levels on board: full cost 12 paid from 10"
    );
}

/// Board-wide: 12 levels on the OPPONENT's side alone still reduce (DCGO sums
/// every battle-area Digimon of both players).
#[test]
fn bt25_077_cost_reduced_by_opponent_levels_alone() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(1, "TS-BIG", Some(0)); // Lv.5
    runner.place_on_field(1, "OWN-PLAY", Some(0)); // Lv.4
    runner.place_on_field(1, "OPP-SMALL", Some(0)); // Lv.3
    runner.play(0, 0).expect("play Bacchusmon");
    assert_eq!(
        runner.memory(),
        3,
        "opponent's 12 levels count toward the total"
    );
}
