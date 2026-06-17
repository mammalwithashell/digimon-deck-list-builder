//! BT23-035 Dynasmon — Digimon, Lv.6, Yellow/Red, DP 12000, Cost 12.
//!
//! # Card text (image + cards.json)
//!
//! <Barrier> (When this Digimon would be deleted in battle, by trashing the
//! top card of your security stack, prevent that deletion.)
//! [On Play] [When Digivolving] By trashing your top security card, all of
//! your opponent's Digimon get -6000 DP for the turn.
//! [All Turns] [Once Per Turn] When your security stack is removed from, this
//! Digimon gains <Security A. +1> until your turn ends. Then, if you have 3 or
//! fewer security cards, <Recovery +1 (Deck)>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Yellow/BT23_035.cs
//!
//! # Patterns this test covers
//! - D4 Barrier keyword (kept)
//! - On Play/When Digivolving security-trash + board-wide -6000 DP debuff (kept)
//! - H4 Security A. +1 grant with `end_of_your_next_turn` expiry
//! - Security-removed observer (OnOwnSecurityRemoved) + conditional Recovery

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::selection::TriggerSource;
use digimon_engine::trigger_context::EventCause;

const CARD_ID: &str = "BT23-035";

fn opponent_digimon(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card.play_cost = 4;
    card
}

fn dynasmon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT23-035 must load from embedded DSL pack")
}

/// Fire the `OnOwnSecurityRemoved` observer exactly as the engine's combat
/// fire-site builds it: player 0 is both the affected player (their security
/// stack is removed from) and the observer (Dynasmon's controller).
fn fire_own_security_removed(runner: &mut DebugRunner) {
    let removed_card = runner.game.players[0]
        .security
        .first()
        .map(|c| c.handle())
        .unwrap_or(digimon_engine::card_source::CardHandle(0));
    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: removed_card,
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn bt23_035_loads() {
    let runner = dynasmon_runner().start();
    assert!(runner.compiled_card(CARD_ID).is_some());
}

#[test]
fn bt23_035_keeps_barrier_keyword() {
    let runner = dynasmon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT23-035");
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "Barrier"
        )),
        "BT23-035 must keep its Barrier keyword"
    );
}

#[test]
fn bt23_035_keeps_on_play_when_digivolving_debuff() {
    let runner = dynasmon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT23-035");
    assert!(
        card.effects.iter().any(|clause| match clause {
            CompiledClause::Triggered(t) =>
                t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving),
            _ => false,
        }),
        "BT23-035 must keep its [On Play][When Digivolving] debuff clause"
    );
}

#[test]
fn bt23_035_has_security_removed_opt_clause() {
    let runner = dynasmon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT23-035");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnOwnSecurityRemoved) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT23-035 must have an OnOwnSecurityRemoved clause");
    assert!(
        clause.once_per_turn,
        "the security-removed clause is [Once Per Turn]"
    );
}

// ─── Section 2: Security-removed grant + recovery ────────────────────────────

/// With 2 security cards (≤3), the security-removed trigger grants Security
/// A. +1 to Dynasmon AND recovers 1 (security stack grows by 1).
#[test]
fn bt23_035_security_removed_grants_sec_attack_and_recovers_at_low_security() {
    let mut runner = dynasmon_runner()
        .deck(0, &[CARD_ID, CARD_ID, CARD_ID])
        .security(0, &[CARD_ID, CARD_ID])
        .memory(12)
        .start();
    let dyna = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.turn_count = 1;

    assert!(
        !runner.game.has_keyword(dyna, Keyword::SecurityAttackPlus(1)),
        "Dynasmon must not have Security A. +1 before the trigger"
    );
    let sec_before = runner.security_count(0);

    fire_own_security_removed(&mut runner);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.has_keyword(dyna, Keyword::SecurityAttackPlus(1)),
        "Dynasmon must gain Security A. +1 after security is removed from"
    );
    assert_eq!(
        runner.security_count(0),
        sec_before + 1,
        "with 3 or fewer security, Recovery +1 must add a security card"
    );
}

/// With 5 security cards (>3), the trigger grants Security A. +1 but does NOT
/// recover (security count unchanged).
#[test]
fn bt23_035_security_removed_no_recovery_above_three_security() {
    let mut runner = dynasmon_runner()
        .deck(0, &[CARD_ID, CARD_ID, CARD_ID])
        .security(0, &[CARD_ID, CARD_ID, CARD_ID, CARD_ID, CARD_ID])
        .memory(12)
        .start();
    let dyna = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.turn_count = 1;
    let sec_before = runner.security_count(0);

    fire_own_security_removed(&mut runner);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.has_keyword(dyna, Keyword::SecurityAttackPlus(1)),
        "Dynasmon must still gain Security A. +1 regardless of security count"
    );
    assert_eq!(
        runner.security_count(0),
        sec_before,
        "with more than 3 security, no Recovery happens"
    );
}

/// The granted Security A. +1 is turn-scoped ("until your turn ends"): after
/// the controller's next turn ends it must be gone.
#[test]
fn bt23_035_sec_attack_grant_expires_after_your_turn() {
    let mut runner = dynasmon_runner()
        .deck(0, &[CARD_ID, CARD_ID, CARD_ID, CARD_ID, CARD_ID])
        .deck(1, &[CARD_ID, CARD_ID, CARD_ID, CARD_ID, CARD_ID])
        .security(0, &[CARD_ID, CARD_ID])
        .memory(12)
        .start();
    let dyna = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.turn_count = 1;

    // Fire during the opponent's turn (the usual case: their attack removes
    // your security). "Until your turn ends" = end of YOUR next turn.
    runner.end_turn(); // P0 -> P1's turn
    if runner.game_over() {
        return;
    }
    fire_own_security_removed(&mut runner);
    let _ = runner.auto_resolve();
    assert!(
        runner.game.has_keyword(dyna, Keyword::SecurityAttackPlus(1)),
        "grant must be active while it persists"
    );

    runner.end_turn(); // P1 -> P0's turn
    if runner.game_over() {
        return;
    }
    runner.end_turn(); // P0 -> P1's turn (your turn has now ended once)
    if runner.game_over() {
        return;
    }
    assert!(
        !runner.game.has_keyword(dyna, Keyword::SecurityAttackPlus(1)),
        "the Security A. +1 grant must expire after your turn ends"
    );
}
