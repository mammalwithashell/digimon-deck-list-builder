//! Complete physical hand/trash/source choices in one union prompt.
use digimon_engine::{
    action::{build_action_mask, space::{BREEDING_TARGET, PASS}},
    card_choice_page::{CARD_CHOICE_NEXT, CARD_CHOICE_START},
    debug_runner::{make_test_card, DebugRunner},
    effect_context::EffectContext,
    permanent::PermanentHandle,
    selection::{UnionZoneOrigin, UnionZoneSet},
};

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("UNION-SOURCE", "Source"))
        .add_card(make_test_card("UNION-TOP", "Top"))
        .deck(0, &["UNION-SOURCE"; 100]).deck(1, &["UNION-SOURCE"; 100])
        .memory(3).start();
    r.skip_mulligan(); r
}
fn carrier(r: &mut DebugRunner, owner: u8, breeding: bool, depth: usize) -> (PermanentHandle, digimon_engine::card_source::CardHandle) {
    let mut stack = vec!["UNION-SOURCE"; depth + 1]; stack.push("UNION-TOP");
    let field = r.place_stack(owner, &stack);
    let expected = r.game.player(owner).battle_area[field.index as usize].card_sources[depth].handle();
    let host = if breeding {
        r.game.players[owner as usize].breeding_area = Some(r.game.players[owner as usize].battle_area.remove(field.index as usize));
        PermanentHandle { player: owner, index: BREEDING_TARGET as u8 }
    } else { field };
    (host, expected)
}

#[test]
fn raw_union_reaches_depth_12_39_in_battle_and_breeding_for_both_owners() {
    for owner in [0, 1] { for breeding in [false, true] { for depth in [11, 12, 39] {
        let mut r = runner(); let (host, expected) = carrier(&mut r, owner, breeding, depth);
        let source = r.game.player(owner).battle_area.first().map(|p| p.top_card().handle())
            .or_else(|| r.game.player(owner).breeding_area.as_ref().map(|p| p.top_card().handle())).unwrap();
        EffectContext::new(&mut r.game, source, Some(host), 0).select_union_zone(
            owner, UnionZoneSet::MATERIAL, Some(host), None, "Union sources", false,
            |_, _, _| true, move |ctx, card, origin| {
                assert_eq!(card, expected);
                let UnionZoneOrigin::Material { carrier, source_index } = origin else { panic!("source origin lost"); };
                assert_eq!(carrier, host); assert_eq!(source_index, depth);
                ctx.game.memory = 8;
            });
        assert!(r.game.pending_selection.as_ref().unwrap().card_choice.is_some(), "all union sources need an owned page");
        for _ in 0..depth / 28 { r.execute_action(0, CARD_CHOICE_NEXT).unwrap(); }
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
        r.execute_action(0, CARD_CHOICE_START + (depth % 28) as u16).unwrap();
        assert_eq!(r.game.memory, 8);
    } } }
}

#[test]
fn dsl_union_deep_material_continuation_clones_and_trashes_exact_card() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledUnionZoneFilters, CompiledZone};
    use digimon_engine::dsl_cards::{bindings::Bindings, step::run_steps};
    for breeding in [false, true] {
        let mut r = runner(); let (host, expected) = carrier(&mut r, 0, breeding, 39);
        let top = if breeding { r.game.player(0).breeding_area.as_ref().unwrap().top_card().handle() }
            else { r.game.player(0).battle_area[host.index as usize].top_card().handle() };
        let mut bindings = Bindings::new(); bindings.insert_permanent("carrier", host);
        let steps = [CompiledStep::SelectUnionZone {
            of: CompiledPlayerRef::You, zones: vec![CompiledZone::Material],
            material_of: Some(CompiledBindingRef::Named("carrier".into())),
            filter: CompiledPredicate::default(), zone_filters: CompiledUnionZoneFilters::default(),
            material_carrier_filter: None, bind_as: Some("chosen".into()), prompt: "Union DSL".into(),
            prompt_key: None, optional: false, cost: false,
            then: vec![CompiledStep::TrashUnionBound { binding: "chosen".into() }],
        }, CompiledStep::GainMemory(4)];
        run_steps(&steps, &mut EffectContext::new(&mut r.game, top, Some(host), 0), &mut bindings);
        assert!(r.game.pending_selection.as_ref().unwrap().card_choice.is_some());
        r.execute_action(0, CARD_CHOICE_NEXT).unwrap();
        let mut cloned = r.game.clone();
        for game in [&mut r.game, &mut cloned] {
            game.resolve_selection(0, CARD_CHOICE_START + 11).unwrap();
            assert!(game.player(0).trash.iter().any(|c| c.handle() == expected));
            assert_eq!(game.memory, 7);
            assert!(game.pending_selection.is_none());
        }
    }
}

#[test]
fn mixed_union_keeps_hand_trash_and_sources_in_one_complete_menu() {
    let mut r = runner(); let (host, _) = carrier(&mut r, 0, false, 39);
    // Populate independently of opening hand size, including beyond old hand/trash action bands.
    let mut cards = std::mem::take(&mut r.game.players[0].deck);
    let mut trash = cards.split_off(cards.len() / 2);
    r.game.players[0].hand.append(&mut cards);
    r.game.players[0].trash.append(&mut trash);
    let hand_count = r.game.player(0).hand.len();
    let trash_count = r.game.player(0).trash.len();
    let source = r.game.player(0).battle_area[host.index as usize].top_card().handle();
    EffectContext::new(&mut r.game, source, Some(host), 0).select_union_zone(
        0, UnionZoneSet::HAND | UnionZoneSet::TRASH | UnionZoneSet::MATERIAL,
        Some(host), None, "Mixed union", true, |_, _, _| true, |_, _, _| {});
    let mut visible = 0;
    loop {
        let owner = r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().expect("union page");
        let own = owner.view(&r.game, 0); let other = owner.view(&r.game, 1);
        for row in &own.rows {
            if matches!(row.operation, digimon_engine::owned_card_choice::CardChoiceOperation::CommitCard) {
                visible += 1;
                assert!(row.identity_visible);
                let peer = other.rows.iter().find(|peer| peer.action_id == row.action_id).unwrap();
                if format!("{:?}", row.zone) == "Some(Hand)" {
                    assert!(!peer.identity_visible); assert!(peer.card_id.is_none());
                } else { assert!(peer.identity_visible); }
            }
        }
        if build_action_mask(&r.game, 0)[CARD_CHOICE_NEXT as usize] == 0.0 { break; }
        r.execute_action(0, CARD_CHOICE_NEXT).unwrap();
    }
    assert_eq!(visible, hand_count + trash_count + 40, "no union zone may be trimmed to a legacy action band");
}

#[test]
fn union_source_pin_rebases_after_carrier_and_stack_compaction() {
    let mut r = runner();
    let removed_host = r.place_stack(0, &["UNION-TOP"]);
    let (host, expected) = carrier(&mut r, 0, false, 39);
    let source = r.game.player(0).battle_area[host.index as usize].top_card().handle();
    EffectContext::new(&mut r.game, source, Some(host), 0).select_union_zone(
        0, UnionZoneSet::MATERIAL, Some(host), None, "Compacted union", false,
        |_, _, _| true, move |ctx, card, origin| {
            assert_eq!(card, expected);
            assert_eq!(origin, UnionZoneOrigin::Material {
                carrier: PermanentHandle { player: 0, index: 0 }, source_index: 38 });
            ctx.game.memory = 9;
        });
    r.execute_action(0, CARD_CHOICE_NEXT).unwrap();
    r.game.players[0].battle_area.remove(removed_host.index as usize);
    let removed = r.game.players[0].battle_area[0].card_sources.remove(0);
    r.game.players[0].trash.push(removed);
    r.execute_action(0, CARD_CHOICE_START + 11).unwrap();
    assert_eq!(r.game.memory, 9);
}

#[test]
fn union_hand_and_trash_commit_cards_beyond_old_action_band_limits() {
    for owner in [0, 1] { for hand in [false, true] {
        let mut r = runner();
        let source_host = r.place_stack(0, &["UNION-TOP"]);
        let source = r.game.player(0).battle_area[source_host.index as usize].top_card().handle();
        let cards: Vec<_> = r.game.players[owner as usize].deck.drain(..40).collect();
        if hand { r.game.players[owner as usize].hand.extend(cards); }
        else { r.game.players[owner as usize].trash.extend(cards); }
        let expected = if hand { r.game.player(owner).hand.last().unwrap().handle() }
            else { r.game.player(owner).trash.last().unwrap().handle() };
        let count = if hand { r.game.player(owner).hand.len() } else { r.game.player(owner).trash.len() };
        EffectContext::new(&mut r.game, source, Some(source_host), 0).select_union_zone(
            owner, if hand { UnionZoneSet::HAND } else { UnionZoneSet::TRASH }, None, None,
            "Deep hand/trash union", false, |_, _, _| true, move |ctx, card, origin| {
                assert_eq!(card, expected);
                assert_eq!(origin, if hand { UnionZoneOrigin::Hand } else { UnionZoneOrigin::Trash });
                ctx.game.memory = 6;
            });
        for _ in 0..(count - 1) / 28 { r.execute_action(0, CARD_CHOICE_NEXT).unwrap(); }
        r.execute_action(0, CARD_CHOICE_START + ((count - 1) % 28) as u16).unwrap();
        assert_eq!(r.game.memory, 6);
    } }
}
