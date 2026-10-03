//! BT25-059 Ceresmon — Digimon, Lv.6, Green/Yellow, DP 12000, Cost 12.
//! Traits: Shaman, Olympos XII, Iliad, TS + (Rule) [Vegetation]. Attribute: Data.
//! Digivolve (official DB): Green Lv.5 / 4, Yellow Lv.5 / 4; alt Lv.5 w/[Vegetation]/[TS] / 3.
//!
//! # Card text (card image BT25-059 — authoritative for printed text)
//! When this card would be played, if there are 2 or more suspended Digimon,
//! reduce the cost by 5.
//! [On Play] [When Digivolving] You may suspend up to 2 Digimon. Then, none of
//! your suspended [Vegetation] or [TS] trait Digimon are affected by your
//! opponent's Digimon effects until their turn ends.
//! [All Turns] [Once Per Turn] When any Digimon suspend, to 1 of your opponent's
//! Digimon, give -3000 DP until their turn ends for each suspended Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_059.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API 4.3)
//! - D2 conditional cost reduction; CountCappedMultiSelect suspend up to 2
//! - F6 effect immunity (own suspended Veg/TS); on_suspend DP-debuff formula

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_dsl::compiled::CompiledColor;
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, PlaySource, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-059";

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
        .expect("BT25-059 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("OWN-VEG", 4, 4000, &["Vegetation"]))
        .add_card(make_digimon("OPP-DIGI", 5, 9000, &["Beast"]))
        .add_card(make_digimon("OPP-DIGI2", 4, 6000, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

#[test]
fn bt25_059_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Ceresmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for t in ["Shaman", "Olympos XII", "Iliad", "TS", "Vegetation"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
    assert_eq!(
        card.color,
        vec![CompiledColor::Green, CompiledColor::Yellow],
        "official DB: Green/Yellow"
    );
}

#[test]
fn bt25_059_has_cost_reduction() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
    )));
}

#[test]
fn bt25_059_has_op_wd_suspend_clause() {
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
        .expect("OP/WD clause present");
    let has_suspend = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectCountCappedMulti { .. }));
    assert!(
        has_suspend,
        "OP/WD clause has a count-capped suspend selection"
    );
}

#[test]
fn bt25_059_has_on_suspend_debuff_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => Some(t),
            _ => None,
        })
        .expect("on_suspend clause present");
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(
        clause.condition.is_none(),
        "DCGO activates (and spends the OPT) even with no opponent Digimon"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddDpModifier { .. })),
        "on_suspend clause applies a DP modifier"
    );
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

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

fn immune(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner
        .game
        .permanent_is_unaffected_by_effect(h, 1, EffectSourceKind::Digimon)
}

fn make_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

// ─── Behavior: OP/WD suspend up to 2 ────────────────────────────────────────

#[test]
fn bt25_059_on_play_installs_suspend_up_to_2_prompt() {
    let mut runner = base().memory(12).start();
    runner.place_on_field(0, "OWN-VEG", Some(0));
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let ceres = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, ceres.index as usize);

    // "Suspend up to 2 Digimon" spans BOTH battle areas (`of: any`).
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::AnyField)),
        "suspend-up-to-2 prompt installs over both sides"
    );
    let view = runner.pending_selection_view().unwrap();
    assert!(view.is_optional, "you MAY suspend");
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        3,
        "own Vegetation, Ceresmon, and the opponent's Digimon"
    );
}

/// Pick two targets (one own, one opponent): both are suspended.
#[test]
fn bt25_059_suspend_picks_two_targets_and_both_suspend() {
    let mut runner = base().memory(12).start();
    let veg = runner.place_on_field(0, "OWN-VEG", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let opp2 = runner.place_on_field(1, "OPP-DIGI2", Some(0));
    let ceres = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, ceres.index as usize);

    pick(&mut runner, veg);
    assert!(
        runner.pending_selection().is_some(),
        "the prompt re-installs for the second pick"
    );
    pick(&mut runner, opp);
    // Two Digimon suspended → the [All Turns] debuff fires once (OPT);
    // target OPP-DIGI (9000 → 3000, survives).
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    pick(&mut runner, opp);
    assert!(runner.pending_selection().is_none());

    assert!(is_suspended(&runner, veg), "own Vegetation suspended");
    assert!(is_suspended(&runner, opp), "opponent Digimon suspended");
    assert!(!is_suspended(&runner, ceres), "Ceresmon not picked → unsuspended");
    assert!(!is_suspended(&runner, opp2));
    assert_eq!(runner.effective_dp(opp), Some(3000), "2 suspended → -6000");
}

// ─── Behavior: suspended [Vegetation]/[TS] immunity (continuous) ────────────

#[test]
fn bt25_059_immunity_covers_only_own_suspended_veg_or_ts_digimon() {
    let mut runner = base()
        .add_card(make_digimon("OWN-TS", 4, 4000, &["TS"]))
        .add_card(make_digimon("OWN-PLAIN", 4, 4000, &["Beast"]))
        .memory(12)
        .start();
    let veg_s = runner.place_on_field(0, "OWN-VEG", Some(0));
    let ts_s = runner.place_on_field(0, "OWN-TS", Some(0));
    let veg_u = runner.place_on_field(0, "OWN-VEG", Some(0));
    let plain_s = runner.place_on_field(0, "OWN-PLAIN", Some(0));
    let opp_s = runner.place_on_field(1, "OPP-DIGI", Some(0));
    for h in [veg_s, ts_s, plain_s, opp_s] {
        runner.game.suspend(h);
    }
    let ceres = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, ceres.index as usize);
    pass(&mut runner); // suspend nothing
    runner.auto_resolve().ok();
    runner.game.tick_declarative_effects();

    assert!(immune(&runner, veg_s), "suspended [Vegetation] is protected");
    assert!(immune(&runner, ts_s), "suspended [TS] is protected");
    assert!(!immune(&runner, veg_u), "UNsuspended [Vegetation] is not protected");
    assert!(!immune(&runner, plain_s), "suspended non-trait Digimon is not protected");
    assert!(!immune(&runner, ceres), "Ceresmon itself is unsuspended → not protected");
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(opp_s, 0, EffectSourceKind::Digimon),
        "the opponent's suspended Digimon gain nothing"
    );
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(veg_s, 1, EffectSourceKind::Tamer),
        "protection is Digimon-effects only"
    );
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(veg_s, 0, EffectSourceKind::Digimon),
        "protection is opponent-scoped (own effects still apply)"
    );
}

/// DCGO re-evaluates the condition continuously: a [Vegetation] Digimon that
/// becomes suspended later in the window is covered; one that unsuspends
/// drops out. The protection persists through the opponent's turn.
#[test]
fn bt25_059_immunity_tracks_suspension_and_lasts_through_opponents_turn() {
    let mut runner = base().memory(12).start();
    let veg_a = runner.place_on_field(0, "OWN-VEG", Some(0));
    let veg_b = runner.place_on_field(0, "OWN-VEG", Some(0));
    runner.game.suspend(veg_a);
    let ceres = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, ceres.index as usize);
    pass(&mut runner);
    runner.auto_resolve().ok();
    runner.game.tick_declarative_effects();
    assert!(immune(&runner, veg_a));
    assert!(!immune(&runner, veg_b));

    // Later in the window: B suspends (covered), A unsuspends (not covered).
    runner.game.suspend(veg_b);
    runner.auto_resolve().ok();
    runner.game.unsuspend(veg_a);
    runner.game.tick_declarative_effects();
    assert!(immune(&runner, veg_b), "newly suspended [Vegetation] is covered");
    assert!(!immune(&runner, veg_a), "unsuspended [Vegetation] drops out");

    // Persists into the opponent's turn ("until their turn ends").
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);
    runner.game.tick_declarative_effects();
    assert!(
        immune(&runner, veg_b),
        "still protected during the opponent's turn"
    );
}

// ─── Behavior: [All Turns] on_suspend DP debuff ─────────────────────────────

/// Exact amount: -3000 × the number of suspended Digimon (both players),
/// fixed at resolution, lasting through the opponent's turn and gone after.
#[test]
fn bt25_059_debuff_is_minus_3000_per_suspended_and_expires() {
    let mut runner = base().memory(12).start();
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0)); // 9000
    let opp2 = runner.place_on_field(1, "OPP-DIGI2", Some(0)); // 6000
    let veg = runner.place_on_field(0, "OWN-VEG", Some(0));
    runner.game.suspend(opp2); // before Ceresmon is on the field → no trigger
    let _ceres = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.suspend(veg); // 2 suspended Digimon now
    let v = runner
        .pending_selection_view()
        .expect("a Digimon suspending triggers the debuff pick");
    assert!(!v.is_optional, "the debuff pick is mandatory");
    pick(&mut runner, opp);
    runner.auto_resolve().ok();
    assert_eq!(runner.effective_dp(opp), Some(3000), "9000 - 2 × 3000");
    assert_eq!(runner.effective_dp(opp2), Some(6000), "only the chosen Digimon");

    runner.end_turn(); // opponent's turn (their unsuspend phase un-suspends OPP-DIGI2)
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(
        runner.effective_dp(opp),
        Some(3000),
        "debuff lasts through the opponent's turn and is fixed at resolution"
    );
    runner.end_turn(); // back to our turn → expired
    assert_eq!(runner.turn_player(), 0);
    assert_eq!(runner.effective_dp(opp), Some(9000), "debuff expired");
}

/// Only a DIGIMON suspending triggers — a Tamer suspending does not (and does
/// not spend the once-per-turn).
#[test]
fn bt25_059_tamer_suspending_does_not_trigger() {
    let mut runner = base().add_card(make_tamer("TAMER")).memory(12).start();
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let tamer = runner.place_on_field(0, "TAMER", Some(0));
    let veg = runner.place_on_field(0, "OWN-VEG", Some(0));
    let _ceres = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.suspend(tamer);
    assert!(is_suspended(&runner, tamer));
    assert!(
        runner.pending_selection().is_none(),
        "a Tamer suspending does not trigger [All Turns]"
    );
    assert_eq!(runner.effective_dp(opp), Some(9000));

    runner.game.suspend(veg);
    assert!(
        runner.pending_selection().is_some(),
        "a Digimon suspending still triggers (OPT unspent)"
    );
    pick(&mut runner, opp);
    assert_eq!(runner.effective_dp(opp), Some(6000), "1 suspended Digimon → -3000");
}

/// [Once Per Turn]: the second Digimon suspension in the same turn does not
/// trigger; the clause resets on the next turn (All Turns → opponent's turn).
#[test]
fn bt25_059_once_per_turn_lockout_and_next_turn_reset() {
    let mut runner = base()
        .add_card(make_digimon("OPP-DIGI3", 5, 10000, &["Beast"]))
        // Memory 0: after end_turn the opponent sits at 0, so resolving the
        // prompt on their turn does not end it (a + gauge would flip to −).
        .memory(0)
        .start();
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let opp3 = runner.place_on_field(1, "OPP-DIGI3", Some(0));
    let veg_a = runner.place_on_field(0, "OWN-VEG", Some(0));
    let veg_b = runner.place_on_field(0, "OWN-VEG", Some(0));
    let _ceres = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.suspend(veg_a);
    pick(&mut runner, opp);
    assert_eq!(runner.effective_dp(opp), Some(6000));

    runner.game.suspend(veg_b);
    assert!(
        runner.pending_selection().is_none(),
        "OPT spent → no second trigger this turn"
    );

    runner.end_turn(); // opponent's turn: new turn, OPT resets
    assert_eq!(runner.turn_player(), 1);
    let before = runner.effective_dp(opp3);
    runner.game.suspend(opp3);
    assert!(
        runner.pending_selection().is_some(),
        "[All Turns] fires again on the next turn"
    );
    pick(&mut runner, opp3);
    assert_eq!(runner.turn_player(), 1);
    // Our two [Vegetation] are still suspended + OPP-DIGI3 → 3 × -3000.
    assert_eq!(runner.effective_dp(opp3), Some(1000), "10000 - 3 × 3000");
}

/// No activation condition: with no opponent Digimon the clause still
/// activates (spending the OPT) and does nothing — DCGO behavior.
#[test]
fn bt25_059_no_opponent_digimon_still_spends_opt() {
    let mut runner = base().memory(12).start();
    let veg_a = runner.place_on_field(0, "OWN-VEG", Some(0));
    let veg_b = runner.place_on_field(0, "OWN-VEG", Some(0));
    let _ceres = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.suspend(veg_a);
    assert!(runner.pending_selection().is_none(), "no candidate → no prompt");

    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    runner.game.suspend(veg_b);
    assert!(
        runner.pending_selection().is_none(),
        "the earlier activation spent the once-per-turn"
    );
    assert_eq!(runner.effective_dp(opp), Some(9000));
}

// ─── Digivolution routes ─────────────────────────────────────────────────────

#[test]
fn bt25_059_digivolves_from_yellow_lv5_for_cost_4() {
    let mut yellow5 = make_digimon("YELLOW-5", 5, 7000, &["Beast"]);
    yellow5.colors = vec![CardColor::Yellow];
    let mut runner = base().add_card(yellow5).hand(0, &[CARD_ID]).memory(10).start();
    let host = runner.place_on_field(0, "YELLOW-5", Some(0));
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, host.index as usize, PlaySource::ByHand),
        "Lv.5 Yellow → Ceresmon (standard yellow circle)"
    );
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID
    );
    assert_eq!(runner.memory(), 6, "yellow circle costs 4");
}

#[test]
fn bt25_059_cannot_digivolve_from_blue_lv5_without_trait() {
    let mut blue5 = make_digimon("BLUE-5", 5, 7000, &["Beast"]);
    blue5.colors = vec![CardColor::Blue];
    let mut runner = base().add_card(blue5).hand(0, &[CARD_ID]).memory(10).start();
    let host = runner.place_on_field(0, "BLUE-5", Some(0));
    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, host.index as usize, PlaySource::ByHand),
        "Blue is not a printed circle (the card is Green/Yellow)"
    );
}

// ─── Cost reduction: 2+ suspended Digimon (either player) → -5 ───────────────

/// Positive: two suspended OPPONENT Digimon satisfy "if there are 2 or more
/// suspended Digimon" (DCGO counts both players) → play cost 12 - 5 = 7.
#[test]
fn bt25_059_cost_reduced_with_two_suspended_opponent_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    let a = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let b = runner.place_on_field(1, "OPP-DIGI2", Some(0));
    runner.game.suspend(a);
    runner.game.suspend(b);
    runner.play(0, 0).expect("play Ceresmon");
    assert_eq!(runner.memory(), 3, "2 suspended Digimon: paid 7 (12 - 5) from 10");
}

/// Negative: only one suspended Digimon → full cost 12.
#[test]
fn bt25_059_cost_not_reduced_with_one_suspended_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    let a = runner.place_on_field(1, "OPP-DIGI", Some(0));
    runner.place_on_field(1, "OPP-DIGI2", Some(0));
    runner.game.suspend(a);
    runner.play(0, 0).expect("play Ceresmon");
    assert_eq!(runner.memory(), -2, "1 suspended Digimon: full cost 12 paid from 10");
}
