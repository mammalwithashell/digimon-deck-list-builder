//! Registry replacement must not reuse either old positive or negative memo entries.
//! This is cache identity evidence, not purity or projection permission.
use crate::{
    cards::CardEffectRegistry,
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::raw_rust::EngineRawRustRegistry,
    effect::{CardEffect, Effect},
    effect_context::EffectContext,
    game::Game,
    trigger_context::TriggerContext,
};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

#[derive(Clone, Copy, Debug)]
enum Route { Full, Registry }
fn lookup(game: &Game, route: Route, card: CardHandle) -> Option<Arc<Vec<Effect>>> {
    match route {
        Route::Full => game.effects_for_card("GEN", card),
        Route::Registry => game.registry_effects("GEN", card),
    }
}
struct Native { value: i16, calls: Arc<AtomicUsize> }
impl CardEffect for Native {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        effects(card, self.value)
    }
}
fn effects(card: CardHandle, value: i16) -> Vec<Effect> {
    vec![Effect::on_play(card).name(&format!("generation-{value}"))
        .process(move |ctx| ctx.game.memory = value).build()]
}
fn install(registry: &mut CardEffectRegistry, compiled: bool, value: i16, calls: Arc<AtomicUsize>) {
    if compiled {
        let spec: digimon_dsl::CardSpec = serde_yml::from_str(
            "card: GEN\nname: Generation\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 1000\neffects:\n  - kind: raw_rust\n    fn: generation\n").unwrap();
        let compiled = Arc::new(digimon_dsl::compile::compile(&spec).unwrap());
        let mut raw = EngineRawRustRegistry::new();
        raw.register_declarative("generation", move |card| {
            calls.fetch_add(1, Ordering::SeqCst);
            effects(card, value)
        });
        registry.insert_compiled("GEN", compiled, Arc::new(raw));
    } else {
        registry.insert("GEN", Arc::new(Native { value, calls }));
    }
}
fn fixture(seat: u8) -> (Game, CardHandle) {
    let mut runner = DebugRunner::builder().add_card(make_test_card("GEN", "Generation"))
        .hand(seat, &["GEN"]).start();
    runner.set_first_player(seat);
    let mut game = runner.game;
    let card = game.players[seat as usize].hand[0].handle();
    game.effect_registry = CardEffectRegistry::new();
    (game, card)
}
fn execute(game: &mut Game, list: &Arc<Vec<Effect>>, card: CardHandle, seat: u8, value: i16) {
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, format!("generation-{value}"));
    game.memory = 0;
    list[0].process.as_ref().unwrap()(&mut EffectContext::new(game, card, None, seat));
    assert_eq!(game.memory, value);
}
#[test]
fn replacement_refreshes_both_memos_without_rebinding_retained_outputs_or_clones() {
    for route in [Route::Full, Route::Registry] {
        for seat in 0..2 {
            for cloned in [false, true] {
                for replacement in 0..3 {
                    let (mut game, card) = fixture(seat);
                    let calls = Arc::new(AtomicUsize::new(0));
                    install(&mut game.effect_registry, true, 7, calls.clone());
                    let retained_factory = game.effect_registry.get("GEN").unwrap();
                    let retained = lookup(&game, route, card).unwrap();
                    let hit = lookup(&game, route, card).unwrap();
                    assert!(Arc::ptr_eq(&retained, &hit));
                    let mut prior = game.clone();
                    if cloned { game = game.clone(); }
                    if replacement == 2 { game.effect_registry = CardEffectRegistry::new(); }
                    install(&mut game.effect_registry, replacement != 0, 9, calls.clone());
                    assert!(!Arc::ptr_eq(&retained_factory, &game.effect_registry.get("GEN").unwrap()));
                    assert_eq!(game.effect_registry.compiled_origin("GEN").is_some(), replacement != 0);
                    let fresh = lookup(&game, route, card).unwrap();
                    execute(&mut game, &fresh, card, seat, 9);
                    assert!(!Arc::ptr_eq(&retained, &fresh), "{route:?} replacement={replacement}");
                    assert!(Arc::ptr_eq(&fresh, &lookup(&game, route, card).unwrap()));
                    execute(&mut game, &retained, card, seat, 7);
                    execute(&mut game, &Arc::new(retained_factory.effects(card)), card, seat, 7);
                    let unchanged = lookup(&prior, route, card).unwrap();
                    assert!(Arc::ptr_eq(&retained, &unchanged));
                    execute(&mut prior, &unchanged, card, seat, 7);
                    let owned = Arc::new(game.effects_for_card_owned("GEN", card).unwrap());
                    execute(&mut game, &owned, card, seat, 9);
                }
            }
        }
    }
}
#[test]
fn negative_memos_refresh_on_insert_and_independent_registry_assignment() {
    for route in [Route::Full, Route::Registry] {
        for seat in 0..2 {
            for cloned in [false, true] {
                for compiled in [false, true] {
                    let (mut game, card) = fixture(seat);
                    assert!(lookup(&game, route, card).is_none());
                    let prior = game.clone();
                    if cloned { game = game.clone(); }
                    let calls = Arc::new(AtomicUsize::new(0));
                    install(&mut game.effect_registry, compiled, 9, calls.clone());
                    let current = lookup(&game, route, card).expect("cached absence must expire");
                    execute(&mut game, &current, card, seat, 9);
                    assert!(lookup(&prior, route, card).is_none());
                    game.effect_registry = CardEffectRegistry::new();
                    assert!(lookup(&game, route, card).is_none(), "independent empty registry has no prior factory");
                    install(&mut game.effect_registry, compiled, 11, calls);
                    let next = lookup(&game, route, card).unwrap();
                    execute(&mut game, &next, card, seat, 11);
                    assert!(!Arc::ptr_eq(&current, &next));
                }
            }
        }
    }
}
fn rejected(run: impl FnOnce()) {
    let err = catch_unwind(AssertUnwindSafe(run)).expect_err("world obligation must still reject");
    let message = err.downcast_ref::<String>().map(String::as_str)
        .or_else(|| err.downcast_ref::<&str>().copied()).unwrap_or("");
    assert!(message.contains("closed-list projected"), "{message}");
}
#[test]
fn registry_replacement_cannot_exempt_old_or_new_callbacks_or_cache_queries() {
    for route in [Route::Full, Route::Registry] {
        for seat in 0..2 {
            for cloned in [false, true] {
                for caller in 0..3 {
                    let (mut game, card) = fixture(seat);
                    let calls = Arc::new(AtomicUsize::new(0));
                    install(&mut game.effect_registry, false, 7, calls.clone());
                    let old = lookup(&game, route, card).unwrap();
                    install(&mut game.effect_registry, true, 9, calls.clone());
                    // Owned route is deliberately uncached, independent of this batch's memo fix.
                    let new = Arc::new(game.effects_for_card_owned("GEN", card).unwrap());
                    let mut context = TriggerContext::default();
                    if caller == 2 {
                        let mut child = TriggerContext::default();
                        child.simultaneous_play_members.push(TriggerContext::default());
                        context.simultaneous_play_members.push(child);
                    }
                    game.current_trigger_context = (caller != 0).then_some(context);
                    if cloned { game = game.clone(); }
                    game.require_projected_world_read_guard_for_test();
                    let before = calls.load(Ordering::SeqCst);
                    let context_before = format!("{:?}", game.current_trigger_context);
                    rejected(|| { lookup(&game, route, card); });
                    assert_eq!(calls.load(Ordering::SeqCst), before);
                    for list in [&old, &new] {
                        game.memory = 3;
                        rejected(|| list[0].process.as_ref().unwrap()(
                            &mut EffectContext::new(&mut game, card, None, seat)));
                        assert_eq!(game.memory, 3);
                    }
                    assert_eq!(format!("{:?}", game.current_trigger_context), context_before);
                }
            }
        }
    }
}
