#![cfg(feature = "dsl-yaml-loader")]
use digimon_engine::debug_runner::{DebugRunner, make_test_card};
use digimon_engine::dcgo_recording::SelectionRow;
use digimon_engine::runners::selection_resolve::resolve_next;
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

#[test]
fn any_field_target_preserves_opponent_owner_when_both_slots_are_legal() {
    let yaml = r#"
card: TEST-ANY
name: Any Field
kind: digimon
level: 3
color: [green]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_any_permanent:
          filter: { is_unsuspended: true }
          bind_as: target
          prompt: "Choose a permanent"
      - suspend: { target: target }
"#;
    for controller in [0, 1] {
        let mut r = DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
            .add_card(make_test_card("PAD", "Padding"))
            .deck(0, &["PAD"; 12]).deck(1, &["PAD"; 12]).memory(5).start();
        r.skip_mulligan();
        let own = r.place_on_field(controller, "TEST-ANY", Some(0));
        let opp = r.place_on_field(1-controller, "PAD", Some(0));
        r.game.enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(own));
        r.game.drain_effect_queue();
        let row: SelectionRow = serde_json::from_value(serde_json::json!({
            "step": 0, "actor": controller, "prompt": "SelectPermanentEffect", "phase": "Main",
            "targets": [{"player": opp.player, "frame": 0}]
        })).unwrap();
        let action = resolve_next(&r.game, &row, 0).unwrap().unwrap();
        let mut clone = r.game.clone();
        for game in [&mut r.game, &mut clone] {
            assert!(digimon_engine::action::build_action_mask(game, controller)[action as usize] > 0.5);
            game.decode_action(action, controller);
            assert!(game.players[opp.player as usize].battle_area[0].is_suspended, "selected opponent must suspend");
            assert!(!game.players[own.player as usize].battle_area[0].is_suspended, "same-slot own card must remain unsuspended");
        }
    }
}

#[test]
fn compact_remainder_keeps_identity_when_play_cost_passes_turn() {
    use digimon_engine::card_source::CardSource;
    use digimon_engine::action::space::PLAY_HAND_START;
    use digimon_engine::enums::GamePhase;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../cards/ex3/EX3-037.yaml")).unwrap()
        .add_card(make_test_card("REST-A", "Plain A"))
        .add_card(make_test_card("REST-B", "Plain B"))
        .deck(0, &["REST-A"; 12]).deck(1, &["REST-A"; 12])
        .hand(0, &["EX3-037"]).memory(0).start();
    r.skip_mulligan();
    r.set_phase(GamePhase::Main);
    r.game.players[0].deck.clear();
    for id in ["REST-B", "REST-A"] {
        let data = r.game.card_data.iter().position(|c| c.card_id == id).unwrap();
        let serial = r.game.next_card_index();
        r.game.players[0].deck.push(CardSource::new(data, 0, serial));
    }
    r.game.decode_action(PLAY_HAND_START, 0);
    let row: SelectionRow = serde_json::from_value(serde_json::json!({
        "step": 0, "actor": 0, "prompt": "SelectCardEffect", "phase": "Main",
        "card_ids": ["REST-A", "REST-B"]
    })).unwrap();
    assert!(r.game.pending_selection.is_some());
    let first = resolve_next(&r.game, &row, 0).unwrap().unwrap();
    r.game.decode_action(first, 0);
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        let second = resolve_next(game, &row, 1).expect("second remainder identity must survive").unwrap();
        game.decode_action(second, 0);
        assert!(game.pending_selection.is_none());
        assert_eq!(game.players[0].deck.len(), 2);
    }
}

#[test]
fn blast_dna_field_material_uses_parked_defender_and_raw_slot() {
    use digimon_engine::selection::{PendingSelection, SelectionKind};
    use digimon_engine::resume::{ResumeStack, ResumeFrame, CombatBlastDnaFieldMaterialState};
    use digimon_engine::enums::{GamePhase, EffectSourceKind};
    for defender in [0, 1] {
        let mut r = DebugRunner::builder().add_card(make_test_card("PAD", "Padding"))
            .deck(0, &["PAD";12]).deck(1, &["PAD";12]).start();
        r.skip_mulligan();
        let h = r.place_on_field(defender, "PAD", Some(0));
        let source_card = r.game.players[defender as usize].battle_area[0].top_card().handle();
        r.game.pending_selection = Some(PendingSelection {
            card_choice: None,
            zone_owner: None, kind: SelectionKind::Material, selecting_player: defender,
            previous_phase: GamePhase::Main, valid_action_ids: vec![0], is_optional: false,
            prompt: "Select Blast DNA field material".into(), effect_choices: None,
            source_card, source_permanent: Some(h), source_kind: EffectSourceKind::Digimon,
            callback: Box::new(|_, _| {}), on_decline: None,
        });
        r.game.pending_selection_resume = Some(ResumeStack { frames: vec![
            ResumeFrame::CounterBlastDnaFieldMaterial(CombatBlastDnaFieldMaterialState {
                defender, result_hand_index: 0, source_card, previous_phase: GamePhase::Main,
                outer_conts: vec![],
            })
        ]});
        let row = |player| -> SelectionRow { serde_json::from_value(serde_json::json!({
            "step": 0, "actor": defender, "prompt": "SelectPermanentEffect", "phase": "Main",
            "targets": [{"player": player, "frame": 0}]
        })).unwrap() };
        for game in [&r.game, &r.game.clone()] {
            assert_eq!(resolve_next(game, &row(defender), 0), Ok(Some(0)));
            assert!(resolve_next(game, &row(1-defender), 0).is_err());
        }
    }
}
