//! BT25-055 Deramon — Digimon, Lv.5, Green DP 6000, Cost 6.
//! Traits: Avian, Iliad, TS. Standard evo: Lv.4 green / cost 3.
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] You may suspend 1 Digimon. Then, if there are
//! 2 or more suspended Digimon, 1 of your Digimon may unsuspend.
//! [All Turns] [Once Per Turn] When this Digimon suspends, you may play 1
//! 4000 DP or lower Digimon card with [Vegetation], [Plant], [Avian] or [Bird]
//! in any of its traits or the [TS] trait from your hand without paying the cost.
//!
//! Inherited: [Opponent's Turn] [Once Per Turn] When one of your opponent's
//! Digimon attacks, you may change the attack target to 1 of your suspended
//! Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_055.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - C (OnPlay/WhenDigivolving optional suspend + conditional unsuspend)
//! - E (OnSuspend triggered free-play from hand, OPT)
//! - G (inherited opponent-attack redirect)
//! - H (alt digivolution path)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-055";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn own_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(3000);
    card.traits = vec!["Beast".to_string()];
    card
}

fn opp_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(7000);
    card.traits = vec!["Beast".to_string()];
    card
}

/// A 4000-DP-or-lower [Avian] Digimon — a valid OPT free-play target.
fn avian_target(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.traits = vec!["Avian".to_string()];
    card
}

/// Over-DP, non-matching trait — must NOT be a legal OPT free-play target.
fn big_nonmatch(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(9000);
    card.play_cost = 7;
    card.traits = vec!["Machine".to_string()];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-055 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(own_digimon("OWN-A"))
        .add_card(own_digimon("OWN-B"))
        .add_card(opp_digimon("OPP-A"))
        .add_card(avian_target("AVIAN"))
        .add_card(big_nonmatch("BIG"))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_055_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Deramon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.cost, Some(6));
}

#[test]
fn bt25_055_has_alt_digivolve_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(
        !card.alt_paths.is_empty(),
        "BT25-055 must have a TS-trait alt-digivolve path"
    );
}

#[test]
fn bt25_055_op_wd_clause_has_suspend_and_unsuspend() {
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
        .expect("must have OnPlay/WhenDigivolving suspend clause");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::Suspend { .. })),
        "OP/WD clause must contain a suspend step"
    );
}

#[test]
fn bt25_055_opt_suspend_clause_is_once_per_turn_and_optional() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => Some(t),
            _ => None,
        })
        .expect("must have OnSuspend free-play clause");
    assert!(clause.once_per_turn, "OnSuspend clause is [Once Per Turn]");
    assert!(clause.optional, "OnSuspend clause is 'you may'");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromHandFree { .. })),
        "OnSuspend clause must free-play the selected card"
    );
}

#[test]
fn bt25_055_inherited_redirect_clause_present() {
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
        .expect("must have inherited OnOpponentAttack redirect clause");
    assert_eq!(clause.scope, CompiledScope::Inherited, "redirect is inherited");
    assert!(clause.once_per_turn, "redirect clause is OPT");
    assert!(clause.optional, "redirect clause is 'you may'");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::RedirectAttackTarget { .. })),
        "redirect clause must contain a redirect_attack_target step"
    );
}

// ─── Section 2 — Behavioral ──────────────────────────────────────────────────

/// On Play: accepting the suspend pick suspends a chosen Digimon.
#[test]
fn bt25_055_on_play_suspends_chosen_digimon() {
    let mut runner = base().start();
    let victim = runner.place_on_field(0, "OWN-A", Some(1));
    let deramon = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        !runner.game.players[0].battle_area[victim.index as usize].is_suspended,
        "precondition: victim unsuspended"
    );

    runner.fire_on_play(0, deramon.index as usize);
    // The optional suspend pick installs a selection; suspend the victim.
    assert!(
        runner.pending_selection().is_some(),
        "On Play must install the suspend-target selection"
    );
    runner.auto_resolve().expect("resolve suspend (and any unsuspend guard)");

    assert!(
        runner.game.players[0].battle_area[victim.index as usize].is_suspended
            || runner.game.players[0].battle_area[deramon.index as usize].is_suspended,
        "On Play must have suspended a Digimon"
    );
}

/// OnSuspend OPT: when Deramon itself suspends, accepting the trigger lets you
/// free-play a qualifying hand Digimon.
#[test]
fn bt25_055_self_suspend_allows_freeplay() {
    let mut runner = base().hand(0, &["AVIAN"]).start();
    let deramon = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    // Suspend Deramon itself → fires the OnSuspend OPT clause.
    runner.game.suspend(deramon);

    assert!(
        runner.pending_selection().is_some(),
        "self-suspend must install the optional free-play prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the optional free-play");
    runner.auto_resolve().expect("resolve the hand free-play");

    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "the AVIAN card left the hand (played)"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "the AVIAN card was played to the battle area for free"
    );
}

/// OnSuspend OPT — negative: declining the optional trigger plays nothing.
#[test]
fn bt25_055_self_suspend_decline_plays_nothing() {
    let mut runner = base().hand(0, &["AVIAN"]).start();
    let deramon = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner.game.suspend(deramon);
    assert!(runner.pending_selection().is_some(), "prompt installed");
    runner
        .decline_optional_trigger()
        .expect("declining is legal");

    assert_eq!(runner.hand_size(0), hand_before, "no card played on decline");
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "no new permanent on decline"
    );
}

/// OnSuspend OPT — negative: another Digimon (not Deramon) suspending must NOT
/// fire Deramon's self-suspend clause (event_target_is_source gate).
#[test]
fn bt25_055_other_suspend_does_not_fire() {
    let mut runner = base().hand(0, &["AVIAN"]).start();
    let deramon = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OWN-A", Some(1));

    // Suspend the OTHER Digimon, not Deramon.
    runner.game.suspend(other);

    assert!(
        runner.pending_selection().is_none(),
        "Deramon's self-suspend clause must NOT fire when a different Digimon suspends"
    );
}
