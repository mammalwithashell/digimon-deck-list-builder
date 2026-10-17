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
    // A Plug-In carries [Link]; without one the rule check trashes it
    // (§17-1-3-2-6 / -7).
    r.give_link_requirement("PLUGIN", 0);

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

/// §4-9-5 "1 card can have a maximum of 1 link card. When linking to a
/// Digimon that has already reached the link limit, the same number of the
/// existing link cards are trashed at the same time as the newly linked
/// cards." Inverted from the old base-5 capacity refusal: a carrier with no
/// <Link +> at its limit of 1 still takes a Plug-In, which replaces the one
/// it holds.
#[test]
fn plug_in_install_at_link_limit_replaces_the_link_card() {
    let plugin_ids = ["PLUGIN0", "PLUGIN1"];
    let mut builder = DebugRunner::builder().add_card(digimon_card("HOST"));
    for id in plugin_ids {
        builder = builder.add_card(option_card(id));
    }
    let mut r = builder.hand(0, &plugin_ids).start();
    for id in plugin_ids {
        r.give_link_requirement(id, 0);
    }

    let host = r.place_on_field(0, "HOST", Some(0));
    let first = r.game.player_mut(0).hand.remove(0);
    let first_handle = first.handle();
    assert!(r.game.install_field_option_as_plug_in(first, host, 0));
    let second = r.game.player_mut(0).hand.remove(0);
    let second_handle = second.handle();
    assert!(r.game.install_field_option_as_plug_in(second, host, 1));

    let linked: Vec<_> = r.game.player(0).battle_area[host.index as usize]
        .linked_cards
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(linked, vec![second_handle], "the new Plug-In replaced the old one");
    assert!(
        r.game.player(0).trash.iter().any(|c| c.handle() == first_handle),
        "the replaced Plug-In is trashed"
    );
}

/// §4-9-5 (as above). Inverted from the old base-5 refusal: relinking an
/// orphaned Plug-In onto a carrier at its limit of 1 succeeds and replaces
/// the carrier's link card.
#[test]
fn relink_onto_carrier_at_link_limit_replaces_the_link_card() {
    let plugin_ids = ["PLUGIN0", "ORPHAN"];
    let mut builder = DebugRunner::builder().add_card(digimon_card("HOST"));
    for id in plugin_ids {
        builder = builder.add_card(option_card(id));
    }
    let mut r = builder.hand(0, &plugin_ids).start();
    for id in plugin_ids {
        r.give_link_requirement(id, 0);
    }

    let host = r.place_on_field(0, "HOST", Some(0));
    let plugin = r.game.player_mut(0).hand.remove(0);
    let plugin_handle = plugin.handle();
    assert!(r.game.install_field_option_as_plug_in(plugin, host, 0));
    let orphan_card = r.game.player_mut(0).hand.remove(0);
    let orphan_handle = orphan_card.handle();
    let orphan = r.game.install_field_option_as_ordinary(orphan_card, 0);
    assert!(r.game.orphan_plug_in(orphan, host.player));

    assert!(r.game.relink_plug_in(orphan, host, 1));
    let linked: Vec<_> = r.game.player(0).battle_area[host.index as usize]
        .linked_cards
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(linked, vec![orphan_handle], "the relinked Plug-In replaced the old one");
    assert!(
        r.game.player(0).trash.iter().any(|c| c.handle() == plugin_handle),
        "the replaced Plug-In is trashed"
    );
}

/// §4-9-5 / §10-1-3-1: relinking an orphaned Plug-In onto a carrier at its
/// link limit with 2 link cards asks the carrier's controller (the linking
/// player) which existing link card is replaced, before the orphan moves; the
/// relink completes after the pick (the shared `LinkCardTrashChoice` path).
#[test]
fn relink_onto_carrier_at_link_limit_offers_the_replacement_choice() {
    let plugin_ids = ["PLUGIN0", "PLUGIN1", "ORPHAN"];
    let mut builder = DebugRunner::builder().add_card(digimon_card("HOST"));
    for id in plugin_ids {
        builder = builder.add_card(option_card(id));
    }
    let mut r = builder.hand(0, &plugin_ids).start();
    for id in plugin_ids {
        r.give_link_requirement(id, 0);
    }

    let host = r.place_on_field(0, "HOST", Some(0));
    r.give_link_plus(host, 1);
    let mut existing = Vec::new();
    for slot in 0..2u8 {
        let plugin = r.game.player_mut(0).hand.remove(0);
        existing.push(plugin.handle());
        assert!(r.game.install_field_option_as_plug_in(plugin, host, slot));
    }
    let orphan_card = r.game.player_mut(0).hand.remove(0);
    let orphan_handle = orphan_card.handle();
    let orphan = r.game.install_field_option_as_ordinary(orphan_card, 0);
    assert!(r.game.orphan_plug_in(orphan, host.player));

    assert!(r.game.relink_plug_in(orphan, host, 2));
    let sel = r.game.pending_selection.as_ref().expect("the replacement choice");
    assert_eq!(sel.selecting_player, 0, "the linking player chooses");
    let offered: Vec<_> = sel
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|c| c.source_card)
        .collect();
    assert_eq!(offered, existing, "exactly the existing link cards are offered");
    assert_eq!(
        r.game.option_field_state(orphan),
        Some(OptionFieldState::OrphanedPlugIn {
            last_carrier_owner: host.player,
        }),
        "the orphan has not moved before the choice"
    );

    let pick = sel.valid_action_ids[0];
    r.game.resolve_selection(0, pick).expect("pick PLUGIN0");
    assert!(r.game.pending_selection.is_none());
    let linked: Vec<_> = r.game.player(0).battle_area[host.index as usize]
        .linked_cards
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(linked, vec![existing[1], orphan_handle]);
    assert!(
        r.game.player(0).trash.iter().any(|c| c.handle() == existing[0]),
        "the picked Plug-In is trashed"
    );
    assert_eq!(r.game.player(0).battle_area.len(), 1, "the orphan left the field");
}

/// As above with the orphan BELOW the carrier (orphan at index 0, HOST at
/// index 1): the relink is validated and the choice parked against the
/// carrier as it stands (index 1), and after the pick the orphan leaves and
/// the carrier, now at index 0, keeps its <Link +1> and takes the orphan.
#[test]
fn relink_from_a_lower_orphan_onto_carrier_at_link_limit_offers_the_choice() {
    let plugin_ids = ["PLUGIN0", "PLUGIN1", "ORPHAN"];
    let mut builder = DebugRunner::builder().add_card(digimon_card("HOST"));
    for id in plugin_ids {
        builder = builder.add_card(option_card(id));
    }
    let mut r = builder.hand(0, &plugin_ids).start();
    for id in plugin_ids {
        r.give_link_requirement(id, 0);
    }

    let orphan_card = r.game.player_mut(0).hand.remove(2);
    let orphan_handle = orphan_card.handle();
    let orphan = r.game.install_field_option_as_ordinary(orphan_card, 0);
    let host = r.place_on_field(0, "HOST", Some(0));
    assert_eq!((orphan.index, host.index), (0, 1), "orphan below the carrier");
    r.give_link_plus(host, 1);
    let mut existing = Vec::new();
    for slot in 0..2u8 {
        let plugin = r.game.player_mut(0).hand.remove(0);
        existing.push(plugin.handle());
        assert!(r.game.install_field_option_as_plug_in(plugin, host, slot));
    }
    assert!(r.game.orphan_plug_in(orphan, host.player));

    assert!(r.game.relink_plug_in(orphan, host, 2));
    let sel = r.game.pending_selection.as_ref().expect("the replacement choice");
    assert_eq!(sel.selecting_player, 0, "the linking player chooses");
    assert_eq!(sel.source_permanent, Some(host), "asked about the carrier at index 1");
    let offered: Vec<_> = sel
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|c| c.source_card)
        .collect();
    assert_eq!(offered, existing, "exactly the carrier's link cards are offered");

    let pick = sel.valid_action_ids[1];
    r.game.resolve_selection(0, pick).expect("pick PLUGIN1");
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.player(0).battle_area.len(), 1, "the orphan left the field");
    let linked: Vec<_> = r.game.player(0).battle_area[0]
        .linked_cards
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(linked, vec![existing[0], orphan_handle]);
    assert!(
        r.game.player(0).trash.iter().any(|c| c.handle() == existing[1]),
        "the picked Plug-In is trashed"
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
