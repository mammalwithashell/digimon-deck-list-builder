//! BT10-053 Ajatarmon — Digimon, Lv.5, Green, Cost 8, DP 7000.
//! Traits: Vegetation. Form: Ultimate. Attribute: Vaccine. Digivolve: green Lv.4, cost 3.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT10-053.md)
//! [Main][Once Per Turn] By suspending 1 of your green Digimon, you may play 1
//! Digimon card with [Vegetation], [Plant], or [Fairy] in one of its traits and
//! 3000 DP or less from your hand without paying its play cost.
//! Inherited: [Your Turn][Once Per Turn] When an effect suspends one of your
//! Digimon, gain 1 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT10/Green/BT10_053.cs
//!   OnDeclaration OPT; SelectPermanent (own green Digimon, canNoSelect) →
//!   suspend by this effect → if suspended: SelectHand (canNoSelect) Digimon
//!   with Plant/Fairy traits & DP ≤ 3000 → play free. Inherited:
//!   OnTappedAnyone, own turn, own Digimon, IsByEffect, OPT → AddMemory(1).
//!
//! # Patterns
//! - [Main] field activation via FIELD_EFFECT action + mask visibility
//! - optional suspend-cost pick, gated free play, decline paths, OPT lockout
//! - inherited on_suspend gain-memory: effect positive, attack negative, OPT

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
    PLAY_HAND_START, SECURITY_TARGET,
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
        .add_card(digimon("VEG3K", CardColor::Green, 3, 3000, &["Vegetation"]))
        .add_card(digimon("FAIRY2K", CardColor::Yellow, 3, 2000, &["Fairy"]))
        .add_card(digimon("PLANTBIG", CardColor::Green, 4, 5000, &["Plant"]))
        .add_card(digimon("BEAST", CardColor::Green, 3, 2000, &["Beast"]))
        .add_card(make_test_card("SRC", "Src"))
        .add_card(digimon("FILLER", CardColor::Blue, 3, 3000, &[]))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .security(1, &["FILLER"; 5])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

fn main_act(h: PermanentHandle) -> u16 {
    FIELD_EFFECT_START + h.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

fn own_field_pick(slot: u8) -> u16 {
    encode_attack(0, slot as u16)
}

fn hand_idx(r: &DebugRunner, id: &str) -> usize {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in hand"))
}

fn on_field(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

// ─── Structural ─────────────────────────────────────────────────────────────

#[test]
fn bt10_053_structure() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Ajatarmon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    let t: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 2);
    assert!(t[0].when.contains(&CompiledTiming::MainOnField));
    assert!(t[0].once_per_turn);
    assert_eq!(t[1].scope, CompiledScope::Inherited);
    assert!(t[1].when.contains(&CompiledTiming::OnSuspend));
    assert!(t[1].once_per_turn);
    assert!(!t[1].optional);
}

// ─── [Main][OPT] ────────────────────────────────────────────────────────────

#[test]
fn bt10_053_main_suspend_self_and_play_free() {
    let mut r = runner();
    let aja = r.place_on_field(0, CARD_ID, Some(0));
    for id in ["VEG3K", "FAIRY2K", "PLANTBIG", "BEAST"] {
        r.add_to_hand(0, id);
    }
    r.game.enter_main_phase();
    r.game.set_memory(5);
    assert_eq!(build_action_mask(&r.game, 0)[main_act(aja) as usize], 1.0);
    r.game.decode_action(main_act(aja), 0);

    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(v.is_optional);
    r.execute_action(0, own_field_pick(aja.index)).unwrap();
    assert!(suspended(&r, aja));

    let v = r.pending_selection_view().expect("hand pick");
    assert_eq!(v.kind, SelectionKind::Hand);
    assert!(v.is_optional);
    let id = |n: &str| PLAY_HAND_START + hand_idx(&r, n) as u16;
    assert!(v.valid_action_ids.contains(&id("VEG3K")));
    assert!(v.valid_action_ids.contains(&id("FAIRY2K")));
    assert!(!v.valid_action_ids.contains(&id("PLANTBIG")), "DP 5000 > 3000");
    assert!(!v.valid_action_ids.contains(&id("BEAST")));
    r.execute_action(0, id("VEG3K")).unwrap();
    let _ = r.auto_resolve();
    assert!(on_field(&r, "VEG3K"));
    assert_eq!(r.memory(), 5, "played without paying its cost");

    // [Once Per Turn]: the [Main] is no longer offered this turn.
    assert_eq!(build_action_mask(&r.game, 0)[main_act(aja) as usize], 0.0);
}

#[test]
fn bt10_053_main_suspends_another_green_not_blue() {
    let mut r = runner();
    let aja = r.place_on_field(0, CARD_ID, Some(0));
    let g = r.place_on_field(0, "ALLY-G", Some(0));
    let b = r.place_on_field(0, "ALLY-B", Some(0));
    r.add_to_hand(0, "FAIRY2K");
    r.game.enter_main_phase();
    r.game.decode_action(main_act(aja), 0);
    let v = r.pending_selection_view().unwrap();
    assert!(v.valid_action_ids.contains(&own_field_pick(g.index)));
    assert!(!v.valid_action_ids.contains(&own_field_pick(b.index)));
    r.execute_action(0, own_field_pick(g.index)).unwrap();
    assert!(suspended(&r, g));
    assert!(!suspended(&r, aja));
    let i = PLAY_HAND_START + hand_idx(&r, "FAIRY2K") as u16;
    r.execute_action(0, i).unwrap();
    let _ = r.auto_resolve();
    assert!(on_field(&r, "FAIRY2K"));
}

#[test]
fn bt10_053_main_hidden_when_no_unsuspended_green_digimon() {
    let mut r = runner();
    let aja = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[aja.index as usize].is_suspended = true;
    let _b = r.place_on_field(0, "ALLY-B", Some(0));
    r.game.enter_main_phase();
    assert_eq!(build_action_mask(&r.game, 0)[main_act(aja) as usize], 0.0);
}

#[test]
fn bt10_053_main_decline_suspend_plays_nothing() {
    let mut r = runner();
    let aja = r.place_on_field(0, CARD_ID, Some(0));
    r.add_to_hand(0, "VEG3K");
    r.game.enter_main_phase();
    r.game.decode_action(main_act(aja), 0);
    let v = r.pending_selection_view().unwrap();
    assert!(v.is_optional, "declinable (PASS via is_optional)");
    r.execute_action(0, PASS).unwrap();
    assert!(r.pending_selection_view().is_none(), "no play pick after decline");
    assert!(!suspended(&r, aja));
    assert!(!on_field(&r, "VEG3K"));
}

#[test]
fn bt10_053_main_suspend_then_decline_play() {
    let mut r = runner();
    let aja = r.place_on_field(0, CARD_ID, Some(0));
    r.add_to_hand(0, "VEG3K");
    r.game.enter_main_phase();
    r.game.decode_action(main_act(aja), 0);
    r.execute_action(0, own_field_pick(aja.index)).unwrap();
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, aja));
    assert!(!on_field(&r, "VEG3K"));
}

// ─── Inherited: gain 1 memory ───────────────────────────────────────────────

fn suspend_via_effect(r: &mut DebugRunner, player: PlayerId, target: PermanentHandle) {
    let src = r.place_on_field(player, "SRC", None);
    r.register_effect("SRC", Arc::new(SuspendByEffect { target }));
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(src));
    r.game.drain_effect_queue();
}

#[test]
fn bt10_053_inherited_effect_suspend_gains_1_memory_once_per_turn() {
    let mut r = runner();
    let _host = r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let a1 = r.place_on_field(0, "ALLY-B", Some(0));
    let a2 = r.place_on_field(0, "ALLY-B", Some(0));
    r.game.set_memory(3);
    suspend_via_effect(&mut r, 0, a1);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 4, "gain 1 memory");
    suspend_via_effect(&mut r, 0, a2);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 4, "[Once Per Turn]");
}

#[test]
fn bt10_053_inherited_attack_suspend_no_memory() {
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    r.game.enter_main_phase();
    r.game.set_memory(3);
    let act = encode_attack(host.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[act as usize], 1.0);
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();
    assert!(suspended(&r, host));
    assert_eq!(r.memory(), 3, "attack suspension is not 'by an effect'");
}

#[test]
fn bt10_053_inherited_not_for_opponent_digimon_or_opponent_turn() {
    let mut r = runner();
    let _host = r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let opp = r.place_on_field(1, "ALLY-B", Some(0));
    r.game.set_memory(3);
    suspend_via_effect(&mut r, 0, opp);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 3, "opponent's Digimon");
    let ally = r.place_on_field(0, "ALLY-B", Some(0));
    r.game.turn_player_idx = 1;
    let p0_mem_before = r.memory();
    suspend_via_effect(&mut r, 1, ally);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), p0_mem_before, "[Your Turn]");
}
