//! EX4-002 Kokomon — Digi-Egg (In-Training), Lv.2, Green, Cost 0, no DP.
//! Traits: Lesser.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX4-002.md) — INHERITED ONLY
//! `Inherited: [Your Turn][Once Per Turn] When an effect suspends one of your`
//! `Digimon, <Draw 1>. (Draw 1 card from your deck.)`
//!
//! Official Q&A: "Attack with this Digimon" suspends by an ATTACK, not by an
//! effect — Kokomon does NOT activate.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX4/Green/EX4_002.cs
//!   OnTappedAnyone, inherited, IsOwnerTurn, maxUsableCount 1, own Digimon,
//!   IsByEffect → DrawClass(owner, 1).
//!
//! # Patterns
//! - inherited on_suspend observer gated `event_is_effect_initiated`
//! - [Your Turn] gate, own-Digimon gate, [Once Per Turn] lockout
//! - attack-declaration suspend negative (official Q&A)
//! - integration: real effect suspend via BT10-053 Ajatarmon [Main]

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
    SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "EX4-002";

fn make_digimon(card_id: &str, color: CardColor, dp: i32) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.dp = Some(dp);
    card.level = Some(4);
    card
}

/// Test effect whose body suspends a fixed target THROUGH the effect context.
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
        .expect("EX4-002 in pack")
        .dsl_card("BT10-053")
        .expect("BT10-053 in pack")
        .add_card(make_digimon("HOST", CardColor::Green, 4000))
        .add_card(make_digimon("ALLY", CardColor::Green, 3000))
        .add_card(make_test_card("SRC", "Src"))
        .add_card(make_digimon("FILLER", CardColor::Blue, 3000))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .security(1, &["FILLER"; 5])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

fn suspend_via_effect(r: &mut DebugRunner, player: PlayerId, target: PermanentHandle) {
    let src = r.place_on_field(player, "SRC", None);
    r.register_effect("SRC", Arc::new(SuspendByEffect { target }));
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(src));
    r.game.drain_effect_queue();
}

// ─── Structural ─────────────────────────────────────────────────────────────

#[test]
fn ex4_002_metadata_and_single_inherited_clause() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Kokomon");
    assert_eq!(card.level, Some(2));
    assert_eq!(card.dp, None);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    let t = triggered[0];
    assert_eq!(t.scope, CompiledScope::Inherited);
    assert!(t.when.contains(&CompiledTiming::OnSuspend));
    assert!(t.once_per_turn);
    assert!(!t.optional, "<Draw 1> is mandatory");
}

// ─── Behavioral ─────────────────────────────────────────────────────────────

#[test]
fn ex4_002_effect_suspend_of_own_digimon_draws_1() {
    let mut r = runner();
    let _host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, ally);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1, "<Draw 1> on effect suspend");
}

/// Integration: BT10-053 Ajatarmon's [Main] suspends the Kokomon carrier
/// itself (a real effect suspension) → Kokomon draws 1.
#[test]
fn ex4_002_ajatarmon_main_suspend_triggers_draw() {
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "BT10-053"]);
    r.game.enter_main_phase();
    let hand = r.hand_size(0);
    let act = FIELD_EFFECT_START
        + host.index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(build_action_mask(&r.game, 0)[act as usize], 1.0);
    r.game.decode_action(act, 0);
    // Pick Ajatarmon itself as the suspend target.
    let v = r.pending_selection_view().expect("suspend-target pick");
    let pick = *v
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .unwrap();
    r.execute_action(0, pick).unwrap();
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[host.index as usize].is_suspended);
    assert_eq!(r.hand_size(0), hand + 1, "Kokomon drew 1");
}

#[test]
fn ex4_002_attack_suspend_does_not_draw() {
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    r.game.enter_main_phase();
    let hand = r.hand_size(0);
    let act = encode_attack(host.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[act as usize], 1.0);
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[host.index as usize].is_suspended);
    assert_eq!(r.hand_size(0), hand, "attack suspension is not 'by an effect'");
}

#[test]
fn ex4_002_raw_suspend_does_not_draw() {
    let mut r = runner();
    let _host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let hand = r.hand_size(0);
    r.game.suspend(ally);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand);
}

#[test]
fn ex4_002_opponent_digimon_suspend_does_not_draw() {
    let mut r = runner();
    let _host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let opp = r.place_on_field(1, "ALLY", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, opp);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand, "not one of YOUR Digimon");
}

#[test]
fn ex4_002_does_not_draw_on_opponents_turn() {
    let mut r = runner();
    let _host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.turn_player_idx = 1;
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 1, ally);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand, "[Your Turn] gate");
}

#[test]
fn ex4_002_top_card_kokomon_alone_is_not_inherited() {
    // Kokomon as a bare egg in the battle area is never a source; place a
    // Digimon WITHOUT Kokomon underneath → no draw.
    let mut r = runner();
    let _host = r.place_on_field(0, "HOST", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, ally);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand);
}

#[test]
fn ex4_002_once_per_turn() {
    let mut r = runner();
    let _host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let a1 = r.place_on_field(0, "ALLY", Some(0));
    let a2 = r.place_on_field(0, "ALLY", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, a1);
    let _ = r.auto_resolve();
    suspend_via_effect(&mut r, 0, a2);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1, "[Once Per Turn]: only one draw");
}
