use digimon_engine::{action::space::PASS, card_choice_page::CARD_CHOICE_START,
    debug_runner::{make_test_card, DebugRunner}, effect_context::EffectContext};

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder().add_card(make_test_card("PREFIX-S", "Source"))
        .add_card(make_test_card("PREFIX-T", "Top"))
        .deck(0, &["PREFIX-S"; 20]).deck(1, &["PREFIX-S"; 20]).memory(3).start();
    r.skip_mulligan(); r
}

#[test]
fn raw_source_prefix_rebases_carrier_and_source_compaction() {
    let mut r = runner();
    r.place_stack(0, &["PREFIX-T"]);
    let host = r.place_stack(0, &["PREFIX-S", "PREFIX-S", "PREFIX-S", "PREFIX-T"]);
    let expected = r.game.players[0].battle_area[1].card_sources[1].handle();
    let top = r.game.players[0].battle_area[1].top_card().handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).select_own_sources("Two", 2, 2,
        |_, _| true, move |ctx, picks| {
            assert_eq!(picks[0].card, expected);
            assert_eq!(picks[0].permanent.index, 0, "prefix must resolve original carrier after field compaction");
            assert_eq!(picks[0].source_index, 0, "prefix must resolve source position after compaction");
            ctx.game.memory = 7;
        });
    r.execute_action(0, CARD_CHOICE_START + 1).unwrap();
    r.game.players[0].battle_area.remove(0);
    r.game.players[0].battle_area[0].card_sources.remove(0);
    r.execute_action(0, CARD_CHOICE_START + 1).unwrap();
    assert_eq!(r.game.memory, 7);
}

#[test]
fn dsl_moved_prefix_cannot_pay_a_required_two_source_selection() {
    use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};
    use digimon_engine::dsl_cards::{bindings::Bindings, step::run_steps};
    let mut r = runner();
    let host = r.place_stack(0, &["PREFIX-S", "PREFIX-S", "PREFIX-S", "PREFIX-T"]);
    let other = r.place_stack(0, &["PREFIX-T"]);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    run_steps(&[CompiledStep::SelectOwnSources { target: None, filter: CompiledPredicate::default(),
        min: 2, max: 2, bind_as: Some("chosen".into()), prompt: "Two".into(),
        then: vec![CompiledStep::GainMemory(4)] }],
        &mut EffectContext::new(&mut r.game, top, Some(host), 0), &mut Bindings::new());
    r.execute_action(0, CARD_CHOICE_START).unwrap();
    let moved = r.game.players[0].battle_area[0].card_sources.remove(0);
    r.game.players[0].battle_area[other.index as usize].card_sources.insert(0, moved);
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        game.resolve_selection(0, CARD_CHOICE_START).unwrap();
        assert_eq!(game.memory, 3, "a card moved away from its original carrier no longer pays the prefix");
        assert!(game.pending_selection.is_some(), "must request the missing second live source");
        assert_eq!(digimon_engine::action::build_action_mask(game, 0)[PASS as usize], 0.0);
        game.resolve_selection(0, CARD_CHOICE_START).unwrap();
        assert_eq!(game.memory, 7);
    }
}

#[test]
fn dsl_capped_material_rebases_candidate_pool_after_source_compaction() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledCountBound, CompiledPredicate, CompiledStep};
    use digimon_engine::dsl_cards::{bindings::Bindings, step::run_steps};
    let mut r = runner();
    r.place_stack(0, &["PREFIX-T"]);
    let host = r.place_stack(0, &["PREFIX-S", "PREFIX-S", "PREFIX-S", "PREFIX-S", "PREFIX-T"]);
    let last = r.game.players[0].battle_area[1].card_sources[3].handle();
    let top = r.game.players[0].battle_area[1].top_card().handle();
    let mut bindings = Bindings::new(); bindings.insert_permanent("host", host);
    run_steps(&[CompiledStep::SelectMaterials { named_groups: Vec::new(), require_max: false,
        of_permanent: CompiledBindingRef::Named("host".into()), min: 3, max: CompiledCountBound::Literal(3),
        filter: CompiledPredicate::default(), uniqueness: None, bind_as: Some("picked".into()),
        prompt: "Three".into(), prompt_key: None, optional_zero: false, same_by: None },
        CompiledStep::GainMemory(4)],
        &mut EffectContext::new(&mut r.game, top, Some(host), 0), &mut bindings);
    r.execute_action(0, CARD_CHOICE_START + 1).unwrap();
    r.game.players[0].battle_area.remove(0);
    r.game.players[0].battle_area[0].card_sources.remove(0);
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        game.resolve_selection(0, CARD_CHOICE_START + 1).unwrap();
        let owner = game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap();
        let rows = owner.view(game, 0).rows;
        let commits: Vec<_> = rows.iter().filter(|row| matches!(row.operation,
            digimon_engine::owned_card_choice::CardChoiceOperation::CommitCard)).collect();
        assert_eq!(commits.len(), 1, "already selected physical sources cannot reappear after compaction");
        assert_eq!(commits[0].position.as_deref(), Some("2"));
        let frames = &game.pending_selection_resume.as_ref().unwrap().frames;
        let digimon_engine::resume::ResumeFrame::MultiPickStep(state) = frames.last().unwrap() else { panic!() };
        assert!(!state.accum.contains(&last));
        assert_eq!(game.memory, 3);
        game.resolve_selection(0, CARD_CHOICE_START).unwrap();
        assert_eq!(game.memory, 7);
    }
}

#[test]
fn dsl_capped_material_moved_prefix_does_not_satisfy_required_floor() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledCountBound, CompiledPredicate, CompiledStep};
    use digimon_engine::dsl_cards::{bindings::Bindings, step::run_steps};
    let mut r = runner();
    let host = r.place_stack(0, &["PREFIX-S", "PREFIX-S", "PREFIX-S", "PREFIX-T"]);
    let other = r.place_stack(0, &["PREFIX-T"]);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    let mut bindings = Bindings::new(); bindings.insert_permanent("host", host);
    run_steps(&[CompiledStep::SelectMaterials { named_groups: Vec::new(), require_max: false,
        of_permanent: CompiledBindingRef::Named("host".into()), min: 2, max: CompiledCountBound::Literal(2),
        filter: CompiledPredicate::default(), uniqueness: None, bind_as: Some("picked".into()),
        prompt: "Two".into(), prompt_key: None, optional_zero: false, same_by: None },
        CompiledStep::GainMemory(4)],
        &mut EffectContext::new(&mut r.game, top, Some(host), 0), &mut bindings);
    r.execute_action(0, CARD_CHOICE_START).unwrap();
    let moved = r.game.players[0].battle_area[0].card_sources.remove(0);
    r.game.players[0].battle_area[other.index as usize].card_sources.insert(0, moved);
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        game.resolve_selection(0, CARD_CHOICE_START).unwrap();
        assert_eq!(game.memory, 3);
        assert!(game.pending_selection.is_some());
        assert_eq!(digimon_engine::action::build_action_mask(game, 0)[PASS as usize], 0.0);
        game.resolve_selection(0, CARD_CHOICE_START).unwrap();
        assert_eq!(game.memory, 7);
        assert!(game.pending_selection.is_none());
    }
}

#[test]
fn dsl_source_target_restriction_follows_original_carrier_after_compaction() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate, CompiledStep};
    use digimon_engine::dsl_cards::{bindings::Bindings, step::run_steps};
    let mut r = runner();
    r.place_stack(0, &["PREFIX-T"]);
    let host = r.place_stack(0, &["PREFIX-S", "PREFIX-S", "PREFIX-S", "PREFIX-T"]);
    let top = r.game.players[0].battle_area[1].top_card().handle();
    let mut bindings = Bindings::new(); bindings.insert_permanent("host", host);
    run_steps(&[CompiledStep::SelectOwnSources { target: Some(CompiledBindingRef::Named("host".into())),
        filter: CompiledPredicate::default(), min: 3, max: 3, bind_as: Some("picked".into()),
        prompt: "Three from original carrier".into(), then: vec![CompiledStep::GainMemory(4)] }],
        &mut EffectContext::new(&mut r.game, top, Some(host), 0), &mut bindings);
    r.execute_action(0, CARD_CHOICE_START).unwrap();
    r.game.players[0].battle_area.remove(0);
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        game.resolve_selection(0, CARD_CHOICE_START).unwrap();
        assert_eq!(game.memory, 3);
        assert!(game.pending_selection.is_some(), "target restriction must keep the compacted physical carrier");
        game.resolve_selection(0, CARD_CHOICE_START).unwrap();
        assert_eq!(game.memory, 7);
    }
}
