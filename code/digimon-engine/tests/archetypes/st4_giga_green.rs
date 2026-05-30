//! ST-4 Giga Green — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st4-giga-green-model.md` (the suspend-engine deck:
//! Green starter built around *making the opponent's Digimon suspended* and
//! converting that state into tempo/card advantage). These tests assert the
//! deck's combos as a **system** — facts no per-card behavioral test in
//! `tests/cards_behavioral/st4/` can see, because each spans ≥2 real cards
//! cooperating.
//!
//! The archetype's spine is a single causal chain:
//!
//! 1. **Suspend an opponent Digimon** (the trigger). Many cards do this —
//!    Needle Spray (ST4-15) as a [Main] Option, HerculesKabuterimon (ST4-13)
//!    via `<Digi-Burst 2>`, etc. The suspend is the *event*, not the source.
//! 2. **Izzy Izumi (ST4-14)** reacts: `[Your Turn]` when an *opponent* Digimon
//!    becomes suspended, optionally suspend Izzy himself to **gain 1 memory**.
//!    This is the deck's memory engine — every suspend can be a +1 swing.
//! 3. **Electro Shocker (ST4-16)** cashes the suspended state in: a [Main]
//!    Option that returns 1 *suspended* opponent Digimon to hand. It has **no
//!    legal target** until step 1 has happened — its removal is *gated on the
//!    suspend engine having fired*.
//!
//! The three tests below pin those three links:
//! - Test 1 — Needle Spray → Izzy (+1 memory), plus the negatives that keep
//!   the Izzy trigger from over-firing (RANK 1 combo).
//! - Test 2 — HerculesKabuterimon Digi-Burst → Izzy (+1 memory): the *same*
//!   Izzy engine driven by a different suspend source, proving it is
//!   source-agnostic (RANK 2 combo).
//! - Test 3 — suspend → Electro Shocker bounce: the payoff is dead without the
//!   suspend, live with it (RANK 3 combo).
//!
//! Sources:
//! - Card text: `code/digimon-engine/cards/st4/ST4-{13,14,15,16}.yaml` (+ `.json`).
//! - DCGO C# reference: `$BASE_DCGO/Assets/Scripts/CardEffect/ST4/Green/ST4_{13,14,15,16}.cs`.
//! - Rules basis: suspend/unsuspend status + `[Your Turn]` trigger window
//!   (`general_rule.pdf` §6 status, §14 triggered timing); return-to-hand
//!   removal semantics.
//! - Per-card coverage: `tests/cards_behavioral/st4/mod.rs`
//!   (`st4_14_optional_suspend_cost_gains_memory_on_opponent_suspend`,
//!   `st4_13_pierces_and_digi_bursts_to_suspend`,
//!   `st4_15_main_suspends_and_st4_16_main_returns_suspended_digimon`).
//!
//! NOTE on optionality (post-audit): ST4-13/15/16's suspend / return target
//! selections are **mandatory** (a prior audit removed `optional:true`), so
//! once a legal target exists the engine forces a pick — `auto_resolve` selects
//! it; there is no decline branch. Only Izzy's *own* self-suspend activation
//! (ST4-14) is optional, and these tests drive it explicitly.

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};

use super::support::snapshot;

// ─── Neutral fixtures (opponent targets only; real cards via `dsl_card`) ──────

/// A plain opponent Digimon target. Synthetic — it is only ever the *object*
/// of one of our real cards' suspend/return effects, never a card under test,
/// so a neutral stand-in is correct here (per the author rules).
fn make_opp_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(3000);
    c
}

/// A neutral Lv.4 Green base to sit under HerculesKabuterimon as a Digi-Burst
/// source. Two of these give the ST4-13 stack the ≥2 trashable sources its
/// `<Digi-Burst 2>` cost (and `stack_size_gte: 3` gate) require.
fn make_green_source(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(3000);
    c
}

/// Configure player 0 as the active player in Main phase. Izzy's `[Your Turn]`
/// trigger (ST4-14) is gated on `your_turn: true`, so the suspend that drives
/// it must occur on player 0's turn for the trigger to be live.
fn make_p0_turn(runner: &mut DebugRunner) {
    runner.game.turn_order = vec![0, 1];
    runner.game.turn_player_idx = 0;
    runner.game.memory_pair = (0, 1);
    runner.game.current_phase = GamePhase::Main;
}

/// Drain any pending mandatory selections (auto-picking the forced target).
fn resolve_all(runner: &mut DebugRunner) {
    for _ in 0..8 {
        if runner.pending_selection().is_none() {
            return;
        }
        runner.auto_resolve().expect("pending selection resolves");
    }
    assert!(
        runner.pending_selection().is_none(),
        "pending selection did not resolve"
    );
}

// ─── Test 1 — Needle Spray → Izzy +1 memory (RANK 1) ─────────────────────────

/// **Combo: "Needle Spray feeds Izzy".**
/// Cards: ST4-15 Needle Spray ([Main] suspend 1 opponent Digimon) + ST4-14 Izzy
/// Izumi ([Your Turn] when an opponent Digimon is suspended, optionally suspend
/// Izzy for +1 memory) + a neutral opponent Digimon.
///
/// Expected: on player 0's turn, playing Needle Spray suspends the opponent
/// Digimon; that suspend fires Izzy's optional `[Your Turn]` trigger; taking it
/// suspends Izzy AND nets player 0 +1 memory.
///
/// Driving note: `resolve_all` (→ `auto_resolve`) drains both the *mandatory*
/// suspend pick AND Izzy's *optional* self-suspend activation, auto-taking the
/// optional one (it picks the first valid action, which is "activate" — exactly
/// what the per-card test `st4_14_optional_suspend_cost_gains_memory_on_opponent_suspend`
/// relies on). So the resolved-trigger effects (Izzy suspended + memory +1) are
/// the observable proof the optional trigger fired.
///
/// Negatives proven in the sibling tests below:
/// - Izzy's own self-suspend does NOT re-trigger Izzy (he is a Tamer, and the
///   trigger requires `event_target_kind: digimon` + `event_target_owner:
///   opponent`).
/// - Suspending an already-suspended opponent Digimon does not re-fire
///   `OnSuspend` (no status change ⇒ no event).
#[test]
fn needle_spray_suspend_triggers_izzy_for_one_memory() {
    // Play Needle Spray (cost 2) and drive both the mandatory suspend and any
    // Izzy follow-up. Returns the resulting memory and whether the opponent
    // Digimon ended suspended, plus Izzy's suspended state (None when absent).
    fn play_needle_spray(with_izzy: bool) -> (i16, bool, Option<bool>) {
        let mut runner = DebugRunner::builder()
            .dsl_card("ST4-15")
            .expect("ST4-15 (Needle Spray) in embedded DSL pack")
            .dsl_card("ST4-14")
            .expect("ST4-14 (Izzy Izumi) in embedded DSL pack")
            .add_card(make_opp_digimon("OPP", "OppMon"))
            .hand(0, &["ST4-15"])
            .memory(5)
            .start();
        make_p0_turn(&mut runner);

        let izzy = with_izzy.then(|| runner.place_on_field(0, "ST4-14", Some(0)));
        let opp = runner.place_on_field(1, "OPP", Some(0));
        assert!(
            !runner.game.players[1].battle_area[opp.index as usize].is_suspended,
            "precondition: opponent Digimon starts unsuspended"
        );

        // Play Needle Spray → mandatory suspend pick → (if Izzy present) the
        // auto-accepted [Your Turn] optional activation. `resolve_all` drains
        // all of it, mirroring the per-card Izzy test's drive.
        assert!(runner.play(0, 0).is_some(), "Needle Spray plays from hand");
        resolve_all(&mut runner);

        let opp_suspended = runner.game.players[1].battle_area[opp.index as usize].is_suspended;
        let izzy_suspended =
            izzy.map(|h| runner.game.players[0].battle_area[h.index as usize].is_suspended);
        (runner.memory(), opp_suspended, izzy_suspended)
    }

    // Control: Needle Spray alone suspends the opponent Digimon (no Izzy).
    let (mem_without, opp_suspended_without, izzy_state) = play_needle_spray(false);
    assert!(
        opp_suspended_without,
        "Needle Spray must suspend the opponent Digimon"
    );
    assert_eq!(izzy_state, None, "control run has no Izzy on the field");

    // With Izzy: the same play suspends the opponent, Izzy takes his trigger
    // (suspends himself), and memory ends exactly +1 above the control — proving
    // Izzy's contribution in isolation from Needle Spray's own play cost.
    let (mem_with, opp_suspended_with, izzy_suspended) = play_needle_spray(true);
    assert!(
        opp_suspended_with,
        "Needle Spray suspends the opponent Digimon with Izzy present too"
    );
    assert_eq!(
        izzy_suspended,
        Some(true),
        "Izzy pays his suspend-self activation cost off the opponent suspend"
    );
    assert_eq!(
        mem_with,
        mem_without + 1,
        "Izzy's resolved [Your Turn] trigger nets player 0 exactly +1 memory over the Izzy-less control"
    );
}

/// **Negative (Izzy does not feed himself).** Suspending Izzy directly — he is a
/// Tamer, not an opponent Digimon — must NOT fire his own `[Your Turn]` trigger
/// (`event_target_kind: digimon` + `event_target_owner: opponent` both exclude
/// him). No memory is gained and no optional prompt is offered.
#[test]
fn izzy_self_suspend_does_not_retrigger_izzy() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-14")
        .expect("ST4-14 (Izzy Izumi) in embedded DSL pack")
        .memory(0)
        .start();
    make_p0_turn(&mut runner);
    let izzy = runner.place_on_field(0, "ST4-14", Some(0));

    let before = runner.memory();
    runner.game.suspend(izzy);
    resolve_all(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "suspending Izzy (a Tamer) offers no [Your Turn] trigger"
    );
    assert_eq!(
        runner.memory(),
        before,
        "Izzy's own suspension is not an opponent-Digimon suspend — no memory gain"
    );
}

/// **Negative (no double-dip on an already-suspended target).** `OnSuspend` is a
/// status-*change* event: "suspending" an opponent Digimon that is already
/// suspended produces no event, so Izzy's `[Your Turn]` trigger does not fire
/// and no memory is gained. We drive the suspend directly on an already-
/// suspended target (avoiding Needle Spray's own play cost, which would muddy
/// the memory read) — the engine's `suspend` is the same entry point any of the
/// deck's suspend sources funnel through.
#[test]
fn re_suspending_an_already_suspended_target_does_not_refire_izzy() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-14")
        .expect("ST4-14 (Izzy Izumi) in embedded DSL pack")
        .add_card(make_opp_digimon("OPP", "OppMon"))
        .memory(0)
        .start();
    make_p0_turn(&mut runner);

    runner.place_on_field(0, "ST4-14", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    // Put the opponent Digimon into the suspended state with no event.
    runner.game.players[1].battle_area[opp.index as usize].is_suspended = true;

    let before = runner.memory();
    // Re-issue a suspend on the already-suspended target.
    runner.game.suspend(opp);
    resolve_all(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "no Izzy optional trigger when the opponent Digimon was already suspended"
    );
    assert!(
        runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "the opponent Digimon remains suspended"
    );
    assert_eq!(
        runner.memory(),
        before,
        "no fresh suspend event ⇒ Izzy contributes no memory"
    );
}

// ─── Test 2 — HerculesKabuterimon Digi-Burst → Izzy (RANK 2) ─────────────────

/// **Combo: "Digi-Burst suspend feeds Izzy".**
/// Cards: ST4-13 HerculesKabuterimon ([Main] `<Digi-Burst 2>` suspend 1
/// opponent Digimon) + ST4-14 Izzy Izumi + a neutral opponent Digimon.
///
/// Expected: activating HercKabu's [Main] effect pays Digi-Burst 2 (trashes its
/// two digivolution sources) and suspends the opponent Digimon; that suspend
/// fires Izzy's optional `[Your Turn]` trigger exactly as Needle Spray's did,
/// gaining +1 memory when accepted.
///
/// This is the *system* point: the Izzy memory engine is **source-agnostic** —
/// it keys off the suspend *event*, not on Needle Spray specifically. Proving it
/// with a second, mechanically unrelated suspend source (a Digi-Burst on a
/// Mega, vs. a [Main] Option) pins that generality, which neither card's
/// per-card test covers.
#[test]
fn herckabu_digiburst_suspend_triggers_izzy_for_one_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-13")
        .expect("ST4-13 (HerculesKabuterimon) in embedded DSL pack")
        .dsl_card("ST4-14")
        .expect("ST4-14 (Izzy Izumi) in embedded DSL pack")
        .add_card(make_green_source("SRC-A"))
        .add_card(make_green_source("SRC-B"))
        .add_card(make_opp_digimon("OPP", "OppMon"))
        .memory(5)
        .start();
    make_p0_turn(&mut runner);

    let izzy = runner.place_on_field(0, "ST4-14", Some(0));
    // Stack [SRC-A, SRC-B, HercKabu] → 3 cards, ≥2 burnable sources.
    let hercules = runner.place_stack(0, &["SRC-A", "SRC-B", "ST4-13"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    // HercKabu is already on the field and its Digi-Burst cost is *trashing
    // sources*, not memory, so the on-field [Main] activation itself is
    // memory-neutral — the only memory mover in the drain is Izzy's +1. We
    // capture memory immediately before activation so `before + 1` isolates it.
    let before = runner.memory();
    assert!(
        runner.game.activate_field_main(0, hercules.index as usize),
        "HercKabu's [Main] Digi-Burst effect activates"
    );
    // Resolve the mandatory Digi-Burst source picks + the mandatory suspend
    // target, then the auto-accepted Izzy optional activation (same drain
    // pattern as the per-card Izzy test).
    resolve_all(&mut runner);

    assert!(
        runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "Digi-Burst must suspend the selected opponent Digimon"
    );
    assert_eq!(
        runner.game.players[0].battle_area[hercules.index as usize]
            .card_sources
            .len(),
        1,
        "Digi-Burst 2 trashes the two sources, leaving only HercKabu itself"
    );
    assert!(
        runner.game.players[0].battle_area[izzy.index as usize].is_suspended,
        "Izzy's trigger pays its suspend-self cost off the Digi-Burst suspend"
    );
    assert_eq!(
        runner.memory(),
        before + 1,
        "the Digi-Burst suspend drives the same Izzy +1 memory as Needle Spray — source-agnostic"
    );
}

// ─── Test 3 — Suspend → Electro Shocker bounce (RANK 3) ──────────────────────

/// **Combo: "suspend, then Electro Shocker bounces it".**
/// Cards: ST4-15 Needle Spray (the suspend enabler) + ST4-16 Electro Shocker
/// ([Main] return 1 *suspended* opponent Digimon to hand) + a neutral opponent
/// Digimon.
///
/// Expected: Electro Shocker's filter is `is_suspended: true`, so against an
/// unsuspended-only board it has **no legal target** — playing it does not bounce
/// anything (opp field/hand unchanged, no bounce selection pending). After
/// Needle Spray suspends the opponent Digimon, Electro Shocker can target *only*
/// the suspended one and returns the whole permanent to hand (opp field −1,
/// opp hand +1).
///
/// The system fact: Electro Shocker is a dead card on its own; the suspend
/// engine is what *turns it on*. That gating across the two cards is exactly
/// what no per-card test sees.
#[test]
fn electro_shocker_bounce_is_gated_on_a_prior_suspend() {
    // ── Phase A: no suspended target ⇒ Electro Shocker bounces nothing. ──
    let mut dead = DebugRunner::builder()
        .dsl_card("ST4-16")
        .expect("ST4-16 (Electro Shocker) in embedded DSL pack")
        .add_card(make_opp_digimon("OPP", "OppMon"))
        .hand(0, &["ST4-16"])
        .memory(10)
        .start();
    make_p0_turn(&mut dead);
    let opp = dead.place_on_field(1, "OPP", Some(0));
    assert!(
        !dead.game.players[1].battle_area[opp.index as usize].is_suspended,
        "precondition: the only opponent Digimon is unsuspended"
    );

    let before = snapshot(&dead);
    assert!(dead.play(0, 0).is_some(), "Electro Shocker plays from hand");
    // With no suspended opponent Digimon the return clause finds no target: the
    // bounce selection (SelectKind::OppField) must NOT install with a live pick.
    assert!(
        dead.pending_kind() != Some(digimon_engine::selection::SelectionKind::OppField)
            || dead.pending_action_count() == 0,
        "Electro Shocker exposes no valid bounce target against an unsuspended board"
    );
    resolve_all(&mut dead);
    let after = snapshot(&dead);

    assert_eq!(
        after.field[1], before.field[1],
        "no suspended target ⇒ no opponent Digimon is bounced (opp field unchanged)"
    );
    assert_eq!(
        after.hand[1], before.hand[1],
        "no suspended target ⇒ nothing returns to the opponent's hand"
    );

    // ── Phase B: Needle Spray suspends, then Electro Shocker can bounce. ──
    let mut live = DebugRunner::builder()
        .dsl_card("ST4-15")
        .expect("ST4-15 (Needle Spray) in embedded DSL pack")
        .dsl_card("ST4-16")
        .expect("ST4-16 (Electro Shocker) in embedded DSL pack")
        .add_card(make_opp_digimon("OPP", "OppMon"))
        // Needle Spray first in hand (index 0), Electro Shocker second (index 1).
        .hand(0, &["ST4-15", "ST4-16"])
        .memory(10)
        .start();
    make_p0_turn(&mut live);
    let opp = live.place_on_field(1, "OPP", Some(0));

    // Needle Spray (hand index 0) suspends the opponent Digimon.
    assert!(live.play(0, 0).is_some(), "Needle Spray plays");
    resolve_all(&mut live);
    assert!(
        live.game.players[1].battle_area[opp.index as usize].is_suspended,
        "Needle Spray suspends the opponent Digimon, arming Electro Shocker"
    );

    let before = snapshot(&live);
    // Electro Shocker is now at hand index 0 (Needle Spray left the hand).
    assert!(
        live.play(0, 0).is_some(),
        "Electro Shocker plays now that a suspended target exists"
    );
    resolve_all(&mut live);
    let after = snapshot(&live);

    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "the suspended opponent Digimon (whole permanent) leaves the field"
    );
    assert_eq!(
        after.hand[1],
        before.hand[1] + 1,
        "the returned Digimon's top card lands in the opponent's hand"
    );
    assert!(
        live.game.players[1].battle_area.is_empty(),
        "the only opponent Digimon was the suspended one and it is now gone from the field"
    );
}
