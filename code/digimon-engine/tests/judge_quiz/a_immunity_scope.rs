//! Cluster A — immunity scope ("affects ME" vs "affects the BATTLE",
//! granted-effect ownership, self-immunity).
//!
//! Questions (see `card-resolution.md`):
//!   Q1  Belphemon: Sleep Mode (BT13-088) [Opp Turn] ends an attack vs
//!       Medusamon (BT24-017) `<Progress>` — judge: YES (Progress guards the
//!       Digimon, not the battle).
//!   Q2  Medusamon (BT24-017) `<Progress>` vs Ice Wall! (EX1-068) "[When
//!       Attacking] lose 2 memory" — judge: NO memory loss.  [READY: both impl]
//!   Q17 Magnamon (X Antibody) (BT16-102) `[When Digivolving]` immunity removes
//!       a Lilithmon (EX6-057)-granted `[EoT] Delete` — judge: NO.
//!   Q18 Quantumon (LM-020) immunity blocks its OWN `<Blast Digivolve>` into
//!       Imperialdramon: Paladin Mode ACE (BT17-077) — judge: NO.
//!   Q28 Gankoomon (X Antibody) (BT20-059) protection beats Dragomon (EX5-060)
//!       "[On Play] don't activate" on Sistermon Ciel (BT6-084) — judge: YES,
//!       plays AND activates.
//!
//! Discover-then-pin: assert the judge answer; a failing assertion is a logged
//! engine gap, never weakened. Scenarios authored under tasks §3.

#![allow(unused_imports)]

use digimon_engine::debug_runner::DebugRunner;

// ── Cluster-A immunity-machinery probe (de-risks Q17/Q18/Q28) ────────────────
//
// Q18 needs the rare "immune to ALL Digimon effects INCLUDING your own"
// (Quantumon). `Game::permanent_is_unaffected_by_effect` (game.rs:3468) supports
// `EffectControllerFilter::{Any, OpponentOnly, OwnOnly}`, and a bare
// `CannotBeAffected` (no filter) means unconditional immunity = Any. This probe
// confirms an own-controller effect IS blocked by Any immunity — i.e. the
// self-immunity machinery EXISTS. (Q18 remains BLOCKED-CARD on Quantumon LM-020
// AND needs verification that the `<Blast Digivolve>` path consults
// `can_affect_permanent` on the digivolving Digimon — a separate, card-gated check.)

/// Self-immunity machinery PRESENT: a `CannotBeAffected` (Any) target is
/// unaffected even by its OWN controller's effect. Confirmed via the canonical
/// `permanent_is_unaffected_by_effect` predicate.
#[test]
fn cluster_a_self_immunity_blocks_own_controller_effect() {
    use digimon_engine::enums::{EffectSourceKind, ModifierType};
    let mut r = DebugRunner::builder()
        .add_card(digimon_engine::debug_runner::make_test_card("IMMUNE", "Immune"))
        .memory(10)
        .start();
    let h = r.place_on_field(0, "IMMUNE", Some(0));
    r.game.modifiers.add(
        h,
        digimon_engine::modifiers::ModifierEntry::simple(
            ModifierType::CannotBeAffected,
            0,
            digimon_engine::enums::Expiry::Permanent,
            0,
        ),
    );
    // effect_controller == target.player (0) — the target's OWN side.
    assert!(
        r.game
            .permanent_is_unaffected_by_effect(h, 0, EffectSourceKind::Digimon),
        "unconditional CannotBeAffected (Any) must block even the target's own \
         controller's effect — the machinery Q18 needs exists"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Q2 — Medusamon (BT24-017) `<Progress>` vs Ice Wall! (EX1-068)
// ─────────────────────────────────────────────────────────────────────────────
//
// Board (card-resolution.md Q2):
//   1. Player A has Medusamon (BT24-017) in the battle area — it carries
//      `<Progress>` ("While attacking, your opponent's effects don't affect
//      this Digimon").
//   2. On Player B's turn, Player B plays Ice Wall! (EX1-068): "[Main] All of
//      your opponent's Digimon gain '[When Attacking] lose 2 memory' until the
//      end of their next turn." → Medusamon gains that effect.
//   3. On Player A's turn, Medusamon declares an attack at Player B.
//   4. JUDGE ANSWER: Player A does NOT lose 2 memory. Medusamon's `<Progress>`
//      makes it immune to the opponent's effects while attacking, so the
//      Ice-Wall-granted "[When Attacking] lose 2 memory" does not fire.
//
// DCGO: DCGO/Assets/Scripts/CardEffect/EX1/Blue/EX1_068.cs (UntilOpponentTurnEnd
// grant of an OnAllyAttack `AddMemory(-2)` ActivateClass on each opp Digimon);
// Progress consumption at the opponent-effect mutation points (combat.rs:2667,
// docs/DCGO_KEYWORD_PARITY.md "Progress").
//
// ── DISCOVERY-WAVE FINDING (2026-05-28) ──────────────────────────────────────
// One half is implemented, the other is BLOCKED:
//   • `<Progress>` immunity IS implemented — `Game::progress_excludes(...)`,
//     consumed at the opponent-effect mutation sites (combat.rs:2667).
//   • Ice Wall! (EX1-068)'s `[Main]` grant is OMITTED from its YAML, BLOCKED on
//     `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` (qa/dsl-vocab-gaps.md). The DSL
//     has no verb to install a triggered effect on an opponent's permanent with
//     a turn-scoped expiry. Only EX1-068's `[Security] gain 2 memory` clause is
//     implemented.
// Because the grant is a no-op today, a test that "played Ice Wall then
// attacked and asserted no memory loss" would PASS FOR THE WRONG REASON (the
// effect was never granted — not because Progress blocked it). Per the suite's
// discover-then-pin rule we do NOT write that false-passing test; the scenario
// is `#[ignore]`-blocked on the named primitive instead.

/// Q2 — faithful scenario is blocked on the Ice Wall grant primitive. See the
/// finding above. Un-ignore once `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT`
/// lands; then stage the grant on Medusamon, attack, and assert Player A's
/// memory is unchanged.
#[test]
#[ignore = "BLOCKED: EX1-068 Ice Wall [Main] grant omitted (G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT, qa/dsl-vocab-gaps.md). Faithful Q2 cannot be staged without a false pass; Progress immunity itself IS implemented (Game::progress_excludes, combat.rs:2667)."]
fn q2_medusamon_progress_blocks_ice_wall_memory_loss() {
    // Body intentionally empty — blocked per #[ignore] above. The intended
    // assertion (Player A memory unchanged after Medusamon attacks while
    // carrying the Ice-Wall-granted "[When Attacking] lose 2 memory") cannot be
    // staged until the grant primitive exists.
    let _ = DebugRunner::builder; // keep the import live; documents the harness.
}

// ── Q1 / Q17 / Q18 / Q28 — BLOCKED-CARD (cards not yet implemented) ───────────
// Each scenario's faithful staging needs ≥1 unimplemented card (see
// card-resolution.md §"Implementation status"). Stubs assert the judge answer in
// their docstring and are `#[ignore]`-d on the specific missing card(s); promote
// to a real test once authored (cluster-A authoring, tasks §3).

/// Q1 — Belphemon: Sleep Mode (BT13-088) [Opp Turn] CAN end Medusamon's
/// (BT24-017) attack; `<Progress>` guards the Digimon, not the battle. Judge: YES.
#[test]
#[ignore = "BLOCKED-CARD: needs BT13-088 (Belphemon: Sleep Mode). BT24-017 is implemented."]
fn q1_belphemon_opp_turn_ends_attack_through_progress() {}

/// Q17 — Magnamon (X Antibody) (BT16-102) [When Digivolving] immunity removes the
/// Lilithmon (EX6-057)-granted "[EoT] Delete this". Judge: NO (does not activate).
#[test]
#[ignore = "BLOCKED-CARD: needs BT16-102 (Magnamon X), BT21-036 (Magnamon), EX6-057 (Lilithmon)."]
fn q17_magnamon_x_immunity_removes_granted_eot_delete() {}

/// Q18 — Quantumon (LM-020) immunity is to ALL Digimon effects incl. its own;
/// `<Blast Digivolve>` into Imperialdramon: PM ACE (BT17-077) is an effect. Judge: NO.
#[test]
#[ignore = "BLOCKED-CARD: needs LM-020 (Quantumon). BT17-077 is implemented."]
fn q18_quantumon_self_immunity_blocks_own_blast_digivolve() {}

/// Q28 — Gankoomon (X Antibody) (BT20-059) protection beats Dragomon (EX5-060)
/// "[On Play] don't activate" on Sistermon Ciel (BT6-084). Judge: YES, plays AND activates.
#[test]
#[ignore = "BLOCKED-CARD: needs BT20-059 (Gankoomon X), EX5-060 (Dragomon). BT23-057, BT6-084 implemented."]
fn q28_gankoomon_x_protection_beats_dragomon_on_play_lock() {}
