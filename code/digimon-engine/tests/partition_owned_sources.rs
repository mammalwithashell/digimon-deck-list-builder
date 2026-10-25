use digimon_engine::{action::build_action_mask, card_choice_page::CARD_CHOICE_START, debug_runner::{make_test_card, DebugRunner}, effect_context::{EffectContext, PartitionRequirement}};

#[test]
fn partition_reaches_uncapped_physical_sources_for_both_seats() {
    for player in [0, 1] {
        for depth in [11usize, 12, 39, 300] {
            let mut r = DebugRunner::builder().add_card(make_test_card("PART-SOURCE", "Source"))
                .add_card(make_test_card("PART-TOP", "Top"))
                .deck(0, &["PART-SOURCE"; 20]).deck(1, &["PART-SOURCE"; 20]).memory(3).start();
            r.skip_mulligan();
            let mut stack = vec!["PART-SOURCE"; depth + 1]; stack.push("PART-TOP");
            let host = r.place_stack(player, &stack);
            let expected = r.game.player(player).battle_area[host.index as usize].card_sources[depth].handle();
            let top = r.game.player(player).battle_area[host.index as usize].top_card().handle();
            EffectContext::new(&mut r.game, top, Some(host), player).select_partition_sources(host, "Deep Partition", vec![
                PartitionRequirement::new("deep source", move |_, source| source.card == expected)
            ], move |ctx, selected| {
                assert_eq!(selected.len(), 1); assert_eq!(selected[0].card, expected);
                assert_eq!(selected[0].source_index, depth); ctx.game.memory = 7;
            });
            assert!(r.game.pending_selection.is_some(), "Partition must expose source {depth}");
            assert_eq!(build_action_mask(&r.game, player)[CARD_CHOICE_START as usize], 1.0);
            r.game.resolve_selection(player, CARD_CHOICE_START).unwrap();
            assert_eq!(r.game.memory, 7);
        }
    }
}

#[test]
fn partition_rebases_the_selected_prefix_after_source_compaction() {
    let mut r = DebugRunner::builder().add_card(make_test_card("PART-SOURCE", "Source"))
        .add_card(make_test_card("PART-TOP", "Top"))
        .deck(0, &["PART-SOURCE"; 20]).deck(1, &["PART-SOURCE"; 20]).memory(3).start();
    r.skip_mulligan();
    let mut stack = vec!["PART-SOURCE"; 40]; stack.push("PART-TOP");
    let host = r.place_stack(0, &stack);
    let expected = r.game.player(0).battle_area[0].card_sources[39].handle();
    let top = r.game.player(0).battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).select_partition_sources(host, "Partition pair", vec![
        PartitionRequirement::new("any", |_, _| true), PartitionRequirement::new("any", |_, _| true)
    ], move |ctx, selected| {
        assert_eq!(selected[0].card, expected);
        assert_eq!(selected[0].source_index, 38, "earlier pick must follow its physical source after compaction");
        for source in selected { assert_eq!(ctx.game.player(0).battle_area[0].card_sources[source.source_index].handle(), source.card); }
        ctx.game.memory = 7;
    });
    r.game.resolve_selection(0, digimon_engine::card_choice_page::CARD_CHOICE_NEXT).unwrap();
    assert_eq!(r.game.memory, 3, "navigation cannot run Partition's terminal");
    r.game.resolve_selection(0, CARD_CHOICE_START + 11).unwrap();
    r.game.players[0].battle_area[0].card_sources.remove(0);
    r.game.resolve_selection(0, CARD_CHOICE_START + 1).unwrap();
    assert_eq!(r.game.memory, 7);
}

#[path = "replacements/partition.rs"]
mod existing_partition_controls;
