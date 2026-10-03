//! BT25-050 Kiwimon — Digimon, Lv.4, Green, DP 4000, Cost 4.
//! Traits (official DB): Ancient Bird, Iliad, TS + (Rule) Trait: Has [Vegetation] Type.
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] You may suspend 1 Digimon. Then, if there are
//! 2 or more suspended Digimon, 1 of your opponent's Digimon can't unsuspend
//! until their turn ends.
//!
//! Inherited: [Your Turn] All of your Digimon get +1000 DP.
//!
//! Digivolve (official DB): Green Lv.3 / Cost 2; alt Lv.3 w/[TS] trait / Cost 2
//! (cards.json evo_costs empty).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_050.cs
//!
//! - Region "Digivolution Condition" (None): AddSelfDigivolutionRequirement-
//!   StaticEffect(HasTSTraits, cost 2, level 3).
//! - Region "Shared OP/WD": Step 1 optional suspend of ANY battle-area Digimon
//!   (canNoSelect: true). Step 2 gate `SuspendedDigimon count >= 2` (both
//!   players) AND an opponent Digimon exists → MANDATORY pick of 1 opp Digimon
//!   → GainCanNotUnsuspend(UntilOpponentTurnEnd).
//! - Region "ESS" (inherited): ChangeDPStaticEffect(+1000) over OWNER's Digimon,
//!   gated on owner-turn — "[Your Turn] All of your Digimon get +1000 DP."
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - F7 cannot-unsuspend lock (conditional on a board count)
//! - D4 inherited declarative aura (team-wide +DP, your-turn gated)
//! - E2 optional inner select (you-may-suspend) with PASS path

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType, PlaySource, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-050";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 3,
        level: level.saturating_sub(1),
        memory_cost: 2,
    }];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-050 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("OPP-A", 4, 5000, &["Beast"]))
        .add_card(make_digimon("OPP-B", 4, 4000, &["Beast"]))
        .add_card(make_digimon("OWN-A", 5, 4000, &["Beast"]))
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_050_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-050 must be present in embedded DSL pack");

    assert_eq!(card.name, "Kiwimon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
    for trait_name in ["Ancient Bird", "Iliad", "TS", "Vegetation"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-050 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_050_has_shared_on_play_when_digivolving_clause() {
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
        .expect("BT25-050 must have a shared OnPlay/WhenDigivolving clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    // The lock (AddModifier) lives inside the count-gate `if` block, so recurse.
    fn has_add_modifier(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| match s {
            CompiledStep::AddModifier { .. } => true,
            CompiledStep::If {
                then, else_branch, ..
            } => has_add_modifier(then) || has_add_modifier(else_branch),
            _ => false,
        })
    }
    assert!(
        has_add_modifier(&clause.process),
        "shared clause must add a CannotUnsuspend modifier (the lock step)"
    );
}

#[test]
fn bt25_050_has_inherited_dp_aura() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let aura = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                dp_modifier,
                scope,
                ..
            }) => Some((*dp_modifier, *scope)),
            _ => None,
        })
        .expect("BT25-050 must have a DP aura clause");

    assert_eq!(aura.0, Some(1000), "inherited aura grants +1000 DP");
    assert_eq!(
        aura.1,
        CompiledScope::Inherited,
        "the DP aura must be inherited scope"
    );
}

// ─── Section 2 — Behavior: suspend + conditional lock ─────────────────────────

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

#[test]
fn bt25_050_suspend_pick_is_optional_and_suspends_chosen_target() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));

    let _k = runner.play(0, 0).expect("play Kiwimon");
    assert!(
        runner.pending_selection().is_some(),
        "OnPlay installs the suspend prompt"
    );
    assert!(
        runner.pending_is_optional(),
        "the you-may-suspend pick exposes PASS"
    );
    pick(&mut runner, opp_b);

    assert!(
        is_suspended(&runner, opp_b),
        "the chosen Digimon is suspended"
    );
    assert!(
        !is_suspended(&runner, opp_a),
        "the other Digimon is untouched"
    );
    // Only 1 suspended Digimon → no lock prompt.
    assert!(runner.pending_selection().is_none());
    assert!(!runner.modifiers().has(opp_a, ModifierType::CannotUnsuspend));
    assert!(!runner.modifiers().has(opp_b, ModifierType::CannotUnsuspend));
}

/// Negative gate: declining the suspend leaves fewer than 2 suspended Digimon,
/// so no opponent permanent is locked.
#[test]
fn bt25_050_declining_suspend_with_fewer_than_two_suspended_does_not_lock() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    runner.game.suspend(opp_b); // only 1 suspended

    let _k = runner.play(0, 0).expect("play Kiwimon");
    runner
        .execute_action(0, PASS)
        .expect("decline the optional suspend");

    assert!(
        runner.pending_selection().is_none(),
        "with fewer than 2 suspended Digimon, no lock prompt"
    );
    assert!(!runner.modifiers().has(opp, ModifierType::CannotUnsuspend));
    assert!(!runner.modifiers().has(opp_b, ModifierType::CannotUnsuspend));
}

/// PASS on the suspend still reaches the "2 or more suspended" check
/// (DCGO canNoSelect continues the coroutine).
#[test]
fn bt25_050_declining_suspend_with_two_already_suspended_still_locks() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    runner.game.suspend(opp_a);
    runner.game.suspend(opp_b);

    let _k = runner.play(0, 0).expect("play Kiwimon");
    runner.execute_action(0, PASS).expect("decline suspend");
    let v = runner
        .pending_selection_view()
        .expect("lock prompt installs even after a declined suspend");
    assert!(!v.is_optional, "the lock pick is mandatory");
    pick(&mut runner, opp_a);
    assert!(runner.modifiers().has(opp_a, ModifierType::CannotUnsuspend));
    assert!(!runner.modifiers().has(opp_b, ModifierType::CannotUnsuspend));
}

/// Positive lock with an explicit target: the lock lands on the chosen
/// Digimon and keeps it suspended through the opponent's unsuspend phase.
#[test]
fn bt25_050_two_suspended_locks_chosen_opponent_digimon_through_their_unsuspend() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    runner.game.suspend(opp_a);
    runner.game.suspend(opp_b);

    let _k = runner.play(0, 0).expect("play Kiwimon");
    runner.execute_action(0, PASS).expect("decline suspend");
    let v = runner.pending_selection_view().expect("lock prompt");
    assert!(v.valid_action_ids.contains(&pick_id(&runner, opp_a)));
    assert!(v.valid_action_ids.contains(&pick_id(&runner, opp_b)));
    pick(&mut runner, opp_b);
    assert!(runner.pending_selection().is_none());

    assert!(runner.modifiers().has(opp_b, ModifierType::CannotUnsuspend));
    assert!(
        !runner.modifiers().has(opp_a, ModifierType::CannotUnsuspend),
        "only the chosen Digimon is locked"
    );

    runner.end_turn(); // opponent's turn: their unsuspend phase runs
    assert_eq!(runner.turn_player(), 1);
    assert!(
        !is_suspended(&runner, opp_a),
        "control: the unlocked Digimon unsuspends in the opponent's unsuspend phase"
    );
    assert!(
        is_suspended(&runner, opp_b),
        "the locked Digimon stays suspended through the opponent's unsuspend phase"
    );
}

/// The effect's own suspend brings the count to 2 (1 already suspended + the
/// one just suspended) → the lock applies.
#[test]
fn bt25_050_own_suspend_brings_count_to_two_and_locks() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    runner.game.suspend(opp_a);

    let _k = runner.play(0, 0).expect("play Kiwimon");
    pick(&mut runner, opp_b);
    assert!(is_suspended(&runner, opp_b), "Kiwimon suspended OPP-B");

    let v = runner
        .pending_selection_view()
        .expect("2 suspended → mandatory lock prompt");
    assert!(!v.is_optional);
    pick(&mut runner, opp_a);
    assert!(runner.modifiers().has(opp_a, ModifierType::CannotUnsuspend));
}

/// [When Digivolving] fires the same clause (Lv.3 [TS] base, cost 2).
#[test]
fn bt25_050_digivolve_from_lv3_ts_base_fires_when_digivolving() {
    let mut base3 = make_digimon("TS-3", 3, 3000, &["TS"]); // Red by default
    base3.evo_costs.clear();
    let mut runner = base().add_card(base3).hand(0, &[CARD_ID]).memory(6).start();
    let host = runner.place_on_field(0, "TS-3", Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));

    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, host.index as usize, PlaySource::ByHand),
        "a Red Lv.3 [TS] base qualifies via the [TS] route"
    );
    assert_eq!(runner.memory(), 4, "[TS] route costs 2");
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID
    );
    assert!(
        runner.pending_is_optional(),
        "[When Digivolving] installs the optional suspend prompt"
    );
    pick(&mut runner, opp);
    assert!(is_suspended(&runner, opp));
}

/// Standard circle: Green Lv.3 (no [TS]) / cost 2.
#[test]
fn bt25_050_digivolve_from_green_lv3_standard_circle() {
    let mut green3 = make_digimon("GREEN-3", 3, 3000, &["Beast"]);
    green3.colors = vec![CardColor::Green];
    let mut runner = base()
        .add_card(green3)
        .hand(0, &[CARD_ID])
        .memory(6)
        .start();
    let host = runner.place_on_field(0, "GREEN-3", Some(0));
    assert!(runner
        .game
        .digivolve_from_hand(0, 0, host.index as usize, PlaySource::ByHand));
    assert_eq!(runner.memory(), 4, "standard green circle costs 2");
}

/// Negative route: a Red Lv.3 without [TS] matches neither circle.
#[test]
fn bt25_050_cannot_digivolve_from_red_lv3_without_ts() {
    let mut runner = base()
        .add_card(make_digimon("RED-3", 3, 3000, &["Beast"]))
        .hand(0, &[CARD_ID])
        .memory(6)
        .start();
    let host = runner.place_on_field(0, "RED-3", Some(0));
    assert!(!runner
        .game
        .digivolve_from_hand(0, 0, host.index as usize, PlaySource::ByHand));
    assert_eq!(runner.memory(), 6);
}

// ─── Section 3 — Behavior: inherited +1000 DP aura ────────────────────────────

/// Inherited aura: Kiwimon as a digivolution source under a carrier grants ALL
/// of the controller's Digimon +1000 DP on the controller's turn — and nothing
/// to the opponent's Digimon.
#[test]
fn bt25_050_inherited_aura_buffs_all_own_digimon_on_your_turn() {
    let mut runner = base().memory(6).start();
    // place_stack: last id is the top card. Kiwimon underneath OWN-A.
    let carrier = runner.place_stack(0, &["BT25-050", "OWN-A"]);
    let other = runner.place_on_field(0, "OPP-B", Some(0)); // own, base 4000
    let opp = runner.place_on_field(1, "OPP-A", Some(0)); // opponent, base 5000
                                                          // Filtered (team-wide) auras materialize on a declarative tick; place_stack
                                                          // skips the play action, so tick explicitly before reading effective DP.
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(carrier),
        Some(5000),
        "carrier 4000 + 1000"
    );
    assert_eq!(
        runner.effective_dp(other),
        Some(5000),
        "second own Digimon 4000 + 1000"
    );
    assert_eq!(
        runner.effective_dp(opp),
        Some(5000),
        "opponent Digimon unchanged (5000)"
    );
}

/// Gate (negative): the inherited aura is inactive on the opponent's turn.
#[test]
fn bt25_050_inherited_aura_inactive_on_opponents_turn() {
    let mut runner = base().memory(6).start();
    let carrier = runner.place_stack(0, &["BT25-050", "OWN-A"]);
    runner.end_turn(); // now player 1's turn
    runner.game.tick_declarative_effects();

    let dp = runner
        .effective_dp(carrier)
        .expect("carrier has effective DP");
    assert_eq!(
        dp, 4000,
        "on the opponent's turn the [Your Turn] aura is inactive (base 4000)"
    );
}
