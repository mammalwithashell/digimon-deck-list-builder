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
fn decline_keeps_full_cost_and_leaves_reducer_armed() {
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
    assert_eq!(
        r.game.player_digivolve_cost_reducers.len(),
        1,
        "a declined prompt must leave the reducer armed"
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
