//! EX4-002 Kokomon — Digi-Egg (In-Training), Lv.2, Green, no DP.
//! Traits: Lesser. Form: In-Training.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX4-002.md; card image agrees) — INHERITED ONLY
//!
//! ```text
//! Inherited: [Your Turn][Once Per Turn] When an effect suspends one of your
//! Digimon, <Draw 1>. (Draw 1 card from your deck.)
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX4/Green/EX4_002.cs
//!   OnTappedAnyone, SetIsInheritedEffect, IsOwnerTurn, maxCountPerTurn 1,
//!   CanTriggerWhenPermanentSuspends(own battle-area Digimon) +
//!   IsByEffect(hashtable, null) → DrawClass(owner, 1). isOptional false.
//!
//! # Official Q&A (data/card_qa.json)
//! - Q3438: an "attack with this Digimon" effect suspends by the ATTACK, not by
//!   an effect — Kokomon does not activate. Likewise a plain attack declaration.
//!
//! # Patterns this test covers
//! - inherited on_suspend observer with `trigger_if: event_is_effect_initiated`
//!   (the EX6-004 Kokomon shape, G-SUSPEND-EFFECT-INITIATED)
//! - [Your Turn] gate, "one of YOUR Digimon" gate, attack-suspend negative
//! - [Once Per Turn] lockout + reset on the next own turn

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
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
        .add_card(make_digimon("HOST", CardColor::Green, 4000))
        .add_card(make_digimon("ALLY", CardColor::Green, 3000))
        .add_card(make_test_card("SRC", "Src"))
        .add_card(make_digimon("FILLER", CardColor::Blue, 3000))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .security(0, &["FILLER"; 5])
        .security(1, &["FILLER"; 5])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

/// Host Digimon for `player` with Kokomon underneath as a digivolution card.
fn host_with_egg(r: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    r.place_stack(player, &[CARD_ID, "HOST"])
}

/// Fire `SuspendByEffect { target }` from an SRC permanent owned by `player`.
fn suspend_via_effect(r: &mut DebugRunner, player: PlayerId, target: PermanentHandle) {
    let src = r.place_on_field(player, "SRC", None);
    r.register_effect("SRC", Arc::new(SuspendByEffect { target }));
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(src));
    r.game.drain_effect_queue();
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn ex4_002_metadata_is_green_lv2_egg_without_dp() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Kokomon");
    assert_eq!(card.level, Some(2));
    assert_eq!(card.dp, None, "an In-Training egg has no DP");
    assert_eq!(card.traits, vec!["Lesser".to_string()]);
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Green));
}

#[test]
fn ex4_002_has_single_mandatory_inherited_opt_on_suspend_clause() {
    let card = dsl_card_data::compiled(CARD_ID);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(card.effects.len(), 1, "only the inherited clause");
    assert_eq!(triggered.len(), 1);
    let t = triggered[0];
    assert_eq!(t.scope, CompiledScope::Inherited);
    assert_eq!(t.when, vec![CompiledTiming::OnSuspend]);
    assert!(t.once_per_turn, "[Once Per Turn]");
    assert!(!t.optional, "<Draw 1> is mandatory");
}

// ─── SECTION 2 — Behavioral + condition gating ─────────────────────────────

#[test]
fn ex4_002_effect_suspend_of_another_own_digimon_draws_1() {
    let mut r = runner();
    host_with_egg(&mut r, 0);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let hand = r.hand_size(0);
    let deck = r.deck_size(0);
    suspend_via_effect(&mut r, 0, ally);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1, "<Draw 1>");
    assert_eq!(r.deck_size(0), deck - 1, "drawn from the deck");
}

#[test]
fn ex4_002_effect_suspend_of_the_carrier_itself_draws_1() {
    let mut r = runner();
    let host = host_with_egg(&mut r, 0);
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, host);
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[host.index as usize].is_suspended);
    assert_eq!(r.hand_size(0), hand + 1, "the carrier is 'one of your Digimon'");
}

/// Q3438: suspending to attack is not "an effect suspends".
#[test]
fn ex4_002_attack_declaration_suspend_does_not_draw() {
    let mut r = runner();
    let host = host_with_egg(&mut r, 0);
    r.game.enter_main_phase();
    let hand = r.hand_size(0);
    let act = encode_attack(host.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[act as usize], 1.0, "attack legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[host.index as usize].is_suspended);
    assert_eq!(r.hand_size(0), hand, "attack suspension is not 'by an effect'");
}

#[test]
fn ex4_002_non_effect_suspend_does_not_draw() {
    let mut r = runner();
    host_with_egg(&mut r, 0);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let hand = r.hand_size(0);
    r.game.suspend(ally);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand);
}

#[test]
fn ex4_002_opponent_digimon_effect_suspend_does_not_draw() {
    let mut r = runner();
    host_with_egg(&mut r, 0);
    let opp = r.place_on_field(1, "ALLY", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, opp);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand, "not one of YOUR Digimon");
}

#[test]
fn ex4_002_does_not_draw_on_opponents_turn() {
    let mut r = runner();
    host_with_egg(&mut r, 0);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.turn_player_idx = 1;
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 1, ally);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand, "[Your Turn] gate");
}

#[test]
fn ex4_002_no_draw_when_kokomon_is_not_a_digivolution_card() {
    let mut r = runner();
    r.place_on_field(0, "HOST", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, ally);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand, "inherited text needs Kokomon as a source");
}

// ─── SECTION 3 — OPT lockout ────────────────────────────────────────────────

#[test]
fn ex4_002_once_per_turn_blocks_second_draw_same_turn() {
    let mut r = runner();
    host_with_egg(&mut r, 0);
    let a1 = r.place_on_field(0, "ALLY", Some(0));
    let a2 = r.place_on_field(0, "ALLY", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, a1);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1);
    suspend_via_effect(&mut r, 0, a2);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1, "[Once Per Turn]: no second draw");
}

#[test]
fn ex4_002_once_per_turn_resets_next_own_turn() {
    let mut r = runner();
    host_with_egg(&mut r, 0);
    let a1 = r.place_on_field(0, "ALLY", Some(0));
    suspend_via_effect(&mut r, 0, a1);
    let _ = r.auto_resolve();

    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0, "back on P0's turn");

    let a2 = r.place_on_field(0, "ALLY", Some(0));
    let hand = r.hand_size(0);
    suspend_via_effect(&mut r, 0, a2);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand + 1, "lockout clears on the next own turn");
}
