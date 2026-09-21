//! `OnAddDigivolutionCards` trigger timing [G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS].
//!
//! Drivers (all `[Your Turn] [Once Per Turn]` inherited observers):
//! - BT7-056 Dorumon — "When one of your effects places a digivolution card
//!   under this Digimon, gain 1 memory." (host == self AND the placing effect
//!   is the observer's own — DCGO `BT7_056.cs`
//!   `CanTriggerOnAddDigivolutionCard(permanent == self,
//!   cardEffect.EffectSourceCard.Owner == card.Owner, null)`).
//! - EX7-005 Kapurimon — "When effects place Option cards with the
//!   [Three Musketeers] trait in this Digimon's digivolution cards, gain 1
//!   memory." (host == self AND ANY added card is an Option with the trait —
//!   DCGO `EX7_005.cs` `cardCondition: IsOption && ContainsTraits(...)`).
//! - BT25-005 Pagumon — "When [Three Musketeers] trait cards are placed in
//!   this Digimon's digivolution cards, it may digivolve ... cost reduced by 2".
//!
//! Rules (`general_rule.pdf`, Comprehensive Rules Manual Ver.3.6):
//! - 15-5-2 (p.23): "A triggering from 1 trigger condition is considered to
//!   only trigger once, even if it occurred multiple times at the same time"
//!   → an effect that places SEVERAL cards under one host fires the observer
//!   ONCE (batch), not once per card.
//! - 15-5-3 (p.23-24), whose worked example IS this timing: "If a card has a
//!   '[Your Turn] [Once Per Turn] When your effect places a digivolution card
//!   under this Digimon, gain 1 memory' inherited effect, the trigger
//!   conditions will be met even when that card itself is placed in
//!   digivolution cards by one of your effects" → the placed card's own
//!   inherited observer fires.
//!
//! DCGO semantics (`Permanent.cs` `AddDigivolutionCardsTop` :1078-1136 /
//! `AddDigivolutionCardsBottom` :1147-1240 + `CardEffectCommons/CanUseEffects/
//! OnAddDigivolutionCards.cs`):
//! - `StackSkillInfos(hashtable, EffectTiming.OnAddDigivolutionCards)` fires
//!   ONCE per call with the whole `addedCards` list, the host `Permanent`, and
//!   the placing `CardEffect`.
//! - `CanTriggerOnAddDigivolutionCard` requires a NON-NULL `CardEffect`:
//!   normal digivolution (`Permanent.AddDigivolutionCardTop(CardSource)` —
//!   no trigger at all), DNA digivolution (`CardController.cs:1736`,
//!   `AddDigivolutionCardsTop(newDigivolutionCards, null)`), DigiXros
//!   materials (`SelectDigiXrosClass.cs:910/926/932`, `null`), Assembly
//!   materials (`SelectAssemblyClass.cs:302`, `null`) and App Fusion
//!   (`SelectAppFusionEffect.cs:234`, `null`) never fire it.
//! - Effect-driven placements DO: `<Save>` (`Save.cs:61`), `<Material Save>`
//!   (`MaterialSave.cs:114`), `<Training>` (`Training.cs:29`),
//!   `IPlacePermanentToDigivolutionCards` with a cardEffect
//!   (`CardController.cs:3362-3366`, e.g. `<Mind Link>`), and the ~347 card
//!   scripts that call `AddDigivolutionCardsBottom(cards, this)`.
//!
//! These tests drive the ENGINE dispatch with hand-written recording effects
//! (plus two inline-YAML DSL fixtures for the `when:` / predicate vocabulary).
//! They do NOT author the driver cards.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, make_test_dna_card, DebugRunner,
};
use digimon_engine::effect::{CardEffect, Effect, EffectBuilder};
use digimon_engine::enums::{
    CardKind, CardSourceRef, EffectTiming, GamePhase, PlaySource, PlayerId,
};
use digimon_engine::permanent::PermanentHandle;
use std::sync::{Arc, Mutex};

fn digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(3000);
    c
}

fn option(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// One recorded firing of the observer.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Fire {
    observer_player: PlayerId,
    host: Option<PermanentHandle>,
    added: Vec<CardHandle>,
    own_effect: bool,
}

// ─── An OnAddDigivolutionCards observer that records each fire ──────────────
//
// `host_only`: gate on the event host being THIS permanent (BT7-056 / EX7-005
// "under this Digimon"). `own_only`: gate on the placing effect belonging to
// the observer's controller (BT7-056 "one of your effects").
struct AddSourcesObserver {
    host_only: bool,
    own_only: bool,
    inherited: bool,
    fired: Arc<Mutex<Vec<Fire>>>,
}
impl CardEffect for AddSourcesObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let host_only = self.host_only;
        let own_only = self.own_only;
        let fired = self.fired.clone();
        let mut builder = Effect::on_add_digivolution_cards(card)
            .name("observer: when an effect places digivolution cards")
            .condition(move |ctx| {
                if host_only {
                    let Some(host) = ctx.event_host_permanent() else {
                        return false;
                    };
                    if ctx.source_permanent != Some(host) {
                        return false;
                    }
                }
                if own_only && !ctx.event_caused_by_own_effect() {
                    return false;
                }
                true
            })
            .process(move |ctx| {
                let added = ctx.added_source_cards();
                let own_effect = ctx.event_caused_by_own_effect();
                fired.lock().unwrap().push(Fire {
                    observer_player: ctx.player,
                    host: ctx.event_host_permanent(),
                    added,
                    own_effect,
                });
            });
        if self.inherited {
            builder = builder.inherited();
        }
        vec![builder.build()]
    }
}

// ─── Placing effects (the CAUSE) ────────────────────────────────────────────

/// On play: place the first `count` cards of the controller's hand under the
/// permanent at `target_index` on `target_player`'s battle area, one
/// `place_as_bottom_source` call per card.
struct PlaceHandUnderTarget {
    count: usize,
    target_player: PlayerId,
    target_index: u8,
}
impl CardEffect for PlaceHandUnderTarget {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.count;
        let target = PermanentHandle {
            player: self.target_player,
            index: self.target_index,
        };
        vec![Effect::on_play(card)
            .name("effect that places hand cards under a permanent")
            .process(move |ctx| {
                for _ in 0..count {
                    // Always hand index 0 (indices shift as cards leave).
                    let ok = ctx.place_as_bottom_source(
                        CardSourceRef::Hand(ctx.player, 0),
                        target,
                        false,
                    );
                    assert!(ok, "placement must succeed");
                }
            })
            .build()]
    }
}

fn observer(
    host_only: bool,
    own_only: bool,
    fired: &Arc<Mutex<Vec<Fire>>>,
) -> Arc<AddSourcesObserver> {
    Arc::new(AddSourcesObserver {
        host_only,
        own_only,
        inherited: false,
        fired: fired.clone(),
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Positive: own effect places ONE card under the observer → fires once,
// carrying the host and the added card.
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn fires_once_when_own_effect_places_one_card_under_host() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(digimon("OBS", "Observer"))
        .add_card(digimon("TRIG", "Placer"))
        .add_card(digimon("H", "HandCard"))
        .add_card(digimon("F", "F"))
        .hand(0, &["H"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS", observer(true, true, &fired));
    r.register_effect(
        "TRIG",
        Arc::new(PlaceHandUnderTarget {
            count: 1,
            target_player: 0,
            target_index: 0,
        }),
    );
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    let obs = r.place_on_field(0, "OBS", Some(0));
    let hand_card = r.game.players[0].hand[0].handle();
    let trig = r.place_on_field(0, "TRIG", Some(0));
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();

    let fires = fired.lock().unwrap().clone();
    assert_eq!(fires.len(), 1, "exactly one OnAddDigivolutionCards firing");
    assert_eq!(fires[0].observer_player, 0);
    assert_eq!(
        fires[0].host,
        Some(obs),
        "host must be the receiving permanent"
    );
    assert_eq!(
        fires[0].added,
        vec![hand_card],
        "the added batch is the placed card"
    );
    assert!(
        fires[0].own_effect,
        "placing effect belongs to the observer's controller"
    );
    // The card really is under the observer.
    assert_eq!(
        r.game.players[0].battle_area[obs.index as usize]
            .card_sources
            .len(),
        2
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Batch (rule 15-5-2 / DCGO once-per-list): an effect that places TWO cards
// under the same host in one body fires the observer ONCE with both cards.
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn multi_card_placement_in_one_effect_body_fires_once_with_the_whole_batch() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(digimon("OBS", "Observer"))
        .add_card(digimon("TRIG", "Placer"))
        .add_card(digimon("H", "HandCard"))
        .add_card(digimon("F", "F"))
        .hand(0, &["H", "H"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS", observer(true, true, &fired));
    r.register_effect(
        "TRIG",
        Arc::new(PlaceHandUnderTarget {
            count: 2,
            target_player: 0,
            target_index: 0,
        }),
    );
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    let obs = r.place_on_field(0, "OBS", Some(0));
    let h0 = r.game.players[0].hand[0].handle();
    let h1 = r.game.players[0].hand[1].handle();
    let trig = r.place_on_field(0, "TRIG", Some(0));
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();

    let fires = fired.lock().unwrap().clone();
    assert_eq!(
        fires.len(),
        1,
        "two placements under one host in one effect body coalesce into ONE firing (15-5-2)"
    );
    assert_eq!(fires[0].host, Some(obs));
    assert_eq!(
        fires[0].added,
        vec![h0, h1],
        "both placed cards ride in the batch"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Negative: OPPONENT's effect places a card under YOUR Digimon → the own-only
// (BT7-056) observer does NOT fire; a host-only observer without the
// own-effect gate DOES (the event happened, the gate is what differs).
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn own_only_observer_does_not_fire_when_opponent_effect_places_under_it() {
    let fired_own_only = Arc::new(Mutex::new(Vec::new()));
    let fired_any = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(digimon("OBS-OWN", "OwnOnlyObserver"))
        .add_card(digimon("OBS-ANY", "AnyEffectObserver"))
        .add_card(digimon("TRIG-OPP", "OppPlacer"))
        .add_card(digimon("H", "HandCard"))
        .add_card(digimon("F", "F"))
        .hand(1, &["H"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS-OWN", observer(true, true, &fired_own_only));
    r.register_effect("OBS-ANY", observer(true, false, &fired_any));
    // Player 1's effect places player 1's hand card under player 0's slot 0.
    r.register_effect(
        "TRIG-OPP",
        Arc::new(PlaceHandUnderTarget {
            count: 1,
            target_player: 0,
            target_index: 0,
        }),
    );
    r.game.turn_count = 1;
    r.game.turn_player_idx = 1;

    // Slot 0 on player 0 hosts BOTH observers' effects? No — one permanent
    // per observer. Run the scenario twice: once with each observer at slot 0.
    let own = r.place_on_field(0, "OBS-OWN", Some(0));
    let trig = r.place_on_field(1, "TRIG-OPP", Some(0));
    r.game.fire_on_play(1, trig.index as usize);
    r.auto_resolve().ok();
    assert!(
        fired_own_only.lock().unwrap().is_empty(),
        "BT7-056 shape: the OPPONENT's effect placing a card under this Digimon must NOT fire \
         the own-effect-gated observer (event_caused_by_own_effect == false)"
    );
    assert!(
        fired_any.lock().unwrap().is_empty(),
        "OBS-ANY is not on the field yet"
    );

    // Now the any-effect observer at slot 0 of a fresh board.
    let fired_any = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(digimon("OBS-ANY", "AnyEffectObserver"))
        .add_card(digimon("TRIG-OPP", "OppPlacer"))
        .add_card(digimon("H", "HandCard"))
        .add_card(digimon("F", "F"))
        .hand(1, &["H"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS-ANY", observer(true, false, &fired_any));
    r.register_effect(
        "TRIG-OPP",
        Arc::new(PlaceHandUnderTarget {
            count: 1,
            target_player: 0,
            target_index: 0,
        }),
    );
    r.game.turn_count = 1;
    r.game.turn_player_idx = 1;
    let any = r.place_on_field(0, "OBS-ANY", Some(0));
    let trig = r.place_on_field(1, "TRIG-OPP", Some(0));
    r.game.fire_on_play(1, trig.index as usize);
    r.auto_resolve().ok();
    let fires = fired_any.lock().unwrap().clone();
    assert_eq!(
        fires.len(),
        1,
        "an ungated host observer sees the opponent's placement"
    );
    assert_eq!(fires[0].host, Some(any));
    assert!(
        !fires[0].own_effect,
        "the placing effect was the OPPONENT's"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Host gate: a card placed under a SIBLING does not fire a host-only observer
// on a different permanent (but the board-wide dispatch does reach an
// ungated observer — DCGO's `permanentCondition` is the card's own gate).
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn host_only_observer_ignores_placement_under_a_sibling() {
    let fired_host_only = Arc::new(Mutex::new(Vec::new()));
    let fired_boardwide = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(digimon("OBS-HOST", "HostOnlyObserver"))
        .add_card(digimon("OBS-WIDE", "BoardWideObserver"))
        .add_card(digimon("SIB", "Sibling"))
        .add_card(digimon("TRIG", "Placer"))
        .add_card(digimon("H", "HandCard"))
        .add_card(digimon("F", "F"))
        .hand(0, &["H"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS-HOST", observer(true, false, &fired_host_only));
    r.register_effect("OBS-WIDE", observer(false, false, &fired_boardwide));
    // Place under slot 2 (the sibling).
    r.register_effect(
        "TRIG",
        Arc::new(PlaceHandUnderTarget {
            count: 1,
            target_player: 0,
            target_index: 2,
        }),
    );
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    let _host_only = r.place_on_field(0, "OBS-HOST", Some(0)); // slot 0
    let _wide = r.place_on_field(0, "OBS-WIDE", Some(0)); // slot 1
    let sib = r.place_on_field(0, "SIB", Some(0)); // slot 2
    let trig = r.place_on_field(0, "TRIG", Some(0)); // slot 3
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();

    assert!(
        fired_host_only.lock().unwrap().is_empty(),
        "host-only observer must not fire for a placement under a different permanent"
    );
    let wide = fired_boardwide.lock().unwrap().clone();
    assert_eq!(
        wide.len(),
        1,
        "board-wide dispatch reaches an ungated observer once"
    );
    assert_eq!(
        wide[0].host,
        Some(sib),
        "event host is the sibling that received the card"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Rule 15-5-3: the PLACED card's own inherited observer fires when it is
// itself placed under a host by an effect.
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn placed_card_own_inherited_observer_fires_when_it_is_itself_placed() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(digimon("HOST", "Host"))
        .add_card(digimon("TRIG", "Placer"))
        .add_card(digimon("DORU", "DorumonShape"))
        .add_card(digimon("F", "F"))
        .hand(0, &["DORU"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    // DORU carries the observer as an INHERITED effect (live only while it is
    // a digivolution source) gated host == self AND own effect.
    r.register_effect(
        "DORU",
        Arc::new(AddSourcesObserver {
            host_only: true,
            own_only: true,
            inherited: true,
            fired: fired.clone(),
        }),
    );
    r.register_effect(
        "TRIG",
        Arc::new(PlaceHandUnderTarget {
            count: 1,
            target_player: 0,
            target_index: 0,
        }),
    );
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    let host = r.place_on_field(0, "HOST", Some(0));
    let doru = r.game.players[0].hand[0].handle();
    let trig = r.place_on_field(0, "TRIG", Some(0));
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();

    let fires = fired.lock().unwrap().clone();
    assert_eq!(
        fires.len(),
        1,
        "15-5-3: the placed card's own inherited observer triggers on its own placement"
    );
    assert_eq!(fires[0].host, Some(host));
    assert_eq!(fires[0].added, vec![doru]);
}

// ═══════════════════════════════════════════════════════════════════════════
// Negative: NORMAL digivolution places no card "by effect" → no firing.
// (DCGO: `Permanent.AddDigivolutionCardTop(CardSource)` stacks nothing.)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn normal_digivolution_does_not_fire() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let base = make_test_card_with_level("BASE-LV4", "BaseLv4", 4);
    let mut evo = make_test_card_with_level("EVO-LV5", "EvoLv5", 5);
    evo.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 4,
        memory_cost: 0,
    }];
    let mut r = DebugRunner::builder()
        .add_card(base)
        .add_card(evo)
        .add_card(digimon("OBS", "BoardWideObserver"))
        .hand(0, &["EVO-LV5"])
        .memory(10)
        .start();
    r.register_effect("OBS", observer(false, false, &fired));
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    r.place_on_field(0, "OBS", Some(0));
    let base_slot = r.place_on_field(0, "BASE-LV4", Some(0));
    r.game.current_phase = GamePhase::Main;
    let ok = r
        .game
        .digivolve_from_hand(0, 0, base_slot.index as usize, PlaySource::ByHand);
    assert!(ok, "regular digivolve must succeed");
    r.auto_resolve().ok();

    assert!(
        fired.lock().unwrap().is_empty(),
        "a normal digivolution is not an effect placing digivolution cards"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Negative: DNA digivolution stacks its materials with a `null` cardEffect in
// DCGO (`CardController.cs:1736`) → no firing.
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn dna_digivolution_does_not_fire() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let lv5 = make_test_card_with_level("TST-LV5", "FiveDigi", 5);
    let lv6 = make_test_card_with_level("TST-LV6", "SixDigi", 6);
    let dna = make_test_dna_card("TST-DNA", "DnaDigi", 5, 6, 0);
    let mut r = DebugRunner::builder()
        .add_card(lv5)
        .add_card(lv6)
        .add_card(dna)
        .add_card(digimon("OBS", "BoardWideObserver"))
        .hand(0, &["TST-DNA"])
        .memory(5)
        .start();
    r.register_effect("OBS", observer(false, false, &fired));
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    r.place_on_field(0, "OBS", None);
    let handle_lv5 = r.place_on_field(0, "TST-LV5", None);
    let handle_lv6 = r.place_on_field(0, "TST-LV6", None);
    r.game.current_phase = GamePhase::Main;

    assert!(r.game.initiate_dna_digivolve(0, 0));
    r.game
        .resolve_selection(0, handle_lv5.index as u16)
        .expect("stage 1");
    r.game
        .resolve_selection(0, handle_lv6.index as u16)
        .expect("stage 2");
    r.auto_resolve().ok();

    assert_eq!(
        r.game.n_dna_digivolutions,
        [1u32, 0u32],
        "DNA digivolve happened"
    );
    assert!(
        fired.lock().unwrap().is_empty(),
        "DNA digivolution materials are not placed by an effect (DCGO null cardEffect)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Host that left play before the batch flushed fires nothing (DCGO
// `Permanent.TopCard != null` gate).
// ═══════════════════════════════════════════════════════════════════════════

/// On play: place hand[0] under slot `target_index`, then delete that host.
struct PlaceThenDeleteHost {
    target_index: u8,
}
impl CardEffect for PlaceThenDeleteHost {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let target_index = self.target_index;
        vec![Effect::on_play(card)
            .name("place under a host, then delete the host")
            .process(move |ctx| {
                let target = PermanentHandle {
                    player: ctx.player,
                    index: target_index,
                };
                let ok =
                    ctx.place_as_bottom_source(CardSourceRef::Hand(ctx.player, 0), target, false);
                assert!(ok);
                ctx.delete_permanent(target);
            })
            .build()]
    }
}

#[test]
fn host_deleted_before_flush_fires_nothing() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(digimon("OBS", "BoardWideObserver"))
        .add_card(digimon("HOST", "Host"))
        .add_card(digimon("TRIG", "Placer"))
        .add_card(digimon("H", "HandCard"))
        .add_card(digimon("F", "F"))
        .hand(0, &["H"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS", observer(false, false, &fired));
    r.register_effect("TRIG", Arc::new(PlaceThenDeleteHost { target_index: 1 }));
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    r.place_on_field(0, "OBS", Some(0)); // slot 0
    r.place_on_field(0, "HOST", Some(0)); // slot 1
    let trig = r.place_on_field(0, "TRIG", Some(0)); // slot 2
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();

    assert!(
        fired.lock().unwrap().is_empty(),
        "the receiving permanent left play before the batch flushed — nothing to trigger on"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// DSL vocabulary: `when: on_add_digivolution_cards` +
// `event_host_permanent_is_source` + `event_caused_by_own_effect` (BT7-056
// Dorumon shape) and `event_added_card_any` (EX7-005 Kapurimon shape).
// ═══════════════════════════════════════════════════════════════════════════

const DORUMON_SHAPE: &str = r#"
card: TEST-DORU
name: Test Dorumon Shape
kind: digimon
level: 3
color: [black]
cost: 3
dp: 3000
effects:
  - scope: inherited
    when: on_add_digivolution_cards
    once_per_turn: true
    active_when: { your_turn: true }
    condition:
      all_of:
        - event_host_permanent_is_source: true
        - event_caused_by_own_effect: true
    summary: "[Your Turn][Once Per Turn] When one of your effects places a digivolution card under this Digimon, gain 1 memory"
    process:
      - gain_memory: 1
"#;

const KAPURIMON_SHAPE: &str = r#"
card: TEST-KAPU
name: Test Kapurimon Shape
kind: digimon
level: 3
color: [black]
cost: 3
dp: 3000
effects:
  - scope: inherited
    when: on_add_digivolution_cards
    once_per_turn: true
    active_when: { your_turn: true }
    condition:
      all_of:
        - event_host_permanent_is_source: true
        - event_added_card_any: { kind: option, trait_has: Three Musketeers }
    summary: "[Your Turn][Once Per Turn] When effects place Option cards with the [Three Musketeers] trait in this Digimon's digivolution cards, gain 1 memory"
    process:
      - gain_memory: 1
"#;

/// Build a host with `under_id` as its sole digivolution source (the DSL
/// observer card), and a placer that tucks hand[0] under it.
fn dsl_board(yaml: &str, under_id: &str, hand_card: CardData) -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline YAML compiles")
        .add_card(digimon("HOST", "Host"))
        .add_card(digimon("TRIG", "Placer"))
        .add_card(hand_card)
        .add_card(digimon("F", "F"))
        .hand(0, &["H"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(5)
        .start();
    r.register_effect(
        "TRIG",
        Arc::new(PlaceHandUnderTarget {
            count: 1,
            target_player: 0,
            target_index: 0,
        }),
    );
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    let host = r.place_stack(0, &[under_id, "HOST"]);
    (r, host)
}

#[test]
fn dsl_dorumon_shape_gains_memory_when_own_effect_places_under_it() {
    let (mut r, host) = dsl_board(DORUMON_SHAPE, "TEST-DORU", digimon("H", "HandCard"));
    let before = r.game.memory;
    let trig = r.place_on_field(0, "TRIG", Some(0));
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();
    assert_eq!(
        r.game.memory,
        before + 1,
        "Dorumon-shape inherited observer gains 1 memory when an own effect places a card under its host"
    );
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        3
    );
}

#[test]
fn dsl_dorumon_shape_does_not_fire_for_opponent_effect() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(DORUMON_SHAPE)
        .expect("inline YAML compiles")
        .add_card(digimon("HOST", "Host"))
        .add_card(digimon("TRIG-OPP", "OppPlacer"))
        .add_card(digimon("H", "HandCard"))
        .add_card(digimon("F", "F"))
        .hand(1, &["H"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(5)
        .start();
    r.register_effect(
        "TRIG-OPP",
        Arc::new(PlaceHandUnderTarget {
            count: 1,
            target_player: 0,
            target_index: 0,
        }),
    );
    // Player 0's turn so the observer's `your_turn` gate passes; the OPPONENT's
    // effect (player 1) does the placing.
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    let _host = r.place_stack(0, &["TEST-DORU", "HOST"]);
    let before = r.game.memory;
    let trig = r.place_on_field(1, "TRIG-OPP", Some(0));
    r.game.fire_on_play(1, trig.index as usize);
    r.auto_resolve().ok();
    assert_eq!(
        r.game.memory, before,
        "`event_caused_by_own_effect: true` rejects the opponent's placing effect"
    );
}

#[test]
fn dsl_kapurimon_shape_fires_only_when_an_added_card_is_a_three_musketeers_option() {
    // Positive: a [Three Musketeers] Option is placed.
    let (mut r, _host) = dsl_board(
        KAPURIMON_SHAPE,
        "TEST-KAPU",
        option("H", "MusketeerOption", &["Three Musketeers"]),
    );
    let before = r.game.memory;
    let trig = r.place_on_field(0, "TRIG", Some(0));
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();
    assert_eq!(
        r.game.memory,
        before + 1,
        "`event_added_card_any` matches the placed Option"
    );

    // Negative: a Digimon (not an Option) is placed — the batch gate fails.
    let (mut r, _host) = dsl_board(KAPURIMON_SHAPE, "TEST-KAPU", digimon("H", "PlainDigimon"));
    let before = r.game.memory;
    let trig = r.place_on_field(0, "TRIG", Some(0));
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();
    assert_eq!(
        r.game.memory, before,
        "a non-Option added card does not satisfy the gate"
    );

    // Negative: an Option WITHOUT the trait.
    let (mut r, _host) = dsl_board(
        KAPURIMON_SHAPE,
        "TEST-KAPU",
        option("H", "OtherOption", &["Toho"]),
    );
    let before = r.game.memory;
    let trig = r.place_on_field(0, "TRIG", Some(0));
    r.game.fire_on_play(0, trig.index as usize);
    r.auto_resolve().ok();
    assert_eq!(
        r.game.memory, before,
        "an Option lacking the trait does not satisfy the gate"
    );
}
