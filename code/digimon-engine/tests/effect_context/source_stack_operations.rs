use std::sync::{Arc, Mutex};

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

struct SourceTrashRecordSources {
    expected_host: CardHandle,
    seen: Arc<Mutex<Vec<CardHandle>>>,
}

impl CardEffect for SourceTrashRecordSources {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let expected_host = self.expected_host;
        let seen = self.seen.clone();
        vec![Effect::on_digivolution_card_trashed(card)
            .name("record source trash context")
            .condition(move |ctx| {
                ctx.event_host_card() == Some(expected_host) && ctx.event_source_card().is_some()
            })
            .process(move |ctx| {
                if let Some(source) = ctx.event_source_card() {
                    seen.lock().expect("record mutex poisoned").push(source);
                }
            })
            .build()]
    }
}

struct TrashQueuedSourceOnce {
    target: PermanentHandle,
    queued_source: CardHandle,
    fired: Arc<Mutex<bool>>,
}

impl CardEffect for TrashQueuedSourceOnce {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let target = self.target;
        let queued_source = self.queued_source;
        let fired = self.fired.clone();
        vec![Effect::on_digivolution_card_trashed(card)
            .name("trash queued source once")
            .process(move |ctx| {
                let should_fire = {
                    let mut fired = fired.lock().expect("fired mutex poisoned");
                    if *fired {
                        false
                    } else {
                        *fired = true;
                        true
                    }
                };
                if should_fire {
                    ctx.trash_card_source(target, queued_source);
                }
            })
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

#[test]
fn trash_all_sources_preserves_top_card_and_trashes_each_source() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_test_card("MID", "Mid"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .start();

    let target = runner.place_on_field(0, "BASE", None);
    let source_a = push_source(&mut runner, target, "MID");
    let top = push_source(&mut runner, target, "TOP");
    let source_card = runner.place_on_field(0, "EFFECT", None);
    let effect_card = runner.top_card(source_card);
    let base = runner.game.players[0].battle_area[target.index as usize].card_sources[0].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, effect_card, Some(source_card), 0);
        assert!(ctx.trash_all_sources(target));
    }

    let remaining: Vec<_> = runner.game.players[0].battle_area[target.index as usize]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(remaining, vec![top]);
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.handle() == base));
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.handle() == source_a));
}

#[test]
fn trash_all_sources_dispatches_each_removed_source_with_host_context() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_test_card("MID", "Mid"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("OBSERVER", "Observer"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .start();

    let target = runner.place_on_field(0, "BASE", None);
    let source_a =
        runner.game.players[0].battle_area[target.index as usize].card_sources[0].handle();
    let source_b = push_source(&mut runner, target, "MID");
    let top = push_source(&mut runner, target, "TOP");
    let seen = Arc::new(Mutex::new(Vec::new()));
    runner.register_effect(
        "OBSERVER",
        Arc::new(SourceTrashRecordSources {
            expected_host: top,
            seen: seen.clone(),
        }),
    );
    runner.place_on_field(0, "OBSERVER", None);
    let effect_perm = runner.place_on_field(0, "EFFECT", None);
    let effect_card = runner.top_card(effect_perm);

    {
        let mut ctx = EffectContext::new(&mut runner.game, effect_card, Some(effect_perm), 0);
        assert!(ctx.trash_all_sources(target));
    }

    assert_eq!(
        *seen.lock().expect("record mutex poisoned"),
        vec![source_a, source_b]
    );
}

#[test]
fn trash_all_sources_skips_sources_already_removed_by_observers() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_test_card("MID", "Mid"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("OBSERVER", "Observer"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .start();

    let target = runner.place_on_field(0, "BASE", None);
    let base = runner.game.players[0].battle_area[target.index as usize].card_sources[0].handle();
    let queued_source = push_source(&mut runner, target, "MID");
    let top = push_source(&mut runner, target, "TOP");
    let fired = Arc::new(Mutex::new(false));
    runner.register_effect(
        "OBSERVER",
        Arc::new(TrashQueuedSourceOnce {
            target,
            queued_source,
            fired: fired.clone(),
        }),
    );
    runner.place_on_field(0, "OBSERVER", None);
    let effect_perm = runner.place_on_field(0, "EFFECT", None);
    let effect_card = runner.top_card(effect_perm);

    {
        let mut ctx = EffectContext::new(&mut runner.game, effect_card, Some(effect_perm), 0);
        assert!(ctx.trash_all_sources(target));
    }

    let remaining: Vec<_> = runner.game.players[0].battle_area[target.index as usize]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(remaining, vec![top]);
    assert!(*fired.lock().expect("fired mutex poisoned"));
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.handle() == base));
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.handle() == queued_source));
}

#[test]
fn play_selected_sources_without_cost_removes_stable_sources_and_plays_them() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC-A", "Source A"))
        .add_card(make_test_card("TOP-A", "Top A"))
        .add_card(make_test_card("SRC-B", "Source B"))
        .add_card(make_test_card("TOP-B", "Top B"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .start();

    let stack_a = runner.place_stack(0, &["SRC-A", "TOP-A"]);
    let stack_b = runner.place_stack(0, &["SRC-B", "TOP-B"]);
    let source_a =
        runner.game.players[0].battle_area[stack_a.index as usize].card_sources[0].handle();
    let source_b =
        runner.game.players[0].battle_area[stack_b.index as usize].card_sources[0].handle();
    let effect_perm = runner.place_on_field(0, "EFFECT", None);
    let effect_card = runner.top_card(effect_perm);

    let selected = vec![
        digimon_engine::effect_context::SourceSelectionRef {
            permanent: stack_a,
            field_index: stack_a.index,
            source_index: 0,
            card: source_a,
        },
        digimon_engine::effect_context::SourceSelectionRef {
            permanent: stack_b,
            field_index: stack_b.index,
            source_index: 0,
            card: source_b,
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, effect_card, Some(effect_perm), 0);
        assert!(ctx.play_selected_sources_without_cost(selected));
    }

    let played_tops: Vec<_> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().handle())
        .collect();
    assert!(played_tops.contains(&source_a));
    assert!(played_tops.contains(&source_b));
    assert_eq!(
        runner.game.players[0].battle_area[stack_a.index as usize]
            .card_sources
            .len(),
        1
    );
    assert_eq!(
        runner.game.players[0].battle_area[stack_b.index as usize]
            .card_sources
            .len(),
        1
    );
}
