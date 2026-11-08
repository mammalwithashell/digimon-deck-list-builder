//! A fixed compiled Option-hand selector must keep predicate/read obligations.
use crate::{
    action::space::PLAY_HAND_START,
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{bindings::Bindings, step::run_steps},
    effect_context::EffectContext,
    enums::{Expiry, GamePhase, ModifierType},
    modifiers::PlayerModifierEntry,
    permanent::PermanentHandle,
    trigger_context::TriggerContext,
    Game,
};
use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep};
use std::{collections::HashSet, panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn fixture(seat: u8) -> (Game, CardHandle, PermanentHandle, CardHandle) {
    let mut builder = DebugRunner::builder()
        .add_card(make_test_card("OPTION-SOURCE", "Source"))
        .add_card(make_test_card("OPTION-NONOPTION", "Allowed nonoption"));
    for (id, cost, name) in [("OPTION-ZERO", 0, "Allowed zero"),
        ("OPTION-LOW", 2, "Allowed low"), ("OPTION-HIGH", 5, "Allowed high"),
        ("OPTION-OTHER", 0, "Rejected other")] {
        builder = builder.from_dsl_yaml(&format!(
            "card: {id}\nname: {name}\nkind: option\ncolor: [red]\ncost: {cost}\neffects:\n  - when: main_from_hand\n    process:\n      - draw: {{ of: you, count: 1 }}\n"
        )).unwrap();
    }
    let mut r = builder.deck(0, &["OPTION-SOURCE"; 20]).deck(1, &["OPTION-SOURCE"; 20])
        .hand(seat, &["OPTION-ZERO", "OPTION-LOW", "OPTION-HIGH", "OPTION-OTHER",
            "OPTION-NONOPTION", "OPTION-ZERO"]).memory(3).start();
    // Bind the same compiled fixture through the production constructor.
    // DebugRunner's generic insert deliberately marks its factories opaque.
    for id in ["OPTION-ZERO", "OPTION-LOW", "OPTION-HIGH", "OPTION-OTHER"] {
        let compiled = Arc::new(r.compiled_card(id).unwrap().clone());
        r.game.effect_registry.insert_compiled(id, compiled,
            Arc::new(crate::dsl_cards::raw_rust::EngineRawRustRegistry::new()));
    }
    r.skip_mulligan(); r.set_first_player(seat); r.set_phase(GamePhase::Main);
    let host = r.place_on_field(seat, "OPTION-SOURCE", None);
    let source = r.game.players[seat as usize].battle_area[0].top_card().handle();
    let excluded = r.game.players[seat as usize].hand[5].handle();
    (r.game, source, host, excluded)
}

fn fence(game: &mut Game) {
    let mut context = TriggerContext::default();
    context.event_card = Some(CardHandle(65000)).into();
    assert!(context.event_card.project_if_unentitled(&HashSet::new()));
    game.current_trigger_context = Some(context);
    game.require_event_card_read_fence_for_test();
}

fn selection(ceiling: bool) -> CompiledStep {
    CompiledStep::UseOptionFromHand {
        of: CompiledPlayerRef::You,
        filter: CompiledPredicate { name_contains: Some("Allowed".into()),
            not_in_binding: Some("excluded".into()), ..Default::default() },
        use_cost_lte_opponent_memory: ceiling, optional: true,
        prompt: Some("Retained Option prompt".into()),
    }
}

fn install(game: &mut Game, seat: u8, source: CardHandle, host: PermanentHandle,
    excluded: CardHandle, override_seat: bool, step: CompiledStep) {
    let mut bindings = Bindings::new(); bindings.insert_card_list("excluded", vec![excluded]);
    run_steps(&[step, CompiledStep::GainMemory(2)],
        &mut EffectContext::new_with_override(game, source, Some(host), seat,
            override_seat.then_some(1 - seat)), &mut bindings);
}

#[test]
fn compiled_option_hand_candidates_and_cloned_accept_decline_preserve_tail() {
    for seat in 0..2 { for fenced in [false, true] { for cloned in [false, true] {
      for override_seat in [false, true] { for decline in [false, true] {
        let (mut game, source, host, excluded) = fixture(seat);
        if fenced { fence(&mut game); }
        install(&mut game, seat, source, host, excluded, override_seat, selection(false));
        let p = game.pending_selection.as_ref().unwrap();
        assert_eq!(p.valid_action_ids, vec![PLAY_HAND_START, PLAY_HAND_START + 1, PLAY_HAND_START + 2]);
        assert_eq!(p.prompt, "Retained Option prompt");
        assert_eq!(p.zone_owner, Some(seat));
        let actor = if override_seat { 1 - seat } else { seat };
        assert_eq!(p.selecting_player, actor);
        assert!(matches!(game.pending_selection_resume.as_ref().unwrap().frames.last(),
            Some(crate::resume::ResumeFrame::UseOptionFromHandStep(_))));
        let mut game = if cloned { game.clone() } else { game };
        let before_deck = game.players[seat as usize].deck.len();
        let action = if decline { crate::action::PASS } else { PLAY_HAND_START + 1 };
        assert!(crate::build_action_mask(&game, actor)[action as usize] > 0.5);
        game.resolve_selection(actor, action).unwrap();
        assert!(game.pending_selection.is_none());
        assert!(game.pending_selection_resume.is_none());
        assert_eq!(game.memory, 5, "free use or decline executes the same outer tail once");
        assert_eq!(game.players[seat as usize].deck.len(), before_deck - usize::from(!decline));
        assert_eq!(game.players[seat as usize].trash.iter()
            .filter(|c| c.card_id(&game.card_data) == "OPTION-LOW").count(), usize::from(!decline));
        assert_eq!(game.projected_event_card_read_fence(), fenced);
      } }
    } } }
}

#[test]
fn compiled_option_hand_memory_ceiling_and_blocked_candidates_stay_strict() {
    for seat in 0..2 { for fenced in [false, true] { for cloned in [false, true] {
      for memory in [-3, 3] { for blocked in [false, true] {
        let (mut game, source, host, excluded) = fixture(seat);
        game.memory = memory;
        if fenced { fence(&mut game); }
        if blocked { game.modifiers.add_player_modifier(seat, PlayerModifierEntry::simple(
            ModifierType::CannotUseOptionCards, 0, Expiry::Permanent, None, seat)); }
        let mut game = if cloned { game.clone() } else { game };
        install(&mut game, seat, source, host, excluded, false, selection(true));
        if blocked {
            assert!(game.pending_selection.is_none());
        } else {
            let expected = if memory < 0 { vec![PLAY_HAND_START, PLAY_HAND_START + 1] }
                else { vec![PLAY_HAND_START] };
            assert_eq!(game.pending_selection.as_ref().unwrap().valid_action_ids, expected);
        }
      } }
    } } }
}

#[test]
fn compiled_option_hand_unavailable_event_predicate_fails_at_actual_read() {
    for seat in 0..2 { for cloned in [false, true] {
        let (mut game, source, host, excluded) = fixture(seat); fence(&mut game);
        let mut game = if cloned { game.clone() } else { game };
        let mut step = selection(false);
        if let CompiledStep::UseOptionFromHand { filter, .. } = &mut step {
            filter.event_card_name_contains = Some("Unavailable".into());
        }
        let phase = game.current_phase;
        let result = catch_unwind(AssertUnwindSafe(||
            install(&mut game, seat, source, host, excluded, false, step)));
        let err = result.unwrap_err();
        let message = err.downcast_ref::<String>().map(String::as_str)
            .or_else(|| err.downcast_ref::<&str>().copied()).unwrap_or("");
        assert_eq!(message, "closed-list unavailable event-card reference read");
        assert!(game.pending_selection.is_none()); assert_eq!(game.current_phase, phase);
        assert_eq!(game.memory, 3);
    } }
}

#[test]
fn raw_option_hand_filter_still_rejects_before_opaque_body() {
    for seat in 0..2 { for cloned in [false, true] {
        let (mut game, source, host, _) = fixture(seat); fence(&mut game);
        let mut game = if cloned { game.clone() } else { game };
        let calls = Arc::new(AtomicUsize::new(0)); let body_calls = calls.clone();
        let result = catch_unwind(AssertUnwindSafe(|| {
            EffectContext::new(&mut game, source, Some(host), seat).select_hand(
                seat, "Opaque Option filter", true,
                move |_, _| { body_calls.fetch_add(1, Ordering::SeqCst); true }, |_, _| {});
        }));
        assert!(result.is_err()); assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(game.pending_selection.is_none());
    } }
}
