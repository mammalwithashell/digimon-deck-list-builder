//! Public selection reachability through the coordinated card paging owner.
use digimon_engine::{
    action::{build_action_mask, space::PASS},
    card_choice_page::{CARD_CHOICE_NEXT, CARD_CHOICE_PREVIOUS, CARD_CHOICE_START},
    debug_runner::{make_test_card, DebugRunner},
    effect_context::EffectContext,
};

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("OWNED-SOURCE", "Source"))
        .add_card(make_test_card("OWNED-TOP", "Top"))
        .deck(0, &["OWNED-SOURCE"; 20])
        .deck(1, &["OWNED-SOURCE"; 20])
        .security(0, &["OWNED-SOURCE"; 40])
        .memory(3)
        .start();
    r.skip_mulligan();
    r
}

#[test]
fn dsl_capped_source_page_commits_physical_identity_and_clones_required_prefix() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledCountBound, CompiledPredicate, CompiledStep};
    use digimon_engine::{dsl_cards::{bindings::Bindings, step::run_steps}, resume::ResumeFrame};
    let mut r = runner();
    let mut stack = vec!["OWNED-SOURCE"; 40];
    stack.push("OWNED-TOP");
    let host = r.place_stack(0, &stack);
    let top = r.game.player(0).battle_area[0].top_card().handle();
    let deep = r.game.player(0).battle_area[0].card_sources[39].handle();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("carrier", host);
    let steps = vec![CompiledStep::SelectMaterials {
        named_groups: Vec::new(), require_max: false,
        of_permanent: CompiledBindingRef::Named("carrier".into()),
        min: 2, max: CompiledCountBound::Literal(2), filter: CompiledPredicate::default(),
        uniqueness: None, bind_as: Some("chosen".into()), prompt: "Capped physical sources".into(),
        prompt_key: None, optional_zero: false, same_by: None,
    }, CompiledStep::GainMemory(4)];
    run_steps(&steps, &mut EffectContext::new(&mut r.game, top, Some(host), 0), &mut bindings);
    r.execute_action(0, CARD_CHOICE_NEXT).unwrap();
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        game.resolve_selection(0, CARD_CHOICE_START + 11).unwrap();
        let frames = &game.pending_selection_resume.as_ref().unwrap().frames;
        let Some(ResumeFrame::MultiPickStep(state)) = frames.last() else { panic!("required prefix must remain parked"); };
        assert_eq!(state.accum, vec![deep], "page row must commit its physical source");
        assert_eq!(game.memory, 3, "outer reward waits for the required second pick");
        assert_eq!(build_action_mask(game, 0)[PASS as usize], 0.0);
        game.resolve_selection(0, CARD_CHOICE_START).unwrap();
        assert!(game.pending_selection.is_none());
        assert_eq!(game.memory, 7);
    }
}

#[test]
fn own_and_opponent_multi_source_pickers_reach_physical_depth_39() {
    for (owner, depth) in [(0, 11), (0, 12), (1, 11), (1, 12), (0, 39), (1, 39), (0, 300), (1, 300)] {
        let mut r = runner();
        let mut stack = vec!["OWNED-SOURCE"; depth + 1];
        stack.push("OWNED-TOP");
        let target = r.place_stack(owner, &stack);
        let expected =
            r.game.player(owner).battle_area[target.index as usize].card_sources[depth].handle();
        let source = r.place_stack(0, &["OWNED-TOP"]);
        let top = r.game.player(0).battle_area[source.index as usize]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut r.game, top, Some(source), 0);
        let filter =
            move |_: &digimon_engine::game::Game,
                  candidate: digimon_engine::selection::SourceSelectionRef| {
                candidate.card == expected
            };
        let callback =
            move |ctx: &mut EffectContext<'_>,
                  selected: Vec<digimon_engine::selection::SourceSelectionRef>| {
                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].card, expected);
                assert_eq!(selected[0].source_index as usize, depth);
                ctx.game.memory = 7;
            };
        if owner == 0 {
            ctx.select_own_sources("Own deep source", 1, 1, filter, callback);
        } else {
            ctx.select_opponent_sources("Opponent deep source", 1, 1, filter, callback);
        }
        let pending = r
            .pending_selection_view()
            .expect("deep multi-source candidate must be offered");
        assert_eq!(pending.valid_action_ids, vec![CARD_CHOICE_START]);
        assert_eq!(
            build_action_mask(&r.game, 0)[CARD_CHOICE_START as usize],
            1.0
        );
        r.execute_action(0, CARD_CHOICE_START).unwrap();
        assert_eq!(r.game.memory, 7);
    }
}

#[test]
fn count_capped_material_reaches_deep_battle_and_breeding_sources() {
    use digimon_engine::{
        action::space::BREEDING_TARGET, effect_context::CountCappedZone, permanent::PermanentHandle,
    };
    for (breeding, depth) in [(false, 11), (false, 12), (false, 39), (true, 11), (true, 12), (true, 39)] {
        let mut r = runner();
        let mut stack = vec!["OWNED-SOURCE"; depth + 1];
        stack.push("OWNED-TOP");
        let battle = r.place_stack(0, &stack);
        let expected =
            r.game.player(0).battle_area[battle.index as usize].card_sources[depth].handle();
        let top = r.game.player(0).battle_area[battle.index as usize]
            .top_card()
            .handle();
        let host = if breeding {
            r.game.players[0].breeding_area =
                Some(r.game.players[0].battle_area.remove(battle.index as usize));
            PermanentHandle {
                player: 0,
                index: BREEDING_TARGET as u8,
            }
        } else {
            battle
        };
        EffectContext::new(&mut r.game, top, Some(host), 0).select_count_capped_multi_min(
            0,
            CountCappedZone::Material(host),
            1,
            1,
            "Deep capped material",
            false,
            None,
            move |_, card| card.handle() == expected,
            move |ctx, cards| {
                assert_eq!(cards, vec![expected]);
                ctx.game.memory = 7;
            },
        );
        let pending = r
            .pending_selection_view()
            .expect("deep capped material must be offered");
        assert_eq!(pending.valid_action_ids, vec![CARD_CHOICE_START]);
        assert_eq!(
            build_action_mask(&r.game, 0)[CARD_CHOICE_START as usize],
            1.0
        );
        r.execute_action(0, CARD_CHOICE_START).unwrap();
        assert_eq!(r.game.memory, 7);
    }
}

#[test]
fn dsl_multi_source_second_page_and_partial_prefix_survive_clone() {
    use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};
    use digimon_engine::dsl_cards::{
        bindings::Bindings,
        step::{run_steps, RunOutcome},
    };
    let mut r = runner();
    let mut stack = vec!["OWNED-SOURCE"; 40];
    stack.push("OWNED-TOP");
    let host = r.place_stack(0, &stack);
    let first = r.game.player(0).battle_area[0].card_sources[0].handle();
    let deep = r.game.player(0).battle_area[0].card_sources[39].handle();
    let top = r.game.player(0).battle_area[0].top_card().handle();
    let steps = vec![CompiledStep::SelectOwnSources {
        target: None,
        filter: CompiledPredicate::default(),
        min: 2,
        max: 2,
        bind_as: Some("chosen".into()),
        prompt: "Choose two physical sources".into(),
        then: vec![CompiledStep::TrashSelectedSources {
            source_refs: "chosen".into(),
        }],
    }];
    let outcome = run_steps(
        &steps,
        &mut EffectContext::new(&mut r.game, top, Some(host), 0),
        &mut Bindings::new(),
    );
    assert_eq!(outcome, RunOutcome::Parked);
    assert_eq!(
        build_action_mask(&r.game, 0)[CARD_CHOICE_NEXT as usize],
        1.0
    );
    r.execute_action(0, CARD_CHOICE_NEXT).unwrap();
    let mut clone = r.game.clone();
    for game in [&mut r.game, &mut clone] {
        game.resolve_selection(0, CARD_CHOICE_START + 11).unwrap();
        assert_eq!(
            build_action_mask(game, 0)[PASS as usize],
            0.0,
            "required prefix cannot stop at one"
        );
        game.resolve_selection(0, CARD_CHOICE_START).unwrap();
        assert!(game.pending_selection.is_none());
        let mut trashed: Vec<_> = game.player(0).trash.iter().map(|c| c.handle()).collect();
        let mut expected = vec![deep, first];
        trashed.sort_by_key(|card| card.0);
        expected.sort_by_key(|card| card.0);
        assert_eq!(trashed, expected);
        assert_eq!(game.player(0).battle_area[0].card_sources.len(), 39);
    }
}

#[test]
fn security_beyond_the_old_ten_slots_is_a_real_masked_choice() {
    let mut r = runner();
    let host = r.place_stack(0, &["OWNED-TOP"]);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).select_security(
        0,
        "Deep security",
        false,
        |_, i| i == 39,
        |ctx, index| ctx.game.memory = index as i16,
    );
    let p = r
        .pending_selection_view()
        .expect("eligible physical security39 must be offered");
    assert_eq!(p.valid_action_ids, vec![CARD_CHOICE_START]);
    assert_eq!(
        build_action_mask(&r.game, 0)[CARD_CHOICE_START as usize],
        1.0
    );
    r.execute_action(0, CARD_CHOICE_START).unwrap();
    assert_eq!(r.game.memory, 39);
}

#[test]
fn material_beyond_the_old_stride_is_a_real_masked_choice() {
    let mut r = runner();
    let mut stack = vec!["OWNED-SOURCE"; 40];
    stack.push("OWNED-TOP");
    let host = r.place_stack(0, &stack);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).select_material(
        host,
        "Deep material",
        false,
        |_, i| i == 39,
        |ctx, index| ctx.game.memory = index as i16,
    );
    let p = r
        .pending_selection_view()
        .expect("eligible physical source39 must be offered");
    assert_eq!(p.valid_action_ids, vec![CARD_CHOICE_START]);
    assert_eq!(
        build_action_mask(&r.game, 0)[CARD_CHOICE_START as usize],
        1.0
    );
    r.execute_action(0, CARD_CHOICE_START).unwrap();
    assert_eq!(r.game.memory, 39);
}

#[test]
fn page_navigation_has_no_gameplay_or_rng_effect_and_commits_the_visible_card() {
    use rand::RngCore;
    let mut r = runner();
    let mut stack = vec!["OWNED-SOURCE"; 65];
    stack.push("OWNED-TOP");
    let host = r.place_stack(0, &stack);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).select_material(
        host,
        "Many materials",
        true,
        |_, _| true,
        |ctx, index| ctx.game.memory = index as i16,
    );
    let before = r.game.players[0].battle_area[0].card_sources.clone();
    let mut rng = r.game.rng.clone();
    assert!(r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .contains(&CARD_CHOICE_NEXT));
    for action in [
        CARD_CHOICE_NEXT,
        CARD_CHOICE_PREVIOUS,
        CARD_CHOICE_NEXT,
        CARD_CHOICE_NEXT,
    ] {
        assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
        r.execute_action(0, action).unwrap();
        assert_eq!(r.game.memory, 3);
        assert_eq!(
            r.game.players[0].battle_area[0]
                .card_sources
                .iter()
                .map(|c| c.handle())
                .collect::<Vec<_>>(),
            before.iter().map(|c| c.handle()).collect::<Vec<_>>()
        );
    }
    assert_eq!(r.game.rng.next_u64(), rng.next_u64());
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    r.execute_action(0, CARD_CHOICE_START + 8).unwrap();
    assert_eq!(r.game.memory, 64);
}

#[test]
fn checked_submissions_reject_old_page_tokens_without_advancing_any_state() {
    let mut r = runner();
    let mut stack = vec!["OWNED-SOURCE"; 40];
    stack.push("OWNED-TOP");
    let host = r.place_stack(0, &stack);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).select_material(
        host,
        "Tokened material",
        true,
        |_, _| true,
        |_, _| {},
    );
    let old = r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .card_choice
        .as_ref()
        .unwrap()
        .token();
    r.game
        .resolve_selection_checked(0, CARD_CHOICE_NEXT, old)
        .unwrap();
    let before = r.game.verification_digest();
    assert!(r
        .game
        .resolve_selection_checked(0, CARD_CHOICE_START, old)
        .is_err());
    assert_eq!(r.game.verification_digest(), before);
    let current = r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .card_choice
        .as_ref()
        .unwrap()
        .token();
    assert_ne!(old, current);
    r.game
        .resolve_selection_checked(0, CARD_CHOICE_START, current)
        .unwrap();
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn stale_security_order_cannot_select_a_replacement_at_an_old_position() {
    let mut r = runner();
    let host = r.place_stack(0, &["OWNED-TOP"]);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).select_security(
        0,
        "Pinned security",
        true,
        |_, _| true,
        |_, _| panic!("stale card committed"),
    );
    let token = r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .card_choice
        .as_ref()
        .unwrap()
        .token();
    r.game.shuffle_security(0);
    let before = r.game.verification_digest();
    assert_eq!(
        build_action_mask(&r.game, 0)[CARD_CHOICE_START as usize],
        0.0
    );
    assert!(r
        .game
        .resolve_selection_checked(0, CARD_CHOICE_START, token)
        .is_err());
    assert_eq!(r.game.verification_digest(), before);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    r.game.resolve_selection_checked(0, PASS, token).unwrap();
    assert!(r.game.pending_selection.is_none());
}
