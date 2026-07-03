//! Round-2 engine gap-closure — `G-ENGINE-GRANTED-EFFECT-SELECTION-BODY` (Gap 3).
//!
//! Driver: BT23-032 Shakkoumon (NOT authored here — this exercises the
//! *substrate*). "[When Digivolving] Until your opponent's turn ends, give 1 of
//! their Digimon '[Start of Your Main Phase] This Digimon attacks.'"
//! DCGO `BT23_032.cs` (+ EX10-034 Blastmon, same granted clause).
//!
//! The v1 `grant_triggered_effect` limitation note (grant_triggered.rs) claimed a
//! granted body installing a `PendingSelection` "will not compose … requires
//! extending `QueuedEffect` with a granted-effect discriminator". That extension
//! LANDED (Phase 4i — `QueuedEffect::granted_effect_id`): granted bodies now ride
//! the standard queue and PARK selections like printed effects. These tests are
//! the cited proof, and they add the three dimensions the driver actually needs
//! and that no existing test covered:
//!   1. the granted body drives a MANDATORY attack (`force_attack` on the carrier)
//!      at the OPPONENT's start-of-main-phase — target choice surfaces, no decline;
//!   2. the parked granted selection is CLONE-SAFE (rule 28) — a forked `Game`
//!      resolves it via the resumable VM (`BeginAttack` frame), never the
//!      panic-stub closure;
//!   3. the grant + its body-registry entry are CLEANED UP by expiry
//!      (`end_of_opponents_turn`).

use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::SelectionKind;

/// A substrate-vehicle grantor: an On-Play clause that grants
/// `[Start of Your Main Phase] force_attack: { attacker: carrier }` to every
/// opponent Digimon, expiring at the end of the opponent's turn. This is the
/// BT23-032 / EX10-034 granted clause reduced to the primitive under test — the
/// granted body drives a selection, which is the whole point of the gap.
const GRANTOR_YAML: &str = r#"
card: TEST-GRANT-SELECT-BODY
name: Granted Attack Grantor
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 5000
effects:
  - when: on_play
    process:
      - grant_triggered_effect:
          target:
            of: opponent
            zone: [battle_area]
            kind: digimon
          timing: start_of_your_main_phase
          expiry: end_of_opponents_turn
          body:
            - force_attack:
                attacker: carrier
                targets: any
                prompt: "This Digimon attacks"
"#;

/// Build a runner with the grantor in P0's hand + P1 security so a forced attack
/// has a legal target.
fn setup() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(GRANTOR_YAML)
        .expect("grantor YAML must parse + compile (grant_triggered_effect + force_attack body)")
        .add_card(make_test_card("OPP-A", "Opponent A"))
        .add_card(make_test_card("OPP-B", "Opponent B"))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["TEST-GRANT-SELECT-BODY"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        // Both players hold security so a forced attack into the player checks a
        // security card instead of ending the game (the carrier is P1's Digimon,
        // so it attacks INTO P0's security).
        .security(0, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(20)
        .start()
}

#[test]
fn granted_force_attack_body_parks_mandatory_attack_at_opponent_main_phase() {
    let mut runner = setup();
    // Single opponent Digimon → exactly one granted body fires, isolating the
    // selection-parking behavior (no TriggerOrder over multiple bodies).
    let opp_a = runner.place_on_field(1, "OPP-A", None);

    // Play the grantor; its On-Play grants the force-attack body to the opp Digimon.
    runner.play(0, 0).expect("play grantor");
    assert!(
        runner.game.pending_selection.is_none(),
        "granting is not itself a selection"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(opp_a, EffectTiming::StartOfYourMainPhase)
            .len(),
        1,
        "opponent Digimon must carry the granted Start-of-Main attack body"
    );

    // Advance to P1's turn → StartOfYourMainPhase fires for P1's battle area,
    // draining the granted force_attack body which must PARK a mandatory attack
    // selection whose target choice surfaces (no decline).
    runner.end_turn();
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("granted force_attack must PARK an attack-target selection");
    assert_eq!(
        pending.kind,
        SelectionKind::Target,
        "the granted body drives a SelectAttack (Target) selection"
    );
    assert_eq!(
        pending.selecting_player, 1,
        "the CARRIER's controller (opponent) chooses the attack target"
    );
    assert!(
        !pending.is_optional,
        "mandatory attack (DCGO SetCanNotSelectNotAttack) — no decline"
    );
    assert!(
        !pending.valid_action_ids.contains(&PASS),
        "mandatory attack prompt must not offer PASS"
    );
    // Target choice still surfaces (attack the player / security here).
    assert!(
        pending
            .valid_action_ids
            .contains(&encode_attack(opp_a.index as u16, SECURITY_TARGET)),
        "the carrier's attack-the-player target must be a legal pick"
    );
    // The parked selection MUST carry a resume stack (the clone-safe data path);
    // without it, a cloned Game would hit the panic-stub closure.
    assert!(
        runner.game.pending_selection_resume.is_some(),
        "the parked granted selection must carry a resumable-VM stack (clone-safe)"
    );
}

#[test]
fn granted_force_attack_body_selection_is_clone_safe() {
    // Rule 28: a granted body that parks a selection must be resolvable on a
    // CLONED Game via the resumable VM — the panic-stub closure must never run.
    let mut runner = setup();
    let opp_a = runner.place_on_field(1, "OPP-A", None);
    runner.play(0, 0).expect("play grantor");
    runner.end_turn();

    assert!(
        runner.game.pending_selection.is_some(),
        "precondition: granted force_attack parked a selection"
    );

    // Snapshot P0's security count so we can prove the forced attack actually
    // declared + checked security on the CLONE.
    let p0_security_before = runner.game.player(0).security.len();

    // Clone the whole Game with the granted selection parked. Cloning replaces
    // the closure callback with a panic-stub; the resume frame carries the data.
    let mut cloned = runner.game.clone();
    assert!(
        cloned.pending_selection_resume.is_some(),
        "the clone must inherit the resumable-VM stack (data), not just the panic-stub closure"
    );

    // Resolve the mandatory attack ON THE CLONE. If the resume VM is not driving
    // it, the panic-stub closure fires and this panics.
    let action = encode_attack(opp_a.index as u16, SECURITY_TARGET);
    cloned
        .resolve_selection(1, action)
        .expect("cloned granted force_attack resolves via the resumable VM (no panic-stub)");
    assert!(
        cloned.pending_selection.is_none(),
        "the cloned selection resolved (attack declared), no panic-stub hit"
    );
    // The forced attack was actually DECLARED on the clone: it suspended the
    // carrier and checked one of P0's security cards. (The carrier may or may
    // not survive the security battle; the clone-safety claim is that the
    // resume VM drove the resolution to a real attack, which it did.)
    assert_eq!(
        cloned.player(0).security.len(),
        p0_security_before - 1,
        "the forced attack on the CLONE checked one security card — resolved via data VM"
    );
}

#[test]
fn granted_force_attack_body_expires_and_cleans_up_registry() {
    let mut runner = setup();
    let opp_a = runner.place_on_field(1, "OPP-A", None);
    runner.play(0, 0).expect("play grantor");

    let bodies_after_grant = runner.game.granted_effect_bodies.bodies.len();
    assert!(
        bodies_after_grant >= 1,
        "granting registers >=1 body in the granted-effect-body registry"
    );

    // P0 -> P1: fire the granted attack, then resolve it so P1's turn proceeds.
    runner.end_turn();
    if let Some(pending) = runner.game.pending_selection.as_ref() {
        assert_eq!(pending.selecting_player, 1);
        let action = encode_attack(opp_a.index as u16, SECURITY_TARGET);
        runner
            .game
            .resolve_selection(1, action)
            .expect("resolve granted forced attack");
    }
    assert!(
        !runner.game.game_over,
        "test invariant: the forced attack must not end the game (both players hold security)"
    );
    // End P1's turn — the `end_of_opponents_turn` expiry (opponent of the
    // GRANTOR = P1) must drain the grant off the carriers.
    runner.end_turn(); // P1 -> P0; expires P1's granted entries.

    assert!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(opp_a, EffectTiming::StartOfYourMainPhase)
            .is_empty(),
        "the granted body must be cleaned off the carrier once the opponent's turn ends"
    );
    assert_eq!(
        runner.game.granted_effect_bodies.bodies.len(),
        0,
        "the granted-effect-body registry entry must be pruned on expiry (no leak)"
    );
}

#[test]
fn two_granted_force_attack_bodies_both_park_via_trigger_order() {
    // Composition proof: when TWO opponent Digimon each carry a granted
    // force-attack body, the start-of-main fan-out queues both. The controller
    // picks the resolution order (TriggerOrder), then each granted body parks
    // its own mandatory attack in turn — the SECOND body parks correctly AFTER
    // the first resolves (the queue stays alive across the parked selections).
    let mut runner = setup();
    let opp_a = runner.place_on_field(1, "OPP-A", None);
    let opp_b = runner.place_on_field(1, "OPP-B", None);
    runner.play(0, 0).expect("play grantor");
    runner.end_turn();

    // First: a TriggerOrder prompt (two queued granted bodies for P1 to order).
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("two granted bodies must produce an ordering prompt");
    assert_eq!(
        pending.kind,
        SelectionKind::TriggerOrder,
        "two granted Start-of-Main bodies require a trigger-order pick first"
    );
    assert_eq!(pending.selecting_player, 1);
    // Resolve the trigger order by picking the first offered bundle entry.
    let first_choice = *pending
        .valid_action_ids
        .first()
        .expect("trigger-order prompt offers at least one queued body");
    runner
        .game
        .resolve_selection(1, first_choice)
        .expect("resolve trigger order → run first granted body");

    // The first granted body now parks its mandatory attack.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("first granted body must park its mandatory attack");
    assert_eq!(pending.kind, SelectionKind::Target);
    assert!(!pending.is_optional, "mandatory attack — no decline");
    let (first_attacker, _) = digimon_engine::action::space::decode_attack(
        *pending
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("a legal attack action"),
    );
    // Resolve the first attack (attack security).
    runner
        .game
        .resolve_selection(1, encode_attack(first_attacker, SECURITY_TARGET))
        .expect("resolve first granted attack");

    // The SECOND granted body must then park its own mandatory attack — proving
    // the granted-selection bodies COMPOSE across the queue (the v1 gap).
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("second granted body must park after the first resolves (composition)");
    assert_eq!(pending.kind, SelectionKind::Target);
    assert!(!pending.is_optional);
    let (second_attacker, _) = digimon_engine::action::space::decode_attack(
        *pending
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("a legal attack action for the second body"),
    );
    assert_ne!(
        first_attacker, second_attacker,
        "the two granted bodies force DIFFERENT carriers to attack"
    );
    // Sanity: both attackers are the two opponent Digimon.
    let attackers = [first_attacker as u8, second_attacker as u8];
    assert!(attackers.contains(&opp_a.index) && attackers.contains(&opp_b.index));
}
