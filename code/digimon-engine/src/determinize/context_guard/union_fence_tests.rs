use crate::{
    card_source::{CardHandle, CardSource},
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{bindings::Bindings, lower_triggered::leading_cost_guard, step::run_steps},
    effect_context::{EffectContext, EffectReadContext},
    enums::GamePhase,
    permanent::PermanentHandle,
    selection::{UnionZoneOrigin, UnionZoneSet},
    trigger_context::TriggerContext,
    Game,
};
use digimon_dsl::compiled::{CompiledBindingRef, CompiledPlayerRef, CompiledPredicate,
    CompiledStep, CompiledUnionZoneFilters, CompiledZone};
use std::{collections::HashSet, panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn fixture(seat: u8) -> (Game, CardHandle, [CardHandle; 3], CardHandle, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SOURCE", "Source"))
        .add_card(make_test_card("HAND", "Hand eligible"))
        .add_card(make_test_card("TRASH", "Trash eligible"))
        .add_card(make_test_card("MATERIAL", "Material eligible"))
        .add_card(make_test_card("OTHER", "Other"))
        .add_card(make_test_card("HOST", "Allowed carrier"))
        .add_card(make_test_card("BLOCKED", "Blocked carrier"))
        .deck(0, &["SOURCE"; 20]).deck(1, &["SOURCE"; 20])
        .hand(seat, &["SOURCE", "HAND", "HAND", "OTHER"]).memory(3).start();
    r.skip_mulligan(); r.set_first_player(seat); r.set_phase(GamePhase::Main);
    let source = r.game.players[seat as usize].hand[0].handle();
    let hand = r.game.players[seat as usize].hand[1].handle();
    let excluded = r.game.players[seat as usize].hand[2].handle();
    let mut trash = None;
    for id in ["TRASH", "OTHER"] {
        let data = r.game.card_data.iter().position(|c| c.card_id == id).unwrap();
        let index = r.game.next_card_index();
        let card = CardSource::new(data, seat, index);
        if id == "TRASH" { trash = Some(card.handle()); }
        r.game.players[seat as usize].put_in_trash(card);
    }
    let host = r.place_stack(seat, &["MATERIAL", "OTHER", "HOST"]);
    r.place_stack(seat, &["MATERIAL", "BLOCKED"]);
    let material = r.game.players[seat as usize].battle_area[host.index as usize].card_sources[0].handle();
    (r.game, source, [hand, trash.unwrap(), material], excluded, host)
}

fn fence(game: &mut Game) {
    let mut context = TriggerContext::default();
    context.event_card = Some(CardHandle(65000)).into();
    assert!(context.event_card.project_if_unentitled(&HashSet::new()));
    game.current_trigger_context = Some(context);
    game.require_event_card_read_fence_for_test();
}

fn name(text: &str) -> CompiledPredicate {
    CompiledPredicate { name_contains: Some(text.into()),
        not_in_binding: Some("excluded".into()), ..Default::default() }
}

fn step(pinned: bool) -> CompiledStep {
    CompiledStep::SelectUnionZone {
        of: CompiledPlayerRef::You,
        zones: vec![CompiledZone::Hand, CompiledZone::Trash, CompiledZone::Material],
        material_of: pinned.then(|| CompiledBindingRef::Named("host".into())),
        filter: name("Material"),
        zone_filters: CompiledUnionZoneFilters { hand: Some(name("Hand")),
            trash: Some(name("Trash")), material: None },
        material_carrier_filter: Some(CompiledPredicate {
            name_contains: Some("Allowed".into()), ..Default::default() }),
        bind_as: Some("picked".into()), prompt: "Union choice".into(), prompt_key: None,
        optional: true, cost: false, then: vec![CompiledStep::GainMemory(1)],
    }
}

fn choices(game: &Game) -> Vec<(u16, CardHandle, UnionZoneOrigin)> {
    let owner = game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap();
    owner.valid_action_ids(game).into_iter().filter_map(|action| {
        match owner.clone().resolve(game, action).unwrap() {
            crate::card_choice_page::CardChoiceResult::Selected(selected) => {
                let (_, card, origin) = selected.union_ref().unwrap(); Some((action, card, origin))
            }
            _ => None,
        }
    }).collect()
}

#[test]
fn compiled_union_preserves_per_zone_carrier_binding_filters_and_cloned_tails() {
    for seat in [0, 1] { for fenced in [false, true] { for cloned in [false, true] {
      for pinned in [false, true] { for finish in 0..4 {
        let (mut game, source, expected, excluded, host) = fixture(seat);
        let mut bindings = Bindings::new();
        bindings.insert_card_list("excluded", vec![excluded]);
        bindings.insert_permanent("host", host);
        if fenced { fence(&mut game); }
        run_steps(&[step(pinned), CompiledStep::GainMemory(2)],
            &mut EffectContext::new(&mut game, source, Some(host), seat), &mut bindings);
        assert!(game.pending_selection_resume.is_some());
        assert_eq!(game.memory, 3);
        let mut game = if cloned { game.clone() } else { game };
        let picks = choices(&game);
        assert_eq!(picks.iter().map(|(_, card, _)| *card).collect::<Vec<_>>(), expected);
        assert_eq!(picks[0].2, UnionZoneOrigin::Hand);
        assert_eq!(picks[1].2, UnionZoneOrigin::Trash);
        assert_eq!(picks[2].2, UnionZoneOrigin::Material { carrier: host, source_index: 0 });
        let action = if finish == 3 { crate::action::PASS } else { picks[finish].0 };
        assert!(crate::build_action_mask(&game, seat)[action as usize] > 0.5);
        game.resolve_selection(seat, action).unwrap();
        assert!(game.pending_selection.is_none());
        assert!(game.pending_selection_resume.is_none());
        assert_eq!(game.memory, if finish == 3 { 5 } else { 6 });
        assert_eq!(game.projected_event_card_read_fence(), fenced);
        if fenced { assert!(game.current_trigger_context.as_ref().unwrap().event_card.is_unavailable()); }
      } }
    } } }
}

#[test]
fn compiled_union_actual_event_reads_reject_in_card_zone_and_carrier_predicates() {
    for seat in [0, 1] { for cloned in [false, true] { for reader in 0..3 {
        let (mut game, source, _, excluded, host) = fixture(seat);
        fence(&mut game);
        let mut game = if cloned { game.clone() } else { game };
        let mut selection = step(false);
        if let CompiledStep::SelectUnionZone { filter, zone_filters, material_carrier_filter, zones, .. } = &mut selection {
            let event = CompiledPredicate { event_card_name_contains: Some("Missing".into()), ..Default::default() };
            match reader {
                0 => { *filter = event; *zone_filters = Default::default(); }
                1 => { zone_filters.hand = Some(event); }
                _ => { *zones = vec![CompiledZone::Material]; *material_carrier_filter = Some(event); }
            }
        }
        let mut bindings = Bindings::new(); bindings.insert_card_list("excluded", vec![excluded]);
        let phase = game.current_phase;
        let outcome = catch_unwind(AssertUnwindSafe(|| run_steps(&[selection],
            &mut EffectContext::new(&mut game, source, Some(host), seat), &mut bindings)));
        let error = outcome.unwrap_err();
        let text = error.downcast_ref::<String>().map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied()).unwrap_or("");
        assert_eq!(text, "closed-list unavailable event-card reference read");
        assert!(game.pending_selection.is_none()); assert_eq!(game.current_phase, phase);
        assert_eq!(game.memory, 3);
    } } }
}

#[test]
fn compiled_union_cost_guard_uses_the_same_structured_candidate_rules() {
    for seat in [0, 1] { for fenced in [false, true] { for cloned in [false, true] {
      for present in [false, true] { for pinned in [false, true] {
        let (mut game, source, _, _, host) = fixture(seat);
        if fenced { fence(&mut game); }
        let game = if cloned { game.clone() } else { game };
        let mut selection = step(false);
        if let CompiledStep::SelectUnionZone { filter, zone_filters, material_carrier_filter,
            material_of, zones, cost, then, .. } = &mut selection {
            *zones = vec![CompiledZone::Material];
            *filter = CompiledPredicate { name_contains: Some(if present { "Material" } else { "Absent" }.into()), ..Default::default() };
            *zone_filters = Default::default();
            *material_of = pinned.then_some(CompiledBindingRef::Source);
            *material_carrier_filter = Some(CompiledPredicate { name_contains: Some("Allowed".into()), ..Default::default() });
            *cost = true;
            *then = vec![CompiledStep::TrashUnionBound { binding: "picked".into() }];
        }
        let guard = leading_cost_guard(&[selection]).expect("leading union trash cost");
        let read = EffectReadContext::new(&game, source, Some(host), seat);
        assert_eq!(guard.holds(&read), present);
        assert!(game.pending_selection.is_none()); assert_eq!(game.memory, 3);
      } }
    } } }
}

#[test]
fn public_union_opaque_card_and_carrier_callbacks_remain_strict() {
    for seat in [0, 1] { for fenced in [false, true] { for cloned in [false, true] {
      for carrier in [false, true] {
        let (mut game, source, _, _, host) = fixture(seat);
        if fenced { fence(&mut game); }
        let mut game = if cloned { game.clone() } else { game };
        let calls = Arc::new(AtomicUsize::new(0));
        let card_calls = calls.clone(); let carrier_calls = calls.clone();
        let phase = game.current_phase;
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let mut ctx = EffectContext::new(&mut game, source, Some(host), seat);
            ctx.select_union_zone(seat, if carrier { UnionZoneSet::MATERIAL } else { UnionZoneSet::HAND },
                None, carrier.then(|| Box::new(move |_: &Game, _: PermanentHandle| {
                    carrier_calls.fetch_add(1, Ordering::SeqCst); true
                }) as Box<dyn Fn(&Game, PermanentHandle) -> bool + Send + Sync>),
                "Opaque", false, move |_, _, _| {
                    if !carrier { card_calls.fetch_add(1, Ordering::SeqCst); } true
                }, |_, _, _| panic!("install must not execute callback"));
        }));
        if fenced {
            assert!(outcome.is_err()); assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(game.pending_selection.is_none()); assert_eq!(game.current_phase, phase);
        } else {
            outcome.unwrap(); assert!(calls.load(Ordering::SeqCst) > 0);
            assert!(game.pending_selection.is_some());
        }
        assert_eq!(game.memory, 3);
      }
    } } }
}
