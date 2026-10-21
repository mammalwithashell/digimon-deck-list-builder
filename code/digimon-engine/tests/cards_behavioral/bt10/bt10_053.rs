//! BT10-053 Ajatarmon — Digimon, Lv.5, Green, Cost 8, DP 7000.
//! Traits: Vegetation. Form: Ultimate. Attribute: Vaccine.
//! Digivolve: green Lv.4, cost 3.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT10-053.md; card image agrees)
//!
//! ```text
//! [Main][Once Per Turn] By suspending 1 of your green Digimon, you may play 1
//! Digimon card with [Vegetation], [Plant], or [Fairy] in one of its traits
//! and 3000 DP or less from your hand without paying its play cost.
//! Inherited: [Your Turn][Once Per Turn] When an effect suspends one of your
//! Digimon, gain 1 memory.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT10/Green/BT10_053.cs
//!   - [Main] (OnDeclaration, maxCountPerTurn 1): usable while an own green
//!     battle-area Digimon exists. SelectPermanent (own green, canNoSelect) →
//!     if unsuspended & can suspend: SuspendPermanentsClass by THIS effect →
//!     only if it became suspended: SelectHand (canNoSelect) Digimon with
//!     Plant/Fairy traits and DP <= 3000 → PlayPermanentCards free.
//!   - Inherited (OnTappedAnyone): IsOwnerTurn, own Digimon, IsByEffect,
//!     maxCountPerTurn 1 → AddMemory(1). isOptional false.
//!
//! # Official Q&A (data/card_qa.json)
//! - Q1977: Ajatarmon may suspend itself for its own [Main].
//! - Q1978: suspending to declare an attack (even via an "attack" effect) is
//!   not "an effect suspends" — the inherited memory gain does not activate.
//!
//! # Rules
//! - §15-7-1/-2/-4/-5 optional processing condition ("By suspending ...").
//! - §15-6-3: the [Main] can't be activated when its processing condition
//!   can't be met (no unsuspended green Digimon of yours).
//!
//! # Patterns this test covers
//! - `main_on_field` + `once_per_turn` activated ability through the action mask
//! - optional own-permanent suspend cost → gated optional free play from hand
//! - inherited on_suspend `gain_memory` (`trigger_if: event_is_effect_initiated`), OPT

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledCost, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, ATTACK_START, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN,
    FIELD_EFFECT_START, PASS, PLAY_HAND_START, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT10-053";

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c
}

struct SuspendByEffect {
    target: PermanentHandle,
}
impl CardEffect for SuspendByEffect {
    fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        let target = self.target;
        vec![Effect::when_attacking(card)
            .name("suspend by effect")
            .process(move |ctx| {
                ctx.suspend(target);
            })
            .build()]
    }
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT10-053 in pack")
        .add_card(digimon("ALLY-G", CardColor::Green, 4, 4000, &[]))
        .add_card(digimon("ALLY-B", CardColor::Blue, 4, 4000, &[]))
        .add_card(digimon("PLANT3K", CardColor::Green, 3, 3000, &["Plant"]))
        .add_card(digimon("FAIRY2K", CardColor::Yellow, 3, 2000, &["Fairy"]))
        .add_card(digimon("VEGBIG", CardColor::Green, 4, 4000, &["Vegetation"]))
        .add_card(digimon("BEAST", CardColor::Green, 3, 2000, &["Beast"]))
        .add_card(make_test_card("SRC", "Src"))
        .add_card(digimon("FILLER", CardColor::Blue, 3, 3000, &[]))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .security(0, &["FILLER"; 5])
        .security(1, &["FILLER"; 5])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

fn hand_idx(r: &DebugRunner, id: &str) -> usize {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in hand"))
}

fn hand_pick(r: &DebugRunner, id: &str) -> u16 {
    PLAY_HAND_START + hand_idx(r, id) as u16
}

fn on_field(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

fn own_field_pick(h: PermanentHandle) -> u16 {
    ATTACK_START + h.index as u16
}

fn main_action(h: PermanentHandle) -> u16 {
    FIELD_EFFECT_START + h.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

fn main_legal(r: &DebugRunner, h: PermanentHandle) -> bool {
    build_action_mask(&r.game, 0)[main_action(h) as usize] == 1.0
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

/// Ajatarmon on P0's field in the main phase, `hand` added to P0's hand.
fn ajatarmon_in_main(r: &mut DebugRunner, hand: &[&str]) -> PermanentHandle {
    let aja = r.place_on_field(0, CARD_ID, Some(0));
    for id in hand {
        r.add_to_hand(0, id);
    }
    r.game.enter_main_phase();
    aja
}

fn suspend_via_effect(r: &mut DebugRunner, player: PlayerId, target: PermanentHandle) {
    let src = r.place_on_field(player, "SRC", None);
    r.register_effect("SRC", Arc::new(SuspendByEffect { target }));
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(src));
    r.game.drain_effect_queue();
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt10_053_metadata_and_green_lv4_cost3_circle() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Ajatarmon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(8));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.traits, vec!["Vegetation".to_string()]);
    assert_eq!(card.attribute, ["Vaccine"]);
    assert_eq!(card.alt_paths.len(), 1);
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(3)));
}

#[test]
fn bt10_053_has_main_opt_and_inherited_opt_on_suspend_clauses() {
    let card = dsl_card_data::compiled(CARD_ID);
    let t: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(card.effects.len(), 2);
    assert_eq!(t.len(), 2);
    assert_eq!(t[0].when, vec![CompiledTiming::MainOnField]);
    assert_ne!(t[0].scope, CompiledScope::Inherited);
    assert!(t[0].once_per_turn, "[Main][Once Per Turn]");
    assert_eq!(t[1].scope, CompiledScope::Inherited);
    assert_eq!(t[1].when, vec![CompiledTiming::OnSuspend]);
    assert!(t[1].once_per_turn);
    assert!(!t[1].optional, "the memory gain is mandatory");
}

// ─── SECTION 2 — [Main] gating ──────────────────────────────────────────────

#[test]
fn bt10_053_main_legal_with_an_unsuspended_green_digimon() {
    let mut r = runner();
    let aja = ajatarmon_in_main(&mut r, &[]);
    assert!(main_legal(&r, aja));
}

/// §15-6-3 / §15-7-4: no unsuspended green Digimon of yours → can't activate.
#[test]
fn bt10_053_main_illegal_without_an_unsuspended_green_digimon() {
    let mut r = runner();
    r.place_on_field(0, "ALLY-B", Some(0));
    let aja = ajatarmon_in_main(&mut r, &["PLANT3K"]);
    r.game.players[0].battle_area[aja.index as usize].is_suspended = true;
    assert!(!main_legal(&r, aja), "only a blue Digimon is unsuspended");
}

// ─── SECTION 3 — [Main] behavior ────────────────────────────────────────────

/// Q1977: Ajatarmon may suspend itself.
#[test]
fn bt10_053_main_suspend_self_then_play_matching_digimon_free() {
    let mut r = runner();
    let aja = ajatarmon_in_main(&mut r, &["PLANT3K", "FAIRY2K", "VEGBIG", "BEAST"]);
    let mem = r.memory();
    r.game.decode_action(main_action(aja), 0);

    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(v.is_optional, "the 'by suspending' condition is declinable");
    assert!(v.valid_action_ids.contains(&own_field_pick(aja)), "Q1977");
    r.execute_action(0, own_field_pick(aja)).unwrap();
    assert!(suspended(&r, aja));

    let v = r.pending_selection_view().expect("hand play pick");
    assert_eq!(v.kind, SelectionKind::Hand);
    assert!(v.is_optional, "you MAY play");
    assert!(v.valid_action_ids.contains(&hand_pick(&r, "PLANT3K")));
    assert!(v.valid_action_ids.contains(&hand_pick(&r, "FAIRY2K")));
    assert!(!v.valid_action_ids.contains(&hand_pick(&r, "VEGBIG")), "DP 4000");
    assert!(!v.valid_action_ids.contains(&hand_pick(&r, "BEAST")), "trait");
    r.execute_action(0, hand_pick(&r, "FAIRY2K")).unwrap();
    let _ = r.auto_resolve();
    assert!(on_field(&r, "FAIRY2K"));
    assert_eq!(r.memory(), mem, "played without paying its play cost");
}

#[test]
fn bt10_053_main_can_suspend_another_green_digimon_not_a_blue_one() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY-G", Some(0));
    let blue = r.place_on_field(0, "ALLY-B", Some(0));
    let aja = ajatarmon_in_main(&mut r, &["PLANT3K"]);
    r.game.decode_action(main_action(aja), 0);
    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert!(v.valid_action_ids.contains(&own_field_pick(ally)));
    assert!(!v.valid_action_ids.contains(&own_field_pick(blue)));
    r.execute_action(0, own_field_pick(ally)).unwrap();
    assert!(suspended(&r, ally));
    assert!(!suspended(&r, aja));
    r.execute_action(0, hand_pick(&r, "PLANT3K")).unwrap();
    let _ = r.auto_resolve();
    assert!(on_field(&r, "PLANT3K"));
}

#[test]
fn bt10_053_main_declining_the_suspend_plays_nothing() {
    let mut r = runner();
    let aja = ajatarmon_in_main(&mut r, &["PLANT3K"]);
    r.game.decode_action(main_action(aja), 0);
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(!suspended(&r, aja));
    assert!(!on_field(&r, "PLANT3K"), "§15-7-2");
}

#[test]
fn bt10_053_main_suspend_then_decline_the_play() {
    let mut r = runner();
    let aja = ajatarmon_in_main(&mut r, &["PLANT3K"]);
    r.game.decode_action(main_action(aja), 0);
    r.execute_action(0, own_field_pick(aja)).unwrap();
    let v = r.pending_selection_view().expect("hand play pick");
    assert!(v.is_optional);
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, aja));
    assert!(!on_field(&r, "PLANT3K"));
}

// ─── SECTION 4 — [Main] OPT lockout ────────────────────────────────────────

#[test]
fn bt10_053_main_once_per_turn_locks_then_clears_next_own_turn() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY-G", Some(0));
    let aja = ajatarmon_in_main(&mut r, &["PLANT3K"]);
    r.game.decode_action(main_action(aja), 0);
    r.execute_action(0, own_field_pick(ally)).unwrap();
    r.execute_action(0, hand_pick(&r, "PLANT3K")).unwrap();
    let _ = r.auto_resolve();
    assert!(!suspended(&r, aja), "Ajatarmon still unsuspended");
    assert!(!main_legal(&r, aja), "[Once Per Turn]: second activation gated");

    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0);
    r.game.enter_main_phase();
    assert!(main_legal(&r, aja), "lockout clears on the next own turn");
}

// ─── SECTION 5 — Inherited gain 1 memory ───────────────────────────────────

#[test]
fn bt10_053_inherited_effect_suspend_of_own_digimon_gains_1_memory() {
    let mut r = runner();
    r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let a1 = r.place_on_field(0, "ALLY-B", Some(0));
    let mem = r.memory();
    suspend_via_effect(&mut r, 0, a1);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem + 1, "gain 1 memory");
}

/// Q1978: an attack-declaration suspension is not "by an effect".
#[test]
fn bt10_053_inherited_attack_suspend_gains_nothing() {
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    r.game.enter_main_phase();
    let mem = r.memory();
    let act = encode_attack(host.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[act as usize], 1.0);
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();
    assert!(suspended(&r, host));
    assert_eq!(r.memory(), mem, "attacking is not 'by an effect'");
}

#[test]
fn bt10_053_inherited_opponent_digimon_suspend_gains_nothing() {
    let mut r = runner();
    r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let opp = r.place_on_field(1, "ALLY-B", Some(0));
    let mem = r.memory();
    suspend_via_effect(&mut r, 0, opp);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem, "not one of YOUR Digimon");
}

#[test]
fn bt10_053_inherited_does_not_fire_on_opponents_turn() {
    let mut r = runner();
    r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let ally = r.place_on_field(0, "ALLY-B", Some(0));
    r.game.turn_player_idx = 1;
    let mem = r.memory();
    suspend_via_effect(&mut r, 1, ally);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem, "[Your Turn]");
}

#[test]
fn bt10_053_inherited_once_per_turn() {
    let mut r = runner();
    r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let a1 = r.place_on_field(0, "ALLY-B", Some(0));
    let a2 = r.place_on_field(0, "ALLY-B", Some(0));
    let mem = r.memory();
    suspend_via_effect(&mut r, 0, a1);
    let _ = r.auto_resolve();
    suspend_via_effect(&mut r, 0, a2);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem + 1, "[Once Per Turn]");
}

/// A buried Ajatarmon under a top Ajatarmon: the top's [Main] suspend (an
/// effect) fires the buried copy's inherited memory gain.
#[test]
fn bt10_053_main_suspend_fires_a_buried_ajatarmons_inherited_gain() {
    let mut r = runner();
    let aja = r.place_stack(0, &[CARD_ID, CARD_ID]);
    r.game.enter_main_phase();
    let mem = r.memory();
    r.game.decode_action(main_action(aja), 0);
    r.execute_action(0, own_field_pick(aja)).unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, aja));
    assert_eq!(r.memory(), mem + 1, "buried Ajatarmon's inherited +1 memory");
}
