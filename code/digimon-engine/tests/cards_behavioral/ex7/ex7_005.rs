//! EX7-005 Kapurimon — DIGI-EGG, Lv.2, Black, Cost 0.
//! Traits: Lesser. Form: In-Training.
//!
//! # Card text (data/card_bundles/EX7-005.md — official Bandai DB, verbatim;
//! cross-checked against the card image EX7-005.webp)
//!
//! ```text
//! Inherited Effect [Your Turn] [Once Per Turn] When effects place Option
//! cards with the [Three Musketeers] trait in this Digimon's digivolution
//! cards, gain 1 memory.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Black/EX7_005.cs —
//! `EffectTiming.OnAddDigivolutionCards`, `SetIsInheritedEffect(true)`,
//! `SetUpActivateClass(..., 1 /* OPT */, false /* mandatory */)`.
//! CanUse = IsExistOnBattleAreaDigimon && IsOwnerTurn &&
//! `CanTriggerOnAddDigivolutionCard(permanentCondition: permanent == self,
//! cardEffectCondition: EffectSourceCard != null (ANY effect — the
//! opponent's too), cardCondition: IsOption && ContainsTraits("Three
//! Musketeers"))` → `AddMemory(1)`.
//!
//! # Rules
//! - `general_rule.pdf` 15-5-2 (p.23): one trigger condition met several times
//!   at once triggers ONCE → an effect placing several cards under the host
//!   in one body fires the observer once.
//! - 15-14-1 (p.29): [Once Per Turn] — each activation counts one use.
//!
//! # DSL YAML
//! code/digimon-engine/cards/ex7/EX7-005.yaml
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - G4: inherited triggered clause (source-only), [Your Turn] gate
//! - E1: OPT lockout + reset across turns
//! - `on_add_digivolution_cards` + `event_host_permanent_is_source` +
//!   `event_added_card_any` (G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX7-005";

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

fn option(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Black];
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A [Three Musketeers]-trait DIGIMON — the `kind: option` half of the gate.
fn tm_digimon(id: &str) -> CardData {
    let mut c = digimon(id, 3);
    c.traits = vec!["Three Musketeers".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Green];
    c
}

/// The CAUSE: an [On Play] effect that places the first `count` cards of its
/// controller's hand under the permanent at `target` (one
/// `place_as_bottom_source` per card — the engine's per-host batch window
/// coalesces them into ONE `OnAddDigivolutionCards` firing).
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
        .expect("EX7-005 YAML loads from the embedded pack")
        .add_card(digimon("HOST", 3))
        .add_card(digimon("SIB", 3))
        .add_card(digimon("PLACER", 3))
        .add_card(digimon("OPP-PLACER", 3))
        .add_card(option("TM-OPT", &["Three Musketeers"]))
        .add_card(option("OTHER-OPT", &["Toho"]))
        .add_card(tm_digimon("TM-DIGI"))
        .add_card(filler("F"))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(5)
}

/// Board: player 0's turn; `HOST` on player 0's slot 0 with Kapurimon as its
/// sole digivolution source. Returns the host handle.
fn host_with_kapurimon(r: &mut DebugRunner) -> PermanentHandle {
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    r.place_stack(0, &[CARD_ID, "HOST"])
}

/// Play `placer_id` for `player` (already in the registry with a
/// `PlaceHandUnder` effect) and let its [On Play] tuck `player`'s hand[0..n]
/// under `target`.
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
fn ex7_005_is_a_black_lv2_lesser_digi_egg() {
    let r = base().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.level, Some(2));
    assert_eq!(c.cost, Some(0));
    assert!(c.traits.iter().any(|t| t == "Lesser"), "traits={:?}", c.traits);
}

#[test]
fn ex7_005_single_inherited_opt_your_turn_on_add_digivolution_cards_clause() {
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
    assert_eq!(c.effects.len(), 1, "exactly one clause");
    assert_eq!(triggered.len(), 1);
    let t = triggered[0];
    assert_eq!(t.scope, CompiledScope::Inherited, "Inherited Effect");
    assert_eq!(t.when, vec![CompiledTiming::OnAddDigivolutionCards]);
    assert!(t.once_per_turn, "[Once Per Turn]");
    assert!(!t.optional, "gain 1 memory is mandatory (DCGO isOptional: false)");
    let gate = t.active_when.as_ref().expect("[Your Turn] gate");
    assert_eq!(gate.your_turn, Some(true));
    let cond = t.condition.as_ref().expect("host + added-card gate");
    let leaves = &cond.all_of;
    assert!(!leaves.is_empty(), "all_of");
    assert!(
        leaves.iter().any(|p| p.event_host_permanent_is_source == Some(true)),
        "\"this Digimon's digivolution cards\" → host == self"
    );
    let added = leaves
        .iter()
        .find_map(|p| p.event_added_card_any.as_deref())
        .expect("event_added_card_any gate");
    assert_eq!(added.kind, Some(CompiledCardKind::Option), "Option cards only");
    assert_eq!(
        added.trait_has.as_deref(),
        Some("Three Musketeers"),
        "[Three Musketeers] trait"
    );
    assert!(
        !leaves.iter().any(|p| p.event_caused_by_own_effect.is_some()),
        "printed \"effects\" is NOT owner-gated (DCGO cardEffectCondition: EffectSourceCard != null)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2/3 — Behavioral
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: your own effect places a [Three Musketeers] Option under the
/// Digimon Kapurimon sits under, on your turn → gain 1 memory.
#[test]
fn ex7_005_gains_1_memory_when_effect_places_tm_option_under_host() {
    let mut r = base().hand(0, &["TM-OPT"]).start();
    let host = host_with_kapurimon(&mut r);
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, host);
    assert_eq!(r.memory(), before + 1, "+1 memory");
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        3,
        "Kapurimon + HOST + the tucked Option"
    );
}

/// NEGATIVE (kind gate): a [Three Musketeers]-trait DIGIMON placed under the
/// host is not an Option → no gain.
#[test]
fn ex7_005_no_gain_when_placed_card_is_a_tm_digimon_not_an_option() {
    let mut r = base().hand(0, &["TM-DIGI"]).start();
    let host = host_with_kapurimon(&mut r);
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, host);
    assert_eq!(r.memory(), before, "a Digimon card does not satisfy \"Option cards\"");
}

/// NEGATIVE (trait gate): an Option WITHOUT the [Three Musketeers] trait → no
/// gain.
#[test]
fn ex7_005_no_gain_when_placed_option_lacks_the_trait() {
    let mut r = base().hand(0, &["OTHER-OPT"]).start();
    let host = host_with_kapurimon(&mut r);
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, host);
    assert_eq!(r.memory(), before, "an Option lacking the trait does not qualify");
}

/// NEGATIVE (host gate): the Option is tucked under a SIBLING Digimon, not
/// under the one Kapurimon sits under → no gain.
#[test]
fn ex7_005_no_gain_when_tm_option_is_placed_under_a_sibling() {
    let mut r = base().hand(0, &["TM-OPT"]).start();
    let _host = host_with_kapurimon(&mut r);
    let sib = r.place_on_field(0, "SIB", Some(0));
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, sib);
    assert_eq!(r.memory(), before, "\"this Digimon's digivolution cards\" only");
}

/// NEGATIVE (inherited only): a face-up Kapurimon on the battle area (not a
/// digivolution source) receiving a [Three Musketeers] Option gains nothing —
/// the printed effect is an Inherited Effect.
#[test]
fn ex7_005_face_up_kapurimon_does_not_gain() {
    let mut r = base().hand(0, &["TM-OPT"]).start();
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    let egg = r.place_on_field(0, CARD_ID, Some(0));
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, egg);
    assert_eq!(r.memory(), before, "inherited effects only work from the digivolution cards");
}

/// POSITIVE (any effect): the OPPONENT's effect placing a [Three Musketeers]
/// Option under your Digimon during YOUR turn still triggers — the text says
/// "effects", not "your effects" (DCGO: `EffectSourceCard != null`).
#[test]
fn ex7_005_gains_when_opponent_effect_places_tm_option_on_your_turn() {
    let mut r = base().hand(1, &["TM-OPT"]).start();
    let host = host_with_kapurimon(&mut r); // player 0's turn
    let before = r.memory();
    tuck(&mut r, 1, "OPP-PLACER", 1, host);
    assert_eq!(r.memory(), before + 1, "any effect qualifies; the gain is player 0's");
}

/// NEGATIVE ([Your Turn]): the same placement on the OPPONENT's turn does not
/// trigger.
#[test]
fn ex7_005_no_gain_on_opponents_turn() {
    let mut r = base().hand(1, &["TM-OPT"]).start();
    let host = host_with_kapurimon(&mut r);
    r.game.turn_player_idx = 1; // opponent's turn
    let before = r.memory();
    tuck(&mut r, 1, "OPP-PLACER", 1, host);
    assert_eq!(r.memory(), before, "[Your Turn] gate");
}

/// Rule 15-5-2 batch: one effect tucking a [Three Musketeers] Option AND a
/// filler under the host in one body fires the observer ONCE.
#[test]
fn ex7_005_multi_card_placement_in_one_effect_gains_once() {
    let mut r = base().hand(0, &["TM-OPT", "F"]).start();
    let host = host_with_kapurimon(&mut r);
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 2, host);
    assert_eq!(r.memory(), before + 1, "one trigger condition → one activation (15-5-2)");
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        4
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — [Once Per Turn]
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_005_second_placement_in_the_same_turn_is_locked_out() {
    let mut r = base().hand(0, &["TM-OPT", "TM-OPT"]).start();
    let host = host_with_kapurimon(&mut r);
    let before = r.memory();
    tuck(&mut r, 0, "PLACER", 1, host);
    assert_eq!(r.memory(), before + 1, "first placement gains");
    tuck(&mut r, 0, "SIB", 1, host); // a second placer (SIB re-registered as a placer)
    assert_eq!(r.memory(), before + 1, "[Once Per Turn]: the second placement gains nothing");
}

#[test]
fn ex7_005_lockout_clears_on_your_next_turn() {
    let mut r = base().hand(0, &["TM-OPT", "TM-OPT"]).start();
    let host = host_with_kapurimon(&mut r);
    tuck(&mut r, 0, "PLACER", 1, host);
    // Player 0 → player 1 → player 0.
    r.end_turn();
    r.end_turn();
    assert_eq!(r.turn_player(), 0);
    r.game.set_memory(5);
    let before = r.memory();
    tuck(&mut r, 0, "SIB", 1, host);
    assert_eq!(r.memory(), before + 1, "OPT resets on your next turn");
}
