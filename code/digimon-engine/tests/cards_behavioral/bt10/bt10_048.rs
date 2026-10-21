//! BT10-048 Sunflowmon — Digimon, Lv.4, Green, Cost 4, DP 3000.
//! Traits: Vegetation. Form: Champion. Attribute: Data.
//! Digivolve: green Lv.3, cost 2 (the only circle on the card image and in
//! DCGO's BT10_048.asset; cards.json + the official mirror also list a phantom
//! green Lv.3 / cost 0 circle — see the YAML header).
//!
//! # Card text (official Bandai DB — data/card_bundles/BT10-048.md; card image agrees)
//!
//! ```text
//! [When Digivolving] By suspending 1 of your green Digimon, you may play 1
//! Digimon card with [Vegetation], [Plant], or [Fairy] in one of its traits
//! and 3000 DP or less from your hand without paying its play cost.
//! Inherited: [Your Turn][Once Per Turn] When an effect suspends one of your
//! Digimon, <Draw 1>. (Draw 1 card from your deck.)
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT10/Green/BT10_048.cs
//!   - WD (OnEnterFieldAnyone + CanTriggerWhenDigivolving): isOptional; usable
//!     only while an own green Digimon can pay a suspend cost. SelectPermanent
//!     (own green Digimon, canNoSelect) → SuspendPermanentsClass by THIS effect
//!     → only if it really became suspended: SelectHand (canNoSelect) over
//!     Digimon with Plant/Fairy traits (HasPlantTraits = Vegetation or Plant
//!     substring) and DP <= 3000 that can be played → PlayPermanentCards free.
//!   - Inherited (OnTappedAnyone): IsOwnerTurn, own battle-area Digimon,
//!     IsByEffect, maxCountPerTurn 1 → Draw 1. isOptional false.
//!
//! # Rules
//! - §15-7-1/-2: "By suspending ..." is an optional processing condition; if
//!   it is not executed, the play after it can't be executed.
//! - §15-7-4: it can't be chosen when no green Digimon of yours can suspend.
//! - §15-7-5: it can be executed even when no hand card could be played.
//! - Q1974: a suspension made before a digivolution completes can't trigger the
//!   new stack's inherited effects; conversely Sunflowmon's own inherited text is
//!   inactive while it is the top card.
//!
//! # Patterns this test covers
//! - real digivolve flow (`encode_digivolve` through the action mask) → WD
//! - optional own-permanent suspend cost, decline paths, filter positives/negatives
//! - gated free play from hand (trait substring, DP cap, Digimon-only)
//! - inherited on_suspend <Draw 1> (`trigger_if: event_is_effect_initiated`), OPT

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledCost, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, ATTACK_START, PASS, PLAY_HAND_START, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
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

fn tamer(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Green];
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
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
        .add_card(digimon("VEG1K", CardColor::Red, 3, 1000, &["Vegetation"]))
        .add_card(digimon("VEGBIG", CardColor::Green, 4, 4000, &["Vegetation"]))
        .add_card(digimon("BEAST", CardColor::Green, 3, 2000, &["Beast"]))
        .add_card(tamer("PLANT-TAMER", &["Plant"]))
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

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

/// Digivolve `base` into Sunflowmon from hand through the real action mask.
fn digivolve_into_sunflowmon(r: &mut DebugRunner, base: PermanentHandle, extra: &[&str]) {
    r.add_to_hand(0, CARD_ID);
    for id in extra {
        r.add_to_hand(0, id);
    }
    r.game.enter_main_phase();
    r.game.set_memory(5);
    let act = encode_digivolve(hand_idx(r, CARD_ID) as u16, base.index as u16);
    assert_eq!(
        build_action_mask(&r.game, 0)[act as usize],
        1.0,
        "green Lv.3 → Sunflowmon digivolve must be legal"
    );
    r.game.decode_action(act, 0);
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
fn bt10_048_metadata_and_single_green_lv3_cost2_circle() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Sunflowmon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.traits, vec!["Vegetation".to_string()]);
    assert_eq!(card.attribute, ["Data"]);
    assert_eq!(card.alt_paths.len(), 1, "only the printed green Lv.3 circle");
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(2)));
}

#[test]
fn bt10_048_has_optional_wd_and_inherited_opt_on_suspend_clauses() {
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
    assert_eq!(t[0].when, vec![CompiledTiming::WhenDigivolving]);
    assert_ne!(t[0].scope, CompiledScope::Inherited);
    assert!(t[0].optional, "'By suspending ..., you may play' is optional");
    assert!(!t[0].once_per_turn);
    assert_eq!(t[1].scope, CompiledScope::Inherited);
    assert_eq!(t[1].when, vec![CompiledTiming::OnSuspend]);
    assert!(t[1].once_per_turn);
    assert!(!t[1].optional, "<Draw 1> is mandatory");
}

// ─── SECTION 2 — [When Digivolving] ─────────────────────────────────────────

#[test]
fn bt10_048_wd_suspend_self_then_play_matching_digimon_free() {
    let mut r = runner();
    let sun = r.place_on_field(0, "ROOKIE", Some(0));
    digivolve_into_sunflowmon(
        &mut r,
        sun,
        &["PLANT3K", "FAIRY2K", "VEG1K", "VEGBIG", "BEAST", "PLANT-TAMER"],
    );
    assert_eq!(r.memory(), 3, "digivolution cost 2 paid");

    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(v.is_optional, "the 'by suspending' condition is declinable");
    assert!(v.valid_action_ids.contains(&own_field_pick(sun)));
    r.execute_action(0, own_field_pick(sun)).unwrap();
    assert!(suspended(&r, sun), "Sunflowmon suspended itself (Q: any green Digimon)");

    let v = r.pending_selection_view().expect("hand play pick");
    assert_eq!(v.kind, SelectionKind::Hand);
    assert!(v.is_optional, "you MAY play");
    assert!(v.valid_action_ids.contains(&hand_pick(&r, "PLANT3K")), "[Plant], 3000 DP");
    assert!(v.valid_action_ids.contains(&hand_pick(&r, "FAIRY2K")), "[Fairy], any colour");
    assert!(v.valid_action_ids.contains(&hand_pick(&r, "VEG1K")), "[Vegetation]");
    assert!(!v.valid_action_ids.contains(&hand_pick(&r, "VEGBIG")), "DP 4000 > 3000");
    assert!(!v.valid_action_ids.contains(&hand_pick(&r, "BEAST")), "no matching trait");
    assert!(
        !v.valid_action_ids.contains(&hand_pick(&r, "PLANT-TAMER")),
        "only Digimon cards"
    );
    r.execute_action(0, hand_pick(&r, "PLANT3K")).unwrap();
    let _ = r.auto_resolve();
    assert!(on_field(&r, "PLANT3K"), "played");
    assert_eq!(r.memory(), 3, "played without paying its play cost");
}

#[test]
fn bt10_048_wd_can_suspend_another_green_digimon_but_not_a_blue_one() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY-G", Some(0));
    let blue = r.place_on_field(0, "ALLY-B", Some(0));
    let sun = r.place_on_field(0, "ROOKIE", Some(0));
    digivolve_into_sunflowmon(&mut r, sun, &["FAIRY2K"]);

    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert!(v.valid_action_ids.contains(&own_field_pick(ally)));
    assert!(v.valid_action_ids.contains(&own_field_pick(sun)));
    assert!(
        !v.valid_action_ids.contains(&own_field_pick(blue)),
        "a non-green Digimon can't pay the cost"
    );
    r.execute_action(0, own_field_pick(ally)).unwrap();
    assert!(suspended(&r, ally));
    assert!(!suspended(&r, sun));
    r.execute_action(0, hand_pick(&r, "FAIRY2K")).unwrap();
    let _ = r.auto_resolve();
    assert!(on_field(&r, "FAIRY2K"));
}

#[test]
fn bt10_048_wd_already_suspended_green_digimon_is_not_offered() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY-G", Some(0));
    r.game.players[0].battle_area[ally.index as usize].is_suspended = true;
    let sun = r.place_on_field(0, "ROOKIE", Some(0));
    digivolve_into_sunflowmon(&mut r, sun, &["FAIRY2K"]);
    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert!(!v.valid_action_ids.contains(&own_field_pick(ally)), "§15-7-4");
    assert!(v.valid_action_ids.contains(&own_field_pick(sun)));
}

#[test]
fn bt10_048_wd_no_payable_green_digimon_installs_no_prompt() {
    let mut r = runner();
    r.place_on_field(0, "ALLY-B", Some(0));
    let sun = r.place_on_field(0, "ROOKIE", Some(0));
    // The base is suspended, so the digivolved Sunflowmon stays suspended.
    r.game.players[0].battle_area[sun.index as usize].is_suspended = true;
    digivolve_into_sunflowmon(&mut r, sun, &["PLANT3K"]);
    assert!(
        r.pending_selection_view().is_none(),
        "§15-7-4: no unsuspended green Digimon → the condition can't be chosen"
    );
    assert!(!on_field(&r, "PLANT3K"));
}

#[test]
fn bt10_048_wd_declining_the_suspend_plays_nothing() {
    let mut r = runner();
    let sun = r.place_on_field(0, "ROOKIE", Some(0));
    digivolve_into_sunflowmon(&mut r, sun, &["PLANT3K"]);
    let v = r.pending_selection_view().expect("suspend-cost pick");
    assert!(v.is_optional);
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(!suspended(&r, sun));
    assert!(!on_field(&r, "PLANT3K"), "§15-7-2: no suspend → no play");
}

#[test]
fn bt10_048_wd_suspend_then_decline_the_play() {
    let mut r = runner();
    let sun = r.place_on_field(0, "ROOKIE", Some(0));
    digivolve_into_sunflowmon(&mut r, sun, &["PLANT3K"]);
    r.execute_action(0, own_field_pick(sun)).unwrap();
    let v = r.pending_selection_view().expect("hand play pick");
    assert!(v.is_optional);
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, sun), "the paid suspend stands");
    assert!(!on_field(&r, "PLANT3K"));
}

/// §15-7-5: the suspend may be executed even when nothing can be played.
#[test]
fn bt10_048_wd_suspend_allowed_without_a_playable_hand_card() {
    let mut r = runner();
    let sun = r.place_on_field(0, "ROOKIE", Some(0));
    digivolve_into_sunflowmon(&mut r, sun, &["BEAST", "VEGBIG"]);
    let v = r.pending_selection_view().expect("suspend-cost pick still offered");
    assert!(v.valid_action_ids.contains(&own_field_pick(sun)));
    r.execute_action(0, own_field_pick(sun)).unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, sun));
    assert!(!on_field(&r, "BEAST"));
    assert!(!on_field(&r, "VEGBIG"));
}

/// Sunflowmon's own inherited text is inactive while it is the top card.
#[test]
fn bt10_048_wd_self_suspend_does_not_fire_its_own_inherited_draw() {
    let mut r = runner();
    let sun = r.place_on_field(0, "ROOKIE", Some(0));
    digivolve_into_sunflowmon(&mut r, sun, &[]);
    let hand = r.hand_size(0);
    r.execute_action(0, own_field_pick(sun)).unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, sun));
    assert_eq!(r.hand_size(0), hand, "top-card inherited text is inactive");
}

/// A second Sunflowmon buried in the stack: the WD suspend (an effect) of the
/// new top card fires the buried copy's inherited <Draw 1>.
#[test]
fn bt10_048_wd_suspend_triggers_a_buried_sunflowmons_inherited_draw() {
    let mut r = runner();
    let base = r.place_stack(0, &[CARD_ID, "ROOKIE"]);
    digivolve_into_sunflowmon(&mut r, base, &[]);
    let hand = r.hand_size(0);
    r.execute_action(0, own_field_pick(base)).unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, base));
    assert_eq!(r.hand_size(0), hand + 1, "buried Sunflowmon's inherited <Draw 1>");
}

// ─── SECTION 3 — Inherited <Draw 1> ────────────────────────────────────────

#[test]
fn bt10_048_inherited_effect_suspend_of_own_digimon_draws_1() {
    let mut r = runner();
    r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let a1 = r.place_on_field(0, "ALLY-B", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, a1);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1, "<Draw 1>");
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
    assert!(suspended(&r, host));
    assert_eq!(r.hand_size(0), hand, "attacking is not 'by an effect'");
}

#[test]
fn bt10_048_inherited_opponent_digimon_suspend_does_not_draw() {
    let mut r = runner();
    r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let opp = r.place_on_field(1, "ALLY-B", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, opp);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand, "not one of YOUR Digimon");
}

#[test]
fn bt10_048_inherited_does_not_draw_on_opponents_turn() {
    let mut r = runner();
    r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let ally = r.place_on_field(0, "ALLY-B", Some(0));
    r.game.turn_player_idx = 1;
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 1, ally);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand, "[Your Turn]");
}

#[test]
fn bt10_048_inherited_once_per_turn() {
    let mut r = runner();
    r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let a1 = r.place_on_field(0, "ALLY-B", Some(0));
    let a2 = r.place_on_field(0, "ALLY-B", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, a1);
    let _ = r.auto_resolve();
    suspend_via_effect(&mut r, 0, a2);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1, "[Once Per Turn]");
}

#[test]
fn bt10_048_inherited_once_per_turn_resets_next_own_turn() {
    let mut r = runner();
    r.place_stack(0, &[CARD_ID, "ALLY-G"]);
    let a1 = r.place_on_field(0, "ALLY-B", Some(0));
    suspend_via_effect(&mut r, 0, a1);
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0);
    let a2 = r.place_on_field(0, "ALLY-B", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, a2);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1, "lockout clears");
}
