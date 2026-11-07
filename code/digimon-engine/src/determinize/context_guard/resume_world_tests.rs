use crate::{
    action::space::{HAND_EFFECT_START, PASS},
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{bindings::Bindings, step::{selections::run_resume, StepRuntime}},
    enums::{EffectSourceKind, GamePhase},
    game::Game,
    resume::{ResumeDecline, ResumeFrame, ResumeProvenance, ResumeSelectKind, ResumeStack},
    selection::{PendingSelection, SelectionError, SelectionKind},
    trigger_context::TriggerContext,
};
use digimon_dsl::compiled::CompiledStep;
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::Arc};

fn parked(seat: u8, branch: u8) -> Game {
    let mut game = DebugRunner::builder().add_card(make_test_card("RW-1", "Resume"))
        .deck(0, &["RW-1";20]).deck(1, &["RW-1";20]).memory(0).start().game;
    game.turn_player_idx = usize::from(seat);
    game.pending_selection = Some(PendingSelection {
        card_choice: None, kind: SelectionKind::EffectChoice, selecting_player: seat,
        previous_phase: GamePhase::Main, valid_action_ids: vec![HAND_EFFECT_START],
        is_optional: true, prompt: "Resume world boundary".into(), effect_choices: None,
        source_card: CardHandle(0), source_permanent: None,
        source_kind: EffectSourceKind::Digimon, zone_owner: None,
        callback: Box::new(|_,_| panic!("raw accept must not execute")),
        on_decline: Some(Box::new(|_| panic!("raw decline must not execute"))),
    });
    game.current_phase = GamePhase::EffectChoice;
    game.pending_selection_resume = Some(ResumeStack { frames: vec![ResumeFrame::RunTail {
        prov: ResumeProvenance { source_card: CardHandle(0), source_permanent: None,
            source_kind: EffectSourceKind::Digimon, controller: seat, override_pin: None },
        select_kind: ResumeSelectKind::EffectChoice { post: None }, bind_as: None,
        inner_tail: Arc::new(vec![CompiledStep::GainMemory(1)]), outer_conts: vec![],
        bindings: Bindings::new(), runtime: StepRuntime::default(),
        trigger_context: Some(TriggerContext::default()),
        decline: if branch == 2 { ResumeDecline::None } else { ResumeDecline::RunTail {
            tail: Arc::new(vec![CompiledStep::GainMemory(2)]), aborts_clause: branch == 3,
        } },
    }] });
    game
}

fn caller(game: &mut Game, mode: u8) {
    game.current_trigger_context = match mode {
        0 => None,
        1 => Some(TriggerContext::default()),
        _ => { let mut ctx = TriggerContext::default();
            ctx.simultaneous_play_members.push(TriggerContext::default()); Some(ctx) }
    };
}

fn guard_panic(error: Box<dyn std::any::Any + Send>, site: &str) {
    let message = error.downcast_ref::<String>().map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied()).unwrap_or("");
    assert_eq!(message, format!("closed-list projected world reached unproven continuation: {site}"));
}

fn expected_memory(_seat: u8, branch: u8) -> i16 {
    // parked() makes the effect controller the active player for both seats.
    // The engine stores memory from that active player's perspective.
    match branch { 0 => 1, 1 => 2, _ => 0 }
}

#[test]
fn parked_resume_world_rejects_before_prompt_phase_and_scope_changes() {
    let mut cases = 0;
    for seat in [0,1] { for branch in 0..4 { for mode in 0..3 {
        for cloned in [false,true] { for probe in [false,true] { for marked in [false,true] {
            let original = parked(seat, branch);
            let mut game = if cloned { original.clone() } else { original };
            caller(&mut game, mode);
            if marked { game.require_projected_world_read_guard_for_test(); }
            let before = (game.draining_deferred, game.selection_resolution_depth,
                game.dsl_clause_aborted, game.effect_source_player, game.effect_source_card);
            let context = format!("{:?}", game.current_trigger_context);
            let resume = format!("{:?}", game.pending_selection_resume);
            let action = if branch == 0 { HAND_EFFECT_START } else { PASS };
            let result = catch_unwind(AssertUnwindSafe(|| if probe {
                game.resolve_payment_probe_selection(seat, action)
            } else { game.resolve_selection(seat, action) }));
            if marked {
                guard_panic(result.expect_err("marked structured selection must reject"), "selection_resume");
                assert_eq!(game.memory, 0);
                assert_eq!(game.current_phase, GamePhase::EffectChoice);
                assert!(game.pending_selection.is_some());
                assert_eq!(format!("{:?}", game.pending_selection_resume), resume);
                assert_eq!((game.draining_deferred, game.selection_resolution_depth,
                    game.dsl_clause_aborted, game.effect_source_player, game.effect_source_card), before);
            } else {
                assert_eq!(result.unwrap(), Ok(()));
                assert_eq!(game.memory, expected_memory(seat, branch));
                assert!(game.pending_selection.is_none());
                assert!(game.pending_selection_resume.is_none());
                assert_eq!(game.draining_deferred, before.0);
                assert_eq!(game.selection_resolution_depth, before.1);
                assert_eq!(game.dsl_clause_aborted, before.2);
            }
            assert_eq!(format!("{:?}", game.current_trigger_context), context);
            cases += 1;
        } } }
    } } }
    assert_eq!(cases, 192);
}

#[test]
fn direct_resume_executor_rejects_world_before_tail_or_decline_mutation() {
    let mut cases = 0;
    for seat in [0,1] { for branch in 0..4 { for mode in 0..3 { for marked in [false,true] {
        let mut game = parked(seat, branch);
        // Retain a cloned stack before imposing the later world obligation.
        let stack = game.pending_selection_resume.take().unwrap().clone();
        game.pending_selection = None;
        game.current_phase = GamePhase::Main;
        caller(&mut game, mode);
        let context = format!("{:?}", game.current_trigger_context);
        if marked { game.require_projected_world_read_guard_for_test(); }
        let result = catch_unwind(AssertUnwindSafe(|| run_resume(&mut game, stack,
            if branch == 0 { HAND_EFFECT_START } else { PASS }, branch != 0)));
        if marked {
            guard_panic(result.expect_err("direct marked resume must reject"), "run_resume");
            assert_eq!(game.memory, 0);
            assert!(!game.dsl_clause_aborted);
        } else {
            result.unwrap();
            assert_eq!(game.memory, expected_memory(seat, branch));
            assert_eq!(game.dsl_clause_aborted, branch == 3);
        }
        assert_eq!(format!("{:?}", game.current_trigger_context), context);
        assert_eq!(game.current_phase, GamePhase::Main);
        cases += 1;
    } } } }
    assert_eq!(cases, 48);
}

#[test]
fn world_marked_structured_invalid_actions_and_page_navigation_keep_their_contract() {
    use crate::{card_choice_page::CARD_CHOICE_NEXT, owned_card_choice::OwnedCardChoice};
    for probe in [false,true] { for seat in [0,1] {
        let mut game = parked(seat, 0);
        game.require_projected_world_read_guard_for_test();
        for (player,action,error) in [(1-seat,HAND_EFFECT_START,SelectionError::WrongPlayer),
            (seat,HAND_EFFECT_START+1,SelectionError::InvalidAction)] {
            let result = if probe { game.resolve_payment_probe_selection(player,action) }
                else { game.resolve_selection(player,action) };
            assert_eq!(result, Err(error)); assert!(game.pending_selection_resume.is_some());
        }
        let mut nav = DebugRunner::builder().add_card(make_test_card("RW-N", "Navigation"))
            .security(seat, &["RW-N";40]).memory(0).start().game;
        let owner = OwnedCardChoice::security(&mut nav,seat,(0..40).collect(),false).unwrap();
        let token = owner.token();
        let valid = owner.valid_action_ids(&nav);
        assert!(valid.contains(&CARD_CHOICE_NEXT));
        let mut prompt = game.pending_selection.take().unwrap();
        prompt.card_choice = Some(owner); prompt.valid_action_ids = valid;
        prompt.kind = SelectionKind::Security; prompt.is_optional = false;
        prompt.zone_owner = Some(seat);
        nav.pending_selection = Some(prompt);
        nav.pending_selection_resume = game.pending_selection_resume.take();
        nav.current_phase = GamePhase::SelectSecurity;
        nav.require_projected_world_read_guard_for_test();
        let stack = format!("{:?}",nav.pending_selection_resume);
        let result = if probe { nav.resolve_payment_probe_selection(seat,CARD_CHOICE_NEXT) }
            else { nav.resolve_selection(seat,CARD_CHOICE_NEXT) };
        assert_eq!(result, Ok(()));
        assert_ne!(nav.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().token(),token);
        assert_eq!(format!("{:?}",nav.pending_selection_resume),stack);
        assert_eq!(nav.current_phase,GamePhase::SelectSecurity);
        assert_eq!(nav.memory,0); assert_eq!(nav.draining_deferred,0);
        assert_eq!(nav.selection_resolution_depth,0);
    } }
}
