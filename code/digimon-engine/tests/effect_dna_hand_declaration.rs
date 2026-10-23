//! Effect DNA declares both physical materials before paying its chosen route.
//! CR 4.3 8-2-2-2 and 8-2-3; EX6-072 official Q3822.
use digimon_engine::{
    action::{mask::build_action_mask, space::PASS},
    card_source::CardHandle,
    debug_runner::{make_test_card, make_test_dna_card, DebugRunner},
    effect_context::EffectContext,
    enums::{CardColor, GamePhase},
    game::Game,
    permanent::PermanentHandle,
};

fn fixture(owner: u8, field_is_upper: bool, cost: i16) -> (DebugRunner, PermanentHandle, CardHandle, CardHandle) {
    let mut field = make_test_card("DNA-FIELD", "Field material");
    field.level = Some(6);
    field.colors = vec![CardColor::Green];
    let mut partner = make_test_card("DNA-PARTNER", "Hand material");
    partner.level = Some(5);
    let result = if field_is_upper {
        make_test_dna_card("DNA-RESULT", "DNA result", 6, 5, cost)
    } else {
        make_test_dna_card("DNA-RESULT", "DNA result", 5, 6, cost)
    };
    let mut r = DebugRunner::builder()
        .add_card(field).add_card(partner).add_card(result)
        .add_card(make_test_card("DNA-LOWER", "Original lower source"))
        .add_card(make_test_card("DNA-DRAW", "Draw"))
        .hand(owner, &["DNA-RESULT", "DNA-PARTNER", "DNA-LOWER"])
        .deck(owner, &["DNA-DRAW"; 8]).memory(1).start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    let field = r.place_on_field(owner, "DNA-FIELD", None);
    let lower = r.game.players[owner as usize].hand.pop().unwrap();
    r.game.players[owner as usize].battle_area[field.index as usize].card_sources.insert(0, lower);
    let result = r.game.players[owner as usize].hand[0].handle();
    let partner = r.game.players[owner as usize].hand[1].handle();
    (r, field, partner, result)
}

fn declare(g: &mut Game, field: PermanentHandle, partner: CardHandle, result: CardHandle, cost: i32) -> Option<PermanentHandle> {
    let source = g.players[field.player as usize].battle_area[field.index as usize].top_card().handle();
    EffectContext::new(g, source, Some(field), field.player)
        .effect_initiated_dna_digivolve_with_hand_partner(field, partner, result, cost, false)
}

#[test]
fn hand_partner_stacks_field_above_hand_when_field_matches_left_requirement() {
    let (mut r, field, partner, result) = fixture(0, true, 0);
    assert_eq!(declare(&mut r.game, field, partner, result, 0), Some(field));
    let ids: Vec<_> = r.game.players[0].battle_area[field.index as usize].card_sources.iter()
        .map(|card| card.card_id(&r.game.card_data)).collect();
    assert_eq!(ids, ["DNA-PARTNER", "DNA-LOWER", "DNA-FIELD", "DNA-RESULT"]);
}

#[test]
fn hand_partner_stacks_hand_above_field_when_hand_matches_left_requirement() {
    let (mut r, field, partner, result) = fixture(0, false, 0);
    assert_eq!(declare(&mut r.game, field, partner, result, 0), Some(field));
    let ids: Vec<_> = r.game.players[0].battle_area[field.index as usize].card_sources.iter()
        .map(|card| card.card_id(&r.game.card_data)).collect();
    assert_eq!(ids, ["DNA-LOWER", "DNA-FIELD", "DNA-PARTNER", "DNA-RESULT"]);
}

#[test]
fn hand_partner_dna_pays_from_inactive_owner_and_draws_once() {
    let (mut r, field, partner, result) = fixture(1, false, 3);
    assert_eq!(r.game.turn_player(), 0);
    let before_deck = r.game.players[1].deck.len();
    assert!(declare(&mut r.game, field, partner, result, 3).is_some());
    assert_eq!(r.game.memory, 4, "inactive owner pays toward the active player's side");
    assert_eq!(r.game.players[1].deck.len(), before_deck - 1);
}

#[test]
fn opposing_effect_controller_dna_uses_the_material_owners_hand_and_memory() {
    let (mut r, field, partner, result) = fixture(1, false, 3);
    let source = r.place_on_field(0, "DNA-FIELD", None);
    let source_card = r.game.players[0].battle_area[source.index as usize].top_card().handle();
    let before_deck = r.game.players[1].deck.len();
    assert_eq!(EffectContext::new(&mut r.game, source_card, Some(source), 0)
        .effect_initiated_dna_digivolve_with_hand_partner(field, partner, result, 3, false), Some(field));
    assert_eq!(r.game.players[1].battle_area[field.index as usize].top_card().handle(), result);
    assert_eq!(r.game.memory, 4, "inactive material owner pays cost three, independent of the effect controller");
    assert_eq!(r.game.players[1].deck.len(), before_deck - 1);
    assert_eq!(r.game.players[1].hand.len(), 1);
    assert_eq!(r.game.players[0].battle_area[source.index as usize].top_card().handle(), source_card);
}

struct OpposingDnaWindow { target: PermanentHandle }
impl digimon_engine::effect::CardEffect for OpposingDnaWindow {
    fn effects(&self, card: CardHandle) -> Vec<digimon_engine::effect::Effect> {
        let target = self.target;
        vec![digimon_engine::effect::Effect::when_permanent_would_digivolve(card)
            .name("Opposing source controller is retained through DNA")
            .optional()
            .condition(move |ctx| ctx.source_permanent == Some(target) && ctx.dna_origin()
                && ctx.replacement_cause() == Some(digimon_engine::replacement::ReplacementCause::OpponentEffect)
                && ctx.replacement_source_controller() == Some(0)
                && ctx.replacement_subject_controller() == Some(1))
            .replacement_process(|ctx| ctx.cancel()).build()]
    }
}

#[test]
fn opposing_effect_controller_dna_keeps_replacement_cause_and_controller_on_resume() {
    let (mut r, field, partner, result) = fixture(1, false, 3);
    let source = r.place_on_field(0, "DNA-FIELD", None);
    let source_card = r.game.players[0].battle_area[source.index as usize].top_card().handle();
    let original_top = r.game.players[1].battle_area[field.index as usize].top_card().handle();
    r.register_effect("DNA-FIELD", std::sync::Arc::new(OpposingDnaWindow { target: field }));
    assert!(EffectContext::new(&mut r.game, source_card, Some(source), 0)
        .effect_initiated_dna_digivolve_with_hand_partner(field, partner, result, 3, false).is_none());
    let pending = r.game.pending_selection.as_ref().expect("opposing-effect replacement must be offered");
    assert_eq!(pending.selecting_player, 1);
    assert_eq!(pending.kind, digimon_engine::selection::SelectionKind::Replacement);
    for cancel in [false, true] {
        let mut game = r.game.clone();
        let action = if cancel { digimon_engine::action::space::REPLACEMENT_ACCEPT } else { PASS };
        assert_eq!(build_action_mask(&game, 1)[action as usize], 1.0);
        game.resolve_selection(1, action).unwrap();
        assert!(game.pending_selection.is_none());
        assert_eq!(game.players[1].battle_area[field.index as usize].top_card().handle(),
            if cancel { original_top } else { result });
        assert_eq!(game.memory, if cancel { 1 } else { 4 });
        assert_eq!(game.players[1].deck.len(), if cancel { 8 } else { 7 });
        assert_eq!(game.players[0].battle_area[source.index as usize].top_card().handle(), source_card);
    }
}

#[test]
fn hand_partner_dna_surfaces_optional_reducer_then_clone_safe_suspend_payment() {
    let (mut r, field, partner, result) = fixture(0, false, 3);
    r.game.memory = -9;
    let source = r.game.players[0].battle_area[field.index as usize].top_card().handle();
    EffectContext::new(&mut r.game, source, Some(field), 0)
        .arm_player_digivolve_cost_reducer(5, true, Some(CardColor::Green), true);
    assert!(declare(&mut r.game, field, partner, result, 3).is_none(), "payment can park");
    let pending = r.game.pending_selection.as_ref().expect("DNA must expose eligible optional reducer");
    let accept = pending.valid_action_ids[0];
    let mask = build_action_mask(&r.game, 0);
    assert!(mask[accept as usize] > 0.0);
    assert!(mask[PASS as usize] > 0.0);
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        game.resolve_selection(0, accept).unwrap();
        let pending = game.pending_selection.as_ref().expect("suspend cost stays player-visible");
        let suspend = pending.valid_action_ids[0];
        assert!(build_action_mask(game, 0)[suspend as usize] > 0.0);
        game.resolve_selection(0, suspend).unwrap();
        assert!(game.pending_selection.is_none());
        assert_eq!(game.players[0].battle_area[field.index as usize].top_card().handle(), result);
        let owner_memory = if game.turn_player() == 0 { game.memory } else { -game.memory };
        assert_eq!(owner_memory, -9);
        assert!(!game.players[0].battle_area[field.index as usize].is_suspended, "DNA creates an unsuspended object");
    }
    assert_eq!(r.game.verification_digest(), cloned.verification_digest());
}

#[test]
fn hand_partner_rejects_a_field_material_owned_by_the_other_player() {
    let (mut r, field, partner, result) = fixture(0, false, 0);
    let foreign = r.place_on_field(1, "DNA-FIELD", None);
    let source = r.game.players[0].battle_area[field.index as usize].top_card().handle();
    let before = r.game.verification_digest();
    let outcome = EffectContext::new(&mut r.game, source, Some(field), 0)
        .effect_initiated_dna_digivolve_with_hand_partner(foreign, partner, result, 0, false);
    assert!(outcome.is_none());
    assert_eq!(r.game.verification_digest(), before, "rejection does not touch either owner's cards");
}

#[test]
fn hand_partner_selects_the_requested_physical_duplicate_and_never_the_result_itself() {
    let (mut r, field, original_partner, result) = fixture(0, false, 0);
    let duplicate = r.add_to_hand(0, "DNA-PARTNER");
    let picked_partner = r.game.players[0].hand[duplicate].handle();
    assert_ne!(picked_partner, original_partner);
    let before = r.game.verification_digest();
    assert!(declare(&mut r.game, field, result, result, 0).is_none());
    assert_eq!(r.game.verification_digest(), before);
    assert!(declare(&mut r.game, field, picked_partner, result, 0).is_some());
    assert!(r.game.players[0].hand.iter().any(|card| card.handle() == original_partner));
    assert!(r.game.players[0].battle_area[field.index as usize].card_sources.iter()
        .any(|card| card.handle() == picked_partner));
}

struct OptionalDnaCancel;
impl digimon_engine::effect::CardEffect for OptionalDnaCancel {
    fn effects(&self, card: CardHandle) -> Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::Effect::when_permanent_would_digivolve(card)
            .name("Optional DNA replacement")
            .optional()
            .condition(|ctx| ctx.dna_origin())
            .replacement_process(|ctx| ctx.cancel())
            .build()]
    }
}

#[test]
fn dna_would_digivolve_accept_cancels_without_moving_or_paying() {
    let (mut r, field, partner, result) = fixture(0, false, 3);
    r.register_effect("DNA-FIELD", std::sync::Arc::new(OptionalDnaCancel));
    let before_hand = r.game.players[0].hand.iter().map(|c| c.handle()).collect::<Vec<_>>();
    let before_deck = r.game.players[0].deck.len();
    assert!(declare(&mut r.game, field, partner, result, 3).is_none());
    assert_eq!(r.game.pending_selection.as_ref().expect("replacement precedes DNA commit").kind,
        digimon_engine::selection::SelectionKind::Replacement);
    r.game.resolve_selection(0, digimon_engine::action::space::REPLACEMENT_ACCEPT).unwrap();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[0].hand.iter().map(|c| c.handle()).collect::<Vec<_>>(), before_hand);
    assert_eq!(r.game.players[0].deck.len(), before_deck);
    assert_eq!(r.game.memory, 1);
}

#[test]
fn dna_would_digivolve_decline_and_clone_resume_exact_pinned_recipe() {
    let (mut r, field, partner, result) = fixture(0, true, 3);
    r.game.memory = 5;
    r.register_effect("DNA-FIELD", std::sync::Arc::new(OptionalDnaCancel));
    assert!(declare(&mut r.game, field, partner, result, 3).is_none());
    assert!(r.game.pending_selection.is_some(), "DNA has a replacement choice");
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        game.resolve_selection(0, PASS).unwrap();
        assert_eq!(game.players[0].battle_area[field.index as usize].top_card().handle(), result);
        assert_eq!(game.players[0].battle_area[field.index as usize].card_sources[0].handle(), partner);
        assert_eq!(game.memory, 2);
        assert!(game.pending_selection.is_none());
    }
    assert_eq!(r.game.verification_digest(), clone.verification_digest());
}

#[test]
fn dna_replacement_revalidates_the_exact_hand_partner_before_paying() {
    let (mut r, field, partner, result) = fixture(0, false, 3);
    r.game.memory = 5;
    r.register_effect("DNA-FIELD", std::sync::Arc::new(OptionalDnaCancel));
    let duplicate = r.add_to_hand(0, "DNA-PARTNER");
    let duplicate = r.game.players[0].hand[duplicate].handle();
    assert!(declare(&mut r.game, field, partner, result, 3).is_none());
    let index = r.game.players[0].hand.iter().position(|card| card.handle() == partner).unwrap();
    let removed = r.game.players[0].hand.remove(index);
    r.game.players[0].trash.push(removed);
    let before_deck = r.game.players[0].deck.len();
    r.game.resolve_selection(0, PASS).unwrap();
    assert_eq!(r.game.memory, 5);
    assert_eq!(r.game.players[0].deck.len(), before_deck);
    assert!(r.game.players[0].hand.iter().any(|card| card.handle() == result));
    assert!(r.game.players[0].hand.iter().any(|card| card.handle() == duplicate));
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn dna_replacement_revalidates_result_and_memory_bound_after_the_pause() {
    for remove_result in [true, false] {
        let (mut r, field, partner, result) = fixture(0, false, 3);
        r.game.memory = 5;
        r.register_effect("DNA-FIELD", std::sync::Arc::new(OptionalDnaCancel));
        assert!(declare(&mut r.game, field, partner, result, 3).is_none());
        if remove_result {
            let index = r.game.players[0].hand.iter().position(|card| card.handle() == result).unwrap();
            let removed = r.game.players[0].hand.remove(index);
            r.game.players[0].trash.push(removed);
        } else { r.game.memory = -9; }
        let before_deck = r.game.players[0].deck.len();
        r.game.resolve_selection(0, PASS).unwrap();
        let owner_memory = if r.game.turn_player() == 0 { r.game.memory } else { -r.game.memory };
        assert_eq!(owner_memory, if remove_result { 5 } else { -9 });
        assert_eq!(r.game.players[0].deck.len(), before_deck);
        assert!(r.game.players[0].hand.iter().any(|card| card.handle() == partner));
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn dna_replacement_cannot_use_a_same_id_field_replacement() {
    for accept in [false, true] {
        let (mut r, field, partner, result) = fixture(0, false, 3);
        r.game.memory = 5;
        r.register_effect("DNA-FIELD", std::sync::Arc::new(OptionalDnaCancel));
        assert!(declare(&mut r.game, field, partner, result, 3).is_none());
        let removed = r.game.players[0].battle_area.remove(field.index as usize);
        for card in removed.card_sources { r.game.players[0].trash.push(card); }
        let replacement = r.place_on_field(0, "DNA-FIELD", None);
        let replacement_top = r.game.players[0].battle_area[replacement.index as usize].top_card().handle();
        let before_deck = r.game.players[0].deck.len();
        let action = if accept { digimon_engine::action::space::REPLACEMENT_ACCEPT } else { PASS };
        r.game.resolve_selection(0, action).unwrap();
        assert_eq!(r.game.memory, 5);
        assert_eq!(r.game.players[0].deck.len(), before_deck);
        assert_eq!(r.game.players[0].battle_area[replacement.index as usize].top_card().handle(), replacement_top);
        assert!(r.game.players[0].hand.iter().any(|card| card.handle() == result));
        assert!(r.game.players[0].hand.iter().any(|card| card.handle() == partner));
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn equal_cost_material_orientations_remain_explicit_and_clone_safe() {
    let (mut r, field, partner, result) = fixture(0, true, 0);
    r.game.card_data.iter_mut().find(|card| card.card_id == "DNA-PARTNER").unwrap().level = Some(6);
    r.game.card_data.iter_mut().find(|card| card.card_id == "DNA-RESULT").unwrap().dna_costs =
        make_test_dna_card("DNA-RESULT", "DNA result", 6, 6, 0).dna_costs;
    assert!(declare(&mut r.game, field, partner, result, 0).is_none());
    let actions = r.game.pending_selection.as_ref().unwrap().valid_action_ids.clone();
    assert_eq!(actions.len(), 2, "equal costs do not remove either legal source order");
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    for (index, action) in actions.into_iter().enumerate() {
        let mut game = r.game.clone();
        game.resolve_selection(0, action).unwrap();
        let sources = &game.players[0].battle_area[field.index as usize].card_sources;
        assert_eq!(sources.last().unwrap().handle(), result);
        assert_eq!(sources.iter().position(|card| card.handle() == partner), Some(if index == 0 { 0 } else { 2 }));
        assert!(game.pending_selection.is_none());
    }
}

#[test]
fn printed_cost_uses_the_chosen_dna_requirement() {
    let (mut r, field, partner, result) = fixture(0, true, 3);
    r.game.memory = 10;
    let meta = r.game.card_data.iter_mut().find(|card| card.card_id == "DNA-RESULT").unwrap();
    let mut second = meta.dna_costs[0].clone();
    second.memory_cost = 5;
    meta.dna_costs.push(second);
    let source = r.game.players[0].battle_area[field.index as usize].top_card().handle();
    assert!(EffectContext::new(&mut r.game, source, Some(field), 0)
        .effect_initiated_dna_digivolve_with_hand_partner_cost(field, partner, result,
            digimon_engine::enums::CostDelta::Reduce(0), false).is_none());
    let actions = &r.game.pending_selection.as_ref().unwrap().valid_action_ids;
    assert_eq!(actions.len(), 2);
    let second = actions[1];
    r.game.resolve_selection(0, second).unwrap();
    assert_eq!(r.game.memory, 5);
    assert_eq!(r.game.players[0].battle_area[field.index as usize].top_card().handle(), result);
}

struct DnaOnlyFieldReducer;
impl digimon_engine::effect::CardEffect for DnaOnlyFieldReducer {
    fn effects(&self, card: CardHandle) -> Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::Effect::before_pay_cost(card)
            .name("DNA context must survive the optional gate")
            .condition(|ctx| ctx.cost_is_digivolve && ctx.dna_origin() && ctx.source_is_cost_target_permanent())
            .optional().cost_reduction(3)
            .pay_cost_fn(|ctx| ctx.suspend_self_as_cost()).build()]
    }
}

#[test]
fn optional_field_reducer_keeps_dna_context_when_accepted_after_pause() {
    let (mut r, field, partner, result) = fixture(0, false, 3);
    r.game.memory = -9;
    r.register_effect("DNA-FIELD", std::sync::Arc::new(DnaOnlyFieldReducer));
    assert!(declare(&mut r.game, field, partner, result, 3).is_none());
    let accept = *r.game.pending_selection.as_ref().expect("payable DNA reducer is offered")
        .valid_action_ids.iter().find(|&&action| action != PASS).unwrap();
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        game.resolve_selection(0, accept).unwrap();
        assert_eq!(game.players[0].battle_area[field.index as usize].top_card().handle(), result,
            "the same DNA-conditioned payment accepted in the probe must execute live");
        let memory = if game.turn_player() == 0 { game.memory } else { -game.memory };
        assert_eq!(memory, -9);
        assert!(game.pending_selection.is_none());
    }
    assert_eq!(r.game.verification_digest(), clone.verification_digest());
}

#[test]
fn dna_field_reducer_payment_keeps_origin_after_its_dsl_selection() {
    const REDUCER: &str = r#"
card: DNA-PARKED-REDUCER
name: DNA payment with a suspended fee
kind: tamer
color: [white]
cost: 1
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: DNA-RESULT}
    condition: {dna_origin: true}
    amount: 1
    pay_cost:
      - select_own_permanent:
          filter: {card_number_is: DNA-PARKED-REDUCER, is_suspended: false}
          bind_as: fee
          prompt: Choose the DNA payment
      - suspend: {target: fee}
      - if:
          condition: {dna_origin: true}
          then:
            - gain_memory: 2
"#;
    let mut field_card = make_test_card("DNA-FIELD", "Field material");
    field_card.level = Some(6);
    let mut partner_card = make_test_card("DNA-PARTNER", "Hand material");
    partner_card.level = Some(5);
    let mut r = DebugRunner::builder().from_dsl_yaml(REDUCER).unwrap()
        .add_card(field_card).add_card(partner_card)
        .add_card(make_test_dna_card("DNA-RESULT", "DNA result", 6, 5, 3))
        .add_card(make_test_card("DNA-DRAW", "Draw"))
        .hand(0, &["DNA-RESULT", "DNA-PARTNER"])
        .deck(0, &["DNA-DRAW"; 8]).memory(5).start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    let field = r.place_on_field(0, "DNA-FIELD", None);
    let fee = r.place_on_field(0, "DNA-PARKED-REDUCER", None);
    let result = r.game.players[0].hand[0].handle();
    let partner = r.game.players[0].hand[1].handle();
    r.game.tick_declarative_effects();
    assert!(declare(&mut r.game, field, partner, result, 3).is_none());
    let accept = *r.game.pending_selection.as_ref().unwrap().valid_action_ids.iter()
        .find(|&&action| action != PASS).unwrap();
    assert_eq!(build_action_mask(&r.game, 0)[accept as usize], 1.0);
    r.game.resolve_selection(0, accept).unwrap();
    let action = digimon_engine::action::space::ATTACK_START + fee.index as u16;
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        game.resolve_selection(0, action).unwrap();
        assert!(game.pending_selection.is_none());
        assert_eq!(game.players[0].battle_area[field.index as usize].top_card().handle(), result);
        assert!(game.players[0].battle_area[fee.index as usize].is_suspended);
        assert_eq!(game.memory, 5, "the DNA-only payment tail gains two before paying the reduced cost two");
        assert_eq!(game.players[0].hand.len(), 1);
        assert_eq!(game.players[0].deck.len(), 7);
    }
    assert_eq!(r.game.verification_digest(), clone.verification_digest());
}
