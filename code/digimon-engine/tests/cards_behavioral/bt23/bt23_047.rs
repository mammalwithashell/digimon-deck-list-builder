//! BT23-047 Examon — Digimon, Lv.7, Green/Red, DP 15000, Cost 15.
//!
//! # Card text (image + cards.json)
//!
//! <Piercing> <Security A. +1> <Partition (green Lv.5 & blue Lv.5)>
//! [On Play] [When Digivolving] Suspend 5 of your opponent's Digimon or
//! Tamers. None of your opponent's Digimon can unsuspend in their next
//! unsuspend phase. Then, this Digimon may attack.
//! [Your Turn] [Once Per Turn] When your opponent's security stack is removed
//! from, trash 1 of their Option cards in the battle area. Then, delete 1 of
//! their suspended Digimon or Tamers.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Green/BT23_047.cs
//!
//! # Patterns this test covers
//! - Multi-suspend (select_count_capped_multi, clamp_to_available)
//! - CannotUnsuspend aura over opponent Digimon (F7)
//! - may_attack_now ("this Digimon may attack", PASS exposed)
//! - Opponent-security-removed observer → opponent Option trash + suspended delete
//! - Kept: Piercing / Security A. +1 / Partition

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::trigger_context::EventCause;

const CARD_ID: &str = "BT23-047";

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card.play_cost = 4;
    card
}

fn opp_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

fn opp_option(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card.play_cost = 2;
    card
}

fn examon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT23-047 must load from embedded DSL pack")
}

fn fire_on_play(runner: &mut DebugRunner, source: digimon_engine::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

/// Fire `OnOpponentSecurityRemoved`: the opponent (player 1) had their security
/// removed; player 0 (Examon's controller) is the observer.
fn fire_opp_security_removed(runner: &mut DebugRunner) {
    let removed = runner.game.players[1]
        .security
        .first()
        .map(|c| c.handle())
        .unwrap_or(digimon_engine::card_source::CardHandle(0));
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 1,
            observer_player: 0,
            source_player: 0,
            card: removed,
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();
}

// ─── Section 1: Structural assertions (kept keywords + new clauses) ──────────

#[test]
fn bt23_047_keeps_partition_green_lv5_and_blue_lv5() {
    let runner = examon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let partition = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Partition {
            scope: CompiledScope::FaceUp,
            sources,
            ..
        }) => Some(sources),
        _ => None,
    });
    let sources = partition.expect("BT23-047 Partition clause");
    assert_eq!(sources.len(), 2);
    assert!(sources
        .iter()
        .any(|p| p.level_eq == Some(5) && p.color_is == Some(CompiledColor::Green)));
    assert!(sources
        .iter()
        .any(|p| p.level_eq == Some(5) && p.color_is == Some(CompiledColor::Blue)));
}

#[test]
fn bt23_047_keeps_piercing_and_security_attack() {
    let runner = examon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "Piercing"
        )),
        "Piercing kept"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "SecurityAttackPlus"
        )),
        "Security A. +1 kept"
    );
}

#[test]
fn bt23_047_has_on_play_when_digivolving_suspend_clause() {
    let runner = examon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("On Play / When Digivolving clause");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::SelectCountCappedMulti { .. })),
        "suspend clause must use a count-capped multi-select (suspend up to 5)"
    );
}

#[test]
fn bt23_047_has_your_turn_opt_security_removed_clause() {
    let runner = examon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnOpponentSecurityRemoved) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OnOpponentSecurityRemoved clause");
    assert!(clause.once_per_turn, "the [Your Turn] clause is OPT");
}

// ─── Section 2: [On Play] suspend 5 + unsuspend lock + may attack ────────────

/// Suspend up to 5 opponent Digimon/Tamers; with 3 present, all 3 are suspended
/// (clamp_to_available), and the opponent's Digimon get CannotUnsuspend.
#[test]
fn bt23_047_on_play_suspends_opponents_and_locks_digimon_unsuspend() {
    let mut runner = examon_runner()
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(opp_digimon("OPP-B", 6000))
        .add_card(opp_tamer("OPP-T"))
        .memory(20)
        .start();
    let examon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    let opp_t = runner.place_on_field(1, "OPP-T", Some(0));
    runner.game.turn_count = 1;

    fire_on_play(&mut runner, examon);

    // Multi-suspend selection over opponent Digimon + Tamers.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "suspend selection must install"
    );
    runner
        .execute_action(0, encode_attack(0, opp_a.index as u16))
        .expect("suspend A");
    runner
        .execute_action(0, encode_attack(0, opp_b.index as u16))
        .expect("suspend B");
    runner
        .execute_action(0, encode_attack(0, opp_t.index as u16))
        .expect("suspend Tamer");

    assert!(runner.game.players[1].battle_area[opp_a.index as usize].is_suspended);
    assert!(runner.game.players[1].battle_area[opp_b.index as usize].is_suspended);
    assert!(runner.game.players[1].battle_area[opp_t.index as usize].is_suspended);

    // The opponent's Digimon (not the Tamer) get CannotUnsuspend.
    assert!(
        runner
            .game
            .modifiers
            .has(opp_a, ModifierType::CannotUnsuspend),
        "opponent Digimon must get CannotUnsuspend"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(opp_b, ModifierType::CannotUnsuspend),
        "opponent Digimon must get CannotUnsuspend"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_t, ModifierType::CannotUnsuspend),
        "the unsuspend lock targets Digimon only, not Tamers"
    );

    // "Then, this Digimon may attack." — an optional attack prompt for Examon.
    let view = runner
        .pending_selection_view()
        .expect("may-attack prompt must install");
    assert!(
        view.is_optional,
        "the 'may attack' prompt must be optional (PASS exposed)"
    );
}

/// With no opponent Digimon/Tamers, the suspend no-ops but the may-attack
/// prompt still surfaces (Examon may attack regardless).
#[test]
fn bt23_047_on_play_may_attack_even_with_no_suspend_targets() {
    let mut runner = examon_runner().memory(20).start();
    let examon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.turn_count = 1;

    fire_on_play(&mut runner, examon);

    let view = runner
        .pending_selection_view()
        .expect("may-attack prompt must install even with no suspend targets");
    assert!(view.is_optional, "may-attack is optional");
}

// ─── Section 3: [Your Turn][OPT] opponent-security-removed tail ──────────────

/// On the controller's turn, when the opponent's security is removed: trash 1
/// opponent Option, then delete 1 suspended opponent Digimon/Tamer.
#[test]
fn bt23_047_security_removed_trashes_option_then_deletes_suspended() {
    let mut runner = examon_runner()
        .add_card(opp_option("OPP-OPT"))
        .add_card(opp_digimon("OPP-SUSP", 3000))
        .security(1, &["OPP-SUSP", "OPP-SUSP"])
        .memory(20)
        .start();
    let _examon = runner.place_on_field(0, CARD_ID, Some(0));
    let opt = runner.place_on_field(1, "OPP-OPT", Some(0));
    let susp = runner.place_on_field(1, "OPP-SUSP", Some(0));
    runner.game.players[1].battle_area[susp.index as usize].is_suspended = true;
    runner.game.turn_count = 1; // player 0's turn

    let opp_field_before = runner.game.players[1].battle_area.len();

    fire_opp_security_removed(&mut runner);

    // First: trash the opponent's Option (mandatory pick when one exists).
    let view = runner
        .pending_selection_view()
        .expect("opponent Option trash prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(view.selecting_player, encode_attack(0, opt.index as u16))
        .expect("trash opponent Option");
    let _ = runner.auto_resolve();

    // After resolution, the Option is gone and the suspended Digimon was deleted.
    assert!(
        runner.game.players[1].battle_area.len() < opp_field_before,
        "the opponent's Option and suspended Digimon must both be removed"
    );
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-OPT"),
        "the opponent's Option card must be trashed"
    );
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-SUSP"),
        "the suspended opponent Digimon must be deleted"
    );
}

/// Negative: the [Your Turn] clause must not fire on the opponent's turn.
#[test]
fn bt23_047_security_removed_clause_does_not_fire_on_opponents_turn() {
    let mut runner = examon_runner()
        .add_card(opp_option("OPP-OPT"))
        .security(1, &["OPP-OPT"])
        .memory(20)
        .start();
    let _examon = runner.place_on_field(0, CARD_ID, Some(0));
    let _opt = runner.place_on_field(1, "OPP-OPT", Some(0));
    runner.game.turn_count = 1;
    runner.end_turn(); // -> opponent's (player 1's) turn
    if runner.game_over() {
        return;
    }

    let field_before = runner.game.players[1].battle_area.len();
    fire_opp_security_removed(&mut runner);

    // On the opponent's turn the [Your Turn] clause is inert — no Option trash.
    assert!(
        runner.pending_selection().is_none(),
        "the [Your Turn] clause must not install a selection on the opponent's turn"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        field_before,
        "no opponent permanent should be removed on the opponent's turn"
    );
}
