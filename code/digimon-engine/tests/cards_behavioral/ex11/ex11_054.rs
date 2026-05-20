//! EX11-054 Owen Dreadnought — Tamer | Cost4 | Red | Trait: LIBERATOR
//!
//! # Card text (cards.json / fandom wiki / DCGO)
//!
//! "[Start of Your Turn] If your memory is 2 or less, set your memory to 3."
//!
//! "[All Turns] When your Digimon are played or digivolve, if any of them have
//! the [Reptile] or [Dragonkin] trait, by suspending this Tamer, <Draw 1>.
//! After, 1 of your Digimon with <Progress> gets +3000 DP for the turn."
//!
//! "[Security] Play this card without paying the cost."
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Red/EX11_054.cs
//!
//! # Patterns this test covers
//! - B2 Cost-4 tamer: start-of-turn memory swing + all-turns ally observer
//! - F9 Security-loss conditioned tamers (Owen Dreadnought pattern)
//! - Security play (play_from_security)
//!
//! # Engine notes
//!
//! The all-turns observer now uses event context: played/digivolved Digimon are
//! visible through `event_target_owner` and `event_target_trait_has`, so the
//! clause is native DSL instead of a raw-rust no-op.

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("EX11-054")
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn ex11_054_is_tamer_cost4_red() {
    use digimon_dsl::compiled::{CompiledCardKind, CompiledColor};
    let card = compiled();
    assert_eq!(card.kind, CompiledCardKind::Tamer, "kind must be Tamer");
    assert_eq!(card.cost, Some(4), "play cost must be 4");
    assert!(
        card.color.contains(&CompiledColor::Red),
        "EX11-054 must be Red"
    );
}

#[test]
fn ex11_054_has_start_of_your_turn_clause() {
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
            .any(|t| t.when.contains(&CompiledTiming::StartOfYourTurn)),
        "EX11-054 must have a StartOfYourTurn triggered clause; got: {triggered:?}"
    );
}

#[test]
fn ex11_054_has_security_clause() {
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
        "EX11-054 must have an OnSecurity clause; got: {triggered:?}"
    );
}

#[test]
fn ex11_054_has_native_all_turns_observer_clause() {
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
        .expect("EX11-054 must have a native all-turns enter/digivolve observer");
    assert_eq!(observer.scope, CompiledScope::FaceUp);
    assert!(observer.optional, "suspending Owen is an optional cost");
}

// ─── Section 2: Condition gating — Clause 1 (start_of_your_turn) ─────────────

#[test]
fn ex11_054_start_of_turn_sets_memory_to_3_when_lte_2() {
    // Strategy (mirrors BT18-087 clause1 tests):
    //   1. Build runner with filler decks so end_turn()'s draw step doesn't panic.
    //   2. Place Owen on P0's field.
    //   3. Reset memory to exactly 2 (seesaw: +2 → P1 turn (-2) → P0 turn (+2)).
    //   4. end_turn() twice to arrive at P0's start_of_turn.
    //   5. Owen's clause fires: memory_lte: 2 → TRUE → set_memory(3).
    let filler = make_test_card("FILLER-054-ST", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-054")
        .expect("EX11-054 in embedded pack")
        .add_card(filler)
        .deck(0, &["FILLER-054-ST"])
        .deck(1, &["FILLER-054-ST"])
        .memory(2)
        .start();

    runner.place_on_field(0, "EX11-054", None);

    // Reset memory explicitly after placement — start() sets memory post-begin_turn
    // so Owen didn't see the StartOfYourTurn at construction; this is the memory
    // that the NEXT P0 turn-start will observe.
    runner.game.memory = 2;

    // P0 ends turn → memory flips to -2 (P1's side: P1 has 2).
    runner.end_turn();
    // P1 ends turn → memory flips to +2 (P0's side again).
    runner.end_turn();
    // begin_turn for P0 just fired StartOfYourTurn.
    // Owen: memory = 2, memory_lte: 2 → TRUE → set_memory(3).

    assert_eq!(
        runner.memory(),
        3,
        "start_of_your_turn should set memory to 3 when memory was 2 (≤ 2 threshold)"
    );
}

#[test]
fn ex11_054_start_of_turn_does_not_lower_memory_above_3() {
    // The StartOfYourTurn clause must have a `condition: { memory_lte: 2 }` guard.
    // Structural verification: confirm the clause carries a condition so that
    // memory > 2 does NOT trigger the set_memory: 3 effect.
    //
    // If the condition were absent, calling the clause at memory = 5 would
    // unconditionally drop memory to 3, which is incorrect. We verify by
    // asserting the condition is present on the compiled clause.
    let card = compiled();
    let sot_clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::StartOfYourTurn) => {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("StartOfYourTurn clause must exist");
    assert!(
        sot_clause.condition.is_some(),
        "StartOfYourTurn clause must have a condition (memory_lte: 2 guard); \
         an unconditional set_memory: 3 would incorrectly lower memory from values > 2"
    );
}

// ─── Section 3: Behavioral — Clause 3 (security play) ────────────────────────

#[test]
fn ex11_054_security_clause_plays_without_cost() {
    // Structural: on_security timing present and not optional.
    let card = compiled();
    let sec_clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .next()
        .expect("OnSecurity clause must exist on EX11-054");

    // Not optional (security effects are mandatory).
    assert!(
        !sec_clause.optional,
        "Security clause must not be optional (it auto-plays)"
    );
}

#[test]
fn ex11_054_security_play_puts_tamer_on_field() {
    // When EX11-054 is revealed as a security card and the security effect fires,
    // Owen should land on P1's field without paying the cost.
    //
    // Setup: use builder's .security(1, &["EX11-054"]) to pre-seed P1's security
    // stack correctly. P0 has an attacker Digimon on field and attacks P1's player.
    //
    // Pattern mirrors security_effects.rs / test_021_plays_self_from_security.
    let mut attacker = make_test_card("ATTACKER-054", "BigAttacker");
    attacker.card_kind = digimon_engine::enums::CardKind::Digimon;
    attacker.level = Some(5);
    attacker.dp = Some(9000);
    attacker.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-054")
        .expect("EX11-054 in embedded pack")
        .add_card(attacker)
        .security(1, &["EX11-054"])
        .memory(10)
        .start();

    // P0 plays an attacker onto the field. Pass turn=0 to bypass summoning
    // sickness (place_on_field with turn=0 marks the card as played before
    // the current turn, so can_attack returns true).
    let atk_perm = runner.place_on_field(0, "ATTACKER-054", Some(0));

    let p1_field_before = runner.battle_area_size(1);
    assert_eq!(
        runner.security_count(1),
        1,
        "pre: P1 must have 1 security card (Owen Dreadnought)"
    );

    // P0 attacks P1's player — the security check fires Owen's SecuritySkill
    // (on_security) effect, which calls play_from_security → play_pending_security
    // → places Owen on P1's field.
    runner.attack_player(atk_perm, 1, false);

    // Owen should now be on P1's field.
    let p1_field_after = runner.battle_area_size(1);
    assert_eq!(
        p1_field_after,
        p1_field_before + 1,
        "Owen Dreadnought should land on P1's field after the security effect fires"
    );

    // Verify it's Owen specifically (not some other card).
    let owen_on_field = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "EX11-054");
    assert!(
        owen_on_field,
        "EX11-054 must be the card placed on P1's field by the security effect"
    );

    // Security should now be empty (card was consumed by the security check).
    assert_eq!(
        runner.security_count(1),
        0,
        "security stack must be empty after Owen plays from security"
    );
}

// ─── Section 4: All Turns clause ─────────────────────────────────────────────

/// Build a runner with Owen on P0's field and a Reptile Digimon ready in
/// P0's hand. Returns the runner, Owen's handle, and the Reptile hand index.
fn reptile_observer_runner() -> (DebugRunner, digimon_engine::permanent::PermanentHandle, usize) {
    use digimon_engine::card_source::CardSource;
    use digimon_engine::enums::CardKind;

    let mut reptile = make_test_card("REPTILE", "ReptileDigimon");
    reptile.card_kind = CardKind::Digimon;
    reptile.level = Some(4);
    reptile.dp = Some(4000);
    reptile.traits = vec!["Reptile".to_string()];
    reptile.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-054")
        .expect("EX11-054 in embedded pack")
        .add_card(reptile)
        .add_card(make_test_card("DRAW-FILLER", "DrawFiller"))
        .deck(0, &["DRAW-FILLER"])
        .memory(10)
        .start();

    let owen = runner.place_on_field(0, "EX11-054", None);
    assert!(
        !runner.game.player(0).battle_area[owen.index as usize].is_suspended,
        "Owen must start unsuspended"
    );

    let reptile_hand_idx = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "REPTILE")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
        runner.game.players[0].hand.len() - 1
    };
    (runner, owen, reptile_hand_idx)
}

#[test]
fn ex11_054_all_turns_suspends_and_draws_when_reptile_ally_played() {
    // Printed text: "by suspending this Tamer, <Draw 1>". The suspend is an
    // optional ACTIVATION COST — the player chooses whether to pay it. The
    // observer therefore installs a pre-cost TriggerOrder selection (with
    // PASS) before the suspend fires. This test exercises the ACCEPT path:
    // the player picks the trigger, Owen suspends, and Draw 1 fires.
    use digimon_engine::action::space::PASS;
    use digimon_engine::selection::SelectionKind;

    let (mut runner, owen, reptile_hand_idx) = reptile_observer_runner();
    let hand_before = runner.hand_size(0);

    runner.play(0, reptile_hand_idx); // play REPTILE from hand

    // The all-turns observer's event-context condition (own Reptile played)
    // is genuinely TRUE, so the optional + activation_cost trigger installs a
    // pre-cost decline prompt before the suspend cost runs.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder),
        "an optional activation-cost trigger whose event-condition is TRUE \
         must offer a pre-cost decline prompt"
    );
    assert!(
        runner.pending_is_optional(),
        "the pre-cost TriggerOrder must expose PASS (suspending Owen is optional)"
    );
    // The suspend cost must not have fired yet.
    assert!(
        !runner.game.player(0).battle_area[owen.index as usize].is_suspended,
        "Owen must not suspend before the player accepts the activation cost"
    );

    // Accept the trigger (pick it, not PASS).
    let trigger_action = {
        let sel = runner.pending_selection().unwrap();
        assert_ne!(
            sel.valid_action_ids[0], PASS,
            "the first valid action must be the trigger, not PASS"
        );
        sel.valid_action_ids[0]
    };
    runner
        .execute_action(0, trigger_action)
        .expect("accept the activation cost");
    let _ = runner.auto_resolve();

    // After accepting: the activation_cost step suspends Owen, the draw step
    // runs, and the optional select_own_permanent for a Progress Digimon
    // finds no candidates (none in this fixture) so completes silently.
    assert!(
        runner.game.player(0).battle_area[owen.index as usize].is_suspended,
        "Owen must be suspended after the activation cost is accepted"
    );
    // `hand_before` already counts the REPTILE in hand. Playing REPTILE
    // removes it (-1); the accepted Draw 1 adds DRAW-FILLER (+1). Net: the
    // hand size is back to `hand_before`.
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "draw 1 must fire (REPTILE leaves hand, DRAW-FILLER replaces it)"
    );
}

#[test]
fn ex11_054_all_turns_can_decline_suspend_cost_when_reptile_ally_played() {
    // Printed text: "by suspending this Tamer, <Draw 1>" — the suspend cost is
    // optional. Declining the pre-cost prompt must leave Owen unsuspended and
    // skip the Draw 1 entirely (no-approximations: the choice must be real).
    use digimon_engine::action::space::PASS;
    use digimon_engine::selection::SelectionKind;

    let (mut runner, owen, reptile_hand_idx) = reptile_observer_runner();
    let hand_before = runner.hand_size(0);

    runner.play(0, reptile_hand_idx); // play REPTILE from hand

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder),
        "the optional activation-cost trigger must offer a decline prompt"
    );

    // Decline — choose PASS.
    runner
        .execute_action(0, PASS)
        .expect("PASS is legal on an optional pre-cost TriggerOrder");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.player(0).battle_area[owen.index as usize].is_suspended,
        "Owen must stay unsuspended when the player declines the suspend cost"
    );
    // REPTILE left the hand when played; no Draw 1 fires on decline, so the
    // hand is one smaller than before the play.
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "Draw 1 must not fire when the activation cost is declined"
    );
}

#[test]
fn ex11_054_all_turns_does_not_trigger_without_reptile_or_dragonkin() {
    // When P0 plays a non-Reptile/Dragonkin Digimon, Owen's clause should NOT fire.
    use digimon_engine::enums::CardKind;

    let mut plain = make_test_card("PLAIN", "PlainDigimon");
    plain.card_kind = CardKind::Digimon;
    plain.level = Some(3);
    plain.dp = Some(2000);
    plain.traits = vec!["Beast".to_string()];
    plain.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-054")
        .expect("EX11-054 in embedded pack")
        .add_card(plain)
        .memory(10)
        .start();

    runner.place_on_field(0, "EX11-054", None);

    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "PLAIN")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "no activation prompt should install when a non-Reptile/Dragonkin ally is played"
    );
}

#[test]
#[ignore = "pending: entering_permanent_trigger_context + G-DECLARATIVE-KEYWORD"]
fn ex11_054_all_turns_grants_3000_dp_to_progress_digimon() {
    // After suspending Owen and drawing 1, the next step selects one of
    // your Digimon with <Progress> and grants it +3000 DP until end of turn.
    //
    // Requires: (1) observer mechanism working, (2) Progress keyword active
    // on Medusamon (G-DECLARATIVE-KEYWORD gap). Blocked by both gaps.
}

#[test]
fn ex11_054_all_turns_does_not_offer_activation_when_tamer_already_suspended() {
    // DCGO's CanActivateCondition checks CanActivateSuspendCostEffect — returns
    // false when the tamer is already suspended. The activation prompt must NOT
    // appear when Owen is suspended (can't pay the suspend cost twice).
    use digimon_engine::enums::CardKind;

    let mut reptile = make_test_card("REPTILE-2", "ReptileDigimon2");
    reptile.card_kind = CardKind::Digimon;
    reptile.level = Some(4);
    reptile.dp = Some(4000);
    reptile.traits = vec!["Reptile".to_string()];
    reptile.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-054")
        .expect("EX11-054 in embedded pack")
        .add_card(reptile)
        .memory(10)
        .start();

    let owen = runner.place_on_field(0, "EX11-054", None);

    // Pre-suspend Owen manually.
    runner.game.players[0]
        .battle_area
        .get_mut(owen.index as usize)
        .unwrap()
        .is_suspended = true;

    {
        use digimon_engine::card_source::CardSource;
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "REPTILE-2")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "no activation prompt when Owen is already suspended (cost cannot be paid)"
    );
}
