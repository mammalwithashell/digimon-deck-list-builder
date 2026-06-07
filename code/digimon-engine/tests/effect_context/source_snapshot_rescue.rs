use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::trigger_context::{DeletedObjectSnapshot, EventCause, TriggerContext};

fn card(id: &str, traits: &[&str]) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
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

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|data| data.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown test card {card_id}"));
    let card = CardSource::new(data_idx, player, runner.game.next_card_index());
    let handle = card.handle();
    runner.game.players[player as usize].trash.push(card);
    handle
}

fn tamer(id: &str) -> CardData {
    let mut data = card(id, &[]);
    data.card_kind = CardKind::Tamer;
    data.level = None;
    data.dp = None;
    data
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
fn deleted_source_snapshot_rescue_uses_arbitrary_trait_filter() {
    let mut runner = DebugRunner::builder()
        .add_card(card("DELETED-TOP", &["Xros Heart"]))
        .add_card(card("XROS-SOURCE", &["Xros Heart"]))
        .add_card(card("BLUE-SOURCE", &["Blue Flare"]))
        .add_card(card("OTHER-SOURCE", &["Puppet"]))
        .memory(5)
        .start();

    let deleted_top = push_to_trash(&mut runner, 0, "DELETED-TOP");
    let xros_source = push_to_trash(&mut runner, 0, "XROS-SOURCE");
    let blue_source = push_to_trash(&mut runner, 0, "BLUE-SOURCE");
    let other_source = push_to_trash(&mut runner, 0, "OTHER-SOURCE");

    runner.game.current_trigger_context = Some(TriggerContext {
        deleted_object: Some(DeletedObjectSnapshot {
            former_controller: 0,
            top_card: deleted_top,
            card_kind: CardKind::Digimon,
            traits: vec!["Xros Heart".to_string()],
            level: Some(4),
            dp: Some(4000),
            cause: EventCause::EffectDeletion,
            dp_just_before: Some(4000),
            level_just_before: Some(4),
            cost_just_before: Some(4),
            names_just_before: vec!["DELETED-TOP".to_string()],
            traits_just_before: vec!["Xros Heart".to_string()],
            source_count_just_before: 3,
            digisources_just_before: vec![xros_source, blue_source, other_source],
            is_token: false,
        }),
        ..TriggerContext::default()
    });

    let picked = Arc::new(Mutex::new(None::<Vec<CardHandle>>));
    {
        let picked = Arc::clone(&picked);
        let mut ctx = EffectContext::new(&mut runner.game, deleted_top, None, 0);
        ctx.select_deleted_self_digisources_from_trash(
            4,
            "select trait-filtered source snapshot cards",
            true,
            xros_or_blue_flare,
            move |_ctx, handles| {
                *picked.lock().unwrap() = Some(handles);
            },
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("eligible snapshot source cards install a rescue prompt");
    assert!(pending.valid_action_ids.contains(&(TRASH_EFFECT_START + 1)));
    assert!(pending.valid_action_ids.contains(&(TRASH_EFFECT_START + 2)));
    assert!(
        !pending.valid_action_ids.contains(&(TRASH_EFFECT_START + 3)),
        "arbitrary trait filter excludes nonmatching source snapshot card"
    );

    runner
        .game
        .resolve_selection(pending.selecting_player, TRASH_EFFECT_START + 2)
        .expect("choose Blue Flare source from snapshot");
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("up-to-N snapshot selection remains open after one pick");
    runner
        .game
        .resolve_selection(pending.selecting_player, PASS)
        .expect("finish source snapshot selection");

    assert_eq!(*picked.lock().unwrap(), Some(vec![blue_source]));
}

#[test]
fn snapshot_rescue_moves_selected_handles_under_tamer_after_carrier_left_battle() {
    let mut runner = DebugRunner::builder()
        .add_card(card("DELETED-TOP", &["Xros Heart"]))
        .add_card(card("XROS-SOURCE", &["Xros Heart"]))
        .add_card(card("BLUE-SOURCE", &["Blue Flare"]))
        .add_card(card("OTHER-SOURCE", &["Puppet"]))
        .add_card(tamer("TARGET-TAMER"))
        .memory(5)
        .start();

    let tamer = runner.place_on_field(0, "TARGET-TAMER", None);
    let deleted_top = push_to_trash(&mut runner, 0, "DELETED-TOP");
    let xros_source = push_to_trash(&mut runner, 0, "XROS-SOURCE");
    let blue_source = push_to_trash(&mut runner, 0, "BLUE-SOURCE");
    let other_source = push_to_trash(&mut runner, 0, "OTHER-SOURCE");

    runner.game.current_trigger_context = Some(TriggerContext {
        deleted_object: Some(DeletedObjectSnapshot {
            former_controller: 0,
            top_card: deleted_top,
            card_kind: CardKind::Digimon,
            traits: vec!["Xros Heart".to_string()],
            level: Some(4),
            dp: Some(4000),
            cause: EventCause::EffectDeletion,
            dp_just_before: Some(4000),
            level_just_before: Some(4),
            cost_just_before: Some(4),
            names_just_before: vec!["DELETED-TOP".to_string()],
            traits_just_before: vec!["Xros Heart".to_string()],
            source_count_just_before: 3,
            digisources_just_before: vec![xros_source, blue_source, other_source],
            is_token: false,
        }),
        ..TriggerContext::default()
    });

    {
        let mut ctx = EffectContext::new(&mut runner.game, deleted_top, None, 0);
        ctx.select_deleted_self_digisources_from_trash(
            2,
            "select source snapshot cards to rescue under Tamer",
            false,
            xros_or_blue_flare,
            move |ctx, handles| {
                assert_eq!(
                    ctx.place_cards_under_tamer_bottom_in_order(handles, tamer, false),
                    2
                );
            },
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("first snapshot rescue pick installs");
    runner
        .game
        .resolve_selection(pending.selecting_player, TRASH_EFFECT_START + 1)
        .expect("choose Xros Heart source");
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("second snapshot rescue pick installs");
    runner
        .game
        .resolve_selection(pending.selecting_player, TRASH_EFFECT_START + 2)
        .expect("choose Blue Flare source");

    assert_eq!(
        source_ids(&runner, tamer),
        vec!["XROS-SOURCE", "BLUE-SOURCE", "TARGET-TAMER"],
        "snapshot handles move under the Tamer in chosen order after carrier removal"
    );
    let trash_ids: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(trash_ids, vec!["DELETED-TOP", "OTHER-SOURCE"]);
}
