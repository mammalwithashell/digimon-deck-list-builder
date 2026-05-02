use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};

struct FableWaltzDelay(Arc<Mutex<u32>>);
struct NeverMatchingDelay(Arc<Mutex<u32>>);
struct SelectingEventDelay {
    fired: Arc<Mutex<u32>>,
    selected: Arc<Mutex<u32>>,
}
struct OuterDelaySuspends {
    target: digimon_engine::permanent::PermanentHandle,
    seen: Arc<Mutex<u32>>,
}
struct PlainThisTurnDelay(Arc<Mutex<u32>>);

impl CardEffect for FableWaltzDelay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::on_play(card)
            .name("Arisa suspend gated Delay")
            .delay(DelayTrigger::OnEvent(EffectTiming::OnSuspend))
            .condition(|ctx| ctx.event_card_name_contains("Arisa Kinosaki"))
            .process(move |_ctx| {
                *seen.lock().unwrap() += 1;
            })
            .build()]
    }
}

impl CardEffect for NeverMatchingDelay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::on_play(card)
            .name("never matching suspend gated Delay")
            .delay(DelayTrigger::OnEvent(EffectTiming::OnSuspend))
            .condition(|ctx| ctx.event_card_name_contains("No Such Tamer"))
            .process(move |_ctx| {
                *seen.lock().unwrap() += 1;
            })
            .build()]
    }
}

impl CardEffect for SelectingEventDelay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let fired = self.fired.clone();
        let selected = self.selected.clone();
        vec![Effect::on_play(card)
            .name("selecting event gated Delay")
            .delay(DelayTrigger::OnEvent(EffectTiming::OnSuspend))
            .condition(|ctx| ctx.event_card_name_contains("Arisa Kinosaki"))
            .process(move |ctx| {
                *fired.lock().unwrap() += 1;
                let selected = selected.clone();
                ctx.select_own_permanent(
                    "select event delay target",
                    false,
                    |_game, handle| handle.player == 0,
                    move |_ctx, _handle| {
                        *selected.lock().unwrap() += 1;
                    },
                );
            })
            .build()]
    }
}

impl CardEffect for OuterDelaySuspends {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let target = self.target;
        let seen = self.seen.clone();
        vec![Effect::on_play(card)
            .name("outer Delay suspends")
            .delay(DelayTrigger::EndOfThisTurn)
            .process(move |ctx| {
                *seen.lock().unwrap() += 1;
                ctx.suspend(target);
            })
            .build()]
    }
}

impl CardEffect for PlainThisTurnDelay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::on_play(card)
            .name("plain this turn Delay")
            .delay(DelayTrigger::EndOfThisTurn)
            .process(move |_ctx| {
                *seen.lock().unwrap() += 1;
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
    cd.colors = vec![CardColor::Yellow];
    cd
}

fn tamer_card(card_id: &str, name: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, name);
    cd.card_kind = CardKind::Tamer;
    cd.level = None;
    cd.dp = None;
    cd.colors = vec![CardColor::Yellow];
    cd
}

fn delayed_option_count(r: &DebugRunner, player: u8) -> usize {
    r.game
        .player(player)
        .battle_area
        .iter()
        .filter(|perm| {
            matches!(
                perm.option_state,
                digimon_engine::permanent::OptionState::Delayed { .. }
            )
        })
        .count()
}

#[test]
fn event_gated_delay_only_fires_after_placement_turn_and_matching_event() {
    let witness = Arc::new(Mutex::new(0));
    let never_seen = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(option_card("BT22-098"))
        .add_card(option_card("BT22-099"))
        .add_card(tamer_card("ARISA", "Arisa Kinosaki"))
        .add_card(tamer_card("OTHER", "Other Tamer"))
        .add_card(tamer_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .hand(0, &["BT22-098", "BT22-099"])
        .memory(0)
        .start();
    r.register_effect("BT22-098", Arc::new(FableWaltzDelay(witness.clone())));
    r.register_effect("BT22-099", Arc::new(NeverMatchingDelay(never_seen.clone())));
    let arisa = r.place_on_field(0, "ARISA", Some(0));
    let other = r.place_on_field(0, "OTHER", Some(0));
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    r.game.suspend(other);
    assert_eq!(
        *witness.lock().unwrap(),
        0,
        "wrong Tamer event does not fire"
    );
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    r.game.suspend(arisa);
    assert_eq!(*witness.lock().unwrap(), 0, "placement-turn event is gated");

    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.unsuspend(other);
    r.game.suspend(other);
    assert_eq!(
        *witness.lock().unwrap(),
        0,
        "wrong Tamer event after placement turn does not fire"
    );
    assert_eq!(
        *never_seen.lock().unwrap(),
        0,
        "nonmatching delayed option does not fire"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "nonmatching delayed options should not enter trigger-order selection"
    );
    r.game.unsuspend(arisa);
    r.game.suspend(arisa);

    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "matching event after placement turn fires once"
    );
    assert_eq!(*never_seen.lock().unwrap(), 0);
    assert_eq!(
        r.trash_size(0),
        1,
        "Delay trashes itself as activation cost"
    );
}

#[test]
fn nested_event_gated_delay_resume_preserves_outer_delayed_scan() {
    let event_fired = Arc::new(Mutex::new(0));
    let event_selected = Arc::new(Mutex::new(0));
    let outer_seen = Arc::new(Mutex::new(0));
    let sibling_seen = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(option_card("EVENT"))
        .add_card(option_card("OUTER"))
        .add_card(option_card("SIBLING"))
        .add_card(tamer_card("ARISA", "Arisa Kinosaki"))
        .add_card(tamer_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .hand(0, &["EVENT", "OUTER", "SIBLING"])
        .memory(0)
        .start();
    let arisa = r.place_on_field(0, "ARISA", Some(0));
    r.register_effect(
        "EVENT",
        Arc::new(SelectingEventDelay {
            fired: event_fired.clone(),
            selected: event_selected.clone(),
        }),
    );
    r.register_effect(
        "OUTER",
        Arc::new(OuterDelaySuspends {
            target: arisa,
            seen: outer_seen.clone(),
        }),
    );
    r.register_effect(
        "SIBLING",
        Arc::new(PlainThisTurnDelay(sibling_seen.clone())),
    );
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );

    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    assert_eq!(r.game.turn_player(), 0);

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed
    );
    assert_eq!(delayed_option_count(&r, 0), 3, "all delays are parked");

    r.end_turn();

    assert_eq!(*outer_seen.lock().unwrap(), 1, "outer Delay fired");
    assert_eq!(
        *event_fired.lock().unwrap(),
        1,
        "nested event-gated Delay fired during outer scan"
    );
    assert_eq!(
        *event_selected.lock().unwrap(),
        0,
        "event selection is still parked"
    );
    assert!(
        r.game.pending_selection.is_some(),
        "nested event Delay selection should pause lifecycle"
    );

    let (selecting_player, action_id) = {
        let pending = r.game.pending_selection.as_ref().expect("event selection");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    r.game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve nested event selection");

    assert_eq!(
        *event_selected.lock().unwrap(),
        1,
        "nested event selection callback resolved"
    );
    assert_eq!(
        *sibling_seen.lock().unwrap(),
        1,
        "outer delayed scan resumed to the sibling Delay"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "all delayed lifecycle continuations should settle"
    );
    assert_eq!(
        delayed_option_count(&r, 0),
        0,
        "event, outer, and sibling delayed options all trashed"
    );
    assert_eq!(r.trash_size(0), 3);
}
