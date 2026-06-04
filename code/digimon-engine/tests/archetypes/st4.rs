//! ST-4 Giga Green — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st-4-giga-green-model.md`. A Green / Insectoid tempo
//! deck whose signature engine is *suspension as a resource*: suspending an
//! opponent's Digimon simultaneously (a) feeds **Izzy Izumi**'s [Your Turn]
//! memory engine and (b) unlocks **Electro Shocker**'s bounce, which is gated on
//! the suspended state. Suspension comes from **Needle Spray** and
//! **HerculesKabuterimon** (Digi-Burst 2). A separate lockdown tool (**Rosemon**)
//! deliberately does NOT suspend — the contrast clarifies the engine.
//!
//! Per-card behavioral coverage lives in `tests/cards_behavioral/st4/`; this file
//! asserts the cross-card SYSTEM combos those per-card tests can't see — the
//! suspend→Izzy→Electro-Shocker loop, the suspended-state gate, the Digi-Burst
//! suspend feeding the same engine, and Rosemon's distinct lockdown.

#![allow(dead_code)]

use digimon_engine::action::space::{encode_attack, encode_source_select};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use super::support::snapshot;

// ─── Shared fixtures ─────────────────────────────────────────────────────────
//
// Neutral opponent bodies / digivolution sources below are REAL vanilla green
// DSL cards (zero effect clauses), chosen as pure targets/fillers — never combo
// pieces (the combo pieces are the signature ST-4 cards):
//   ST4-02 Floramon (Lv3, 4000)  ST4-05 Kunemon (Lv3, 5000)
//   ST4-07 Kuwagamon (Lv4, 6000) ST4-09 Okuwamon (Lv5, 7000)

/// Fire a `[When Digivolving]` timing for `handle` (direct enqueue + drain —
/// `place_stack` builds the stack without firing the trigger, so we fire it
/// explicitly to model the digivolve, mirroring `rocks.rs` / `st5.rs`).
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

/// Resolve any chain of pending selections by auto-resolving (mirrors the ST-4
/// per-card `resolve_all` helper). Izzy's optional [Your Turn] response is
/// auto-accepted here because every test that uses it wants the engine to fire.
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

// ═══════════════════════════════════════════════════════════════════════════
// Combo 1 — Suspend → Izzy memory → Electro Shocker bounce (signature engine)
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST4-15 Needle Spray + ST4-14 Izzy Izumi + ST4-16 Electro Shocker.
///
/// Expected mechanical outcome: on your turn, Needle Spray suspends an opponent
/// Digimon. That suspend fires Izzy's [Your Turn] optional response — Izzy
/// suspends *herself* and you gain exactly +1 memory. Then Electro Shocker,
/// seeing the same Digimon is suspended, returns it to the owner's hand (opp
/// field −1, opp hand +1). The whole three-card loop is the deck's defining play.
///
/// Sources:
/// - Printed text: ST4-15 "[Main] Suspend 1 of your opponent's Digimon"; ST4-14
///   "[Your Turn] When an opponent's Digimon is suspended, by suspending this
///   Tamer, gain 1 memory"; ST4-16 "[Main] Return 1 of your opponent's suspended
///   Digimon to the hand".
/// - Rules: `general_rule.pdf` §16 (suspend/unsuspend timing) + return-to-hand
///   routing. Izzy keys off the opponent's Digimon *becoming* suspended.
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST4/Green/ST4_15.cs`,
///   `ST4_14.cs`, `ST4_16.cs`.
/// - Rank: A.
#[test]
fn needle_spray_suspend_feeds_izzy_then_electro_shocker_bounces() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-15")
        .expect("ST4-15 (Needle Spray) in embedded DSL pack")
        .dsl_card("ST4-14")
        .expect("ST4-14 (Izzy Izumi) in embedded DSL pack")
        .dsl_card("ST4-16")
        .expect("ST4-16 (Electro Shocker) in embedded DSL pack")
        .dsl_card("ST4-02")
        .expect("ST4-02 (Floramon, vanilla opp body) in embedded DSL pack")
        .hand(0, &["ST4-15", "ST4-16"])
        // Start BELOW the memory cap (10) so Izzy's +1 is observable — at the
        // cap a `gain_memory: 1` clamps to a no-op.
        .memory(3)
        .start();
    // Your turn — both Needle Spray's [Main] and Izzy's [Your Turn] need it.
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let izzy = runner.place_on_field(0, "ST4-14", Some(0));
    let target = runner.place_on_field(1, "ST4-02", Some(0));

    let before = snapshot(&runner);
    assert!(
        !runner.game.players[1].battle_area[target.index as usize].is_suspended,
        "the opponent body starts unsuspended"
    );

    // --- Needle Spray [Main]: suspend the opponent body. ---
    assert!(
        runner.game.activate_hand_main(0, 0),
        "Needle Spray [Main] should activate"
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let suspend_pick = encode_attack(0, target.index as u16);
    assert!(
        runner
            .pending_selection_view()
            .unwrap()
            .valid_action_ids
            .contains(&suspend_pick),
        "the unsuspended opponent body is a legal Needle Spray suspend target"
    );
    runner
        .execute_action(0, suspend_pick)
        .expect("suspend the opponent body with Needle Spray");
    resolve_all(&mut runner);

    // The target is suspended; Izzy's response fired (self-suspend, +1 memory).
    assert!(
        runner.game.players[1].battle_area[target.index as usize].is_suspended,
        "Needle Spray suspended the opponent Digimon"
    );
    assert!(
        runner.game.players[0].battle_area[izzy.index as usize].is_suspended,
        "Izzy suspended herself to pay her [Your Turn] response cost"
    );
    assert_eq!(
        runner.memory(),
        before.memory + 1,
        "Izzy's response grants exactly +1 memory off the opponent suspend"
    );

    // --- Electro Shocker [Main]: bounce the now-suspended target. ---
    // `activate_hand_main` activates the option's [Main] effect WITHOUT removing
    // the card from hand, so the hand is still [ST4-15, ST4-16] — Electro Shocker
    // is hand index 1.
    let pre_bounce = snapshot(&runner);
    assert!(
        runner.game.activate_hand_main(0, 1),
        "Electro Shocker [Main] should activate"
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let bounce_pick = encode_attack(0, target.index as u16);
    assert!(
        runner
            .pending_selection_view()
            .unwrap()
            .valid_action_ids
            .contains(&bounce_pick),
        "a SUSPENDED opponent Digimon is a legal Electro Shocker bounce target"
    );
    runner
        .execute_action(0, bounce_pick)
        .expect("bounce the suspended target with Electro Shocker");
    resolve_all(&mut runner);

    let after = snapshot(&runner);
    assert_eq!(
        after.field[1],
        pre_bounce.field[1] - 1,
        "Electro Shocker removed the suspended Digimon from the opponent's field"
    );
    assert_eq!(
        after.hand[1],
        pre_bounce.hand[1] + 1,
        "the bounced Digimon returned to its owner's hand"
    );
}

/// Control / isolation: with NO Izzy on the field, Needle Spray still suspends
/// the opponent body but there is no memory gain — proving the +1 memory in the
/// signature combo is Izzy's contribution, not Needle Spray's.
#[test]
fn needle_spray_alone_suspends_but_grants_no_memory_without_izzy() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-15")
        .expect("ST4-15 (Needle Spray) in embedded DSL pack")
        .dsl_card("ST4-02")
        .expect("ST4-02 (Floramon, vanilla opp body) in embedded DSL pack")
        .hand(0, &["ST4-15"])
        .memory(5)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let target = runner.place_on_field(1, "ST4-02", Some(0));
    let before = snapshot(&runner);

    assert!(runner.game.activate_hand_main(0, 0), "Needle Spray activates");
    let pick = encode_attack(0, target.index as u16);
    runner.execute_action(0, pick).expect("suspend the target");
    resolve_all(&mut runner);

    assert!(
        runner.game.players[1].battle_area[target.index as usize].is_suspended,
        "Needle Spray suspends the target on its own"
    );
    assert_eq!(
        runner.memory(),
        before.memory,
        "without Izzy, the opponent-suspend grants no memory"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 2 — Electro Shocker is gated on the suspended state (unhappy → happy)
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST4-16 Electro Shocker against an UNSUSPENDED opponent Digimon, then
/// after the same Digimon is suspended.
///
/// Expected mechanical outcome: the valid-target set flips on the suspended
/// state. Against an unsuspended body Electro Shocker offers no legal field
/// target (its only out is the optional PASS); once the body is suspended it
/// becomes a legal bounce target. The gate (`is_suspended: true`) is the system
/// point — Electro Shocker only reaches *suspended* threats.
///
/// Sources:
/// - Printed text: ST4-16 "[Main] Return 1 of your opponent's *suspended*
///   Digimon to the hand". The DSL filter is `is_suspended: true`.
/// - Rules: `general_rule.pdf` §16 (suspended state).
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST4/Green/ST4_16.cs`.
/// - Rank: A.
#[test]
fn electro_shocker_target_set_flips_on_suspended_state() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-16")
        .expect("ST4-16 (Electro Shocker) in embedded DSL pack")
        .dsl_card("ST4-02")
        .expect("ST4-02 (Floramon, vanilla opp body) in embedded DSL pack")
        .hand(0, &["ST4-16"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let target = runner.place_on_field(1, "ST4-02", Some(0));
    let target_action = encode_attack(0, target.index as u16);

    // --- Unsuspended: the target is NOT a legal Electro Shocker bounce. ---
    // Electro Shocker's `select_opponent_permanent` is OPTIONAL and filtered to
    // `is_suspended: true`. With NO suspended opponent Digimon there are zero
    // eligible candidates, so the engine installs NO selection prompt at all
    // (the optional select with an empty candidate set just resolves to a no-op
    // — faithful: "Return 1 of your opponent's SUSPENDED Digimon" can target
    // nothing). `pending_kind()` is therefore `None`, and the opponent board is
    // untouched.
    assert!(
        runner.game.activate_hand_main(0, 0),
        "Electro Shocker [Main] activates even with no suspended target"
    );
    assert_eq!(
        runner.pending_kind(),
        None,
        "with NO suspended opponent Digimon, Electro Shocker's filtered optional select has zero candidates and installs no prompt"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the opponent board is untouched while the body is unsuspended"
    );

    // --- Suspend the same body, then Electro Shocker CAN bounce it. ---
    runner.game.players[1].battle_area[target.index as usize].is_suspended = true;
    assert!(
        runner.game.activate_hand_main(0, 0),
        "Electro Shocker [Main] activates again"
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let suspended_view = runner.pending_selection_view().unwrap();
    assert!(
        suspended_view.valid_action_ids.contains(&target_action),
        "once SUSPENDED the same Digimon becomes a legal Electro Shocker target"
    );
    runner
        .execute_action(0, target_action)
        .expect("bounce the suspended body");
    resolve_all(&mut runner);
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the suspended body is bounced off the opponent's field"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 3 — HerculesKabuterimon Digi-Burst suspend → Izzy memory
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST4-13 HerculesKabuterimon (over ≥2 digivolution sources) + ST4-14
/// Izzy Izumi.
///
/// Expected mechanical outcome: HerculesKabuterimon's [Main] <Digi-Burst 2>
/// trashes 2 of its own digivolution cards and suspends 1 opponent Digimon; that
/// suspend feeds the SAME Izzy engine (Izzy self-suspends, +1 memory). A second
/// suspend source into the deck's central engine. Asserts: opp target suspended,
/// 2 sources trashed (stack down to top card, trash +2), Izzy suspended, +1 mem.
///
/// Sources:
/// - Printed text: ST4-13 "[Main] <Digi-Burst 2> Suspend 1 of your opponent's
///   Digimon"; ST4-14 "[Your Turn] ... gain 1 memory".
/// - Rules: `general_rule.pdf` §16 <Digi-Burst> (trash N sources as a cost) +
///   suspend timing.
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST4/Green/ST4_13.cs`,
///   `ST4_14.cs`.
/// - Rank: B.
#[test]
fn hercules_digi_burst_suspend_feeds_izzy_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-13")
        .expect("ST4-13 (HerculesKabuterimon) in embedded DSL pack")
        .dsl_card("ST4-14")
        .expect("ST4-14 (Izzy Izumi) in embedded DSL pack")
        .dsl_card("ST4-05")
        .expect("ST4-05 (Kunemon, vanilla source) in embedded DSL pack")
        .dsl_card("ST4-07")
        .expect("ST4-07 (Kuwagamon, vanilla source) in embedded DSL pack")
        .dsl_card("ST4-02")
        .expect("ST4-02 (Floramon, vanilla opp body) in embedded DSL pack")
        .memory(0)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let izzy = runner.place_on_field(0, "ST4-14", Some(0));
    // HerculesKabuterimon over two vanilla digivolution sources (Digi-Burst fodder).
    let hercules = runner.place_stack(0, &["ST4-05", "ST4-07", "ST4-13"]);
    let target = runner.place_on_field(1, "ST4-02", Some(0));

    let before = snapshot(&runner);

    assert!(
        runner.game.activate_field_main(0, hercules.index as usize),
        "HerculesKabuterimon's [Main] Digi-Burst should activate"
    );
    // First: pick the 2 Digi-Burst sources to trash.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0,
        })
    );
    runner
        .execute_action(
            0,
            encode_source_select(hercules.index as u16, 0).expect("first source pick"),
        )
        .expect("pick first Digi-Burst source");
    runner
        .execute_action(
            0,
            encode_source_select(hercules.index as u16, 1).expect("second source pick"),
        )
        .expect("pick second Digi-Burst source");

    // Then: suspend the opponent body.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner
        .execute_action(0, encode_attack(0, target.index as u16))
        .expect("suspend the opponent body via Digi-Burst");
    resolve_all(&mut runner);

    let after = snapshot(&runner);

    assert!(
        runner.game.players[1].battle_area[target.index as usize].is_suspended,
        "Digi-Burst suspends the chosen opponent Digimon"
    );
    assert_eq!(
        runner.game.player(0).battle_area[hercules.index as usize]
            .card_sources
            .len(),
        1,
        "Digi-Burst 2 trashed both digivolution sources (stack down to the top card)"
    );
    assert_eq!(
        after.trash[0],
        before.trash[0] + 2,
        "the two trashed Digi-Burst sources land in your trash"
    );
    assert!(
        runner.game.players[0].battle_area[izzy.index as usize].is_suspended,
        "the Digi-Burst suspend feeds Izzy — she self-suspends to respond"
    );
    assert_eq!(
        runner.memory(),
        before.memory + 1,
        "Izzy grants +1 memory off the Digi-Burst suspend (the same engine)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 4 — Rosemon lockdown, and its contrast with the suspend engine
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST4-12 Rosemon.
///
/// Expected mechanical outcome: Rosemon's [When Digivolving] applies BOTH
/// CannotAttack and CannotBlock to 1 opponent Digimon until the end of their next
/// turn. This is the deck's alternate lockdown line.
///
/// Sources:
/// - Printed text: ST4-12 "[When Digivolving] 1 of your opponent's Digimon can't
///   attack or block until the end of your opponent's next turn".
/// - Rules: CannotAttack/CannotBlock modifiers with `end_of_opponents_turn`
///   expiry.
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST4/Green/ST4_12.cs`.
/// - Rank: B.
#[test]
fn rosemon_lockdown_modifiers() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-12")
        .expect("ST4-12 (Rosemon) in embedded DSL pack")
        .dsl_card("ST4-09")
        .expect("ST4-09 (Okuwamon, vanilla Lv5 7000 opp body) in embedded DSL pack")
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let rosemon = runner.place_on_field(0, "ST4-12", Some(0));
    let target = runner.place_on_field(1, "ST4-09", Some(0));

    fire_when_digivolving(&mut runner, rosemon);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner
        .execute_action(0, encode_attack(0, target.index as u16))
        .expect("lock down the opponent body with Rosemon");
    resolve_all(&mut runner);

    assert!(
        runner.game.modifiers.has(target, ModifierType::CannotAttack),
        "Rosemon applies CannotAttack to the chosen opponent Digimon"
    );
    assert!(
        runner.game.modifiers.has(target, ModifierType::CannotBlock),
        "Rosemon applies CannotBlock to the chosen opponent Digimon"
    );
    // Crucially: Rosemon does NOT suspend.
    assert!(
        !runner.game.players[1].battle_area[target.index as usize].is_suspended,
        "Rosemon LOCKS the target but does NOT suspend it (distinct from the suspend engine)"
    );
}

/// Contrast (unhappy for the bounce engine): a Rosemon-locked-but-UNSUSPENDED
/// opponent Digimon is NOT a legal Electro Shocker target. Electro Shocker needs
/// the *suspended* state, which Rosemon never grants — so the two lockdown tools
/// are mechanically distinct and do not chain into a bounce.
///
/// Sources:
/// - Printed text: ST4-12 (lock, no suspend) vs ST4-16 ("suspended" filter).
/// - DCGO: `ST4_12.cs`, `ST4_16.cs`.
/// - Rank: B.
#[test]
fn electro_shocker_cannot_bounce_rosemon_locked_unsuspended_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-12")
        .expect("ST4-12 (Rosemon) in embedded DSL pack")
        .dsl_card("ST4-16")
        .expect("ST4-16 (Electro Shocker) in embedded DSL pack")
        .dsl_card("ST4-09")
        .expect("ST4-09 (Okuwamon, vanilla Lv5 7000 opp body) in embedded DSL pack")
        .hand(0, &["ST4-16"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let rosemon = runner.place_on_field(0, "ST4-12", Some(0));
    let target = runner.place_on_field(1, "ST4-09", Some(0));

    // Lock the target with Rosemon.
    fire_when_digivolving(&mut runner, rosemon);
    runner
        .execute_action(0, encode_attack(0, target.index as u16))
        .expect("lock the target with Rosemon");
    resolve_all(&mut runner);
    assert!(
        runner.game.modifiers.has(target, ModifierType::CannotAttack),
        "the target is Rosemon-locked"
    );
    assert!(
        !runner.game.players[1].battle_area[target.index as usize].is_suspended,
        "the Rosemon-locked target is NOT suspended"
    );

    // Electro Shocker still has no legal target — the lock is not a suspend. Its
    // optional `is_suspended: true` select has zero candidates (the target is
    // locked but unsuspended), so the engine installs NO prompt (pending None)
    // and nothing is bounced.
    assert!(
        runner.game.activate_hand_main(0, 0),
        "Electro Shocker [Main] activates"
    );
    assert_eq!(
        runner.pending_kind(),
        None,
        "a Rosemon-locked-but-unsuspended Digimon is not eligible, so Electro Shocker's filtered optional select installs no prompt"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the locked-but-unsuspended target survives Electro Shocker"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 5 — Izzy is optional / costs a suspend (unhappy boundary)
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST4-14 Izzy Izumi (already suspended) + a suspend event.
///
/// Expected mechanical outcome: Izzy's [Your Turn] response is "by suspending
/// this Tamer, gain 1 memory" — an `activation_cost: suspend_self`. If Izzy is
/// ALREADY suspended she cannot pay the cost, so an opponent-suspend event yields
/// no memory (memory unchanged, Izzy stays suspended). This is the boundary of
/// the engine — the same shape as ST5-14 Tai's "cannot pay suspend cost".
///
/// Sources:
/// - Printed text: ST4-14 "[Your Turn] When an opponent's Digimon is suspended,
///   by suspending this Tamer, gain 1 memory".
/// - Rules: activation cost `suspend_self`; an already-suspended permanent cannot
///   pay it.
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST4/Green/ST4_14.cs`.
/// - Rank: C.
#[test]
fn izzy_already_suspended_gains_no_memory_on_opponent_suspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-14")
        .expect("ST4-14 (Izzy Izumi) in embedded DSL pack")
        .dsl_card("ST4-02")
        .expect("ST4-02 (Floramon, vanilla opp body) in embedded DSL pack")
        .memory(0)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let izzy = runner.place_on_field(0, "ST4-14", Some(0));
    let target = runner.place_on_field(1, "ST4-02", Some(0));

    // Izzy is already suspended — she cannot pay the response's suspend cost.
    runner.game.players[0].battle_area[izzy.index as usize].is_suspended = true;
    let before = snapshot(&runner);

    // An opponent Digimon becomes suspended (the trigger condition).
    runner.game.suspend(target);
    resolve_all(&mut runner);

    assert!(
        runner.game.players[1].battle_area[target.index as usize].is_suspended,
        "the opponent Digimon is suspended (the trigger condition is met)"
    );
    assert_eq!(
        runner.memory(),
        before.memory,
        "an already-suspended Izzy cannot pay the suspend cost, so no memory is gained"
    );
    assert!(
        runner.game.players[0].battle_area[izzy.index as usize].is_suspended,
        "Izzy stays suspended — she never paid (and could not pay) the cost"
    );
}
