use std::sync::{Arc, Mutex};

use digimon_engine::action::space::encode_source_select;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::{EffectContext, SourceSelectionRef};
use digimon_engine::enums::{CardColor, CardKind, CostDelta};
use digimon_engine::permanent::PermanentHandle;

fn card(id: &str, kind: CardKind, traits: &[&str]) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: kind,
        level: matches!(kind, CardKind::Digimon).then_some(4),
        dp: matches!(kind, CardKind::Digimon).then_some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: traits.iter().map(|s| s.to_string()).collect(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn insert_source_below_top(
    runner: &mut DebugRunner,
    host: PermanentHandle,
    card_id: &str,
) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|data| data.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown test card {card_id}"));
    let card = CardSource::new(data_idx, host.player, runner.game.next_card_index());
    let handle = card.handle();
    let sources = &mut runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources;
    let insert_at = sources.len().saturating_sub(1);
    sources.insert(insert_at, card);
    handle
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|data| data.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown test card {card_id}"));
    let card = CardSource::new(data_idx, player, runner.game.next_card_index());
    let handle = card.handle();
    runner.game.players[player as usize].hand.push(card);
    handle
}

fn source_ids(runner: &DebugRunner, host: PermanentHandle) -> Vec<String> {
    runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn xros_or_blue_flare(game: &digimon_engine::game::Game, handle: CardHandle) -> bool {
    game.card_data_for_handle(handle).is_some_and(|data| {
        data.traits
            .iter()
            .any(|trait_name| trait_name == "Xros Heart" || trait_name == "Blue Flare")
    })
}

#[test]
fn move_all_matching_sources_under_tamer_preserves_source_order() {
    let mut runner = DebugRunner::builder()
        .add_card(card("SOURCE-HOST", CardKind::Digimon, &["Xros Heart"]))
        .add_card(card("TARGET-TAMER", CardKind::Tamer, &[]))
        .add_card(card("SRC-A", CardKind::Digimon, &["Xros Heart"]))
        .add_card(card("SRC-B", CardKind::Digimon, &["Puppet"]))
        .add_card(card("SRC-C", CardKind::Digimon, &["Blue Flare"]))
        .memory(5)
        .start();

    let host = runner.place_on_field(0, "SOURCE-HOST", None);
    let tamer = runner.place_on_field(0, "TARGET-TAMER", None);
    let source_card = runner.game.players[0].battle_area[host.index as usize]
        .top_card()
        .handle();

    insert_source_below_top(&mut runner, host, "SRC-A");
    insert_source_below_top(&mut runner, host, "SRC-B");
    insert_source_below_top(&mut runner, host, "SRC-C");
    assert_eq!(
        source_ids(&runner, host),
        vec!["SRC-A", "SRC-B", "SRC-C", "SOURCE-HOST"]
    );

    let moved = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(host), 0);
        ctx.move_all_matching_sources_under_tamer(host, tamer, xros_or_blue_flare)
    };

    assert_eq!(moved, 2);
    assert_eq!(source_ids(&runner, host), vec!["SRC-B", "SOURCE-HOST"]);
    assert_eq!(
        source_ids(&runner, tamer),
        vec!["SRC-A", "SRC-C", "TARGET-TAMER"],
        "moved sources keep their original bottom-to-top order under the Tamer"
    );
}

#[test]
fn move_selected_sources_under_tamer_uses_filtered_up_to_n_selection() {
    let mut runner = DebugRunner::builder()
        .add_card(card("SOURCE-HOST", CardKind::Digimon, &["Xros Heart"]))
        .add_card(card("TARGET-TAMER", CardKind::Tamer, &[]))
        .add_card(card("SRC-A", CardKind::Digimon, &["Xros Heart"]))
        .add_card(card("SRC-B", CardKind::Digimon, &["Puppet"]))
        .add_card(card("SRC-C", CardKind::Digimon, &["Blue Flare"]))
        .memory(5)
        .start();

    let host = runner.place_on_field(0, "SOURCE-HOST", None);
    let tamer = runner.place_on_field(0, "TARGET-TAMER", None);
    let source_card = runner.game.players[0].battle_area[host.index as usize]
        .top_card()
        .handle();

    insert_source_below_top(&mut runner, host, "SRC-A");
    insert_source_below_top(&mut runner, host, "SRC-B");
    insert_source_below_top(&mut runner, host, "SRC-C");

    let moved = Arc::new(Mutex::new(0usize));
    {
        let moved = Arc::clone(&moved);
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(host), 0);
        ctx.select_own_sources(
            "Choose up to 2 matching sources to place under a Tamer",
            0,
            2,
            |game, source_ref: SourceSelectionRef| xros_or_blue_flare(game, source_ref.card),
            move |ctx, refs| {
                *moved.lock().unwrap() = ctx.move_selected_sources_under_tamer(refs, tamer);
            },
        );
    }

    let a_action = encode_source_select(host.index as u16, 0).unwrap();
    let b_action = encode_source_select(host.index as u16, 1).unwrap();
    let c_action = encode_source_select(host.index as u16, 2).unwrap();
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("filtered source movement installs source selection");
    assert!(pending.valid_action_ids.contains(&a_action));
    assert!(pending.valid_action_ids.contains(&c_action));
    assert!(!pending.valid_action_ids.contains(&b_action));

    runner
        .game
        .resolve_selection(pending.selecting_player, a_action)
        .expect("select first matching source");
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("up-to-N source selection remains open");
    runner
        .game
        .resolve_selection(pending.selecting_player, c_action)
        .expect("select second matching source");
    assert!(
        runner.game.pending_selection.is_none(),
        "reaching the max pick count commits the source selection"
    );

    assert_eq!(*moved.lock().unwrap(), 2);
    assert_eq!(source_ids(&runner, host), vec!["SRC-B", "SOURCE-HOST"]);
    assert_eq!(
        source_ids(&runner, tamer),
        vec!["SRC-A", "SRC-C", "TARGET-TAMER"]
    );
}

#[test]
fn moved_source_count_can_feed_followup_play_cost_reduction() {
    let mut runner = DebugRunner::builder()
        .add_card(card("SOURCE-HOST", CardKind::Digimon, &["Xros Heart"]))
        .add_card(card("TARGET-TAMER", CardKind::Tamer, &[]))
        .add_card(card("SRC-A", CardKind::Digimon, &["Xros Heart"]))
        .add_card(card("SRC-C", CardKind::Digimon, &["Blue Flare"]))
        .add_card(card("PLAY-HAND", CardKind::Digimon, &["Xros Heart"]))
        .memory(5)
        .start();

    let host = runner.place_on_field(0, "SOURCE-HOST", None);
    let tamer = runner.place_on_field(0, "TARGET-TAMER", None);
    let source_card = runner.game.players[0].battle_area[host.index as usize]
        .top_card()
        .handle();

    insert_source_below_top(&mut runner, host, "SRC-A");
    insert_source_below_top(&mut runner, host, "SRC-C");
    push_to_hand(&mut runner, 0, "PLAY-HAND");

    let played = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(host), 0);
        let moved = ctx.move_all_matching_sources_under_tamer(host, tamer, xros_or_blue_flare);
        ctx.play_from_hand_with_cost(0, 0, CostDelta::Reduce(moved as i16))
    };

    assert!(played.is_some());
    assert_eq!(
        runner.game.memory, 3,
        "PLAY-HAND printed cost 4 reduced by moved count 2 pays 2 memory"
    );
    assert_eq!(
        source_ids(&runner, tamer),
        vec!["SRC-A", "SRC-C", "TARGET-TAMER"]
    );
}
