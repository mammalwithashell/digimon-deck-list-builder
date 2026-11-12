//! ST21-03 Ikkakumon — Digimon, Lv.4, Blue, DP 4000, Cost 4.
//! Traits: Sea Beast / ADVENTURE. Attribute: Vaccine.
//!
//! Printed (official observation):
//!   [Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2
//!   [Security] At the end of the battle, play this card without paying the cost.
//!   [On Play] [When Digivolving] Trash the top 2 digivolution cards of 1 of your
//!     opponent's Digimon. Then, 1 of their Digimon with no digivolution cards
//!     can't attack or block until their turn ends.
//!   Inherited: [When Attacking] [Once Per Turn] If you have 7 or fewer cards in
//!     your hand, <Draw 1>.
//! Official Q&A: the lock persists even if the Digimon later gains sources.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/ST21/Blue/ST21_03.cs — both picks
//! mandatory (canNoSelect false); first pick over ANY opponent Digimon; second
//! over opponent Digimon with HasNoDigivolutionCards; GainCanNotAttack +
//! GainCanNotBlock UntilOpponentTurnEnd. ESS hand-count gate <= 7.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{ATTACK_START, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, Expiry, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::TriggerSource;

const CARD_ID: &str = "ST21-03";

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-03 YAML must be present in the embedded DSL pack")
        .add_card(digimon("SRC", 3))
        .add_card(digimon("OPP-TOP", 5))
        .add_card(digimon("OPP-BARE", 4))
        .add_card(digimon("HOST", 5))
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
        .memory(10)
}

fn sources(r: &DebugRunner, h: PermanentHandle) -> usize {
    r.game.players[h.player as usize].battle_area[h.index as usize].stack_size() - 1
}

fn locked(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.modifiers.has(h, ModifierType::CannotAttack)
        && r.game.modifiers.has(h, ModifierType::CannotBlock)
}

#[test]
fn st21_03_declares_security_op_wd_and_inherited_draw() {
    let r = base().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
    let t: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(t.iter().any(|t| t.when == vec![CompiledTiming::OnSecurity]));
    assert!(t.iter().any(|t| t.when.contains(&CompiledTiming::OnPlay)
        && t.when.contains(&CompiledTiming::WhenDigivolving)
        && !t.optional));
    assert!(t.iter().any(|t| t.scope == CompiledScope::Inherited
        && t.when == vec![CompiledTiming::WhenAttacking]
        && t.once_per_turn));
}

/// Strip 2 of 3 sources from the chosen Digimon; then the lock goes to a
/// Digimon with no digivolution cards.
#[test]
fn st21_03_on_play_trashes_top_two_sources_then_locks_sourceless_digimon() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let stacked = r.place_stack(1, &["SRC", "SRC", "SRC", "OPP-TOP"]);
    let bare = r.place_on_field(1, "OPP-BARE", Some(0));
    assert_eq!(sources(&r, stacked), 3);

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();

    // First pick: any opponent Digimon (both offered), mandatory.
    let view = r.pending_selection_view().expect("strip pick installs");
    assert!(!view.valid_action_ids.contains(&PASS), "strip pick is mandatory");
    assert_eq!(view.valid_action_ids.len(), 2, "any opponent Digimon may be chosen");
    // Opponent field slots encode as ATTACK_START + index.
    let action = ATTACK_START + stacked.index as u16;
    assert!(view.valid_action_ids.contains(&action), "stacked Digimon is selectable");
    r.execute_action(0, action).expect("pick stacked");

    assert_eq!(sources(&r, stacked), 1, "top 2 digivolution cards trashed");

    // Second pick: only the sourceless Digimon is eligible.
    let view = r.pending_selection_view().expect("lock pick installs");
    assert!(!view.valid_action_ids.contains(&PASS), "lock pick is mandatory");
    assert_eq!(view.valid_action_ids.len(), 1, "only the sourceless Digimon");
    r.auto_resolve().expect("resolve");

    assert!(locked(&r, bare), "sourceless Digimon can't attack or block");
    assert!(!locked(&r, stacked), "Digimon that still has a source is not locked");
    let e = r.game.modifiers.get(bare, ModifierType::CannotAttack);
    assert_eq!(e[0].expiry, Expiry::EndOfOpponentsTurn);
}

/// Stripping a 2-source Digimon to zero makes it eligible for the lock.
#[test]
fn st21_03_when_digivolving_strip_to_zero_then_lock_same_digimon() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let stacked = r.place_stack(1, &["SRC", "SRC", "OPP-TOP"]);

    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    r.auto_resolve().expect("resolve");

    assert_eq!(sources(&r, stacked), 0);
    assert!(locked(&r, stacked));
}

#[test]
fn st21_03_no_opponent_digimon_installs_nothing() {
    let mut r = base().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(me));
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
}

#[test]
fn st21_03_security_plays_card_onto_field() {
    let mut r = base().add_card(digimon("ATTACKER", 4)).start();
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .unwrap();
    let iid = r.game.next_card_index();
    r.game.players[1].security.push(CardSource::new(idx, 1, iid));

    r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();

    assert!(r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID));
}

#[test]
fn st21_03_inherited_when_attacking_draws_with_seven_or_fewer_cards_in_hand() {
    let mut r = base().hand(0, &["PAD"; 7]).start();
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let before = r.hand_size(0);
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), before + 1, "7 cards in hand → draw 1");

    // Once per turn.
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), before + 1, "only once per turn");
}

#[test]
fn st21_03_inherited_when_attacking_no_draw_with_eight_cards_in_hand() {
    let mut r = base().hand(0, &["PAD"; 8]).start();
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let before = r.hand_size(0);
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), before, "8 cards in hand → no draw");
}
