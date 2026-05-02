use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, GamePhase};
use digimon_engine::permanent::OptionState;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DelaySnapshot {
    turn_count: u16,
    turn_player: u8,
    phase: GamePhase,
    hand_size: usize,
    deck_size: usize,
}

struct StartDelayWitness {
    seen: Arc<Mutex<u32>>,
    snapshot: Arc<Mutex<Option<DelaySnapshot>>>,
}

impl CardEffect for StartDelayWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.seen.clone();
        let snapshot = self.snapshot.clone();
        vec![Effect::on_play(card)
            .name("Start of your next turn Delay")
            .delay(DelayTrigger::StartOfYourNextTurn)
            .process(move |ctx| {
                *seen.lock().unwrap() += 1;
                let player = ctx.game.player(ctx.player);
                *snapshot.lock().unwrap() = Some(DelaySnapshot {
                    turn_count: ctx.game.turn_count,
                    turn_player: ctx.game.turn_player(),
                    phase: ctx.game.current_phase,
                    hand_size: player.hand.len(),
                    deck_size: player.deck.len(),
                });
            })
            .build()]
    }
}

struct StartTurnSelector(Arc<Mutex<u32>>);

impl CardEffect for StartTurnSelector {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::start_of_your_turn(card)
            .name("Start turn selection")
            .process(move |ctx| {
                *seen.lock().unwrap() += 1;
                ctx.select_own_permanent("pick one", false, |_game, _h| true, |_, _| {});
            })
            .build()]
    }
}

struct StartDelaySelection(Arc<Mutex<u32>>);

impl CardEffect for StartDelaySelection {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::on_play(card)
            .name("Start Delay selection")
            .delay(DelayTrigger::StartOfYourNextTurn)
            .process(move |ctx| {
                *seen.lock().unwrap() += 1;
                ctx.select_own_permanent("pick one", false, |_game, _h| true, |_, _| {});
            })
            .build()]
    }
}

fn option_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = 0;
    cd.colors = vec![CardColor::Red];
    cd
}

fn digimon_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![CardColor::Red];
    cd
}

#[test]
fn start_of_your_next_turn_delay_fires_at_turn_start_not_end() {
    let witness = Arc::new(Mutex::new(0));
    let snapshot = Arc::new(Mutex::new(None));
    let mut r = DebugRunner::builder()
        .add_card(option_card("LM-027"))
        .add_card(digimon_card("RED-MATCH"))
        .add_card(digimon_card("FILLER"))
        .hand(0, &["LM-027"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(0)
        .start();
    r.register_effect(
        "LM-027",
        Arc::new(StartDelayWitness {
            seen: witness.clone(),
            snapshot: snapshot.clone(),
        }),
    );
    r.place_on_field(0, "RED-MATCH", Some(0));
    r.game.enter_main_phase();

    let start_turn = r.game.turn_count;
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    assert_eq!(*witness.lock().unwrap(), 0);

    r.end_turn();
    assert_eq!(
        *witness.lock().unwrap(),
        0,
        "does not fire at end of placement turn"
    );
    r.game.enter_main_phase();
    assert_eq!(
        *witness.lock().unwrap(),
        0,
        "does not fire during opponent turn"
    );
    r.end_turn();

    assert_eq!(r.game.turn_count, start_turn + 2);
    assert_eq!(r.game.turn_player(), 0);
    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "fires from begin_turn before draw/main"
    );
    assert_eq!(
        *snapshot.lock().unwrap(),
        Some(DelaySnapshot {
            turn_count: start_turn + 2,
            turn_player: 0,
            phase: GamePhase::EndTurn,
            hand_size: 0,
            deck_size: 6,
        }),
        "Delay process observes the turn-start boundary before draw/main mutates hand, deck, or phase"
    );
    assert!(!r
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| matches!(p.option_state, OptionState::Delayed { .. })));
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn start_delay_waits_when_start_observer_installs_selection() {
    let selector_seen = Arc::new(Mutex::new(0));
    let delay_seen = Arc::new(Mutex::new(0));
    let snapshot = Arc::new(Mutex::new(None));
    let mut r = DebugRunner::builder()
        .add_card(option_card("LM-027"))
        .add_card(digimon_card("RED-MATCH"))
        .add_card(digimon_card("FILLER"))
        .hand(0, &["LM-027"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(0)
        .start();
    r.register_effect(
        "LM-027",
        Arc::new(StartDelayWitness {
            seen: delay_seen.clone(),
            snapshot,
        }),
    );
    r.register_effect(
        "RED-MATCH",
        Arc::new(StartTurnSelector(selector_seen.clone())),
    );
    r.place_on_field(0, "RED-MATCH", Some(0));
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );

    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();

    assert_eq!(*selector_seen.lock().unwrap(), 1);
    assert_eq!(
        *delay_seen.lock().unwrap(),
        0,
        "start Delay must wait behind the pending start-of-turn selection"
    );
    assert!(
        r.game.pending_selection.is_some(),
        "start-of-turn observer selection remains parked"
    );
    assert!(
        r.game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "delayed option is not trashed while a start-of-turn selection is pending"
    );

    let (selecting_player, action_id) = {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("start-of-turn observer selection should be pending");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    r.game
        .resolve_selection(selecting_player, action_id)
        .expect("start-of-turn observer selection resolves");

    assert!(
        r.game.pending_selection.is_none(),
        "resolving the start-of-turn observer selection must not leave the game stuck"
    );
    assert_eq!(
        *delay_seen.lock().unwrap(),
        1,
        "mature start Delay drains after the earlier start observer selection resolves"
    );
    assert_eq!(
        r.game.current_phase,
        GamePhase::Main,
        "begin_turn continues through draw/breeding into main after start observer selection resolves"
    );
    assert_eq!(r.game.turn_player(), 0);
    assert_eq!(
        r.game.player(0).hand.len(),
        1,
        "player 0 draws after the start-of-turn lifecycle resumes"
    );
    assert_eq!(r.game.player(0).deck.len(), 5);
    assert!(
        !r.game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "mature start Delay is disposed after lifecycle resumes"
    );
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn start_delay_scan_stops_when_delay_effect_installs_selection() {
    let first_seen = Arc::new(Mutex::new(0));
    let second_seen = Arc::new(Mutex::new(0));
    let snapshot = Arc::new(Mutex::new(None));
    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-SELECT"))
        .add_card(option_card("DELAY-WAIT"))
        .add_card(digimon_card("RED-MATCH"))
        .add_card(digimon_card("FILLER"))
        .hand(0, &["DELAY-SELECT", "DELAY-WAIT"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(0)
        .start();
    r.register_effect(
        "DELAY-SELECT",
        Arc::new(StartDelaySelection(first_seen.clone())),
    );
    r.register_effect(
        "DELAY-WAIT",
        Arc::new(StartDelayWitness {
            seen: second_seen.clone(),
            snapshot,
        }),
    );
    r.place_on_field(0, "RED-MATCH", Some(0));
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );

    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();

    assert_eq!(*first_seen.lock().unwrap(), 1);
    assert_eq!(
        *second_seen.lock().unwrap(),
        0,
        "sibling start Delay must not run while the first Delay selection is pending"
    );
    assert!(
        r.game.pending_selection.is_some(),
        "DelayEffect selection remains parked"
    );
    assert_eq!(
        r.game.current_phase,
        GamePhase::SelectTarget,
        "turn lifecycle pauses at the DelayEffect selection instead of advancing into draw/main"
    );
    assert_eq!(r.game.turn_count, 3);
    assert_eq!(r.game.turn_player(), 0);
    assert_eq!(
        r.game.player(0).hand.len(),
        0,
        "player 0 has not drawn for the new turn while the DelayEffect selection is pending"
    );
    assert_eq!(
        r.game.player(0).deck.len(),
        6,
        "player 0 deck is unchanged because draw has not happened"
    );
    let delayed_remaining = r
        .game
        .player(0)
        .battle_area
        .iter()
        .filter(|p| matches!(p.option_state, OptionState::Delayed { .. }))
        .count();
    assert_eq!(
        delayed_remaining, 2,
        "the selecting Delay and its sibling both remain parked until selection resolution"
    );
    assert_eq!(r.trash_size(0), 0);

    let (selecting_player, action_id) = {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("DelayEffect selection should be pending");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    r.game
        .resolve_selection(selecting_player, action_id)
        .expect("DelayEffect selection resolves");

    assert!(
        r.game.pending_selection.is_none(),
        "resolving the DelayEffect selection must not leave the game stuck"
    );
    assert_eq!(
        *first_seen.lock().unwrap(),
        1,
        "the selecting Delay must not re-fire when delayed lifecycle resumes"
    );
    assert_eq!(
        *second_seen.lock().unwrap(),
        1,
        "remaining mature start Delay resolves after the first selection is answered"
    );
    assert_eq!(
        r.game.current_phase,
        GamePhase::Main,
        "begin_turn resumes through draw/breeding into main after delayed selections settle"
    );
    assert_eq!(r.game.turn_count, 3);
    assert_eq!(r.game.turn_player(), 0);
    assert_eq!(
        r.game.player(0).hand.len(),
        1,
        "player 0 draws for the new turn after delayed lifecycle resumes"
    );
    assert_eq!(r.game.player(0).deck.len(), 5);
    let delayed_remaining = r
        .game
        .player(0)
        .battle_area
        .iter()
        .filter(|p| matches!(p.option_state, OptionState::Delayed { .. }))
        .count();
    assert_eq!(
        delayed_remaining, 0,
        "both mature start delays are disposed after lifecycle resumes"
    );
    assert_eq!(r.trash_size(0), 2);
}
