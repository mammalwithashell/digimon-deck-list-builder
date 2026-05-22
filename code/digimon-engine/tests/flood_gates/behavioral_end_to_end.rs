//! End-to-end behavioral test: player-scoped modifier install, `UntilLeaveField`
//! expiry, and Tamer-source discrimination in a realistic card-style scenario.
//!
//! **Scenario (Shamanmon-style TS Olympos Tamer lockout)**
//!
//! Player 0 has a Tamer permanent ("ANCHOR") on their battle area. While ANCHOR
//! is in play, the modifier `CannotGainMemoryExceptFromTamers` is installed on
//! player 1 with `Expiry::UntilLeaveField` tied to ANCHOR's `PermanentHandle`.
//!
//! Test flow:
//!   1. Player 1 tries to gain memory from a Digimon-sourced effect → BLOCKED (no-op)
//!   2. Player 1 gains memory from a Tamer-sourced effect → ALLOWED
//!   3. Player 0's ANCHOR Tamer is deleted → modifier expires via `UntilLeaveField`
//!   4. Player 1 tries again to gain memory from a Digimon-sourced effect → ALLOWED
//!
//! This exercises:
//!   - Player-scoped `ModifierRegistry` install tied to a field permanent
//!   - `UntilLeaveField` expiry triggered transparently by `EffectContext::delete_permanent`
//!   - Tamer-source discrimination (`source_is_tamer` + `CannotGainMemoryExceptFromTamers`)
//!   - Full round-trip: install → enforce (block) → enforce (allow) → expire → re-allow

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;

// ─── Card factory helpers ──────────────────────────────────────────────────────

fn make_tamer(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn make_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(3000),
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
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn make_filler() -> CardData {
    CardData {
        card_id: "FILL-BEE".to_string(),
        card_name: "FILL-BEE".to_string(),
        card_kind: CardKind::DigiEgg,
        level: Some(2),
        dp: Some(1000),
        play_cost: 0,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: "FILL_BEE".to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

// ─── Main test ────────────────────────────────────────────────────────────────

/// Full round-trip: Tamer-anchored memory lockout installs, enforces (blocks
/// Digimon source, allows Tamer source), then expires on permanent deletion.
#[test]
fn shamanmon_style_tamer_anchors_opponent_memory_gain() {
    // ─── Setup ────────────────────────────────────────────────────────────────
    //
    // Player 0: ANCHOR Tamer on battle area (the "Shamanmon-like" lockout Tamer).
    // Player 1: DIG-P1 Digimon and TMR-P1 Tamer, both on battle area — used as
    //   source handles to simulate "player 1's Digimon effect" and "player 1's
    //   Tamer effect" trying to grant player 1 memory.

    let mut r = DebugRunner::builder()
        .add_card(make_tamer("ANCHOR"))
        .add_card(make_digimon("DIG-P1"))
        .add_card(make_tamer("TMR-P1"))
        .add_card(make_filler())
        .deck(0, &["FILL-BEE"])
        .deck(1, &["FILL-BEE"])
        .start();

    // Place all three permanents on their respective players' battle areas.
    let anchor_handle = r.place_on_field(0, "ANCHOR", None);
    let dig_handle = r.place_on_field(1, "DIG-P1", None);
    let tmr_handle = r.place_on_field(1, "TMR-P1", None);

    // Retrieve card handles for use as EffectContext sources.
    let anchor_src = r.game.player(0).battle_area[anchor_handle.index as usize]
        .top_card()
        .handle();
    let dig_src = r.game.player(1).battle_area[dig_handle.index as usize]
        .top_card()
        .handle();
    let tmr_src = r.game.player(1).battle_area[tmr_handle.index as usize]
        .top_card()
        .handle();

    // Install CannotGainMemoryExceptFromTamers on player 1, sourced from ANCHOR,
    // with UntilLeaveField expiry — simulates ANCHOR's passive effect being active
    // while it remains in play.
    r.game.modifiers.add_player_modifier(
        1,
        PlayerModifierEntry::simple(
            ModifierType::CannotGainMemoryExceptFromTamers,
            0,
            Expiry::UntilLeaveField,
            Some(anchor_handle),
            0,
        ),
    );

    assert!(
        r.game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "modifier must be installed on player 1 before any gates fire"
    );

    // ─── Phase 1: Digimon-sourced gain is BLOCKED ─────────────────────────────
    //
    // Player 1's Digimon (DIG-P1) tries to give player 1 memory.
    // EffectContext::player = 1 (beneficiary / gated player).
    // source_card = dig_src (Digimon kind → source_is_tamer() returns false).
    // Expected: gain_memory is a no-op; memory unchanged.

    r.game.set_memory(0);
    let memory_before_phase1 = r.game.memory;

    {
        let mut ctx = EffectContext::new(&mut r.game, dig_src, Some(dig_handle), 1);
        ctx.gain_memory(2);
    }

    assert_eq!(
        r.game.memory, memory_before_phase1,
        "Phase 1: Digimon-sourced gain_memory must be blocked by \
         CannotGainMemoryExceptFromTamers (memory must stay at {})",
        memory_before_phase1
    );

    // ─── Phase 2: Tamer-sourced gain is ALLOWED ───────────────────────────────
    //
    // Player 1's Tamer (TMR-P1) gives player 1 memory.
    // source_card = tmr_src (Tamer kind → source_is_tamer() returns true).
    // Expected: gain_memory succeeds for player 1; from player 0's
    // turn-player perspective the raw memory counter changes by -2.

    r.game.set_memory(0);
    let memory_before_phase2 = r.game.memory;

    {
        let mut ctx = EffectContext::new(&mut r.game, tmr_src, Some(tmr_handle), 1);
        ctx.gain_memory(2);
    }

    assert_eq!(
        r.game.memory,
        memory_before_phase2 - 2,
        "Phase 2: Tamer-sourced gain_memory must be ALLOWED under \
         CannotGainMemoryExceptFromTamers (memory must change from {} to {})",
        memory_before_phase2,
        memory_before_phase2 - 2
    );

    // ─── Phase 3: Delete ANCHOR → modifier expires via UntilLeaveField ────────
    //
    // EffectContext::delete_permanent calls expire_player_on_permanent_leave
    // transparently — no manual expiry call needed.
    // After deletion, the modifier must be gone from player 1's registry.

    {
        // Use a synthetic EffectContext to call delete_permanent on ANCHOR.
        // The source/player here are arbitrary; we only need the mutation path.
        let mut ctx = EffectContext::new(&mut r.game, anchor_src, Some(anchor_handle), 0);
        ctx.delete_permanent(anchor_handle);
    }

    assert!(
        !r.game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "Phase 3: deleting ANCHOR must expire the UntilLeaveField modifier on player 1"
    );

    // ─── Phase 4: Digimon-sourced gain is now ALLOWED ─────────────────────────
    //
    // The modifier is gone, so player 1's Digimon can now gain memory freely.
    // Note: dig_handle is still valid — player 1's battle area was not changed.

    r.game.set_memory(0);
    let memory_before_phase4 = r.game.memory;

    {
        let mut ctx = EffectContext::new(&mut r.game, dig_src, Some(dig_handle), 1);
        ctx.gain_memory(2);
    }

    assert_eq!(
        r.game.memory,
        memory_before_phase4 - 2,
        "Phase 4: after ANCHOR is deleted, Digimon-sourced gain_memory must be \
         ALLOWED (modifier expired; memory must change from {} to {})",
        memory_before_phase4,
        memory_before_phase4 - 2
    );
}
