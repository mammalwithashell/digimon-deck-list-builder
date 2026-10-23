//! Scalar payment observers see the same declared DNA context in their
//! condition and body, including during side-effect-free affordability probes.
use std::sync::Arc;

use digimon_engine::{
    action::{mask::build_action_mask, space::{HAND_EFFECT_START, PASS}},
    card_source::CardHandle,
    debug_runner::{make_test_card, make_test_dna_card, DebugRunner},
    effect::{CardEffect, Effect},
    effect_context::{EffectContext, EffectReadContext},
    enums::{CostDelta, GamePhase},
    game::Game,
    permanent::PermanentHandle,
};

#[derive(Clone, Copy)]
enum ObserverMode { Condition, Body, InvalidatePartner }

struct DnaObserver {
    mode: ObserverMode,
    result: CardHandle,
    partner: CardHandle,
    host: PermanentHandle,
}

fn declared_target(ctx: &EffectReadContext<'_>, source: CardHandle,
    result: CardHandle, host: PermanentHandle) -> bool {
    ctx.dna_origin()
        && ctx.source_card == source
        && ctx.source_permanent == Some(host)
        && ctx.cost_target_card == Some(result)
        && ctx.cost_target_from_hand
        && ctx.cost_target_permanents == [host]
        && ctx.source_is_cost_target_permanent()
}

impl CardEffect for DnaObserver {
    fn effects(&self, source: CardHandle) -> Vec<Effect> {
        let (mode, result, partner, host) = (self.mode, self.result, self.partner, self.host);
        vec![Effect::before_pay_cost_observe(source)
            .condition(move |ctx| declared_target(ctx, source, result, host)
                && (matches!(mode, ObserverMode::Body) || ctx.cost_is_digivolve))
            .process(move |ctx| {
                if matches!(mode, ObserverMode::Body)
                    && !(ctx.cost_is_digivolve && declared_target(&ctx.as_read(), source, result, host))
                { return; }
                if matches!(mode, ObserverMode::InvalidatePartner) {
                    let hand = &mut ctx.game.players[host.player as usize].hand;
                    if let Some(index) = hand.iter().position(|card| card.handle() == partner) {
                        let card = hand.remove(index);
                        ctx.game.players[host.player as usize].trash.push(card);
                    }
                } else {
                    ctx.gain_memory(3);
                }
            }).build()]
    }
}

fn fixture(mode: ObserverMode, memory: i16) -> (DebugRunner, PermanentHandle, CardHandle, CardHandle) {
    let mut host = make_test_card("OBSERVER-FIELD", "Observer field material");
    host.level = Some(6);
    let mut partner = make_test_card("OBSERVER-PARTNER", "Hand material");
    partner.level = Some(5);
    let mut result = make_test_dna_card("OBSERVER-RESULT", "DNA result", 6, 5, 3);
    result.level = Some(7);
    let mut second = result.dna_costs[0].clone();
    second.memory_cost = 4;
    result.dna_costs.push(second);
    let mut r = DebugRunner::builder().add_card(host).add_card(partner).add_card(result)
        .add_card(make_test_card("OBSERVER-DRAW", "Draw"))
        .hand(0, &["OBSERVER-RESULT", "OBSERVER-PARTNER"])
        .deck(0, &["OBSERVER-DRAW"; 8]).memory(5).start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r.game.memory = memory;
    let host = r.place_on_field(0, "OBSERVER-FIELD", None);
    let result = r.game.players[0].hand[0].handle();
    let partner = r.game.players[0].hand[1].handle();
    r.register_effect("OBSERVER-FIELD", Arc::new(DnaObserver { mode, result, partner, host }));
    (r, host, partner, result)
}

fn declare(game: &mut Game, host: PermanentHandle, partner: CardHandle,
    result: CardHandle) -> Option<PermanentHandle> {
    let source = game.players[0].battle_area[host.index as usize].top_card().handle();
    EffectContext::new(game, source, Some(host), 0)
        .effect_initiated_dna_digivolve_with_hand_partner_cost(
            host, partner, result, CostDelta::Reduce(0), false)
}

fn assert_observer_funds_routes_without_mutating_live_probe(mode: ObserverMode) {
    let (mut r, host, partner, result) = fixture(mode, -9);
    let original_top = r.game.players[0].battle_area[host.index as usize].top_card().handle();
    let original_hand = r.game.players[0].hand.iter().map(|c| c.handle()).collect::<Vec<_>>();
    let original_deck = r.game.players[0].deck.len();
    assert!(declare(&mut r.game, host, partner, result).is_none());
    let pending = r.game.pending_selection.as_ref().expect("observer gain makes both printed DNA costs affordable");
    assert_eq!(pending.valid_action_ids, [HAND_EFFECT_START, HAND_EFFECT_START + 1]);
    assert_eq!(r.game.memory, -9, "probing both recipes cannot gain live memory");
    assert_eq!(r.game.players[0].deck.len(), original_deck);
    assert_eq!(r.game.players[0].hand.iter().map(|c| c.handle()).collect::<Vec<_>>(), original_hand);
    assert_eq!(r.game.players[0].battle_area[host.index as usize].top_card().handle(), original_top);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[PASS as usize], 0.0);
    assert_eq!(mask[HAND_EFFECT_START as usize], 1.0);
    assert_eq!(mask[(HAND_EFFECT_START + 1) as usize], 1.0);
    let mut cloned = r.game.clone();
    for (game, action, expected_memory) in [
        (&mut r.game, HAND_EFFECT_START, -9),
        (&mut cloned, HAND_EFFECT_START + 1, -10),
    ] {
        game.resolve_selection(0, action).unwrap();
        let owner_memory = if game.turn_player() == 0 { game.memory } else { -game.memory };
        assert_eq!(owner_memory, expected_memory, "observer gains exactly once for the chosen live recipe");
        assert_eq!(game.players[0].battle_area[host.index as usize].top_card().handle(), result);
        assert!(game.players[0].battle_area[host.index as usize].card_sources.iter().any(|c| c.handle() == partner));
        assert_eq!(game.players[0].deck.len(), original_deck - 1);
        assert_eq!(game.players[0].hand.len(), 1);
        assert!(game.pending_selection.is_none());
    }
}

#[test]
fn dna_observer_condition_receives_digivolve_origin_and_declared_target_context() {
    assert_observer_funds_routes_without_mutating_live_probe(ObserverMode::Condition);
}

#[test]
fn dna_observer_body_receives_the_same_cost_target_context_as_its_condition() {
    assert_observer_funds_routes_without_mutating_live_probe(ObserverMode::Body);
}

#[test]
fn dna_observer_invalidating_the_hand_partner_rejects_every_recipe_without_live_mutation() {
    let (mut r, host, partner, result) = fixture(ObserverMode::InvalidatePartner, 5);
    let before = r.game.verification_digest();
    assert!(declare(&mut r.game, host, partner, result).is_none());
    assert!(r.game.pending_selection.is_none(), "a scalar observer invalidates the declared physical partner in every probe");
    assert_eq!(r.game.verification_digest(), before, "rejected affordability probes cannot trash live cards or charge memory");
}
