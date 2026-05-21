//! EX11-008 Elizamon — Digimon | Lv3 | DP1000 | Cost3 | Red.
//!
//! # Card text (cards.json)
//!
//! [When Moving] [On Play]
//! 1 of your Digimon with the [Reptile] or [Dragonkin] trait gains <Raid>
//! (When this Digimon attacks, you may switch the target of attack to 1 of
//! your opponent's unsuspended Digimon with the highest DP.) and +3000 DP
//! for the turn.
//!
//! Inherited Effect:
//! [Your Turn] [Once Per Turn] When your opponent's security stack is removed
//! from, gain 1 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Red/EX11_008.cs
//!
//! # Engine coverage note
//!
//! ## [When Moving]
//! The printed [When Moving] timing fires when THIS Digimon moves from the
//! breeding area to the battle area. The Rust engine dispatches
//! `EffectTiming::OnMove` with moved-card event context.
//!
//! ## Inherited effects from digivolution stack sources
//! An inherited effect (the lower portion of a digi card) is active only while
//! that card is a digivolution SOURCE sitting beneath another card in a stack
//! (RULES_CONTEXT.md 15-3-1). The engine's `enqueue_from_permanent` runs a
//! dedicated source scan that dispatches such effects. Tests place EX11-008 as
//! a digivolution source under a generic top card to verify the inherited
//! clause from its legitimate position.
//!
//! # Patterns this test covers
//! - D1 temporary DP buff granted to a selected target (end_of_turn expiry)
//! - H9 Raid keyword grant (end_of_turn expiry)
//! - Condition gating: eligible Reptile/Dragonkin must exist on field (DSL
//!   condition; in practice always true since Elizamon itself has Reptile)
//! - Mandatory selection (canNoSelect=false) when the effect fires
//! - Inherited OPT on_opponent_security_removed, Your Turn gate
//!   (tested with EX11-008 as a digivolution source — see note above)

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;

// ────────────────────────────────────────────────────────────────────────────
// Test fixtures
// ────────────────────────────────────────────────────────────────────────────

fn reptile_lv4_card() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("REPTILE-LV4", "Reptile Buddy");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = vec!["Reptile".to_string()];
    card
}

fn dragonkin_lv4_card() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("DRAGONKIN-LV4", "Dragonkin Buddy");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = vec!["Dragonkin".to_string()];
    card
}

fn filler_card() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("FILLER", "Filler");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 0;
    card
}

/// A strong attacker for triggering security checks.
fn attacker_card() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("ATTACKER", "Big Attacker");
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(9000);
    card.play_cost = 0;
    card
}

fn elizamon_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("EX11-008")
        .expect("EX11-008 must be in the embedded DSL pack")
}

// ────────────────────────────────────────────────────────────────────────────
// Section 1: Structural assertions
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn ex11_008_has_two_triggered_clauses() {
    let runner = elizamon_builder().build();
    let compiled = runner
        .compiled_card("EX11-008")
        .expect("EX11-008 compiled card must be present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "EX11-008 must have exactly two triggered clauses"
    );
}

#[test]
fn ex11_008_first_clause_has_on_play_timing() {
    let runner = elizamon_builder().build();
    let compiled = runner
        .compiled_card("EX11-008")
        .expect("EX11-008 compiled card must be present");

    let first_triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::FaceUp => Some(t),
            _ => None,
        })
        .expect("must have a face-up triggered clause");

    assert!(
        first_triggered.when.contains(&CompiledTiming::OnPlay),
        "first clause must include OnPlay timing"
    );
    assert!(
        first_triggered.when.contains(&CompiledTiming::OnMove),
        "first clause must include OnMove timing for [When Moving]"
    );
    assert!(
        !first_triggered.optional,
        "main clause is not optional — selection is mandatory when targets exist"
    );
    assert!(
        !first_triggered.once_per_turn,
        "main clause has no OPT flag"
    );
}

#[test]
fn ex11_008_inherited_clause_shape() {
    let runner = elizamon_builder().build();
    let compiled = runner
        .compiled_card("EX11-008")
        .expect("EX11-008 compiled card must be present");

    let inherited = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .expect("must have an inherited triggered clause");

    assert_eq!(
        inherited.when,
        vec![CompiledTiming::OnOpponentSecurityRemoved],
        "inherited clause triggers on on_opponent_security_removed"
    );
    assert!(
        inherited.once_per_turn,
        "inherited clause must be once_per_turn"
    );
    assert!(
        !inherited.optional,
        "inherited clause is not optional — gain 1 memory is mandatory"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// Section 2: Condition gating (on-play clause)
// ────────────────────────────────────────────────────────────────────────────

/// After Elizamon is played, it is itself on the field with the Reptile trait.
/// The DSL condition (any_permanent with Reptile/Dragonkin in battle_area) will
/// always be satisfied. Verify a selection installs whenever Elizamon is played.
#[test]
fn ex11_008_on_play_installs_selection_after_play() {
    // Plain setup — no other Digimon; Elizamon is the only Reptile.
    let mut runner = elizamon_builder().hand(0, &["EX11-008"]).memory(10).start();

    let _played = runner.play(0, 0).expect("Elizamon plays from hand");

    let kind = runner
        .pending_kind()
        .expect("a pending selection must install when Elizamon (a Reptile) is on field");
    assert_eq!(
        kind,
        digimon_engine::selection::SelectionKind::OwnField,
        "selection kind must be OwnField"
    );
}

#[test]
fn ex11_008_on_play_installs_selection_with_reptile_on_field() {
    let mut runner = elizamon_builder()
        .add_card(reptile_lv4_card())
        .hand(0, &["EX11-008"])
        .memory(10)
        .start();

    runner.place_on_field(0, "REPTILE-LV4", Some(0));

    let _played = runner.play(0, 0).expect("Elizamon plays from hand");

    let kind = runner
        .pending_kind()
        .expect("a pending selection must install when Reptile Digimon is on field");
    assert_eq!(
        kind,
        digimon_engine::selection::SelectionKind::OwnField,
        "selection kind must be OwnField"
    );
}

#[test]
fn ex11_008_on_play_installs_selection_with_dragonkin_on_field() {
    let mut runner = elizamon_builder()
        .add_card(dragonkin_lv4_card())
        .hand(0, &["EX11-008"])
        .memory(10)
        .start();

    runner.place_on_field(0, "DRAGONKIN-LV4", Some(0));

    let _played = runner.play(0, 0).expect("Elizamon plays from hand");

    let kind = runner
        .pending_kind()
        .expect("a pending selection must install when Dragonkin Digimon is on field");
    assert_eq!(
        kind,
        digimon_engine::selection::SelectionKind::OwnField,
        "selection kind must be OwnField"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// Section 3: Behavioral outcomes — main clause
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn ex11_008_on_play_grants_3000_dp_to_reptile_target() {
    let mut runner = elizamon_builder()
        .add_card(reptile_lv4_card())
        .hand(0, &["EX11-008"])
        .memory(10)
        .start();

    let target_h = runner.place_on_field(0, "REPTILE-LV4", Some(0));
    let base_dp = runner
        .effective_dp(target_h)
        .expect("REPTILE-LV4 must have DP");

    let _played = runner.play(0, 0).expect("Elizamon plays");
    runner.auto_resolve().expect("auto_resolve succeeds");

    let after_dp = runner
        .effective_dp(target_h)
        .expect("REPTILE-LV4 must still be on field");
    assert_eq!(
        after_dp,
        base_dp + 3000,
        "REPTILE-LV4 should have +3000 DP after selection"
    );
}

#[test]
fn ex11_008_on_play_grants_raid_keyword_to_reptile_target() {
    let mut runner = elizamon_builder()
        .add_card(reptile_lv4_card())
        .hand(0, &["EX11-008"])
        .memory(10)
        .start();

    let target_h = runner.place_on_field(0, "REPTILE-LV4", Some(0));

    let _played = runner.play(0, 0).expect("Elizamon plays");
    runner.auto_resolve().expect("auto_resolve succeeds");

    let has_raid = runner.game.modifiers.has_keyword(target_h, Keyword::Raid);
    assert!(
        has_raid,
        "REPTILE-LV4 must have Raid keyword modifier after selection"
    );
}

#[test]
fn ex11_008_on_play_grants_3000_dp_to_dragonkin_target() {
    let mut runner = elizamon_builder()
        .add_card(dragonkin_lv4_card())
        .hand(0, &["EX11-008"])
        .memory(10)
        .start();

    let target_h = runner.place_on_field(0, "DRAGONKIN-LV4", Some(0));
    let base_dp = runner
        .effective_dp(target_h)
        .expect("DRAGONKIN-LV4 must have DP");

    let _played = runner.play(0, 0).expect("Elizamon plays");
    runner.auto_resolve().expect("auto_resolve succeeds");

    let after_dp = runner
        .effective_dp(target_h)
        .expect("DRAGONKIN-LV4 must still be on field");
    assert_eq!(
        after_dp,
        base_dp + 3000,
        "DRAGONKIN-LV4 should have +3000 DP after selection"
    );
}

#[test]
fn ex11_008_on_play_can_self_target_when_elizamon_is_only_reptile() {
    // Elizamon itself has the Reptile trait; when it's the only Reptile/Dragonkin
    // on the field after play, it must appear as a valid selection target.
    // DCGO PermanentDigimonCondition does not exclude self.
    let mut runner = elizamon_builder().hand(0, &["EX11-008"]).memory(10).start();

    let _played = runner.play(0, 0).expect("Elizamon plays");

    // Elizamon IS on the field now and it has the Reptile trait.
    // A selection prompt must install.
    let kind = runner.pending_kind();
    assert!(
        kind.is_some(),
        "selection must install when Elizamon itself is the only Reptile on field after play"
    );
}

#[test]
fn ex11_008_dp_buff_expires_after_end_of_turn() {
    let mut runner = elizamon_builder()
        .add_card(reptile_lv4_card())
        .hand(0, &["EX11-008"])
        .memory(10)
        .start();

    let target_h = runner.place_on_field(0, "REPTILE-LV4", Some(0));
    let base_dp = runner
        .effective_dp(target_h)
        .expect("REPTILE-LV4 must have DP");

    let _played = runner.play(0, 0).expect("Elizamon plays");
    runner.auto_resolve().expect("auto_resolve succeeds");

    // Buff is active this turn.
    let during_dp = runner
        .effective_dp(target_h)
        .expect("REPTILE-LV4 must be on field");
    assert_eq!(during_dp, base_dp + 3000, "buff active during play turn");

    // End P0's turn → P1's turn. EndOfTurn expiry clears.
    runner.end_turn();

    let after_end_dp = runner
        .effective_dp(target_h)
        .expect("REPTILE-LV4 must persist");
    assert_eq!(
        after_end_dp, base_dp,
        "DP buff must expire at end of the turn it was granted"
    );
}

#[test]
fn ex11_008_raid_keyword_expires_after_end_of_turn() {
    let mut runner = elizamon_builder()
        .add_card(reptile_lv4_card())
        .hand(0, &["EX11-008"])
        .memory(10)
        .start();

    let target_h = runner.place_on_field(0, "REPTILE-LV4", Some(0));

    let _played = runner.play(0, 0).expect("Elizamon plays");
    runner.auto_resolve().expect("auto_resolve succeeds");

    // Raid active this turn.
    assert!(
        runner.game.modifiers.has_keyword(target_h, Keyword::Raid),
        "Raid must be active during play turn"
    );

    // After end of turn it should expire.
    runner.end_turn();

    assert!(
        !runner.game.modifiers.has_keyword(target_h, Keyword::Raid),
        "Raid must expire after end of turn"
    );
}

#[test]
fn ex11_008_when_moving_installs_selection_after_breeding_move() {
    let mut runner = elizamon_builder().memory(10).start();

    runner.place_in_breeding(0, "EX11-008");
    assert!(
        runner.move_from_breeding(0),
        "Elizamon should move from breeding to battle"
    );

    assert_eq!(
        runner.pending_kind(),
        Some(digimon_engine::selection::SelectionKind::OwnField),
        "[When Moving] must install the Raid/+3000 DP target prompt"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// Section 4: Inherited — on_opponent_security_removed
//
// These tests place EX11-008 as a digivolution SOURCE beneath a generic top
// card on P0's field. An inherited effect is active only while its card is a
// digivolution source under another card (RULES_CONTEXT.md 15-3-1); the
// engine's enqueue_from_permanent source scan dispatches such effects from
// that position.
// ────────────────────────────────────────────────────────────────────────────

/// Helper: build a runner where P0 has a permanent whose digivolution source
/// is EX11-008 (a plain FILLER Digimon is the top card), so the inherited
/// effect is dispatched on P0's turn through the source scan.
///
/// Memory starts at 5 (not 10) so that gain_memory(1) is observable — the
/// memory_range max is 10, and starting at 10 would mean +1 is capped away.
fn build_runner_with_elizamon_on_field(
    extra_builder: impl FnOnce(
        digimon_engine::debug_runner::DebugRunnerBuilder,
    ) -> digimon_engine::debug_runner::DebugRunnerBuilder,
) -> (DebugRunner, PermanentHandle) {
    let builder = elizamon_builder()
        .add_card(attacker_card())
        .add_card(filler_card());
    let builder = extra_builder(builder);
    let mut runner = builder.memory(5).start();

    // Place EX11-008 as a digivolution source under a plain FILLER top card so
    // its inherited effect fires through the source scan. EX11-008 is not the
    // attacker in these tests, so the FILLER top just sits in P0's battle area.
    let elizamon_h = runner.place_stack(0, &["EX11-008", "FILLER"]);

    (runner, elizamon_h)
}

#[test]
fn ex11_008_inherited_gains_memory_when_opponent_security_removed_on_your_turn() {
    let (mut runner, _elizamon_h) = build_runner_with_elizamon_on_field(|b| {
        b.security(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
            .deck(0, &["FILLER", "FILLER", "FILLER"])
            .deck(1, &["FILLER", "FILLER", "FILLER"])
    });

    // Place a strong attacker on P0's field.
    let atk_h = runner.place_on_field(0, "ATTACKER", Some(0));

    let memory_before = runner.memory();

    // P0 attacks P1's security — this removes a security card on P0's turn.
    // The inherited effect ([Your Turn]) should fire from the EX11-008 top card.
    let _result = runner.attack_player(atk_h, 1, true);
    // Auto-resolve any security-effect selections.
    let _ = runner.auto_resolve();

    assert!(
        runner.memory() > memory_before,
        "inherited effect must grant +1 memory when P0 attacks P1's security on P0's turn (before={}, after={})",
        memory_before, runner.memory()
    );
}

#[test]
fn ex11_008_inherited_does_not_fire_when_not_your_turn() {
    // P1 attacks P0's security. The inherited effect on P0's digimon has
    // active_when: { your_turn: true }. Since P1 is the turn player, it
    // should NOT grant P0 +1 memory.
    let (mut runner, _) = build_runner_with_elizamon_on_field(|b| {
        b.security(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
            .deck(0, &["FILLER", "FILLER", "FILLER"])
            .deck(1, &["FILLER", "FILLER", "FILLER"])
    });

    // Switch to P1's turn.
    runner.end_turn();

    // Lower memory so that a gain_memory(1) would be observable if it fired.
    runner.game_mut().set_memory(5);

    // P1 attacks P0's security.
    let atk_h = runner.place_on_field(1, "ATTACKER", Some(0));

    let memory_before = runner.memory();
    let _result = runner.attack_player(atk_h, 0, true);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "inherited effect must NOT grant memory when it's not P0's turn"
    );
}

/// ENGINE GAP: `max_per_turn` (OPT) is not enforced for triggered effects
/// dispatched through the effect queue (`run_queued_effect_inner` has no
/// OPT check). OPT only works for activated [Main] effects. Until the engine
/// tracks `activation_count` during queue drain, this test would fire twice.
/// Tracked in qa/dsl-vocab-gaps.md.
#[test]
fn ex11_008_inherited_opt_blocks_second_trigger_same_turn() {
    let (mut runner, _elizamon_h) = build_runner_with_elizamon_on_field(|b| {
        // P1 needs 3+ security to trigger twice.
        b.security(1, &["FILLER", "FILLER", "FILLER"])
            .deck(0, &["FILLER", "FILLER", "FILLER"])
            .deck(1, &["FILLER", "FILLER", "FILLER"])
    });

    let atk_h = runner.place_on_field(0, "ATTACKER", Some(0));

    // First attack → first security removed → inherited fires, grants +1 memory.
    let mem_before_1 = runner.memory();
    let _ = runner.attack_player(atk_h, 1, true);
    let _ = runner.auto_resolve();
    let mem_after_1 = runner.memory();
    assert!(
        mem_after_1 > mem_before_1,
        "first trigger must grant +1 memory"
    );

    // Unsuspend attacker for second attack.
    runner.game_mut().players[0].battle_area[atk_h.index as usize].is_suspended = false;

    // Second attack same turn → OPT must block.
    let mem_before_2 = runner.memory();
    let _ = runner.attack_player(atk_h, 1, true);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.memory(),
        mem_before_2,
        "OPT must block second trigger in the same turn"
    );
}

#[test]
fn ex11_008_inherited_opt_resets_after_end_of_turn() {
    let (mut runner, _elizamon_h) = build_runner_with_elizamon_on_field(|b| {
        b.security(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
            .deck(0, &["FILLER", "FILLER", "FILLER"])
            .deck(1, &["FILLER", "FILLER", "FILLER"])
    });

    let atk_h = runner.place_on_field(0, "ATTACKER", Some(0));

    // Fire on P0's first turn.
    let _ = runner.attack_player(atk_h, 1, true);
    let _ = runner.auto_resolve();

    // P0's turn ends → P1's turn → P1's turn ends → back to P0.
    runner.end_turn();
    runner.end_turn();

    // P0's second turn — OPT should have reset.
    // Re-place attacker (it may have been suspended/destroyed).
    let atk_h2 = runner.place_on_field(0, "ATTACKER", Some(0));

    let memory_before = runner.memory();
    let _ = runner.attack_player(atk_h2, 1, true);
    let _ = runner.auto_resolve();
    assert!(
        runner.memory() > memory_before,
        "OPT must reset after full turn cycle; second turn should grant +1 memory"
    );
}
