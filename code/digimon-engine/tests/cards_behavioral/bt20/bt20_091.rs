//! BT20-091 Cool Boy - Tamer, White, LIBERATOR.

use digimon_dsl::compiled::{
    CompiledActivationCostKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .memory(10)
        .start()
}

fn royal_knight_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, "Royal Knight Test");
    cd.traits = vec!["Royal Knight".into()];
    cd.colors = vec![CardColor::White];
    cd
}

fn omekamon_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, "Omekamon");
    cd.card_kind = CardKind::Digimon;
    cd.colors = vec![CardColor::White];
    cd
}

fn field_contains(r: &DebugRunner, player: u8, card_id: &str) -> bool {
    r.game.player(player).battle_area.iter().any(|perm| {
        perm.card_sources
            .iter()
            .any(|card| card.card_id(&r.game.card_data) == card_id)
    })
}

#[test]
fn bt20_091_has_printed_metadata() {
    let runner = runner();
    let card = runner.compiled_card("BT20-091").expect("BT20-091 present");
    assert_eq!(card.name, "Cool Boy");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "LIBERATOR"));
}

#[test]
fn bt20_091_has_rk_play_or_digivolve_suspend_draw_memory_clause() {
    let runner = runner();
    let card = runner.compiled_card("BT20-091").expect("BT20-091 present");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    && triggered.when.contains(&CompiledTiming::OnDigivolve) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Royal Knight observer exists");
    assert!(clause.optional);
    assert!(clause.process.iter().any(|step| matches!(
        step,
        CompiledStep::ActivationCost {
            kind: CompiledActivationCostKind::SuspendSelf
        }
    )));
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::Draw { count: 1, .. })));
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::GainMemory(1))));
}

#[test]
fn bt20_091_security_plays_self() {
    let runner = runner();
    let card = runner.compiled_card("BT20-091").expect("BT20-091 present");
    assert!(card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => {
            triggered.when.contains(&CompiledTiming::OnSecurity)
                && triggered
                    .process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::PlayFromSecurity))
        }
        _ => false,
    }));
}

#[test]
fn bt20_091_opponent_turn_may_play_omekamon_when_royal_knight_would_leave() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .add_card(royal_knight_card("RK-TARGET"))
        .add_card(omekamon_card("OMEKAMON-HAND"))
        .hand(0, &["OMEKAMON-HAND"])
        .memory(0)
        .start();
    r.game.turn_player_idx = 1;

    r.place_on_field(0, "BT20-091", Some(0));
    let target = r.place_on_field(0, "RK-TARGET", Some(0));

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Cool Boy should offer the optional would-leave response");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert!(pending.valid_action_ids.contains(&REPLACEMENT_ACCEPT));

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Cool Boy response");
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("accepting asks which Omekamon to play");
    assert_eq!(pending.kind, SelectionKind::Hand);
    assert!(pending.valid_action_ids.contains(&PLAY_HAND_START));

    r.game
        .resolve_selection(0, PLAY_HAND_START)
        .expect("play Omekamon from hand");

    assert!(
        field_contains(&r, 0, "OMEKAMON-HAND"),
        "Cool Boy plays the selected Omekamon"
    );
    assert!(
        !field_contains(&r, 0, "RK-TARGET"),
        "the original Royal Knight leave event still proceeds"
    );
    assert!(
        field_contains(&r, 0, "BT20-091"),
        "Cool Boy itself remains on field"
    );
}

#[test]
fn bt20_091_decline_would_leave_response_proceeds_without_playing_omekamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .add_card(royal_knight_card("RK-TARGET"))
        .add_card(omekamon_card("OMEKAMON-HAND"))
        .hand(0, &["OMEKAMON-HAND"])
        .memory(0)
        .start();
    r.game.turn_player_idx = 1;

    r.place_on_field(0, "BT20-091", Some(0));
    let target = r.place_on_field(0, "RK-TARGET", Some(0));

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, PASS)
        .expect("decline Cool Boy response");

    assert!(!field_contains(&r, 0, "RK-TARGET"));
    assert!(
        !field_contains(&r, 0, "OMEKAMON-HAND"),
        "decline leaves Omekamon in hand"
    );
}

#[test]
fn bt20_091_no_omekamon_in_hand_does_not_offer_response() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .add_card(royal_knight_card("RK-TARGET"))
        .memory(0)
        .start();
    r.game.turn_player_idx = 1;

    r.place_on_field(0, "BT20-091", Some(0));
    let target = r.place_on_field(0, "RK-TARGET", Some(0));

    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "without an Omekamon to play, no would-leave prompt is offered"
    );
    assert!(!field_contains(&r, 0, "RK-TARGET"));
}

// ─── Omekamon pick legality ──────────────────────────────────────────────────
// DCGO BT20_091.cs:90-94 `IsOmekamon`: EqualsCardName("Omekamon") &&
// CanPlayAsNewPermanent(source, payCost: false, activateClass) — the hand pick
// (:133-147) and `CanActivateCondition` (:118-123, HasMatchConditionOwnersHand)
// both use it, so an Omekamon the play would refuse (no free battle-area
// frame, a "can't play Digimon by effects" lock) is never offered, and with
// no playable Omekamon the response isn't offered at all. CR 4.3 1-3-2.

/// Cool Boy + a Royal Knight on P0's field, an Omekamon in hand, P1's turn.
fn would_leave_runner() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .add_card(royal_knight_card("RK-TARGET"))
        .add_card(omekamon_card("OMEKAMON-HAND"))
        .hand(0, &["OMEKAMON-HAND"])
        .memory(0)
        .start();
    r.game.turn_player_idx = 1;
    r.place_on_field(0, "BT20-091", Some(0));
    let target = r.place_on_field(0, "RK-TARGET", Some(0));
    (r, target)
}

#[test]
fn bt20_091_omekamon_not_offered_under_effect_play_lock() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut r, target) = would_leave_runner();
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_none(),
        "the only Omekamon can't be played by effects: no response offered"
    );
    assert!(!field_contains(&r, 0, "RK-TARGET"), "the leave proceeds");
    assert!(!field_contains(&r, 0, "OMEKAMON-HAND"));
}

#[test]
fn bt20_091_omekamon_not_offered_with_a_full_battle_area() {
    let (mut r, target) = would_leave_runner();
    // The Royal Knight is still on the field when the response would play
    // Omekamon, so a field at capacity has no frame for it.
    r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    assert!(
        r.game.pending_selection.is_none(),
        "no free frame for Omekamon: no response offered"
    );
    assert!(!field_contains(&r, 0, "RK-TARGET"), "the leave proceeds");
}

// ─── [Your Turn] suspend-self observer scoping ──────────────────────────────
// DCGO BT20_091.cs:27,62: only own Royal Knight Digimon trigger it, and each
// Cool Boy checks its own CanActivateSuspendCostEffect(card) — an already
// suspended Cool Boy must not trigger just because a second copy on the board
// is unsuspended.

#[test]
fn bt20_091_suspended_copy_does_not_trigger_when_sibling_is_unsuspended() {
    let mut rk = royal_knight_card("RK-PLAY");
    rk.play_cost = 3;
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .add_card(rk)
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["RK-PLAY"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.skip_mulligan();

    let first = r.place_on_field(0, "BT20-091", Some(0));
    let second = r.place_on_field(0, "BT20-091", Some(0));
    r.game.players[0].battle_area[first.index as usize].is_suspended = true;

    r.play(0, 0).expect("play Royal Knight Digimon");

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("only the unsuspended Cool Boy should offer its optional trigger");
    assert_eq!(pending.kind, SelectionKind::TriggerOrder);
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "already-suspended Cool Boy must not also queue a trigger"
    );
    let action = pending.valid_action_ids[0];
    r.execute_action(0, action)
        .expect("activate unsuspended Cool Boy");

    assert!(
        r.game.pending_selection.is_none(),
        "no second trigger should remain after the lone unsuspended copy resolves"
    );
    assert!(r.game.players[0].battle_area[second.index as usize].is_suspended);
    assert_eq!(r.game.players[0].hand.len(), 1, "drew 1 card");
    assert_eq!(r.game.memory, 8, "10 - 3 play cost + 1 from Cool Boy");
}

#[test]
fn bt20_091_cannot_suspend_lock_blocks_trigger() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;

    let mut rk = royal_knight_card("RK-PLAY");
    rk.play_cost = 3;
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .add_card(rk)
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["RK-PLAY"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.skip_mulligan();

    let cool_boy = r.place_on_field(0, "BT20-091", Some(0));
    r.game.modifiers.add(
        cool_boy,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 1),
    );

    r.play(0, 0).expect("play Royal Knight Digimon");

    assert!(
        r.game.pending_selection.is_none(),
        "Cool Boy under CannotSuspend cannot pay its suspend cost"
    );
    assert!(!r.game.players[0].battle_area[cool_boy.index as usize].is_suspended);
    assert_eq!(r.game.players[0].hand.len(), 0);
    assert_eq!(r.game.memory, 7);
}

#[test]
fn bt20_091_non_digimon_royal_knight_does_not_trigger() {
    let mut rk_tamer = royal_knight_card("RK-TAMER");
    rk_tamer.card_kind = CardKind::Tamer;
    rk_tamer.level = None;
    rk_tamer.dp = None;
    rk_tamer.play_cost = 3;
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .add_card(rk_tamer)
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["RK-TAMER"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.skip_mulligan();

    let cool_boy = r.place_on_field(0, "BT20-091", Some(0));
    r.play(0, 0).expect("play Royal Knight Tamer");

    assert!(
        r.game.pending_selection.is_none(),
        "Cool Boy triggers only on Digimon played/digivolving, not Tamers"
    );
    assert!(!r.game.players[0].battle_area[cool_boy.index as usize].is_suspended);
    assert_eq!(r.game.players[0].hand.len(), 0);
    assert_eq!(r.game.memory, 7);
}
