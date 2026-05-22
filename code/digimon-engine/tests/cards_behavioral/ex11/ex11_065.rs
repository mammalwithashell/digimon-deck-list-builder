//! EX11-065 Close — Tamer, Purple, Cost 3, Trait: [LIBERATOR]
//!
//! # Card text (cards.json / authoritative printed text)
//!
//! "[Start of Your Main Phase] By trashing 1 [Mineral] or [Rock] trait card
//! from your hand or your Digimon's digivolution cards, gain 1 memory."
//!
//! "[All Turns] When your Digimon are played or digivolve, if any of them have
//! the [Mineral] or [Rock] trait, by suspending this Tamer, you may place 1
//! [Mineral] or [Rock] trait card from your hand or trash as any of those
//! Digimon's bottom digivolution card."
//!
//! "Security Effect [Security] Play this card without paying the cost."
//!
//! # DCGO C# reference
//! DCGO submodule uninitialized — no C# reference available.
//!
//! # Patterns this test covers
//! - B1  Start-of-main tamer (memory swing with cost payment)
//! - B3  Trigger-on-event tamer (All Turns played/digivolve observer)
//! - A6  Trash-to-digi-stack (place_as_bottom_source)
//! - E2  Optional with cost gating (suspend self as activation cost)
//! - Security play (play_from_security)
//!
//! # Implementation status
//!
//! Clause 0 (Start of Main Phase):
//!   BLOCKED — G-DSL-SELECT-OWN-SOURCES-FILTER (qa/dsl-vocab-gaps.md).
//!   The cost requires trashing 1 Mineral/Rock from hand OR own digivolution
//!   cards. The `select_own_sources` DSL step exists but carries no `filter:`
//!   field, so the trait gate on digivolution-card sources cannot be expressed.
//!   A hand-only approximation would be unfaithful; the clause is omitted.
//!
//! Clause 1 (All Turns on play/digivolve):
//!   IMPLEMENTED — uses the EX11-054 pattern (on_enter_field_anyone +
//!   on_digivolve with event_target_trait_has, suspend source as cost) plus
//!   BT21-013 pattern (select_effect_choice → select_hand/select_trash →
//!   place_as_bottom_source { source, target: event_target }).
//!
//! Security:
//!   IMPLEMENTED — standard play_from_security.

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("EX11-065")
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn ex11_065_is_tamer_cost3_purple() {
    use digimon_dsl::compiled::{CompiledCardKind, CompiledColor};
    let card = compiled();
    assert_eq!(card.kind, CompiledCardKind::Tamer, "kind must be Tamer");
    assert_eq!(card.cost, Some(3), "play cost must be 3");
    assert!(
        card.color.contains(&CompiledColor::Purple),
        "EX11-065 must be Purple"
    );
}

#[test]
fn ex11_065_has_all_turns_observer_clause() {
    let card = compiled();
    let observer = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    && t.when.contains(&CompiledTiming::OnDigivolve) =>
            {
                Some(t)
            }
            _ => None,
        });
    assert!(
        observer.is_some(),
        "EX11-065 must have an all-turns [OnEnterFieldAnyone + OnDigivolve] observer clause"
    );
}

#[test]
fn ex11_065_all_turns_observer_is_optional() {
    let card = compiled();
    let observer = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    && t.when.contains(&CompiledTiming::OnDigivolve) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("all-turns observer clause must exist");
    assert!(
        observer.optional,
        "All Turns clause is optional (printed 'by suspending this Tamer, you may')"
    );
}

#[test]
fn ex11_065_all_turns_observer_scope_is_face_up() {
    let card = compiled();
    let observer = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    && t.when.contains(&CompiledTiming::OnDigivolve) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("all-turns observer clause must exist");
    assert_eq!(
        observer.scope,
        CompiledScope::FaceUp,
        "all-turns clause on a Tamer has FaceUp scope"
    );
}

#[test]
fn ex11_065_has_security_clause() {
    let card = compiled();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnSecurity)),
        "EX11-065 must have an OnSecurity clause; got: {triggered:?}"
    );
}

#[test]
fn ex11_065_security_clause_is_not_optional() {
    let card = compiled();
    let sec_clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .next()
        .expect("OnSecurity clause must exist on EX11-065");
    assert!(
        !sec_clause.optional,
        "Security clause must not be optional (security effects auto-fire per rules)"
    );
}

// ─── Section 2: Clause 0 (Start of Main Phase) — BLOCKED ─────────────────────
//
// The printed cost is "by trashing 1 [Mineral] or [Rock] trait card from
// your hand OR your Digimon's digivolution cards."  The DSL `select_own_sources`
// step has no `filter:` field (G-DSL-SELECT-OWN-SOURCES-FILTER), making a
// faithful source-zone side of the union impossible. The clause is omitted from
// the YAML entirely; all tests for it carry the ignore tag below.

#[test]
#[ignore = "pending: G-DSL-SELECT-OWN-SOURCES-FILTER from qa/dsl-vocab-gaps.md — start_of_main trash-hand-or-source cost"]
fn ex11_065_start_of_main_has_clause() {
    // When G-DSL-SELECT-OWN-SOURCES-FILTER closes and the hand-or-source union
    // trash cost is expressible, EX11-065 must gain a StartOfYourMainPhase
    // clause with optional: true (printed "By trashing …") and gain_memory: 1
    // in its process body.
    let card = compiled();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase)),
        "EX11-065 must have a StartOfYourMainPhase clause once the gap closes"
    );
}

#[test]
#[ignore = "pending: G-DSL-SELECT-OWN-SOURCES-FILTER from qa/dsl-vocab-gaps.md — start_of_main trash-hand cost only (positive)"]
fn ex11_065_start_of_main_trash_hand_mineral_gains_memory() {
    // Positive: have a Mineral card in hand, exercise the hand-trash branch
    // → gain 1 memory.
    todo!()
}

#[test]
#[ignore = "pending: G-DSL-SELECT-OWN-SOURCES-FILTER from qa/dsl-vocab-gaps.md — start_of_main trash-source cost only (positive)"]
fn ex11_065_start_of_main_trash_source_mineral_gains_memory() {
    // Positive: have a Mineral source in own Digimon's digivolution stack,
    // exercise the source-trash branch → gain 1 memory.
    todo!()
}

#[test]
#[ignore = "pending: G-DSL-SELECT-OWN-SOURCES-FILTER from qa/dsl-vocab-gaps.md — start_of_main no eligible card (negative)"]
fn ex11_065_start_of_main_no_eligible_card_no_prompt() {
    // Negative: no Mineral/Rock in hand and no Mineral/Rock in own sources →
    // no pending selection installs and memory is unchanged.
    todo!()
}

// ─── Section 3: Clause 1 (All Turns observer) ────────────────────────────────

/// Helper: build a runner with Close (EX11-065), a Mineral Digimon (played
/// ally), and a Mineral/Rock hand card to place as bottom source.
fn close_runner_with_mineral_ally_and_source() -> DebugRunner {
    let mut mineral_ally = make_test_card("MINERAL-ALLY", "MineralAlly");
    mineral_ally.card_kind = CardKind::Digimon;
    mineral_ally.level = Some(4);
    mineral_ally.dp = Some(5000);
    mineral_ally.traits = vec!["Mineral".to_string()];
    mineral_ally.play_cost = 0;

    // A Mineral/Rock card that can be placed as a bottom source.
    let mut rock_hand_card = make_test_card("ROCK-HAND", "RockCard");
    rock_hand_card.card_kind = CardKind::Digimon;
    rock_hand_card.level = Some(3);
    rock_hand_card.dp = Some(2000);
    rock_hand_card.traits = vec!["Rock".to_string()];
    rock_hand_card.play_cost = 3;

    let mut rock_trash_card = make_test_card("ROCK-TRASH", "RockTrash");
    rock_trash_card.card_kind = CardKind::Digimon;
    rock_trash_card.level = Some(3);
    rock_trash_card.dp = Some(2000);
    rock_trash_card.traits = vec!["Rock".to_string()];
    rock_trash_card.play_cost = 3;

    DebugRunner::builder()
        .dsl_card("EX11-065")
        .expect("EX11-065 in embedded pack")
        .add_card(mineral_ally)
        .add_card(rock_hand_card)
        .add_card(rock_trash_card)
        .memory(10)
        .start()
}

#[test]
fn ex11_065_all_turns_triggers_when_mineral_ally_played() {
    let mut runner = close_runner_with_mineral_ally_and_source();

    // Place Close on P0's field (unsuspended).
    runner.place_on_field(0, "EX11-065", None);

    // Give P0 a hand card and a trash card for the place step.
    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "ROCK-HAND")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }

    // Play the Mineral ally.
    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "MINERAL-ALLY")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    runner.play(0, 0); // plays MINERAL-ALLY (index 0 after ROCK-HAND is index 0, so MINERAL-ALLY is now index 1 but play(0,0) pops hand[0])

    // After OnEnterFieldAnyone fires, Close's observer should install a
    // pending optional selection (suspend → place a card).
    assert!(
        runner.pending_selection().is_some(),
        "optional activation prompt must install when a Mineral ally is played and Close is unsuspended"
    );
}

#[test]
fn ex11_065_all_turns_does_not_trigger_for_non_mineral_rock_ally() {
    // Negative: playing a Beast (no Mineral/Rock trait) → no activation prompt.
    let mut beast = make_test_card("BEAST", "BeastDigimon");
    beast.card_kind = CardKind::Digimon;
    beast.level = Some(4);
    beast.dp = Some(4000);
    beast.traits = vec!["Beast".to_string()];
    beast.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-065")
        .expect("EX11-065 in embedded pack")
        .add_card(beast)
        .memory(10)
        .start();

    runner.place_on_field(0, "EX11-065", None);

    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BEAST")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "no activation prompt when ally has no Mineral or Rock trait"
    );
}

#[test]
fn ex11_065_all_turns_does_not_trigger_when_close_already_suspended() {
    // If Close is already suspended it cannot pay the cost → no prompt.
    let mut mineral_ally = make_test_card("MINERAL-ALLY-S", "MineralAllyS");
    mineral_ally.card_kind = CardKind::Digimon;
    mineral_ally.level = Some(4);
    mineral_ally.dp = Some(5000);
    mineral_ally.traits = vec!["Mineral".to_string()];
    mineral_ally.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-065")
        .expect("EX11-065 in embedded pack")
        .add_card(mineral_ally)
        .memory(10)
        .start();

    let close_perm = runner.place_on_field(0, "EX11-065", None);

    // Pre-suspend Close.
    runner.game.players[0]
        .battle_area
        .get_mut(close_perm.index as usize)
        .unwrap()
        .is_suspended = true;

    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "MINERAL-ALLY-S")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "no prompt when Close is already suspended (cannot pay suspend cost)"
    );
}

#[test]
fn ex11_065_all_turns_accepts_activation_and_suspends_close() {
    // When the optional prompt installs and the player accepts (action 0 = accept),
    // Close must become suspended.
    let mut mineral_ally = make_test_card("MINERAL-A2", "MineralA2");
    mineral_ally.card_kind = CardKind::Digimon;
    mineral_ally.level = Some(4);
    mineral_ally.dp = Some(5000);
    mineral_ally.traits = vec!["Mineral".to_string()];
    mineral_ally.play_cost = 0;

    let mut rock_hand = make_test_card("ROCK-H2", "RockH2");
    rock_hand.card_kind = CardKind::Digimon;
    rock_hand.level = Some(3);
    rock_hand.dp = Some(2000);
    rock_hand.traits = vec!["Rock".to_string()];
    rock_hand.play_cost = 3;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-065")
        .expect("EX11-065 in embedded pack")
        .add_card(mineral_ally)
        .add_card(rock_hand)
        .memory(10)
        .start();

    let close_perm = runner.place_on_field(0, "EX11-065", None);

    // Pre-load P0's hand with ROCK-H2.
    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "ROCK-H2")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }

    // Play the Mineral ally.
    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "MINERAL-A2")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    runner.play(0, 0); // Play MINERAL-A2 (it's now hand[0] since we played ROCK-H2 last)

    // Close's observer should fire.
    assert!(
        runner.pending_selection().is_some(),
        "optional activation prompt must install"
    );

    // Accept the activation (first valid action = accept).
    let act = {
        let s = runner.pending_selection().unwrap();
        s.valid_action_ids[0]
    };
    runner.execute_action(0, act).ok();
    // Resolve any follow-on prompts (zone choice, card pick) with auto_resolve.
    runner.auto_resolve();

    // Close must now be suspended.
    let close_now = runner
        .game
        .player(0)
        .battle_area
        .get(close_perm.index as usize)
        .unwrap();
    assert!(
        close_now.is_suspended,
        "Close must be suspended after activation is accepted"
    );
}

#[test]
fn ex11_065_all_turns_declining_does_not_suspend_close() {
    // When the optional prompt installs but the player passes (declines),
    // Close must remain unsuspended and no bottom-source placement occurs.
    let mut mineral_ally = make_test_card("MINERAL-D", "MineralD");
    mineral_ally.card_kind = CardKind::Digimon;
    mineral_ally.level = Some(4);
    mineral_ally.dp = Some(5000);
    mineral_ally.traits = vec!["Mineral".to_string()];
    mineral_ally.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-065")
        .expect("EX11-065 in embedded pack")
        .add_card(mineral_ally)
        .memory(10)
        .start();

    let close_perm = runner.place_on_field(0, "EX11-065", None);

    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "MINERAL-D")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    runner.play(0, 0);

    assert!(runner.pending_selection().is_some(), "prompt should install");
    // Pass = decline.
    runner.execute_action(0, digimon_engine::action::space::PASS).ok();

    let close_now = runner
        .game
        .player(0)
        .battle_area
        .get(close_perm.index as usize)
        .unwrap();
    assert!(
        !close_now.is_suspended,
        "Close must NOT be suspended if the player declines"
    );
}

#[test]
fn ex11_065_all_turns_rock_trait_also_triggers() {
    // The trait condition is Mineral OR Rock. Verify Rock alone is sufficient.
    let mut rock_ally = make_test_card("ROCK-ALLY", "RockAlly");
    rock_ally.card_kind = CardKind::Digimon;
    rock_ally.level = Some(4);
    rock_ally.dp = Some(5000);
    rock_ally.traits = vec!["Rock".to_string()];
    rock_ally.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-065")
        .expect("EX11-065 in embedded pack")
        .add_card(rock_ally)
        .memory(10)
        .start();

    runner.place_on_field(0, "EX11-065", None);

    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "ROCK-ALLY")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_some(),
        "Rock trait alone must trigger the All Turns observer"
    );
}

// ─── Section 4: Security clause ──────────────────────────────────────────────

#[test]
fn ex11_065_security_play_puts_tamer_on_field() {
    // When EX11-065 is revealed as a security card, the security effect fires
    // and Close lands on the defending player's field without paying the cost.
    let mut attacker = make_test_card("ATTACKER-065", "BigAttacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(5);
    attacker.dp = Some(9000);
    attacker.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-065")
        .expect("EX11-065 in embedded pack")
        .add_card(attacker)
        .security(1, &["EX11-065"])
        .memory(10)
        .start();

    let atk_perm = runner.place_on_field(0, "ATTACKER-065", Some(0));

    let p1_field_before = runner.battle_area_size(1);
    assert_eq!(runner.security_count(1), 1, "pre: P1 has 1 security card");

    runner.attack_player(atk_perm, 1, false);

    assert_eq!(
        runner.battle_area_size(1),
        p1_field_before + 1,
        "Close must land on P1's field after the security effect fires"
    );

    let close_on_field = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "EX11-065");
    assert!(
        close_on_field,
        "EX11-065 must be the card placed on P1's field via security effect"
    );

    assert_eq!(
        runner.security_count(1),
        0,
        "security stack must be empty after Close plays from security"
    );
}

// ─── Section 5: Place-as-bottom-source integrated test ───────────────────────

#[test]
fn ex11_065_all_turns_place_from_hand_increases_source_count() {
    // After accepting the activation, choosing "From hand", and selecting the
    // Rock hand card, the ally Digimon's source count must grow by 1 (the Rock
    // card is now its bottom digivolution card).
    let mut mineral_ally = make_test_card("MINERAL-P", "MineralP");
    mineral_ally.card_kind = CardKind::Digimon;
    mineral_ally.level = Some(4);
    mineral_ally.dp = Some(5000);
    mineral_ally.traits = vec!["Mineral".to_string()];
    mineral_ally.play_cost = 0;

    let mut rock_hand = make_test_card("ROCK-P", "RockP");
    rock_hand.card_kind = CardKind::Digimon;
    rock_hand.level = Some(3);
    rock_hand.dp = Some(2000);
    rock_hand.traits = vec!["Rock".to_string()];
    rock_hand.play_cost = 3;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-065")
        .expect("EX11-065 in embedded pack")
        .add_card(mineral_ally)
        .add_card(rock_hand)
        .memory(10)
        .start();

    runner.place_on_field(0, "EX11-065", None);

    // Pre-load ROCK-P into hand.
    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "ROCK-P")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }

    // Play MINERAL-P from hand.
    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "MINERAL-P")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    runner.play(0, 0); // plays MINERAL-P (hand order: ROCK-P at idx 0, MINERAL-P at idx 1 → play(0,0) pops idx 0)

    // Ally is now on field. Identify its field index (last entry).
    let ally_index = runner.game.player(0).battle_area.len().saturating_sub(1);
    let sources_before = runner
        .game
        .player(0)
        .battle_area
        .get(ally_index)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    assert!(
        runner.pending_selection().is_some(),
        "optional activation prompt must fire after Mineral ally is played"
    );

    // Accept the activation.
    let act = {
        let s = runner.pending_selection().unwrap();
        s.valid_action_ids[0]
    };
    runner.execute_action(0, act).ok();

    // Resolve all follow-on prompts automatically (zone choice → hand card pick).
    runner.auto_resolve();

    let sources_after = runner
        .game
        .player(0)
        .battle_area
        .get(ally_index)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    assert_eq!(
        sources_after,
        sources_before + 1,
        "ally Digimon's source count must increase by 1 after place_as_bottom_source"
    );
}
