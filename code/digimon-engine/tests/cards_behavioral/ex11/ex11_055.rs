//! EX11-055 Chitose Horaiji — Tamer, Red/Purple, Cost 4.
//! Traits: LIBERATOR.
//!
//! # Card text (official Bandai DB bundle, data/card_bundles/EX11-055.md —
//! authoritative; cross-checked against data/cards.json which agrees)
//!
//! [On Play] [Start of Your Main Phase] By trashing 1 [Composite] or [Wicked
//! God] trait card from your hand, <Draw 1> (Draw 1 card from your deck.)
//! and gain 1 memory.
//! [All Turns] When any of your [Composite] or [Wicked God] trait Digimon
//! are deleted, by suspending this Tamer, you may play 1 [Gazimon] or
//! [Gizamon] from your hand without paying the cost.
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO reference
//!
//! DCGO/Assets/Scripts/CardEffect/EX11/Red/EX11_055.cs
//! - Shared OnStartMainPhase / OnEnterFieldAnyone: SharedCanActivateCondition
//!   = IsExistOnBattleArea && HasMatchConditionOwnersHand(ValidDiscardCard);
//!   ValidDiscardCard = EqualsTraits("Composite") || EqualsTraits("Wicked
//!   God"). SharedActivateCoroutine: SelectHandEffect(canNoSelect:true,
//!   maxCount:1, mode:Discard) -> if discarded: Draw 1 + AddMemory(1).
//! - OnDestroyedAnyone (All Turns, maxCountPerTurn -1 = unlimited, optional):
//!   CanUseCondition = IsExistOnBattleArea && CanTriggerOnPermanentDeleted(
//!   PermanentCondition); PermanentCondition = owner-battle-area Digimon &&
//!   (EqualsTraits("Composite") || EqualsTraits("Wicked God")) on the
//!   DELETED permanent's top card. CanActivateCondition =
//!   CanActivateSuspendCostEffect(card). ActivateCoroutine: suspend this
//!   Tamer (Tap) UNCONDITIONALLY first, THEN if HasMatchConditionOwnersHand
//!   (CanSelectCardCondition: EqualsCardName("Gazimon") ||
//!   EqualsCardName("Gizamon")) offers SelectHandEffect(canNoSelect:true) ->
//!   PlayPermanentCards(payCost:false).
//! - SecuritySkill: CardEffectFactory.PlaySelfTamerSecurityEffect(card).
//!
//! # Patterns this test covers
//! - Section 1: structural assertions (metadata, clause shapes).
//! - Section 2: Clause 1 shared On Play / Start of Your Main Phase — trash
//!   cost gating Draw 1 + gain 1 memory, optionality, trait filter
//!   (Composite / Wicked God only), no-candidate no-prompt.
//! - Section 3: Clause 2 [All Turns] deletion observer — event_target_owner/
//!   event_target_kind/event_target_trait_has gating (own Composite/Wicked
//!   God Digimon only), activation_cost { suspend_self } cost gating,
//!   optional Gazimon/Gizamon free hand-play, exact-name gate (no
//!   substring match), decline paths.
//! - Section 4: Clause 3 [Security] play-self-free.
//! - Section 5: event-log coverage for the Clause 1 trash cost.
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | Clause 1 event-log (Trash event on trash cost) | `Game::trash_from_hand_by_index` (code/digimon-engine/src/game_actions/zones.rs:177-190) moves the card directly (`player.hand.remove` + `player.trash.push`) and does NOT call `Game::trash_card`, so no `GameEvent::Trash` is emitted for this DSL step (`CompiledStep::TrashFromHandByIndex` at code/digimon-engine/src/dsl_cards/step/zone_moves.rs:466-479 calls `ctx.trash_from_hand_by_index`, which is `EffectContext::trash_from_hand_by_index` at code/digimon-engine/src/effect_context/action/trash.rs:371-398, which itself calls `Game::trash_from_hand_by_index`, not `Game::trash_card`). Empirically verified by un-ignoring the identically-shaped BT24-008 sibling test (`bt24_008_on_play_accept_emits_trash_event_for_cost_card`) and running it: it FAILS with "a Trash event must fire when the cost card is trashed from hand" (confirmed 2026-07-02 in this worktree). This is a genuine, still-open engine gap (`engine-trash-event-from-hand`) — the runtime behavior of the trash itself (card leaves hand, enters trash) is fully faithful; only the *event log instrumentation* is missing. | Test authored below and marked `#[ignore]` with an accurate docstring, matching the (correctly still-ignored) BT24-008 sibling. |

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::events::GameEvent;
use digimon_engine::{
    card_data::CardData, replacement::ReplacementCause, selection::SelectionKind,
};

// ============================================================================
// Section 1 — Structural assertions
// ============================================================================

fn load_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-055")
        .expect("EX11-055 YAML loads")
        .memory(10)
        .start()
}

#[test]
fn ex11_055_has_printed_metadata() {
    let runner = load_runner();
    let compiled = runner
        .compiled_card("EX11-055")
        .expect("EX11-055 must be compiled");

    assert_eq!(compiled.name, "Chitose Horaiji");
    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Tamer
    );
    assert_eq!(compiled.cost, Some(4));
    assert_eq!(
        compiled.color,
        vec![
            digimon_dsl::compiled::CompiledColor::Red,
            digimon_dsl::compiled::CompiledColor::Purple
        ]
    );
}

#[test]
fn ex11_055_has_shared_on_play_and_start_of_main_phase_clause() {
    let runner = load_runner();
    let compiled = runner
        .compiled_card("EX11-055")
        .expect("EX11-055 must be compiled");

    let shared = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .next()
        .expect("shared On Play / Start of Your Main Phase clause must exist");

    assert!(
        shared.when.contains(&CompiledTiming::StartOfYourMainPhase),
        "the shared clause must ALSO fire at Start of Your Main Phase"
    );
    assert!(
        shared.optional,
        "the trash cost is a 'you may' — DCGO canNoSelect:true"
    );
}

#[test]
fn ex11_055_has_all_turns_deletion_observer() {
    let runner = load_runner();
    let compiled = runner
        .compiled_card("EX11-055")
        .expect("EX11-055 must be compiled");

    let observer = find_deletion_observer(compiled)
        .expect("EX11-055 must ship an [All Turns] OnAnyDeletion observer");

    assert!(
        observer.when.contains(&CompiledTiming::OnAnyDeletion),
        "deletion observer must trigger on on_any_deletion"
    );
    assert_eq!(
        observer.scope,
        CompiledScope::FaceUp,
        "printed [All Turns] clause is a face-up effect of this Tamer"
    );
    assert!(
        observer.active_when.is_some(),
        "[All Turns] window must be encoded via active_when"
    );
    assert!(
        !observer.once_per_turn,
        "printed text has no [Once Per Turn]; DCGO's maxCountPerTurn is -1 (unlimited); the suspend cost is the natural limiter"
    );
    assert!(
        observer.optional,
        "the suspend cost + Gazimon/Gizamon play are both 'you may' via DCGO canNoSelect:true"
    );
    assert!(
        observer.condition.is_some(),
        "observer must gate on the deleted-object event context (owner/kind/trait)"
    );
}

#[test]
fn ex11_055_has_security_clause() {
    let runner = load_runner();
    let compiled = runner
        .compiled_card("EX11-055")
        .expect("EX11-055 must be compiled");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect();

    assert!(
        triggered
            .iter()
            .any(|clause| clause.when.contains(&CompiledTiming::OnSecurity)),
        "supported YAML must include the printed Security clause"
    );
}

// ============================================================================
// Section 2 — Clause 1: shared On Play / Start of Your Main Phase
// ============================================================================

fn composite_hand_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.traits = vec!["Composite".to_string()];
    card
}

fn wicked_god_hand_card(id: &str) -> CardData {
    let mut card = composite_hand_card(id);
    card.traits = vec!["Wicked God".to_string()];
    card
}

fn non_matching_hand_card(id: &str) -> CardData {
    let mut card = composite_hand_card(id);
    card.traits = vec!["Beast".to_string()];
    card
}

fn draw_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

#[test]
fn ex11_055_on_play_offers_only_composite_and_wicked_god_trait_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-055")
        .expect("EX11-055 YAML loads")
        .add_card(composite_hand_card("COMPOSITE-HAND"))
        .add_card(wicked_god_hand_card("WICKEDGOD-HAND"))
        .add_card(non_matching_hand_card("BEAST-HAND"))
        .add_card(draw_filler("DRAW-FILLER"))
        .hand(
            0,
            &["EX11-055", "COMPOSITE-HAND", "WICKEDGOD-HAND", "BEAST-HAND"],
        )
        .deck(0, &["DRAW-FILLER"])
        .memory(10)
        .start();

    runner.play(0, 0);

    let view = runner
        .pending_selection_view()
        .expect("On Play should offer the trash-cost trait pick");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "the trash cost must allow PASS (DCGO canNoSelect:true)"
    );
    // Only the Composite and Wicked God cards should be selectable, plus PASS.
    let non_pass_count = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != digimon_engine::action::space::PASS)
        .count();
    assert_eq!(
        non_pass_count, 2,
        "only the Composite and Wicked God trait cards should be selectable; the Beast card must be filtered out"
    );
}

#[test]
fn ex11_055_on_play_trashing_composite_card_draws_and_gains_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-055")
        .expect("EX11-055 YAML loads")
        .add_card(composite_hand_card("COMPOSITE-HAND"))
        .add_card(draw_filler("DRAW-FILLER"))
        .hand(0, &["EX11-055", "COMPOSITE-HAND"])
        .deck(0, &["DRAW-FILLER"])
        .memory(9)
        .start();

    let hand_before = runner.hand_size(0);
    runner.play(0, 0);
    let memory_after_tamer_cost = runner.memory();

    let action = runner
        .pending_selection_view()
        .expect("Composite selection must be pending")
        .valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("select the Composite card to trash");
    runner
        .auto_resolve()
        .expect("resolve trash cost + Draw 1 + gain 1 memory");

    // Hand: started at 2 (EX11-055 + COMPOSITE-HAND). EX11-055 leaves hand to
    // play, COMPOSITE-HAND leaves hand to trash, DRAW-FILLER enters hand.
    // Net vs hand_before (2): -1 (EX11-055 played) -1 (trashed) +1 (drawn) = -1.
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "net hand size: EX11-055 played (-1), Composite trashed (-1), 1 card drawn (+1)"
    );
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "COMPOSITE-HAND"),
        "the selected Composite card must be trashed"
    );
    assert_eq!(runner.deck_size(0), 0, "Draw 1 must consume the deck card");
    assert_eq!(
        runner.memory(),
        memory_after_tamer_cost + 1,
        "gain 1 memory after the trash cost fires"
    );
}

#[test]
fn ex11_055_on_play_declining_trash_cost_does_not_draw_or_gain_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-055")
        .expect("EX11-055 YAML loads")
        .add_card(composite_hand_card("COMPOSITE-HAND"))
        .add_card(draw_filler("DRAW-FILLER"))
        .hand(0, &["EX11-055", "COMPOSITE-HAND"])
        .deck(0, &["DRAW-FILLER"])
        .memory(9)
        .start();

    runner.play(0, 0);
    let memory_after_tamer_cost = runner.memory();

    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the optional trash cost");
    runner.auto_resolve().expect("resolve declined On Play");

    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "COMPOSITE-HAND"),
        "declining must leave the Composite card in hand"
    );
    assert_eq!(runner.deck_size(0), 1, "declining must not Draw");
    assert_eq!(
        runner.memory(),
        memory_after_tamer_cost,
        "declining must not gain memory"
    );
}

#[test]
fn ex11_055_on_play_does_not_prompt_without_eligible_hand_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-055")
        .expect("EX11-055 YAML loads")
        .add_card(non_matching_hand_card("BEAST-HAND"))
        .hand(0, &["EX11-055", "BEAST-HAND"])
        .memory(10)
        .start();

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "with no Composite/Wicked God card in hand, the trash-cost prompt must not surface"
    );
}

#[test]
fn ex11_055_start_of_main_phase_offers_trash_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-055")
        .expect("EX11-055 YAML loads")
        .add_card(composite_hand_card("COMPOSITE-HAND"))
        .add_card(draw_filler("DRAW-FILLER-1"))
        .add_card(draw_filler("DRAW-FILLER-2"))
        .hand(0, &["COMPOSITE-HAND"])
        .deck(0, &["DRAW-FILLER-1"])
        .deck(1, &["DRAW-FILLER-2"])
        .memory(10)
        .start();
    runner.place_on_field(0, "EX11-055", Some(0));

    // Cycle to P0's next turn (Start of Your Main Phase for P0 fires on
    // P0's own turn, so end_turn twice: P0 -> P1 -> P0).
    runner.end_turn();
    runner.end_turn();

    assert!(
        runner.pending_selection().is_some(),
        "Start of Your Main Phase must re-offer the shared trash-cost clause"
    );
}

// ============================================================================
// Section 3 — Clause 2: [All Turns] deletion observer
// ============================================================================

fn horaiji_deletion_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-055")
        .expect("EX11-055 YAML loads")
        .add_card(composite_digimon("COMPOSITE-SAC", 4, 4000))
        .add_card(wicked_god_digimon("WICKEDGOD-SAC", 5, 5000))
        .add_card(non_matching_digimon("NON-MATCH-SAC", 4, 4000))
        .add_card(gazimon_hand("GAZIMON-HAND"))
        .add_card(gizamon_hand("GIZAMON-HAND"))
        .add_card(non_gazimon_hand("NON-GAZIMON-HAND"))
        .add_card(make_test_card("DRAW-FILLER-EX11-055", "Draw Filler"))
        .hand(0, &["GAZIMON-HAND"])
        .start()
}

fn composite_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = 4;
    card.traits = vec!["Composite".to_string()];
    card
}

fn wicked_god_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = composite_digimon(id, level, dp);
    card.traits = vec!["Wicked God".to_string()];
    card
}

fn non_matching_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = composite_digimon(id, level, dp);
    card.traits = vec!["Beast".to_string()];
    card
}

fn gazimon_hand(id: &str) -> CardData {
    let mut card = make_test_card(id, "Gazimon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(2);
    card.dp = Some(2000);
    card.play_cost = 2;
    card.traits = vec!["Beast Man".to_string()];
    card
}

fn gizamon_hand(id: &str) -> CardData {
    let mut card = gazimon_hand(id);
    card.card_name = "Gizamon".to_string();
    card
}

/// A card whose name merely CONTAINS "Gazimon" (not an exact match) — must
/// NOT be offered by the exact-name `name_is` gate (DCGO EqualsCardName).
fn non_gazimon_hand(id: &str) -> CardData {
    let mut card = gazimon_hand(id);
    card.card_name = "Gazimon (X Antibody)".to_string();
    card
}

#[test]
fn ex11_055_all_turns_fires_when_own_composite_digimon_deleted() {
    let mut runner = horaiji_deletion_runner();
    let horaiji = runner.place_on_field(0, "EX11-055", Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-SAC", Some(0));

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    // Cost-bearing optional trigger: the engine's `needs_pre_cost_prompt`
    // path installs a single `SelectionKind::TriggerOrder` prompt
    // (`is_optional: true`, PASS available) as the sole accept/decline gate
    // BEFORE the `activation_cost` (suspend) runs. Accepting it fires the
    // clause (suspend + Gazimon/Gizamon pick) in one shot.
    let view = runner
        .pending_selection_view()
        .expect("own Composite Digimon deletion must offer the suspend-to-play activation");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert!(view.is_optional, "the cost-bearing trigger must allow PASS");
    let accept = view.valid_action_ids[0];
    runner
        .execute_action(view.selecting_player, accept)
        .expect("accept the suspend cost");
    runner.auto_resolve().expect("finish Composite branch");

    assert!(
        runner.game.player(0).battle_area[horaiji.index as usize].is_suspended,
        "paying the suspend cost suspends this Tamer"
    );
}

#[test]
fn ex11_055_all_turns_fires_when_own_wicked_god_digimon_deleted() {
    let mut runner = horaiji_deletion_runner();
    let horaiji = runner.place_on_field(0, "EX11-055", Some(0));
    let wicked_god = runner.place_on_field(0, "WICKEDGOD-SAC", Some(0));

    runner
        .game
        .delete_permanent_with_cause(wicked_god, ReplacementCause::OwnEffect);

    let view = runner
        .pending_selection_view()
        .expect("own Wicked God Digimon deletion must offer the suspend-to-play activation");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert!(view.is_optional, "the cost-bearing trigger must allow PASS");
    let accept = view.valid_action_ids[0];
    runner
        .execute_action(view.selecting_player, accept)
        .expect("accept the suspend cost");
    runner.auto_resolve().expect("finish Wicked God branch");

    assert!(
        runner.game.player(0).battle_area[horaiji.index as usize].is_suspended,
        "paying the suspend cost suspends this Tamer"
    );
}

#[test]
fn ex11_055_all_turns_does_not_fire_for_non_matching_own_digimon_deletion() {
    let mut runner = horaiji_deletion_runner();
    runner.place_on_field(0, "EX11-055", Some(0));
    let non_match = runner.place_on_field(0, "NON-MATCH-SAC", Some(0));

    runner
        .game
        .delete_permanent_with_cause(non_match, ReplacementCause::OwnEffect);
    drain_trigger_order_if_any(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "deleting your own non-Composite/non-Wicked-God Digimon must not offer the activation"
    );
}

#[test]
fn ex11_055_all_turns_does_not_fire_for_opponent_composite_deletion() {
    let mut runner = horaiji_deletion_runner();
    runner.place_on_field(0, "EX11-055", Some(0));
    let opp_composite = runner.place_on_field(1, "COMPOSITE-SAC", Some(1));

    runner
        .game
        .delete_permanent_with_cause(opp_composite, ReplacementCause::OwnEffect);
    drain_trigger_order_if_any(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "printed 'any of YOUR Composite/Wicked God Digimon' excludes the opponent's board"
    );
}

#[test]
fn ex11_055_all_turns_cannot_activate_when_horaiji_already_suspended() {
    let mut runner = horaiji_deletion_runner();
    let horaiji = runner.place_on_field(0, "EX11-055", Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-SAC", Some(0));
    runner.game.players[0].battle_area[horaiji.index as usize].is_suspended = true;

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);
    drain_trigger_order_if_any(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "a suspended Horaiji cannot pay the suspend cost, so no activation prompt appears"
    );
}

#[test]
fn ex11_055_all_turns_suspend_activation_is_optional_and_may_be_declined() {
    let mut runner = horaiji_deletion_runner();
    let horaiji = runner.place_on_field(0, "EX11-055", Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-SAC", Some(0));

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    let view = runner
        .pending_selection_view()
        .expect("deletion must offer the suspend activation choice");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert!(view.is_optional, "the cost-bearing trigger must allow PASS");
    // PASS declines the cost-bearing trigger outright (the pre-cost prompt
    // IS the accept/decline gate — see the YAML Clause 2 comment).
    runner
        .execute_action(view.selecting_player, digimon_engine::action::space::PASS)
        .expect("decline the suspend activation via PASS");
    runner.auto_resolve().expect("resolve after decline");

    assert!(
        !runner.game.player(0).battle_area[horaiji.index as usize].is_suspended,
        "declining the optional activation must leave Horaiji unsuspended"
    );
}

#[test]
fn ex11_055_all_turns_may_play_gazimon_from_hand_for_free() {
    let mut runner = horaiji_deletion_runner();
    let _horaiji = runner.place_on_field(0, "EX11-055", Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-SAC", Some(0));

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    // Accept the cost-bearing TriggerOrder prompt (the sole accept/decline
    // gate for this clause — see the YAML Clause 2 comment).
    let accept_view = runner
        .pending_selection_view()
        .expect("deletion must offer the suspend activation choice");
    assert_eq!(accept_view.kind, SelectionKind::TriggerOrder);
    let accept = accept_view.valid_action_ids[0];
    runner
        .execute_action(accept_view.selecting_player, accept)
        .expect("accept the suspend cost");

    let hand_pick = runner
        .pending_selection_view()
        .expect("after suspending, the Gazimon/Gizamon free-play must be offered");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    let play_action = hand_pick
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .expect("Gazimon hand card action");
    runner
        .execute_action(hand_pick.selecting_player, play_action)
        .expect("choose Gazimon from hand");
    runner.auto_resolve().expect("finish Gazimon free-play");

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "GAZIMON-HAND"),
        "the selected Gazimon should be played to the field for free"
    );
}

#[test]
fn ex11_055_all_turns_may_play_gizamon_from_hand_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-055")
        .expect("EX11-055 YAML loads")
        .add_card(composite_digimon("COMPOSITE-SAC", 4, 4000))
        .add_card(gizamon_hand("GIZAMON-HAND"))
        .hand(0, &["GIZAMON-HAND"])
        .start();
    let _horaiji = runner.place_on_field(0, "EX11-055", Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-SAC", Some(0));

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    let accept_view = runner
        .pending_selection_view()
        .expect("deletion must offer the suspend activation choice");
    assert_eq!(accept_view.kind, SelectionKind::TriggerOrder);
    let accept = accept_view.valid_action_ids[0];
    runner
        .execute_action(accept_view.selecting_player, accept)
        .expect("accept the suspend cost");

    let hand_pick = runner
        .pending_selection_view()
        .expect("after suspending, the Gazimon/Gizamon free-play must be offered");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    let play_action = hand_pick
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .expect("Gizamon hand card action");
    runner
        .execute_action(hand_pick.selecting_player, play_action)
        .expect("choose Gizamon from hand");
    runner.auto_resolve().expect("finish Gizamon free-play");

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "GIZAMON-HAND"),
        "the selected Gizamon should be played to the field for free"
    );
}

#[test]
fn ex11_055_all_turns_free_play_pick_is_optional_and_may_be_declined() {
    let mut runner = horaiji_deletion_runner();
    let _horaiji = runner.place_on_field(0, "EX11-055", Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-SAC", Some(0));

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    let accept_view = runner
        .pending_selection_view()
        .expect("deletion must offer the suspend activation choice");
    assert_eq!(accept_view.kind, SelectionKind::TriggerOrder);
    let accept = accept_view.valid_action_ids[0];
    runner
        .execute_action(accept_view.selecting_player, accept)
        .expect("accept the suspend cost");

    let hand_pick = runner
        .pending_selection_view()
        .expect("after suspending, the Gazimon/Gizamon free-play must be offered");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    runner
        .execute_action(
            hand_pick.selecting_player,
            digimon_engine::action::space::PASS,
        )
        .expect("decline the optional Gazimon/Gizamon free-play");
    runner
        .auto_resolve()
        .expect("finish after declining the free-play");

    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "GAZIMON-HAND"),
        "declining the optional free-play must leave Gazimon in hand"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "GAZIMON-HAND"),
        "no Gazimon enters the battle area when the optional play is declined"
    );
}

#[test]
fn ex11_055_all_turns_free_play_does_not_offer_non_gazimon_or_gizamon_named_card() {
    // Only "GAZIMON-HAND" (named exactly "Gazimon") is in hand alongside a
    // card whose name CONTAINS "Gazimon" but is not exactly equal — DCGO's
    // EqualsCardName gate (name_is) must reject the substring-only card.
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-055")
        .expect("EX11-055 YAML loads")
        .add_card(composite_digimon("COMPOSITE-SAC", 4, 4000))
        .add_card(non_gazimon_hand("NON-GAZIMON-HAND"))
        .hand(0, &["NON-GAZIMON-HAND"])
        .start();
    let _horaiji = runner.place_on_field(0, "EX11-055", Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-SAC", Some(0));

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    let accept_view = runner
        .pending_selection_view()
        .expect("deletion must offer the suspend activation choice");
    assert_eq!(accept_view.kind, SelectionKind::TriggerOrder);
    let accept = accept_view.valid_action_ids[0];
    runner
        .execute_action(accept_view.selecting_player, accept)
        .expect("accept the suspend cost");

    // With no exact-name Gazimon/Gizamon in hand, the free-play prompt must
    // not surface at all (DCGO HasMatchConditionOwnersHand(CanSelectCardCondition)
    // gates the SelectHandEffect from even being offered).
    assert!(
        runner.pending_selection().is_none(),
        "a card whose name merely CONTAINS 'Gazimon' must not satisfy the exact-name free-play gate"
    );
}

// ============================================================================
// Section 4 — Clause 3: [Security] play-self-free
// ============================================================================

#[test]
fn ex11_055_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER-EX11-055", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-055")
        .expect("EX11-055 YAML loads")
        .add_card(attacker)
        .security(1, &["EX11-055"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER-EX11-055", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX11-055"),
        "EX11-055 should be played from the defender's security"
    );
    assert_eq!(
        runner.security_count(1),
        0,
        "security card should be consumed by the security check"
    );
}

// ============================================================================
// Section 5 — Clause 1 event-log: Trash event on the cost card
// ============================================================================

/// Accepting the Clause 1 trash cost should fire a Trash event for the
/// discarded Composite/Wicked-God card.
///
/// NOTE (verified 2026-07-02 in this worktree, NOT a stale premise): the
/// engine's hand→trash path does NOT currently emit `GameEvent::Trash`.
/// `CompiledStep::TrashFromHandByIndex` (code/digimon-engine/src/dsl_cards/step/zone_moves.rs:466-479)
/// calls `EffectContext::trash_from_hand_by_index` (code/digimon-engine/src/effect_context/action/trash.rs:371-398),
/// which calls `Game::trash_from_hand_by_index` (code/digimon-engine/src/game_actions/zones.rs:177-190) —
/// that function moves the card directly (`player.hand.remove(hand_index)` +
/// `player.trash.push(card)`) and does NOT call `Game::trash_card` (the
/// event-emitting helper at code/digimon-engine/src/game/mod.rs:1649-1668).
/// So no `GameEvent::Trash` is pushed for this path. Confirmed empirically
/// by temporarily un-ignoring the identically-shaped sibling test
/// `bt24_008_on_play_accept_emits_trash_event_for_cost_card`
/// (code/digimon-engine/tests/cards_behavioral/bt24/bt24_008.rs) and running
/// it: it FAILS with "a Trash event must fire when the cost card is trashed
/// from hand". This is a real, still-open engine gap
/// (`engine-trash-event-from-hand`) — the runtime trash itself (card leaves
/// hand, enters trash zone) is fully faithful; only the event-log
/// instrumentation for this specific hand-to-trash path is missing. Once the
/// engine gap closes (either by routing `Game::trash_from_hand_by_index`
/// through `Game::trash_card`, or by emitting the event directly at the call
/// site), remove the `#[ignore]` here (and on the BT24-008 sibling).
#[test]
#[ignore = "pending: engine-trash-event-from-hand — Game::trash_from_hand_by_index (game_actions/zones.rs) does not call Game::trash_card and so does not emit GameEvent::Trash; empirically re-verified 2026-07-02 via the BT24-008 sibling test"]
fn ex11_055_on_play_trashing_hand_card_fires_trash_event() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-055")
        .expect("EX11-055 YAML loads")
        .add_card(composite_hand_card("COMPOSITE-HAND"))
        .add_card(draw_filler("DRAW-FILLER"))
        .hand(0, &["EX11-055", "COMPOSITE-HAND"])
        .deck(0, &["DRAW-FILLER"])
        .memory(10)
        .start();

    runner.play(0, 0);

    let cp = runner.event_checkpoint();

    let action = runner
        .pending_selection_view()
        .expect("Composite selection must be pending")
        .valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("select the Composite card to trash");
    runner
        .auto_resolve()
        .expect("resolve trash cost + Draw 1 + gain 1 memory");

    let trash_events = runner.events_of_kind(cp, |event| {
        matches!(event, GameEvent::Trash { card_id, player, .. } if card_id == "COMPOSITE-HAND" && *player == 0)
    });
    assert_eq!(
        trash_events.len(),
        1,
        "the Clause 1 trash cost must emit exactly one Trash event for the discarded Composite hand card"
    );
}

// ─── Helpers ───────────────────────────────────────────────────────────

/// Simultaneous-trigger ordering: `on_any_deletion` enqueues via
/// `enqueue_triggered`, which may bundle a `SelectionKind::TriggerOrder`
/// prompt even for a single eligible observer (APNAP simultaneous-trigger
/// ordering semantics). Drain it (picking the first entry) before reading
/// the next real prompt. Mirrors `drain_trigger_order_if_any` in
/// bt16_055.rs.
fn drain_trigger_order_if_any(runner: &mut DebugRunner) -> bool {
    if let Some(view) = runner.pending_selection_view() {
        if view.kind == SelectionKind::TriggerOrder {
            runner
                .execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("resolve TriggerOrder");
            return true;
        }
    }
    false
}

fn find_deletion_observer(
    card: &digimon_dsl::compiled::CompiledCard,
) -> Option<&CompiledTriggeredClause> {
    card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnAnyDeletion) =>
        {
            Some(triggered)
        }
        _ => None,
    })
}
