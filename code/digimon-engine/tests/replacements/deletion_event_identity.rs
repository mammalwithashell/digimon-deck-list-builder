//! CR 4.3 16-36-1/2: Decode excludes battle, not effects during an attack.
//! CR 15-8-5-3/4: simultaneous immediate effects share the original event.
//! Printed BT26-016, EX12-031, EX12-070, EX12-026 faces checked 2026-10-26.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, ATTACK_START, PASS, REPLACEMENT_ACCEPT, SECURITY_TARGET},
    },
    card_data::CardData,
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    effect::{CardEffect, Effect},
    enums::GamePhase,
    events::GameEvent,
    game::Game,
    permanent::PermanentHandle,
    selection::SelectionKind,
    serialization::to_ui_json,
    trigger_context::EventCause,
};

fn base_runner() -> DebugRunner {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    ))
    .unwrap();
    let mut cards = CardData::load_from_str(&raw).unwrap();
    let mut cause_witness = make_test_card("CAUSE-WITNESS", "Deletion cause witness");
    cause_witness.card_kind = digimon_engine::enums::CardKind::Tamer;
    let mut builder = DebugRunner::builder().add_card(cause_witness);
    for id in [
        "BT26-016", "EX12-031", "EX12-026", "EX12-070", "EX12-036", "EX12-076", "BT1-019",
    ] {
        builder = builder.add_card(cards.remove(id).unwrap());
        if id != "BT1-019" {
            builder = builder.dsl_card(id).unwrap();
        }
    }
    let mut r = builder
        .deck(0, &["BT1-019"; 12])
        .deck(1, &["BT1-019"; 12])
        .hand(0, &["BT1-019"])
        .hand(1, &["EX12-036"])
        .security(0, &["BT1-019"; 3])
        .security(1, &["BT1-019"; 3])
        .memory(10)
        .start();
    r.set_phase(GamePhase::Main);
    r
}

fn fixture() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = base_runner();
    let attacker = r.place_on_field(0, "BT26-016", Some(0));
    let marine = r.place_on_field(1, "EX12-031", Some(0));
    r.push_source(marine, "EX12-026");
    (r, attacker, marine)
}

fn masked(game: &mut Game, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(game, player)[action as usize],
        1.0,
        "illegal {action}, phase {:?}, pending {:?}",
        game.current_phase,
        game.pending_selection
    );
    game.decode_action(action, player);
}

fn assert_decode(game: &Game) {
    let pending = game
        .pending_selection
        .as_ref()
        .expect("effect deletion must offer MarineBullmon Decode");
    assert_eq!(pending.kind, SelectionKind::Replacement, "{pending:?}");
    assert_eq!(game.card(pending.source_card).card_id, "EX12-031");
    assert_eq!(pending.selecting_player, 1);
    assert_eq!(pending.previous_phase, GamePhase::Main);
    assert!(pending.is_optional);
    assert_eq!(build_action_mask(game, 1)[REPLACEMENT_ACCEPT as usize], 1.0);
    assert_eq!(build_action_mask(game, 1)[PASS as usize], 1.0);
}

fn finish_choices(game: &mut Game, clone: &mut Game, reject_decode: bool) {
    for _ in 0..40 {
        let Some(s) = game.pending_selection.as_ref() else {
            break;
        };
        if reject_decode {
            assert!(
                !(s.kind == SelectionKind::Replacement
                    && game.card(s.source_card).card_id == "EX12-031"),
                "a genuine battle must not offer Decode: {s:?}"
            );
        }
        assert_eq!(s.previous_phase, GamePhase::Main, "{s:?}");
        let player = s.selecting_player;
        let mask = build_action_mask(game, player);
        assert_eq!(mask, build_action_mask(clone, player));
        let action = if mask[PASS as usize] > 0.0 {
            PASS
        } else {
            s.valid_action_ids[0]
        };
        masked(game, player, action);
        masked(clone, player, action);
        assert_eq!(to_ui_json(game), to_ui_json(clone));
    }
    assert!(game.pending_selection.is_none());
    assert!(game.pending_attack.is_none());
    assert_eq!(game.current_phase, GamePhase::Main);
}

#[test]
fn chronomon_attack_effect_offers_decode_and_clone_resumes_once() {
    let (mut r, attacker, marine) = fixture();
    masked(
        &mut r.game,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    assert!(r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .prompt
        .contains("delete 1 of your opponent"));
    masked(&mut r.game, 0, ATTACK_START + marine.index as u16);
    assert_decode(&r.game);
    for accept in [false, true] {
        let mut game = r.game.clone();
        let mut clone = game.clone();
        let action = if accept { REPLACEMENT_ACCEPT } else { PASS };
        masked(&mut game, 1, action);
        masked(&mut clone, 1, action);
        assert_eq!(to_ui_json(&game), to_ui_json(&clone));
        finish_choices(&mut game, &mut clone, false);
        let plays = game
            .events
            .iter()
            .filter(|e| matches!(e, GameEvent::Play {card_id, ..} if card_id == "EX12-026"))
            .count();
        assert_eq!(
            plays,
            usize::from(accept),
            "Decode plays its physical source exactly once"
        );
        assert_eq!(
            game.players[1]
                .trash
                .iter()
                .filter(|c| c.card_id(&game.card_data) == "EX12-031")
                .count(),
            1
        );
        assert_eq!(
            game.players[1]
                .battle_area
                .iter()
                .filter(|p| p.top_card().card_id(&game.card_data) == "EX12-026")
                .count(),
            usize::from(accept)
        );
    }
}

#[test]
fn chronomon_actual_battle_never_offers_decode() {
    let (mut r, attacker, marine) = fixture();
    r.game.players[1].battle_area[marine.index as usize].is_suspended = true;
    masked(
        &mut r.game,
        0,
        encode_attack(attacker.index as u16, marine.index as u16),
    );
    assert!(r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .prompt
        .contains("delete 1 of your opponent"));
    // Decline the effect; the declared attack itself now defeats 7000-DP MarineBullmon.
    masked(&mut r.game, 0, PASS);
    let mut clone = r.game.clone();
    finish_choices(&mut r.game, &mut clone, true);
    assert!(r.game.players[1].battle_area.is_empty());
    for id in ["EX12-031", "EX12-026"] {
        assert_eq!(
            r.game.players[1]
                .trash
                .iter()
                .filter(|c| c.card_id(&r.game.card_data) == id)
                .count(),
            1
        );
    }
    assert_eq!(
        r.game
            .events
            .iter()
            .filter(|e| matches!(e, GameEvent::Play {card_id, ..} if card_id == "EX12-026"))
            .count(),
        0
    );
}

/// A native observer checks the emitted cause without replacing either real
/// card's script. Its one draw is an independently countable deletion receipt.
struct OriginalEffectCause;
impl CardEffect for OriginalEffectCause {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_any_deletion(card)
            .name("observe original MarineBullmon deletion")
            .condition(|ctx| {
                ctx.deleted_object_snapshot().is_some_and(|s| {
                    ctx.game
                        .card_data_for_handle(s.top_card)
                        .is_some_and(|c| c.card_id == "EX12-031")
                })
            })
            .process(|ctx| {
                assert_eq!(
                    ctx.event_cause(),
                    Some(EventCause::OpponentEffect),
                    "Arrival cannot recast opponent deletion as its own effect"
                );
                let snapshot = ctx
                    .deleted_object_snapshot()
                    .expect("deleted object snapshot");
                assert_eq!(snapshot.cause, EventCause::OpponentEffect);
                assert_eq!(snapshot.former_controller, 1);
                ctx.draw(0, 1);
            })
            .build()]
    }
}

fn arrival_fixture(compact: bool) -> (Game, CardHandle) {
    let mut r = base_runner();
    r.register_effect("CAUSE-WITNESS", std::sync::Arc::new(OriginalEffectCause));
    let attacker = r.place_on_field(0, "BT26-016", Some(0));
    r.place_on_field(0, "CAUSE-WITNESS", Some(0));
    let arrival = r.place_on_field(1, "EX12-070", Some(0));
    r.game.players[1].battle_area[arrival.index as usize].option_state =
        digimon_engine::permanent::OptionState::Delayed {
            owner: 1,
            trash_on_turn: u16::MAX,
            trigger: digimon_engine::enums::DelayTrigger::ExternallyGated,
            placed_on_turn: 0,
        };
    if !compact {
        r.place_on_field(1, "EX12-076", Some(0));
    }
    let marine = r.place_on_field(1, "EX12-031", Some(0));
    let source = r.push_source(marine, "EX12-026");
    if compact {
        r.place_on_field(1, "EX12-076", Some(0));
    }
    masked(
        &mut r.game,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    masked(&mut r.game, 0, ATTACK_START + marine.index as u16);
    let selection = r
        .game
        .pending_selection
        .as_ref()
        .expect("same-event replacement window");
    assert_eq!(selection.previous_phase, GamePhase::Main);
    assert_eq!(selection.selecting_player, 1);
    let choices = selection
        .effect_choices
        .as_ref()
        .expect("both immediate effects must be orderable before either activates");
    let mut ids: Vec<_> = choices
        .iter()
        .map(|c| r.game.card(c.source_card.unwrap()).card_id.as_str())
        .collect();
    ids.sort();
    assert_eq!(ids, ["EX12-031", "EX12-070"]);
    assert_eq!(
        build_action_mask(&r.game, 1)[PASS as usize],
        0.0,
        "choose an effect before accepting or declining it"
    );
    (r.game, source)
}

fn arrival_matrix(compact: bool, arrival_first: bool) {
    let (initial, shell_source) = arrival_fixture(compact);
    for accept_arrival in [false, true] {
        for accept_decode in [false, true] {
            let reward_cases: &[bool] = if accept_arrival {
                &[false, true]
            } else {
                &[false]
            };
            for &play_reward in reward_cases {
                let mut game = initial.clone();
                let mut clone = game.clone();
                let first = if arrival_first {
                    "EX12-070"
                } else {
                    "EX12-031"
                };
                let first_action = game
                    .pending_selection
                    .as_ref()
                    .unwrap()
                    .effect_choices
                    .as_ref()
                    .unwrap()
                    .iter()
                    .find(|c| game.card(c.source_card.unwrap()).card_id == first)
                    .unwrap()
                    .action_id;
                masked(&mut game, 1, first_action);
                masked(&mut clone, 1, first_action);
                assert_eq!(to_ui_json(&game), to_ui_json(&clone));
                let mut arrival_offers = 0;
                let mut decode_offers = 0;
                let mut reward_offers = 0;
                let mut blocks = 0;
                for _ in 0..60 {
                    let Some(s) = game.pending_selection.as_ref() else {
                        break;
                    };
                    assert_eq!(s.previous_phase, GamePhase::Main, "{s:?}");
                    let player = s.selecting_player;
                    let mask = build_action_mask(&game, player);
                    assert_eq!(mask, build_action_mask(&clone, player));
                    let source_id = game
                        .card_data_for_handle(s.source_card)
                        .map(|c| c.card_id.as_str());
                    let action = if s.kind == SelectionKind::Replacement
                        && source_id == Some("EX12-070")
                    {
                        arrival_offers += 1;
                        assert_eq!(
                            arrival_offers, 1,
                            "Arrival cannot activate twice for one event"
                        );
                        if accept_arrival {
                            REPLACEMENT_ACCEPT
                        } else {
                            PASS
                        }
                    } else if s.kind == SelectionKind::Replacement && source_id == Some("EX12-031")
                    {
                        decode_offers += 1;
                        assert_eq!(
                            decode_offers, 1,
                            "Decode cannot be reoffered after this event's choice"
                        );
                        if accept_decode {
                            REPLACEMENT_ACCEPT
                        } else {
                            PASS
                        }
                    } else if s.prompt
                        == "Play 1 [Sanmyojin] Digimon from hand without paying the cost"
                    {
                        reward_offers += 1;
                        assert_eq!(reward_offers, 1);
                        assert!(game.players[1].battle_area.iter().any(|p| p.top_card().card_id(&game.card_data) == "EX12-031"),
                            "Arrival's reward resolves while the original departure remains pending");
                        if play_reward {
                            s.valid_action_ids[0]
                        } else {
                            PASS
                        }
                    } else if game.current_phase == GamePhase::BlockTiming {
                        blocks += 1;
                        let index = game.players[1]
                            .battle_area
                            .iter()
                            .position(|p| p.top_card().card_id(&game.card_data) == "EX12-076")
                            .unwrap();
                        ATTACK_START + index as u16
                    } else if mask[PASS as usize] > 0.0 {
                        PASS
                    } else {
                        s.valid_action_ids[0]
                    };
                    masked(&mut game, player, action);
                    masked(&mut clone, player, action);
                    assert_eq!(to_ui_json(&game), to_ui_json(&clone));
                }
                assert_eq!(arrival_offers, 1);
                assert_eq!(decode_offers, 1);
                assert_eq!(reward_offers, usize::from(accept_arrival));
                assert_eq!(blocks, 1);
                assert!(game.pending_selection.is_none());
                assert!(game.pending_attack.is_none());
                assert_eq!(game.current_phase, GamePhase::Main);
                assert_eq!(
                    game.players[0].hand.len(),
                    2,
                    "one cause-observer draw, one actual MarineBullmon deletion"
                );
                assert_eq!(
                    game.players[1]
                        .trash
                        .iter()
                        .filter(|c| c.card_id(&game.card_data) == "EX12-031")
                        .count(),
                    1
                );
                assert_eq!(
                    game.players[1]
                        .trash
                        .iter()
                        .filter(|c| c.card_id(&game.card_data) == "EX12-070")
                        .count(),
                    usize::from(accept_arrival)
                );
                let shell_plays = game
                    .events
                    .iter()
                    .filter(|e| matches!(e, GameEvent::Play {card_id, ..} if card_id == "EX12-026"))
                    .count();
                assert_eq!(shell_plays, usize::from(accept_decode));
                let reward_plays = game
                    .events
                    .iter()
                    .filter(|e| matches!(e, GameEvent::Play {card_id, ..} if card_id == "EX12-036"))
                    .count();
                assert_eq!(reward_plays, usize::from(accept_arrival && play_reward));
                assert_eq!(
                    game.players[1]
                        .battle_area
                        .iter()
                        .filter(|p| p.top_card().handle() == shell_source)
                        .count(),
                    usize::from(accept_decode),
                    "Decode plays the same physical source"
                );
                assert!(
                    build_action_mask(&game, 0)
                        .iter()
                        .filter(|x| **x > 0.0)
                        .count()
                        > 1
                );
            }
        }
    }
}

#[test]
fn arrival_first_compacted_slot_shares_original_deletion_window() {
    arrival_matrix(true, true);
}
#[test]
fn decode_first_compacted_slot_shares_original_deletion_window() {
    arrival_matrix(true, false);
}
#[test]
fn arrival_first_separated_slot_shares_original_deletion_window() {
    arrival_matrix(false, true);
}
#[test]
fn decode_first_separated_slot_shares_original_deletion_window() {
    arrival_matrix(false, false);
}
