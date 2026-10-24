//! A synthetic nested payment proves continuation ownership across ordinary and
//! hand-partner DNA declarations; it is not a printed-card payment claim.
use digimon_engine::{
    action::{build_action_mask, space::{ATTACK_START, PLAY_HAND_START, PASS}},
    card_data::EvoCost,
    card_source::CardHandle,
    debug_runner::{make_test_card, make_test_dna_card, DebugRunner},
    effect::{CardEffect, Effect},
    enums::{CostDelta, GamePhase, PlaySource},
    game::Game,
};

const REDUCERS: &str = r#"
card: DNA-OUTER-REDUCER
name: Outer payment through DNA
kind: tamer
color: [white]
cost: 1
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: DNA-OUTER-RESULT}
    amount: 1
    pay_cost:
      - select_own_permanent:
          filter: {card_number_is: DNA-INNER-FIELD}
          bind_as: dna_field
          prompt: Choose the inner DNA field material
      - select_hand:
          of: you
          filter: {card_number_is: DNA-INNER-RESULT}
          bind_as: dna_result
          prompt: Choose the inner DNA result
      - select_hand:
          of: you
          filter: {card_number_is: DNA-INNER-PARTNER}
          bind_as: dna_partner
          prompt: Choose the inner DNA hand material
      - effect_initiated_dna_digivolve_hand_partner:
          target: dna_field
          hand_partner: dna_partner
          from_hand: dna_result
          cost: printed
          ignore_requirements: false
      - if:
          condition: {effect_digivolved_any_digimon: true}
          then:
            - gain_memory: 4
      - if:
          condition:
            any_permanent: {of: you, zone: [battle_area], card_number_is: DNA-OUTER-BASE}
          then:
            - gain_memory: 1
"#;

const INNER_REDUCER: &str = r#"
card: DNA-INNER-REDUCER
name: Inner DNA payment
kind: tamer
color: [white]
cost: 1
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: DNA-INNER-RESULT}
    amount: 1
    pay_cost:
      - select_own_permanent:
          filter: {card_number_is: DNA-INNER-REDUCER, is_suspended: false}
          bind_as: fee
          prompt: Choose the inner reducer payment
      - suspend: {target: fee}
"#;

struct OptionalDnaWindow;
impl CardEffect for OptionalDnaWindow {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_permanent_would_digivolve(card)
            .name("DNA replacement remains optional").optional()
            .condition(|ctx| ctx.dna_origin())
            .replacement_process(|ctx| ctx.cancel()).build()]
    }
}

fn choose(game: &mut Game, action: u16) {
    let pending = game.pending_selection.as_ref().expect("player-visible decision");
    let player = pending.selecting_player;
    assert_eq!(build_action_mask(game, player)[action as usize], 1.0, "{}", pending.prompt);
    game.resolve_selection(player, action).unwrap();
}

fn accept(game: &mut Game) {
    let action = *game.pending_selection.as_ref().expect("optional reducer")
        .valid_action_ids.iter().find(|&&action| action != PASS).unwrap();
    choose(game, action);
}

fn setup(initial: i16, replacement: bool) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut outer_base = make_test_card("DNA-OUTER-BASE", "Outer base");
    outer_base.level = Some(3);
    let mut outer_result = make_test_card("DNA-OUTER-RESULT", "Outer result");
    outer_result.level = Some(4);
    outer_result.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 3 }];
    let mut field = make_test_card("DNA-INNER-FIELD", "DNA field");
    field.level = Some(6);
    let mut partner = make_test_card("DNA-INNER-PARTNER", "DNA partner");
    partner.level = Some(5);
    let inner = make_test_dna_card("DNA-INNER-RESULT", "Inner DNA result", 6, 5, 1);
    let mut r = DebugRunner::builder().from_dsl_yaml(REDUCERS).unwrap()
        .from_dsl_yaml(INNER_REDUCER).unwrap()
        .add_card(outer_base).add_card(outer_result).add_card(field).add_card(partner).add_card(inner)
        .hand(0, &["DNA-OUTER-RESULT", "DNA-INNER-RESULT", "DNA-INNER-PARTNER"])
        .deck(0, &["DNA-OUTER-BASE"; 16]).deck(1, &["DNA-OUTER-BASE"; 16])
        .memory(initial).start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    let outer = r.place_on_field(0, "DNA-OUTER-BASE", None);
    r.place_on_field(0, "DNA-INNER-FIELD", None);
    r.place_on_field(0, "DNA-OUTER-REDUCER", None);
    r.place_on_field(0, "DNA-INNER-REDUCER", None);
    if replacement { r.register_effect("DNA-INNER-FIELD", std::sync::Arc::new(OptionalDnaWindow)); }
    r.game.tick_declarative_effects();
    (r, outer)
}

fn nested_payment(initial: i16, replacement: bool) {
    let (mut r, outer) = setup(initial, replacement);
    r.game.effect_initiated_digivolve(0, 0, outer, CostDelta::Reduce(0), false, PlaySource::ByEffect);
    accept(&mut r.game);
    choose(&mut r.game, ATTACK_START + 1);
    choose(&mut r.game, PLAY_HAND_START + 1);
    choose(&mut r.game, PLAY_HAND_START + 2);
    accept(&mut r.game);
    let parked = r.game.clone();
    let mut completed = None;
    for _ in 0..2 {
        r.game = parked.clone();
        choose(&mut r.game, ATTACK_START + 3);
        if replacement {
            assert_eq!(r.game.pending_selection.as_ref().unwrap().kind,
                digimon_engine::selection::SelectionKind::Replacement);
            assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "DNA-OUTER-BASE");
            choose(&mut r.game, PASS);
        }
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "DNA-OUTER-RESULT");
        assert_eq!(r.game.players[0].battle_area[1].top_card().card_id(&r.game.card_data), "DNA-INNER-RESULT");
        let memory = if r.game.turn_player() == 0 { r.game.memory } else { -r.game.memory };
        assert_eq!(memory, initial + 3, "inner DNA success tail and enclosing payment tail must precede outer payment");
        assert_eq!(r.game.players[0].hand.len(), 2, "each evolution draws exactly once");
        if let Some(previous) = completed { assert_eq!(r.game.verification_digest(), previous); }
        completed = Some(r.game.verification_digest());
    }
}

#[test]
fn dna_inner_payment_finishes_before_ordinary_outer_payment() { nested_payment(0, false); }

#[test]
fn dna_inner_success_tail_funds_otherwise_unaffordable_outer_payment() { nested_payment(-10, false); }

#[test]
fn dna_inner_replacement_keeps_enclosing_payment_and_result_tail_owned() { nested_payment(0, true); }

#[test]
fn missing_dna_subject_on_replacement_accept_finishes_failed_result_tail() {
    let (mut r, outer) = setup(5, true);
    r.game.effect_initiated_digivolve(0, 0, outer, CostDelta::Reduce(0), false, PlaySource::ByEffect);
    accept(&mut r.game);
    choose(&mut r.game, ATTACK_START + 1);
    choose(&mut r.game, PLAY_HAND_START + 1);
    choose(&mut r.game, PLAY_HAND_START + 2);
    // Decline the inner reducer so this test isolates replacement cleanup,
    // without the second payment hook exercised by the ordering regressions.
    choose(&mut r.game, PASS);
    assert_eq!(r.game.pending_selection.as_ref().unwrap().kind,
        digimon_engine::selection::SelectionKind::Replacement);
    let removed = r.game.players[0].battle_area.remove(1);
    for card in removed.card_sources { r.game.players[0].trash.push(card); }
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        choose(game, digimon_engine::action::space::REPLACEMENT_ACCEPT);
        assert!(game.pending_selection.is_none());
        assert_eq!(game.players[0].battle_area[0].top_card().card_id(&game.card_data), "DNA-OUTER-RESULT");
        assert_eq!(game.memory, 4, "failed inner DNA grants no success bonus, but its enclosing +1 tail still precedes the outer cost 2");
        assert_eq!(game.players[0].hand.len(), 3, "only the outer evolution consumes and draws");
    }
    assert_eq!(r.game.verification_digest(), clone.verification_digest());
}

const OUTERMOST_REDUCER: &str = r#"
card: DNA-OUTERMOST-REDUCER
name: Outermost payment through ordinary evolution
kind: tamer
color: [white]
cost: 1
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: DNA-OUTERMOST-RESULT}
    amount: 1
    pay_cost:
      - select_own_permanent:
          filter: {card_number_is: DNA-OUTER-BASE}
          bind_as: middle_field
          prompt: Choose the middle ordinary base
      - select_hand:
          of: you
          filter: {card_number_is: DNA-OUTER-RESULT}
          bind_as: middle_result
          prompt: Choose the middle ordinary result
      - effect_initiated_digivolve:
          target: middle_field
          from_hand: middle_result
          cost: printed
      - if:
          condition: {effect_digivolved_any_digimon: true}
          then:
            - gain_memory: 2
      - if:
          condition:
            any_permanent: {of: you, zone: [battle_area], card_number_is: DNA-OUTERMOST-BASE}
          then:
            - gain_memory: 1
"#;

#[test]
fn dna_three_deep_payment_preserves_transitive_ordinary_ancestry() {
    for replacement in [false, true] {
        let mut middle_base = make_test_card("DNA-OUTER-BASE", "Middle ordinary base");
        middle_base.level = Some(3);
        let mut middle_result = make_test_card("DNA-OUTER-RESULT", "Middle ordinary result");
        middle_result.level = Some(4);
        middle_result.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 3 }];
        let mut outer_base = make_test_card("DNA-OUTERMOST-BASE", "Outermost ordinary base");
        outer_base.level = Some(3);
        let mut outer_result = make_test_card("DNA-OUTERMOST-RESULT", "Outermost ordinary result");
        outer_result.level = Some(4);
        outer_result.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 3 }];
        let mut field = make_test_card("DNA-INNER-FIELD", "DNA field");
        field.level = Some(6);
        let mut partner = make_test_card("DNA-INNER-PARTNER", "DNA partner");
        partner.level = Some(5);
        let inner = make_test_dna_card("DNA-INNER-RESULT", "Inner DNA result", 6, 5, 1);
        let mut r = DebugRunner::builder().from_dsl_yaml(REDUCERS).unwrap()
            .from_dsl_yaml(INNER_REDUCER).unwrap().from_dsl_yaml(OUTERMOST_REDUCER).unwrap()
            .add_card(middle_base).add_card(middle_result).add_card(outer_base).add_card(outer_result)
            .add_card(field).add_card(partner).add_card(inner)
            .hand(0, &["DNA-OUTERMOST-RESULT", "DNA-OUTER-RESULT", "DNA-INNER-RESULT", "DNA-INNER-PARTNER"])
            .deck(0, &["DNA-OUTER-BASE"; 16]).deck(1, &["DNA-OUTER-BASE"; 16])
            .memory(0).start();
        r.skip_mulligan();
        r.game.current_phase = GamePhase::Main;
        let outer = r.place_on_field(0, "DNA-OUTERMOST-BASE", None);
        r.place_on_field(0, "DNA-OUTER-BASE", None);
        r.place_on_field(0, "DNA-INNER-FIELD", None);
        r.place_on_field(0, "DNA-OUTER-REDUCER", None);
        r.place_on_field(0, "DNA-INNER-REDUCER", None);
        r.place_on_field(0, "DNA-OUTERMOST-REDUCER", None);
        if replacement { r.register_effect("DNA-INNER-FIELD", std::sync::Arc::new(OptionalDnaWindow)); }
        r.game.tick_declarative_effects();
        r.game.effect_initiated_digivolve(0, 0, outer, CostDelta::Reduce(0), false, PlaySource::ByEffect);
        accept(&mut r.game);
        choose(&mut r.game, ATTACK_START + 1);
        choose(&mut r.game, PLAY_HAND_START + 1);
        accept(&mut r.game);
        choose(&mut r.game, ATTACK_START + 2);
        choose(&mut r.game, PLAY_HAND_START + 2);
        choose(&mut r.game, PLAY_HAND_START + 3);
        accept(&mut r.game);
        let parked = r.game.clone();
        let mut completed = None;
        for _ in 0..2 {
            r.game = parked.clone();
            choose(&mut r.game, ATTACK_START + 4);
            if replacement {
                assert_eq!(r.game.pending_selection.as_ref().unwrap().kind,
                    digimon_engine::selection::SelectionKind::Replacement);
                assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "DNA-OUTERMOST-BASE");
                assert_eq!(r.game.players[0].battle_area[1].top_card().card_id(&r.game.card_data), "DNA-OUTER-BASE");
                choose(&mut r.game, PASS);
            }
            assert!(r.game.pending_selection.is_none());
            for (index, result) in ["DNA-OUTERMOST-RESULT", "DNA-OUTER-RESULT", "DNA-INNER-RESULT"].iter().enumerate() {
                assert_eq!(r.game.players[0].battle_area[index].top_card().card_id(&r.game.card_data), *result);
            }
            let memory = if r.game.turn_player() == 0 { r.game.memory } else { -r.game.memory };
            assert_eq!(memory, 4, "DNA and both enclosing payment tails must finish inside-out before their respective costs");
            assert_eq!(r.game.players[0].hand.len(), 3, "three declarations each draw exactly once");
            if let Some(previous) = completed { assert_eq!(r.game.verification_digest(), previous); }
            completed = Some(r.game.verification_digest());
        }
    }
}
