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
//! ## DCGO crosscheck — clause optionality
//!
//! - `[On Play][When Digivolving]` "**By trashing** your top security card,
//!   all of your opponent's Digimon get -6000 DP for the turn":
//!   `SetUpActivateClass(..., isOptional: TRUE, ...)` at `BT23_035.cs:77`
//!   (On Play) and `:106` (When Digivolving) — both share
//!   `SharedActivateCoroutine`, which runs `IDestroySecurity` BEFORE the
//!   debuff. `general_rule.pdf` §15-7-1 names "by X, Y" an OPTIONAL
//!   PROCESSING CONDITION; §15-7-4 lets the player decline it; §15-7-2 makes
//!   the decline skip the post-condition debuff too. → clause carries
//!   `optional: true` + `outer_prompt: true` (the body's first step
//!   `trash_top_security` is a bare cost with no PASS of its own).
//! - `[All Turns][Once Per Turn]` security-removed clause:
//!   `SetUpActivateClass(..., isOptional: FALSE, ...)` at `BT23_035.cs:135`,
//!   and its printed text carries no "by X, Y" — it stays MANDATORY. Do not
//!   add `optional:` there.
//!
//! # Patterns this test covers
//! - D4 Barrier keyword (kept)
//! - On Play/When Digivolving security-trash + board-wide -6000 DP debuff (kept)
//! - E2 optional processing condition ("by trashing your top security card")
//!   with an explicit decline path (15-7-1 / 15-7-2 / 15-7-4)
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
        .expect("BT23-035 must keep its [On Play][When Digivolving] debuff clause");

    // 15-7-1: "Optional processing conditions include text such as 'by X,
    // Y.'" — "BY TRASHING your top security card, all of your opponent's
    // Digimon get -6000 DP for the turn" is that shape. 15-7-4: the player
    // "can choose whether or not to execute" it. DCGO agrees:
    // BT23_035.cs:77 / :106 pass isOptional=true.
    assert!(
        clause.optional,
        "the security-trash cost is an optional processing condition (15-7-1/15-7-4)"
    );
    // The body's first step is a bare `trash_top_security` — it exposes no
    // PASS of its own, so the forced outer confirm is the only decline gate.
    assert!(
        clause.outer_prompt,
        "a bare cost first step needs the forced outer accept/decline confirm"
    );
}

/// The [All Turns][Once Per Turn] security-removed clause prints NO "by X, Y"
/// and DCGO passes `isOptional: FALSE` (`BT23_035.cs:135`) — it must stay
/// MANDATORY. Guards against over-applying the 15-7 optionality fix and
/// exposing an illegal PASS on a mandatory trigger.
#[test]
fn bt23_035_security_removed_clause_stays_mandatory() {
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
        !clause.optional,
        "no 'by X, Y' in the printed text and DCGO isOptional=false -> mandatory"
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
        !runner
            .game
            .has_keyword(dyna, Keyword::SecurityAttackPlus(1)),
        "Dynasmon must not have Security A. +1 before the trigger"
    );
    let sec_before = runner.security_count(0);

    fire_own_security_removed(&mut runner);
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .has_keyword(dyna, Keyword::SecurityAttackPlus(1)),
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
        runner
            .game
            .has_keyword(dyna, Keyword::SecurityAttackPlus(1)),
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
        runner
            .game
            .has_keyword(dyna, Keyword::SecurityAttackPlus(1)),
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
        !runner
            .game
            .has_keyword(dyna, Keyword::SecurityAttackPlus(1)),
        "the Security A. +1 grant must expire after your turn ends"
    );
}

// ─── Section 3: [On Play][When Digivolving] optional security-trash (15-7) ───

/// Dynasmon on P0's field, 5 security cards, and two opponent Digimon (9000 /
/// 7000 DP — both survive the -6000, so the debuff is observable without the
/// 0-DP rule-check deletion muddying the assertion).
///
/// FIVE security, not three, and that matters: Dynasmon's OWN third clause is
/// "[All Turns][Once Per Turn] When your security stack is removed from, ...
/// Then, if you have 3 or fewer security cards, ＜Recovery +1 (Deck)＞". Paying
/// clause 2's security-trash cost REMOVES from the stack, which fires clause 3;
/// starting at 3 the Recovery gate (security_count_lte: 3) is satisfied and
/// puts the card straight back, so the stack reads 3 -> 2 -> 3 and a
/// "trashed exactly 1" assertion fails against correct engine behaviour.
/// Starting at 5 keeps the gate shut so the cost is observable on its own.
fn debuff_runner() -> (
    DebugRunner,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
) {
    let mut runner = dynasmon_runner()
        .add_card(opponent_digimon("OPP-A", 9000))
        .add_card(opponent_digimon("OPP-B", 7000))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(12)
        .start();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    let dyna = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, dyna, opp_a, opp_b)
}

fn fire_timing(
    runner: &mut DebugRunner,
    timing: EffectTiming,
    dyna: digimon_engine::permanent::PermanentHandle,
) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(dyna));
    runner.game.drain_effect_queue();
}

/// ACCEPT: paying the optional processing condition trashes exactly 1 security
/// card and applies -6000 DP to every opponent Digimon.
#[test]
fn bt23_035_accepting_security_trash_applies_board_debuff() {
    let (mut runner, dyna, opp_a, opp_b) = debuff_runner();
    let sec_before = runner.security_count(0);
    let dp_a_before = runner.effective_dp(opp_a).expect("OPP-A has DP");
    let dp_b_before = runner.effective_dp(opp_b).expect("OPP-B has DP");

    fire_timing(&mut runner, EffectTiming::OnPlay, dyna);

    // 15-7-4: the choice must surface BEFORE the security card is trashed.
    let outer = runner
        .pending_selection_view()
        .expect("the optional processing condition must surface a prompt (rule 17)");
    assert!(
        outer.is_optional,
        "the outer confirm must expose PASS (15-7-4)"
    );
    assert_eq!(
        runner.security_count(0),
        sec_before,
        "no security card may be trashed before the player accepts"
    );

    runner
        .accept_optional_trigger()
        .expect("accepting the security-trash cost must be reachable");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        sec_before - 1,
        "accepting trashes exactly 1 security card"
    );
    assert_eq!(
        runner.effective_dp(opp_a),
        Some(dp_a_before - 6000),
        "every opponent Digimon gets -6000 DP"
    );
    assert_eq!(
        runner.effective_dp(opp_b),
        Some(dp_b_before - 6000),
        "every opponent Digimon gets -6000 DP"
    );
}

/// DECLINE: 15-7-1 names "By trashing your top security card, ..." an OPTIONAL
/// PROCESSING CONDITION and 15-7-4 says "A player can choose whether or not to
/// execute the content of optional processing conditions, REGARDLESS of whether
/// or not the content of the conditions can be executed" — so merely having
/// security to trash does not force the payment. 15-7-2: with the condition's
/// content unexecuted, "the processing after the conditions can't be executed",
/// so BOTH halves must be skipped — the security stack is untouched AND no
/// opponent Digimon loses DP.
///
/// DCGO agrees: `BT23_035.cs:77` / `:106` pass `isOptional: true`.
///
/// This is the branch the engine had no way to reach — the clause fired
/// unconditionally, auto-trashing a security card (rule 17: no
/// auto-selections).
#[test]
fn bt23_035_declining_security_trash_skips_cost_and_debuff() {
    let (mut runner, dyna, opp_a, opp_b) = debuff_runner();
    let sec_before = runner.security_count(0);
    let dp_a_before = runner.effective_dp(opp_a).expect("OPP-A has DP");
    let dp_b_before = runner.effective_dp(opp_b).expect("OPP-B has DP");

    fire_timing(&mut runner, EffectTiming::OnPlay, dyna);

    assert!(
        runner.pending_selection().is_some(),
        "the optional processing condition must prompt before the decline test"
    );
    runner
        .decline_optional_trigger()
        .expect("declining must be reachable from the action space");
    let _ = runner.auto_resolve();

    // 15-7-2, half 1: the COST was not paid.
    assert_eq!(
        runner.security_count(0),
        sec_before,
        "declining must NOT trash a security card"
    );

    // 15-7-2, half 2: "the processing after the conditions can't be executed".
    assert_eq!(
        runner.effective_dp(opp_a),
        Some(dp_a_before),
        "declining must NOT apply the -6000 DP debuff"
    );
    assert_eq!(
        runner.effective_dp(opp_b),
        Some(dp_b_before),
        "declining must NOT apply the -6000 DP debuff"
    );
    assert!(
        runner.pending_selection().is_none(),
        "declining must resolve cleanly with no leftover prompt"
    );
}

/// The same choice is offered on the [When Digivolving] timing (DCGO shares one
/// `SharedActivateCoroutine` across both, each with `isOptional: true`).
#[test]
fn bt23_035_when_digivolving_security_trash_may_be_declined() {
    let (mut runner, dyna, opp_a, _opp_b) = debuff_runner();
    let sec_before = runner.security_count(0);
    let dp_a_before = runner.effective_dp(opp_a).expect("OPP-A has DP");

    fire_timing(&mut runner, EffectTiming::WhenDigivolving, dyna);

    runner
        .decline_optional_trigger()
        .expect("[When Digivolving] must also expose the decline");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        sec_before,
        "declining on [When Digivolving] must not trash security"
    );
    assert_eq!(
        runner.effective_dp(opp_a),
        Some(dp_a_before),
        "declining on [When Digivolving] must not apply the debuff"
    );
}

/// With an EMPTY security stack the cost cannot be paid at all, so DCGO's
/// `CanActivateCondition` (`SecurityCards.Count >= 1`, `BT23_035.cs:94` /
/// `:123`) never lets the clause activate — no prompt, no debuff. The clause's
/// `condition: { security_count_gte: 1 }` reproduces that, which is what keeps
/// the forced outer confirm from appearing in a state DCGO never reaches.
#[test]
fn bt23_035_no_prompt_and_no_debuff_with_empty_security() {
    let mut runner = dynasmon_runner()
        .add_card(opponent_digimon("OPP-A", 9000))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(12)
        .start();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let dyna = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].security.clear();
    let dp_before = runner.effective_dp(opp_a).expect("OPP-A has DP");

    fire_timing(&mut runner, EffectTiming::OnPlay, dyna);

    assert!(
        runner.pending_selection().is_none(),
        "with no security to trash the clause must not activate or prompt"
    );
    assert_eq!(
        runner.effective_dp(opp_a),
        Some(dp_before),
        "with the cost unpayable the -6000 DP debuff must not apply"
    );
}
