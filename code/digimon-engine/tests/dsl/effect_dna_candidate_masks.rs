//! Correlated result, field and hand-material choices must all lead to a legal DNA.
use digimon_engine::{
    action::{mask::build_action_mask, space::{encode_attack, PASS}},
    card_data::CardData,
    debug_runner::{make_test_card, make_test_dna_card, DebugRunner},
    effect_context::EffectContext,
    enums::{CardKind, EffectTiming},
    selection::TriggerSource,
};

const YAML: &str = r#"
card: DNA-PICKER
name: DNA picker
kind: tamer
color: [white]
cost: 1
effects:
  - when: main_on_field
    summary: "DNA with a field and hand material"
    process:
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter:
            level_eq: 7
            dna_hand_candidate:
              role: result
              field_filter: { kind: digimon }
              cost: printed
          prompt: "Choose result"
      - select_own_permanent:
          bind_as: field
          optional: true
          filter:
            kind: digimon
            dna_hand_candidate:
              role: field
              result: result
              cost: printed
          prompt: "Choose field material"
      - select_hand:
          of: you
          bind_as: partner
          optional: true
          filter:
            kind: digimon
            dna_hand_candidate:
              role: partner
              result: result
              field: field
              cost: printed
          prompt: "Choose hand material"
      - effect_initiated_dna_digivolve_hand_partner:
          target: field
          hand_partner: partner
          from_hand: result
          cost: printed
          ignore_requirements: false
"#;

fn card(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c
}

fn setup() -> (DebugRunner, u16) {
    setup_payment(5, "printed", false)
}

fn setup_payment(memory: i16, cost: &str, reducer: bool) -> (DebugRunner, u16) {
    let mut source = make_test_card("DNA-PICKER", "DNA picker");
    source.card_kind = CardKind::Tamer;
    source.level = None;
    source.dp = None;
    let mut result = make_test_dna_card("DNA-GOOD", "Good result", 6, 5, 3);
    result.level = Some(7);
    let yaml = YAML.replace("cost: printed", &format!("cost: {cost}"));
    let mut r = DebugRunner::builder().from_dsl_yaml(&yaml).unwrap()
        .add_card(source).add_card(result).add_card(card("DNA-BAD", 7))
        .add_card(card("DNA-PARTNER", 5)).add_card(card("DNA-JUNK", 3))
        .add_card(card("DNA-FIELD", 6)).add_card(card("DNA-WRONG", 4))
        .hand(0, &["DNA-GOOD", "DNA-BAD", "DNA-PARTNER", "DNA-JUNK"])
        .deck(0, &["DNA-JUNK"; 8]).memory(5).start();
    r.skip_mulligan();
    let source = r.place_on_field(0, "DNA-PICKER", None);
    let field = r.place_on_field(0, "DNA-FIELD", None);
    r.place_on_field(0, "DNA-WRONG", None);
    r.game.enter_main_phase();
    r.game.memory = memory;
    if reducer {
        let card = r.game.players[0].battle_area[source.index as usize].top_card().handle();
        EffectContext::new(&mut r.game, card, Some(source), 0)
            .arm_player_digivolve_cost_reducer(2, true, None, true);
    }
    r.game.enqueue_triggered(EffectTiming::MainOnField, TriggerSource::Permanent(source));
    r.game.drain_effect_queue();
    (r, encode_attack(0, field.index as u16))
}

#[test]
fn result_probe_uses_declared_cost_policy_and_does_not_consume_optional_payment() {
    for (cost, reducer, expected) in [
        ("printed", false, false), ("free", false, true), ("1", false, true),
        ("{ reduce: 2 }", false, true), ("printed", true, true),
    ] {
        let (r, _) = setup_payment(-9, cost, reducer);
        assert_eq!(r.pending_selection().is_some(), expected, "cost={cost}, reducer={reducer}");
        assert_eq!(r.game.memory, -9, "candidate probing cannot spend or gain memory");
        assert_eq!(r.game.player_digivolve_cost_reducers.len(), usize::from(reducer));
        assert!(r.game.players[0].battle_area.iter().all(|p| !p.is_suspended));
        assert_eq!(r.game.players[0].hand.len(), 4);
        if expected { assert_eq!(r.pending_selection_view().unwrap().valid_action_ids, [0]); }
    }
}

#[test]
fn reducer_enabled_candidate_completes_the_same_player_visible_payment() {
    let (mut r, field) = setup_payment(-9, "printed", true);
    r.execute_action(0, 0).unwrap();
    r.execute_action(0, field).unwrap();
    assert_eq!(r.pending_selection_view().unwrap().valid_action_ids, [2]);
    r.execute_action(0, 2).unwrap();
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        let accept = game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        assert!(build_action_mask(game, 0)[accept as usize] > 0.0);
        assert!(build_action_mask(game, 0)[PASS as usize] > 0.0);
        game.resolve_selection(0, accept).unwrap();
        let suspend = game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        assert!(build_action_mask(game, 0)[suspend as usize] > 0.0);
        game.resolve_selection(0, suspend).unwrap();
        assert!(game.pending_selection.is_none());
        let relative = if game.turn_player() == 0 { game.memory } else { -game.memory };
        assert_eq!(relative, -10, "printed 3 less the actual paid reduction of 2");
        assert!(game.player_digivolve_cost_reducers.is_empty());
        assert!(game.players[0].battle_area.iter().any(|p| p.top_card().card_id(&game.card_data) == "DNA-GOOD"));
    }
    assert_eq!(r.game.verification_digest(), cloned.verification_digest());
}

#[test]
fn prior_result_binding_follows_hand_reorder_but_not_departure() {
    for depart in [false, true] {
        let (mut r, field) = setup();
        r.execute_action(0, 0).unwrap();
        let mut cloned = r.game.clone();
        for game in [&mut r.game, &mut cloned] {
            if depart {
                let card = game.players[0].hand.remove(0);
                game.players[0].trash.push(card);
            } else { game.players[0].hand.swap(0, 3); }
            game.resolve_selection(0, field).unwrap();
            if depart {
                assert!(game.pending_selection.is_none());
                assert_eq!(game.memory, 5);
                assert_eq!(game.players[0].battle_area.len(), 3);
            } else {
                assert_eq!(game.pending_selection.as_ref().unwrap().valid_action_ids, [2]);
                game.resolve_selection(0, 2).unwrap();
                assert!(game.pending_selection.is_none());
                assert_eq!(game.memory, 2);
                assert_eq!(game.players[0].battle_area[1].top_card().card_id(&game.card_data), "DNA-GOOD");
            }
        }
        assert_eq!(r.game.verification_digest(), cloned.verification_digest());
    }
}

#[test]
fn prior_field_binding_follows_compaction_but_not_departure() {
    for depart in [false, true] {
        let (mut r, field) = setup();
        r.execute_action(0, 0).unwrap();
        r.execute_action(0, field).unwrap();
        let mut cloned = r.game.clone();
        for game in [&mut r.game, &mut cloned] {
            let removed = game.players[0].battle_area.remove(if depart { 1 } else { 0 });
            game.players[0].trash.extend(removed.card_sources);
            game.resolve_selection(0, 2).unwrap();
            assert!(game.pending_selection.is_none());
            if depart {
                assert_eq!(game.memory, 5);
                assert_eq!(game.players[0].hand.len(), 4);
            } else {
                assert_eq!(game.memory, 2);
                assert_eq!(game.players[0].battle_area[0].top_card().card_id(&game.card_data), "DNA-GOOD");
            }
        }
        assert_eq!(r.game.verification_digest(), cloned.verification_digest());
    }
}

#[test]
fn result_choice_excludes_cards_without_a_complete_legal_triple() {
    let (r, _) = setup();
    assert_eq!(r.pending_selection_view().unwrap().valid_action_ids, vec![0]);
    assert!(build_action_mask(&r.game, 0)[PASS as usize] > 0.0);
}

#[test]
fn field_choice_is_correlated_with_selected_result_and_remaining_hand() {
    let (mut r, field) = setup();
    r.execute_action(0, 0).unwrap();
    assert_eq!(r.pending_selection_view().unwrap().valid_action_ids, vec![field]);
}

#[test]
fn partner_choice_excludes_result_and_incompatible_cards_on_live_and_clone() {
    let (mut r, field) = setup();
    r.execute_action(0, 0).unwrap();
    r.execute_action(0, field).unwrap();
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        assert_eq!(game.pending_selection.as_ref().unwrap().valid_action_ids, vec![2]);
        assert!(build_action_mask(game, 0)[PASS as usize] > 0.0);
        game.resolve_selection(0, PASS).unwrap();
        assert!(game.pending_selection.is_none());
        assert_eq!(game.players[0].hand.len(), 4);
        assert_eq!(game.memory, 5);
    }
    assert_eq!(r.game.verification_digest(), cloned.verification_digest());
}
