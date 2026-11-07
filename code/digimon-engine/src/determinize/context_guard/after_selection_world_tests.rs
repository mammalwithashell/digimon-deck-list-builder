use crate::{
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    effect::{CardEffect, Effect},
    enums::{CostDelta, EffectSourceKind, GamePhase, PlaySource},
    game::{Game, PendingWouldPlayOrigin},
    game_actions::{CostTargetContext, PlayCostAdjustments},
    resume::AfterSelectionHook,
    selection::{PendingSelection, SelectionKind},
    trigger_context::TriggerContext,
};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

struct CountFactory(Arc<AtomicUsize>);
impl CardEffect for CountFactory {
    fn effects(&self, _: CardHandle) -> Vec<Effect> {
        self.0.fetch_add(1, Ordering::SeqCst);
        vec![]
    }
}

fn fixture(seat: u8, route: u8) -> (Game, AfterSelectionHook, CardHandle, Arc<AtomicUsize>) {
    let mut target_data = make_test_card("HOOK-TARGET", "Hook target");
    target_data.play_cost = 4;
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("HOOK-SOURCE", "Hook source"))
        .add_card(target_data)
        .deck(0, &["HOOK-SOURCE";20]).deck(1, &["HOOK-SOURCE";20])
        .hand(seat, &[]).memory(10).start();
    runner.skip_mulligan();
    runner.set_first_player(seat);
    runner.set_phase(GamePhase::Main);
    runner.inject_trash(seat, "HOOK-SOURCE");
    let source = runner.game.player(seat).trash[0].handle();
    let target;
    let hook;
    if route == 3 {
        runner.game.stage_inject_card(seat, "HOOK-TARGET", "hand").unwrap();
        target = runner.game.player(seat).hand[0].handle();
        let mut total = PlayCostAdjustments::default();
        total += 2;
        hook = AfterSelectionHook::PlayCostContinuation {
            effect_controller: None, player_id: seat, hand_index: 0,
            target: CostTargetContext { card: target, from_hand: true,
                is_digivolve: false, target_permanents: [None, None] },
            cost_delta: CostDelta::Reduce(0), source: PlaySource::ByHand,
            origin: PendingWouldPlayOrigin::Hand, suppress_on_play: false,
            accumulated_reduction: total, processed: vec![],
        };
    } else {
        runner.inject_trash(seat, "HOOK-TARGET");
        target = if route == 2 { CardHandle(u16::MAX) }
            else { runner.game.player(seat).trash.last().unwrap().handle() };
        hook = AfterSelectionHook::PartitionSecondPlay { controller: seat, source_card: source, card: target };
        if route == 0 {
            runner.game.pending_selection = Some(PendingSelection {
                card_choice: None, kind: SelectionKind::EffectChoice, selecting_player: seat,
                previous_phase: GamePhase::Main, valid_action_ids: vec![0], is_optional: true,
                prompt: "First play still parked".into(), effect_choices: None,
                source_card: source, source_permanent: None, source_kind: EffectSourceKind::Digimon,
                zone_owner: None, callback: Box::new(|_,_| panic!("hook must not resolve parked choice")),
                on_decline: None,
            });
            runner.game.current_phase = GamePhase::EffectChoice;
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    runner.game.effect_registry.insert("HOOK-TARGET", Arc::new(CountFactory(calls.clone())));
    (runner.game, hook, target, calls)
}

fn caller(game: &mut Game, mode: u8) {
    game.current_trigger_context = match mode {
        0 => None,
        1 => Some(TriggerContext::default()),
        _ => { let mut root = TriggerContext::default();
            let mut child = TriggerContext::default();
            child.simultaneous_play_members.push(TriggerContext::default());
            root.simultaneous_play_members.push(child); Some(root) }
    };
}

fn snapshot(game: &Game, seat: u8) -> (i16, GamePhase, bool, usize, Vec<CardHandle>, Vec<CardHandle>, usize) {
    (game.memory, game.current_phase, game.pending_selection.is_some(),
        game.after_selection_resume_hooks.0.len(),
        game.player(seat).hand.iter().map(|card|card.handle()).collect(),
        game.player(seat).trash.iter().map(|card|card.handle()).collect(),
        game.player(seat).battle_area.len())
}

fn matrix(routes: &[u8]) -> usize {
    let mut cases = 0;
    for &route in routes { for seat in [0,1] { for mode in 0..3 {
        for cloned in [false,true] { for marked in [false,true] {
            let (original, retained_hook, target, calls) = fixture(seat, route);
            // The hook and optional Game clone predate the later world obligation.
            let mut game = if cloned { original.clone() } else { original };
            let hook = retained_hook.clone();
            caller(&mut game, mode);
            if marked { game.require_projected_world_read_guard_for_test(); }
            let before = snapshot(&game, seat);
            let context = format!("{:?}",game.current_trigger_context);
            let result = catch_unwind(AssertUnwindSafe(|| game.run_after_selection_hook(hook)));
            if marked {
                let error = result.expect_err("marked hook must reject at dispatch entry");
                let message = error.downcast_ref::<String>().map(String::as_str)
                    .or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("");
                assert_eq!(message, "closed-list projected world reached unproven continuation: after_selection_hook");
                assert_eq!(snapshot(&game,seat), before);
                assert_eq!(calls.load(Ordering::SeqCst),0);
            } else {
                result.unwrap();
                if route == 0 {
                    assert_eq!(game.after_selection_resume_hooks.0.len(),1);
                    assert_eq!(calls.load(Ordering::SeqCst),0);
                    assert!(game.pending_selection.is_some());
                    assert_eq!(game.player(seat).battle_area.len(),0);
                    assert!(game.player(seat).trash.iter().any(|card|card.handle()==target));
                    // A legal optional PASS has no raw decline callback, but still drains hooks.
                    game.resolve_selection(seat, crate::action::space::PASS).unwrap();
                    assert!(game.after_selection_resume_hooks.0.is_empty());
                }
                if route == 2 {
                    assert_eq!(snapshot(&game,seat),before);
                    assert_eq!(calls.load(Ordering::SeqCst),0);
                } else {
                    assert_eq!(game.player(seat).battle_area.len(),1);
                    assert_eq!(game.player(seat).battle_area[0].top_card().handle(),target);
                    assert!(!game.player(seat).battle_area[0].is_suspended);
                    assert_eq!(game.memory,if route == 3 {8} else {10});
                    assert!(calls.load(Ordering::SeqCst)>0);
                    assert!(!game.player(seat).trash.iter().any(|card|card.handle()==target));
                    if route == 3 { assert!(!game.player(seat).hand.iter().any(|card|card.handle()==target)); }
                }
            }
            assert_eq!(format!("{:?}",game.current_trigger_context),context);
            cases += 1;
        } }
    } } }
    cases
}

#[test]
fn partition_hook_checks_world_before_rearming() { assert_eq!(matrix(&[0]),24); }

#[test]
fn partition_hook_checks_world_before_play_or_stale_target_return() { assert_eq!(matrix(&[1,2]),48); }

#[test]
fn payment_hook_checks_world_before_cost_or_card_movement() { assert_eq!(matrix(&[3]),24); }

#[test]
fn raw_optional_pass_without_decline_callback_still_guards_its_hook() {
    let mut cases = 0;
    for seat in [0,1] { for mode in 0..3 { for cloned in [false,true] { for marked in [false,true] {
        let (mut original, hook, target, calls) = fixture(seat, 0);
        original.run_after_selection_hook(hook);
        assert_eq!(original.after_selection_resume_hooks.0.len(),1);
        assert_eq!(calls.load(Ordering::SeqCst),0);
        let mut game = if cloned { original.clone() } else { original };
        caller(&mut game, mode);
        let before = snapshot(&game,seat);
        let context = format!("{:?}",game.current_trigger_context);
        let depths = (game.draining_deferred,game.selection_resolution_depth);
        if marked { game.require_projected_world_read_guard_for_test(); }
        let result = catch_unwind(AssertUnwindSafe(|| game.resolve_selection(seat,crate::action::space::PASS)));
        if marked {
            let error = result.expect_err("raw PASS must reject before hook body");
            let message = error.downcast_ref::<String>().map(String::as_str)
                .or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("");
            assert_eq!(message,"closed-list projected world reached unproven continuation: after_selection_hook");
            assert_eq!(game.memory,before.0);
            assert_eq!(game.player(seat).hand.iter().map(|c|c.handle()).collect::<Vec<_>>(),before.4);
            assert_eq!(game.player(seat).trash.iter().map(|c|c.handle()).collect::<Vec<_>>(),before.5);
            assert_eq!(game.player(seat).battle_area.len(),before.6);
            assert_eq!(calls.load(Ordering::SeqCst),0);
            // The dispatcher already consumed the prompt and entered scopes. No rollback claim.
            assert_eq!(game.draining_deferred,depths.0+1);
            assert_eq!(game.selection_resolution_depth,depths.1+1);
        } else {
            assert_eq!(result.unwrap(),Ok(()));
            assert_eq!(game.player(seat).battle_area.len(),1);
            assert_eq!(game.player(seat).battle_area[0].top_card().handle(),target);
            assert_eq!(game.memory,10);
            assert!(calls.load(Ordering::SeqCst)>0);
            assert_eq!((game.draining_deferred,game.selection_resolution_depth),depths);
        }
        assert!(game.pending_selection.is_none());
        assert!(game.after_selection_resume_hooks.0.is_empty());
        assert_eq!(game.current_phase,GamePhase::Main);
        assert_eq!(format!("{:?}",game.current_trigger_context),context);
        cases += 1;
    } } } }
    assert_eq!(cases,24);
}
