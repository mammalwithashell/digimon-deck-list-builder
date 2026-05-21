use digimon_engine::action::space::BREEDING_TARGET;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardSourceRef;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::{CardHandle, CardSource};
use std::sync::Arc;

struct OnMoveObserver;

impl CardEffect for OnMoveObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_move(card)
            .name("OnMove observer gains memory")
            .condition(|ctx| ctx.event_permanent().is_some())
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

#[test]
fn move_from_breeding_by_effect_moves_real_breeding_stack_and_fires_move_observers() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("OBS", "Move Observer"))
        .add_card(make_test_card("LOW", "Lower Source"))
        .add_card(make_test_card("TOP", "Top Card"))
        .hand(0, &["OBS"])
        .memory(5)
        .start();
    r.register_effect("OBS", Arc::new(OnMoveObserver));

    let observer = r.place_on_field(0, "OBS", Some(0));
    let observer_card = r.top_card(observer);
    r.place_in_breeding(0, "TOP");
    let lower = new_card_source(&mut r, 0, "LOW");
    r.game
        .player_mut(0)
        .breeding_area
        .as_mut()
        .unwrap()
        .push_under(lower);
    let breeding_top = r
        .game
        .player(0)
        .breeding_area
        .as_ref()
        .unwrap()
        .top_card()
        .handle();

    let before_memory = r.memory();
    {
        let mut ctx = EffectContext::new(&mut r.game, observer_card, Some(observer), 0);
        assert!(ctx.move_from_breeding_by_effect(0));
    }

    assert!(r.game.player(0).breeding_area.is_none());
    assert_eq!(r.game.player(0).battle_area.len(), 2);
    let moved = &r.game.player(0).battle_area[1];
    assert_eq!(moved.top_card().handle(), breeding_top);
    assert_eq!(moved.card_sources.len(), 2);
    assert_eq!(moved.card_sources[0].card_id(&r.game.card_data), "LOW");
    assert_eq!(moved.card_sources[1].card_id(&r.game.card_data), "TOP");
    assert_eq!(r.memory(), before_memory + 1);
}

#[test]
fn play_to_breeding_from_hand_uses_real_breeding_slot_and_rejects_occupied_slot() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("HANDMON", "Hand Digimon"))
        .add_card(make_test_card("OTHER", "Other Digimon"))
        .hand(0, &["SRC", "HANDMON", "OTHER"])
        .memory(10)
        .start();

    let source = r.place_on_field(0, "SRC", Some(0));
    let source_card = r.top_card(source);
    {
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), 0);
        assert!(ctx.play_to_breeding_from_hand(0, 1));
    }

    assert_eq!(r.game.player(0).hand.len(), 2);
    let breeding = r.game.player(0).breeding_area.as_ref().unwrap();
    assert_eq!(breeding.top_card().card_id(&r.game.card_data), "HANDMON");
    assert!(r
        .game
        .player(0)
        .battle_area
        .iter()
        .all(|perm| perm.top_card().card_id(&r.game.card_data) != "HANDMON"));

    {
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), 0);
        assert!(!ctx.play_to_breeding_from_hand(0, 1));
    }

    assert_eq!(r.game.player(0).hand[1].card_id(&r.game.card_data), "OTHER");
    assert_eq!(
        r.game
            .player(0)
            .breeding_area
            .as_ref()
            .unwrap()
            .top_card()
            .card_id(&r.game.card_data),
        "HANDMON"
    );
}

#[test]
fn place_as_bottom_source_with_breeding_target_tucks_under_real_breeding_stack() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("BREED", "Breeding Digimon"))
        .add_card(make_test_card("TUCK", "Tuck Card"))
        .hand(0, &["SRC", "TUCK"])
        .memory(10)
        .start();

    let source = r.place_on_field(0, "SRC", Some(0));
    let source_card = r.top_card(source);
    r.place_in_breeding(0, "BREED");
    let breeding_target = PermanentHandle {
        player: 0,
        index: BREEDING_TARGET as u8,
    };

    {
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), 0);
        assert!(ctx.place_as_bottom_source(CardSourceRef::Hand(0, 1), breeding_target, false));
    }

    assert_eq!(r.game.player(0).hand.len(), 1);
    let breeding = r.game.player(0).breeding_area.as_ref().unwrap();
    assert_eq!(breeding.card_sources.len(), 2);
    assert_eq!(breeding.card_sources[0].card_id(&r.game.card_data), "TUCK");
    assert_eq!(breeding.card_sources[1].card_id(&r.game.card_data), "BREED");
    assert!(r
        .game
        .player(0)
        .battle_area
        .iter()
        .all(|perm| perm.top_card().card_id(&r.game.card_data) != "BREED"));
}

fn new_card_source(r: &mut DebugRunner, player: u8, card_id: &str) -> CardSource {
    let data_index = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("test card exists");
    let card_index = r.game.next_card_index();
    CardSource::new(data_index, player, card_index)
}
