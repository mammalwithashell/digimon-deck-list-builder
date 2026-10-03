//! BT19-001 Pickmons — Digi-Egg, Lv.2, Red. Traits: Minor, Xros Heart.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT19-001.md`)
//! Inherited Effect:
//! `[When Attacking] [Once Per Turn] By placing 1 Digimon card with the`
//! `[Xros Heart] or [Blue Flare] trait from your hand under any of your`
//! `Tamers, ＜Draw 1＞.`
//!
//! (Not a reprint of BT10-003 — that printing's inherited is an unconditional
//! Xros-Heart-gated Draw 1.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT19/Red/BT19_001.cs
//!   - OnAllyAttack, maxCount 1 ([Once Per Turn]), SetIsInheritedEffect(true).
//!   - CanActivateCondition: carrier is a battle-area Digimon AND the hand
//!     holds a Digimon card with [Xros Heart]/[Blue Flare] AND an own Tamer
//!     exists.
//!   - SelectHandEffect canNoSelect:true (the "by placing" cost is
//!     declinable) → SelectPermanentEffect (own Tamer) canNoSelect:true →
//!     AddDigivolutionCardsBottom → Draw 1. The draw is gated on the
//!     placement having happened.
//!
//! # Patterns
//! - inherited [When Attacking][OPT]
//! - "by placing a hand card under a Tamer, <Draw N>" cost-then-payoff
//!   (ST23-10 idiom: select_hand → select_own_permanent → place_selected_card_under_tamer → draw)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT19-001";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn digimon(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base(hand: &[&str]) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-001 loads")
        .add_card(digimon("CARRIER", &["Mini Dragon"]))
        .add_card(digimon("XH-DIGI", &["Xros Heart"]))
        .add_card(digimon("BF-DIGI", &["Blue Flare"]))
        .add_card(digimon("PLAIN-DIGI", &["Beast"]))
        .add_card(tamer("XH-TAMER-CARD", &["Xros Heart"]))
        .add_card(tamer("TAMER", &[]))
        .add_card(digimon("FILLER", &[]))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .hand(0, hand)
        .memory(5)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r
}

fn fire_attack(r: &mut DebugRunner, carrier: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(carrier));
    r.game.drain_effect_queue();
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default()
}

fn sources_of(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

/// Drive the place-then-draw flow: pick the (only) eligible hand card, then
/// the (only) Tamer.
fn accept_place(r: &mut DebugRunner) {
    let picks = non_pass(r);
    assert_eq!(picks.len(), 1, "exactly one eligible hand card");
    r.execute_action(0, picks[0]).expect("pick hand card");
    let tamers = non_pass(r);
    assert_eq!(tamers.len(), 1, "exactly one Tamer destination");
    r.execute_action(0, tamers[0]).expect("pick Tamer");
    let _ = r.auto_resolve();
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt19_001_structure_is_egg_with_single_inherited_when_attacking_opt() {
    let r = base(&[]);
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Pickmons");
    assert_eq!(card.level, Some(2));
    assert_eq!(card.kind, CompiledCardKind::DigiEgg);
    assert_eq!(card.effects.len(), 1, "only the inherited effect");
    let ok = matches!(&card.effects[0], CompiledClause::Triggered(t)
        if t.scope == CompiledScope::Inherited
        && t.when.contains(&CompiledTiming::WhenAttacking)
        && t.once_per_turn);
    assert!(ok, "inherited [When Attacking][Once Per Turn]");
}

// ─── Section 2/3 — Behaviour ─────────────────────────────────────────────────

#[test]
fn bt19_001_places_xros_heart_digimon_under_tamer_then_draws_1() {
    let mut r = base(&["XH-DIGI", "PLAIN-DIGI"]);
    let tamer = r.place_on_field(0, "TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let hand_before = r.hand_size(0);
    let deck_before = r.deck_size(0);

    fire_attack(&mut r, carrier);
    let view = r.pending_selection_view().expect("hand pick installs");
    assert!(view.is_optional, "the 'by placing' cost is declinable");
    accept_place(&mut r);

    assert!(
        sources_of(&r, tamer).iter().any(|c| c == "XH-DIGI"),
        "the [Xros Heart] Digimon card is placed under the Tamer"
    );
    assert_eq!(r.deck_size(0), deck_before - 1, "<Draw 1>");
    assert_eq!(r.hand_size(0), hand_before, "-1 placed +1 drawn");
    assert!(
        r.game.players[0].hand.iter().any(|c| c.card_id(&r.game.card_data) == "PLAIN-DIGI"),
        "the non-trait card stays in hand"
    );
}

#[test]
fn bt19_001_blue_flare_digimon_is_also_eligible() {
    let mut r = base(&["BF-DIGI"]);
    let tamer = r.place_on_field(0, "TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let deck_before = r.deck_size(0);

    fire_attack(&mut r, carrier);
    accept_place(&mut r);

    assert!(sources_of(&r, tamer).iter().any(|c| c == "BF-DIGI"));
    assert_eq!(r.deck_size(0), deck_before - 1);
}

#[test]
fn bt19_001_no_trait_digimon_in_hand_no_activation() {
    // A [Xros Heart] TAMER card is not a "Digimon card" — ineligible.
    let mut r = base(&["PLAIN-DIGI", "XH-TAMER-CARD"]);
    r.place_on_field(0, "TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let deck_before = r.deck_size(0);
    let hand_before = r.hand_size(0);

    fire_attack(&mut r, carrier);
    let _ = r.auto_resolve();

    assert!(r.game.pending_selection.is_none(), "no eligible card ⇒ no prompt");
    assert_eq!(r.deck_size(0), deck_before, "no draw");
    assert_eq!(r.hand_size(0), hand_before);
}

#[test]
fn bt19_001_no_tamer_no_activation() {
    let mut r = base(&["XH-DIGI"]);
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let deck_before = r.deck_size(0);
    let hand_before = r.hand_size(0);

    fire_attack(&mut r, carrier);
    let _ = r.auto_resolve();

    assert!(r.game.pending_selection.is_none(), "no Tamer ⇒ no prompt");
    assert_eq!(r.deck_size(0), deck_before, "no draw");
    assert_eq!(r.hand_size(0), hand_before, "nothing placed");
}

#[test]
fn bt19_001_declining_the_placement_skips_the_draw() {
    let mut r = base(&["XH-DIGI"]);
    let tamer = r.place_on_field(0, "TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let deck_before = r.deck_size(0);
    let hand_before = r.hand_size(0);

    fire_attack(&mut r, carrier);
    // Decline every prompt.
    for _ in 0..4 {
        let Some(v) = r.pending_selection_view() else { break };
        assert!(v.is_optional, "decline is legal");
        r.execute_action(0, PASS).expect("decline");
    }
    let _ = r.auto_resolve();

    assert_eq!(r.deck_size(0), deck_before, "no placement ⇒ no draw");
    assert_eq!(r.hand_size(0), hand_before);
    assert!(!sources_of(&r, tamer).iter().any(|c| c == "XH-DIGI"));
}

#[test]
fn bt19_001_inherited_only_not_active_as_top_card_effect() {
    // Pickmons itself on top (in the breeding-area sense it would be the
    // egg) — the effect is INHERITED only, so a carrier whose top card is
    // Pickmons has no face-up [When Attacking] from it.
    let mut r = base(&["XH-DIGI"]);
    r.place_on_field(0, "TAMER", Some(0));
    let egg = r.place_on_field(0, CARD_ID, Some(0));
    let deck_before = r.deck_size(0);

    fire_attack(&mut r, egg);
    let _ = r.auto_resolve();

    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.deck_size(0), deck_before);
}

// ─── Section 5 — Once Per Turn ───────────────────────────────────────────────

#[test]
fn bt19_001_once_per_turn_lockout() {
    let mut r = base(&["XH-DIGI", "BF-DIGI"]);
    r.place_on_field(0, "TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);

    fire_attack(&mut r, carrier);
    // First activation — pick any eligible card + the Tamer.
    let picks = non_pass(&r);
    assert_eq!(picks.len(), 2, "both trait Digimon are eligible");
    r.execute_action(0, picks[0]).unwrap();
    let t = non_pass(&r);
    r.execute_action(0, t[0]).unwrap();
    let _ = r.auto_resolve();
    let deck_after_first = r.deck_size(0);

    fire_attack(&mut r, carrier);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "second activation gated by OPT");
    assert_eq!(r.deck_size(0), deck_after_first, "no second draw this turn");

    // Lockout clears: back to player 0's next turn, re-arm the hand, fire.
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0, "back on player 0's turn");
    r.add_to_hand(0, "XH-DIGI");
    fire_attack(&mut r, carrier);
    let view = r
        .pending_selection_view()
        .expect("OPT lock cleared — the hand prompt is offered again next turn");
    assert!(view.is_optional);
    assert!(!non_pass(&r).is_empty(), "an eligible hand card is offered");
}

#[test]
fn bt19_001_declining_does_not_consume_opt() {
    // DCGO's ActivateClass is optional: declining must not spend the
    // [Once Per Turn] use, so a second attack the same turn can still use it.
    let mut r = base(&["XH-DIGI"]);
    r.place_on_field(0, "TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let deck_before = r.deck_size(0);

    fire_attack(&mut r, carrier);
    assert!(r.pending_is_optional());
    r.execute_action(0, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.deck_size(0), deck_before, "declined: no draw");

    fire_attack(&mut r, carrier);
    let view = r
        .pending_selection_view()
        .expect("declined activation does not consume OPT — prompt offered again");
    assert!(view.is_optional);
    accept_place(&mut r);
    assert_eq!(r.deck_size(0), deck_before - 1, "second attack places + draws");
}

// ─── Integrated — real attack declaration ────────────────────────────────────

#[test]
fn bt19_001_real_attack_places_and_draws() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-001 loads")
        .add_card(digimon("CARRIER", &["Mini Dragon"]))
        .add_card(digimon("XH-DIGI", &["Xros Heart"]))
        .add_card(tamer("TAMER", &[]))
        .add_card(digimon("FILLER", &[]))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .security(1, &["FILLER"; 3])
        .hand(0, &["XH-DIGI"])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    let tamer = r.place_on_field(0, "TAMER", Some(0));
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let deck_before = r.deck_size(0);

    r.attack_player(carrier, 1, false);
    let view = r
        .pending_selection_view()
        .expect("[When Attacking] hand pick surfaces on a real attack");
    assert!(view.is_optional);
    accept_place(&mut r);
    let _ = r.auto_resolve();

    assert!(
        sources_of(&r, tamer).iter().any(|c| c == "XH-DIGI"),
        "the [Xros Heart] Digimon card is under the Tamer"
    );
    assert_eq!(r.deck_size(0), deck_before - 1, "<Draw 1>");
}
