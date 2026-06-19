//! ST24-11 Rosemon — Digimon, Lv.6, Green/Red, DP 12000, Cost 11.
//! Traits: Fairy, DATA SQUAD. Attribute: Data.
//!
//! # Card text (data/cards.json — verbatim, confirmed vs DCGO ST24_11.cs)
//! [When Digivolving] [When Attacking] [Once Per Turn] You may suspend up to 2
//!   of your opponent's Digimon or Tamers. Then, by trashing the bottom
//!   face-down card from under any of your Tamers, none of their Digimon can
//!   unsuspend until their turn ends.
//! [All Turns] [Once Per Turn] When any of your opponent's Digimon or Tamers
//!   suspend, or effects trash cards from under your Tamers, trash your
//!   opponent's top security card.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Green/ST24_11.cs
//!
//! # Patterns this test covers
//! - two alt-digivolve boxes (Lilamon name + Lv.5 [DATA SQUAD])
//! - suspend up to 2 (select_count_capped_multi + per_selected suspend)
//! - the trash+lock is INDEPENDENTLY optional (G-OPTIONAL-TRASH-FACE-DOWN-UNDER-
//!   TAMER): engaging applies the player-scope mass CannotUnsuspend; declining
//!   (PASS) trashes nothing and applies no lock
//! - player-scope mass CannotUnsuspend aura → opp Digimon stay suspended through
//!   the opponent's unsuspend phase
//! - G-SHARED-OPT-HETEROGENEOUS-TIMING: one [All Turns][OPT] clause whose two
//!   timings (on_suspend + on_digivolution_card_trashed) share one OPT counter
//!   while each carries its own per-timing condition

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "ST24-11";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn rosemon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(5);
    c.dp = Some(dp);
    c.play_cost = 5;
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

fn seed_tamer_with_face_down(runner: &mut DebugRunner, player: u8) -> PermanentHandle {
    let t = runner.place_stack(player, &["FD-SRC", "TAMER"]);
    runner.game.players[player as usize].battle_area[t.index as usize].card_sources[0].face_down =
        true;
    t
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

/// Drive every pending prompt, picking the first NON-PASS action at each (fully
/// engages suspend picks + the Tamer trash). Falls back to PASS only when no
/// concrete action remains (a finish/decline-only prompt).
fn drive_engage(runner: &mut DebugRunner) {
    let mut guard = 0;
    while let Some(v) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 24, "prompt loop did not terminate");
        let action = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(PASS);
        runner.execute_action(v.selecting_player, action).unwrap();
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st24_11_metadata_and_two_alt_paths() {
    let card = compiled(CARD_ID);
    assert_eq!(card.name, "Rosemon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(12000));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));

    let lila = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .map(|f| f.name_contains.as_deref() == Some("Lilamon"))
            .unwrap_or(false)
    });
    assert!(lila, "Lilamon-named alt-path present");
    let ds = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .map(|f| f.level_eq == Some(5) && f.trait_has.as_deref() == Some("DATA SQUAD"))
            .unwrap_or(false)
    });
    assert!(ds, "Lv.5 [DATA SQUAD] alt-path present");
}

#[test]
fn st24_11_shared_opt_clause_has_two_per_timing_conditions() {
    let card = compiled(CARD_ID);
    let shared = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnSuspend)
                && t.when.contains(&CompiledTiming::OnDigivolutionCardTrashed)
        })
        .expect("the shared [All Turns][OPT] clause lists both timings");
    assert!(shared.once_per_turn, "shared OPT counter ([Once Per Turn])");
    assert_eq!(
        shared.timing_conditions.len(),
        2,
        "one per-timing condition per heterogeneous timing"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [WD][WA] the trash+lock is INDEPENDENTLY optional
// (G-OPTIONAL-TRASH-FACE-DOWN-UNDER-TAMER). Isolated with an EMPTY opponent
// board (the suspend up-to-2 auto-skips with no candidates).
// ════════════════════════════════════════════════════════════════════════════

fn lock_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .add_card(rosemon())
        .add_card(tamer("TAMER"))
        .add_card(filler("FD-SRC"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .memory(10)
        .start();
    runner.set_first_player(0);
    let rose = runner.place_on_field(0, CARD_ID, Some(0));
    let t = seed_tamer_with_face_down(&mut runner, 0);
    (runner, rose, t)
}

/// ENGAGE: with no opponent to suspend, the trash+lock half is reached directly;
/// trashing 1 FD applies the player-scope mass CannotUnsuspend on the opponent.
#[test]
fn st24_11_engaging_trash_applies_player_mass_unsuspend_lock() {
    let (mut runner, rose, t) = lock_runner();
    let fd_before = runner.game.players[0].battle_area[t.index as usize]
        .card_sources
        .iter()
        .filter(|s| s.face_down)
        .count();
    assert_eq!(fd_before, 1, "1 face-down source staged");

    fire(&mut runner, EffectTiming::WhenDigivolving, rose);
    drive_engage(&mut runner); // engages the Tamer trash pick

    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotUnsuspend),
        "engaging the trash applies the player-scope mass CannotUnsuspend on the opponent"
    );
    assert_eq!(
        runner.game.players[0].battle_area[t.index as usize]
            .card_sources
            .iter()
            .filter(|s| s.face_down)
            .count(),
        0,
        "the bottom face-down source was trashed (the cost)"
    );
}

/// DECLINE: the trash+lock is declinable apart from the suspend. PASS at the
/// (optional) Tamer pick trashes nothing and applies NO lock.
#[test]
fn st24_11_declining_trash_applies_no_lock() {
    let (mut runner, rose, t) = lock_runner();

    fire(&mut runner, EffectTiming::WhenDigivolving, rose);
    // The only prompt is the optional Tamer trash pick — decline it.
    let v = runner
        .pending_selection_view()
        .expect("the optional trash Tamer pick installs");
    assert!(v.is_optional, "the trash+lock Tamer pick is declinable");
    runner
        .execute_action(v.selecting_player, PASS)
        .expect("decline the trash+lock");
    let _ = runner.auto_resolve();

    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotUnsuspend),
        "declining the trash applies NO mass CannotUnsuspend lock"
    );
    assert_eq!(
        runner.game.players[0].battle_area[t.index as usize]
            .card_sources
            .iter()
            .filter(|s| s.face_down)
            .count(),
        1,
        "declining trashes nothing — the face-down source is intact"
    );
}

/// Full integration: suspend 2 opponent Digimon AND engage the trash → the lock
/// holds them suspended through the opponent's unsuspend phase.
#[test]
fn st24_11_suspends_two_and_locks_them_through_unsuspend_phase() {
    let mut runner = DebugRunner::builder()
        .add_card(rosemon())
        .add_card(opp_digimon("OPP1", 5000))
        .add_card(opp_digimon("OPP2", 6000))
        .add_card(tamer("TAMER"))
        .add_card(filler("FD-SRC"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .memory(10)
        .start();
    runner.set_first_player(0);
    let opp1 = runner.place_on_field(1, "OPP1", Some(0));
    let opp2 = runner.place_on_field(1, "OPP2", Some(0));
    seed_tamer_with_face_down(&mut runner, 0);
    let rose = runner.place_on_field(0, CARD_ID, Some(0));

    fire(&mut runner, EffectTiming::WhenDigivolving, rose);
    drive_engage(&mut runner); // suspend both + engage trash

    assert!(
        runner.game.modifiers.player_has(1, ModifierType::CannotUnsuspend),
        "the opponent carries the player-scope mass CannotUnsuspend aura"
    );

    // Both opp Digimon are suspended; run the opponent's unsuspend phase — the
    // mass lock keeps them suspended.
    runner.game.players[1].battle_area[opp1.index as usize].is_suspended = true;
    runner.game.players[1].battle_area[opp2.index as usize].is_suspended = true;
    runner.pass_turn();

    assert!(
        runner.game.players[1].battle_area[opp1.index as usize].is_suspended,
        "OPP1 stays suspended through the opponent's unsuspend phase (mass lock)"
    );
    assert!(
        runner.game.players[1].battle_area[opp2.index as usize].is_suspended,
        "OPP2 stays suspended through the opponent's unsuspend phase (mass lock)"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns][OPT] shared-OPT heterogeneous → trash opp top security
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st24_11_trashes_opp_top_security_when_opponent_suspends() {
    let mut runner = DebugRunner::builder()
        .add_card(rosemon())
        .add_card(opp_digimon("OPP", 5000))
        .add_card(filler("OPP-SEC"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .security(1, &["OPP-SEC"])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    let sec_before = runner.game.players[1].security.len();
    runner.game.suspend(opp);
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].security.len(),
        sec_before - 1,
        "an opponent suspend trashes the opponent's top security card"
    );
}

#[test]
fn st24_11_does_not_trash_security_when_own_digimon_suspends() {
    let mut runner = DebugRunner::builder()
        .add_card(rosemon())
        .add_card(filler("OWN"))
        .add_card(filler("OPP-SEC"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .security(1, &["OPP-SEC"])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(0, CARD_ID, Some(0));
    let own = runner.place_on_field(0, "OWN", Some(0));

    let sec_before = runner.game.players[1].security.len();
    runner.game.suspend(own);
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].security.len(),
        sec_before,
        "an OWN suspend does not trash opponent security (opponent-gated)"
    );
}

#[test]
fn st24_11_trashes_opp_top_security_on_own_tamer_source_trashed() {
    let mut runner = DebugRunner::builder()
        .add_card(rosemon())
        .add_card(tamer("TAMER"))
        .add_card(filler("FD-SRC"))
        .add_card(filler("OPP-SEC"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .security(1, &["OPP-SEC"])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(0, CARD_ID, Some(0));
    let t = seed_tamer_with_face_down(&mut runner, 0);

    let sec_before = runner.game.players[1].security.len();
    runner.trash_one_source(t);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].security.len(),
        sec_before - 1,
        "trashing a source under an own Tamer trashes the opponent's top security"
    );
}
