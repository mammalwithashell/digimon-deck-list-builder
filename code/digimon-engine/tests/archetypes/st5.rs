//! ST-5 Starter Deck Machine Black — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st-5-machine-black-model.md`. A Black / Cyborg
//! control wall: it stalls behind Blockers, keeps them up (granting
//! Blocker/Reboot + re-using the Tai-Kamiya unsuspend-on-block loop), buffs a
//! blocker with Digi-Burst so it survives a swing, and grinds the opponent down
//! with De-Digivolve (Laser Eye) and a cost-gated delete (Dark Side Attack).
//!
//! Per-card behavioral coverage lives in `tests/cards_behavioral/st5/`; this
//! file asserts the cross-card SYSTEM combos those per-card tests can't see —
//! e.g. a keyword GRANT flipping a conditional INHERITED aura, and the
//! block-then-Tai-unsuspend loop driven off a *real* ST-5 granted Blocker.

#![allow(dead_code)]

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectTiming, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use super::support::snapshot;

// ─── Shared fixtures ─────────────────────────────────────────────────────────
//
// Neutral opponent attackers / bodies / digivolution sources / wall members are
// REAL vanilla black DSL cards (zero effect clauses), pure targets/fillers — never
// combo pieces (the combo pieces are the signature ST-5 cards):
//   ST5-02 Jazamon (Lv3, cost2, 4000)  ST5-07 Jazardmon (Lv4, cost5, 6000)
//   ST5-10 MetalTyrannomon (Lv5, cost6, 9000)
// A cost-≥8 body for the Dark Side Attack window borrows vanilla ST2-10 Plesiomon
// (Lv6, cost10, 12000).

/// Fire a `[When Digivolving]` timing for `handle` (direct enqueue + drain —
/// `place_stack` builds the stack without firing the trigger, so we fire it
/// explicitly to model the digivolve, mirroring `rocks.rs` / `st1.rs`).
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 1 — Blocker + Tai unsuspend reuse, off a REAL ST-5 granted Blocker
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST5-14 Tai Kamiya + ST5-09 MetalGreymon (granting itself ＜Blocker＞ via
/// its [When Digivolving]).
///
/// Expected mechanical outcome: on the opponent's turn the opponent attacks;
/// you declare MetalGreymon as the blocker (it suspends to block); Tai's
/// optional [Opponent's Turn] response is offered; you accept (Tai suspends to
/// pay the cost) and unsuspend MetalGreymon — leaving the blocker READY to block
/// again while Tai is now suspended. This mirrors `st5_14.rs` exactly but proves
/// the loop fires off a *real* ST-5 Blocker SOURCE (MetalGreymon's grant) rather
/// than the synthetic blocker the per-card test uses — the system-level fact
/// that Tai's trigger keys off the act of blocking, not a specific blocker.
///
/// Sources:
/// - Printed text: ST5-14 "[Opponent's Turn] when you use ＜Blocker＞ to suspend
///   one of your Digimon, you may suspend this Tamer to unsuspend 1 of your
///   Digimon"; ST5-09 "[When Digivolving] 1 of your Digimon gains ＜Blocker＞".
/// - Rules: `general_rule.pdf` §16 ＜Blocker＞ (suspend-to-block) + the
///   opponent-turn trigger window.
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST5/Black/ST5_14.cs`, `ST5_09.cs`.
#[test]
fn tai_unsuspends_a_real_metalgreymon_granted_blocker_after_it_blocks() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-14")
        .expect("ST5-14 (Tai) in embedded DSL pack")
        .dsl_card("ST5-09")
        .expect("ST5-09 (MetalGreymon) in embedded DSL pack")
        .dsl_card("ST5-07")
        .expect("ST5-07 (Jazardmon, vanilla opp attacker) in embedded DSL pack")
        .memory(0)
        .start();
    // Opponent's turn — Tai's response and the attack flow both require it.
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 1;

    let tai = runner.place_on_field(0, "ST5-14", Some(0));
    let metalgreymon = runner.place_on_field(0, "ST5-09", Some(0));
    let attacker = runner.place_on_field(1, "ST5-07", Some(0));

    // MetalGreymon's [When Digivolving] grants ＜Blocker＞ to one of your Digimon —
    // direct it onto MetalGreymon itself so it becomes the REAL blocker.
    fire_when_digivolving(&mut runner, metalgreymon);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let grant_pick = runner
        .pending_selection_view()
        .expect("ST5-09 Blocker-grant target selection")
        .valid_action_ids[0];
    runner
        .execute_action(0, grant_pick)
        .expect("grant Blocker to MetalGreymon");
    let _ = runner.auto_resolve();
    assert!(
        runner.game.has_keyword(metalgreymon, Keyword::Blocker),
        "MetalGreymon must hold the granted ＜Blocker＞ before the opponent attacks"
    );

    // Opponent attacks the player → you are offered the chance to block.
    let _ = runner.attack_player(attacker, 0, false);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "a Blocker on the field opens a block-declaration prompt"
    );
    let block_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, block_action)
        .expect("declare MetalGreymon as the blocker");

    // The block suspends MetalGreymon, then Tai's optional response is offered.
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::Replacement | SelectionKind::TriggerOrder)
        ),
        "Tai's optional blocker response should be offered after a real ＜Blocker＞ redirect"
    );
    runner
        .accept_optional_trigger()
        .expect("accept Tai's unsuspend response");

    // Tai's response asks which of your Digimon to unsuspend — pick MetalGreymon.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let unsuspend_pick = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, unsuspend_pick)
        .expect("unsuspend MetalGreymon with Tai");

    assert!(
        runner.game.players[0].battle_area[tai.index as usize].is_suspended,
        "Tai pays the response cost by suspending"
    );
    assert!(
        !runner.game.players[0].battle_area[metalgreymon.index as usize].is_suspended,
        "the real granted blocker is unsuspended again — ready to block a second attack"
    );
}

/// Unhappy path: with Tai on the field you DECLINE to block (the opponent's
/// attack just goes through), so Tai's "when you use ＜Blocker＞" trigger never
/// fires and Tai stays unsuspended. The loop is gated on actually blocking — the
/// system fact a per-card test of either card can't express together.
#[test]
fn tai_loop_does_not_fire_when_you_decline_to_block() {
    use digimon_engine::action::space::PASS;

    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-14")
        .expect("ST5-14 (Tai) in embedded DSL pack")
        .dsl_card("ST5-09")
        .expect("ST5-09 (MetalGreymon) in embedded DSL pack")
        .dsl_card("ST5-07")
        .expect("ST5-07 (Jazardmon, vanilla opp attacker) in embedded DSL pack")
        .memory(0)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 1;

    let tai = runner.place_on_field(0, "ST5-14", Some(0));
    let metalgreymon = runner.place_on_field(0, "ST5-09", Some(0));
    let attacker = runner.place_on_field(1, "ST5-07", Some(0));

    fire_when_digivolving(&mut runner, metalgreymon);
    let grant_pick = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, grant_pick)
        .expect("grant Blocker to MetalGreymon");
    let _ = runner.auto_resolve();

    let _ = runner.attack_player(attacker, 0, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    runner.execute_action(0, PASS).expect("decline the block");

    assert!(
        runner.pending_selection_view().is_none(),
        "declining the block means Tai's ＜Blocker＞ response is never offered"
    );
    assert!(
        !runner.game.players[0].battle_area[tai.index as usize].is_suspended,
        "Tai remains unsuspended when no block happens"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 2 — MetalGreymon grants Blocker → Kapurimon inherited +1000 turns on
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST5-01 Kapurimon (DigiEgg, inherited "[Your Turn] while this Digimon
/// has ＜Blocker＞ it gets +1000 DP") as a SOURCE under ST5-09 MetalGreymon (the
/// carrier that grants itself ＜Blocker＞ via its [When Digivolving]).
///
/// Expected mechanical outcome: a stack [Kapurimon, MetalGreymon] starts with no
/// Blocker on the carrier, so Kapurimon's conditional inherited aura is INACTIVE
/// — effective_dp = MetalGreymon's base 7000. After MetalGreymon's grant gives
/// the carrier ＜Blocker＞ (on your turn), the aura turns ON: effective_dp jumps
/// to 8000 AND the carrier has ＜Blocker＞. A keyword GRANT flipping a conditional
/// INHERITED aura is exactly the cross-card fact neither card's per-card test
/// sees.
///
/// Sources:
/// - Printed text: ST5-01 "[Your Turn] while this Digimon has ＜Blocker＞ it gets
///   +1000 DP" (inherited); ST5-09 "[When Digivolving] 1 of your Digimon gains
///   ＜Blocker＞".
/// - Rules: `general_rule.pdf` §11 (DP — auras sum), §16 ＜Blocker＞; inherited
///   effects resolve from below the top card.
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST5/Black/ST5_01.cs`, `ST5_09.cs`.
#[test]
fn metalgreymon_blocker_grant_turns_on_kapurimon_inherited_plus_1000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-01")
        .expect("ST5-01 (Kapurimon) in embedded DSL pack")
        .dsl_card("ST5-09")
        .expect("ST5-09 (MetalGreymon) in embedded DSL pack")
        .memory(0)
        .start();
    // Your turn — Kapurimon's aura is gated on `[Your Turn]`.
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // Stack [Kapurimon, MetalGreymon] — the inherited-aura source sits under the
    // carrier. (place_stack builds the stack without enforcing digivolve rules.)
    let stack = runner.place_stack(0, &["ST5-01", "ST5-09"]);
    runner.game.tick_declarative_effects();

    // No Blocker yet → Kapurimon's conditional aura is inactive → base 7000.
    assert!(
        !runner.game.has_keyword(stack, Keyword::Blocker),
        "carrier has no Blocker before MetalGreymon's grant"
    );
    assert_eq!(
        runner.effective_dp(stack),
        Some(7_000),
        "MetalGreymon base DP 7000 with the Kapurimon +1000 aura still inactive"
    );

    // MetalGreymon's [When Digivolving] grants ＜Blocker＞ to itself.
    fire_when_digivolving(&mut runner, stack);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let grant_pick = runner
        .pending_selection_view()
        .expect("ST5-09 Blocker-grant target selection")
        .valid_action_ids[0];
    runner
        .execute_action(0, grant_pick)
        .expect("grant Blocker to the carrier itself");
    let _ = runner.auto_resolve();
    runner.game.tick_declarative_effects();

    // Now the carrier has Blocker, so Kapurimon's +1000 inherited aura activates.
    assert!(
        runner.game.has_keyword(stack, Keyword::Blocker),
        "MetalGreymon's grant gives the carrier ＜Blocker＞"
    );
    assert_eq!(
        runner.effective_dp(stack),
        Some(8_000),
        "Kapurimon's inherited +1000 turns ON once the carrier has Blocker (7000 + 1000)"
    );
}

/// Unhappy path: the SAME [Kapurimon, MetalGreymon] stack WITHOUT firing the
/// Blocker grant leaves the carrier without ＜Blocker＞, so Kapurimon's conditional
/// inherited aura never activates and effective_dp stays at the base 7000. The
/// +1000 is gated on the keyword being present — exactly the system fact.
#[test]
fn kapurimon_plus_1000_stays_off_without_a_blocker_grant() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-01")
        .expect("ST5-01 (Kapurimon) in embedded DSL pack")
        .dsl_card("ST5-09")
        .expect("ST5-09 (MetalGreymon) in embedded DSL pack")
        .memory(0)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let stack = runner.place_stack(0, &["ST5-01", "ST5-09"]);
    runner.game.tick_declarative_effects();

    // No Blocker grant fired → aura inactive → DP stays at base.
    assert!(
        !runner.game.has_keyword(stack, Keyword::Blocker),
        "no Blocker on the carrier without the grant"
    );
    assert_eq!(
        runner.effective_dp(stack),
        Some(7_000),
        "without a Blocker, Kapurimon's +1000 never applies — carrier stays at base 7000"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 3 — Machinedramon Reboot keeps the wall up
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST5-12 Machinedramon + two own Digimon (one chosen for Reboot, one not).
///
/// Expected mechanical outcome: Machinedramon's [When Digivolving] grants
/// ＜Reboot＞ to up to 2 of your Digimon; the chosen Digimon gains ＜Reboot＞ (so it
/// unsuspends during the opponent's unsuspend phase and stays up as a blocker
/// across turns) while a non-chosen one does not. This is the wall-persistence
/// engine — the chosen blockers reset before the opponent can punish a tapped
/// wall.
///
/// Sources:
/// - Printed text: ST5-12 "[When Digivolving] up to 2 of your Digimon gain
///   ＜Reboot＞ until the end of your opponent's next turn".
/// - Rules: `general_rule.pdf` §16 ＜Reboot＞ (unsuspend during opp's unsuspend
///   phase).
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST5/Black/ST5_12.cs`.
#[test]
fn machinedramon_grants_reboot_to_chosen_wall_members_only() {
    // Three wall members are real vanilla black Digimon (distinct ids so the
    // Reboot-grant count is unambiguous).
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-12")
        .expect("ST5-12 (Machinedramon) in embedded DSL pack")
        .dsl_card("ST5-02")
        .expect("ST5-02 (Jazamon, vanilla wall member) in embedded DSL pack")
        .dsl_card("ST5-07")
        .expect("ST5-07 (Jazardmon, vanilla wall member) in embedded DSL pack")
        .dsl_card("ST5-10")
        .expect("ST5-10 (MetalTyrannomon, vanilla wall member) in embedded DSL pack")
        .memory(0)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let machinedramon = runner.place_on_field(0, "ST5-12", Some(0));
    let wall_a = runner.place_on_field(0, "ST5-02", Some(0));
    let wall_b = runner.place_on_field(0, "ST5-07", Some(0));
    let wall_c = runner.place_on_field(0, "ST5-10", Some(0));

    fire_when_digivolving(&mut runner, machinedramon);

    // Up-to-2 multi-select; pick two of the three wall members.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 })
    );
    let first = runner.pending_selection_view().unwrap().valid_action_ids[1];
    runner.execute_action(0, first).expect("pick first Reboot target");
    let second = runner.pending_selection_view().unwrap().valid_action_ids[1];
    runner.execute_action(0, second).expect("pick second Reboot target");
    runner.auto_resolve().expect("finish Machinedramon Reboot grant");

    // Two of the chosen wall members hold ＜Reboot＞ and one does not. The exact
    // pair the engine ordered the action ids over is deterministic, so we assert
    // the count: exactly 2 of the 3 wall members have Reboot.
    let reboot_count = [wall_a, wall_b, wall_c]
        .iter()
        .filter(|h| runner.game.has_keyword(**h, Keyword::Reboot))
        .count();
    assert_eq!(
        reboot_count, 2,
        "Machinedramon grants ＜Reboot＞ to exactly the 2 chosen wall members"
    );
    assert!(
        !runner.game.has_keyword(machinedramon, Keyword::Reboot),
        "Machinedramon itself was not chosen, so it has no Reboot"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 4 — BlitzGreymon Digi-Burst buffs a blocker
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST5-13 BlitzGreymon (over 2+ digivolution sources) buffing a separate
/// ST5-03 Agumon Blocker.
///
/// Expected mechanical outcome: BlitzGreymon's [Main] ＜Digi-Burst 2＞ trashes 2 of
/// its own digivolution sources and grants +4000 DP to one of your Digimon —
/// directed onto the Agumon Blocker so the blocker survives blocking a big
/// attacker. Assert the 2 sources are trashed (stack shrinks to top card only,
/// trash +2) AND a +4000 ChangeDp modifier sits on the chosen blocker, lifting
/// it from base 1000 to 5000.
///
/// Sources:
/// - Printed text: ST5-13 "[Main] ＜Digi-Burst 2＞ · 1 of your Digimon gets +4000
///   DP until the end of your opponent's next turn"; ST5-03 native ＜Blocker＞.
/// - Rules: `general_rule.pdf` §16 ＜Digi-Burst＞ (pay by trashing digivolution
///   cards) + §11 (DP modifiers sum).
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST5/Black/ST5_13.cs`.
#[test]
fn blitzgreymon_digi_burst_buffs_an_agumon_blocker() {
    use digimon_engine::action::space::encode_source_select;

    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-13")
        .expect("ST5-13 (BlitzGreymon) in embedded DSL pack")
        .dsl_card("ST5-03")
        .expect("ST5-03 (Agumon) in embedded DSL pack")
        .dsl_card("ST5-02")
        .expect("ST5-02 (Jazamon, vanilla source) in embedded DSL pack")
        .dsl_card("ST5-07")
        .expect("ST5-07 (Jazardmon, vanilla source) in embedded DSL pack")
        .memory(0)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // BlitzGreymon over 2 vanilla digivolution sources (the Digi-Burst fodder).
    let blitz = runner.place_stack(0, &["ST5-02", "ST5-07", "ST5-13"]);
    // The Agumon Blocker we want to protect.
    let agumon = runner.place_on_field(0, "ST5-03", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(agumon, Keyword::Blocker),
        "Agumon (ST5-03) is a native Blocker — the body we want to keep alive"
    );
    assert_eq!(
        runner.effective_dp(agumon),
        Some(1_000),
        "Agumon base DP is 1000 before the Digi-Burst buff"
    );

    let before = snapshot(&runner);

    assert!(
        runner.game.activate_field_main(0, blitz.index as usize),
        "BlitzGreymon's [Main] Digi-Burst should activate"
    );
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
            encode_source_select(blitz.index as u16, 0).expect("first source pick"),
        )
        .expect("pick first Digi-Burst source");
    runner
        .execute_action(
            0,
            encode_source_select(blitz.index as u16, 1).expect("second source pick"),
        )
        .expect("pick second Digi-Burst source");

    // Then direct the +4000 onto the Agumon Blocker.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let view = runner.pending_selection_view().unwrap();
    let agumon_action = digimon_engine::action::space::encode_attack(0, agumon.index as u16);
    assert!(
        view.valid_action_ids.contains(&agumon_action),
        "the Agumon Blocker must be a legal +4000 DP target"
    );
    runner
        .execute_action(0, agumon_action)
        .expect("buff the Agumon Blocker with +4000");
    let _ = runner.auto_resolve();
    runner.game.tick_declarative_effects();

    let after = snapshot(&runner);

    // Digi-Burst 2 trashed both selected sources (stack down to the top card; +2
    // in trash).
    assert_eq!(
        runner.game.player(0).battle_area[blitz.index as usize]
            .card_sources
            .len(),
        1,
        "Digi-Burst 2 trashes both selected digivolution sources"
    );
    assert_eq!(
        after.trash[0],
        before.trash[0] + 2,
        "the two trashed sources land in your trash"
    );
    // The blocker now carries the +4000 window: base 1000 → 5000.
    assert_eq!(
        runner.game.modifiers.sum(agumon, ModifierType::ChangeDp),
        4000,
        "the +4000 Digi-Burst buff sits on the chosen Agumon Blocker"
    );
    assert_eq!(
        runner.effective_dp(agumon),
        Some(5_000),
        "the buffed blocker is now 5000 DP — able to survive blocking a bigger attacker"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 5 — Laser Eye De-Digivolve + Dark Side Attack removal package
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: ST5-15 Laser Eye ([Main] ＜De-Digivolve 1＞ up to 2 opp Digimon, never
/// past Lv.3).
///
/// Expected mechanical outcome: Laser Eye trims one digivolution source off each
/// chosen opponent stack, shrinking the opposing board. Half the deck's removal
/// package — it softens stacks for a finisher.
///
/// Sources:
/// - Printed text: ST5-15 "[Main] ＜De-Digivolve 1＞ up to 2 of your opponent's
///   Digimon".
/// - Rules: `general_rule.pdf` §16 ＜De-Digivolve＞ (trash from the top of the
///   digivolution cards; can't trash past Lv.3).
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST5/Black/ST5_15.cs`.
#[test]
fn laser_eye_trims_two_opponent_stacks() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-15")
        .expect("ST5-15 (Laser Eye) in embedded DSL pack")
        // Deep enough to be trimmable: a Lv.3 floor + a Lv.4 + a Lv.5 active, so
        // De-Digivolve 1 can trash the top digivolution card without hitting the
        // Lv.3 floor (per-card test ST5-15 shows a [Lv3,Lv4] stack is NOT
        // trimmable — the floor protects the lone Lv.3 source). Built from real
        // vanilla black Digimon: ST5-02 (Lv3) / ST5-07 (Lv4) / ST5-10 (Lv5).
        .dsl_card("ST5-02")
        .expect("ST5-02 (Jazamon, vanilla Lv3) in embedded DSL pack")
        .dsl_card("ST5-07")
        .expect("ST5-07 (Jazardmon, vanilla Lv4) in embedded DSL pack")
        .dsl_card("ST5-10")
        .expect("ST5-10 (MetalTyrannomon, vanilla Lv5) in embedded DSL pack")
        .hand(0, &["ST5-15"])
        .memory(10)
        .start();
    // Two opponent stacks, each [Lv3, Lv4, Lv5]. `card_sources` includes the
    // active top, so a 3-card stack has len 3 (2 digivolution sources + top).
    let stack_a = runner.place_stack(1, &["ST5-02", "ST5-07", "ST5-10"]);
    let stack_b = runner.place_stack(1, &["ST5-02", "ST5-07", "ST5-10"]);
    let a_before = runner.game.players[1].battle_area[stack_a.index as usize]
        .card_sources
        .len();
    let b_before = runner.game.players[1].battle_area[stack_b.index as usize]
        .card_sources
        .len();
    assert_eq!(
        a_before, 3,
        "stack A starts as a 3-card pile (2 digivolution sources + the active top)"
    );

    assert!(
        runner.game.activate_hand_main(0, 0),
        "Laser Eye [Main] should activate"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 })
    );
    let first = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, first)
        .expect("select first Laser Eye target");
    let second = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, second)
        .expect("select second Laser Eye target");
    runner.auto_resolve().expect("finish Laser Eye");

    // Each chosen stack loses a digivolution card to De-Digivolve 1 (it stays
    // above the Lv.3 floor, so at least one source is trimmed off each).
    let a_after = runner.game.players[1].battle_area[stack_a.index as usize]
        .card_sources
        .len();
    let b_after = runner.game.players[1].battle_area[stack_b.index as usize]
        .card_sources
        .len();
    assert!(
        a_after < a_before,
        "Laser Eye trims the first opponent stack (sources {a_before} -> {a_after})"
    );
    assert!(
        b_after < b_before,
        "Laser Eye trims the second opponent stack (sources {b_before} -> {b_after})"
    );
}

/// Cards: ST5-16 Dark Side Attack ([Main] delete 1 opp Digimon with play cost
/// ≤ 7).
///
/// Expected mechanical outcome: Dark Side Attack deletes one opponent Digimon
/// whose play cost is 7 or less; a cost-10 body is OUTSIDE the cost window and
/// survives — so the cost-6 body is the only legal target and is the one
/// removed. The finisher half of the removal package, gated on a printed cost
/// window the per-card test already exercises but which we re-assert here as the
/// package's reach.
///
/// Sources:
/// - Printed text: ST5-16 "[Main] delete 1 of your opponent's Digimon with a
///   play cost of 7 or less".
/// - Rules: `general_rule.pdf` deletion semantics + play-cost comparison.
/// - DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST5/Black/ST5_16.cs`.
#[test]
fn dark_side_attack_deletes_only_a_cost_le_7_body() {
    use digimon_engine::action::space::encode_attack;

    // Low body: real vanilla ST5-10 MetalTyrannomon (cost 6, ≤7 → deletable).
    // High body: real vanilla ST2-10 Plesiomon (cost 10, ≥8 → outside the window).
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-16")
        .expect("ST5-16 (Dark Side Attack) in embedded DSL pack")
        .dsl_card("ST5-10")
        .expect("ST5-10 (MetalTyrannomon, cost6 ≤7 body) in embedded DSL pack")
        .dsl_card("ST2-10")
        .expect("ST2-10 (Plesiomon, cost10 ≥8 body) in embedded DSL pack")
        .hand(0, &["ST5-16"])
        .memory(10)
        .start();
    let cost7 = runner.place_on_field(1, "ST5-10", Some(0));
    let cost8 = runner.place_on_field(1, "ST2-10", Some(0));

    let before = snapshot(&runner);

    assert!(
        runner.game.activate_hand_main(0, 0),
        "Dark Side Attack [Main] should activate"
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let view = runner.pending_selection_view().unwrap();
    // Only the ≤7-cost body is a legal target; the cost-10 body is excluded.
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, cost8.index as u16)),
        "the cost-10 Digimon must NOT be a legal Dark Side Attack target"
    );
    assert!(
        view.valid_action_ids
            .contains(&encode_attack(0, cost7.index as u16)),
        "the cost-6 (≤7) Digimon is a legal target"
    );
    runner
        .execute_action(0, encode_attack(0, cost7.index as u16))
        .expect("delete the ≤7-cost body");
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);

    // Exactly the ≤7-cost body is removed; the cost-8 body survives the window.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "exactly one opponent Digimon (the ≤7-cost body) is deleted"
    );
    let survivors: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_name(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        survivors,
        vec!["Plesiomon".to_string()],
        "the cost-10 body (Plesiomon) is the sole survivor; survivors={survivors:?}"
    );
}
