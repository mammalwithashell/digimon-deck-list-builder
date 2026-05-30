//! ST-3 Heaven's Yellow — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st3-heavens-yellow-model.md`. The deck's thesis is a
//! single repeatable engine: **stack -DP reduction effects to drop an opponent
//! Digimon to 0 DP → the 0-DP rule check (general_rule.pdf 17-1-3-1) deletes it
//! → inherited "0-DP deletion" observers (Tokomon/Patamon) refund the carrier.**
//! These interaction tests pin that engine as a *system* — the deletion that
//! one card causes and the observer payoff a *different* inherited source reads,
//! plus the survival half (Angewomon recovery gate + T.K.'s security aura) — the
//! cross-card facts no per-card behavioral test in
//! `tests/cards_behavioral/st3/` can see.
//!
//! Sources, per test:
//! - Printed text: `code/digimon-engine/cards/st3/<ID>.json` + `.yaml`.
//! - DCGO C#: `$BASE_DCGO/Assets/Scripts/CardEffect/ST3/Yellow/ST3_<NN>.cs`.
//! - Rules: `Digimon TCG resources/general_rule.pdf` §17 rule checks
//!   (**17-1-3-1** "a 0-DP Digimon in the battle area is deleted"; **17-1-2-2**
//!   "rule checks are NOT performed during effect processing" — the -DP lands
//!   during processing and the deletion + observer fire at the *following* rule
//!   check, not mid-effect); §16 `[When Attacking]` keyword timing; glossary
//!   `<Recovery>`.

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

// ─── Combo-piece fixtures ────────────────────────────────────────────────────

/// A neutral Yellow Digimon body — used as the *carrier* under a real inherited
/// observer source (its identity is irrelevant; only its base DP / level / color
/// matter), and as a neutral opponent target. Synthetic, never a combo piece in
/// its own right.
fn yellow_digimon(id: &str, dp: i32, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = level as u16 + 1;
    card.colors = vec![CardColor::Yellow];
    card
}

/// Fire a stack's `[When Digivolving]` triggered effects directly. `place_stack`
/// builds the stack without firing the digivolve trigger, so the model of
/// "digivolved into Angewomon" is the explicit enqueue + drain.
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Mark a permanent suspended — a Digimon can only be *attacked* while suspended
/// (rules 17 / combat), so a Digimon-vs-Digimon attack target must be set up
/// suspended.
fn suspend(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize].is_suspended =
        true;
}

// ─── Combo A — Seven Heavens → 0-DP deletion → Tokomon carrier +1000 ─────────

/// **Combo A (model rank 1).** Cards: ST3-16 Seven Heavens (`[Main]` -10000 DP
/// to 1 opp Digimon) + ST3-01 Tokomon (inherited `[Your Turn][OPT]`: when an
/// opp Digimon is deleted by dropping to 0 DP, this carrier +1000 DP for the
/// turn).
///
/// Expected outcome: Seven Heavens lands -10000 on a 7000-DP opponent Digimon →
/// at the following rule check (17-1-2-2 / 17-1-3-1) the now-(-3000)-DP Digimon
/// is deleted (opp field −1, opp trash +1). That 0-DP deletion is observed by
/// the Tokomon source under our carrier, raising the carrier from base 4000 to
/// 5000 DP. The two halves come from two *different* cards — the system fact.
///
/// Sources: ST3-16.yaml (`add_dp_modifier -10000`), ST3-01.yaml (inherited
/// `on_any_deletion` gated `event_target_owner: opponent` + `event_target_dp_lte:
/// 0` + `your_turn` + OPT → carrier `add_dp_modifier +1000`); DCGO `ST3_16.cs`
/// `ChangeDigimonDP(-10000)`, `ST3_01.cs` `IsDPZeroDelete` gate → +1000; rules
/// 17-1-3-1 / 17-1-2-2.
#[test]
fn seven_heavens_zero_dp_deletion_buffs_tokomon_carrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-16")
        .expect("ST3-16 (Seven Heavens) in embedded DSL pack")
        .dsl_card("ST3-01")
        .expect("ST3-01 (Tokomon) in embedded DSL pack")
        .add_card(yellow_digimon("CARRIER", 4000, 4))
        .add_card(yellow_digimon("OPP-7000", 7000, 5))
        .hand(0, &["ST3-16"])
        .memory(0)
        .start();

    // Carrier with a Tokomon inherited source underneath — the 0-DP observer.
    let carrier = runner.place_stack(0, &["ST3-01", "CARRIER"]);
    runner.place_on_field(1, "OPP-7000", Some(0));

    let opp_field_before = runner.battle_area_size(1);
    let opp_trash_before = runner.trash_size(1);

    // Play Seven Heavens from hand and resolve its -10000 target selection
    // (only one opponent Digimon exists, so the choice is forced).
    assert!(
        runner.game.activate_hand_main(0, 0),
        "Seven Heavens [Main] should be activatable from hand"
    );
    runner
        .auto_resolve()
        .expect("Seven Heavens -10000 target selection resolves");

    // Half 1 — the 7000-DP Digimon dropped to -3000 and was deleted by the
    // 0-DP rule check.
    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before - 1,
        "the -10000 target should be deleted at the 0-DP rule check (17-1-3-1)"
    );
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before + 1,
        "the deleted opponent Digimon lands in its owner's trash"
    );

    // Half 2 — a *different* card (the Tokomon inherited source) observed that
    // 0-DP deletion and buffed the carrier +1000 for the turn.
    assert_eq!(
        runner.effective_dp(carrier),
        Some(5000),
        "the Tokomon-source observer raises the carrier from 4000 to 5000 DP \
         on the opponent's 0-DP deletion (your turn, OPT)"
    );
}

/// **Combo A — the gate.** The Tokomon observer must fire ONLY on a deletion
/// *by dropping to 0 DP*, not on every opponent-Digimon deletion. Here Seven
/// Heavens lands -10000 on a 12000-DP Digimon: it survives at 2000 DP (no
/// deletion at all), so the observer never fires and the carrier stays at base
/// 4000. This pins the "deleted *specifically* by 0 DP" half of the gate — the
/// no-fire control the rank-1 combo claims (`event_target_dp_lte: 0`).
#[test]
fn tokomon_carrier_unbuffed_when_no_zero_dp_deletion_occurs() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-16")
        .expect("ST3-16 (Seven Heavens) in embedded DSL pack")
        .dsl_card("ST3-01")
        .expect("ST3-01 (Tokomon) in embedded DSL pack")
        .add_card(yellow_digimon("CARRIER", 4000, 4))
        .add_card(yellow_digimon("OPP-12000", 12000, 6))
        .hand(0, &["ST3-16"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["ST3-01", "CARRIER"]);
    runner.place_on_field(1, "OPP-12000", Some(0));

    let opp_field_before = runner.battle_area_size(1);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .auto_resolve()
        .expect("Seven Heavens -10000 target selection resolves");

    // -10000 on 12000 leaves 2000 DP — no deletion, so no 0-DP event.
    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before,
        "a 12000-DP Digimon survives -10000 at 2000 DP — no 0-DP deletion"
    );
    assert_eq!(
        runner.effective_dp(carrier),
        Some(4000),
        "with no 0-DP deletion the Tokomon observer must NOT fire — carrier stays at base 4000"
    );
}

// ─── Combo B — stacked -DP attack → 0-DP delete → Patamon +1 memory ──────────

/// **Combo B (model rank 2).** Build the archetype's signature attacker stack:
/// Seraphimon (ST3-11, `[When Attacking]` -4000) over MagnaAngemon (ST3-08
/// inherited, `[When Attacking]` -1000) over Patamon (ST3-04 inherited,
/// `[Your Turn][OPT]`: on an opp 0-DP deletion, gain 1 memory). The stack
/// `[Seraphimon-over-MagnaAngemon-over-Patamon]` attacks a 5000-DP suspended
/// opponent Digimon.
///
/// Expected outcome: on the attack, BOTH `[When Attacking]` effects in the stack
/// trigger on the same attack — -4000 (Seraphimon, own) + -1000 (MagnaAngemon,
/// inherited) = -5000 — dropping the 5000-DP defender to 0 → deleted at the rule
/// check (17-1-3-1). The Patamon inherited source observes that 0-DP deletion
/// and gains exactly +1 memory (OPT). Verifies inherited -DP *stacking* on one
/// attack + the second observer payoff + once-per-turn semantics — the parts
/// Combo A doesn't touch.
///
/// Sources: ST3-11.yaml (-4000 when attacking), ST3-08.yaml (inherited -1000
/// when attacking), ST3-04.yaml (inherited `on_any_deletion` 0-DP gate →
/// `gain_memory: 1`, OPT); DCGO `ST3_11.cs` / `ST3_08.cs` / `ST3_04.cs`; rules
/// §16 `[When Attacking]`, 17-1-3-1.
///
/// NOTE — this is the first test to drive the 0-DP deletion through the *full*
/// `attack_digimon` pipeline (the per-card ST3-08/ST3-11 tests only fire
/// `[When Attacking]` directly and assert `effective_dp`, never the deletion).
/// If it goes red while Combo A (the `activate_hand_main` path, proven by the
/// per-card ST3-01/ST3-04 tests) stays green, the gap is most likely that the
/// post-`[When Attacking]` 0-DP rule check / observer fire isn't run inside the
/// attack flow — route that to `docs/RUST_ENGINE_GAPS.md`, don't weaken the
/// assertion.
#[test]
fn stacked_minus_dp_attack_zero_dp_deletion_gains_patamon_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-11")
        .expect("ST3-11 (Seraphimon) in embedded DSL pack")
        .dsl_card("ST3-08")
        .expect("ST3-08 (MagnaAngemon) in embedded DSL pack")
        .dsl_card("ST3-04")
        .expect("ST3-04 (Patamon) in embedded DSL pack")
        .add_card(yellow_digimon("OPP-5000", 5000, 4))
        .memory(0)
        .start();

    // Attacker stack: Patamon (observer) under MagnaAngemon (-1000 inherited)
    // under Seraphimon (-4000, top). place_stack sets turn_played=0 so the
    // attacker is not summoning-sick on turn 1.
    let attacker = runner.place_stack(0, &["ST3-04", "ST3-08", "ST3-11"]);
    let defender = runner.place_on_field(1, "OPP-5000", Some(0));
    // A Digimon can only be attacked while suspended.
    suspend(&mut runner, defender);

    let memory_before = runner.memory();
    let opp_field_before = runner.battle_area_size(1);

    // Real attack: fires the stack's [When Attacking] triggers (-4000 + -1000),
    // then the rule check deletes the 0-DP defender.
    let result = runner.attack_digimon(attacker, defender, false);
    // Both when-attacking selections have exactly one legal target, so resolve
    // them and let the attack pipeline finish.
    let _ = runner.auto_resolve();

    // The defender dropped to 0 DP and was deleted at the rule check.
    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before - 1,
        "the 5000-DP defender, stacked down -5000 to 0 DP, must be deleted (17-1-3-1); \
         attack result was {result:?}"
    );

    // The Patamon inherited source observed the 0-DP deletion: exactly +1 memory
    // (OPT — a single deletion fires it once).
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "the Patamon-source observer gains exactly 1 memory on the opponent's 0-DP deletion"
    );
}

// ─── Combo C — Angewomon recovery gate + T.K. security aura ──────────────────

/// **Combo C (model rank 3), part (a) — recovery gate, ≤3 security.** ST3-09
/// Angewomon `[When Digivolving]`: if controller has ≤3 security, `<Recovery +1
/// (Deck)>` (place top of deck onto security). At exactly 3 security the gate
/// holds → security goes 3 → 4 and the deck shrinks by 1.
///
/// Sources: ST3-09.yaml (`when_digivolving`, `security_count_lte: 3` → `recover
/// {of: you, count: 1}`); DCGO `ST3_09.cs` (`SecurityCards.Count <= 3` →
/// `IRecovery(1)`); glossary `<Recovery +(Deck)>`.
#[test]
fn angewomon_recovers_when_three_or_fewer_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-09")
        .expect("ST3-09 (Angewomon) in embedded DSL pack")
        .add_card(yellow_digimon("BASE", 5000, 4))
        .add_card(yellow_digimon("SEC", 3000, 3))
        .add_card(yellow_digimon("DECKTOP", 3000, 3))
        .security(0, &["SEC", "SEC", "SEC"])
        .deck(0, &["DECKTOP", "DECKTOP"])
        .memory(0)
        .start();
    let angewomon = runner.place_stack(0, &["BASE", "ST3-09"]);

    let security_before = runner.security_count(0);
    let deck_before = runner.deck_size(0);
    assert_eq!(security_before, 3, "precondition: at exactly the ≤3 gate");

    fire_when_digivolving(&mut runner, angewomon);
    runner.auto_resolve().expect("Angewomon recovery resolves");

    assert_eq!(
        runner.security_count(0),
        security_before + 1,
        "Angewomon recovers +1 security when the controller has ≤3 security"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "the recovered card comes off the top of the deck"
    );
}

/// **Combo C, part (a) — recovery gate, 4+ security (no-fire control).** The
/// SAME digivolve at 4 security must NOT recover (security and deck unchanged) —
/// the `security_count_lte: 3` gate is the whole point of the card, so this is
/// the negative half the model calls for.
#[test]
fn angewomon_does_not_recover_at_four_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-09")
        .expect("ST3-09 (Angewomon) in embedded DSL pack")
        .add_card(yellow_digimon("BASE", 5000, 4))
        .add_card(yellow_digimon("SEC", 3000, 3))
        .add_card(yellow_digimon("DECKTOP", 3000, 3))
        .security(0, &["SEC", "SEC", "SEC", "SEC"])
        .deck(0, &["DECKTOP", "DECKTOP"])
        .memory(0)
        .start();
    let angewomon = runner.place_stack(0, &["BASE", "ST3-09"]);

    let security_before = runner.security_count(0);
    let deck_before = runner.deck_size(0);
    assert_eq!(security_before, 4, "precondition: above the ≤3 gate");

    fire_when_digivolving(&mut runner, angewomon);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        security_before,
        "Angewomon must NOT recover when the controller already has 4+ security"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "no card leaves the deck when the recovery gate fails"
    );
}

/// **Combo C, part (b) — T.K.'s Security-Digimon aura, opponent's turn only.**
/// ST3-12 T.K. Takaishi `[Opponent's Turn]`: your **Security Digimon** +2000 DP.
/// We sit T.K. on player 0's field, give player 0 a 7000-DP Digimon in security,
/// and have player 1 attack player 0's security *on player 1's turn* (= player
/// 0's opponent's turn). With the aura active, that security Digimon defends as
/// 9000 — enough to delete the 8000-DP attacker. The aura is window-gated: it is
/// `active_when: opponents_turn`, so the result is observable only while it is
/// the opponent's turn (mirrors the per-card test's combat-resolved assertion,
/// here driven from player 0's perspective as the security owner).
///
/// Sources: ST3-12.yaml (aura `active_when: opponents_turn`, `dp_modifier: 2000`,
/// `applies_to_own_security_dp: true`); DCGO `ST3_12.cs`
/// `ChangeSecurityDigimonCardDPStaticEffect(+2000)` gated `IsOpponentTurn`.
#[test]
fn tk_takaishi_buffs_own_security_digimon_only_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-12")
        .expect("ST3-12 (T.K. Takaishi) in embedded DSL pack")
        .add_card(yellow_digimon("ATK", 8000, 5))
        .add_card(yellow_digimon("SEC", 7000, 4))
        // Player 0 (T.K.'s controller) defends with a 7000-DP Security Digimon.
        .security(0, &["SEC"])
        .start();
    // P1 is the attacker; P0 controls T.K. and the defending security stack.
    let attacker = runner.place_on_field(1, "ATK", Some(0));
    runner.place_on_field(0, "ST3-12", Some(0));
    // Player 1's turn → player 0's *opponent's* turn → T.K.'s aura is active.
    runner.game.turn_player_idx = 1;

    let result = runner.attack_player(attacker, 0, false);

    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "on player 1's turn T.K. raises P0's 7000-DP Security Digimon to 9000, \
         deleting the 8000-DP attacker"
    );
}

/// **Combo C, part (b) — aura window control, own turn.** The SAME board, but
/// now it is player 0's *own* turn, so T.K.'s `[Opponent's Turn]` aura must be
/// inactive: the 7000-DP Security Digimon should defend at base 7000 and lose to
/// the 8000-DP attacker (which survives the security check). Pins that the +2000
/// is gated strictly to the opponent's turn, not a permanent aura.
///
/// FLAGGED — likely fails against the current engine (potential faithfulness
/// bug, not a flaky test). `ST3-12.yaml` correctly encodes the aura with
/// `active_when: { opponents_turn: true }`, and the lowering in
/// `src/dsl_cards/lower_aura.rs` attaches that as the Effect's `.condition(...)`.
/// BUT the security-check call site
/// `Game::defender_security_dp_adjustment` (`src/combat.rs:317-329`) sums
/// `effect.dp_modifier` for every effect with `applies_to_own_security_dp`
/// WITHOUT evaluating the effect's `condition` — so the +2000 appears to apply
/// on the controller's own turn too. The per-card test only exercises the ON
/// (opponent's-turn) case, so it can't catch this. If this test goes red, route
/// it to `docs/RUST_ENGINE_GAPS.md` (engine primitive: the security-DP
/// adjustment must honor each aura effect's `active_when` condition) — do NOT
/// weaken the assertion to the buggy behavior.
#[test]
fn tk_takaishi_aura_is_inactive_on_controllers_own_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-12")
        .expect("ST3-12 (T.K. Takaishi) in embedded DSL pack")
        .add_card(yellow_digimon("ATK", 8000, 5))
        .add_card(yellow_digimon("SEC", 7000, 4))
        .security(0, &["SEC"])
        .start();
    let attacker = runner.place_on_field(1, "ATK", Some(0));
    runner.place_on_field(0, "ST3-12", Some(0));
    // Player 0's own turn — T.K.'s [Opponent's Turn] aura is OFF.
    runner.game.turn_player_idx = 0;

    let result = runner.attack_player(attacker, 0, false);

    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "on P0's own turn T.K.'s aura is inactive, so the 7000-DP Security Digimon \
         cannot stop the 8000-DP attacker"
    );
}
