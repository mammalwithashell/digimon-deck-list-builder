//! Deletion causes, unchanged departure routes, and per-event object identity.
//! CR 4.3 15-8-5-2/-4, 16-36-1/-2: effects during security are not battles;
//! a new deletion event and a newly played incarnation each get their own window.
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, PASS, REPLACEMENT_ACCEPT, SECURITY_TARGET},
    },
    card_data::CardData,
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder},
    effect::{CardEffect, Effect},
    enums::{CardKind, EffectTiming, GamePhase, Zone},
    game::Game,
    permanent::{OptionState, PermanentHandle},
    replacement::{ReplacementCause, ReplacementOutcome, ReplacementSubject},
    selection::SelectionKind,
    serialization::to_ui_json,
};

fn masked(game: &mut Game, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(game, player)[action as usize],
        1.0,
        "illegal {action}, pending {:?}",
        game.pending_selection
    );
    game.decode_action(action, player);
}

fn real_cards() -> DebugRunnerBuilder {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    ))
    .unwrap();
    let mut cards = CardData::load_from_str(&raw).unwrap();
    let mut builder = DebugRunner::builder();
    for id in ["EX12-031", "EX12-026", "EX12-070", "EX12-036", "BT1-019"] {
        builder = builder.add_card(cards.remove(id).unwrap());
        if id != "BT1-019" {
            builder = builder.dsl_card(id).unwrap();
        }
    }
    builder
}

struct SecurityDelete;
impl CardEffect for SecurityDelete {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("Delete the only opposing Digimon")
            .process(|ctx| {
                assert!(ctx.game.security_resolution.is_some());
                assert!(ctx.game.pending_attack.is_some());
                assert_eq!(
                    ctx.game.players[(1 - ctx.player) as usize]
                        .battle_area
                        .len(),
                    1
                );
                ctx.delete_permanent(PermanentHandle {
                    player: 1 - ctx.player,
                    index: 0,
                });
            })
            .build()]
    }
}

struct CauseProbe(Arc<Mutex<Vec<ReplacementCause>>>);
impl CardEffect for CauseProbe {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.0.clone();
        vec![Effect::when_would_be_deleted(card)
            .replacement_condition(|ctx, subject| subject.permanent() == ctx.source_permanent)
            .replacement_process(move |ctx| {
                seen.lock().unwrap().push(ctx.cause);
                ctx.cancel();
            })
            .build()]
    }
}

#[test]
fn security_effect_deletion_is_opponent_effect_but_security_battle_is_battle() {
    for security_effect in [true, false] {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut attacker = make_test_card("ROUTE-PROBE", "Cause probe");
        attacker.dp = Some(1000);
        let mut security = make_test_card("ROUTE-SECURITY", "Security route witness");
        security.card_kind = if security_effect {
            CardKind::Option
        } else {
            CardKind::Digimon
        };
        security.dp = if security_effect { None } else { Some(15000) };
        let mut r = DebugRunner::builder()
            .add_card(attacker)
            .add_card(security)
            .deck(0, &["ROUTE-PROBE"; 5])
            .deck(1, &["ROUTE-PROBE"; 5])
            .security(1, &["ROUTE-SECURITY"])
            .memory(5)
            .start();
        r.register_effect("ROUTE-PROBE", Arc::new(CauseProbe(seen.clone())));
        if security_effect {
            r.register_effect("ROUTE-SECURITY", Arc::new(SecurityDelete));
        }
        r.set_phase(GamePhase::Main);
        let h = r.place_on_field(0, "ROUTE-PROBE", Some(0));
        masked(
            &mut r.game,
            0,
            encode_attack(h.index as u16, SECURITY_TARGET),
        );
        assert!(r.game.pending_selection.is_none());
        assert!(r.game.pending_attack.is_none());
        assert_eq!(
            *seen.lock().unwrap(),
            vec![if security_effect {
                ReplacementCause::OpponentEffect
            } else {
                ReplacementCause::Battle
            }]
        );
        assert_eq!(
            r.game.players[0].battle_area.len(),
            1,
            "the tested prevention ran once"
        );
    }
}

#[test]
fn marine_decode_is_offered_for_security_effect_but_not_security_battle() {
    for security_effect in [true, false] {
        let mut security = make_test_card("ROUTE-SECURITY", "Security route witness");
        security.card_kind = if security_effect {
            CardKind::Option
        } else {
            CardKind::Digimon
        };
        security.dp = if security_effect { None } else { Some(15000) };
        let mut r = real_cards()
            .add_card(security)
            .deck(0, &["BT1-019"; 8])
            .deck(1, &["BT1-019"; 8])
            .security(1, &["ROUTE-SECURITY"])
            .memory(5)
            .start();
        if security_effect {
            r.register_effect("ROUTE-SECURITY", Arc::new(SecurityDelete));
        }
        r.set_phase(GamePhase::Main);
        let marine = r.place_on_field(0, "EX12-031", Some(0));
        r.push_source(marine, "EX12-026");
        masked(
            &mut r.game,
            0,
            encode_attack(marine.index as u16, SECURITY_TARGET),
        );
        if security_effect {
            let choice = r
                .game
                .pending_selection
                .as_ref()
                .expect("Decode before effect deletion");
            assert_eq!(choice.kind, SelectionKind::Replacement);
            assert_eq!(r.game.card(choice.source_card).card_id, "EX12-031");
            assert!(choice.is_optional);
            assert_eq!(
                build_action_mask(&r.game, 0)[REPLACEMENT_ACCEPT as usize],
                1.0
            );
            let mut clone = r.game.clone();
            masked(&mut r.game, 0, PASS);
            masked(&mut clone, 0, PASS);
            assert_eq!(to_ui_json(&r.game), to_ui_json(&clone));
        }
        assert!(
            r.game.pending_selection.is_none(),
            "battle never offers Decode"
        );
        assert!(r.game.pending_attack.is_none());
        assert!(r.game.players[0].battle_area.is_empty());
        for id in ["EX12-031", "EX12-026"] {
            assert_eq!(
                r.game.players[0]
                    .trash
                    .iter()
                    .filter(|c| c.card_id(&r.game.card_data) == id)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn arrival_reward_preserves_original_return_to_hand_destination() {
    for play_reward in [false, true] {
        let mut r = real_cards()
            .deck(0, &["BT1-019"; 8])
            .deck(1, &["BT1-019"; 8])
            .hand(1, &["EX12-036"])
            .memory(5)
            .start();
        r.set_phase(GamePhase::Main);
        let arrival = r.place_on_field(1, "EX12-070", Some(0));
        r.game.players[1].battle_area[arrival.index as usize].option_state = OptionState::Delayed {
            owner: 1,
            trash_on_turn: u16::MAX,
            trigger: digimon_engine::enums::DelayTrigger::ExternallyGated,
            placed_on_turn: 0,
        };
        let marine = r.place_on_field(1, "EX12-031", Some(0));
        // No eligible Decode source: Arrival is the sole immediate effect.
        let original = r.game.players[1].battle_area[marine.index as usize]
            .top_card()
            .handle();
        r.game.return_to_hand_from_effect(marine, 0);
        let choice = r
            .game
            .pending_selection
            .as_ref()
            .expect("Arrival before the return");
        assert_eq!(choice.kind, SelectionKind::Replacement);
        assert_eq!(r.game.card(choice.source_card).card_id, "EX12-070");
        masked(&mut r.game, 1, REPLACEMENT_ACCEPT);
        let reward = r
            .game
            .pending_selection
            .as_ref()
            .expect("Arrival hand reward");
        assert!(reward.prompt.contains("Sanmyojin"));
        assert!(
            r.game.players[1]
                .battle_area
                .iter()
                .any(|p| p.top_card().handle() == original),
            "the original departure waits for its immediate reward body"
        );
        let action = if play_reward {
            reward.valid_action_ids[0]
        } else {
            PASS
        };
        let mut clone = r.game.clone();
        masked(&mut r.game, 1, action);
        masked(&mut clone, 1, action);
        for _ in 0..20 {
            assert_eq!(to_ui_json(&r.game), to_ui_json(&clone));
            let Some(choice) = r.game.pending_selection.as_ref() else {
                break;
            };
            let player = choice.selecting_player;
            let mask = build_action_mask(&r.game, player);
            assert_eq!(mask, build_action_mask(&clone, player));
            let action = if mask[PASS as usize] > 0.0 {
                PASS
            } else {
                choice.valid_action_ids[0]
            };
            masked(&mut r.game, player, action);
            masked(&mut clone, player, action);
        }
        assert!(r.game.pending_selection.is_none());
        assert_eq!(
            r.game.players[1]
                .hand
                .iter()
                .filter(|c| c.handle() == original)
                .count(),
            1
        );
        assert!(
            r.game.players[1]
                .trash
                .iter()
                .all(|c| c.handle() != original),
            "returning is not deletion"
        );
        assert_eq!(
            r.game.players[1]
                .battle_area
                .iter()
                .filter(|p| p.top_card().card_id(&r.game.card_data) == "EX12-036")
                .count(),
            usize::from(play_reward)
        );
        assert_eq!(
            r.game.players[1]
                .trash
                .iter()
                .filter(|c| c.card_id(&r.game.card_data) == "EX12-070")
                .count(),
            1
        );
    }
}

struct CountAndPrevent(Arc<AtomicUsize>);
impl CardEffect for CountAndPrevent {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.0.clone();
        vec![Effect::when_would_be_deleted(card)
            .replacement_condition(|ctx, subject| subject.permanent() == ctx.source_permanent)
            .replacement_process(move |ctx| {
                count.fetch_add(1, Ordering::SeqCst);
                ctx.cancel();
            })
            .build()]
    }
}

struct DeleteOtherTwice;
impl CardEffect for DeleteOtherTwice {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .replacement_condition(|ctx, subject| subject.permanent() == ctx.source_permanent)
            .replacement_process(|ctx| {
                let other = PermanentHandle {
                    player: 0,
                    index: 1,
                };
                ctx.effect
                    .game
                    .delete_permanent_with_cause(other, ReplacementCause::OwnEffect);
                ctx.effect
                    .game
                    .delete_permanent_with_cause(other, ReplacementCause::OwnEffect);
                ctx.cancel();
            })
            .build()]
    }
}

#[test]
fn two_nested_deletion_events_offer_same_survivors_prevention_twice() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("OUTER", "Outer"))
        .add_card(make_test_card("SURVIVOR", "Survivor"))
        .start();
    r.register_effect("OUTER", Arc::new(DeleteOtherTwice));
    r.register_effect("SURVIVOR", Arc::new(CountAndPrevent(count.clone())));
    let outer = r.place_on_field(0, "OUTER", Some(0));
    r.place_on_field(0, "SURVIVOR", Some(0));
    r.game
        .delete_permanent_with_cause(outer, ReplacementCause::OwnEffect);
    assert_eq!(
        count.load(Ordering::SeqCst),
        2,
        "each distinct deletion has its own immediate window"
    );
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert!(r.game.players[0].trash.is_empty());
    assert!(r.game.pending_selection.is_none());
}

struct DeleteTwiceAfterDeparture;
impl CardEffect for DeleteTwiceAfterDeparture {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::when_would_be_deleted(card)
                .name("Continue the original deletion")
                .optional()
                .replacement_condition(|ctx, subject| subject.permanent() == ctx.source_permanent)
                .replacement_process(|_| {})
                .build(),
            Effect::on_deletion(card)
                .process(|ctx| {
                    assert!(ctx.game.players[0]
                        .trash
                        .iter()
                        .any(|c| c.handle() == ctx.source_card));
                    assert_eq!(ctx.game.players[0].battle_area.len(), 1);
                    let survivor = PermanentHandle {
                        player: 0,
                        index: 0,
                    };
                    // Use the Game mutation entry point deliberately: the
                    // queued effect body must establish fresh event scope,
                    // independently of EffectContext::delete_permanent.
                    ctx.game
                        .delete_permanent_with_cause(survivor, ReplacementCause::OwnEffect);
                    ctx.game
                        .delete_permanent_with_cause(survivor, ReplacementCause::OwnEffect);
                })
                .build(),
        ]
    }
}

#[test]
fn queued_on_deletion_starts_fresh_events_and_restores_commit_guards() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("QUEUED-OUTER", "Outer with deletion effect"))
        .add_card(make_test_card("SURVIVOR", "Survivor"))
        .deck(0, &["SURVIVOR"; 4])
        .deck(1, &["SURVIVOR"; 4])
        .memory(5)
        .start();
    r.register_effect("QUEUED-OUTER", Arc::new(DeleteTwiceAfterDeparture));
    r.register_effect("SURVIVOR", Arc::new(CountAndPrevent(count.clone())));
    r.set_phase(GamePhase::Main);
    let outer = r.place_on_field(0, "QUEUED-OUTER", Some(0));
    let departed = r.game.players[0].battle_area[0].top_card().handle();
    r.place_on_field(0, "SURVIVOR", Some(0));
    r.game
        .delete_permanent_with_cause(outer, ReplacementCause::OpponentEffect);
    let choice = r
        .game
        .pending_selection
        .as_ref()
        .expect("original commit is parked");
    assert_eq!(choice.kind, SelectionKind::Replacement);
    assert_eq!(choice.source_card, departed);
    masked(&mut r.game, 0, REPLACEMENT_ACCEPT);
    assert_eq!(
        count.load(Ordering::SeqCst),
        2,
        "each deletion from the queued body is a new event, even inside the old commit"
    );
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0]
            .trash
            .iter()
            .filter(|c| c.handle() == departed)
            .count(),
        1
    );
    assert_eq!(r.game.current_phase, GamePhase::Main);

    r.game.delete_permanent_with_cause(
        PermanentHandle {
            player: 0,
            index: 0,
        },
        ReplacementCause::OwnEffect,
    );
    assert_eq!(
        count.load(Ordering::SeqCst),
        3,
        "the completed commit leaves no stale guard"
    );
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert!(r.game.pending_selection.is_none());
}

struct ReplaySamePhysicalCard(Arc<AtomicUsize>);
impl CardEffect for ReplaySamePhysicalCard {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.0.clone();
        vec![Effect::when_would_be_deleted(card)
            .replacement_condition(|ctx, subject| subject.permanent() == ctx.source_permanent)
            .replacement_process(move |ctx| {
                let invocation = count.fetch_add(1, Ordering::SeqCst);
                if invocation == 0 {
                    let old = ctx.subject.permanent().unwrap();
                    assert!(ctx.effect.game.return_to_hand_from_effect(old, 0));
                    assert!(ctx.effect.game.players[0].battle_area.is_empty());
                    let position = ctx.effect.game.players[0]
                        .hand
                        .iter()
                        .position(|c| c.handle() == card)
                        .unwrap();
                    let replayed = ctx.effect.play_from_hand_free(0, position).unwrap();
                    assert_eq!(replayed.index, old.index, "same slot reused");
                    assert_eq!(
                        ctx.effect.game.players[0].battle_area[replayed.index as usize]
                            .top_card()
                            .handle(),
                        card
                    );
                    let result = ctx.effect.game.try_replace(
                        EffectTiming::WhenWouldBeDeleted,
                        ReplacementSubject::Permanent(replayed),
                        ReplacementCause::OwnEffect,
                        Some(Zone::Trash),
                    );
                    assert_eq!(
                        result,
                        ReplacementOutcome::Cancelled,
                        "new incarnation gets a fresh choice"
                    );
                }
                ctx.cancel();
            })
            .build()]
    }
}

#[test]
fn replayed_same_card_in_same_slot_is_a_new_immediate_subject() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("REPLAY", "Replay witness"))
        .start();
    r.register_effect("REPLAY", Arc::new(ReplaySamePhysicalCard(count.clone())));
    let original = r.place_on_field(0, "REPLAY", Some(0));
    let outcome = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(original),
        ReplacementCause::OwnEffect,
        Some(Zone::Trash),
    );
    assert_eq!(outcome, ReplacementOutcome::Cancelled);
    assert_eq!(count.load(Ordering::SeqCst), 2);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert!(r.game.players[0].trash.is_empty());
}

struct ReplayWithoutCancelling;
impl CardEffect for ReplayWithoutCancelling {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Return this Digimon to hand, then play that card")
            .optional()
            .replacement_condition(|ctx, subject| subject.permanent() == ctx.source_permanent)
            .replacement_process(move |ctx| {
                let original = ctx.subject.permanent().unwrap();
                assert!(ctx.effect.game.return_to_hand_from_effect(original, 0));
                assert!(ctx.effect.game.players[0].battle_area.is_empty());
                let position = ctx.effect.game.players[0]
                    .hand
                    .iter()
                    .position(|c| c.handle() == card)
                    .unwrap();
                let replayed = ctx.effect.play_from_hand_free(0, position).unwrap();
                assert_eq!(replayed.index, original.index);
                assert_eq!(
                    ctx.effect.game.players[0].battle_area[replayed.index as usize]
                        .top_card()
                        .handle(),
                    card
                );
                // Deliberately leave the original outcome as None. The old
                // deletion must resolve its original incarnation, not this play.
            })
            .build()]
    }
}

#[test]
fn parked_deletion_commit_cannot_remove_replayed_incarnation() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("REPLAY-COMMIT", "Replay before commit"))
        .deck(0, &["REPLAY-COMMIT"; 4])
        .deck(1, &["REPLAY-COMMIT"; 4])
        .memory(5)
        .start();
    r.register_effect("REPLAY-COMMIT", Arc::new(ReplayWithoutCancelling));
    r.set_phase(GamePhase::Main);
    let original = r.place_on_field(0, "REPLAY-COMMIT", Some(0));
    let physical_card = r.game.players[0].battle_area[0].top_card().handle();
    r.game
        .delete_permanent_with_cause(original, ReplacementCause::OpponentEffect);
    let choice = r
        .game
        .pending_selection
        .as_ref()
        .expect("original deletion is parked");
    assert_eq!(choice.kind, SelectionKind::Replacement);
    assert_eq!(choice.source_card, physical_card);
    assert!(choice.is_optional);
    let mut clone = r.game.clone();
    masked(&mut r.game, 0, REPLACEMENT_ACCEPT);
    masked(&mut clone, 0, REPLACEMENT_ACCEPT);
    assert_eq!(to_ui_json(&r.game), to_ui_json(&clone));
    for game in [&r.game, &clone] {
        assert!(
            game.pending_selection.is_none(),
            "the original event finished once"
        );
        assert_eq!(game.current_phase, GamePhase::Main);
        assert_eq!(game.players[0].battle_area.len(), 1);
        assert_eq!(
            game.players[0].battle_area[0].top_card().handle(),
            physical_card
        );
        assert!(game.players[0]
            .trash
            .iter()
            .all(|c| c.handle() != physical_card));
        assert_eq!(game.events.iter().filter(|event| matches!(event,
            digimon_engine::events::GameEvent::Play { card_id, .. } if card_id == "REPLAY-COMMIT"
        )).count(), 1, "one new incarnation was actually played");
    }
}
