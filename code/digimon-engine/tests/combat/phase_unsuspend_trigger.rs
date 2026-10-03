//! G-ENGINE-PHASE-UNSUSPEND-NO-ONUNSUSPEND (2026-10-01).
//!
//! The turn-start unsuspend phase (turn player's battle area) and the
//! opponents' `<Reboot>` unsuspension form ONE simultaneous batch that fires
//! `OnUnsuspend` for every battle-area permanent that actually went
//! suspended → unsuspended. DCGO `TurnStateMachine.cs` collects both into one
//! `IUnsuspendPermanents(list, null).Unsuspend()`, which stacks
//! `OnUnTappedAnyone` once for the batch (so a "When any of your Digimon
//! unsuspend" observer triggers ONCE per batch, while "When THIS Digimon
//! unsuspends" observers trigger per permanent).

use std::sync::{Arc, Mutex};

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{GamePhase, Keyword};
use digimon_engine::PermanentHandle;

fn digimon(id: &str, keywords: Vec<Keyword>) -> CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(5000);
    card.keywords = keywords;
    card
}

/// "When THIS Digimon unsuspends" — records (event permanent, phase) per fire.
struct SelfUnsuspendWitness(Arc<Mutex<Vec<(PermanentHandle, GamePhase, bool)>>>);

impl CardEffect for SelfUnsuspendWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let log = self.0.clone();
        vec![Effect::on_unsuspend(card)
            .name("self unsuspend witness")
            .condition(|ctx| ctx.event_permanent().is_some() && ctx.event_permanent() == ctx.source_permanent)
            .process(move |ctx| {
                let ev = ctx.event_permanent().expect("event permanent");
                let initiated = ctx
                    .game
                    .current_trigger_context
                    .as_ref()
                    .map(|t| t.effect_initiated)
                    .unwrap_or(true);
                log.lock()
                    .unwrap()
                    .push((ev, ctx.game.current_phase, initiated));
            })
            .build()]
    }
}

/// "When any of your Digimon unsuspend" — counts fires.
struct AnyOwnUnsuspendWitness(Arc<Mutex<u32>>);

impl CardEffect for AnyOwnUnsuspendWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.0.clone();
        vec![Effect::on_unsuspend(card)
            .name("any own unsuspend witness")
            .condition(|ctx| {
                ctx.event_permanent()
                    .is_some_and(|ev| ev.player == ctx.player())
            })
            .process(move |_ctx| {
                *count.lock().unwrap() += 1;
            })
            .build()]
    }
}

fn suspend(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
}

#[test]
fn unsuspend_phase_fires_on_unsuspend_for_the_turn_players_suspended_digimon() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(digimon("WATCH", vec![]))
        .add_card(digimon("IDLE", vec![]))
        .start();
    r.register_effect("WATCH", Arc::new(SelfUnsuspendWitness(log.clone())));
    r.register_effect("IDLE", Arc::new(SelfUnsuspendWitness(log.clone())));
    let tp = r.game.turn_player();
    let next = 1 - tp;
    let watch = r.place_on_field(next, "WATCH", None);
    let _idle = r.place_on_field(next, "IDLE", None); // never suspended
    suspend(&mut r, watch);

    r.game.end_turn();

    assert_eq!(r.game.turn_player(), next);
    let log = log.lock().unwrap();
    assert_eq!(
        log.len(),
        1,
        "only the permanent that went suspended → unsuspended fires: {log:?}"
    );
    assert_eq!(log[0].0, watch);
    assert_eq!(log[0].1, GamePhase::Unsuspend, "fires in the unsuspend phase");
    assert!(!log[0].2, "phase unsuspension is not effect-initiated");
}

#[test]
fn reboot_unsuspension_fires_on_unsuspend_in_the_opponents_unsuspend_phase() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(digimon("REBOOT", vec![Keyword::Reboot]))
        .add_card(digimon("PLAIN", vec![]))
        .start();
    r.register_effect("REBOOT", Arc::new(SelfUnsuspendWitness(log.clone())));
    r.register_effect("PLAIN", Arc::new(SelfUnsuspendWitness(log.clone())));
    let tp = r.game.turn_player();
    let reboot = r.place_on_field(tp, "REBOOT", None);
    let plain = r.place_on_field(tp, "PLAIN", None);
    suspend(&mut r, reboot);
    suspend(&mut r, plain);

    r.game.end_turn(); // opponent's turn: only <Reboot> unsuspends on our side

    let log = log.lock().unwrap();
    assert_eq!(log.len(), 1, "only the <Reboot> Digimon unsuspended: {log:?}");
    assert_eq!(log[0].0, reboot);
    assert_eq!(log[0].1, GamePhase::Unsuspend);
    assert!(r.game.players[tp as usize].battle_area[plain.index as usize].is_suspended);
}

#[test]
fn any_of_your_digimon_unsuspend_triggers_once_per_unsuspend_phase_batch() {
    let count = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(digimon("OBS", vec![]))
        .add_card(digimon("A", vec![]))
        .start();
    r.register_effect("OBS", Arc::new(AnyOwnUnsuspendWitness(count.clone())));
    let tp = r.game.turn_player();
    let next = 1 - tp;
    let _obs = r.place_on_field(next, "OBS", None);
    for _ in 0..3 {
        let a = r.place_on_field(next, "A", None);
        suspend(&mut r, a);
    }

    r.game.end_turn();

    assert_eq!(
        *count.lock().unwrap(),
        1,
        "three Digimon unsuspending simultaneously trigger the observer once"
    );
}

#[test]
fn effect_unsuspends_still_fire_per_event() {
    // Sanity: the batch collapse is scoped to the phase batch — two separate
    // effect unsuspensions are two events.
    let count = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(digimon("OBS", vec![]))
        .add_card(digimon("A", vec![]))
        .start();
    r.register_effect("OBS", Arc::new(AnyOwnUnsuspendWitness(count.clone())));
    let tp = r.game.turn_player();
    let _obs = r.place_on_field(tp, "OBS", None);
    let a = r.place_on_field(tp, "A", None);
    let b = r.place_on_field(tp, "A", None);
    suspend(&mut r, a);
    suspend(&mut r, b);

    r.game.unsuspend(a);
    r.game.unsuspend(b);

    assert_eq!(*count.lock().unwrap(), 2);
}
