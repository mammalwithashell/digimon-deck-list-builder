//! BEATBREAK (BT25 "Glowing Dawn") — Pass-2 archetype interaction tests.
//!
//! Model: `qa/archetype-qa/beatbreak-model.md` (Pass-2 named combos section).
//!
//! Slice (as given): BT25-088 Kyo Sawashiro, BT25-090 Tomoro Tenma, BT25-035
//! Cougarmon, BT25-049 Armalizamon, BT25-081 Fangmon, BT25-041 Murasamemon,
//! BT25-057 Monarchlizamon.
//!
//! Phase-4 precondition gate (2026-06-06): six of the seven are IMPLEMENTED
//! (YAML present + per-card behavioral tests green): BT25-035, BT25-041,
//! BT25-049, BT25-081, BT25-088, BT25-090. **BT25-057 Monarchlizamon is BLOCKED**
//! (0-byte per-card test stub, no YAML) — every combo naming it is dropped and
//! logged in the model, NOT authored here.
//!
//! Every PARTIAL card's defining clause (the BEATBREAK cost-engine: cost
//! reduction / free-digivolve unlocked by trashing face-down cards banked under
//! Tamers) is BLOCKED on `G-COST-REDUCTION-INTERACTIVE-PAY-COST` /
//! `G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER`; these tests fire only the
//! IMPLEMENTED clauses (−DP, suspend, the face-down *banking* itself, the
//! inherited keyword/unsuspend) and never the blocked payoff.
//!
//! Lazy cross-set closure: cross-set evolution prerequisites are synthesized via
//! neutral `make_test_card` fixtures whose only relevant properties are
//! kind/level/colour/trait — no cross-set card's printed *effect* is fired, so
//! no cross-set implementation is pulled.
//!
//! Each `#[test]` maps 1:1 to a named Pass-2 combo and asserts the combo's
//! claimed *mechanical* outcome — the system-level fact the per-card tests
//! cannot express (one slice card's action firing another slice card's trigger;
//! the producer and consumer of the face-down resource being different cards;
//! two independent debuffs coexisting on one target with different lifetimes).
//!
//! Sources cited inline per combo: card text (the YAML specs / cards.json),
//! DCGO C# (`$BASE_DCGO/Assets/Scripts/CardEffect/BT25/<Color>/BT25_0xx.cs`),
//! and `general_rule.pdf` for suspend / trigger / modifier-expiry timing.

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

const COUGARMON: &str = "BT25-035"; // Lv.4 Yellow payoff: -3000 DP
const MURASAMEMON: &str = "BT25-041"; // Lv.5 Yellow: inherited EoA unsuspend
const ARMALIZAMON: &str = "BT25-049"; // Lv.4 Green payoff: suspend opp Digimon
const TOMORO: &str = "BT25-090"; // Green Tamer: face-down banker

// ─── Shared neutral fixtures (no printed effect fired) ───────────────────────

/// A neutral opponent Digimon — only its kind/level/dp/colour matter, so it is a
/// legal target for Cougarmon's −DP and Armalizamon's suspend. No printed
/// effect (lazy cross-set closure: nothing of its own fires).
fn make_opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c
}

/// A neutral deck-filler Digimon (the cards Tomoro banks face-down; the rule
/// draws). No printed effect.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

/// A neutral [Glowing Dawn] HOST Digimon — Murasamemon's inherited
/// [End of Attack] clause requires the host carrying it to have the Glowing Dawn
/// trait. No printed effect of its own.
fn make_glowing_dawn_host(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(6);
    c.dp = Some(9000);
    c.play_cost = 6;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo BB-IT1 — Armalizamon's suspend feeds Tomoro Tenma's face-down banker
// ═════════════════════════════════════════════════════════════════════════════
//
// Cards: BT25-049 Armalizamon ([On Play]/[WD] optional suspend an opp Digimon)
//        + BT25-090 Tomoro Tenma ([All Turns] when ANY Digimon suspend, by
//        suspending this Tamer, may bank the top 2 deck cards face-down).
//
// Claimed outcome: Armalizamon's suspend of an opponent Digimon is a Digimon-
// suspend event; Tomoro's [All Turns] clause fires off it. Accepting Tomoro's
// optional clause suspends Tomoro (its activation cost) and banks the top 2 deck
// cards face-down under Tomoro (deck −2; Tomoro source-count 1→3; both placed
// sources face-down). The opponent Digimon stays suspended.
//
// System fact a per-card test can't show: Tomoro's banker is fed by ANOTHER
// slice card's real suspend (the per-card 090 test calls game.suspend directly).
//
// Sources: DCGO BT25_049.cs `SelectPermanentEffect Mode.Tap` over opp Digimon;
// DCGO BT25_090.cs `OnTappedAnyone` gated on `IsPermanentExistsOnBattleAreaDigimon`
// → `AddDigivolutionCardsBottom isFacedown:true`; general_rule.pdf suspend timing.

/// Happy path: Armalizamon suspends an opp Digimon → Tomoro banks 2 face-down.
#[test]
fn bb_it1_armalizamon_suspend_feeds_tomoro_face_down_banker() {
    let mut runner = DebugRunner::builder()
        .dsl_card(ARMALIZAMON)
        .expect("BT25-049 (Armalizamon) in embedded DSL pack")
        .dsl_card(TOMORO)
        .expect("BT25-090 (Tomoro Tenma) in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-DIGI", 4000))
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .deck(0, &["D1", "D2"]) // 2 cards for Tomoro to bank
        .deck(1, &["D1"])
        .memory(10)
        .start();

    // P0 fields Tomoro (the banker, a Tamer) and Armalizamon (the suspender).
    let tomoro = runner.place_on_field(0, TOMORO, Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let arm = runner.place_on_field(0, ARMALIZAMON, None);

    let deck_before = runner.deck_size(0);
    assert_eq!(
        runner.game.players[0].battle_area[tomoro.index as usize]
            .card_sources
            .len(),
        1,
        "Tomoro starts with just its own card (no banked face-downs)"
    );

    // Fire Armalizamon's [On Play] → optional suspend of an opp Digimon.
    runner.fire_on_play(0, arm.index as usize);
    let view = runner
        .pending_selection_view()
        .expect("Armalizamon's optional suspend prompt installs (opp Digimon present)");
    assert!(
        view.is_optional,
        "Armalizamon's suspend is 'you may' → optional"
    );
    let target = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != digimon_engine::action::space::PASS)
        .expect("the opponent Digimon is a suspend target");
    runner
        .execute_action(view.selecting_player, target)
        .expect("suspend the opponent Digimon");

    // The opp-Digimon suspend is a Digimon-suspend event that triggers Tomoro's
    // [All Turns] banker. It surfaces as an OPTIONAL accept/decline prompt — we
    // observe it directly (NOT via auto_resolve, which would auto-pick a branch).
    assert!(
        runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "Armalizamon's chosen opponent Digimon is suspended"
    );
    assert!(
        runner.game.pending_selection.is_some(),
        "the Digimon-suspend event installs Tomoro's optional banker prompt — \
         one slice card's action fires another's trigger"
    );
    runner
        .accept_optional_trigger()
        .expect("accept Tomoro's suspend-self + bank-2 clause");
    let _ = runner.auto_resolve();

    // ── Claimed mechanical outcome ───────────────────────────────────────────
    // 1. Tomoro paid its activation cost (suspended itself).
    assert!(
        runner.game.players[0].battle_area[tomoro.index as usize].is_suspended,
        "Tomoro suspended itself as the banker's activation cost"
    );
    // 2. The top 2 deck cards were banked under Tomoro.
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "the top 2 deck cards left the deck to be banked under Tomoro"
    );
    let perm = &runner.game.players[0].battle_area[tomoro.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        3,
        "Tomoro's own card + 2 banked face-down sources"
    );
    // 3. Both banked sources are face-down (the BEATBREAK fuel).
    assert!(
        perm.card_sources[0].face_down && perm.card_sources[1].face_down,
        "the two banked sources are face-down at the bottom of Tomoro"
    );
    assert!(
        runner.pending_selection().is_none(),
        "the banker clause fully resolves once accepted"
    );
}

/// Unhappy path: the opponent has NO Digimon, so Armalizamon's suspend never
/// fires → no Digimon-suspend event → Tomoro's banker never triggers. The
/// system-level negative: the engine only fires when its upstream producer does.
#[test]
fn bb_it1_no_opp_digimon_means_no_suspend_and_no_banking() {
    let mut runner = DebugRunner::builder()
        .dsl_card(ARMALIZAMON)
        .expect("BT25-049 (Armalizamon)")
        .dsl_card(TOMORO)
        .expect("BT25-090 (Tomoro Tenma)")
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .deck(0, &["D1", "D2"])
        .deck(1, &["D1"])
        .memory(10)
        .start();

    let tomoro = runner.place_on_field(0, TOMORO, Some(0));
    let arm = runner.place_on_field(0, ARMALIZAMON, None);
    let deck_before = runner.deck_size(0);

    // No opponent Digimon → Armalizamon installs no suspend prompt at all.
    runner.fire_on_play(0, arm.index as usize);
    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → Armalizamon's suspend never prompts"
    );
    let _ = runner.auto_resolve();

    // Tomoro never banked (its trigger needs a Digimon-suspend event).
    assert!(
        !runner.game.players[0].battle_area[tomoro.index as usize].is_suspended,
        "with no suspend event, Tomoro is not suspended"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tomoro.index as usize]
            .card_sources
            .len(),
        1,
        "Tomoro banked nothing (no upstream suspend)"
    );
    assert_eq!(runner.deck_size(0), deck_before, "no cards banked");
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo BB-IT2 — Tomoro banks the fuel that Murasamemon's inherited unsuspend spends
// ═════════════════════════════════════════════════════════════════════════════
//
// Cards: BT25-090 Tomoro Tenma (banks a face-down under itself, a Tamer) +
//        BT25-041 Murasamemon (inherited [End of Attack][OPT]: by trashing a
//        bottom face-down under ANY of your Tamers, this [Glowing Dawn] Digimon
//        unsuspends).
//
// Claimed outcome: after Tomoro banks face-down cards under itself, a suspended
// Glowing-Dawn host carrying Murasamemon as an inherited source fires
// Murasamemon's [End of Attack]: it trashes one bottom face-down under Tomoro
// (trash +1) and unsuspends the host. The producer (090) and consumer (041) of
// the face-down resource are DIFFERENT cards — the per-card 041 test hand-sets
// `face_down = true` on a synthetic Tamer; here Tomoro's REAL banking clause
// produces the face-down.
//
// Sources: DCGO BT25_090.cs `AddDigivolutionCardsBottom` (face-down) + DCGO
// BT25_041.cs inherited EndOfAttack `IsTamerWithFaceDownCard` →
// `TrashDigivolutionCardsFromTopOrBottom(isFromTop:false)` → unsuspend self;
// general_rule.pdf §16 inherited-effect / End-of-Attack timing.

/// Drive Tomoro's real banker so a face-down exists under it, then fire
/// Murasamemon's inherited unsuspend, which spends that exact banked face-down.
fn bank_one_under_tomoro(
    runner: &mut DebugRunner,
    tomoro: digimon_engine::permanent::PermanentHandle,
) {
    // Trigger Tomoro's [All Turns] banker by suspending a Digimon on the field.
    // (We seed an opp Digimon and suspend it directly — the event is what matters.)
    let opp = runner.place_on_field(1, "BB2-OPP", Some(0));
    runner.game.suspend(opp);
    assert!(
        runner.game.pending_selection.is_some(),
        "Tomoro's optional banker prompt installs on the Digimon suspend"
    );
    runner
        .accept_optional_trigger()
        .expect("accept Tomoro's bank-2 clause");
    let _ = runner.auto_resolve();
    let banked = runner.game.players[0].battle_area[tomoro.index as usize]
        .card_sources
        .iter()
        .filter(|s| s.face_down)
        .count();
    assert!(
        banked >= 1,
        "Tomoro must have banked at least one face-down source"
    );
}

#[test]
fn bb_it2_tomoro_banked_face_down_pays_murasamemon_inherited_unsuspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card(TOMORO)
        .expect("BT25-090 (Tomoro Tenma) in embedded DSL pack")
        .dsl_card(MURASAMEMON)
        .expect("BT25-041 (Murasamemon) in embedded DSL pack")
        .add_card(make_glowing_dawn_host("HOST"))
        .add_card(make_opp_digimon("BB2-OPP", 4000))
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .deck(0, &["D1", "D2", "D1", "D2"])
        .deck(1, &["D1"])
        .memory(10)
        .start();

    // P0 fields Tomoro (the banker). Murasamemon sits as an inherited SOURCE
    // under a Glowing-Dawn host.
    let tomoro = runner.place_on_field(0, TOMORO, Some(0));
    let host = runner.place_stack(0, &[MURASAMEMON, "HOST"]);

    // ── Produce the fuel: Tomoro banks a real face-down under itself. ─────────
    bank_one_under_tomoro(&mut runner, tomoro);

    // Suspend the host (as if it just attacked) and record the trash baseline.
    runner.game.players[0].battle_area[host.index as usize].is_suspended = true;
    let trash_before = runner.trash_size(0);
    let tomoro_sources_before = runner.game.players[0].battle_area[tomoro.index as usize]
        .card_sources
        .len();

    // ── Spend the fuel: fire Murasamemon's inherited [End of Attack]. ─────────
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_some(),
        "the inherited End-of-Attack OPT installs a prompt (Glowing Dawn host + a \
         banked face-down under Tomoro is a legal trash cost)"
    );
    let _ = runner.auto_resolve();

    // ── Claimed mechanical outcome ───────────────────────────────────────────
    // 1. The host unsuspended (cost paid).
    assert!(
        !runner.game.players[0].battle_area[host.index as usize].is_suspended,
        "the Glowing Dawn host unsuspends after Murasamemon's inherited clause spends a face-down"
    );
    // 2. Exactly one banked face-down was trashed from under Tomoro.
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "one banked face-down under Tomoro was trashed as the unsuspend cost"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tomoro.index as usize]
            .card_sources
            .len(),
        tomoro_sources_before - 1,
        "Tomoro's banked-source count dropped by exactly one (the spent fuel)"
    );
}

/// Unhappy path: NO face-down banked anywhere (Tomoro never ran its banker) →
/// Murasamemon's inherited unsuspend cost is unpayable → the host stays
/// suspended. Pins that the consumer (041) is gated on the producer (090) having
/// actually banked — the engine loop breaks without the fuel.
#[test]
fn bb_it2_without_a_banked_face_down_the_host_stays_suspended() {
    let mut runner = DebugRunner::builder()
        .dsl_card(TOMORO)
        .expect("BT25-090 (Tomoro Tenma)")
        .dsl_card(MURASAMEMON)
        .expect("BT25-041 (Murasamemon)")
        .add_card(make_glowing_dawn_host("HOST"))
        .add_card(make_filler("D1"))
        .deck(0, &["D1", "D1"])
        .deck(1, &["D1"])
        .memory(10)
        .start();

    // Tomoro on the field but it NEVER banked (no suspend event drove it).
    let _tomoro = runner.place_on_field(0, TOMORO, Some(0));
    let host = runner.place_stack(0, &[MURASAMEMON, "HOST"]);
    runner.game.players[0].battle_area[host.index as usize].is_suspended = true;

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[host.index as usize].is_suspended,
        "with no banked face-down anywhere, the unsuspend cost is unpayable → host stays suspended"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo BB-IT3 — Cougarmon −DP + Armalizamon suspend stack on one target
// ═════════════════════════════════════════════════════════════════════════════
//
// Cards: BT25-035 Cougarmon ([On Play]/[WD] one opp Digimon gets −3000 DP for
//        the turn) + BT25-049 Armalizamon ([On Play]/[WD] optional suspend one
//        opp Digimon).
//
// Claimed outcome: both payoffs target the SAME opponent Digimon → it ends at
// −3000 effective DP (turn-scoped) AND suspended. The two independent debuffs
// COEXIST on one permanent; the −3000 expires at end of turn while the suspend
// persists across the turn boundary (different lifetimes). This is the deck's
// standard "soften + tap" removal prep.
//
// Sources: DCGO BT25_035.cs `-3000 DP UntilEachTurnEnd`; DCGO BT25_049.cs
// `Mode.Tap`; general_rule.pdf modifier-expiry vs suspend-state persistence.

#[test]
fn bb_it3_cougarmon_dp_debuff_and_armalizamon_suspend_coexist_then_dp_expires() {
    let mut runner = DebugRunner::builder()
        .dsl_card(COUGARMON)
        .expect("BT25-035 (Cougarmon) in embedded DSL pack")
        .dsl_card(ARMALIZAMON)
        .expect("BT25-049 (Armalizamon) in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-DIGI", 6000))
        .add_card(make_filler("D1"))
        .deck(0, &["D1", "D1", "D1"])
        .deck(1, &["D1", "D1", "D1"])
        .memory(10)
        .start();

    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let dp_before = runner.effective_dp(opp).expect("opp dp");

    // ── Payoff 1: Cougarmon's [On Play] −3000 DP on the opp Digimon. ──────────
    let cougar = runner.place_on_field(0, COUGARMON, None);
    runner.fire_on_play(0, cougar.index as usize);
    let view = runner
        .pending_selection_view()
        .expect("Cougarmon's −3000 DP target prompt installs (opp Digimon present)");
    let dp_target = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != digimon_engine::action::space::PASS)
        .expect("opp Digimon is a −DP target");
    runner
        .execute_action(view.selecting_player, dp_target)
        .expect("apply −3000 DP");
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        dp_before - 3000,
        "Cougarmon softened the opp Digimon by 3000"
    );

    // ── Payoff 2: Armalizamon's [On Play] suspend on the SAME opp Digimon. ────
    let arm = runner.place_on_field(0, ARMALIZAMON, None);
    runner.fire_on_play(0, arm.index as usize);
    let view2 = runner
        .pending_selection_view()
        .expect("Armalizamon's optional suspend prompt installs");
    let tap_target = view2
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != digimon_engine::action::space::PASS)
        .expect("opp Digimon is a suspend target");
    runner
        .execute_action(view2.selecting_player, tap_target)
        .expect("suspend the same opp Digimon");
    let _ = runner.auto_resolve();

    // ── Claimed mechanical outcome: the two debuffs coexist on one target. ────
    assert!(
        runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "the opp Digimon is suspended (Armalizamon)"
    );
    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        dp_before - 3000,
        "the −3000 DP (Cougarmon) still stands alongside the suspend — both debuffs coexist"
    );

    // ── Lifetime: the −3000 DP is turn-scoped (UntilEachTurnEnd) and expires at
    //    end of turn. (The suspend itself is NOT turn-scoped, but ending P0's
    //    turn begins P1's Unsuspend phase, which bulk-unsuspends P1's Digimon by
    //    the standard turn-start rule — general_rule.pdf active/unsuspend phase —
    //    so we cannot observe the suspend "persisting" via end_turn. The
    //    verifiable lifetime fact is the DP-modifier expiry, asserted here; the
    //    coexistence during P0's turn is asserted above.)
    runner.end_turn();
    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        dp_before,
        "the turn-scoped −3000 DP (Cougarmon) expires at end of turn, unlike the suspend"
    );
}
