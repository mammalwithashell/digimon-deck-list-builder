//! BT5-106 Demonic Disaster - Option, Purple, Cost 1.
//!
//! # Card text (cards.json)
//!
//! [Main] You may delete 1 of your Digimon to unsuspend 1 of your purple Digimon.
//!
//! Security Effect [Security] You may play 1 level 3 purple Digimon card from
//! your trash without paying its memory cost. Any [On Play] effects on Digimon
//! played with this effect don't activate.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT5/Purple/BT5_106.cs (submodule not initialized)
//!
//! # Patterns this test covers
//! - Option [Main] with a visible sacrifice selection.
//! - Follow-up visible own purple Digimon unsuspend selection.
//! - Security free-play from trash is blocked pending PUPPETS-G030 On Play
//!   suppression provenance.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

fn digimon(id: &str, color: CardColor) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card
}

/// A Lv.3 purple Digimon used to populate the trash for the [Security]
/// effect-play tests. It carries an [On Play] "gain 1 memory" effect (via
/// `ON_PLAY_GAIN`) so a behavioral test can detect whether the On Play
/// enqueue was suppressed.
fn level_3_purple_digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(3);
    card.play_cost = 3;
    card.dp = Some(3000);
    card
}

/// Hand-written `CardEffect`: a single [On Play] clause that gains 5 memory.
/// 5 is large and distinct so the test can unambiguously tell whether the
/// On Play fired by inspecting the memory delta.
struct OnPlayGainMemory;
impl CardEffect for OnPlayGainMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("On Play: gain 5 memory")
            .process(|ctx| {
                ctx.gain_memory(5);
            })
            .build()]
    }
}

/// Push `card_id` (already registered in `card_data`) into `player`'s trash
/// and return its `CardHandle`.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    let handle = card.handle();
    runner.game.players[player as usize].trash.push(card);
    handle
}

fn option_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT5-106")
        .expect("BT5-106 YAML parses and compiles")
        .add_card(digimon("PURPLE-COST", CardColor::Purple))
        .add_card(digimon("PURPLE-TARGET", CardColor::Purple))
        .add_card(digimon("YELLOW-TARGET", CardColor::Yellow))
        .hand(0, &["BT5-106"])
        .memory(10)
        .start()
}

#[test]
fn bt5_106_is_purple_option_cost_1() {
    let runner = option_runner();
    let compiled = runner
        .compiled_card("BT5-106")
        .expect("BT5-106 compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(1));
    assert!(
        compiled
            .color
            .contains(&digimon_dsl::compiled::CompiledColor::Purple),
        "Demonic Disaster must be a purple Option"
    );
}

#[test]
fn bt5_106_has_main_and_security_clauses() {
    let runner = option_runner();
    let compiled = runner
        .compiled_card("BT5-106")
        .expect("BT5-106 compiled card");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "BT5-106 ships both the Main slice and the Security effect-play slice"
    );
    let main = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("BT5-106 must have a Main clause");
    assert_eq!(main.scope, CompiledScope::FaceUp);

    let security = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("BT5-106 must have an on_security clause");
    assert_eq!(security.scope, CompiledScope::FaceUp);
}

#[test]
fn bt5_106_main_prompts_for_sacrifice_then_purple_unsuspend_target() {
    let mut runner = option_runner();
    let cost = runner.place_on_field(0, "PURPLE-COST", Some(0));
    let target = runner.place_on_field(0, "PURPLE-TARGET", Some(0));
    let yellow = runner.place_on_field(0, "YELLOW-TARGET", Some(0));
    runner.game.suspend(cost);
    runner.game.suspend(target);
    runner.game.suspend(yellow);

    assert!(runner.game.activate_hand_main(0, 0));

    let first = runner
        .pending_selection_view()
        .expect("Main effect must first prompt for the Digimon to delete");
    assert_eq!(first.kind, SelectionKind::OwnField);
    assert!(
        first.is_optional,
        "the sacrifice selection is optional because the card says 'You may'"
    );
    assert_eq!(
        first.valid_action_ids,
        vec![
            encode_attack(0, 0),
            encode_attack(0, 1),
            encode_attack(0, 2)
        ],
        "all own Digimon should be legal sacrifice choices"
    );

    runner
        .execute_action(0, encode_attack(0, 0))
        .expect("choose PURPLE-COST to delete");

    assert_eq!(
        runner.battle_area_size(0),
        2,
        "the selected sacrifice Digimon must be deleted before the unsuspend target is chosen"
    );

    let second = runner
        .pending_selection_view()
        .expect("after paying the cost, Main must prompt for a purple Digimon to unsuspend");
    assert_eq!(second.kind, SelectionKind::OwnField);
    assert!(
        !second.is_optional,
        "the follow-up target choice is mandatory after cost payment"
    );
    assert_eq!(
        second.valid_action_ids,
        vec![encode_attack(0, 0)],
        "only the remaining suspended purple Digimon should be a legal unsuspend target"
    );

    runner
        .execute_action(0, encode_attack(0, 0))
        .expect("choose PURPLE-TARGET to unsuspend");

    assert!(
        !runner.game.player(0).battle_area[target.index as usize - 1].is_suspended,
        "the chosen purple Digimon should be unsuspended"
    );
    assert!(
        runner.game.player(0).battle_area[yellow.index as usize - 1].is_suspended,
        "non-purple Digimon must not be affected"
    );
}

#[test]
fn bt5_106_main_decline_sacrifice_short_circuits_effect() {
    let mut runner = option_runner();
    let cost = runner.place_on_field(0, "PURPLE-COST", Some(0));
    let target = runner.place_on_field(0, "PURPLE-TARGET", Some(0));
    runner.game.suspend(cost);
    runner.game.suspend(target);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .execute_action(0, PASS)
        .expect("decline the optional sacrifice");

    assert!(
        runner.pending_selection().is_none(),
        "declining the optional sacrifice must not continue to the unsuspend choice"
    );
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "no Digimon should be deleted"
    );
    assert!(
        runner.game.player(0).battle_area[target.index as usize].is_suspended,
        "target stays suspended when the effect is declined"
    );
}

#[test]
fn bt5_106_main_requires_suspended_purple_unsuspend_target() {
    let mut runner = option_runner();
    let cost = runner.place_on_field(0, "PURPLE-COST", Some(0));
    let purple = runner.place_on_field(0, "PURPLE-TARGET", Some(0));
    let yellow = runner.place_on_field(0, "YELLOW-TARGET", Some(0));
    runner.game.suspend(cost);
    runner.game.suspend(yellow);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .execute_action(0, encode_attack(0, 0))
        .expect("choose PURPLE-COST to delete");

    assert!(
        runner.pending_selection().is_none(),
        "no follow-up target prompt should install when no suspended purple Digimon remains"
    );
    assert!(
        !runner.game.player(0).battle_area[purple.index as usize - 1].is_suspended,
        "unsuspended purple Digimon should not be offered or changed"
    );
    assert!(
        runner.game.player(0).battle_area[yellow.index as usize - 1].is_suspended,
        "suspended non-purple Digimon should not be offered"
    );
}

// ─── [Security] effect-play with On Play suppression (PUPPETS-G030) ──────────
//
// "[Security] You may play 1 level 3 purple Digimon card from your trash
//  without paying its memory cost. Any [On Play] effects on Digimon played
//  with this effect don't activate."
//
// Test substrate: the played Digimon (`ON_PLAY-PURPLE`) carries a hand-
// written [On Play] effect that gains 5 memory. If the suppression flag
// works, that 5-memory gain MUST NOT happen.

/// Build a runner with BT5-106 in P1's security and `count` level-3 purple
/// On Play Digimon in P1's trash. P0 has an attacker on the field.
fn security_runner(on_play_card_ids: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut builder = DebugRunner::builder()
        .dsl_card("BT5-106")
        .expect("BT5-106 YAML loads")
        .add_card(attacker);
    for id in on_play_card_ids {
        builder = builder.add_card(level_3_purple_digimon(id));
    }
    builder = builder
        // A non-purple distractor so the trash filter is exercised.
        .add_card(digimon("TRASH-YELLOW", CardColor::Yellow))
        .security(1, &["BT5-106"])
        .memory(0);

    let mut runner = builder.start();
    for id in on_play_card_ids {
        runner.register_effect(id, Arc::new(OnPlayGainMemory));
        push_to_trash(&mut runner, 1, id);
    }
    push_to_trash(&mut runner, 1, "TRASH-YELLOW");
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    (runner, attacker)
}

/// The [Security] effect surfaces a player-visible trash selection that
/// offers only the level-3 purple Digimon (not the yellow distractor).
#[test]
fn bt5_106_security_prompts_for_level_3_purple_digimon_in_trash() {
    let (mut runner, attacker) = security_runner(&["ON_PLAY-PURPLE"]);

    runner.attack_player(attacker, 1, false);

    let pending = runner
        .pending_selection_view()
        .expect("BT5-106 [Security] must prompt to play a Digimon from trash");
    assert_eq!(pending.kind, SelectionKind::Trash);
    assert!(
        pending.is_optional,
        "the trash play is optional because the card says 'You may'"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the level-3 purple Digimon is a legal pick; the yellow Digimon must be excluded"
    );
}

/// Resolving the [Security] trash selection plays the chosen Digimon into
/// the controller's battle area AND suppresses its [On Play] effect.
#[test]
fn bt5_106_security_suppresses_on_play_effects_of_played_digimon() {
    let (mut runner, attacker) = security_runner(&["ON_PLAY-PURPLE"]);

    let memory_before = runner.memory();
    runner.attack_player(attacker, 1, false);

    let action = runner
        .pending_selection()
        .expect("BT5-106 [Security] trash selection must be installed")
        .valid_action_ids[0];
    runner
        .execute_action(1, action)
        .expect("P1 picks the level-3 purple Digimon from trash");
    runner.auto_resolve().expect("resolve remaining security flow");

    // The Digimon entered P1's battle area.
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "ON_PLAY-PURPLE"),
        "the chosen level-3 purple Digimon must enter P1's battle area"
    );
    // ...and left the trash.
    assert!(
        !runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "ON_PLAY-PURPLE"),
        "the played Digimon must leave the trash"
    );

    // CRITICAL: its [On Play] (gain 5 memory) must NOT have fired.
    //
    // Memory may have shifted from the attack/security check, but the
    // 5-point On Play gain must be absent. The played Digimon's printed
    // cost is also free, so no cost was paid. We assert the net memory
    // change does not include the +5 On Play gain by checking it stayed
    // at the pre-attack baseline (no other memory-moving effects are in
    // this scenario).
    assert_eq!(
        runner.memory(),
        memory_before,
        "the played Digimon's [On Play] gain-5-memory effect must be suppressed; \
         memory_before={memory_before}, memory_after={}",
        runner.memory()
    );
}

/// Regression: suppression is scoped to the just-played permanent only.
///
/// After BT5-106 [Security] plays `ON_PLAY-PURPLE` with its [On Play]
/// suppressed, a *sibling* Digimon carrying the SAME [On Play] effect,
/// played by an ordinary (non-suppressed) effect-play, must STILL fire its
/// [On Play]. This guards against a broad/global suppression bug that would
/// silence unrelated permanents.
#[test]
fn bt5_106_security_suppression_does_not_silence_a_sibling_on_play() {
    let (mut runner, attacker) = security_runner(&["ON_PLAY-PURPLE"]);

    // Add the sibling On Play card to P1's trash. It is a level-4 purple
    // Digimon — NOT a legal BT5-106 [Security] pick (which requires level
    // 3) — so it stays in trash for the second, non-suppressed play.
    {
        let mut sibling = level_3_purple_digimon("ON_PLAY-SIBLING");
        sibling.level = Some(4);
        runner.game.card_data.push(sibling);
    }
    runner.register_effect("ON_PLAY-SIBLING", Arc::new(OnPlayGainMemory));
    push_to_trash(&mut runner, 1, "ON_PLAY-SIBLING");

    // 1) Resolve the suppressed BT5-106 [Security] play of ON_PLAY-PURPLE.
    runner.attack_player(attacker, 1, false);
    let pending = runner
        .pending_selection_view()
        .expect("trash selection installed");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the level-3 ON_PLAY-PURPLE is a legal BT5-106 pick; the level-4 \
         sibling must be excluded"
    );
    runner
        .execute_action(1, pending.valid_action_ids[0])
        .expect("P1 picks ON_PLAY-PURPLE from trash");
    runner.auto_resolve().expect("resolve security flow");

    // The suppressed play must not have run its [On Play] +5.
    let memory_after_suppressed = runner.memory();

    // 2) Play the sibling On Play Digimon for free from trash with a
    //    NON-suppressed effect-play (`suppress_on_play = false`). This is a
    //    distinct play event — the sibling's [On Play] +5 MUST fire.
    let sibling_idx = runner.game.players[1]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "ON_PLAY-SIBLING")
        .expect("ON_PLAY-SIBLING is in P1's trash");

    // Normalize the seesaw baseline; the free play deducts no memory, so any
    // change after this point is attributable to the sibling's [On Play].
    // `gain_memory` shifts the seesaw toward the effect controller (P1), so a
    // P1-controlled +5 [On Play] reads as -5 on the absolute `memory()` axis.
    runner.game.memory = 0;
    runner
        .game
        .play_from_trash_with_cost_options(
            1,
            sibling_idx,
            digimon_engine::enums::CostDelta::Free,
            digimon_engine::enums::PlaySource::ByEffect,
            /* suppress_on_play = */ false,
        )
        .expect("sibling On Play Digimon plays from trash for free");

    assert_eq!(
        runner.memory(),
        -5,
        "a sibling Digimon played by a NON-suppressed effect-play must still \
         fire its [On Play] (P1 gains 5 memory → -5 on the seesaw); \
         suppression must not leak past the BT5-106 effect-play. \
         memory_after_suppressed={memory_after_suppressed}, memory_after_sibling={}",
        runner.memory()
    );
}

// ─── Failure-mode audit (PR #456 cluster 3f — effect-driven play / sacrifice) ──

/// **Adjacent edge-case (cluster 3f):** BT5-106 [Main] is a "you may"
/// sacrifice. With NO own Digimon on field, the optional sacrifice
/// pool is empty. The effect must short-circuit cleanly: no
/// pending_selection installed, no field changes, no crash. Guards
/// against the empty-candidate-pool branch silently keeping the
/// selection alive.
#[test]
fn bt5_106_main_with_no_own_digimon_short_circuits_without_panic() {
    let mut runner = option_runner();
    // No place_on_field calls — P0 has no Digimon.
    assert_eq!(runner.battle_area_size(0), 0);

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "[Main] still fires; the optional inner step is what's empty"
    );

    // Selection should not park (no targets to pick from).
    assert!(
        runner.game.pending_selection.is_none(),
        "no pending_selection should be installed when sacrifice candidate pool is empty"
    );
    assert_eq!(
        runner.battle_area_size(0),
        0,
        "no field state can change when the optional sacrifice has no targets"
    );
}
