use crate::{
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{bindings::Bindings, step::run_steps},
    effect_context::EffectContext,
    enums::GamePhase,
    trigger_context::TriggerContext,
};
use digimon_dsl::compiled::{CompiledBindingRef, CompiledCountBound, CompiledPlayerRef,
    CompiledPredicate, CompiledStep, CompiledZone};
use std::{collections::HashSet, panic::{catch_unwind, AssertUnwindSafe}};

fn fixture(seat: u8, zone: u8, fenced: bool) -> (crate::Game, CardHandle, Bindings) {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("COUNT-SRC", "Source"))
        .add_card(make_test_card("COUNT-A", "Pick A"))
        .add_card(make_test_card("COUNT-B", "Pick B"))
        .add_card(make_test_card("COUNT-C", "Pick C"))
        .hand(seat, &["COUNT-SRC", "COUNT-A", "COUNT-B", "COUNT-C"])
        .memory(3).start();
    r.skip_mulligan(); r.set_first_player(seat); r.set_phase(GamePhase::Main);
    r.game.memory = 3;
    let source = r.game.players[seat as usize].hand[0].handle();
    let mut bindings = Bindings::new();
    let excluded = if zone == 2 {
        let host = r.place_stack(seat, &["COUNT-A", "COUNT-B", "COUNT-C", "COUNT-SRC"]);
        bindings.insert_permanent("host", host);
        r.game.players[seat as usize].battle_area[host.index as usize].card_sources[0].handle()
    } else {
        let excluded = r.game.players[seat as usize].hand[1].handle();
        if zone == 1 {
            let cards: Vec<_> = r.game.players[seat as usize].hand.drain(1..).collect();
            r.game.players[seat as usize].put_all_in_trash(cards);
        }
        excluded
    };
    bindings.insert_card_list("excluded", vec![excluded]);
    if fenced {
        let mut context = TriggerContext::default();
        context.event_card = Some(CardHandle(65000)).into();
        assert!(context.event_card.project_if_unentitled(&HashSet::new()));
        r.game.current_trigger_context = Some(context);
        r.game.require_event_card_read_fence_for_test();
    }
    (r.game, source, bindings)
}

fn selection(zone: u8, min: u8, filter: CompiledPredicate) -> CompiledStep {
    if zone == 2 {
        CompiledStep::SelectMaterials {
            named_groups: vec![], require_max: false,
            of_permanent: CompiledBindingRef::Named("host".into()),
            max: CompiledCountBound::Literal(2), min, filter,
            uniqueness: None, same_by: None, bind_as: Some("picks".into()),
            prompt: "Pick materials".into(), prompt_key: None, optional_zero: min == 0,
        }
    } else {
        CompiledStep::SelectCountCappedMulti {
            of: CompiledPlayerRef::You,
            zone: if zone == 0 { CompiledZone::Hand } else { CompiledZone::Trash },
            max: CompiledCountBound::Literal(2), min, clamp_to_available: false,
            filter, bind_as: Some("picks".into()), prompt: "Pick cards".into(),
            prompt_key: None, optional_zero: min == 0, distinct_by: None,
        }
    }
}

#[test]
fn compiled_count_capped_bindings_survive_hand_trash_material_picks_and_vm_clones() {
    for seat in [0, 1] { for zone in 0..3 { for fenced in [false, true] {
        for cloned in [false, true] { for finish in 0..3 {
            let (mut game, source, mut bindings) = fixture(seat, zone, fenced);
            let steps = [selection(zone, if finish == 0 { 0 } else { 1 }, CompiledPredicate {
                name_contains: Some("Pick".into()), not_in_binding: Some("excluded".into()),
                ..Default::default()
            }), CompiledStep::PerSelected { selection: "picks".into(), bind_as: "picked".into(),
                body: vec![CompiledStep::GainMemory(1)] }, CompiledStep::GainMemory(2)];
            run_steps(&steps, &mut EffectContext::new(&mut game, source, None, seat), &mut bindings);
            let state = match &game.pending_selection_resume.as_ref().unwrap().frames[0] {
                crate::resume::ResumeFrame::MultiPickStep(s) => s,
                other => panic!("expected typed multi-pick: {other:?}"),
            };
            assert_eq!(state.candidate_indices.len(), 2, "saved binding excludes Pick A");
            let mut picks = 0;
            while game.pending_selection.is_some() {
                let p = game.pending_selection.as_ref().unwrap();
                let action = if picks == finish { crate::action::PASS }
                    else { *p.valid_action_ids.iter().find(|&&a| a != crate::action::PASS).unwrap() };
                if cloned { game = game.clone(); }
                assert!(crate::build_action_mask(&game, seat)[action as usize] > 0.5);
                game.resolve_selection(seat, action).unwrap();
                if action != crate::action::PASS { picks += 1; }
                assert!(picks <= 2);
            }
            assert_eq!(picks, finish);
            assert_eq!(game.memory, 5 + finish as i16);
            assert!(game.pending_selection_resume.is_none());
            assert_eq!(game.projected_event_card_read_fence(), fenced);
        }}
    }}}
}

#[test]
fn compiled_count_capped_event_reads_still_reject_before_installing_a_prompt() {
    for seat in [0, 1] { for zone in 0..3 {
        let (mut game, source, mut bindings) = fixture(seat, zone, true);
        let phase = game.current_phase;
        let result = catch_unwind(AssertUnwindSafe(|| run_steps(&[selection(zone, 1,
            CompiledPredicate { event_card_name_contains: Some("Pick".into()), ..Default::default() })],
            &mut EffectContext::new(&mut game, source, None, seat), &mut bindings)));
        let error = result.expect_err("unavailable event identity cannot be read");
        let message = error.downcast_ref::<String>().map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied()).unwrap_or("");
        assert_eq!(message, "closed-list unavailable event-card reference read");
        assert!(game.pending_selection.is_none());
        assert!(game.pending_selection_resume.is_none());
        assert_eq!(game.memory, 3);
        assert_eq!(game.current_phase, phase);
    }}
}
