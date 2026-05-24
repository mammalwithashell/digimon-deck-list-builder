//! Regression tests for change `fix-security-check-recompute-mid-attack`.
//!
//! The Rust engine used to compute the player-attack security check count
//! once at attack declaration, locking it for the rest of the resolution.
//! DCGO re-reads `Permanent.Strike` every iteration
//! ([`CardController.cs:3956-3987`] + [`Permanent.cs:1818-1951`]), so a
//! mid-attack digivolve (e.g. via BT21-001 Gigimon's inherited "may
//! digivolve" trigger on `on_opponent_security_removed`) that grants
//! `<Security A. +N>` extends the loop. This test pins the DCGO-parity
//! behavior so it doesn't drift again.
//!
//! Run with:
//!     cargo test --manifest-path code/digimon-engine/Cargo.toml \
//!         --test mid_attack_security_attack_recompute \
//!         -- --nocapture --test-threads=1

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::logger::VerboseLogger;

/// Stand-in security filler card. Plain L4 Red Digimon, no effect.
fn filler(card_id: &str, dp: i32) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: format!("Filler-{card_id}"),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// Patch a DSL-loaded card's `evo_costs` in-place. The DSL test loader
/// (`DebugRunnerBuilder::dsl_card`) drops `evo_costs` because the DSL
/// keeps that info in `alt_paths`. Production `CardData` deserializes
/// `evo_costs` directly from `cards.json`. Tracked separately from this
/// change; we patch in the test so the regression target is the
/// security-check loop, not the loader.
fn patch_evo_costs(runner: &mut DebugRunner, card_id: &str, evo_costs: Vec<EvoCost>) {
    let card = runner
        .game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("patch_evo_costs: card_id {card_id} not loaded"));
    card.evo_costs = evo_costs;
}

/// Resolver: pick the first valid action for every pending selection.
/// For the mid-attack-digivolve scenario this corresponds to: accept
/// each TriggerOrder choice in queue order, pick the first valid Digimon
/// when prompted to choose a digivolve target, pick the first valid
/// hand card when prompted to choose the new top, and accept any
/// fallback PASS. Returns the number of steps taken.
fn drive_to_completion(runner: &mut DebugRunner) -> usize {
    let mut step = 0;
    let max = 200;
    while runner.pending_selection().is_some() && step < max {
        step += 1;
        let (player, action) = {
            let sel = runner.pending_selection().unwrap();
            let id = sel
                .valid_action_ids
                .first()
                .copied()
                .unwrap_or(digimon_engine::action::space::PASS);
            (sel.selecting_player, id)
        };
        if let Err(e) = runner.execute_action(player, action) {
            panic!("execute_action failed at step {step}: {e:?}");
        }
    }
    assert!(step < max, "selection loop exceeded {max} steps");
    step
}

/// Test 1 (Decision 1 / Decision 2 — the load-bearing scenario).
///
/// Stack [BT21-001 Gigimon → BT24-008 Elizamon → BT21-025 Lamiamon] attacks
/// opp security. Gigimon's inherited `on_opponent_security_removed`
/// fires post-check, controller digivolves Lamiamon → BT21-029 Medusamon
/// (`<Security A. +1>`). With per-iteration recompute, total checks = 2.
#[test]
fn mid_attack_digivolve_into_medusamon_extends_security_check_loop() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-001")
        .expect("BT21-001 (Gigimon) must be in embedded DSL pack")
        .dsl_card("BT24-008")
        .expect("BT24-008 (Elizamon) must be in embedded DSL pack")
        .dsl_card("BT21-025")
        .expect("BT21-025 (Lamiamon) must be in embedded DSL pack")
        .dsl_card("BT21-029")
        .expect("BT21-029 (Medusamon) must be in embedded DSL pack")
        .dsl_card("BT21-093")
        .expect("BT21-093 (Raging Serpentine) must be in embedded DSL pack")
        .add_card(filler("SEC-A", 3000))
        .add_card(filler("SEC-B", 3000))
        .hand(0, &["BT21-029"])
        .deck(0, &["SEC-A"; 30])
        .deck(1, &["SEC-A"; 30])
        // Top of security = last element.
        .security(1, &["SEC-A", "SEC-B", "BT21-093"])
        .memory(10)
        .start();

    runner.skip_mulligan();
    runner.game.set_logger(Box::new(VerboseLogger::new()));

    // DSL loader drops evo_costs; production `cards.json` carries them.
    // Patch only what this test exercises.
    patch_evo_costs(
        &mut runner,
        "BT21-029",
        vec![EvoCost { card_color: 0, level: 5, memory_cost: 4 }],
    );

    let p1: PlayerId = 0;
    let p2: PlayerId = 1;

    let host =
        runner.place_stack(p1, &["BT21-001", "BT24-008", "BT21-025"]);

    assert_eq!(runner.security_count(p2), 3, "P2 security should start at 3");
    let attack_result = runner.game.attack_player(host, p2, false);
    assert_eq!(
        attack_result,
        digimon_engine::combat::AttackResult::InProgress,
        "attack must pause for the post-check triggered-effect ordering"
    );

    let steps = drive_to_completion(&mut runner);
    println!("drove {steps} selection steps");

    // ── Load-bearing assertions ────────────────────────────────────
    // Pre-fix this was 1 check (security count would be 2). Post-fix
    // Medusamon's <Security A. +1> is observed on the post-effect
    // recompute and the loop pops a second card.
    assert_eq!(
        runner.security_count(p2),
        1,
        "exactly 2 security checks must execute (3 - 2 = 1 remaining); \
         see fix-security-check-recompute-mid-attack"
    );

    // The digivolve must have actually landed — Medusamon is now the
    // active top card of the attacker's stack.
    let perm = runner
        .game
        .player(p1)
        .battle_area
        .get(host.index as usize)
        .expect("attacker permanent must still exist");
    let top_id = perm.top_card().card_id(&runner.game.card_data);
    assert_eq!(
        top_id, "BT21-029",
        "Lamiamon must have digivolved into BT21-029 Medusamon"
    );
    assert!(
        runner.game_over() == false,
        "game must not be over (security 1 > 0)"
    );
}

/// Test 2 (Decision 1 sanity — recompute returns the same value when the
/// attacker doesn't change).
///
/// A baseline Digimon with `<Security A. +1>` at attack declaration
/// performs exactly 2 security checks against a 3-card security stack.
/// Pre-fix this test passed too (the snapshot value matched the live
/// value), so its purpose is to guard against future regressions where
/// the recompute over-counts on a stable attacker.
#[test]
fn stable_security_attack_plus_one_performs_exactly_two_checks() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-029")
        .expect("BT21-029 (Medusamon) must be in embedded DSL pack")
        .add_card(filler("SEC-A", 3000))
        .add_card(filler("SEC-B", 3000))
        .add_card(filler("SEC-C", 3000))
        .deck(0, &["SEC-A"; 30])
        .deck(1, &["SEC-A"; 30])
        .security(1, &["SEC-A", "SEC-B", "SEC-C"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.game.set_logger(Box::new(VerboseLogger::new()));

    let p1: PlayerId = 0;
    let p2: PlayerId = 1;
    let attacker = runner.place_on_field(p1, "BT21-029", Some(0));

    assert_eq!(runner.security_count(p2), 3);
    let _ = runner.game.attack_player(attacker, p2, false);
    let _ = drive_to_completion(&mut runner);

    assert_eq!(
        runner.security_count(p2),
        1,
        "Medusamon (Security A. +1) must check exactly 2 cards on a fresh attack"
    );
}

/// Test 3 (Decision 3 — DEFERRED).
///
/// Symmetric of test 1: an attacker with `<Security A. +1>` at attack
/// declaration is de-digivolved mid-attack so the new top has no
/// Security A. bonus, and the loop terminates after exactly 1 check.
///
/// Writing this scenario cleanly requires either (a) a security-card
/// effect that bypasses `<Progress>` (none of the BT-set option cards
/// printed today do), or (b) bespoke test-only `CardEffect` plumbing to
/// strip a `SecurityAttackChange` modifier from inside an
/// `OnLoseSecurity` callback. Both are heavier than the recompute itself
/// and orthogonal to the change's correctness — `current_security_strike`
/// is the only path that reads the strike, so the gain case
/// (test 1) and the stable case (test 2) jointly prove the recompute
/// reads live state. Reduction will piggy-back on a future change that
/// adds the missing primitives.
#[test]
#[ignore = "pending: requires either a security effect that bypasses <Progress>, or a test-only CardEffect that mutates SecurityAttackChange mid-resolution; see test docstring"]
fn mid_attack_de_digivolve_reduces_strike_terminates_loop() {
    // Body intentionally empty — see docstring for rationale.
}
