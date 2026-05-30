//! ST-2 *Cocytus Blue* — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st2-cocytus-blue-model.md`. These tests pin the
//! deck's multi-card combos as a *system*, distinct from the per-card behavioral
//! coverage in `tests/cards_behavioral/st2/st2_cards.rs`.
//!
//! # The engine hinge every combo shares
//!
//! `materials_count = perm.card_sources.len().saturating_sub(1)`
//! (`code/digimon-engine/src/dsl_cards/predicate.rs`). `materials_count == 0`
//! means "top card only, no digivolution cards underneath" — a source-less
//! Digimon (DCGO's `Permanent.HasNoDigivolutionCards`). The archetype's whole
//! engine is: *strip* an opponent Digimon's sources to zero, which simultaneously
//! flips ON three payoffs that all read the same `materials_count_lte: 0`
//! boolean — Tsunomon's +1000-DP-in-battle (ST2-01), WereGarurumon's
//! `<Security A.+1>` (ST2-08), and Matt Ishida's +1 memory/turn (ST2-12).
//!
//! Rules basis: general_rule.pdf §6-8/§6-9 (trashing bottom digivolution cards
//! thins the stack without deleting the Digimon); §16 static-ability continuous
//! re-evaluation (the payoffs read live no-source status). DCGO references:
//! `$BASE_DCGO/Assets/Scripts/CardEffect/ST2/Blue/{ST2_01,ST2_08,ST2_09,ST2_12,ST2_11,ST2_06,ST2_03}.cs`.

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{AttackTarget, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// The four ST-2 cards each combo draws from, loaded into a fresh builder so a
/// typo or un-migrated card fails loudly at fixture-build time.
fn st2_builder(card_ids: &[&str]) -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder();
    for id in card_ids {
        b = b
            .dsl_card(id)
            .unwrap_or_else(|e| panic!("ST2 DSL card {id} not loadable into the fixture: {e}"));
    }
    b
}

/// A neutral synthetic Digimon — used only for opponent strip-targets, stack
/// sources, and filler bodies (never for the ST-2 cards under test).
fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c
}

/// Number of digivolution cards under the top card of `(player, index)` — the
/// engine's `materials_count`. `0` == source-less (the payoff trigger boolean).
fn materials_count(runner: &DebugRunner, player: u8, index: usize) -> usize {
    runner.game.player(player).battle_area[index]
        .card_sources
        .len()
        .saturating_sub(1)
}

/// Fire a triggered timing directly for `source` and drain the queue — the
/// proven harness pattern (`place_stack` builds the stack without firing the
/// digivolve trigger, so we fire it explicitly).
fn fire(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

/// Resolve the first valid action on the current pending selection (the
/// "choose 1 opponent Digimon" prompt the strip effects install).
fn choose_first_pending(runner: &mut DebugRunner) {
    let (player, action) = {
        let pending = runner
            .pending_selection()
            .expect("expected a pending selection");
        (
            pending.selecting_player,
            *pending
                .valid_action_ids
                .first()
                .expect("selection has at least one action"),
        )
    };
    runner
        .execute_action(player, action)
        .expect("pending selection resolves");
    runner.game.drain_effect_queue();
}

// ─── Test 1 (Rank 1, Combo A) ────────────────────────────────────────────────

/// **Strip → triple no-source payoff flip.**
///
/// Cards: ST2-09 Zudomon (strip), ST2-01 Tsunomon (egg inheritable), ST2-08
/// WereGarurumon (Security-pressure inheritable), ST2-12 Matt Ishida (memory).
///
/// Setup: an opponent Digimon that digivolved naturally with **exactly 2
/// sources**. Zudomon's `[When Digivolving]` trashes both bottom sources, flipping
/// it to `materials_count == 0`. The instant it is source-less, all three payoffs
/// that share the `materials_count_lte: 0` boolean turn ON together:
///   (a) a Tsunomon-inheritable attacker gets +1000 DP **only while battling** the
///       source-less opp — proven by a battle outcome that flips from a loss to
///       mutual destruction;
///   (b) a WereGarurumon-inheritable body gains `<Security A.+1>`
///       (`security_attack_keyword_bonus == 1`);
///   (c) Matt Ishida nets +1 memory on the controller's Start of Turn.
///
/// Expected: opp `materials_count` 2 → 0; payoff (b) and (c) read the live
/// source-less board; payoff (a) is the load-bearing +1000 in battle.
///
/// Sources: `$BASE_DCGO/.../ST2/Blue/{ST2_09,ST2_01,ST2_08,ST2_12}.cs`;
/// general_rule.pdf §6-8/§6-9 (source trashing) + §16 (static re-evaluation).
#[test]
fn strip_flips_all_three_no_source_payoffs_on_together() {
    let mut runner = st2_builder(&["ST2-09", "ST2-01", "ST2-08", "ST2-12"])
        // Opponent strip-target: a 2000-DP body sitting on exactly 2 sources.
        .add_card(digimon("OPP-SRC-A", 2, 1000))
        .add_card(digimon("OPP-SRC-B", 3, 2000))
        .add_card(digimon("OPP-TOP", 4, 2000))
        // P0 bodies: Zudomon's digivolve base, the Tsunomon attacker's base, and
        // WereGarurumon's carrier body.
        .add_card(digimon("ZUDO-BASE", 4, 3000))
        .add_card(digimon("TSU-ATK", 3, 1000))
        .add_card(digimon("WERE-BODY", 6, 6000))
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    // Opponent: one naturally-digivolved Digimon with exactly 2 sources.
    let opp = runner.place_stack(1, &["OPP-SRC-A", "OPP-SRC-B", "OPP-TOP"]);
    assert_eq!(
        materials_count(&runner, 1, opp.index as usize),
        2,
        "precondition: the opponent Digimon starts with exactly 2 sources"
    );

    // P0 board: Zudomon (strip), a Tsunomon-inheritable attacker, a
    // WereGarurumon-inheritable body, and Matt Ishida.
    let zudomon = runner.place_stack(0, &["ZUDO-BASE", "ST2-09"]);
    let tsu_attacker = runner.place_stack(0, &["ST2-01", "TSU-ATK"]);
    let were_carrier = runner.place_stack(0, &["ST2-08", "WERE-BODY"]);
    let matt = runner.place_on_field(0, "ST2-12", None);

    // --- The strip: Zudomon [When Digivolving] trashes 2 bottom sources. ---
    fire(&mut runner, EffectTiming::WhenDigivolving, zudomon);
    choose_first_pending(&mut runner);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "the strip resolves fully without a lingering prompt"
    );
    assert_eq!(
        materials_count(&runner, 1, opp.index as usize),
        0,
        "Zudomon's [When Digivolving] strip drops the 2-source opponent to source-less"
    );
    assert_eq!(
        runner.game.player(1).battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "only the top card remains (the two bottom sources were trashed)"
    );

    runner.game.tick_declarative_effects();

    // --- Payoff (b): WereGarurumon inheritable now grants <Security A.+1>. ---
    assert_eq!(
        runner.game.security_attack_keyword_bonus(were_carrier),
        1,
        "WereGarurumon's inheritable grants <Security A.+1> while the opponent has \
         a source-less Digimon"
    );

    // --- Payoff (c): Matt Ishida nets +1 memory at the controller's turn start. ---
    let mem_before = runner.memory();
    fire(&mut runner, EffectTiming::StartOfYourTurn, matt);
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "Matt Ishida gains 1 memory at Start of Your Turn while an opponent Digimon \
         is source-less"
    );

    // --- Payoff (a): Tsunomon's +1000 manifests ONLY in battle vs the source-less opp. ---
    // Outside battle the aura is dormant, so effective DP is the printed 1000.
    assert_eq!(
        runner.effective_dp(tsu_attacker),
        Some(1000),
        "Tsunomon's +1000 is a battle-only aura — dormant outside Digimon-vs-Digimon battle"
    );
    // In battle: 1000 attacker + Tsunomon's +1000 == 2000 == the 2000-DP source-less
    // opp, so the battle is mutual destruction. Without the +1000 the attacker would
    // simply lose (1000 < 2000) — so MutualDestruction is load-bearing proof of the buff.
    let result = runner
        .game
        .begin_attack(tsu_attacker, AttackTarget::Digimon(opp), false);
    assert_eq!(
        result,
        AttackResult::MutualDestruction,
        "Tsunomon's battle-only +1000 lifts the 1000-DP attacker to 2000, matching the \
         source-less 2000-DP opponent — mutual destruction proves the buff applied"
    );
}

// ─── Test 2 (Rank 2, Combo B) ────────────────────────────────────────────────

/// **`<Security A.+1>` reads the board, not the battle target.**
///
/// Cards: ST2-08 WereGarurumon (inheritable carrier) + opponent Digimon.
///
/// WereGarurumon's `[Your Turn]` inheritable grants `<Security A.+1>` while the
/// opponent controls *any* no-source Digimon (`any_permanent of: opponent`), not
/// only the one being attacked. Asserts: (1) NO bonus when every opponent Digimon
/// has sources; (2) the bonus turns ON once ANY opponent Digimon is source-less —
/// even a second, un-attacked one. This is the subtle "board, not target" scoping.
///
/// Sources: `$BASE_DCGO/.../ST2/Blue/ST2_08.cs`
/// (`HasMatchConditionOpponentsPermanent … HasNoDigivolutionCards`,
/// `ChangeSelfSAttackStaticEffect(changeValue: 1)`); glossary.pdf
/// `<Security Attack +N>`; general_rule.pdf §16 static keyword grant.
#[test]
fn weregarurumon_security_plus_keys_off_any_opponent_no_source_digimon() {
    // Baseline: every opponent Digimon has a source → no bonus.
    let mut runner = st2_builder(&["ST2-08"])
        .add_card(digimon("OPP-SRC", 3, 2000))
        .add_card(digimon("OPP-TOP-A", 4, 3000))
        .add_card(digimon("OPP-TOP-B", 4, 3000))
        .add_card(digimon("WERE-BODY", 6, 6000))
        .start();

    let carrier = runner.place_stack(0, &["ST2-08", "WERE-BODY"]);
    // Two opponent Digimon, BOTH with sources.
    let sourced_a = runner.place_stack(1, &["OPP-SRC", "OPP-TOP-A"]);
    let _sourced_b = runner.place_stack(1, &["OPP-SRC", "OPP-TOP-B"]);
    runner.game.tick_declarative_effects();

    assert!(
        materials_count(&runner, 1, sourced_a.index as usize) >= 1,
        "precondition: every opponent Digimon has at least one source"
    );
    assert_eq!(
        runner.game.security_attack_keyword_bonus(carrier),
        0,
        "no <Security A.+1> while every opponent Digimon still has digivolution cards"
    );

    // Now make the *second* (un-attacked) opponent Digimon source-less by placing
    // a fresh, source-less body. The bonus must read the board, not the target.
    let no_source = runner.place_on_field(1, "OPP-TOP-A", None);
    assert_eq!(
        materials_count(&runner, 1, no_source.index as usize),
        0,
        "the freshly-placed opponent Digimon is source-less"
    );
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.game.security_attack_keyword_bonus(carrier),
        1,
        "WereGarurumon's <Security A.+1> turns ON when ANY opponent Digimon is \
         source-less — keyed off the board, not the battle target"
    );
}

// ─── Test 3 (Rank 3, Combo C) ────────────────────────────────────────────────

/// **MetalGarurumon OPT self-unsuspend = exactly two attacks; inherited strip
/// fires on each.**
///
/// Cards: ST2-11 MetalGarurumon (finisher) + ST2-06 Garurumon inheritable strip
/// in its stack + an opponent Digimon with sources to peel.
///
/// MetalGarurumon's `[When Attacking][Once Per Turn]` unsuspends itself, enabling
/// a *second* attack — but the OPT flag means it unsuspends only once, so a turn
/// yields exactly two attacks, never an infinite loop. The Garurumon strip in its
/// digivolution stack is a `[When Attacking]` inheritable, so it fires on *each*
/// attack, peeling one bottom source per attack.
///
/// Asserts: (1) first `[When Attacking]` unsuspends MetalGarurumon (can attack
/// again); (2) the strip fires on the first attack (opp 2 sources → 1); (3) the
/// second `[When Attacking]` does NOT unsuspend a third time (OPT boundary); (4)
/// the strip fires again on the second attack (opp 1 source → 0). Two attacks,
/// two strips, one unsuspend.
///
/// Sources: `$BASE_DCGO/.../ST2/Blue/{ST2_11,ST2_06}.cs`
/// (ST2_11 OPT-tagged `SetHashString("Unsuspend_ST2_11")`); general_rule.pdf §16
/// `[Once Per Turn]` and `[When Attacking]` inherited-effect resolution per attack.
#[test]
fn metalgarurumon_unsuspends_once_strip_fires_each_attack() {
    let mut runner = st2_builder(&["ST2-11", "ST2-06"])
        .add_card(digimon("OPP-SRC-A", 2, 1000))
        .add_card(digimon("OPP-SRC-B", 3, 2000))
        .add_card(digimon("OPP-TOP", 4, 3000))
        .start();

    // MetalGarurumon (top) over a Garurumon source (the inherited [When Attacking]
    // strip rides in the stack). The base under Garurumon is filler.
    let metal = runner.place_stack(0, &["ST2-06", "ST2-11"]);
    // Opponent: one Digimon with exactly 2 sources to peel across the two attacks.
    let opp = runner.place_stack(1, &["OPP-SRC-A", "OPP-SRC-B", "OPP-TOP"]);
    assert_eq!(
        materials_count(&runner, 1, opp.index as usize),
        2,
        "precondition: opponent has 2 sources to be peeled one-per-attack"
    );

    // Start suspended-then-attack model: MetalGarurumon is suspended after its
    // first attack, the OPT [When Attacking] unsuspends it.
    let metal_idx = metal.index as usize;
    runner.game.player_mut(0).battle_area[metal_idx].is_suspended = true;

    // --- First attack: fires MetalGarurumon's unsuspend + Garurumon's strip. ---
    fire(&mut runner, EffectTiming::WhenAttacking, metal);
    // The inherited strip installs a "choose opponent Digimon" prompt; resolve it.
    if runner.pending_selection().is_some() {
        choose_first_pending(&mut runner);
    }
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.player(0).battle_area[metal_idx].is_suspended,
        "first [When Attacking] OPT unsuspends MetalGarurumon, enabling a second attack"
    );
    assert_eq!(
        materials_count(&runner, 1, opp.index as usize),
        1,
        "the inherited Garurumon strip peels one bottom source on the first attack (2 → 1)"
    );

    // --- Second attack: re-suspend, fire again. ---
    runner.game.player_mut(0).battle_area[metal_idx].is_suspended = true;
    fire(&mut runner, EffectTiming::WhenAttacking, metal);
    if runner.pending_selection().is_some() {
        choose_first_pending(&mut runner);
    }
    let _ = runner.auto_resolve();

    assert!(
        runner.game.player(0).battle_area[metal_idx].is_suspended,
        "OPT boundary: the second same-turn [When Attacking] does NOT unsuspend again \
         — exactly two attacks, no infinite loop"
    );
    assert_eq!(
        materials_count(&runner, 1, opp.index as usize),
        0,
        "the inherited strip fires again on the second attack, flipping the opponent \
         Digimon source-less (1 → 0)"
    );
}
