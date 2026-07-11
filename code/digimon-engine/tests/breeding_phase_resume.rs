//! Breeding-phase turn-machine regression tests (user bug report, 2026-07-09).
//!
//! Bug 1 — "Turn resets sometimes, allowing for a second breeding phase and
//! second start of main trigger":
//!
//! `decode_breeding` (HATCH / MOVE_FROM_BREEDING) used to call
//! `enter_main_phase()` unconditionally after the breeding action succeeded.
//! If the action's trigger drain parked a `PendingSelection` (e.g. EX11-008
//! Elizamon's `[When Moving]` Raid/+3000 DP target pick), the selection had
//! captured `previous_phase = Breeding`. `enter_main_phase()` then advanced
//! the phase to `Main` and enqueued `StartOfYourMainPhase` — but when the
//! selection later resolved, `resolve_selection` restored
//! `current_phase = previous_phase = Breeding`, dropping the turn machine
//! back into a phantom second Breeding phase. From there PASS/HATCH would
//! run `enter_main_phase()` again, firing `StartOfYourMainPhase` a second
//! time in the same turn.
//!
//! Bug 2 — "Breeding area not working (observed on Eliza/Dimetro
//! digivolution)": in the phantom Breeding phase all breeding-area
//! digivolve actions are rejected (`digivolve_breeding: not in Main phase`)
//! and never surfaced in the mask — the visible symptom of the same root
//! cause. The Elizamon → Dimetromon breeding digivolve line itself is
//! exercised here against the printed digivolution circles (BT21-008: Red
//! Lv.2 / cost 0; BT21-017: Red Lv.3 / cost 2 — official Bandai DB).

#![cfg(feature = "dsl-yaml-loader")]

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_digivolve, BREEDING_TARGET, HATCH, MOVE_FROM_BREEDING, PASS,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, GamePhase};
use digimon_engine::events::GameEvent;

// ─── Fixtures ───────────────────────────────────────────────────────────────

fn filler_card() -> CardData {
    let mut card = make_test_card("FILLER", "Filler");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 0;
    card
}

fn plain_lv3_card() -> CardData {
    let mut card = make_test_card("PLAIN-LV3", "Plain Rookie");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card
}

fn egg_lv2_card() -> CardData {
    let mut card = make_test_card("EGG-LV2", "Test Egg");
    card.card_kind = CardKind::DigiEgg;
    card.level = Some(2);
    card.dp = Some(0);
    card.play_cost = 0;
    card
}

/// BT21-008 Elizamon printed data (official Bandai DB): Lv.3 Red, DP 1000,
/// cost 3, Reptile/LIBERATOR, standard circle Red Lv.2 / cost 0. Registered
/// via `add_card` BEFORE `dsl_card` so this CardData (with the printed
/// evo_costs, which the DSL loader does not populate) wins while the DSL
/// effects still attach.
fn elizamon_bt21_card() -> CardData {
    let mut card = make_test_card("BT21-008", "Elizamon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 3;
    card.traits = vec!["Reptile".to_string(), "LIBERATOR".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: 0, // Red
        level: 2,
        memory_cost: 0,
    }];
    card
}

/// BT21-017 Dimetromon printed data (official Bandai DB): Lv.4 Red, DP 4000,
/// cost 4, Reptile/LIBERATOR, standard circle Red Lv.3 / cost 2.
fn dimetromon_bt21_card() -> CardData {
    let mut card = make_test_card("BT21-017", "Dimetromon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.traits = vec!["Reptile".to_string(), "LIBERATOR".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: 0, // Red
        level: 3,
        memory_cost: 2,
    }];
    card
}

/// Drive the game to the given player's next Breeding phase by passing turns.
/// Panics if the turn machine doesn't park at Breeding for that player.
fn pass_until_breeding_phase(r: &mut DebugRunner, player: u8) {
    for _ in 0..4 {
        if r.game.turn_player() == player && r.game.current_phase == GamePhase::Breeding {
            return;
        }
        r.game.pass_turn();
    }
    assert_eq!(r.game.turn_player(), player, "expected player {player}'s turn");
    assert_eq!(
        r.game.current_phase,
        GamePhase::Breeding,
        "expected the turn machine to park at the Breeding phase"
    );
}

// ─── Bug 1: phase must not reset to Breeding after a move-triggered pick ────

/// EX11-008 Elizamon's `[When Moving]` clause installs a mandatory OwnField
/// selection when it moves from breeding to the battle area. Resolving that
/// selection must leave the turn machine in the Main phase — NOT reset it to
/// a second Breeding phase — and `StartOfYourMainPhase` must be entered
/// exactly once for the turn.
#[test]
fn move_from_breeding_selection_does_not_reset_phase_to_breeding() {
    let mut r = DebugRunner::builder()
        .add_card(filler_card())
        .dsl_card("EX11-008")
        .expect("EX11-008 must be in the embedded DSL pack")
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    r.place_in_breeding(0, "EX11-008");
    pass_until_breeding_phase(&mut r, 0);

    let checkpoint = r.event_checkpoint();

    // Player action: move Elizamon out of breeding (full decode path — this
    // is the path the real game drives, including the enter-main handoff).
    r.game.decode_action(MOVE_FROM_BREEDING, 0);

    // The [When Moving] clause must have parked a selection.
    let sel = r
        .pending_selection()
        .expect("[When Moving] must install the Raid/+3000 DP target prompt");
    let action_id = *sel
        .valid_action_ids
        .first()
        .expect("selection must have at least one valid target");
    let selecting_player = sel.selecting_player;

    // Resolve the selection through the real decode path.
    r.game.decode_action(action_id, selecting_player);
    assert!(
        r.pending_selection().is_none(),
        "selection must be fully resolved"
    );

    // Core regression: the turn machine must now be in the Main phase, not
    // back in a phantom Breeding phase.
    assert_eq!(
        r.game.current_phase,
        GamePhase::Main,
        "phase must be Main after the move-triggered selection resolves \
         (phantom second Breeding phase = user-reported turn reset)"
    );

    // The Main phase must have been entered exactly once for this turn.
    let main_entries = r
        .events_since(checkpoint)
        .iter()
        .filter(|e| {
            matches!(
                e,
                GameEvent::PhaseChange {
                    phase: GamePhase::Main,
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        main_entries, 1,
        "StartOfYourMainPhase / PhaseChange(Main) must fire exactly once per turn"
    );

    // PASS must now end the turn (Main-phase semantics), not re-enter Main
    // from a phantom Breeding phase.
    r.game.decode_action(PASS, 0);
    assert_eq!(
        r.game.turn_player(),
        1,
        "PASS after the selection must end the turn (turn passes to P1)"
    );
    let main_entries_after_pass = r
        .events_since(checkpoint)
        .iter()
        .filter(|e| {
            matches!(
                e,
                GameEvent::PhaseChange {
                    phase: GamePhase::Main,
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        main_entries_after_pass, 2,
        "exactly one Main entry for P0's turn and one for P1's turn"
    );
}

/// Baseline: moving a plain Digimon (no [When Moving] effect) out of breeding
/// via the decode path still advances straight to the Main phase.
#[test]
fn move_from_breeding_without_selection_enters_main_immediately() {
    let mut r = DebugRunner::builder()
        .add_card(filler_card())
        .add_card(plain_lv3_card())
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    r.place_in_breeding(0, "PLAIN-LV3");
    pass_until_breeding_phase(&mut r, 0);

    r.game.decode_action(MOVE_FROM_BREEDING, 0);

    assert!(r.pending_selection().is_none(), "no selection expected");
    assert_eq!(
        r.game.current_phase,
        GamePhase::Main,
        "plain breeding move must advance to Main immediately"
    );
    assert_eq!(r.game.player(0).battle_area.len(), 1);
    assert!(r.game.player(0).breeding_area.is_none());
}

/// Baseline: hatching via the decode path advances straight to Main.
#[test]
fn hatch_via_decode_enters_main_immediately() {
    let mut r = DebugRunner::builder()
        .add_card(filler_card())
        .add_card(egg_lv2_card())
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .digitama(0, &["EGG-LV2"])
        .memory(10)
        .start();

    pass_until_breeding_phase(&mut r, 0);

    r.game.decode_action(HATCH, 0);

    assert_eq!(
        r.game.current_phase,
        GamePhase::Main,
        "hatch must advance to Main immediately"
    );
    assert!(
        r.game.player(0).breeding_area.is_some(),
        "egg must be in the breeding area after hatching"
    );
}

// ─── Bug 2: Elizamon → Dimetromon breeding-area digivolution ────────────────

/// The full user line: egg (Lv.2 Red) in breeding → digivolve BT21-008
/// Elizamon onto it (Red Lv.2 circle, cost 0) → digivolve BT21-017
/// Dimetromon onto Elizamon (Red Lv.3 circle, cost 2). Both steps happen in
/// the Main phase via the real mask + decode path.
#[test]
fn elizamon_to_dimetromon_breeding_digivolve_works() {
    let mut r = DebugRunner::builder()
        .add_card(filler_card())
        .add_card(egg_lv2_card())
        .add_card(elizamon_bt21_card())
        .add_card(dimetromon_bt21_card())
        .dsl_card("BT21-008")
        .expect("BT21-008 must be in the embedded DSL pack")
        .dsl_card("BT21-017")
        .expect("BT21-017 must be in the embedded DSL pack")
        .hand(0, &["BT21-008", "BT21-017"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(10)
        .start();

    r.place_in_breeding(0, "EGG-LV2");
    assert_eq!(r.game.current_phase, GamePhase::Main);

    // Step 1: Elizamon (hand 0) onto the Lv.2 egg — Red Lv.2 / cost 0 circle.
    let digivolve_elizamon = encode_digivolve(0, BREEDING_TARGET);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[digivolve_elizamon as usize], 1.0,
        "mask must offer digivolving Elizamon onto the breeding-area egg"
    );

    let memory_before = r.memory();
    r.game.decode_action(digivolve_elizamon, 0);

    {
        let breeding = r
            .game
            .player(0)
            .breeding_area
            .as_ref()
            .expect("breeding area must still be occupied");
        assert_eq!(
            breeding.top_card().card_id(&r.game.card_data),
            "BT21-008",
            "Elizamon must be the breeding-area top card"
        );
        assert_eq!(breeding.card_sources.len(), 2, "stack must be egg + Elizamon");
    }
    assert_eq!(r.memory(), memory_before, "Elizamon circle costs 0 memory");
    assert_eq!(
        r.game.player(0).hand.len(),
        2,
        "digivolve consumes Elizamon from hand and draws 1 (Dimetromon + drawn card)"
    );

    // Step 2: Dimetromon (now hand 0) onto Elizamon — Red Lv.3 / cost 2 circle.
    let digivolve_dimetromon = encode_digivolve(0, BREEDING_TARGET);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[digivolve_dimetromon as usize], 1.0,
        "mask must offer digivolving Dimetromon onto breeding-area Elizamon"
    );

    let memory_before = r.memory();
    r.game.decode_action(digivolve_dimetromon, 0);

    {
        let breeding = r
            .game
            .player(0)
            .breeding_area
            .as_ref()
            .expect("breeding area must still be occupied");
        assert_eq!(
            breeding.top_card().card_id(&r.game.card_data),
            "BT21-017",
            "Dimetromon must be the breeding-area top card"
        );
        assert_eq!(
            breeding.card_sources.len(),
            3,
            "stack must be egg + Elizamon + Dimetromon"
        );
        assert_eq!(
            breeding.level(&r.game.card_data),
            Some(4),
            "breeding-area Digimon must now be level 4"
        );
    }
    assert_eq!(
        r.memory(),
        memory_before - 2,
        "Dimetromon circle costs 2 memory"
    );

    // Level ≥ 3: the breeding Digimon may move out next Breeding phase.
    pass_until_breeding_phase(&mut r, 0);
    r.game.decode_action(MOVE_FROM_BREEDING, 0);
    assert_eq!(
        r.game.player(0).battle_area.len(),
        1,
        "Dimetromon stack must move to the battle area"
    );
    assert_eq!(r.game.current_phase, GamePhase::Main);
}
