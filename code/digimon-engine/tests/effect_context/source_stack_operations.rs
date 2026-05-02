use std::sync::Arc;

use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardSourceRef;
use digimon_engine::permanent::PermanentHandle;

struct SourceTrashGainOne;

impl CardEffect for SourceTrashGainOne {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_digivolution_card_trashed(card)
            .name("gain 1 on source trash")
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

struct LoseSecurityGainTwo;

impl CardEffect for LoseSecurityGainTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_lose_security(card)
            .name("gain 2 on lose security")
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

struct SourceTrashContextGainOne {
    expected_source: CardHandle,
    expected_host: CardHandle,
}

impl CardEffect for SourceTrashContextGainOne {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let expected_source = self.expected_source;
        let expected_host = self.expected_host;
        vec![Effect::on_digivolution_card_trashed(card)
            .name("gain 1 on matching source trash context")
            .condition(move |ctx| {
                ctx.event_source_card() == Some(expected_source)
                    && ctx.event_host_card() == Some(expected_host)
            })
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

fn push_source(runner: &mut DebugRunner, target: PermanentHandle, card_id: &str) -> CardHandle {
    push_source_owned(runner, target, target.player, card_id)
}

fn push_source_owned(
    runner: &mut DebugRunner,
    target: PermanentHandle,
    owner: u8,
    card_id: &str,
) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card registered");
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, owner, card_index);
    let handle = card.handle();
    runner.game.players[target.player as usize].battle_area[target.index as usize]
        .card_sources
        .push(card);
    handle
}

#[test]
fn place_as_bottom_source_from_security_fires_loss_observer_and_moves_exact_card() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TARGET", "Target"))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("PICKED", "Picked"))
        .add_card(make_test_card("SEC-C", "Security C"))
        .security(0, &["SEC-A", "PICKED", "SEC-C"])
        .memory(0)
        .start();
    runner.register_effect("PICKED", Arc::new(LoseSecurityGainTwo));

    let target = runner.place_on_field(0, "TARGET", None);
    let target_top = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();
    let low = runner.game.players[0].security[0].handle();
    let picked = runner.game.players[0].security[1].handle();
    let high = runner.game.players[0].security[2].handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, target_top, Some(target), 0);
        ctx.place_as_bottom_source(CardSourceRef::Security(0, 1), target)
    };

    assert!(ok);
    assert_eq!(runner.game.memory, 2);
    let security_handles: Vec<_> = runner.game.players[0]
        .security
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(security_handles, vec![low, high]);

    let target_handles: Vec<_> = runner.game.players[0].battle_area[target.index as usize]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(target_handles, vec![picked, target_top]);
}

#[test]
fn place_as_bottom_source_from_material_moves_exact_source() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SOURCE-BASE", "Source Base"))
        .add_card(make_test_card("PICKED", "Picked"))
        .add_card(make_test_card("SOURCE-TOP", "Source Top"))
        .add_card(make_test_card("TARGET", "Target"))
        .start();

    let source_perm = runner.place_on_field(0, "SOURCE-BASE", None);
    let target = runner.place_on_field(0, "TARGET", None);
    let picked = push_source(&mut runner, source_perm, "PICKED");
    let source_top = push_source(&mut runner, source_perm, "SOURCE-TOP");
    let source_base =
        runner.game.players[0].battle_area[source_perm.index as usize].card_sources[0].handle();
    let target_top = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, target_top, Some(target), 0);
        ctx.place_as_bottom_source(CardSourceRef::Material(source_perm, 1), target)
    };

    assert!(ok);
    let source_handles: Vec<_> = runner.game.players[0].battle_area[source_perm.index as usize]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(source_handles, vec![source_base, source_top]);

    let target_handles: Vec<_> = runner.game.players[0].battle_area[target.index as usize]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(target_handles, vec![picked, target_top]);
}

#[test]
fn trash_card_source_fires_on_digivolution_card_trashed_once() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("OBSERVER", "Observer"))
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_test_card("MID", "Mid"))
        .add_card(make_test_card("TOP", "Top"))
        .memory(0)
        .start();

    runner.register_effect("OBSERVER", Arc::new(SourceTrashGainOne));
    let observer = runner.place_on_field(0, "OBSERVER", None);
    let target = runner.place_on_field(0, "BASE", None);
    let mid = push_source(&mut runner, target, "MID");
    let top = push_source(&mut runner, target, "TOP");
    let base = runner.game.players[0].battle_area[target.index as usize].card_sources[0].handle();

    let source_card = runner.game.players[0].battle_area[observer.index as usize]
        .top_card()
        .handle();
    let before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(observer), 0);
        ctx.trash_card_source(target, mid);
    }

    assert_eq!(
        runner.game.memory,
        before + 1,
        "OnDigivolutionCardTrashed should dispatch once"
    );
    let stack_handles: Vec<_> = runner.game.players[0].battle_area[target.index as usize]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(stack_handles, vec![base, top]);
    assert_eq!(runner.game.players[0].trash.len(), 1);
    assert_eq!(runner.game.players[0].trash[0].handle(), mid);
}

#[test]
fn trash_card_source_carries_trashed_source_and_host_context() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("OBSERVER", "Observer"))
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_test_card("MID", "Mid"))
        .add_card(make_test_card("TOP", "Top"))
        .memory(0)
        .start();

    let observer = runner.place_on_field(0, "OBSERVER", None);
    let target = runner.place_on_field(0, "BASE", None);
    let mid = push_source(&mut runner, target, "MID");
    let top = push_source(&mut runner, target, "TOP");
    runner.register_effect(
        "OBSERVER",
        Arc::new(SourceTrashContextGainOne {
            expected_source: mid,
            expected_host: top,
        }),
    );

    let source_card = runner.game.players[0].battle_area[observer.index as usize]
        .top_card()
        .handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(observer), 0);
        ctx.trash_card_source(target, mid);
    }

    assert_eq!(
        runner.game.memory, 1,
        "source-trash observer should receive exact source and host context"
    );
}

#[test]
fn trash_card_source_routes_borrowed_source_to_owner_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SOURCE", "Source"))
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_test_card("BORROWED", "Borrowed"))
        .start();

    let source = runner.place_on_field(0, "SOURCE", None);
    let target = runner.place_on_field(0, "BASE", None);
    let borrowed = push_source_owned(&mut runner, target, 1, "BORROWED");

    let source_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
        ctx.trash_card_source(target, borrowed);
    }

    assert_eq!(runner.game.players[0].trash.len(), 0);
    assert_eq!(runner.game.players[1].trash.len(), 1);
    assert_eq!(runner.game.players[1].trash[0].handle(), borrowed);
}
