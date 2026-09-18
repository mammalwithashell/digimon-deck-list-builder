//! BT7-056 Dorumon — Digimon, Lv.3, Black, DP 1000, Cost 3.
//! Traits: Beast / X Antibody. Form: Rookie. Attribute: Data.
//! Standard digivolve (official Bandai DB): Black Lv.2 / cost 0.
//!
//! # Card text (card image BT7-056.webp — verbatim; the official Bandai DB
//! bundle data/card_bundles/BT7-056.md carries the re-worded reprint text
//! with identical semantics)
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with
//! [X-Antibody] in its traits and 1 [Kota Domoto] among them to your hand.
//! Place the remaining cards at the bottom of your deck in any order.
//! Inherited [Your Turn] [Once Per Turn] When one of your effects places a
//! digivolution card under this Digimon, gain 1 memory.
//! ```
//!
//! Official Q&A: "Yes. Even if you revealed only a card with [X Antibody] in
//! its traits or [Kota Domoto], you still add that card to your hand."
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT7/Black/BT7_056.cs
//! - [On Play] (OnEnterFieldAnyone, isOptional: false; CanActivate = deck ≥ 1):
//!   `SimplifiedRevealDeckTopCardsAndSelect(revealCount 3, TWO conditions
//!   each maxCount 1 / Mode.AddHand: `HasXAntibodyTraits` and
//!   `CardNames.Contains("Kota Domoto")`; remainingCardsPlace: DeckBottom)`.
//! - Inherited (OnAddDigivolutionCards, SetIsInheritedEffect, OPT, mandatory):
//!   CanUse = IsExistOnBattleArea && IsOwnerTurn &&
//!   `CanTriggerOnAddDigivolutionCard(permanent == self,
//!   cardEffect.EffectSourceCard.Owner == card.Owner, cardCondition: null)`
//!   → `AddMemory(1)`.
//!
//! # Rules
//! - `general_rule.pdf` 15-5-3 (p.23-24) — whose worked example IS this
//!   card's inherited text: the trigger is met even when Dorumon ITSELF is
//!   the card placed under a Digimon by one of your effects.
//! - 15-5-2: several cards placed at once by one effect → ONE trigger.
//!
//! # DSL YAML
//! code/digimon-engine/cards/bt7/BT7-056.yaml
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A2: two-bucket reveal (reveal 3, add ≤1 per bucket, bottom the rest)
//! - G4: inherited triggered clause, [Your Turn] gate, OPT lockout (E1)
//! - `on_add_digivolution_cards` + `event_host_permanent_is_source` +
//!   `event_caused_by_own_effect` (G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT7-056";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

/// A Digimon with the [X Antibody] trait (bucket 1 candidate).
fn x_antibody_digimon(id: &str) -> CardData {
    let mut c = digimon(id, 4);
    c.traits = vec!["Beast".to_string(), "X Antibody".to_string()];
    c
}

/// A Tamer named "Kota Domoto" (bucket 2 candidate).
fn kota(id: &str) -> CardData {
    let mut c = make_test_card(id, "Kota Domoto");
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Black];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Green];
    c
}

/// The CAUSE: an [On Play] effect that places the first `count` cards of its
/// controller's hand under the permanent at `target`.
struct PlaceHandUnder {
    count: usize,
    target: PermanentHandle,
}
impl CardEffect for PlaceHandUnder {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.count;
        let target = self.target;
        vec![Effect::on_play(card)
            .name("test placer: tuck hand cards under a permanent")
            .process(move |ctx| {
                for _ in 0..count {
                    let ok =
                        ctx.place_as_bottom_source(CardSourceRef::Hand(ctx.player, 0), target, false);
                    assert!(ok, "placement must succeed");
                }
            })
            .build()]
    }
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT7-056 YAML loads from the embedded pack")
        .add_card(digimon("HOST", 4))
        .add_card(digimon("SIB", 3))
        .add_card(digimon("PLACER", 3))
        .add_card(digimon("OPP-PLACER", 3))
        .add_card(x_antibody_digimon("XA"))
        .add_card(kota("KOTA"))
        .add_card(digimon("PLAIN", 3))
        .add_card(filler("F"))
        .deck(1, &["F"; 10])
        .memory(10)
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

fn reveal_action(runner: &DebugRunner, card_id: &str) -> u16 {
    let idx = runner
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not revealed"));
    SEL_REVEAL_START + idx as u16
}

/// Player 0's turn; `HOST` on player 0's slot 0 with Dorumon as its sole
/// digivolution source.
fn host_with_dorumon(r: &mut DebugRunner) -> PermanentHandle {
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    // Below the +10 cap so a gain is observable.
    r.game.set_memory(5);
    r.place_stack(0, &[CARD_ID, "HOST"])
}

fn tuck(r: &mut DebugRunner, player: PlayerId, placer_id: &str, count: usize, target: PermanentHandle) {
    r.register_effect(placer_id, Arc::new(PlaceHandUnder { count, target }));
    let placer = r.place_on_field(player, placer_id, Some(0));
    r.game.fire_on_play(player, placer.index as usize);
    r.auto_resolve().expect("no selection expected from a mandatory gain");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt7_056_metadata_and_black_lv2_digivolve_circle() {
    let r = base().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.level, Some(3));
    assert_eq!(c.cost, Some(3));
    assert_eq!(c.dp, Some(1000));
    assert!(c.traits.iter().any(|t| t == "X Antibody"), "traits={:?}", c.traits);
    assert!(c.traits.iter().any(|t| t == "Beast"));
    let circles = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(circles, 1, "one printed circle: Black Lv.2 / 0");
}

#[test]
fn bt7_056_has_mandatory_on_play_and_inherited_opt_your_turn_add_sources_clause() {
    let r = base().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(c.effects.len(), 2, "[On Play] + inherited");
    assert_eq!(triggered.len(), 2);

    let on_play = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnPlay])
        .expect("[On Play] clause");
    assert_eq!(on_play.scope, CompiledScope::FaceUp);
    assert!(!on_play.optional, "the reveal is mandatory");
    assert!(!on_play.once_per_turn);

    let inh = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnAddDigivolutionCards])
        .expect("inherited OnAddDigivolutionCards clause");
    assert_eq!(inh.scope, CompiledScope::Inherited);
    assert!(inh.once_per_turn, "[Once Per Turn]");
    assert!(!inh.optional, "gain 1 memory is mandatory (DCGO isOptional: false)");
    assert_eq!(
        inh.active_when.as_ref().and_then(|g| g.your_turn),
        Some(true),
        "[Your Turn]"
    );
    let leaves = &inh.condition.as_ref().expect("condition").all_of;
    assert!(
        leaves.iter().any(|p| p.event_host_permanent_is_source == Some(true)),
        "\"under this Digimon\" → host == self"
    );
    assert!(
        leaves.iter().any(|p| p.event_caused_by_own_effect == Some(true)),
        "\"one of YOUR effects\" → placing effect belongs to the controller"
    );
    assert!(
        !leaves.iter().any(|p| p.event_added_card_any.is_some()),
        "any digivolution card qualifies (DCGO cardCondition: null)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2/3 — [On Play] reveal 3 / two buckets / bottom the rest
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt7_056_on_play_adds_one_x_antibody_card_and_one_kota_domoto_bottoms_rest() {
    // Deck (last = top): F, F | F, KOTA, XA ← top 3.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["F", "F", "F", "KOTA", "XA"])
        .start();
    let deck_before = r.deck_size(0);
    r.play(0, 0).expect("play Dorumon");

    let view = r.pending_selection_view().expect("bucket 0 prompt");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { bucket_index: 0, .. }));
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "a candidate exists → the add is mandatory (Q&A: you still add it)"
    );
    r.execute_action(0, reveal_action(&r, "XA")).expect("pick XA");
    let view = r.pending_selection_view().expect("bucket 1 prompt");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { bucket_index: 1, .. }));
    assert!(!view.valid_action_ids.contains(&PASS));
    r.execute_action(0, reveal_action(&r, "KOTA")).expect("pick KOTA");
    r.auto_resolve().expect("finish");

    let hand = zone_ids(&r.game.players[0].hand, &r.game.card_data);
    assert!(hand.contains(&"XA".to_string()), "X Antibody card added; hand={hand:?}");
    assert!(hand.contains(&"KOTA".to_string()), "Kota Domoto added; hand={hand:?}");
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(r.deck_size(0), deck_before - 2, "1 of the 3 revealed returned to the deck");
    assert_eq!(
        r.game.players[0].deck[0].card_id(&r.game.card_data),
        "F",
        "the rest go to the BOTTOM of the deck"
    );
    assert!(r.game.revealed_cards.is_empty());
    assert_eq!(r.memory(), 10 - 3, "only the play cost was paid");
}

/// Q&A: only ONE bucket has a candidate → that card is still added; the
/// other bucket is skipped (no dead prompt).
#[test]
fn bt7_056_on_play_adds_only_the_kota_when_no_x_antibody_revealed() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["F", "F", "PLAIN", "F", "KOTA"])
        .start();
    r.play(0, 0).expect("play Dorumon");
    let view = r.pending_selection_view().expect("bucket 1 prompt (bucket 0 skipped: no X Antibody)");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { bucket_index: 1, .. }));
    r.execute_action(0, reveal_action(&r, "KOTA")).expect("pick KOTA");
    r.auto_resolve().expect("finish");
    let hand = zone_ids(&r.game.players[0].hand, &r.game.card_data);
    assert_eq!(hand, vec!["KOTA".to_string()]);
    assert!(r.game.revealed_cards.is_empty());
}

#[test]
fn bt7_056_on_play_no_match_bottoms_all_three() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["F", "F", "PLAIN", "F", "F"])
        .start();
    let deck_before = r.deck_size(0);
    r.play(0, 0).expect("play Dorumon");
    r.auto_resolve().expect("finish");
    assert_eq!(r.hand_size(0), 0, "nothing qualifies");
    assert_eq!(r.deck_size(0), deck_before, "all 3 revealed cards returned");
    assert!(r.game.revealed_cards.is_empty());
    assert!(r.game.pending_selection.is_none());
}

/// A plain Digimon is neither an [X Antibody] card nor a [Kota Domoto] —
/// bucket 1 has no legal pick, so only XA is offered/added.
#[test]
fn bt7_056_buckets_reject_non_matching_cards() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["F", "F", "F", "PLAIN", "XA"])
        .start();
    r.play(0, 0).expect("play Dorumon");
    let view = r.pending_selection_view().expect("bucket 0 prompt");
    assert_eq!(view.valid_action_ids, vec![reveal_action(&r, "XA")], "only XA is a bucket-0 pick");
    r.execute_action(0, reveal_action(&r, "XA")).expect("pick XA");
    r.auto_resolve().expect("finish");
    let hand = zone_ids(&r.game.players[0].hand, &r.game.card_data);
    assert_eq!(hand, vec!["XA".to_string()], "PLAIN must not be added");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2/3 — Inherited: your effect places a digivolution card under this
// Digimon → gain 1 memory
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt7_056_gains_1_memory_when_own_effect_places_a_card_under_host() {
    let mut r = base().hand(0, &["PLAIN"]).deck(0, &["F"; 10]).start();
    let host = host_with_dorumon(&mut r);
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, host);
    assert_eq!(r.memory(), before + 1, "+1 memory");
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        3
    );
}

/// Rule 15-5-3 (the manual's own example): Dorumon ITSELF being placed under
/// one of your Digimon by your effect meets its inherited trigger.
#[test]
fn bt7_056_gains_when_it_is_itself_placed_under_a_digimon_by_your_effect() {
    let mut r = base().hand(0, &[CARD_ID]).deck(0, &["F"; 10]).start();
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    r.game.set_memory(5);
    let host = r.place_on_field(0, "HOST", Some(0));
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, host);
    assert_eq!(r.memory(), before + 1, "15-5-3: the placed card's own inherited observer fires");
    let ids = zone_ids(
        &r.game.players[0].battle_area[host.index as usize].card_sources,
        &r.game.card_data,
    );
    assert_eq!(ids, vec![CARD_ID.to_string(), "HOST".to_string()]);
}

/// NEGATIVE (owner gate): the OPPONENT's effect placing a card under your
/// Digimon on your turn does NOT trigger "one of YOUR effects".
#[test]
fn bt7_056_no_gain_when_opponent_effect_places_under_host() {
    let mut r = base().hand(1, &["PLAIN"]).deck(0, &["F"; 10]).start();
    let host = host_with_dorumon(&mut r); // player 0's turn
    let before = r.memory();
    tuck(&mut r, 1, "OPP-PLACER", 1, host);
    assert_eq!(r.memory(), before, "the placing effect must be the controller's own");
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        3,
        "the card was still placed"
    );
}

/// NEGATIVE ([Your Turn]): your own effect on the opponent's turn (e.g. from
/// a counter window) does not trigger.
#[test]
fn bt7_056_no_gain_on_opponents_turn() {
    let mut r = base().hand(0, &["PLAIN"]).deck(0, &["F"; 10]).start();
    let host = host_with_dorumon(&mut r);
    r.game.turn_player_idx = 1;
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, host);
    assert_eq!(r.memory(), before, "[Your Turn] gate");
}

/// NEGATIVE (host gate): a card placed under a SIBLING → no gain.
#[test]
fn bt7_056_no_gain_when_card_is_placed_under_a_sibling() {
    let mut r = base().hand(0, &["PLAIN"]).deck(0, &["F"; 10]).start();
    let _host = host_with_dorumon(&mut r);
    let sib = r.place_on_field(0, "SIB", Some(0));
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, sib);
    assert_eq!(r.memory(), before, "\"under this Digimon\" only");
}

/// NEGATIVE (inherited only): a face-up Dorumon receiving a card under it
/// gains nothing — the trigger is an inherited effect.
#[test]
fn bt7_056_face_up_dorumon_does_not_gain() {
    let mut r = base().hand(0, &["PLAIN"]).deck(0, &["F"; 10]).start();
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    r.game.set_memory(5);
    let doru = r.place_on_field(0, CARD_ID, Some(0));
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, doru);
    assert_eq!(r.memory(), before, "inherited effects only work from the digivolution cards");
}

/// Rule 15-5-2 batch: two cards tucked by ONE effect → ONE gain.
#[test]
fn bt7_056_multi_card_placement_in_one_effect_gains_once() {
    let mut r = base().hand(0, &["PLAIN", "F"]).deck(0, &["F"; 10]).start();
    let host = host_with_dorumon(&mut r);
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 2, host);
    assert_eq!(r.memory(), before + 1, "one trigger condition → one activation (15-5-2)");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — [Once Per Turn]
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt7_056_second_placement_in_the_same_turn_is_locked_out() {
    let mut r = base().hand(0, &["PLAIN", "PLAIN"]).deck(0, &["F"; 10]).start();
    let host = host_with_dorumon(&mut r);
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, host);
    assert_eq!(r.memory(), before + 1);
    tuck(&mut r, 0, "SIB", 1, host);
    assert_eq!(r.memory(), before + 1, "[Once Per Turn]: no second gain this turn");
}

#[test]
fn bt7_056_lockout_clears_on_your_next_turn() {
    let mut r = base().hand(0, &["PLAIN", "PLAIN"]).deck(0, &["F"; 10]).start();
    let host = host_with_dorumon(&mut r);
    tuck(&mut r, 0, "PLACER", 1, host);
    r.end_turn();
    r.end_turn();
    assert_eq!(r.turn_player(), 0);
    r.game.set_memory(5);
    let before = r.memory();
    tuck(&mut r, 0, "SIB", 1, host);
    assert_eq!(r.memory(), before + 1, "OPT resets on your next turn");
}
