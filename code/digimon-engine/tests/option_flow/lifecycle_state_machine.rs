use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::option_lifecycle::{OptionFieldState, OptionTrashCause};
use digimon_engine::permanent::OptionState;
use std::sync::{Arc, Mutex};

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

struct OptionTrashedWitness {
    seen: Arc<Mutex<u32>>,
    states: Arc<Mutex<Vec<OptionFieldState>>>,
}

impl CardEffect for OptionTrashedWitness {
    fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        let seen = self.seen.clone();
        let states = self.states.clone();
        vec![Effect::on_play(card)
            .timing(digimon_engine::enums::EffectTiming::OnOptionTrashed)
            .process(move |ctx| {
                *seen.lock().unwrap() += 1;
                if let Some(state) = ctx.option_last_field_state() {
                    states.lock().unwrap().push(state);
                }
            })
            .build()]
    }
}

#[test]
fn lifecycle_entry_points_install_queryable_delay_and_ordinary_states() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY"))
        .add_card(option_card("ORDINARY"))
        .hand(0, &["DELAY", "ORDINARY"])
        .start();

    let delay_card = r.game.player_mut(0).hand.remove(0);
    let delay = r
        .game
        .install_field_option_as_delay(delay_card, 0, r.game.turn_count as u32);
    assert_eq!(
        r.game.option_field_state(delay),
        Some(OptionFieldState::Delay {
            placed_turn: r.game.turn_count as u32,
            can_activate_this_turn: false,
        })
    );

    r.game.turn_count += 2;
    assert_eq!(
        r.game.option_field_state(delay),
        Some(OptionFieldState::Delay {
            placed_turn: 1,
            can_activate_this_turn: true,
        })
    );

    let ordinary_card = r.game.player_mut(0).hand.remove(0);
    let ordinary = r.game.install_field_option_as_ordinary(ordinary_card, 0);
    assert_eq!(
        r.game.option_field_state(ordinary),
        Some(OptionFieldState::OrdinaryFieldOption)
    );
}

#[test]
fn lifecycle_entry_points_install_queryable_linked_and_orphaned_plug_in_states() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("PLUGIN"))
        .add_card(digimon_card("HOST"))
        .hand(0, &["PLUGIN"])
        .start();

    let host = r.place_on_field(0, "HOST", Some(0));
    let plugin = r.game.player_mut(0).hand.remove(0);
    assert!(r.game.install_field_option_as_plug_in(plugin, host, 0));
    assert_eq!(
        r.game.linked_plug_in_field_state(host, 0),
        Some(OptionFieldState::LinkedPlugIn {
            carrier: host,
            link_index: 0,
        })
    );

    let orphan = r
        .game
        .orphan_linked_plug_in(host, 0, host.player)
        .expect("linked Plug-In should become a field orphan");
    assert_eq!(
        r.game.option_field_state(orphan),
        Some(OptionFieldState::OrphanedPlugIn {
            last_carrier_owner: host.player,
        })
    );

    assert!(r.game.relink_plug_in(orphan, host, 0));
    assert_eq!(
        r.game.linked_plug_in_field_state(host, 0),
        Some(OptionFieldState::LinkedPlugIn {
            carrier: host,
            link_index: 0,
        })
    );
    assert_eq!(
        r.game.player(0).battle_area.len(),
        1,
        "orphan field slot was consumed"
    );
}

#[test]
fn plug_in_install_respects_link_capacity() {
    let plugin_ids = [
        "PLUGIN0", "PLUGIN1", "PLUGIN2", "PLUGIN3", "PLUGIN4", "PLUGIN5",
    ];
    let mut builder = DebugRunner::builder().add_card(digimon_card("HOST"));
    for id in plugin_ids {
        builder = builder.add_card(option_card(id));
    }
    let mut r = builder.hand(0, &plugin_ids).start();

    let host = r.place_on_field(0, "HOST", Some(0));
    for link_index in 0..5 {
        let plugin = r.game.player_mut(0).hand.remove(0);
        assert!(r
            .game
            .install_field_option_as_plug_in(plugin, host, link_index));
    }

    let over_capacity = r.game.player_mut(0).hand.remove(0);
    assert!(!r
        .game
        .install_field_option_as_plug_in(over_capacity, host, 5));
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        5,
        "failed install must not exceed the carrier's link capacity"
    );
}

#[test]
fn relink_preserves_orphan_when_carrier_is_at_link_capacity() {
    let plugin_ids = [
        "PLUGIN0", "PLUGIN1", "PLUGIN2", "PLUGIN3", "PLUGIN4", "ORPHAN",
    ];
    let mut builder = DebugRunner::builder().add_card(digimon_card("HOST"));
    for id in plugin_ids {
        builder = builder.add_card(option_card(id));
    }
    let mut r = builder.hand(0, &plugin_ids).start();

    let host = r.place_on_field(0, "HOST", Some(0));
    for link_index in 0..5 {
        let plugin = r.game.player_mut(0).hand.remove(0);
        assert!(r
            .game
            .install_field_option_as_plug_in(plugin, host, link_index));
    }
    let orphan_card = r.game.player_mut(0).hand.remove(0);
    let orphan = r.game.install_field_option_as_ordinary(orphan_card, 0);
    assert!(r.game.orphan_plug_in(orphan, host.player));

    assert!(!r.game.relink_plug_in(orphan, host, 5));
    assert_eq!(
        r.game.option_field_state(orphan),
        Some(OptionFieldState::OrphanedPlugIn {
            last_carrier_owner: host.player,
        }),
        "failed relink must keep the orphaned Plug-In on the field"
    );
    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        5
    );
}

#[test]
fn relink_rejects_multi_source_orphans_without_dropping_sources() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("HOST"))
        .add_card(option_card("ORPHAN"))
        .add_card(option_card("EXTRA_SOURCE"))
        .hand(0, &["ORPHAN", "EXTRA_SOURCE"])
        .start();

    let host = r.place_on_field(0, "HOST", Some(0));
    let orphan_card = r.game.player_mut(0).hand.remove(0);
    let orphan = r.game.install_field_option_as_ordinary(orphan_card, 0);
    let extra_source = r.game.player_mut(0).hand.remove(0);
    r.game.player_mut(0).battle_area[orphan.index as usize]
        .card_sources
        .push(extra_source);
    assert!(r.game.orphan_plug_in(orphan, host.player));

    assert!(!r.game.relink_plug_in(orphan, host, 0));
    assert_eq!(
        r.game.player(0).battle_area[orphan.index as usize]
            .card_sources
            .len(),
        2,
        "multi-source orphan rejection must preserve every card source"
    );
    assert_eq!(
        r.game.option_field_state(orphan),
        Some(OptionFieldState::OrphanedPlugIn {
            last_carrier_owner: host.player,
        })
    );
    assert!(r.game.player(0).battle_area[host.index as usize]
        .linked_cards
        .is_empty());
}

#[test]
fn standalone_linked_option_state_is_not_exposed_with_placeholder_index() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("HOST"))
        .add_card(option_card("PLUGIN"))
        .hand(0, &["PLUGIN"])
        .start();

    let host = r.place_on_field(0, "HOST", Some(0));
    let plugin_card = r.game.player_mut(0).hand.remove(0);
    let plugin = r.game.install_field_option_as_ordinary(plugin_card, 0);
    r.game.player_mut(0).battle_area[plugin.index as usize].option_state =
        OptionState::Linked { host };

    assert_eq!(
        r.game.option_field_state(plugin),
        None,
        "standalone linked OptionState lacks a precise link_index; use linked_plug_in_field_state"
    );
}

#[test]
fn trash_field_option_fires_on_option_trashed_with_last_state() {
    let seen = Arc::new(Mutex::new(0));
    let states = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY"))
        .add_card(digimon_card("WATCHER"))
        .hand(0, &["DELAY"])
        .start();
    r.register_effect(
        "WATCHER",
        Arc::new(OptionTrashedWitness {
            seen: seen.clone(),
            states: states.clone(),
        }),
    );
    r.place_on_field(0, "WATCHER", Some(0));

    let delay_card = r.game.player_mut(0).hand.remove(0);
    let placed_turn = r.game.turn_count as u32;
    let delay = r
        .game
        .install_field_option_as_delay(delay_card, 0, placed_turn);

    assert!(r.game.trash_field_option(delay, OptionTrashCause::Effect));
    assert_eq!(*seen.lock().unwrap(), 1);
    assert_eq!(
        states.lock().unwrap().as_slice(),
        &[OptionFieldState::Delay {
            placed_turn,
            can_activate_this_turn: false,
        }]
    );
    assert_eq!(r.trash_size(0), 1);
}
