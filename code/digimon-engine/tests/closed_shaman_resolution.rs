//! Differential synthetic resolution evidence, not an admission certificate.
//! Both branches start from the SAME sampled world. Only the test clone's
//! captured anonymous event references are erased. No historical seed/model.
#[path = "determinization/support.rs"]
#[allow(dead_code)]
mod support;

use digimon_engine::{
    action::space::{PASS, PLAY_HAND_START},
    card_source::{CardHandle, CardSource},
    determinize::{beliefs::*, closed::materialize_closed},
    enums::{EffectTiming, GamePhase, Zone},
    permanent::{Permanent, PermanentHandle},
    selection::TriggerSource,
    trigger_context::{EventSubject, MovedCardSet, TriggerContext},
    Game,
};

fn take(g: &mut Game, owner: u8, id: &str) -> CardSource {
    let data = &g.card_data;
    let p = &mut g.players[owner as usize];
    for zone in [&mut p.hand, &mut p.deck, &mut p.security, &mut p.digitama_deck] {
        if let Some(i) = zone.iter().position(|c| c.card_id(data) == id) {
            return zone.remove(i);
        }
    }
    panic!("missing fixture card {id}");
}

struct CapturedEventReader;
impl digimon_engine::effect::CardEffect for CapturedEventReader {
    fn effects(&self, card: CardHandle) -> Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::Effect::before_pay_cost(card)
            .condition(|ctx| ctx.cost_is_digivolve)
            .cost_reduction_fn(|ctx| i32::from(ctx.event_card().is_some()))
            .build()]
    }
}

fn fixture(owner: u8, seed: u64, callback: bool) -> (Game, CardHandle, CardHandle, u8) {
    let deck: Vec<_> = support::D1.iter().map(|id| match *id {
        "ST1-01" => "BT24-003", "ST1-04" => "BT24-034",
        "ST1-07" => "BT26-029", other => other,
    }).collect();
    let mut g = support::new_game_with(730231, &deck, &deck);
    if callback {
        // Test-only opaque callback installed before any effect-cache access.
        // The queued Tsunomon clause itself remains the real printed clause.
        g.effect_registry.insert("BT24-034", std::sync::Arc::new(CapturedEventReader));
    }
    g.enable_belief_knowledge().unwrap();
    g.start_game();
    g.mulligan_pending.clear();
    let base = take(&mut g, owner, "BT24-034");
    let egg = take(&mut g, owner, "BT24-003");
    let source = egg.handle();
    let mut field = Permanent::new(base, 0);
    field.card_sources.insert(0, egg);
    g.players[owner as usize].battle_area.push(field);
    let evolution = take(&mut g, owner, "BT26-029");
    let target = evolution.handle();
    g.players[owner as usize].hand.push(evolution);
    g.turn_order = vec![owner, 1 - owner];
    g.turn_player_idx = 0;
    g.current_phase = GamePhase::Main;
    g.memory = 10;
    let event = g.players[owner as usize].security.last().unwrap().handle();
    assert!(g.add_to_hand_from_security(owner, event));
    g.effect_queue.clear();
    g.enqueue_triggered(EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::Permanent(PermanentHandle { player: owner, index: 0 }));
    g.effect_queue.retain(|e| e.card_id == "BT24-003" && e.source_card == source);
    assert_eq!(g.effect_queue.len(), 1);
    let mut context = TriggerContext::default();
    context.event_card = (Some(event)).into();
    context.target_card = (Some(source)).into();
    context.subject = (Some(EventSubject::Card { card: event, zone: Zone::Security })).into();
    context.affected_player = Some(owner);
    context.moved_card_sets.push(MovedCardSet { cards: vec![event], from: Some(Zone::Security), to: Some(Zone::Hand) });
    g.effect_queue[0].trigger_context = Some(context);
    let slot = g.effect_queue[0].effect_slot;
    let key = g.effects_for_card("BT24-003", source).unwrap()[slot as usize].shared_opt_group.unwrap_or(slot);
    let viewer = 1 - owner;
    let mut memory = g.belief_knowledge(viewer).unwrap().clone();
    // Synthetic public reveal pins the intended legal candidate in every sample.
    memory.observe(owner, target.0, "BT26-029", KnowledgeZone::Hand, None).unwrap();
    let cards = support::owned(&deck);
    let prior = WeightedDeckPrior::new(PriorSnapshot::new("standard", "shaman-resolution-synthetic",
        cards.iter().cloned().collect(), vec![CandidateDeck { cards, weight: 1.0 }], 0.1, 128).unwrap()).unwrap();
    let world = materialize_closed(&g, viewer, &memory, &prior, seed).unwrap();
    (world, source, target, key)
}

fn assert_observed_equal(a: &mut Game, b: &mut Game) {
    assert_eq!(a.to_scenario(), b.to_scenario());
    assert_eq!(serde_json::to_value(a.drain_events()).unwrap(), serde_json::to_value(b.drain_events()).unwrap());
    assert_eq!(format!("{:?}", a.effect_queue), format!("{:?}", b.effect_queue));
}

fn run(owner: u8, seed: u64, decline: bool, prohibit: bool, outer: bool) {
    let (mut original, source, target, key) = fixture(owner, seed, false);
    if outer { original.memory = 3; }
    if prohibit {
        original.modifiers.add_player_modifier(owner,
            digimon_engine::modifiers::PlayerModifierEntry::simple(
                digimon_engine::enums::ModifierType::CannotDigivolveDigimonByEffect, 0,
                digimon_engine::enums::Expiry::EndOfTurn, None, owner));
    }
    original.drain_events();
    let saved = original.effect_queue[0].clone();
    let mut projected = original.clone();
    let c = projected.effect_queue[0].trigger_context.as_mut().unwrap();
    c.event_card = (None).into();
    c.subject = (None).into();
    c.moved_card_sets[0].cards.clear();
    // Public source / ownership / movement metadata is deliberately preserved.
    assert_eq!(c.target_card.read(), Some(source));
    assert_eq!(c.affected_player, Some(owner));
    original.drain_effect_queue();
    projected.drain_effect_queue();
    if outer {
        // Each enclosing continuation owns a DIFFERENT captured context.
        // Restoring the inner Shaman context for either would change memory.
        for game in [&mut original, &mut projected] {
            let frame = game.pending_selection_resume.as_mut().unwrap().frames.iter_mut()
                .find(|f| matches!(f, digimon_engine::resume::ResumeFrame::RunTail { .. })).unwrap();
            let digimon_engine::resume::ResumeFrame::RunTail { outer_conts, .. } = frame else { unreachable!() };
            for event in [source, target] {
                let mut context = TriggerContext::default();
                context.event_card = (Some(event)).into();
                outer_conts.push(digimon_engine::resume::OuterContinuation {
                    tail: std::sync::Arc::new(vec![digimon_dsl::compiled::CompiledStep::GainMemoryFn {
                        formula: digimon_dsl::compiled::CompiledFormula::EventTargetLevel,
                    }]),
                    bindings: digimon_engine::dsl_cards::bindings::Bindings::new(),
                    runtime: digimon_engine::dsl_cards::step::StepRuntime::default(),
                    trigger_context: Some(context), source_card: source,
                    source_permanent: Some(PermanentHandle { player: owner, index: 0 }), player: owner,
                });
            }
        }
    }
    let mut first = true;
    let mut decisions = 0;
    while original.pending_selection.is_some() {
        assert!(decisions < 32, "synthetic resolution did not finish");
        let player = support::decision_player(&original);
        let a = digimon_engine::build_action_mask(&original, player);
        let b = digimon_engine::build_action_mask(&projected, player);
        assert_eq!(a, b);
        let action = if first && decline {
            PASS
        } else if first {
            let i = original.players[owner as usize].hand.iter().position(|c| c.handle() == target).unwrap();
            PLAY_HAND_START + i as u16
        } else if a[PASS as usize] > 0.5 {
            PASS // Decline subsequent optional effects, not the initial evolution.
        } else {
            a.iter().position(|v| *v > 0.5).unwrap() as u16
        };
        assert!(a[action as usize] > 0.5, "fixture must expose requested action {action}");
        original.decode_action(action, player);
        projected.decode_action(action, player);
        first = false;
        decisions += 1;
    }
    // Migrated 2026-11-12 for c6eda5678 (effect-digivolve candidates must be able to
    // digivolve): the `select_hand` of every requirement-honouring
    // `effect_initiated_digivolve` now carries `can_digivolve_onto`, matching DCGO's
    // `SelectSourceCard` filter `CanPlayCardTargetFrame` -> `CanEvolve` ->
    // `CanNotEvolve` (`CardSource.cs`), which excludes cards an `ICanNotDigivolveEffect`
    // forbids. Under `CannotDigivolveDigimonByEffect` BT26-029 is therefore no longer a
    // candidate and BT24-003 asks nothing (DCGO asks nothing when no card qualifies);
    // nothing is paid either way. Previously the pick was offered and the digivolve
    // then did nothing.
    assert_eq!(decisions > 0, !prohibit, "prohibited Shaman evolution offers no pick");
    assert!(projected.pending_selection.is_none());
    assert!(original.effect_queue.is_empty() && projected.effect_queue.is_empty());
    assert_observed_equal(&mut original, &mut projected);
    if outer {
        assert_eq!(original.memory, if decline { 10 } else { 8 },
            "3 initial minus 2 evolution payment on success, plus distinct level-2 and level-5 outer contexts");
    }
    let a = &original.players[owner as usize].battle_area[0];
    let b = &projected.players[owner as usize].battle_area[0];
    assert_eq!(a.activation_count(source, key), b.activation_count(source, key));
    if decline {
        assert_eq!(a.top_card().card_id(&original.card_data), "BT24-034");
        assert_eq!(a.activation_count(source, key), 0, "declining refunds OPT");
        // The real printed clause must be offered again after decline.
        original.effect_queue.push_back(saved);
        original.drain_effect_queue();
        assert!(original.pending_selection.is_some());
    } else if prohibit {
        assert_eq!(a.top_card().card_id(&original.card_data), "BT24-034");
        assert_eq!(original.memory, 10, "prohibited evolution pays nothing");
    } else {
        assert_eq!(a.top_card().handle(), target, "the intended Shaman evolved");
        assert_eq!(a.activation_count(source, key), 1, "successful activation consumes OPT");
        assert!(original.memory < 10, "printed evolution cost is paid");
        original.effect_queue.push_back(saved);
        original.drain_effect_queue();
        assert!(original.pending_selection.is_none(), "successful OPT cannot activate again");
    }
}

#[test]
fn printed_shaman_success_preserves_payment_mask_and_opt_on_both_seats() {
    support::with_big_stack(|| for owner in 0..2 { for seed in 0..4 { run(owner, seed, false, false, false); } });
}

#[test]
fn printed_shaman_decline_preserves_hand_and_refunds_opt_on_both_seats() {
    support::with_big_stack(|| for owner in 0..2 { for seed in 0..4 { run(owner, seed, true, false, false); } });
}

#[test]
fn printed_shaman_prohibition_preserves_no_payment_on_both_seats() {
    support::with_big_stack(|| for owner in 0..2 { for seed in 0..4 { run(owner, seed, false, true, false); } });
}

#[test]
fn printed_shaman_success_and_decline_restore_each_outer_context() {
    support::with_big_stack(|| for owner in 0..2 { for decline in [false, true] {
        run(owner, 23, decline, false, true);
    } });
}

#[test]
fn printed_shaman_resolution_preserves_captured_context_for_opaque_cost_callback() {
    support::with_big_stack(|| for owner in 0..2 {
        let (mut original, _, target, _) = fixture(owner, 19, true);
        let mut projected = original.clone();
        projected.effect_queue[0].trigger_context.as_mut().unwrap().event_card = (None).into();
        original.drain_effect_queue();
        projected.drain_effect_queue();
        let player = support::decision_player(&original);
        let mask = digimon_engine::build_action_mask(&original, player);
        assert_eq!(mask, digimon_engine::build_action_mask(&projected, player));
        let index = original.players[owner as usize].hand.iter().position(|c| c.handle() == target).unwrap();
        let action = PLAY_HAND_START + index as u16;
        assert!(mask[action as usize] > 0.5);
        original.decode_action(action, player);
        projected.decode_action(action, player);
        let mut decisions = 0;
        while original.pending_selection.is_some() {
            assert!(decisions < 32);
            let player = support::decision_player(&original);
            let mask = digimon_engine::build_action_mask(&original, player);
            assert_eq!(mask, digimon_engine::build_action_mask(&projected, player));
            let action = if mask[PASS as usize] > 0.5 { PASS }
                else { mask.iter().position(|v| *v > 0.5).unwrap() as u16 };
            original.decode_action(action, player);
            projected.decode_action(action, player);
            decisions += 1;
        }
        assert!(projected.pending_selection.is_none());
        assert_eq!(original.players[owner as usize].battle_area[0].top_card().handle(), target);
        assert_eq!(projected.players[owner as usize].battle_area[0].top_card().handle(), target);
        assert_eq!(original.memory, projected.memory + 1,
            "the real parked Shaman selection must restore its context into payment");
        assert!(original.memory < 10 && projected.memory < 10);
    });
}
