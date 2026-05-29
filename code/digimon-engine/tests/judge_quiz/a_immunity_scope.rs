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

use digimon_engine::action::PLAY_HAND_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{AttackState, AttackTarget, PendingAttack, TriggerSource};

/// A plain Blue Lv4 Digimon (no effects) — used both as the grantor's
/// color-requirement enabler and as a non-`<Progress>` control carrier.
fn blue_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

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

/// Q2 — RESOLVED 2026-05-29 (G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT closed by
/// change `add-grant-triggered-effect-dsl`): EX1-068's `[Main]` grant is now
/// authored, and the granted-trigger dispatch consults `progress_excludes`, so
/// a `<Progress>` opponent Digimon does not fire the granted "[When Attacking]
/// lose 2 memory" while it attacks. Player 1 (grantor) plays Ice Wall on its
/// turn → Player 0's Medusamon (`<Progress>`) and a vanilla control Digimon
/// both receive the grant → on Player 0's turn each attacks: the vanilla loses
/// 2 memory, Medusamon (Progress) loses none.
#[test]
fn q2_medusamon_progress_blocks_ice_wall_memory_loss() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX1-068")
        .expect("EX1-068 Ice Wall! loads")
        .dsl_card("BT24-017")
        .expect("BT24-017 Medusamon (<Progress>) loads")
        .add_card(blue_digimon("P0BLUE")) // grantor's color-requirement Digimon
        .add_card(blue_digimon("VANILLA")) // non-Progress control carrier (P1)
        .hand(0, &["EX1-068"]) // Player 0 (grantor) holds Ice Wall, plays on turn 0
        .memory(10)
        .start();
    r.skip_mulligan();

    // Grantor (Player 0) needs a Blue Digimon to play the Blue Option.
    let _p0_blue = r.place_on_field(0, "P0BLUE", Some(0));
    // Player 1's board (the GRANTEE side): Medusamon (<Progress>) + a vanilla
    // control Digimon.
    let medusamon = r.place_on_field(1, "BT24-017", Some(0));
    let vanilla = r.place_on_field(1, "VANILLA", Some(0));

    // Player 0 plays Ice Wall! on its own turn (already in Main) → grants every
    // Player-1 Digimon "[When Attacking] lose 2 memory".
    assert_eq!(r.turn_player(), 0, "Player 0 plays Ice Wall on the start turn");
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        r.game.player(0).hand.is_empty(),
        "Ice Wall must leave Player 0's hand (the [Main] play resolved)"
    );

    // Both opponent (Player 1) Digimon received the grant snapshot.
    assert!(
        !r.game
            .modifiers
            .granted_triggered_for_timing(medusamon, EffectTiming::WhenAttacking)
            .is_empty(),
        "Medusamon must carry the Ice-Wall-granted [When Attacking] effect"
    );
    assert!(
        !r.game
            .modifiers
            .granted_triggered_for_timing(vanilla, EffectTiming::WhenAttacking)
            .is_empty(),
        "the vanilla control Digimon must also carry the grant"
    );

    // Control: the non-Progress Digimon is the active attacker → firing
    // WhenAttacking runs the granted body → the turn player loses 2 memory.
    set_active_attacker(&mut r, vanilla);
    r.game.set_memory(5);
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(vanilla));
    r.game.drain_effect_queue();
    assert_eq!(
        r.game.memory, 3,
        "a non-Progress carrier firing [When Attacking] runs the granted \
         'lose 2 memory' (control); got {}",
        r.game.memory
    );

    // Q2: the <Progress> carrier is the active attacker → the granted effect is
    // the opponent's effect, and Progress makes Medusamon immune to opponent
    // effects while attacking → the granted clause does NOT fire → NO loss.
    set_active_attacker(&mut r, medusamon);
    r.game.set_memory(5);
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(medusamon));
    r.game.drain_effect_queue();
    assert_eq!(
        r.game.memory, 5,
        "Medusamon's <Progress> blocks the Ice-Wall-granted memory loss while \
         it is attacking (judge-quiz Q2: NO memory loss); got {}",
        r.game.memory
    );
}

/// Mark `attacker` as the active attacker via a minimal `PendingAttack` so
/// `progress_excludes` engages — the same staging the `progress_mutation_gates`
/// combat tests use to exercise the Progress gate without driving the full
/// attack state machine.
fn set_active_attacker(r: &mut DebugRunner, attacker: PermanentHandle) {
    let target = AttackTarget::Player(if attacker.player == 0 { 1 } else { 0 });
    r.game.pending_attack = Some(PendingAttack {
        attacker,
        original_target: target,
        effective_target: target,
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        declaration_committed: true,
        cancelled: false,
        battle_occurred: false,
        return_phase: GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });
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
