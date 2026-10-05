//! BT10-048 Sunflowmon — Digimon, Lv.4, Green, Cost 4, DP 3000.
//! Traits: Vegetation. Form: Champion. Attribute: Data. Digivolve: green Lv.3, cost 2.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT10-048.md)
//! [When Digivolving] By suspending 1 of your green Digimon, you may play 1
//! Digimon card with [Vegetation], [Plant], or [Fairy] in one of its traits
//! and 3000 DP or less from your hand without paying its play cost.
//! Inherited: [Your Turn][Once Per Turn] When an effect suspends one of your
//! Digimon, <Draw 1>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT10/Green/BT10_048.cs
//!   WD optional; SelectPermanent (own green Digimon, canNoSelect) → suspend
//!   by this effect → if it became suspended: SelectHand (canNoSelect) Digimon
//!   with Plant/Fairy traits & DP ≤ 3000 → play free. Inherited: OnTappedAnyone,
//!   own turn, own Digimon, IsByEffect, OPT → Draw 1.
//!
//! # Patterns
//! - real digivolve flow (encode_digivolve) → WD
//! - optional suspend-cost pick (self or another green Digimon), decline path
//! - gated free play from hand, filter positives/negatives (trait substring, DP)
//! - inherited on_suspend <Draw 1>: effect positive, attack negative, OPT

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, PASS, PLAY_HAND_START, SECURITY_TARGET,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT10-048";

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
        .expect("BT10-048 in pack")
        .add_card(digimon("ROOKIE", CardColor::Green, 3, 3000, &[]))
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
        .security(1, &["FILLER"; 5])
        .memory(5)
        .start();
    r.set_first_player(0);
    // Printed standard digivolve circle (cards.json): green Lv.3, cost 2.
    let card = Arc::make_mut(&mut r.game.card_data.0)
        .iter_mut()
        .find(|c| c.card_id == CARD_ID)
        .unwrap();
    card.evo_costs = vec![EvoCost {
        card_color: 3,
        level: 3,
        memory_cost: 2,
    }];
    r
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

/// Stage: ROOKIE on field slot 0, Sunflowmon + `extra` in hand, main phase;
/// digivolve ROOKIE → Sunflowmon through the real action. Returns the
/// Sunflowmon permanent handle.
fn digivolve_into_sunflowmon(r: &mut DebugRunner, extra: &[&str]) -> PermanentHandle {
    let rookie = r.place_on_field(0, "ROOKIE", Some(0));
    r.add_to_hand(0, CARD_ID);
    for id in extra {
        r.add_to_hand(0, id);
    }
    r.game.enter_main_phase();
    r.game.set_memory(5);
    let act = encode_digivolve(hand_idx(r, CARD_ID) as u16, rookie.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[act as usize], 1.0, "digivolve legal");
    r.game.decode_action(act, 0);
    rookie
}

fn own_field_pick(slot: u8) -> u16 {
    encode_attack(0, slot as u16)
}

// ─── Structural ─────────────────────────────────────────────────────────────

#[test]
fn bt10_048_structure() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Sunflowmon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.traits, vec!["Vegetation".to_string()]);
    let t: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 2);
    assert!(t[0].when.contains(&CompiledTiming::WhenDigivolving));
    assert_ne!(t[0].scope, CompiledScope::Inherited);
    assert!(!t[0].once_per_turn);
    assert_eq!(t[1].scope, CompiledScope::Inherited);
    assert!(t[1].when.contains(&CompiledTiming::OnSuspend));
    assert!(t[1].once_per_turn);
    assert!(!t[1].optional);
}

// ─── [When Digivolving] ─────────────────────────────────────────────────────

#[test]
fn bt10_048_wd_suspend_self_then_play_plant_free() {
    let mut r = runner();
    let sun = digivolve_into_sunflowmon(&mut r, &["PLANT3K", "FAIRY2K", "VEGBIG", "BEAST"]);
    assert_eq!(r.memory(), 3, "digivolve cost 2 paid");

    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(v.is_optional, "the 'by suspending' cost is declinable");
    assert!(v.valid_action_ids.contains(&own_field_pick(sun.index)));
    r.execute_action(0, own_field_pick(sun.index)).unwrap();
    assert!(r.game.players[0].battle_area[sun.index as usize].is_suspended);

    let v = r.pending_selection_view().expect("hand play pick");
    assert_eq!(v.kind, SelectionKind::Hand);
    assert!(v.is_optional, "you MAY play");
    let ids = |id: &str| PLAY_HAND_START + hand_idx(&r, id) as u16;
    assert!(v.valid_action_ids.contains(&ids("PLANT3K")), "[Plant] DP 3000 ok");
    assert!(v.valid_action_ids.contains(&ids("FAIRY2K")), "[Fairy] any color ok");
    assert!(!v.valid_action_ids.contains(&ids("VEGBIG")), "DP 4000 excluded");
    assert!(!v.valid_action_ids.contains(&ids("BEAST")), "no matching trait");
    r.execute_action(0, ids("PLANT3K")).unwrap();
    let _ = r.auto_resolve();
    assert!(on_field(&r, "PLANT3K"), "played");
    assert_eq!(r.memory(), 3, "played without paying its play cost");
}

#[test]
fn bt10_048_wd_can_suspend_another_green_digimon() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY-G", Some(0));
    let blue = r.place_on_field(0, "ALLY-B", Some(0));
    let sun = digivolve_into_sunflowmon(&mut r, &["FAIRY2K"]);
    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert!(v.valid_action_ids.contains(&own_field_pick(ally.index)));
    assert!(
        !v.valid_action_ids.contains(&own_field_pick(blue.index)),
        "non-green Digimon is not a legal suspend cost"
    );
    r.execute_action(0, own_field_pick(ally.index)).unwrap();
    assert!(r.game.players[0].battle_area[ally.index as usize].is_suspended);
    assert!(!r.game.players[0].battle_area[sun.index as usize].is_suspended);
    let i = PLAY_HAND_START + hand_idx(&r, "FAIRY2K") as u16;
    r.execute_action(0, i).unwrap();
    let _ = r.auto_resolve();
    assert!(on_field(&r, "FAIRY2K"));
}

#[test]
fn bt10_048_wd_already_suspended_green_not_offered() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY-G", Some(0));
    r.game.players[0].battle_area[ally.index as usize].is_suspended = true;
    let _sun = digivolve_into_sunflowmon(&mut r, &["FAIRY2K"]);
    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert!(!v.valid_action_ids.contains(&own_field_pick(ally.index)));
}

#[test]
fn bt10_048_wd_decline_suspend_plays_nothing() {
    let mut r = runner();
    let sun = digivolve_into_sunflowmon(&mut r, &["PLANT3K"]);
    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert!(v.is_optional, "declinable (PASS via is_optional)");
    r.execute_action(0, PASS).unwrap();
    assert!(
        r.pending_selection_view().is_none(),
        "declining the cost skips the play pick"
    );
    assert!(!r.game.players[0].battle_area[sun.index as usize].is_suspended);
    assert!(!on_field(&r, "PLANT3K"));
}

#[test]
fn bt10_048_wd_suspend_then_decline_play() {
    let mut r = runner();
    let sun = digivolve_into_sunflowmon(&mut r, &["PLANT3K"]);
    r.execute_action(0, own_field_pick(sun.index)).unwrap();
    let v = r.pending_selection_view().expect("hand play pick");
    assert!(v.is_optional, "declinable (PASS via is_optional)");
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[sun.index as usize].is_suspended);
    assert!(!on_field(&r, "PLANT3K"));
}

/// Official Q&A: Sunflowmon's own inherited <Draw 1> is not active while it
/// is the TOP card, so suspending itself with its WD draws nothing extra.
#[test]
fn bt10_048_wd_self_suspend_does_not_fire_own_inherited() {
    let mut r = runner();
    let sun = digivolve_into_sunflowmon(&mut r, &[]);
    let hand = r.hand_size(0);
    r.execute_action(0, own_field_pick(sun.index)).unwrap();
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[sun.index as usize].is_suspended);
    assert_eq!(r.hand_size(0), hand, "top-card inherited text is inactive");
}

// ─── Inherited <Draw 1> ─────────────────────────────────────────────────────

fn suspend_via_effect(r: &mut DebugRunner, player: PlayerId, target: PermanentHandle) {
    let src = r.place_on_field(player, "SRC", None);
    r.register_effect("SRC", Arc::new(SuspendByEffect { target }));
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(src));
    r.game.drain_effect_queue();
}

#[test]
fn bt10_048_inherited_effect_suspend_draws_once_per_turn() {
    let mut r = runner();
    let _host = r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let a1 = r.place_on_field(0, "ALLY-B", Some(0));
    let a2 = r.place_on_field(0, "ALLY-B", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, a1);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1, "<Draw 1>");
    suspend_via_effect(&mut r, 0, a2);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1, "[Once Per Turn]");
}

#[test]
fn bt10_048_inherited_attack_suspend_does_not_draw() {
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    r.game.enter_main_phase();
    let hand = r.hand_size(0);
    let act = encode_attack(host.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[act as usize], 1.0);
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[host.index as usize].is_suspended);
    assert_eq!(r.hand_size(0), hand, "attacking is not 'by an effect'");
}

#[test]
fn bt10_048_inherited_not_on_opponents_turn_or_opp_digimon() {
    let mut r = runner();
    let _host = r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let opp = r.place_on_field(1, "ALLY-B", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, opp);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand, "opponent's Digimon");
    let ally = r.place_on_field(0, "ALLY-B", Some(0));
    r.game.turn_player_idx = 1;
    suspend_via_effect(&mut r, 1, ally);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand, "[Your Turn]");
}
