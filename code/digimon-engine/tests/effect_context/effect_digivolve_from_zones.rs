use std::sync::Arc;

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, CostDelta};
use digimon_engine::permanent::PermanentHandle;

struct SecurityLossAndDigivolveGain;

impl CardEffect for SecurityLossAndDigivolveGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::on_lose_security(card)
                .name("gain 2 on lose security")
                .process(|ctx| ctx.gain_memory(2))
                .build(),
            Effect::when_digivolving(card)
                .name("gain 3 when digivolving")
                .process(|ctx| ctx.gain_memory(3))
                .build(),
        ]
    }
}

fn evo_lv4(card_id: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: vec![EvoCost {
            card_color: 0,
            level: 3,
            memory_cost: 0,
        }],
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn add_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card registered");
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    let handle = card.handle();
    runner.game.players[player as usize].trash.push(card);
    handle
}

fn push_source(runner: &mut DebugRunner, target: PermanentHandle, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card registered");
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, target.player, card_index);
    let handle = card.handle();
    runner.game.players[target.player as usize].battle_area[target.index as usize]
        .card_sources
        .push(card);
    handle
}

#[test]
fn effect_digivolve_from_trash_moves_exact_card_to_target_top() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(evo_lv4("EVO4"))
        .add_card(make_test_card("OTHER", "Other"))
        .memory(5)
        .start();

    let target = runner.place_on_field(0, "BASE3", None);
    let evo_handle = add_to_trash(&mut runner, 0, "EVO4");
    let other_handle = add_to_trash(&mut runner, 0, "OTHER");
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Trash(0, 0),
            target,
            CostDelta::Free,
            false,
        )
    };

    assert!(ok);
    let target_perm = &runner.game.players[0].battle_area[target.index as usize];
    assert_eq!(target_perm.top_card().handle(), evo_handle);
    assert_eq!(target_perm.card_sources.len(), 2);
    assert_eq!(runner.game.players[0].trash.len(), 1);
    assert_eq!(runner.game.players[0].trash[0].handle(), other_handle);
}

#[test]
fn effect_digivolve_from_security_moves_selected_card_and_preserves_neighbors() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(evo_lv4("EVO4"))
        .add_card(make_test_card("SEC-C", "Security C"))
        .security(0, &["SEC-A", "EVO4", "SEC-C"])
        .memory(5)
        .start();

    let target = runner.place_on_field(0, "BASE3", None);
    let low = runner.game.players[0].security[0].handle();
    let evo_handle = runner.game.players[0].security[1].handle();
    let high = runner.game.players[0].security[2].handle();
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Security(0, 1),
            target,
            CostDelta::Free,
            false,
        )
    };

    assert!(ok);
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .handle(),
        evo_handle
    );
    let security_handles: Vec<_> = runner.game.players[0]
        .security
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(security_handles, vec![low, high]);
}

#[test]
fn effect_digivolve_from_security_fires_security_loss_before_digivolve_triggers() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(evo_lv4("EVO4"))
        .security(0, &["EVO4"])
        .memory(5)
        .start();
    runner.register_effect("EVO4", Arc::new(SecurityLossAndDigivolveGain));

    let target = runner.place_on_field(0, "BASE3", None);
    let evo_handle = runner.game.players[0].security[0].handle();
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Security(0, 0),
            target,
            CostDelta::Free,
            false,
        )
    };

    assert!(ok);
    assert_eq!(runner.game.players[0].security.len(), 0);
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .handle(),
        evo_handle
    );
    assert_eq!(
        runner.game.memory, 10,
        "OnLoseSecurity (+2) resolves before the final WhenDigivolving (+3)"
    );
}

#[test]
fn effect_digivolve_from_material_moves_exact_source_out_of_stack() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TARGET3", "Target"))
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(evo_lv4("EVO4"))
        .add_card(make_test_card("CARRIER-TOP", "Carrier Top"))
        .memory(5)
        .start();

    let target = runner.place_on_field(0, "TARGET3", None);
    let carrier = runner.place_on_field(0, "CARRIER", None);
    let evo_handle = push_source(&mut runner, carrier, "EVO4");
    let top_handle = push_source(&mut runner, carrier, "CARRIER-TOP");
    let carrier_bottom =
        runner.game.players[0].battle_area[carrier.index as usize].card_sources[0].handle();
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Material(carrier, 1),
            target,
            CostDelta::Free,
            false,
        )
    };

    assert!(ok);
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .handle(),
        evo_handle
    );
    let carrier_handles: Vec<_> = runner.game.players[0].battle_area[carrier.index as usize]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(carrier_handles, vec![carrier_bottom, top_handle]);
}

#[test]
fn failed_effect_digivolve_restores_source_zone() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(evo_lv4("EVO4"))
        .security(0, &["EVO4"])
        .memory(5)
        .start();

    let bogus_target = PermanentHandle {
        player: 0,
        index: 9,
    };
    let evo_handle = runner.game.players[0].security[0].handle();
    let source_card = evo_handle;

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Security(0, 0),
            bogus_target,
            CostDelta::Free,
            false,
        )
    };

    assert!(!ok);
    assert_eq!(runner.game.players[0].security.len(), 1);
    assert_eq!(runner.game.players[0].security[0].handle(), evo_handle);
}
