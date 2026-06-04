//! ST-3 Heaven's Yellow — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st-3-heavens-yellow-model.md`.
//!
//! System: a Yellow control deck that wins by driving opponent Digimon to **0 DP**
//! with stacked −DP modifiers (deletion by `general_rule.pdf` rule **17-1-3-1**),
//! then converting each kill via inherited [Your Turn][Once Per Turn] payoffs —
//! Patamon (ST3-04 → +1 memory) and Tokomon (ST3-01 → carrier +1000 DP) — while
//! holding behind a Security-Digimon wall (T.K. Takaishi ST3-12 + Heaven's Gate
//! security ST3-13). These tests assert the cross-card SYSTEM the per-card tests
//! in `tests/cards_behavioral/st3/st3_starter.rs` can't see.
//!
//! Per-card coverage (shapes/single-card behavior) lives in
//! `cards_behavioral/st3/st3_starter.rs`; this file asserts combined board diffs.

#![allow(dead_code)]

use digimon_engine::combat::AttackResult;
use digimon_engine::permanent::PermanentHandle;

use super::support::{dsl_builder, snapshot};

// All opponent bodies / carriers below are REAL vanilla DSL cards (zero effect
// clauses) chosen purely for their printed DP — Yellow/Vaccine is irrelevant;
// only kind/DP matter for the 0-DP deletion window:
//   ST3-02 Salamon  (yellow, Lv3, 3000)  ST3-03 Tapirmon (yellow, Lv3, 4000)
//   ST3-06 Gatomon  (yellow, Lv4, 5000)  ST3-10 Magnadramon (yellow, Lv6, 12000)
//   ST4-07 Kuwagamon (green, Lv4, 6000)  — borrowed for the 6000-DP body the
//   yellow vanilla pool lacks.

// ─── Combo 1: −DP-to-0 deletion → inherited payoff chain ─────────────────────

/// Happy path — the deck's core engine loop. Seven Heavens (ST3-16, −10000) drops
/// a 3000-DP opponent Digimon to −7000 → 0-DP deletion (rule 17-1-3-1). The
/// carrier has BOTH inherited observers stacked beneath it: Patamon (ST3-04)
/// grants +1 memory and Tokomon (ST3-01) grants the carrier +1000 DP for the turn.
/// We assert the full system: opp field −1, opp trash +1, memory +1, carrier DP
/// +1000 — all from a single deletion event.
///
/// - Card text: ST3-16 [Main] "−10000 DP"; ST3-04 INH "[Your Turn][Once Per Turn]
///   when an opp Digimon is deleted by dropping to 0 DP, gain 1 memory";
///   ST3-01 INH "...this Digimon gets +1000 DP for the turn".
/// - Rules basis: `general_rule.pdf` rule **17-1-3-1** (0-DP Digimon is deleted);
///   inherited effects resolve from digivolution sources; both observers gate on
///   `your_turn` + `once_per_turn` (default runner state is player 0's turn).
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST3/Yellow/ST3_16.cs`,
///   `ST3_04.cs`, `ST3_01.cs`.
#[test]
fn dp_zero_deletion_fires_both_inherited_observers() {
    // Carrier is real vanilla ST3-03 Tapirmon (base 4000); opp body is real
    // vanilla ST3-02 Salamon (3000, deleted by −10000).
    let mut runner = dsl_builder(&["ST3-16", "ST3-04", "ST3-01", "ST3-03", "ST3-02"])
        .hand(0, &["ST3-16"])
        .memory(7)
        .start();

    // Stack [Tokomon, Patamon, Tapirmon] — both inherited payoffs sit as
    // digivolution sources beneath the active carrier.
    let carrier_h = runner.place_stack(0, &["ST3-01", "ST3-04", "ST3-03"]);
    runner.place_on_field(1, "ST3-02", Some(0));

    let before = snapshot(&runner);
    assert!(
        runner.game.activate_hand_main(0, 0),
        "Seven Heavens [Main] must activate from hand"
    );
    runner
        .auto_resolve()
        .expect("Seven Heavens target selection resolves");
    let after = snapshot(&runner);

    // The opp Digimon was driven to −8000 (≤0) and deleted.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "the 3000-DP opponent Digimon should be deleted by Seven Heavens' −10000"
    );
    assert_eq!(
        after.trash[1],
        before.trash[1] + 1,
        "the deleted Digimon should land in the opponent's trash"
    );
    // Patamon's inherited payoff: +1 memory from the 0-DP deletion.
    assert_eq!(
        after.memory,
        before.memory + 1,
        "Patamon inherited must grant +1 memory on the opponent 0-DP deletion"
    );
    // Tokomon's inherited payoff: carrier +1000 DP for the turn (4000 → 5000).
    assert_eq!(
        runner.effective_dp(carrier_h),
        Some(5000),
        "Tokomon inherited must grant the carrier +1000 DP for the turn after the 0-DP deletion"
    );
}

/// Unhappy contrast — a −DP that does NOT reach 0 deletes nothing and fires
/// neither observer. Heaven's Charm (ST3-14, −2000) on a 5000-DP Digimon leaves
/// it at 3000: alive, no deletion event, so the inherited chain stays silent.
///
/// - Card text: ST3-14 [Main] "−2000 DP".
/// - Rules basis: the inherited observers' `event_target_dp_lte: 0` condition is
///   not met (rule 17-1-3-1 deletion never occurs).
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST3/Yellow/ST3_14.cs`.
#[test]
fn dp_reduction_not_reaching_zero_fires_no_observer() {
    // Carrier is real vanilla ST3-03 Tapirmon (4000); opp body is real vanilla
    // ST3-06 Gatomon (5000 → 3000 after −2000, survives).
    let mut runner = dsl_builder(&["ST3-14", "ST3-04", "ST3-01", "ST3-03", "ST3-06"])
        .hand(0, &["ST3-14"])
        .memory(7)
        .start();

    let carrier_h = runner.place_stack(0, &["ST3-01", "ST3-04", "ST3-03"]);
    runner.place_on_field(1, "ST3-06", Some(0));

    let before = snapshot(&runner);
    assert!(
        runner.game.activate_hand_main(0, 0),
        "Heaven's Charm [Main] must activate from hand"
    );
    runner
        .auto_resolve()
        .expect("Heaven's Charm target selection resolves");
    let after = snapshot(&runner);

    // 5000 − 2000 = 3000: the Digimon survives, no deletion.
    assert_eq!(
        after.field[1], before.field[1],
        "a −2000 on a 5000-DP Digimon must not delete it"
    );
    assert_eq!(
        after.trash[1], before.trash[1],
        "no opponent Digimon should be trashed"
    );
    // Neither observer fired.
    assert_eq!(
        after.memory, before.memory,
        "Patamon inherited must NOT fire when no 0-DP deletion occurs"
    );
    assert_eq!(
        runner.effective_dp(carrier_h),
        Some(4000),
        "Tokomon inherited must NOT buff the carrier when no 0-DP deletion occurs"
    );
}

// ─── Combo 2: Seraphimon −4000 softens a too-big Digimon into a deletion ──────

/// Seraphimon's [When Attacking] −4000 (ST3-11) is a *softener*: on its own it
/// can't kill a 6000-DP Digimon, but it lowers the target enough that a follow-up
/// −DP source (Heaven's Charm, ST3-14, −2000) then reaches 0 and deletes it.
/// 6000 − 4000 = 2000 (survives the Seraphimon step), 2000 − 2000 = 0 (deleted).
/// We assert the intermediate effective-DP step-down AND the eventual deletion —
/// the kill is gated on the softener, which neither card achieves alone.
///
/// - Card text: ST3-11 [When Attacking] "−4000 DP"; ST3-14 [Main] "−2000 DP".
/// - Rules basis: `general_rule.pdf` rule **17-1-3-1** (0-DP deletion); the
///   [When Attacking] window is §16 attack-declaration timing; both −DP
///   modifiers expire end-of-turn and stack.
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST3/Yellow/ST3_11.cs`, `ST3_14.cs`.
#[test]
fn seraphimon_softens_then_charm_deletes() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    // Opp body is real vanilla ST4-07 Kuwagamon (6000) — the yellow vanilla pool
    // lacks a 6000-DP body, so a confirmed-vanilla green one is borrowed (color
    // is irrelevant to the −DP/0-DP deletion math).
    let mut runner = dsl_builder(&["ST3-11", "ST3-14", "ST4-07"])
        .hand(0, &["ST3-14"])
        .memory(7)
        .start();

    let seraphimon = runner.place_on_field(0, "ST3-11", Some(0));
    let target = runner.place_on_field(1, "ST4-07", Some(0));

    // Fire Seraphimon's [When Attacking] −4000 directly (models declaring the
    // attack); 6000 → 2000.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::PlayerBattleAreaAttack {
            player: seraphimon.player,
            attacker: seraphimon,
            card: runner.game.players[seraphimon.player as usize].battle_area
                [seraphimon.index as usize]
                .top_card()
                .handle(),
        },
    );
    runner.game.drain_effect_queue();
    runner
        .auto_resolve()
        .expect("Seraphimon target selection resolves");

    assert_eq!(
        runner.effective_dp(target),
        Some(2000),
        "Seraphimon −4000 must lower the 6000-DP target to 2000 (softened, not yet dead)"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "after the softener alone the target still survives"
    );

    // Now Heaven's Charm −2000 finishes it: 2000 → 0 → deleted.
    let before = snapshot(&runner);
    assert!(
        runner.game.activate_hand_main(0, 0),
        "Heaven's Charm [Main] must activate from hand"
    );
    runner
        .auto_resolve()
        .expect("Heaven's Charm target selection resolves");
    let after = snapshot(&runner);

    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "the softened target should be deleted once −2000 brings it to 0 DP"
    );
    assert_eq!(
        after.trash[1],
        before.trash[1] + 1,
        "the deleted Digimon should land in the opponent's trash"
    );
}

// ─── Combo 3: Security-Digimon defensive wall (T.K. + Heaven's Gate) ──────────

/// The defensive half of the deck: on the OPPONENT's turn, T.K. Takaishi (ST3-12,
/// [Opponent's Turn] Security Digimon +2000) stacks with Heaven's Gate's security
/// side (ST3-13, all your Digimon AND Security Digimon +5000 for the turn). The
/// combined Security-Digimon DP adjustment for the defender is +2000 + +5000 =
/// **+7000**, read via `defender_security_dp_adjustment`.
///
/// - Card text: ST3-12 "[Opponent's Turn] all of your Security Digimon get +2000 DP";
///   ST3-13 security "all your Digimon and Security Digimon get +5000 DP for the turn".
/// - Rules basis: §16 Security Attack / Security-Digimon DP; T.K.'s aura gates on
///   `opponents_turn`, so the scenario sets the opponent as turn player.
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST3/Yellow/ST3_12.cs`, `ST3_13.cs`.
#[test]
fn security_wall_stacks_tk_and_heavens_gate() {
    // Attacker is real vanilla ST3-06 Gatomon (DP not asserted); the Security
    // Digimon is real vanilla ST3-03 Tapirmon seeded into security.
    let mut runner = dsl_builder(&["ST3-12", "ST3-13", "ST3-06", "ST3-03"])
        .security(0, &["ST3-03", "ST3-13"])
        .start();

    // Opponent (player 1) is attacking on their turn; player 0 holds the wall.
    let attacker = runner.place_on_field(1, "ST3-06", Some(0));
    runner.place_on_field(0, "ST3-12", Some(0));
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 1;

    // T.K.'s [Opponent's Turn] aura alone: +2000 to player 0's Security Digimon.
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.defender_security_dp_adjustment(0),
        2000,
        "T.K. Takaishi alone must grant +2000 to the defender's Security Digimon on the opp turn"
    );

    // Opponent attacks; the Heaven's Gate security card resolves its +5000 buff.
    let _ = runner.attack_player(attacker, 0, false);
    runner
        .auto_resolve()
        .expect("Heaven's Gate security resolves");

    // Stacked: +2000 (T.K.) + +5000 (Heaven's Gate) = +7000.
    assert_eq!(
        runner.game.defender_security_dp_adjustment(0),
        7000,
        "T.K. +2000 and Heaven's Gate security +5000 must stack to +7000 Security-Digimon DP"
    );
}

// ─── Combo 4: Seven Heavens reaches even a Mega (multi-source −DP stack → 0) ───

/// Seven Heavens (ST3-16, −10000) is huge but not infinite: against a 12000-DP
/// Mega it only reaches 2000 (survives). Adding Heaven's Charm (ST3-14, −2000)
/// pushes the Mega to exactly 0 → deleted (rule 17-1-3-1), which fires Patamon's
/// inherited +1 memory. Two stacked −DP sources turn an un-killable Mega into a
/// deletion that powers the engine.
///
/// - Card text: ST3-16 [Main] "−10000 DP"; ST3-14 [Main] "−2000 DP"; ST3-04 INH
///   "+1 memory on opp 0-DP deletion".
/// - Rules basis: `general_rule.pdf` rule **17-1-3-1** (0-DP deletion); −DP
///   modifiers stack and expire end-of-turn.
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST3/Yellow/ST3_16.cs`,
///   `ST3_14.cs`, `ST3_04.cs`.
#[test]
fn seven_heavens_plus_charm_deletes_a_mega() {
    // Carrier is real vanilla ST3-03 Tapirmon; the Mega is real vanilla
    // ST3-10 Magnadramon (12000 → 0 after −10000 then −2000, deleted).
    let mut runner = dsl_builder(&["ST3-16", "ST3-14", "ST3-04", "ST3-03", "ST3-10"])
        .hand(0, &["ST3-16", "ST3-14"])
        .memory(7)
        .start();

    runner.place_stack(0, &["ST3-04", "ST3-03"]);
    let mega = runner.place_on_field(1, "ST3-10", Some(0));

    // Seven Heavens alone: 12000 − 10000 = 2000 (Mega survives).
    assert!(
        runner.game.activate_hand_main(0, 0),
        "Seven Heavens [Main] must activate from hand"
    );
    runner
        .auto_resolve()
        .expect("Seven Heavens target selection resolves");
    assert_eq!(
        runner.effective_dp(mega),
        Some(2000),
        "Seven Heavens −10000 alone leaves the 12000-DP Mega at 2000 (survives)"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the Mega survives the first −DP source alone"
    );

    // Heaven's Charm finishes: 2000 − 2000 = 0 → deleted; Patamon fires +1 memory.
    // `activate_hand_main` runs the effect without consuming the hand card, so
    // Seven Heavens (index 0) is still present and Charm remains at index 1.
    let before = snapshot(&runner);
    assert!(
        runner.game.activate_hand_main(0, 1),
        "Heaven's Charm [Main] must activate from hand"
    );
    runner
        .auto_resolve()
        .expect("Heaven's Charm target selection resolves");
    let after = snapshot(&runner);

    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "the Mega should be deleted once a second −DP source brings it to 0"
    );
    assert_eq!(
        after.memory,
        before.memory + 1,
        "Patamon inherited must grant +1 memory on the Mega's 0-DP deletion"
    );
}

/// Silence the unused-import lint for `PermanentHandle` (kept in scope for
/// readers extending this file with handle-typed helpers).
fn _unused_silencer(_h: PermanentHandle, _r: AttackResult) {}
