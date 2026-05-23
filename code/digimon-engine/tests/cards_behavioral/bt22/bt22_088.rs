//! BT22-088 Arisa Kinosaki.
//! Printed text covered here:
//! - [Start of Your Main Phase] return-self cost + optional play Arisa from
//!   hand + conditional optional play Shoemon from trash (PUPPETS-G028 closure).
//! - [All Turns] Token/Puppet played observer with suspend_self activation_cost.
//! - [Security] Play this card without paying the cost.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledPredicate, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

#[test]
fn bt22_088_is_yellow_tamer_cost_3_liberator() {
    let runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .start();
    let compiled = runner.compiled_card("BT22-088").expect("compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.traits, vec!["LIBERATOR".to_string()]);
}

#[test]
fn bt22_088_exposes_security_and_token_or_puppet_played_observer() {
    let runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .start();
    let compiled = runner.compiled_card("BT22-088").expect("compiled card");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        3,
        "security, start-of-main, and played-observer slices are live"
    );
    let security = triggered
        .iter()
        .find(|triggered| triggered.when.contains(&CompiledTiming::OnSecurity))
        .expect("security clause is present");
    assert!(security.when.contains(&CompiledTiming::OnSecurity));
    assert!(!security.optional, "[Security] play this card is mandatory");
    assert_eq!(security.scope, CompiledScope::FaceUp);

    let observer = triggered
        .iter()
        .find(|triggered| triggered.when.contains(&CompiledTiming::OnAnyDigimonPlayed))
        .expect("All Turns Token/Puppet played observer is present");
    let condition = observer
        .condition
        .as_ref()
        .expect("observer needs event-target gates");
    assert!(
        predicate_has_event_target_kind(condition, CompiledCardKind::Token)
            && predicate_has_event_target_kind(condition, CompiledCardKind::Digimon),
        "observer must cover Tokens and Puppet Digimon"
    );
    assert!(
        predicate_has_event_target_trait(condition, "Puppet"),
        "Digimon branch must require Puppet"
    );
    assert!(
        steps_lead_with_activation_cost_suspend_self_then_draw(&observer.process),
        "body must lead with activation_cost: suspend_self then draw"
    );
}

#[test]
fn bt22_088_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER-BT22-088", "Attacker");
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .add_card(attacker)
        .security(1, &["BT22-088"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER-BT22-088", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT22-088"),
        "BT22-088 is played from the defender's security"
    );
}

/// [Start of Your Main Phase] — decline branch.
///
/// With the engine's optional-activation-cost prompt (single optional trigger
/// + activation_cost exposes a TriggerOrder selection before paying the cost),
/// the player sees a PASS action before the Tamer is returned.
/// Choosing PASS leaves BT22-088 on the field.
#[test]
fn bt22_088_start_of_main_can_decline_before_returning_tamer_to_deck_bottom() {
    let filler = {
        let mut c = make_test_card("FILLER-BT22088-DECLINE", "Filler");
        c.card_kind = CardKind::Digimon;
        c.level = Some(3);
        c.dp = Some(1000);
        c.play_cost = 0;
        c
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .add_card(filler)
        // Provide a deck so enter_main_phase() does not deck-out.
        .deck(0, &["FILLER-BT22088-DECLINE"; 3])
        .deck(1, &["FILLER-BT22088-DECLINE"; 3])
        .memory(10)
        .start();

    runner.place_on_field(0, "BT22-088", Some(0));
    let field_before = runner.battle_area_size(0);
    let deck_before = runner.deck_size(0);

    runner.game.enter_main_phase();

    // The optional trigger + activation_cost combination installs a
    // TriggerOrder selection so the player can decline BEFORE the cost runs.
    assert!(
        runner.pending_selection().is_some(),
        "a TriggerOrder selection must appear before the return-self cost fires"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder),
        "the pending selection must be a TriggerOrder (not yet in cost/body)"
    );
    assert!(
        runner.pending_is_optional(),
        "TriggerOrder selection for an optional trigger must expose PASS"
    );

    // Player chooses PASS — declines the activation.
    runner
        .execute_action(0, PASS)
        .expect("PASS is a legal action on an optional TriggerOrder");
    let _ = runner.auto_resolve();

    // The Tamer must still be on the field: the cost was not paid.
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "BT22-088 must remain on field after declining the optional activation"
    );
    // The deck must be unchanged (Tamer was not returned).
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "deck must be untouched when the player declines (Tamer not returned)"
    );
}

/// [Start of Your Main Phase] — accept branch: return Tamer, play Arisa from hand.
///
/// After the player accepts the TriggerOrder selection, the activation_cost
/// returns BT22-088 to the deck bottom. A `select_hand` prompt then appears
/// filtered to exact name "Arisa Kinosaki". Picking the card plays it for free.
#[test]
fn bt22_088_start_of_main_accepts_cost_then_plays_exact_arisa_from_hand() {
    // A copy of Arisa Kinosaki with a different card ID (the one in hand).
    let arisa_copy = {
        let mut c = make_test_card("ARISA-COPY-BT22088", "Arisa Kinosaki");
        c.card_kind = CardKind::Tamer;
        c.play_cost = 3;
        c
    };
    // A different card that should NOT match the exact-name filter.
    let non_matching = {
        let mut c = make_test_card("NON-ARISA-BT22088", "Other Tamer");
        c.card_kind = CardKind::Tamer;
        c.play_cost = 2;
        c
    };
    let filler = {
        let mut c = make_test_card("FILLER-BT22088-ARISA", "Filler");
        c.card_kind = CardKind::Digimon;
        c.level = Some(3);
        c.dp = Some(1000);
        c.play_cost = 0;
        c
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .add_card(arisa_copy)
        .add_card(non_matching)
        .add_card(filler)
        .hand(0, &["ARISA-COPY-BT22088", "NON-ARISA-BT22088"])
        .deck(0, &["FILLER-BT22088-ARISA"; 3])
        .deck(1, &["FILLER-BT22088-ARISA"; 3])
        .memory(10)
        .start();

    runner.place_on_field(0, "BT22-088", Some(0));
    let memory_before = runner.memory();

    runner.game.enter_main_phase();

    // Accept the TriggerOrder (pick the trigger, not PASS).
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder),
        "TriggerOrder must appear before cost fires"
    );
    let trigger_action = {
        let sel = runner.pending_selection().unwrap();
        sel.valid_action_ids[0]
    };
    runner
        .execute_action(0, trigger_action)
        .expect("accept the trigger");
    // Accepting the TriggerOrder drains through the activation_cost
    // (BT22-088 returns to deck bottom) and parks at the next pending
    // selection — the select_hand for "Arisa Kinosaki". We deliberately do
    // NOT call auto_resolve() here: auto_resolve would auto-pick the hand
    // selection's first valid action, consuming the prompt before we can
    // assert on the exact-name filter.

    // The select_hand for Arisa Kinosaki must now be pending. This assertion
    // is unconditional: the exact-name-filter discrimination below only has
    // value if the Hand selection genuinely appears, so a skipped branch must
    // fail the test rather than silently pass.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "a select_hand prompt for [Arisa Kinosaki] must appear after the cost is paid"
    );
    // Verify the exact-name filter: ONLY the exact-name Arisa (ARISA-COPY) is
    // valid — the differently-named NON-ARISA-BT22088 must be excluded.
    let valid = {
        let pending = runner.pending_selection().unwrap();
        pending.valid_action_ids.clone()
    };
    assert_eq!(
        valid.len(),
        1,
        "only the exact-name [Arisa Kinosaki] copy must be valid, not [Other Tamer]; \
         valid_action_ids count = {}",
        valid.len()
    );

    // Pick the one valid action — it must be the Arisa copy.
    runner
        .execute_action(0, valid[0])
        .expect("select Arisa Kinosaki from hand");
    runner.auto_resolve().expect("settle Arisa play");

    // BT22-088 is NOT on field (returned as activation cost).
    let arisa_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT22-088");
    assert!(
        !arisa_on_field,
        "BT22-088 must have left the field as the return-self activation cost"
    );

    // The Arisa copy is now on field (played for free).
    let copy_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "ARISA-COPY-BT22088");
    assert!(
        copy_on_field,
        "Arisa Kinosaki copy must be on field after free play"
    );

    // Memory must not have been deducted for Arisa's play cost.
    // (Memory changes from activation_cost and game phase transitions are OK;
    //  but Arisa's printed cost of 3 must not be deducted.)
    let memory_after = runner.memory();
    assert_eq!(
        memory_before, memory_after,
        "play_from_hand_free must not deduct Arisa's printed cost; \
         memory_before={memory_before}, memory_after={memory_after}"
    );
}

/// [Start of Your Main Phase] — no-Digimon branch: return Tamer, skip Arisa
/// (none in hand), then offer Shoemon from trash.
///
/// When there are no Digimon on the player's field, after the Tamer returns
/// and the Arisa hand-selection is skipped (no valid targets), the `if` branch
/// fires and a `select_trash` prompt appears for an exact-name [Shoemon].
#[test]
fn bt22_088_start_of_main_no_digimon_branch_plays_exact_shoemon_from_trash() {
    let shoemon = {
        let mut c = make_test_card("SHOEMON-BT22088", "Shoemon");
        c.card_kind = CardKind::Digimon;
        c.level = Some(3);
        c.dp = Some(2000);
        c.play_cost = 3;
        c
    };
    // A non-Shoemon trash card that must NOT be offered.
    let other_trash = {
        let mut c = make_test_card("OTHER-TRASH-BT22088", "ShoeShoemon");
        c.card_kind = CardKind::Digimon;
        c.level = Some(3);
        c.dp = Some(2000);
        c.play_cost = 3;
        c
    };
    let filler = {
        let mut c = make_test_card("FILLER-BT22088-SHOEMON", "Filler");
        c.card_kind = CardKind::Digimon;
        c.level = Some(3);
        c.dp = Some(1000);
        c.play_cost = 0;
        c
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .add_card(shoemon)
        .add_card(other_trash)
        .add_card(filler)
        // No Arisa in hand; no other Digimon on field.
        .deck(0, &["FILLER-BT22088-SHOEMON"; 3])
        .deck(1, &["FILLER-BT22088-SHOEMON"; 3])
        .memory(10)
        .start();

    runner.place_on_field(0, "BT22-088", Some(0));

    // Manually place Shoemon and ShoeShoemon in player 0's trash.
    let shoemon_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "SHOEMON-BT22088")
        .expect("Shoemon registered");
    let other_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OTHER-TRASH-BT22088")
        .expect("ShoeShoemon registered");

    let next1 = runner.game.next_card_index();
    runner.game.players[0]
        .trash
        .push(digimon_engine::card_source::CardSource::new(
            shoemon_data_idx,
            0,
            next1,
        ));
    let next2 = runner.game.next_card_index();
    runner.game.players[0]
        .trash
        .push(digimon_engine::card_source::CardSource::new(
            other_data_idx,
            0,
            next2,
        ));

    let trash_before = runner.trash_size(0);

    runner.game.enter_main_phase();

    // Accept the TriggerOrder (pick the trigger).
    assert_eq!(runner.pending_kind(), Some(SelectionKind::TriggerOrder));
    let trigger_action = {
        let sel = runner.pending_selection().unwrap();
        sel.valid_action_ids[0]
    };
    runner
        .execute_action(0, trigger_action)
        .expect("accept trigger");
    // Accepting the TriggerOrder drains through the body:
    //   - activation_cost fires → BT22-088 returns to deck bottom;
    //   - the optional select_hand for [Arisa Kinosaki] has no valid target
    //     (none in hand), so it installs no prompt and is silently skipped;
    //   - the `if: no Digimon` branch fires → select_trash for [Shoemon].
    // The drain parks at that select_trash. We deliberately do NOT call
    // auto_resolve() here: auto_resolve would auto-pick the trash selection's
    // first valid action, consuming the prompt before we can assert on the
    // exact-name filter.

    // The select_trash for Shoemon must now be pending. This assertion is
    // unconditional: the exact-name discrimination below (Shoemon vs
    // ShoeShoemon) only has value if the Trash selection genuinely appears,
    // so a skipped branch must fail the test rather than silently pass.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "a select_trash prompt for [Shoemon] must appear in the no-Digimon branch"
    );
    let valid = {
        let pending = runner.pending_selection().unwrap();
        pending.valid_action_ids.clone()
    };
    // Verify exact-name: only SHOEMON-BT22088 is valid, not
    // OTHER-TRASH-BT22088 (named "ShoeShoemon" — a substring near-miss).
    assert_eq!(
        valid.len(),
        1,
        "only exact-name [Shoemon] must be valid, not [ShoeShoemon]; \
         valid_action_ids count = {}",
        valid.len()
    );

    // Pick Shoemon.
    runner
        .execute_action(0, valid[0])
        .expect("select Shoemon from trash");
    runner.auto_resolve().expect("settle Shoemon play");

    // Shoemon is now on field (played free from trash).
    let shoemon_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "SHOEMON-BT22088");
    assert!(
        shoemon_on_field,
        "Shoemon must be on field after free play from trash"
    );

    // BT22-088 is not on field (returned as activation cost).
    let arisa_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT22-088");
    assert!(
        !arisa_on_field,
        "BT22-088 must have left the field as the return-self activation cost"
    );

    // Trash decreased by 1 (Shoemon left trash; ShoeShoemon remains).
    assert_eq!(
        runner.trash_size(0),
        trash_before - 1,
        "trash must shrink by 1 after Shoemon is played free"
    );
}

#[test]
fn bt22_088_all_turns_token_or_puppet_played_suspends_this_tamer_to_draw() {
    let mut runner = observer_runner(&["PUPPET-HAND-BT22-088"], &["DRAW-BT22-088"]);
    let arisa = runner.place_on_field(0, "BT22-088", Some(0));

    runner.play(0, 0).expect("own Puppet plays");
    runner
        .auto_resolve()
        .expect("finish activation_cost + Draw 1");

    assert!(
        runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "BT22-088 must suspend as the activation cost"
    );
    assert_eq!(runner.hand_size(0), 1, "Draw 1 adds the deck card to hand");
    assert_eq!(runner.deck_size(0), 0, "Draw 1 consumes one deck card");
}

#[test]
fn bt22_088_token_played_by_effect_can_trigger_draw_observer() {
    let mut runner = observer_runner(&[], &["DRAW-BT22-088"]);
    let arisa = runner.place_on_field(0, "BT22-088", Some(0));
    let source = runner.place_on_field(0, "EFFECT-SOURCE-BT22-088", Some(0));

    let source_card = runner.top_card(source);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
    ctx.play_token(0, "familiar")
        .expect("effect should play Familiar Token");

    runner.auto_resolve().expect("finish Token observer");

    assert!(
        runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "BT22-088 must suspend for the Token branch"
    );
    assert_eq!(runner.hand_size(0), 1);
    assert_eq!(runner.deck_size(0), 0);
}

#[test]
fn bt22_088_played_observer_silently_skips_when_arisa_already_suspended() {
    let mut runner = observer_runner(&["PUPPET-HAND-BT22-088"], &["DRAW-BT22-088"]);
    let arisa = runner.place_on_field(0, "BT22-088", Some(0));
    runner.game.player_mut(0).battle_area[arisa.index as usize].is_suspended = true;

    runner.play(0, 0).expect("own Puppet plays");
    runner.auto_resolve().expect("finish trigger");

    assert!(
        runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "pre-suspended Arisa stays suspended (cost cannot be paid again)"
    );
    assert_eq!(runner.hand_size(0), 0, "cost failure must not draw");
    assert_eq!(
        runner.deck_size(0),
        1,
        "cost failure leaves the deck untouched"
    );
}

#[test]
fn bt22_088_played_observer_rejects_non_puppet_and_opponent_puppet() {
    let mut runner = observer_runner(
        &["NON-PUPPET-BT22-088"],
        &["DRAW-BT22-088", "DRAW-BT22-088"],
    );
    runner.place_on_field(0, "BT22-088", Some(0));

    runner.play(0, 0).expect("own non-Puppet plays");
    runner.auto_resolve().expect("settle non-Puppet play");
    assert!(
        runner.pending_selection_view().is_none(),
        "own non-Puppet Digimon must not trigger"
    );

    runner.play(1, 0).expect("opponent Puppet plays");
    runner.auto_resolve().expect("settle opponent Puppet play");
    assert!(
        runner.pending_selection_view().is_none(),
        "opponent Puppet Digimon must not trigger"
    );
}

#[test]
fn bt22_088_suspend_cost_preflight_is_bound_to_this_tamer() {
    let mut runner = observer_runner(
        &["PUPPET-HAND-BT22-088"],
        &["DRAW-BT22-088", "DRAW-BT22-088"],
    );
    let suspended = runner.place_on_field(0, "BT22-088", Some(0));
    let unsuspended = runner.place_on_field(0, "BT22-088", Some(1));
    runner.game.player_mut(0).battle_area[suspended.index as usize].is_suspended = true;

    runner.play(0, 0).expect("own Puppet plays");
    runner
        .auto_resolve()
        .expect("finish source-bound activation");

    assert!(
        runner.game.player(0).battle_area[suspended.index as usize].is_suspended,
        "already-suspended source stays suspended and must not pay the cost again"
    );
    assert!(
        runner.game.player(0).battle_area[unsuspended.index as usize].is_suspended,
        "the unsuspended BT22-088 source should pay the cost"
    );
    assert!(
        runner.pending_selection_view().is_none(),
        "only the source that can pay the suspend cost should trigger"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "exactly one Arisa pays the cost and draws once"
    );
}

fn observer_runner(hand: &[&str], deck: &[&str]) -> DebugRunner {
    let mut source = make_test_card("EFFECT-SOURCE-BT22-088", "Effect Source");
    source.card_kind = CardKind::Digimon;
    source.level = Some(3);
    source.play_cost = 0;
    source.dp = Some(1000);

    DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .add_card(source)
        .add_card(puppet_digimon("PUPPET-HAND-BT22-088"))
        .add_card(non_puppet_digimon("NON-PUPPET-BT22-088"))
        .add_card(draw_card("DRAW-BT22-088"))
        .hand(0, hand)
        .hand(1, &["PUPPET-HAND-BT22-088"])
        .deck(0, deck)
        .memory(20)
        .start()
}

fn puppet_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.traits = vec!["Puppet".to_string()];
    card
}

fn non_puppet_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.traits = vec!["LIBERATOR".to_string()];
    card
}

fn draw_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 2;
    card
}

fn predicate_has_event_target_kind(predicate: &CompiledPredicate, kind: CompiledCardKind) -> bool {
    predicate.event_target_kind == Some(kind)
        || predicate
            .all_of
            .iter()
            .any(|child| predicate_has_event_target_kind(child, kind))
        || predicate
            .any_of
            .iter()
            .any(|child| predicate_has_event_target_kind(child, kind))
}

fn predicate_has_event_target_trait(predicate: &CompiledPredicate, trait_name: &str) -> bool {
    predicate.event_target_trait_has.as_deref() == Some(trait_name)
        || predicate
            .all_of
            .iter()
            .any(|child| predicate_has_event_target_trait(child, trait_name))
        || predicate
            .any_of
            .iter()
            .any(|child| predicate_has_event_target_trait(child, trait_name))
}

fn steps_lead_with_activation_cost_suspend_self_then_draw(steps: &[CompiledStep]) -> bool {
    use digimon_dsl::compiled::CompiledActivationCostKind;
    let mut iter = steps.iter();
    let leads_with_suspend_self = matches!(
        iter.next(),
        Some(CompiledStep::ActivationCost {
            kind: CompiledActivationCostKind::SuspendSelf
        })
    );
    if !leads_with_suspend_self {
        return false;
    }
    matches!(iter.next(), Some(CompiledStep::Draw { count: 1, .. }))
}
