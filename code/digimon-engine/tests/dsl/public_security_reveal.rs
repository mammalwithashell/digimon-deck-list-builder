//! Public knowledge of security is distinct from its rules-facing face state.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::determinize::{materialize, Infoset};
use digimon_engine::serialization::to_ui_json;

fn fixture() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("A", "A"))
        .add_card(make_test_card("B", "B"))
        .add_card(make_test_card("C", "C"))
        .security(0, &["A", "B"]).security(1, &["B", "C"])
        .deck(0, &["C", "C", "C"]).deck(1, &["A", "A", "A"])
        .start()
}

const SECURITY_WITNESS: &str = r#"
card: PUBLIC-SECURITY-WITNESS
name: Public security witness
kind: option
color: [yellow]
cost: 0
effects:
  - scope: security
    when: on_security
    process:
      - gain_memory: 2
  - scope: security
    when: on_lose_security
    process:
      - gain_memory: 3
  - scope: security
    when: on_discard_security
    process:
      - gain_memory: 5
"#;

#[test]
fn revealing_does_not_activate_security_or_security_removal_effects() {
    let witness = SECURITY_WITNESS;
    for owner in [0, 1] {
        let mut r = DebugRunner::builder().from_dsl_yaml(witness).unwrap()
            .security(owner, &["PUBLIC-SECURITY-WITNESS"]).memory(5).start();
        r.game.reveal_security(owner);
        r.game.drain_effect_queue();
        r.auto_resolve().unwrap();
        assert_eq!(r.game.memory, 5);
        assert_eq!(r.game.players[owner as usize].security.len(), 1);
        assert!(r.game.players[owner as usize].face_up_security.is_empty());
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn reveal_security_lowering_from_masked_hand_play_reveals_only_the_opponent_stack() {
    use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
    let revealer = r#"
card: PUBLIC-SECURITY-REVEALER
name: Public security revealer
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 1000
effects:
  - when: on_play
    process:
      - reveal_security: { of: opponent }
"#;
    let mut r = DebugRunner::builder().from_dsl_yaml(SECURITY_WITNESS).unwrap()
        .from_dsl_yaml(revealer).unwrap()
        .hand(0, &["PUBLIC-SECURITY-REVEALER"])
        .security(0, &["PUBLIC-SECURITY-WITNESS"])
        .security(1, &["PUBLIC-SECURITY-WITNESS", "PUBLIC-SECURITY-WITNESS"])
        .memory(5).start();
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    let handles: Vec<_> = r.game.players[1].security.iter().map(|card| card.handle()).collect();
    r.game.drain_events();
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.memory, 5, "zero-cost play never activates security witnesses");
    assert_eq!(r.game.players[1].security.iter().map(|card| card.handle()).collect::<Vec<_>>(), handles);
    assert_eq!(r.game.players[0].security.len(), 1);
    assert!(r.game.players.iter().all(|player| player.face_up_security.is_empty()));
    assert!(r.game.revealed_cards.is_empty());
    let ui = to_ui_json(&r.game);
    assert!(ui["player1"]["securityIds"][0].is_null());
    assert!(ui["player1"]["securityIdentityPublic"][0].as_bool() == Some(false));
    assert!(ui["player2"]["securityIds"].as_array().unwrap().iter().all(|id| id.as_str() == Some("PUBLIC-SECURITY-WITNESS")));
    assert!(ui["player2"]["securityIdentityPublic"].as_array().unwrap().iter().all(|bit| bit.as_bool() == Some(true)));
    assert!(ui["player2"]["securityFaceUp"].as_array().unwrap().iter().all(|bit| bit.as_bool() == Some(false)));
    let public_reveals: Vec<_> = r.game.events.iter().filter_map(|event| match event {
        digimon_engine::events::GameEvent::Reveal {
            player, card_id, source_zone: digimon_engine::events::RevealZone::SecurityStack, ..
        } => Some((*player, card_id.as_str())),
        _ => None,
    }).collect();
    assert_eq!(public_reveals, vec![(1, "PUBLIC-SECURITY-WITNESS"); 2]);
    assert!(!r.game.events.iter().any(|event| matches!(event,
        digimon_engine::events::GameEvent::SecurityReveal { .. })));
}

#[test]
fn public_reveal_preserves_security_area_and_face_state_for_both_players() {
    for owner in [0, 1] {
        let mut r = fixture();
        let handles: Vec<_> = r.game.players[owner as usize].security.iter().map(|c| c.handle()).collect();
        r.game.drain_events();
        r.game.reveal_security(owner);
        assert_eq!(r.game.players[owner as usize].security.iter().map(|c| c.handle()).collect::<Vec<_>>(), handles);
        assert!(r.game.players[owner as usize].face_up_security.is_empty());
        assert!(r.game.revealed_cards.is_empty());
        let state = to_ui_json(&r.game);
        let key = if owner == 0 { "player1" } else { "player2" };
        assert!(state[key]["securityIds"].as_array().unwrap().iter().all(|x| x.is_string()));
        assert!(state[key]["securityIdentityPublic"].as_array().unwrap().iter().all(|x| x.as_bool() == Some(true)));
        assert!(state[key]["securityFaceUp"].as_array().unwrap().iter().all(|x| x.as_bool() == Some(false)));
        let events = r.game.drain_events();
        assert_eq!(events.len(), 2, "revealing does not fire security checks/removals/placement");
        assert!(events.iter().all(|event| matches!(event, digimon_engine::events::GameEvent::Reveal {
            source_zone: digimon_engine::events::RevealZone::SecurityStack, ..
        })));
        for viewer in [0, 1] {
            let info = Infoset::extract(&r.game, viewer);
            assert_eq!(info.public[owner as usize].security_pins.len(), 2);
            let world = materialize(&r.game, viewer, 99);
            assert_eq!(world.players[owner as usize].security.iter().map(|c| c.handle()).collect::<Vec<_>>(), handles);
            assert_eq!(Infoset::extract(&world, viewer), info);
        }
    }
}

#[test]
fn public_but_face_down_security_preserves_existing_tensor_contract() {
    use digimon_engine::card_registry::CardRegistry;
    use digimon_engine::tensor_profiles::standard::v2_lite as layout;
    let mut r = fixture();
    let cards = r.game.card_data.iter().map(|card| (card.card_id.clone(), card.clone())).collect();
    let registry = CardRegistry::from_cards(&cards);
    for owner in [0, 1] { r.game.reveal_security(owner); }
    for viewer in [0, 1] {
        let tensor = digimon_engine::tensor_v2_lite::build_tensor_standard_lite_v2(&r.game, viewer, &registry);
        assert_eq!(tensor.len(), layout::TENSOR_SIZE);
        let compact = digimon_engine::tensor_v1::build_tensor_standard_compact_v1(&r.game, viewer, &registry);
        for start in [digimon_engine::tensor_profiles::standard::v1::OFF_MY_SECURITY,
            digimon_engine::tensor_profiles::standard::v1::OFF_OPP_SECURITY] {
            assert_eq!(&compact[start..start + 2], &[0.0, 0.0]);
        }
        for (row, owner) in [(90, viewer), (100, 1 - viewer)] {
            for index in 0..r.game.players[owner as usize].security.len() {
                let offset = layout::OFF_KNOWN_ZONE_CARDS + (row + index) * layout::KNOWN_ZONE_ROW_SIZE + layout::KNOWN_ZONE_CARD_ID_OFFSET;
                assert_eq!(tensor[offset], 0.0, "active contract encodes actual face-up security only");
            }
        }
    }
    r.game.shuffle_security(0);
    let tensor = digimon_engine::tensor_v2_lite::build_tensor_standard_lite_v2(&r.game, 1, &registry);
    for index in 0..2 {
        let offset = layout::OFF_KNOWN_ZONE_CARDS + (100 + index) * layout::KNOWN_ZONE_ROW_SIZE + layout::KNOWN_ZONE_CARD_ID_OFFSET;
        assert_eq!(tensor[offset], 0.0);
    }
    let mut alternate = r.game.clone();
    let security = alternate.players[0].security.remove(0);
    let deck = alternate.players[0].deck.pop().unwrap();
    alternate.players[0].put_on_security_top(deck);
    alternate.players[0].put_on_deck_top(security);
    for viewer in [0, 1] {
        assert_eq!(Infoset::extract(&r.game, viewer), Infoset::extract(&alternate, viewer));
        assert_eq!(
            digimon_engine::tensor_v2_lite::build_tensor_standard_lite_v2(&r.game, viewer, &registry),
            digimon_engine::tensor_v2_lite::build_tensor_standard_lite_v2(&alternate, viewer, &registry),
            "after shuffle, hidden security/deck assignments cannot retain revealed identities",
        );
    }
}

#[test]
fn shuffle_revokes_temporary_public_security_knowledge() {
    let mut r = fixture();
    r.game.reveal_security(0);
    let mut branch = r.game.clone();
    branch.shuffle_security(0);
    assert!(to_ui_json(&r.game)["player1"]["securityIds"][0].is_string(), "clone revocation is independent");
    r.game.shuffle_security(0);
    assert!(to_ui_json(&r.game)["player1"]["securityIds"].as_array().unwrap().iter().all(|x| x.is_null()));
    for viewer in [0, 1] { assert!(Infoset::extract(&r.game, viewer).public[0].security_pins.is_empty()); }
}

#[test]
fn irrelevant_arrival_history_does_not_change_ordinary_verification_digest() {
    let mut r = fixture();
    let before = r.game.verification_digest();
    let card = r.game.players[0].security.pop().unwrap();
    r.game.players[0].put_on_security_top(card);
    assert_eq!(r.game.verification_digest(), before);
    r.game.reveal_security(0);
    assert_ne!(r.game.verification_digest(), before, "active public knowledge is resolver state");
}

#[test]
fn departed_security_handle_cannot_regain_public_identity_by_reentry() {
    let mut r = fixture();
    r.game.reveal_security(0);
    let card = r.game.players[0].security.remove(0);
    let handle = card.handle();
    r.game.players[0].put_in_hand(card);
    assert!(!r.game.security_card_is_public(0, handle));
    let card = r.game.players[0].hand.pop().unwrap();
    r.game.players[0].put_on_security_top(card);
    assert!(!r.game.security_card_is_public(0, handle));
    assert!(to_ui_json(&r.game)["player1"]["securityIds"][1].is_null());
    assert_eq!(Infoset::extract(&r.game, 1).public[0].security_pins.len(), 1);
}

#[test]
fn rejected_parked_security_play_preserves_public_identity_at_original_position() {
    struct CancelPlay;
    impl digimon_engine::effect::CardEffect for CancelPlay {
        fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<digimon_engine::effect::Effect> {
            vec![digimon_engine::effect::Effect::when_permanent_would_play(card)
                .optional().replacement_process(|ctx| ctx.cancel()).build()]
        }
    }
    let mut r = fixture();
    r.register_effect("C", std::sync::Arc::new(CancelPlay));
    r.place_on_field(1, "C", Some(0));
    r.game.reveal_security(0);
    let card = r.game.players[0].security[0].handle();
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, card, None, 0);
        assert!(ctx.play_from_security_card(0, card).is_none());
    }
    assert!(r.pending_selection_view().expect("parked play cancellation").is_optional);
    r.accept_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].security[0].handle(), card);
    assert!(r.game.security_card_is_public(0, card), "rejected extraction is not a new security arrival");
}
