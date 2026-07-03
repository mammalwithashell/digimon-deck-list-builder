//! EX11-055 Chitose Horaiji — Tamer, Red+Purple, Cost 4.
//! Traits: LIBERATOR.
//!
//! # Card text (data/card_bundles/EX11-055.md — official Bandai DB, matches
//! cards.json `effect_description_eng` / `inherited_effect_description_eng`)
//!
//! [On Play] [Start of Your Main Phase] By trashing 1 [Composite] or [Wicked
//! God] trait card from your hand, ＜Draw 1＞ (Draw 1 card from your deck.)
//! and gain 1 memory.
//! [All Turns] When any of your [Composite] or [Wicked God] trait Digimon
//! are deleted, by suspending this Tamer, you may play 1 [Gazimon] or
//! [Gizamon] from your hand without paying the cost.
//!
//! Inherited (Security Effect):
//! [Security] Play this card without paying the cost.
//!
//! # DCGO reference
//!
//! DCGO/Assets/Scripts/CardEffect/EX11/Red/EX11_055.cs
//! - Shared OP/SOMP (`EffectTiming.OnEnterFieldAnyone` /
//!   `EffectTiming.OnStartMainPhase`): `SharedActivateCoroutine` —
//!   `SelectHandEffect` (`mode: Discard`, `maxCount: 1`, `canNoSelect: true`)
//!   over hand cards matching `EqualsTraits("Composite") ||
//!   EqualsTraits("Wicked God")`. Only when a card was actually discarded:
//!   `DrawClass(..., 1).Draw()` then `AddMemory(1)`. Declining discards
//!   nothing and the draw/memory tail does NOT run — the cost gates the
//!   whole reward (`SetUpICardEffect` cost/effect split).
//! - `EffectTiming.OnDestroyedAnyone` (All Turns): `CanUseCondition` =
//!   `CanTriggerOnPermanentDeleted` gated on the deleted permanent being the
//!   controller's own battle-area Digimon whose top card
//!   `EqualsTraits("Composite") || EqualsTraits("Wicked God")`.
//!   `CanActivateCondition` = `CanActivateSuspendCostEffect` (this Tamer
//!   must be unsuspended). `ActivateCoroutine`: `SuspendPermanentsClass`
//!   (mandatory once activated — the "by suspending this Tamer" cost is the
//!   activation gate itself, always paid when the effect activates), then
//!   IF the hand has a matching `Gazimon`/`Gizamon` card:
//!   `SelectHandEffect` (`mode: Custom`, `maxCount: 1`, `canNoSelect: true`)
//!   -> `PlayPermanentCards(payCost: false)`.
//! - `EffectTiming.SecuritySkill`: `PlaySelfTamerSecurityEffect`.
//!
//! # Patterns this test covers
//! - B1/B3 shared start-of-main + on-play Tamer body (ST23-13 / EX11-069 /
//!   AD1-019 sibling shape).
//! - E2 OPT-adjacent optional cost gating decline aborts the whole clause
//!   (`select_hand` with `cost: true, optional: true`, BT24-008 idiom).
//! - [All Turns] `on_any_deletion` observer with deleted-object event
//!   context (`event_target_owner` / `event_target_kind` /
//!   `event_target_trait_has`), mirroring EX11-060 Arisa Kinosaki.
//! - Track B `activation_cost { suspend_self }` visible mandatory cost that
//!   gates the whole body (AD1-019 clause 2 / ST23-13 idiom).
//! - No-approximations: the trash-cost draw/memory decision, and the
//!   Gazimon/Gizamon hand-play, both surface through `pending_selection`.
//! - Security play-self-free (AD1-019 / EX11-060 sibling pattern).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX11-055";

// ─── Card-data factories ─────────────────────────────────────────────────

fn composite_hand_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = vec!["Composite".to_string()];
    c
}

fn wicked_god_hand_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = composite_hand_card(id);
    c.traits = vec!["Wicked God".to_string()];
    c
}

fn non_matching_hand_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = composite_hand_card(id);
    c.traits = vec!["Beast".to_string()];
    c
}

fn composite_field_digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = composite_hand_card(id);
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

fn non_composite_field_digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = composite_field_digimon(id);
    c.traits = vec!["Beast".to_string()];
    c
}

fn gazimon_hand_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Gazimon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn gizamon_hand_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Gizamon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn non_gazimon_hand_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Not Gazimon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn filler_deck_card(id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(id, id)
}

// ═══════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ex11_055_has_three_triggered_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-055 YAML loads")
        .memory(10)
        .start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX11-055 compiled card present");

    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        3,
        "EX11-055 ships exactly 3 triggered clauses: shared OP/SOMP, All Turns deletion, Security"
    );
}

#[test]
fn ex11_055_shared_clause_triggers_on_play_and_start_of_main_phase() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-055 YAML loads")
        .memory(10)
        .start();
    let card = runner.compiled_card(CARD_ID).unwrap();

    let clause = find_shared_clause(card).expect("shared OP/SOMP clause must exist");
    assert!(
        clause.when.contains(&CompiledTiming::OnPlay),
        "shared clause must trigger on_play"
    );
    assert!(
        clause.when.contains(&CompiledTiming::StartOfYourMainPhase),
        "shared clause must trigger start_of_your_main_phase"
    );
    assert_eq!(clause.scope, CompiledScope::FaceUp);
}

#[test]
fn ex11_055_deletion_observer_is_all_turns_optional_on_any_deletion() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-055 YAML loads")
        .memory(10)
        .start();
    let card = runner.compiled_card(CARD_ID).unwrap();

    let observer = find_deletion_observer(card).expect("[All Turns] deletion observer must exist");
    assert!(observer.when.contains(&CompiledTiming::OnAnyDeletion));
    assert_eq!(observer.scope, CompiledScope::FaceUp);
    assert!(
        observer.active_when.is_some(),
        "[All Turns] window must be encoded via active_when"
    );
    assert!(
        observer.optional,
        "the Gazimon/Gizamon play is a 'you may' — the clause must be optional"
    );
    assert!(
        !observer.once_per_turn,
        "printed text carries no [Once Per Turn]; the suspend cost is the natural limiter"
    );
    assert!(
        observer.condition.is_some(),
        "observer must gate on the deleted-object event context"
    );
}

#[test]
fn ex11_055_has_security_clause() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-055 YAML loads")
        .memory(10)
        .start();
    let card = runner.compiled_card(CARD_ID).unwrap();

    let has_security = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::OnSecurity),
        _ => false,
    });
    assert!(has_security, "EX11-055 must ship an on_security clause");
}

// ═══════════════════════════════════════════════════════════════════════
// Section 2 — Shared [On Play]/[Start of Your Main Phase]: condition gating
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ex11_055_on_play_offers_trash_prompt_when_eligible_card_in_hand() {
    let mut runner = shared_clause_runner();
    runner.game.stage_inject_card(0, "COMPOSITE-HAND", "hand").expect("inject COMPOSITE-HAND into hand");

    let idx = play_horaiji(&mut runner);
    let _ = idx;

    let view = runner
        .pending_selection_view()
        .expect("On Play should offer the trash-cost prompt when an eligible card is in hand");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "the trash-cost prompt must be declinable (PASS legal)"
    );
}

#[test]
fn ex11_055_on_play_no_prompt_when_no_eligible_card_in_hand() {
    let mut runner = shared_clause_runner();
    runner.game.stage_inject_card(0, "NON-MATCH-HAND", "hand").expect("inject NON-MATCH-HAND into hand");

    play_horaiji(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "with no [Composite]/[Wicked God] card in hand, no prompt should install"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 3 — Shared clause: behavioral outcome (On Play)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ex11_055_on_play_trashing_composite_draws_and_gains_memory() {
    let mut runner = shared_clause_runner();
    runner.game.stage_inject_card(0, "COMPOSITE-HAND", "hand").expect("inject COMPOSITE-HAND into hand");

    play_horaiji(&mut runner);

    let hand_before = runner.hand_size(0);
    let memory_before = runner.memory();
    let view = runner.pending_selection_view().expect("trash-cost prompt");
    let pick = *view
        .valid_action_ids
        .first()
        .expect("Composite hand card action available");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("trash the Composite hand card");
    runner.auto_resolve().expect("resolve draw + memory");

    // hand_before already excludes the trashed card's future removal:
    // hand loses the Composite card but gains 1 from the draw -> net 0.
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "trashing 1 card then drawing 1 nets to the same hand size"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "trashing a Composite/Wicked God card must gain 1 memory"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "COMPOSITE-HAND"),
        "the trashed card must land in the trash"
    );
}

#[test]
fn ex11_055_on_play_trashing_wicked_god_draws_and_gains_memory() {
    let mut runner = shared_clause_runner();
    runner.game.stage_inject_card(0, "WICKED-GOD-HAND", "hand").expect("inject WICKED-GOD-HAND into hand");

    play_horaiji(&mut runner);

    let memory_before = runner.memory();
    let view = runner.pending_selection_view().expect("trash-cost prompt");
    let pick = *view
        .valid_action_ids
        .first()
        .expect("Wicked God hand card action available");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("trash the Wicked God hand card");
    runner.auto_resolve().expect("resolve draw + memory");

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "trashing a Wicked God card must also gain 1 memory"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "WICKED-GOD-HAND"),
        "the trashed Wicked God card must land in the trash"
    );
}

#[test]
fn ex11_055_on_play_declining_the_trash_cost_does_nothing() {
    let mut runner = shared_clause_runner();
    runner.game.stage_inject_card(0, "COMPOSITE-HAND", "hand").expect("inject COMPOSITE-HAND into hand");

    play_horaiji(&mut runner);

    let hand_before = runner.hand_size(0);
    let memory_before = runner.memory();
    assert!(runner.pending_is_optional());

    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the trash-cost prompt");
    runner.auto_resolve().expect("resolve after decline");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining must not trash or draw"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "declining must not gain memory"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "COMPOSITE-HAND"),
        "declined card must remain out of the trash"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 4 — Shared clause: cost-firing event assertion
// ═══════════════════════════════════════════════════════════════════════

/// NOTE: `game_actions::trash_from_hand_by_index` does not currently emit a
/// `GameEvent::Trash` event — it moves the card directly (`player.trash.push`)
/// without calling the `Game::trash_card` event-emitting helper. This is a
/// pre-existing, already-documented engine gap (see BT24-008's identically
/// `#[ignore]`d `bt24_008_on_play_accept_emits_trash_event_for_cost_card`,
/// the closest sibling clause — "By trashing 1 [...] card from your hand,
/// Draw 2"). Marked pending until the engine emits Trash events for
/// hand-to-trash moves via `trash_from_hand_by_index`.
#[test]
#[ignore = "pending: engine-trash-event-from-hand — trash_from_hand_by_index does not emit GameEvent::Trash"]
fn ex11_055_on_play_trashing_hand_card_fires_trash_event() {
    let mut runner = shared_clause_runner();
    runner.game.stage_inject_card(0, "COMPOSITE-HAND", "hand").expect("inject COMPOSITE-HAND into hand");

    play_horaiji(&mut runner);

    let cp = runner.event_checkpoint();
    let view = runner.pending_selection_view().expect("trash-cost prompt");
    let pick = *view.valid_action_ids.first().expect("pick action");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("trash the Composite hand card");
    runner.auto_resolve().expect("resolve draw + memory");

    let trashes = runner.events_of_kind(cp, |event| {
        matches!(event, GameEvent::Trash { card_id, player, .. } if card_id == "COMPOSITE-HAND" && *player == 0)
    });
    assert_eq!(
        trashes.len(),
        1,
        "trashing the Composite hand card must emit exactly one Trash event"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 5 — Shared clause: [Start of Your Main Phase] integrated
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ex11_055_start_of_main_phase_offers_trash_prompt() {
    let mut runner = shared_clause_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.stage_inject_card(0, "COMPOSITE-HAND", "hand").expect("inject COMPOSITE-HAND into hand");
    // Memory starts below the (-10, 10) ceiling — placing this Tamer via
    // `place_on_field` bypasses cost payment (unlike the On Play path via
    // `play`), so memory must be lowered explicitly to make the +1 gain
    // observable rather than clamped at the printed rules maximum.
    runner.game.memory = 5;

    runner.game.enter_main_phase();
    runner.auto_resolve().expect("resolve the trash-cost reward");

    assert_eq!(
        runner.memory(),
        6,
        "Start of Your Main Phase must offer + auto-accept the trash-cost reward (memory 5 -> 6)"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "COMPOSITE-HAND"),
        "the trashed Composite card must land in the trash"
    );
}

#[test]
fn ex11_055_start_of_main_phase_no_prompt_without_eligible_card() {
    let mut runner = shared_clause_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.stage_inject_card(0, "NON-MATCH-HAND", "hand").expect("inject NON-MATCH-HAND into hand");

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        10,
        "no eligible card in hand -> Start of Your Main Phase gains no memory"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 6 — [All Turns] deletion observer: condition gating
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ex11_055_deletion_observer_fires_for_own_composite_digimon() {
    let mut runner = deletion_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-FIELD", Some(0));

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    let view = runner
        .pending_selection_view()
        .expect("own Composite Digimon deletion should offer Horaiji activation");
    // The clause's first body step is `activation_cost: { suspend_self: true }`
    // (a cost). A single cost-bearing OPTIONAL triggered effect surfaces as a
    // pre-cost `TriggerOrder` accept/decline prompt (PASS-able), not
    // `EffectChoice` — mirrors AD1-019 clause 2 / EX11-069's EoT clause.
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert!(view.is_optional, "the outer prompt must allow PASS (decline)");
}

#[test]
fn ex11_055_deletion_observer_fires_for_own_wicked_god_digimon() {
    let mut runner = deletion_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    let wicked_god = runner.place_on_field(0, "WICKED-GOD-FIELD", Some(0));

    runner
        .game
        .delete_permanent_with_cause(wicked_god, ReplacementCause::OwnEffect);

    let view = runner
        .pending_selection_view()
        .expect("own Wicked God Digimon deletion should offer Horaiji activation");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
}

#[test]
fn ex11_055_deletion_observer_does_not_fire_for_non_matching_own_digimon() {
    let mut runner = deletion_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    let non_matching = runner.place_on_field(0, "NON-COMPOSITE-FIELD", Some(0));

    runner
        .game
        .delete_permanent_with_cause(non_matching, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "deleting a non-[Composite]/[Wicked God] Digimon must not offer Horaiji activation"
    );
}

#[test]
fn ex11_055_deletion_observer_does_not_fire_for_opponent_composite_digimon() {
    let mut runner = deletion_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    let opp_composite = runner.place_on_field(1, "COMPOSITE-FIELD-OPP", Some(1));

    runner
        .game
        .delete_permanent_with_cause(opp_composite, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "printed 'your [Composite]/[Wicked God] Digimon' excludes the opponent's board"
    );
}

#[test]
fn ex11_055_deletion_observer_cannot_activate_when_already_suspended() {
    let mut runner = deletion_runner();
    let horaiji = runner.place_on_field(0, CARD_ID, Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-FIELD", Some(0));

    runner.game.players[0].battle_area[horaiji.index as usize].is_suspended = true;

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "a suspended Horaiji cannot pay the suspend cost, so no activation prompt appears"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 7 — [All Turns] deletion observer: behavioral outcome
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ex11_055_deletion_observer_accept_suspends_horaiji_and_plays_gazimon() {
    let mut runner = deletion_runner();
    let horaiji = runner.place_on_field(0, CARD_ID, Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-FIELD", Some(0));
    runner.game.stage_inject_card(0, "GAZIMON-HAND", "hand").expect("inject GAZIMON-HAND into hand");

    let cp = runner.event_checkpoint();

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    let choice = runner
        .pending_selection_view()
        .expect("deletion should offer the suspend-to-play-Gazimon choice");
    assert_eq!(choice.kind, SelectionKind::TriggerOrder);
    let accept = choice.valid_action_ids[0];
    runner
        .execute_action(choice.selecting_player, accept)
        .expect("accept Horaiji activation");

    let hand_pick = runner
        .pending_selection_view()
        .expect("accepting should offer the optional Gazimon/Gizamon hand pick");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    let play_action = *hand_pick
        .valid_action_ids
        .first()
        .expect("Gazimon hand-play action available");
    runner
        .execute_action(hand_pick.selecting_player, play_action)
        .expect("choose Gazimon from hand");
    runner.auto_resolve().expect("resolve the free Gazimon play");

    assert!(
        runner.game.players[0].battle_area[horaiji.index as usize].is_suspended,
        "accepting the activation must suspend Horaiji (the visible cost)"
    );
    assert!(
        runner
            .game
            .players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "GAZIMON-HAND"),
        "the selected Gazimon must enter the battle area"
    );

    let plays = runner.events_of_kind(cp, |event| {
        matches!(event, GameEvent::Play { card_id, player, cost_paid, .. } if card_id == "GAZIMON-HAND" && *player == 0 && *cost_paid == 0)
    });
    assert_eq!(
        plays.len(),
        1,
        "the free Gazimon play must emit exactly one Play event with cost_paid == 0"
    );
}

#[test]
fn ex11_055_deletion_observer_accept_allows_playing_gizamon() {
    let mut runner = deletion_runner();
    let horaiji = runner.place_on_field(0, CARD_ID, Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-FIELD", Some(0));
    runner.game.stage_inject_card(0, "GIZAMON-HAND", "hand").expect("inject GIZAMON-HAND into hand");

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    let choice = runner.pending_selection_view().expect("activation choice");
    let accept = choice.valid_action_ids[0];
    runner
        .execute_action(choice.selecting_player, accept)
        .expect("accept Horaiji activation");

    let hand_pick = runner
        .pending_selection_view()
        .expect("optional Gazimon/Gizamon hand pick");
    let play_action = *hand_pick
        .valid_action_ids
        .first()
        .expect("Gizamon hand-play action available");
    runner
        .execute_action(hand_pick.selecting_player, play_action)
        .expect("choose Gizamon from hand");
    runner.auto_resolve().expect("resolve the free Gizamon play");

    assert!(
        runner.game.players[0].battle_area[horaiji.index as usize].is_suspended,
        "Horaiji must still be suspended"
    );
    assert!(
        runner
            .game
            .players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "GIZAMON-HAND"),
        "the selected Gizamon must enter the battle area"
    );
}

#[test]
fn ex11_055_deletion_observer_gazimon_pick_excludes_non_matching_names() {
    let mut runner = deletion_runner();
    let _horaiji = runner.place_on_field(0, CARD_ID, Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-FIELD", Some(0));
    runner.game.stage_inject_card(0, "NOT-GAZIMON-HAND", "hand").expect("inject NOT-GAZIMON-HAND into hand");

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    let choice = runner.pending_selection_view().expect("activation choice");
    let accept = choice.valid_action_ids[0];
    runner
        .execute_action(choice.selecting_player, accept)
        .expect("accept Horaiji activation");

    // With no eligible Gazimon/Gizamon in hand, the printed body has nothing
    // to offer — the effect resolves cleanly with no further prompt.
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .players[0]
            .battle_area
            .iter()
            .all(|perm| perm.top_card().card_id(&runner.game.card_data) != "NOT-GAZIMON-HAND"),
        "a non-Gazimon/Gizamon hand card must never be played by this effect"
    );
}

#[test]
fn ex11_055_deletion_observer_declining_the_suspend_choice_does_nothing() {
    let mut runner = deletion_runner();
    let horaiji = runner.place_on_field(0, CARD_ID, Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-FIELD", Some(0));
    runner.game.stage_inject_card(0, "GAZIMON-HAND", "hand").expect("inject GAZIMON-HAND into hand");

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    let choice = runner.pending_selection_view().expect("activation choice");
    assert_eq!(choice.kind, SelectionKind::TriggerOrder);
    assert!(
        choice.is_optional,
        "the pre-cost prompt for this single cost-bearing optional trigger must allow PASS"
    );
    runner
        .execute_action(choice.selecting_player, digimon_engine::action::space::PASS)
        .expect("decline Horaiji activation");
    runner.auto_resolve().expect("resolve after decline");

    assert!(
        !runner.game.players[0].battle_area[horaiji.index as usize].is_suspended,
        "declining must leave Horaiji unsuspended"
    );
    assert!(
        runner
            .game
            .players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "GAZIMON-HAND"),
        "declining must leave the Gazimon candidate in hand"
    );
}

#[test]
fn ex11_055_deletion_observer_declining_gazimon_pick_leaves_it_in_hand() {
    let mut runner = deletion_runner();
    let horaiji = runner.place_on_field(0, CARD_ID, Some(0));
    let composite = runner.place_on_field(0, "COMPOSITE-FIELD", Some(0));
    runner.game.stage_inject_card(0, "GAZIMON-HAND", "hand").expect("inject GAZIMON-HAND into hand");

    runner
        .game
        .delete_permanent_with_cause(composite, ReplacementCause::OwnEffect);

    let choice = runner.pending_selection_view().expect("activation choice");
    let accept = choice.valid_action_ids[0];
    runner
        .execute_action(choice.selecting_player, accept)
        .expect("accept Horaiji activation (still pays the suspend cost)");

    let hand_pick = runner
        .pending_selection_view()
        .expect("optional Gazimon/Gizamon hand pick");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    runner
        .execute_action(hand_pick.selecting_player, digimon_engine::action::space::PASS)
        .expect("decline the optional Gazimon play");
    runner.auto_resolve().expect("resolve after declining the hand pick");

    assert!(
        runner.game.players[0].battle_area[horaiji.index as usize].is_suspended,
        "the suspend cost was already paid to reach the activation body — Horaiji stays suspended"
    );
    assert!(
        runner
            .game
            .players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "GAZIMON-HAND"),
        "declining the optional hand pick must leave the Gazimon candidate in hand"
    );
    assert!(
        runner
            .game
            .players[0]
            .battle_area
            .iter()
            .all(|perm| perm.top_card().card_id(&runner.game.card_data) != "GAZIMON-HAND"),
        "no Gazimon enters the battle area when the optional play is declined"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 8 — [Security] play-self-free
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ex11_055_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER-EX11-055", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(9000);
    attacker.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-055 YAML loads")
        .add_card(attacker)
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER-EX11-055", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "EX11-055 must play itself onto the field via its Security effect"
    );
}

// ─── Helpers ──────────────────────────────────────────────────────────────

fn shared_clause_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-055 YAML loads")
        .add_card(composite_hand_card("COMPOSITE-HAND"))
        .add_card(wicked_god_hand_card("WICKED-GOD-HAND"))
        .add_card(non_matching_hand_card("NON-MATCH-HAND"))
        .add_card(filler_deck_card("FILLER-DECK-EX11-055"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER-DECK-EX11-055"; 5])
        .deck(1, &["FILLER-DECK-EX11-055"; 5])
        .memory(10)
        .start()
}

fn deletion_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-055 YAML loads")
        .add_card(composite_field_digimon("COMPOSITE-FIELD"))
        .add_card(wicked_god_hand_card_as_field("WICKED-GOD-FIELD"))
        .add_card(non_composite_field_digimon("NON-COMPOSITE-FIELD"))
        .add_card(composite_field_digimon("COMPOSITE-FIELD-OPP"))
        .add_card(gazimon_hand_card("GAZIMON-HAND"))
        .add_card(gizamon_hand_card("GIZAMON-HAND"))
        .add_card(non_gazimon_hand_card("NOT-GAZIMON-HAND"))
        .add_card(filler_deck_card("FILLER-DECK-EX11-055"))
        .deck(0, &["FILLER-DECK-EX11-055"; 5])
        .deck(1, &["FILLER-DECK-EX11-055"; 5])
        .memory(10)
        .start()
}

fn wicked_god_hand_card_as_field(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = composite_field_digimon(id);
    c.traits = vec!["Wicked God".to_string()];
    c
}

/// Play EX11-055 from hand (full On Play path — cost payment + OnPlay
/// observers). The runner returns the new battle-area index.
fn play_horaiji(runner: &mut DebugRunner) -> usize {
    runner
        .play(0, 0)
        .expect("EX11-055 must be at hand index 0 and playable")
}

fn find_shared_clause(
    card: &digimon_dsl::compiled::CompiledCard,
) -> Option<&CompiledTriggeredClause> {
    card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnPlay)
                && triggered
                    .when
                    .contains(&CompiledTiming::StartOfYourMainPhase) =>
        {
            Some(triggered)
        }
        _ => None,
    })
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
