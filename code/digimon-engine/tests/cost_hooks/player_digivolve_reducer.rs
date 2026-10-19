//! Substrate tests for the player-scoped one-shot future-digivolve cost
//! reducer (`G-COST-REDUCE-ALLY-DIGIVOLVE`, also `G-COST-REDUCE-NEXT-SINGLE-FIRE`
//! and `G-PAY-COST-SELECT-ARBITRARY-SUSPEND`).
//!
//! These tests exercise the closure-bearing player-scoped reducer store
//! directly via a fixture `CardEffect` whose `[Main]`-style `on_play` body
//! calls `EffectContext::arm_player_digivolve_cost_reducer(...)`. They
//! cover the lifecycle independently of the BT3-103 YAML card:
//!
//!   install → fires at next qualifying digivolve → prompts suspend cost →
//!   accept applies the reduction once → decline keeps the full cost and
//!   leaves the reducer armed → single-fire consumed after one use →
//!   target-color filter (a non-green digivolution never sees the prompt).

use digimon_engine::action::space::{encode_attack, HAND_EFFECT_START, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::selection::SelectionKind;

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A Lv.3 base Digimon of `color`, suitable as a digivolve base.
fn lvl3_base(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.colors = vec![color];
    c
}

/// A Lv.4 Digimon with a printed `Lv.3 <color>, cost N` evo cost.
fn lvl4_evo3(id: &str, color: CardColor, evo_color: u8, memory_cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![color];
    c.evo_costs = vec![EvoCost {
        card_color: evo_color,
        level: 3,
        memory_cost,
    }];
    c
}

/// Card whose `on_play` body arms a player-scoped green-Digimon digivolve
/// cost reducer: -5, single-fire, suspend cost.
struct ArmsGreenDigivolveReducer;
impl CardEffect for ArmsGreenDigivolveReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("arm green digivolve cost reducer")
            .process(|ctx| {
                ctx.arm_player_digivolve_cost_reducer(5, true, Some(CardColor::Green), true);
            })
            .build()]
    }
}

/// Variant: no suspend cost (reduction applies on accept with no cost).
struct ArmsGreenDigivolveReducerNoCost;
impl CardEffect for ArmsGreenDigivolveReducerNoCost {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("arm green digivolve cost reducer (no cost)")
            .process(|ctx| {
                ctx.arm_player_digivolve_cost_reducer(5, true, Some(CardColor::Green), false);
            })
            .build()]
    }
}

// Evo-color byte: Green == 3 (matches `card_data::parse_card_color`).
const EVO_GREEN: u8 = 3;
const EVO_RED: u8 = 0;

/// A synchronous payment that moves an earlier zone entry. The outer optional
/// gate still parks the shared reducer chain and must retain the declared cards.
struct MovingZoneReducer {
    trash_hand: bool,
}
impl CardEffect for MovingZoneReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let trash_hand = self.trash_hand;
        vec![Effect::before_pay_cost(card)
            .name("move earlier zone entry for digivolution -1")
            .condition(|ctx| ctx.cost_is_digivolve)
            .optional()
            .cost_reduction_fn(|ctx| if ctx.cost_is_digivolve { 1 } else { 0 })
            .pay_cost_fn(move |ctx| {
                if trash_hand {
                    ctx.trash_from_hand_by_index(0, 0).is_some()
                } else {
                    ctx.trash_self_as_cost()
                }
            })
            .build()]
    }
}

fn moving_zone_reducer_runner(trash_hand: bool) -> DebugRunner {
    let mut field = make_test_card("MOVING-FIELD", "Moving field");
    field.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("ARM", "Arm"))
        .add_card(make_test_card("FODDER", "Fodder"))
        .add_card(field)
        .add_card(lvl3_base("GBASE", CardColor::Green))
        .add_card(lvl4_evo3("GEVO", CardColor::Green, EVO_GREEN, 4))
        .add_card(lvl4_evo3("GEVO2", CardColor::Green, EVO_GREEN, 4))
        .hand(0, &["ARM", "FODDER", "GEVO", "GEVO2"])
        .deck(0, &["GBASE"; 8])
        .memory(10)
        .start();
    r.register_effect("ARM", std::sync::Arc::new(ArmsGreenDigivolveReducerNoCost));
    r.register_effect(
        "MOVING-FIELD",
        std::sync::Arc::new(MovingZoneReducer { trash_hand }),
    );
    r.place_on_field(0, "MOVING-FIELD", Some(0));
    r.place_on_field(0, "GBASE", Some(0));
    r.place_on_field(0, "GBASE", Some(0));
    r.play(0, 0);
    r.game.player_digivolve_cost_reducers[0].amount = 1;
    r
}

#[test]
fn reducer_payment_hand_compaction_preserves_declared_evolution_card() {
    use digimon_engine::action::space::encode_digivolve;
    let mut r = moving_zone_reducer_runner(true);
    let intended = r.game.players[0].hand[1].handle();
    let other = r.game.players[0].hand[2].handle();
    let before = r.memory();
    let action = encode_digivolve(1, 1);
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[action as usize] > 0.0);
    r.game.decode_action(action, 0);
    assert!(r.game.pending_selection.as_ref().unwrap().is_optional);
    assert!(
        digimon_engine::action::mask::build_action_mask(&r.game, 0)[HAND_EFFECT_START as usize]
            > 0.0
    );
    r.execute_action(0, HAND_EFFECT_START).unwrap();
    assert_eq!(
        r.game.players[0].battle_area[1].top_card().handle(),
        intended,
        "trashing hand index 0 must not substitute the card now at old index 1"
    );
    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|card| card.handle() == other));
    assert_eq!(r.memory(), before - 2);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn reducer_payment_field_compaction_preserves_declared_evolution_host() {
    use digimon_engine::action::space::encode_digivolve;
    let mut r = moving_zone_reducer_runner(false);
    let intended_card = r.game.players[0].hand[1].handle();
    let intended_host = r.game.players[0].battle_area[1].top_card().handle();
    let other_host = r.game.players[0].battle_area[2].top_card().handle();
    let before = r.memory();
    let action = encode_digivolve(1, 1);
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[action as usize] > 0.0);
    r.game.decode_action(action, 0);
    assert!(r.game.pending_selection.as_ref().unwrap().is_optional);
    assert!(
        digimon_engine::action::mask::build_action_mask(&r.game, 0)[HAND_EFFECT_START as usize]
            > 0.0
    );
    r.execute_action(0, HAND_EFFECT_START).unwrap();
    assert_eq!(
        r.game.players[0].battle_area[0].top_card().handle(),
        intended_card,
        "deleting the preceding Tamer must preserve the originally chosen host"
    );
    assert!(r.game.players[0].battle_area[0]
        .card_sources
        .iter()
        .any(|card| card.handle() == intended_host));
    assert_eq!(
        r.game.players[0].battle_area[1].top_card().handle(),
        other_host
    );
    assert_eq!(r.memory(), before - 2);
    assert!(r.game.pending_selection.is_none());
}

/// Build a runner with the reducer-arming card in hand, a green Lv.3 base on
/// the field, and a green Lv.4 evo card in hand that digivolves onto it
/// (printed digivolution cost 4).
fn green_digivolve_runner(arm: std::sync::Arc<dyn CardEffect>) -> (DebugRunner, usize) {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("ARM", "Arm"))
        .add_card(lvl3_base("GBASE", CardColor::Green))
        .add_card(lvl4_evo3("GEVO", CardColor::Green, EVO_GREEN, 4))
        .add_card(lvl4_evo3("GEVO2", CardColor::Green, EVO_GREEN, 4))
        .hand(0, &["ARM", "GEVO", "GEVO2"])
        .deck(0, &["GBASE"; 5])
        .memory(10)
        .start();
    r.register_effect("ARM", arm);
    let base = r.place_on_field(0, "GBASE", Some(0));
    (r, base.index as usize)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

/// Installing the reducer (playing the arming card) records a player-scoped
/// reducer in `Game::player_digivolve_cost_reducers`.
#[test]
fn install_records_player_scoped_reducer() {
    let (mut r, _base) = green_digivolve_runner(std::sync::Arc::new(ArmsGreenDigivolveReducer));
    assert!(r.game.player_digivolve_cost_reducers.is_empty());
    r.play(0, 0); // play ARM
    assert_eq!(
        r.game.player_digivolve_cost_reducers.len(),
        1,
        "playing the arming card must install exactly one player-scoped reducer"
    );
    let reducer = &r.game.player_digivolve_cost_reducers[0];
    assert_eq!(reducer.player, 0);
    assert_eq!(reducer.amount, 5);
    assert!(reducer.single_fire);
    assert!(reducer.suspend_cost);
}

/// A qualifying green digivolution prompts an accept/decline EffectChoice
/// selection BEFORE the cost is paid.
#[test]
fn green_digivolve_prompts_accept_decline() {
    let (mut r, base) = green_digivolve_runner(std::sync::Arc::new(ArmsGreenDigivolveReducer));
    r.play(0, 0); // play ARM
    let memory_before = r.memory();

    // GEVO is now hand index 0 (ARM was removed).
    let ok = r
        .game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    assert!(
        !ok,
        "digivolve must return false while the reducer prompt is pending"
    );
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("accept/decline prompt must be installed");
    assert!(matches!(pending.kind, SelectionKind::EffectChoice));
    assert!(pending.is_optional, "the reducer prompt must be declinable");
    // No memory has been spent yet — the prompt fires before cost payment.
    assert_eq!(r.memory(), memory_before);
}

/// Accepting the prompt installs a suspend-cost selection; choosing a
/// Digimon suspends it, applies the -5 reduction, and completes the
/// digivolution at the reduced cost.
#[test]
fn accept_suspends_and_applies_reduction_once() {
    let (mut r, base) = green_digivolve_runner(std::sync::Arc::new(ArmsGreenDigivolveReducer));
    r.play(0, 0); // play ARM
    let memory_before = r.memory();

    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    // Accept the reducer prompt.
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("accept resolves");

    // A suspend-cost selection must now be pending (OwnField).
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("suspend-cost selection installed after accept");
    assert!(matches!(pending.kind, SelectionKind::OwnField));
    assert!(
        !pending.is_optional,
        "the suspend cost itself is mandatory once the player accepted"
    );

    // Suspend the only available Digimon (the green base at `base`).
    let suspend_action = encode_attack(0, base as u16);
    r.game
        .resolve_selection(0, suspend_action)
        .expect("suspend selection resolves");

    assert!(r.game.pending_selection.is_none(), "digivolution completes");
    // printed digivolution cost 4, reduced by 5 → clamped to 0.
    assert_eq!(
        r.memory(),
        memory_before,
        "reduced digivolution cost is 0 (4 - 5, clamped)"
    );
    // The single-fire reducer is consumed.
    assert!(
        r.game.player_digivolve_cost_reducers.is_empty(),
        "single-fire reducer consumed after a successful application"
    );
}

/// Declining the prompt keeps the full digivolution cost and leaves the
/// reducer armed (a declined prompt does NOT consume a single-fire reducer).
#[test]
fn decline_keeps_full_cost_and_consumes_next_reducer() {
    let (mut r, base) = green_digivolve_runner(std::sync::Arc::new(ArmsGreenDigivolveReducer));
    r.play(0, 0); // play ARM
    let memory_before = r.memory();

    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    // Decline the reducer prompt.
    r.game
        .resolve_selection(0, PASS)
        .expect("decline (PASS) resolves");

    assert!(
        r.game.pending_selection.is_none(),
        "digivolution completes after decline"
    );
    // Full printed digivolution cost 4 was paid.
    assert_eq!(
        r.memory(),
        memory_before - 4,
        "declining keeps the full digivolution cost"
    );
    // The reducer is NOT consumed — a declined prompt leaves it armed.
    assert!(
        r.game.player_digivolve_cost_reducers.is_empty(),
        "the next qualifying digivolution consumes a declined one-shot"
    );
}

/// After a successful application, a SECOND green digivolution in the same
/// turn does not see the prompt — the single-fire reducer was consumed.
#[test]
fn single_fire_consumed_after_one_application() {
    // Two independent green Lv.3 bases so the second digivolution has a
    // fresh, legal target after the first digivolution mutates `base`.
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("ARM", "Arm"))
        .add_card(lvl3_base("GBASE", CardColor::Green))
        .add_card(lvl4_evo3("GEVO", CardColor::Green, EVO_GREEN, 4))
        .add_card(lvl4_evo3("GEVO2", CardColor::Green, EVO_GREEN, 4))
        .hand(0, &["ARM", "GEVO", "GEVO2"])
        .deck(0, &["GBASE"; 5])
        .memory(20)
        .start();
    r.register_effect("ARM", std::sync::Arc::new(ArmsGreenDigivolveReducer));
    let base_a = r.place_on_field(0, "GBASE", Some(0)).index as usize;
    let base_b = r.place_on_field(0, "GBASE", Some(0)).index as usize;

    r.play(0, 0); // play ARM

    // First digivolution onto base_a: accept + suspend base_b.
    r.game
        .digivolve_from_hand(0, 0, base_a, PlaySource::ByDigivolve);
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("accept");
    r.game
        .resolve_selection(0, encode_attack(0, base_b as u16))
        .expect("suspend");
    assert!(
        r.game.player_digivolve_cost_reducers.is_empty(),
        "single-fire reducer consumed after the first application"
    );

    let memory_before = r.memory();
    // Second green digivolution (GEVO2, now hand index 0) onto base_b — no prompt.
    let ok = r
        .game
        .digivolve_from_hand(0, 0, base_b, PlaySource::ByDigivolve);
    assert!(ok, "second digivolution succeeds outright (no prompt)");
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        r.memory(),
        memory_before - 4,
        "second digivolution pays the full cost — reducer was consumed"
    );
}

/// A non-green digivolution never sees the prompt (target-color filter).
#[test]
fn non_green_digivolve_never_prompts() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("ARM", "Arm"))
        .add_card(lvl3_base("RBASE", CardColor::Red))
        .add_card(lvl4_evo3("REVO", CardColor::Red, EVO_RED, 4))
        .hand(0, &["ARM", "REVO"])
        .deck(0, &["RBASE"; 5])
        .memory(10)
        .start();
    r.register_effect("ARM", std::sync::Arc::new(ArmsGreenDigivolveReducer));
    let base = r.place_on_field(0, "RBASE", Some(0));

    r.play(0, 0); // play ARM — arms the GREEN-only reducer
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);

    let memory_before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "non-green digivolution succeeds outright");
    assert!(
        r.game.pending_selection.is_none(),
        "a non-green digivolution must NOT install the reducer prompt"
    );
    assert_eq!(
        r.memory(),
        memory_before - 4,
        "non-green digivolution pays the full cost"
    );
    // The green-only reducer is untouched — still armed.
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
}

/// The "For the turn" expiry: the reducer is dropped when the installing
/// player's turn ends.
#[test]
fn reducer_expires_at_end_of_turn() {
    let (mut r, _base) = green_digivolve_runner(std::sync::Arc::new(ArmsGreenDigivolveReducer));
    r.play(0, 0); // play ARM
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
    r.end_turn();
    assert!(
        r.game.player_digivolve_cost_reducers.is_empty(),
        "a \"For the turn\" reducer is dropped at end of the installing player's turn"
    );
}

/// A FREE reducer (no suspend cost) carries no player choice: the reduction
/// auto-applies synchronously with NO accept/decline prompt (DCGO ST12_15.cs
/// installs an unconditional `Cost -= 1`; parking an optional prompt would
/// over-expose an illegal PASS — G-DELAY-NEXT-DIGIVOLVE-COST-REDUCTION).
#[test]
fn no_suspend_cost_auto_applies_without_prompt() {
    let (mut r, base) =
        green_digivolve_runner(std::sync::Arc::new(ArmsGreenDigivolveReducerNoCost));
    r.play(0, 0); // play ARM
    let memory_before = r.memory();

    let ok = r
        .game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    assert!(ok, "free-reducer digivolution completes synchronously");
    assert!(
        r.game.pending_selection.is_none(),
        "a free (no-suspend-cost) reducer must NOT park an accept/decline prompt"
    );
    assert_eq!(
        r.memory(),
        memory_before,
        "reduced digivolution cost is 0 (4 - 5, clamped)"
    );
    assert!(
        r.game.player_digivolve_cost_reducers.is_empty(),
        "single-fire reducer consumed by the auto-application"
    );
}

fn stacked_reducer_runner(paid: bool) -> (DebugRunner, usize) {
    let (mut r, base) = green_digivolve_runner(std::sync::Arc::new(ArmsGreenDigivolveReducer));
    r.play(0, 0);
    r.game.player_digivolve_cost_reducers[0].amount = 1;
    r.game.player_digivolve_cost_reducers[0].suspend_cost = paid;
    let mut second = r.game.player_digivolve_cost_reducers[0].clone();
    second.amount = if paid { 5 } else { 2 };
    r.game.player_digivolve_cost_reducers.push(second);
    (r, base)
}

#[test]
fn stacked_free_reducers_sum_on_same_next_evolution() {
    let (mut r, base) = stacked_reducer_runner(false);
    let before = r.memory();
    assert!(r
        .game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve));
    assert_eq!(
        r.memory(),
        before - 1,
        "both free reductions must apply: 4 - 1 - 2"
    );
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.player_digivolve_cost_reducers.is_empty());
}

#[test]
fn stacked_paid_reducers_offer_order_and_clone_payment_choices() {
    let (mut r, base) = stacked_reducer_runner(true);
    for (index, permanent) in r.game.players[0].battle_area.iter_mut().enumerate() {
        permanent.is_suspended = index != base;
    }
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    let order = r
        .game
        .pending_selection
        .as_ref()
        .expect("choose reducer order");
    assert!(
        !order.is_optional,
        "ordering is mandatory; individual payments remain optional"
    );
    assert_eq!(
        order.valid_action_ids,
        vec![HAND_EFFECT_START, HAND_EFFECT_START + 1]
    );
    let mask = digimon_engine::action::mask::build_action_mask(&r.game, 0);
    assert_eq!(mask[HAND_EFFECT_START as usize], 1.0);
    assert_eq!(mask[(HAND_EFFECT_START + 1) as usize], 1.0);
    assert_eq!(mask[PASS as usize], 0.0);
    let mut choose_high = r.game.clone();
    choose_high
        .resolve_selection(0, HAND_EFFECT_START + 1)
        .unwrap();
    assert!(choose_high.pending_selection.as_ref().unwrap().is_optional);
    choose_high.resolve_selection(0, HAND_EFFECT_START).unwrap();
    let mut at_payment = choose_high.clone();
    at_payment
        .resolve_selection(0, encode_attack(0, base as u16))
        .unwrap();
    assert_eq!(
        at_payment.memory, before,
        "choose -5 while only one Digimon can suspend"
    );
    assert!(at_payment.player_digivolve_cost_reducers.is_empty());
    assert!(at_payment.pending_selection.is_none());
    assert!(
        choose_high.pending_selection.is_some(),
        "payment clone stays independent"
    );
    let mut choose_low = r.game.clone();
    choose_low.resolve_selection(0, HAND_EFFECT_START).unwrap();
    choose_low.resolve_selection(0, HAND_EFFECT_START).unwrap();
    choose_low
        .resolve_selection(0, encode_attack(0, base as u16))
        .unwrap();
    assert_eq!(
        choose_low.memory,
        before - 3,
        "choosing -1 first is a distinct legal choice"
    );
    assert!(choose_low.player_digivolve_cost_reducers.is_empty());
    assert!(
        r.game.pending_selection.is_some(),
        "ordering clone stays independent"
    );
}

#[test]
fn prohibited_reductions_expire_on_next_qualifying_evolution() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut r, base) = stacked_reducer_runner(false);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotReduceDigivolveCost,
            0,
            Expiry::Permanent,
            None,
            0,
        ),
    );
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    assert_eq!(r.memory(), before - 4);
    assert!(r.game.pending_selection.is_none());
    assert!(
        r.game.player_digivolve_cost_reducers.is_empty(),
        "prohibition blocks reduction, not the next-event lifetime"
    );
}

#[test]
fn reducer_mask_queries_and_rejected_routes_preserve_armed_effects() {
    let (mut r, base) = stacked_reducer_runner(false);
    let before = r.memory();
    for _ in 0..3 {
        let _ = digimon_engine::action::mask::build_action_mask(&r.game, 0);
    }
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 2);
    assert!(!r
        .game
        .digivolve_from_hand(0, 99, base, PlaySource::ByDigivolve));
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 2);
    assert_eq!(r.memory(), before);
}

#[test]
fn cloned_decline_consumes_only_clone_next_reducer() {
    let (mut r, base) = green_digivolve_runner(std::sync::Arc::new(ArmsGreenDigivolveReducer));
    r.play(0, 0);
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    let mut clone = r.game.clone();
    clone.resolve_selection(0, PASS).unwrap();
    assert_eq!(clone.memory, before - 4);
    assert!(clone.player_digivolve_cost_reducers.is_empty());
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
    assert_eq!(r.memory(), before);
    assert!(r.game.pending_selection.is_some());
}

#[test]
fn declining_paid_reducer_keeps_free_reduction_on_same_evolution() {
    let (mut r, base) = stacked_reducer_runner(false);
    r.game.player_digivolve_cost_reducers[0].suspend_cost = true;
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    assert!(r.game.pending_selection.as_ref().unwrap().is_optional);
    r.game.resolve_selection(0, PASS).unwrap();
    assert_eq!(r.memory(), before - 2, "declining paid -1 retains free -2");
    assert!(r.game.player_digivolve_cost_reducers.is_empty());
}

#[test]
fn reusable_reducer_applies_once_per_evolution_and_stays_armed() {
    let (mut r, base) = stacked_reducer_runner(false);
    r.game.player_digivolve_cost_reducers[0].single_fire = false;
    let second_base = r.place_on_field(0, "GBASE", Some(0)).index as usize;
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    assert_eq!(r.memory(), before - 1);
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
    assert!(!r.game.player_digivolve_cost_reducers[0].single_fire);
    let before_second = r.memory();
    r.game
        .digivolve_from_hand(0, 0, second_base, PlaySource::ByDigivolve);
    assert_eq!(r.memory(), before_second - 3);
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
}

/// A field-hosted optional, synchronous cost payment on the same evolution.
struct FieldTrashForDigivolveReduction;
impl CardEffect for FieldTrashForDigivolveReduction {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("field trash one for digivolution -2")
            .condition(|ctx| ctx.cost_is_digivolve)
            .optional()
            .cost_reduction_fn(|ctx| if ctx.cost_is_digivolve { 2 } else { 0 })
            .pay_cost_fn(|ctx| {
                ctx.trash_from_top(0, 1);
                true
            })
            .build()]
    }
}

fn mixed_reducer_runner(paid: bool, field_on_base: bool) -> (DebugRunner, usize) {
    let mut field = make_test_card("FIELD", "Field reducer");
    field.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("ARM", "Arm"))
        .add_card(field)
        .add_card(lvl3_base("GBASE", CardColor::Green))
        .add_card(lvl4_evo3("GEVO", CardColor::Green, EVO_GREEN, 4))
        .hand(0, &["ARM", "GEVO"])
        .deck(0, &["GBASE"; 8])
        .memory(10)
        .start();
    // Register every definition before placing anything on the field: effect
    // definitions are immutable once the stored-card cache is populated.
    r.register_effect(
        "ARM",
        if paid {
            std::sync::Arc::new(ArmsGreenDigivolveReducer)
        } else {
            std::sync::Arc::new(ArmsGreenDigivolveReducerNoCost)
        },
    );
    if field_on_base {
        r.register_effect(
            "GBASE",
            std::sync::Arc::new(FieldSuspendForDigivolveReduction),
        );
    } else {
        r.register_effect(
            "FIELD",
            std::sync::Arc::new(FieldTrashForDigivolveReduction),
        );
        r.place_on_field(0, "FIELD", Some(0));
    }
    let base = r.place_on_field(0, "GBASE", Some(0)).index as usize;
    r.play(0, 0);
    (r, base)
}

#[test]
fn paid_player_reducer_does_not_skip_field_payment_prompt() {
    let (mut r, base) = mixed_reducer_runner(true, false);
    r.game.player_digivolve_cost_reducers[0].amount = 1;
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    if !r.game.pending_selection.as_ref().unwrap().is_optional {
        r.game.resolve_selection(0, HAND_EFFECT_START).unwrap(); // choose player reducer first
    }
    r.game.resolve_selection(0, HAND_EFFECT_START).unwrap();
    r.game
        .resolve_selection(0, encode_attack(0, base as u16))
        .unwrap();
    let field = r
        .game
        .pending_selection
        .as_ref()
        .expect("field payment must still be offered");
    assert!(field.prompt.contains("field trash one"));
    assert_eq!(
        r.memory(),
        before,
        "both reducer classes resolve before payment"
    );
    let mut clone = r.game.clone();
    clone.resolve_selection(0, HAND_EFFECT_START).unwrap();
    assert_eq!(clone.memory, before - 1, "4 - player 1 - field 2");
    assert!(clone.pending_selection.is_none());
    r.game.resolve_selection(0, PASS).unwrap();
    assert_eq!(
        r.memory(),
        before - 3,
        "field decline preserves paid player reduction"
    );
}

#[test]
fn free_player_reduction_survives_field_payment_park() {
    let (mut r, base) = mixed_reducer_runner(false, false);
    r.game.player_digivolve_cost_reducers[0].amount = 1;
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    assert!(r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .prompt
        .contains("field trash one"));
    r.game.resolve_selection(0, HAND_EFFECT_START).unwrap();
    assert_eq!(
        r.memory(),
        before - 1,
        "free player reduction survives field payment continuation"
    );
}

struct FieldSuspendForDigivolveReduction;
impl CardEffect for FieldSuspendForDigivolveReduction {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let mut effect = Effect::before_pay_cost(card)
            .name("field suspend self for digivolution -1")
            .condition(|ctx| ctx.cost_is_digivolve)
            .optional()
            .cost_reduction_fn(|ctx| if ctx.cost_is_digivolve { 1 } else { 0 })
            .pay_cost_fn(|ctx| ctx.suspend_self_as_cost())
            .build();
        effect.activation_cost_kind = Some(digimon_engine::effect::ActivationCostKind::SuspendSelf);
        vec![effect]
    }
}

#[test]
fn competing_field_and_player_payments_expose_both_orders() {
    let (mut r, base) = mixed_reducer_runner(true, true);
    for (index, permanent) in r.game.players[0].battle_area.iter_mut().enumerate() {
        permanent.is_suspended = index != base;
    }
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    let order = r
        .game
        .pending_selection
        .as_ref()
        .expect("cross-class order choice");
    assert!(!order.is_optional);
    let choices = order.effect_choices.as_ref().unwrap();
    assert_eq!(choices.len(), 2);
    let field_action = choices
        .iter()
        .find(|entry| entry.label.contains("field suspend"))
        .unwrap()
        .action_id;
    let player_action = choices
        .iter()
        .find(|entry| entry.action_id != field_action)
        .unwrap()
        .action_id;
    let mut field_first = r.game.clone();
    field_first.resolve_selection(0, field_action).unwrap();
    field_first.resolve_selection(0, HAND_EFFECT_START).unwrap();
    assert!(
        field_first.pending_selection.is_none(),
        "field suspension makes HPD payment impossible"
    );
    assert_eq!(field_first.memory, before - 3);
    assert!(field_first.player_digivolve_cost_reducers.is_empty());
    let mut player_first = r.game.clone();
    player_first.resolve_selection(0, player_action).unwrap();
    player_first
        .resolve_selection(0, HAND_EFFECT_START)
        .unwrap();
    player_first
        .resolve_selection(0, encode_attack(0, base as u16))
        .unwrap();
    assert!(
        player_first.pending_selection.is_none(),
        "HPD suspension makes field payment impossible"
    );
    assert_eq!(player_first.memory, before);
    assert!(player_first.player_digivolve_cost_reducers.is_empty());
}

const SELF_GATED_FIELD_REDUCER: &str = r#"
card: TEST-SELF-GATED-EVO
name: Optional Suspender
kind: tamer
color: [green]
cost: 3
effects:
  - kind: cost_reduction
    active_when: { your_turn: true }
    when_any_ally_digivolves_into: { kind: digimon }
    optional: true
    amount: 2
    pay_cost:
      - select_own_permanent:
          bind_as: payment
          filter: { kind: digimon }
          optional: true
          prompt: "You may suspend a Digimon for the field reduction"
      - suspend: { target: payment }
"#;

fn self_gated_field_runner() -> (DebugRunner, usize) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SELF_GATED_FIELD_REDUCER)
        .expect("self-gated field reducer compiles")
        .add_card(make_test_card("ARM", "Arm"))
        .add_card(lvl3_base("GBASE", CardColor::Green))
        .add_card(lvl4_evo3("GEVO", CardColor::Green, EVO_GREEN, 4))
        .hand(0, &["ARM", "GEVO"])
        .deck(0, &["GBASE"; 8])
        .memory(10)
        .start();
    r.register_effect("ARM", std::sync::Arc::new(ArmsGreenDigivolveReducerNoCost));
    r.place_on_field(0, "TEST-SELF-GATED-EVO", Some(0));
    let base = r.place_on_field(0, "GBASE", Some(0)).index as usize;
    r.play(0, 0);
    r.game.player_digivolve_cost_reducers[0].amount = 1;
    (r, base)
}

#[test]
fn cloned_self_gated_field_payment_decline_gives_no_unpaid_reduction() {
    let (mut r, base) = self_gated_field_runner();
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    let payment = r
        .game
        .pending_selection
        .as_ref()
        .expect("self-gated inner payment");
    assert!(payment.is_optional);
    assert!(matches!(payment.kind, SelectionKind::OwnField));
    assert!(
        r.game.pending_selection_resume.is_some(),
        "exercise data-hook path"
    );
    let mut accepted = r.game.clone();
    accepted
        .resolve_selection(0, encode_attack(0, base as u16))
        .unwrap();
    assert_eq!(
        accepted.memory,
        before - 1,
        "paid both reductions: 4 - 1 - 2"
    );
    assert!(accepted.players[0].battle_area[base].is_suspended);
    let mut declined = r.game.clone();
    declined.resolve_selection(0, PASS).unwrap();
    assert_eq!(
        declined.memory,
        before - 3,
        "declining field cost retains only free -1"
    );
    assert!(!declined.players[0].battle_area[base].is_suspended);
    assert!(declined.after_selection_resume_hooks.0.is_empty());
    assert!(declined.pending_selection.is_none());
    assert!(
        r.game.pending_selection.is_some(),
        "clones cannot resolve original payment"
    );
}

#[test]
fn declining_player_gate_does_not_cancel_next_field_payment_hook() {
    let (mut r, base) = self_gated_field_runner();
    r.game.player_digivolve_cost_reducers[0].suspend_cost = true;
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, 0, base, PlaySource::ByDigivolve);
    assert!(!r.game.pending_selection.as_ref().unwrap().is_optional);
    r.game.resolve_selection(0, HAND_EFFECT_START).unwrap(); // player reducer first
    r.game.resolve_selection(0, PASS).unwrap(); // decline only that payment
    let field = r
        .game
        .pending_selection
        .as_ref()
        .expect("next field payment");
    assert!(matches!(field.kind, SelectionKind::OwnField));
    assert!(field.is_optional);
    let mut accepted = r.game.clone();
    accepted
        .resolve_selection(0, encode_attack(0, base as u16))
        .unwrap();
    assert_eq!(
        accepted.memory,
        before - 2,
        "earlier PASS cannot cancel a later paid -2"
    );
    assert!(accepted.after_selection_resume_hooks.0.is_empty());
    r.game.resolve_selection(0, PASS).unwrap();
    assert_eq!(
        r.memory(),
        before - 4,
        "declining both payments pays full cost"
    );
    assert!(r.game.after_selection_resume_hooks.0.is_empty());
}

#[derive(Clone, Copy)]
enum IdentityPayment {
    RemoveDeclaredCard,
    RemoveHost,
    ReplayHost,
    SuspendDeclaredHost,
}
struct IdentityPaymentReducer(IdentityPayment);
impl CardEffect for IdentityPaymentReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let mode = self.0;
        vec![Effect::before_pay_cost(card)
            .name("identity-sensitive evolution payment")
            .condition(move |ctx| {
                ctx.cost_is_digivolve
                    && (matches!(mode, IdentityPayment::RemoveDeclaredCard)
                        || ctx.source_is_cost_target_permanent())
            })
            .optional()
            .cost_reduction(1)
            .pay_cost_fn(move |ctx| match mode {
                IdentityPayment::RemoveDeclaredCard => {
                    let Some(target) = ctx.cost_target_card else {
                        return false;
                    };
                    let index = ctx.game.players[0]
                        .hand
                        .iter()
                        .position(|card| card.handle() == target);
                    index.is_some_and(|index| ctx.trash_from_hand_by_index(0, index).is_some())
                }
                IdentityPayment::RemoveHost | IdentityPayment::ReplayHost => {
                    let source = ctx.source_card;
                    if !ctx.trash_self_as_cost() {
                        return false;
                    }
                    if matches!(mode, IdentityPayment::ReplayHost) {
                        let Some(index) = ctx.game.players[0]
                            .trash
                            .iter()
                            .position(|card| card.handle() == source)
                        else {
                            return false;
                        };
                        return ctx
                            .game
                            .play_from_trash_with_cost(
                                0,
                                index,
                                digimon_engine::enums::CostDelta::Fixed(0),
                                PlaySource::ByEffect,
                            )
                            .is_some();
                    }
                    true
                }
                IdentityPayment::SuspendDeclaredHost => {
                    ctx.as_read().source_is_cost_target_permanent() && ctx.suspend_self_as_cost()
                }
            })
            .build()]
    }
}

fn identity_payment_runner(mode: IdentityPayment) -> DebugRunner {
    identity_payment_runner_with_optional(mode, true)
}

struct MandatoryIdentityPaymentReducer(IdentityPayment);
impl CardEffect for MandatoryIdentityPaymentReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let mut effects = IdentityPaymentReducer(self.0).effects(card);
        effects[0].optional = false;
        effects
    }
}

fn identity_payment_runner_with_optional(mode: IdentityPayment, optional: bool) -> DebugRunner {
    let mut field = make_test_card("IDENTITY-FIELD", "Identity field");
    field.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder()
        .add_card(field)
        .add_card(lvl3_base("GBASE", CardColor::Green))
        .add_card(lvl4_evo3("GEVO", CardColor::Green, EVO_GREEN, 4))
        .add_card(lvl4_evo3("GEVO2", CardColor::Green, EVO_GREEN, 4))
        .hand(0, &["GEVO", "GEVO2"])
        .deck(0, &["GBASE"; 8])
        .memory(10)
        .start();
    let source = if matches!(mode, IdentityPayment::RemoveDeclaredCard) {
        "IDENTITY-FIELD"
    } else {
        "GBASE"
    };
    if optional {
        r.register_effect(source, std::sync::Arc::new(IdentityPaymentReducer(mode)));
    } else {
        r.register_effect(
            source,
            std::sync::Arc::new(MandatoryIdentityPaymentReducer(mode)),
        );
    }
    r.place_on_field(0, "IDENTITY-FIELD", Some(0));
    r.place_on_field(0, "GBASE", Some(0));
    r.place_on_field(0, "GBASE", Some(0));
    r
}

#[test]
fn reducer_payment_removing_declared_card_cannot_substitute_next_hand_card() {
    let mut r = identity_payment_runner(IdentityPayment::RemoveDeclaredCard);
    let intended = r.game.players[0].hand[0].handle();
    let other = r.game.players[0].hand[1].handle();
    let base = r.game.players[0].battle_area[1].top_card().handle();
    let before = r.memory();
    r.game.digivolve_from_hand(0, 0, 1, PlaySource::ByDigivolve);
    let mut clone = r.game.clone();
    clone.resolve_selection(0, HAND_EFFECT_START).unwrap();
    assert!(clone.players[0]
        .trash
        .iter()
        .any(|card| card.handle() == intended));
    assert!(clone.players[0]
        .hand
        .iter()
        .any(|card| card.handle() == other));
    assert_eq!(clone.players[0].battle_area[1].top_card().handle(), base);
    assert_eq!(
        clone.memory, before,
        "missing declared card aborts evolution, not its payment"
    );
    assert!(clone.pending_selection.is_none());
    assert!(r.game.pending_selection.is_some());
}

#[test]
fn reducer_payment_removed_host_cannot_retarget_compacted_neighbor() {
    let mut r = identity_payment_runner(IdentityPayment::RemoveHost);
    let intended = r.game.players[0].hand[0].handle();
    let other_host = r.game.players[0].battle_area[2].top_card().handle();
    let before = r.memory();
    r.game.digivolve_from_hand(0, 0, 1, PlaySource::ByDigivolve);
    r.game.resolve_selection(0, HAND_EFFECT_START).unwrap();
    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|card| card.handle() == intended));
    assert_eq!(
        r.game.players[0].battle_area[1].top_card().handle(),
        other_host
    );
    assert_eq!(r.memory(), before);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn reducer_payment_replayed_same_card_is_not_original_host() {
    let mut r = identity_payment_runner(IdentityPayment::ReplayHost);
    let intended = r.game.players[0].hand[0].handle();
    let original_host_card = r.game.players[0].battle_area[1].top_card().handle();
    let before = r.memory();
    r.game.digivolve_from_hand(0, 0, 1, PlaySource::ByDigivolve);
    let mut clone = r.game.clone();
    clone.resolve_selection(0, HAND_EFFECT_START).unwrap();
    assert!(clone.players[0]
        .hand
        .iter()
        .any(|card| card.handle() == intended));
    assert!(clone.players[0]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().handle() == original_host_card));
    assert_eq!(
        clone.memory, before,
        "a replayed physical card is a new Digimon"
    );
    assert!(clone.pending_selection.is_none());
    assert!(r.game.pending_selection.is_some());
}

#[test]
fn reducer_payment_receives_declared_host_in_cost_context() {
    let mut r = identity_payment_runner(IdentityPayment::SuspendDeclaredHost);
    let intended = r.game.players[0].hand[0].handle();
    let before = r.memory();
    r.game.digivolve_from_hand(0, 0, 1, PlaySource::ByDigivolve);
    r.game.resolve_selection(0, HAND_EFFECT_START).unwrap();
    assert_eq!(
        r.game.players[0].battle_area[1].top_card().handle(),
        intended
    );
    assert!(
        r.game.players[0].battle_area[1].is_suspended,
        "target-aware pay-cost must see its own declared host"
    );
    assert_eq!(r.memory(), before - 3);
}

struct MandatoryMovingFieldReducer;
impl CardEffect for MandatoryMovingFieldReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let mut effects = MovingZoneReducer { trash_hand: false }.effects(card);
        effects[0].optional = false;
        effects
    }
}

#[test]
fn mandatory_sync_reducer_field_compaction_preserves_declared_host() {
    use digimon_engine::action::space::encode_digivolve;
    let mut field = make_test_card("MANDATORY-FIELD", "Mandatory field");
    field.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder()
        .add_card(field)
        .add_card(lvl3_base("GBASE", CardColor::Green))
        .add_card(lvl4_evo3("GEVO", CardColor::Green, EVO_GREEN, 4))
        .hand(0, &["GEVO"])
        .deck(0, &["GBASE"; 8])
        .memory(10)
        .start();
    r.register_effect(
        "MANDATORY-FIELD",
        std::sync::Arc::new(MandatoryMovingFieldReducer),
    );
    r.place_on_field(0, "MANDATORY-FIELD", Some(0));
    r.place_on_field(0, "GBASE", Some(0));
    r.place_on_field(0, "GBASE", Some(0));
    let intended_card = r.game.players[0].hand[0].handle();
    let intended_host = r.game.players[0].battle_area[1].top_card().handle();
    let other_host = r.game.players[0].battle_area[2].top_card().handle();
    let before = r.memory();
    let action = encode_digivolve(0, 1);
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[action as usize] > 0.0);
    r.game.decode_action(action, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "payment must trash its source"
    );
    assert_eq!(
        r.game.players[0].battle_area[0].top_card().handle(),
        intended_card,
        "the final synchronous scan must preserve the declared host too"
    );
    assert!(r.game.players[0].battle_area[0]
        .card_sources
        .iter()
        .any(|card| card.handle() == intended_host));
    assert_eq!(
        r.game.players[0].battle_area[1].top_card().handle(),
        other_host
    );
    assert_eq!(r.memory(), before - 3);
}

#[test]
fn mandatory_sync_reducer_removed_or_replayed_host_abandons_declared_evolution() {
    for mode in [IdentityPayment::RemoveHost, IdentityPayment::ReplayHost] {
        let mut r = identity_payment_runner_with_optional(mode, false);
        let declared = r.game.players[0].hand[0].handle();
        let untouched_neighbor = r.game.players[0].battle_area[2].top_card().handle();
        let before = r.memory();
        assert!(!r.game.digivolve_from_hand(0, 0, 1, PlaySource::ByDigivolve));
        assert!(r.game.players[0]
            .hand
            .iter()
            .any(|card| card.handle() == declared));
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().handle() == untouched_neighbor));
        assert_eq!(r.memory(), before);
        assert!(r.game.pending_selection.is_none());
    }
}
