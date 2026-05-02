use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};

struct FableWaltzDelay(Arc<Mutex<u32>>);
struct NeverMatchingDelay(Arc<Mutex<u32>>);

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
