use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::UnionZoneSet;

fn card(id: &str, kind: CardKind) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: kind,
        level: matches!(kind, CardKind::Digimon).then_some(4),
        dp: matches!(kind, CardKind::Digimon).then_some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
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

#[test]
fn place_union_hand_or_trash_card_under_source_tamer_uses_origin() {
    let mut runner = DebugRunner::builder()
        .add_card(card("SOURCE-TAMER", CardKind::Tamer))
        .add_card(card("MATCH-HAND", CardKind::Digimon))
        .add_card(card("MATCH-TRASH", CardKind::Digimon))
        .memory(5)
        .start();

    let tamer = runner.place_on_field(0, "SOURCE-TAMER", None);
    let hand_handle = push_to_hand(&mut runner, 0, "MATCH-HAND");
    let trash_handle = push_to_trash(&mut runner, 0, "MATCH-TRASH");
    let source_card = runner.game.players[0].battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let moved = Arc::new(Mutex::new(None::<CardHandle>));

    {
        let moved = Arc::clone(&moved);
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(tamer), 0);
        ctx.select_union_zone(
            0,
            UnionZoneSet::HAND | UnionZoneSet::TRASH,
            None,
            "Choose a matching card from hand or trash to place under this Tamer",
            false,
            move |_game, card| card.handle() == hand_handle || card.handle() == trash_handle,
            move |ctx, card, origin| {
                *moved.lock().unwrap() =
                    ctx.place_union_card_under_source_tamer(card, origin, false);
            },
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("union hand-or-trash stash selection installs a prompt");
    assert_eq!(
        pending.valid_action_ids,
        vec![PLAY_HAND_START, TRASH_EFFECT_START]
    );

    runner
        .game
        .resolve_selection(pending.selecting_player, TRASH_EFFECT_START)
        .expect("resolve trash-origin union placement");

    assert_eq!(*moved.lock().unwrap(), Some(trash_handle));
    assert_eq!(runner.game.players[0].hand.len(), 1);
    assert_eq!(runner.game.players[0].trash.len(), 0);
    let tamer_perm = &runner.game.players[0].battle_area[tamer.index as usize];
    assert_eq!(tamer_perm.card_sources[0].handle(), trash_handle);
    assert_eq!(tamer_perm.top_card().handle(), source_card);
}

#[test]
fn place_union_card_under_chosen_tamer_handles_hand_origin() {
    let mut runner = DebugRunner::builder()
        .add_card(card("SOURCE-TAMER", CardKind::Tamer))
        .add_card(card("DEST-TAMER", CardKind::Tamer))
        .add_card(card("MATCH-HAND", CardKind::Digimon))
        .memory(5)
        .start();

    let source_tamer = runner.place_on_field(0, "SOURCE-TAMER", None);
    let dest_tamer = runner.place_on_field(0, "DEST-TAMER", None);
    let hand_handle = push_to_hand(&mut runner, 0, "MATCH-HAND");
    let source_card = runner.game.players[0].battle_area[source_tamer.index as usize]
        .top_card()
        .handle();
    let moved = Arc::new(Mutex::new(None::<CardHandle>));

    {
        let moved = Arc::clone(&moved);
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source_tamer), 0);
        ctx.select_union_zone(
            0,
            UnionZoneSet::HAND | UnionZoneSet::TRASH,
            None,
            "Choose a matching card from hand or trash to place under a Tamer",
            false,
            move |_game, card| card.handle() == hand_handle,
            move |ctx, card, origin| {
                *moved.lock().unwrap() =
                    ctx.place_union_card_under_tamer(card, origin, dest_tamer, false);
            },
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("hand-origin union stash selection installs a prompt");
    assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START]);

    runner
        .game
        .resolve_selection(pending.selecting_player, PLAY_HAND_START)
        .expect("resolve hand-origin union placement");

    assert_eq!(*moved.lock().unwrap(), Some(hand_handle));
    assert_eq!(runner.game.players[0].hand.len(), 0);
    assert_eq!(
        runner.game.players[0].battle_area[dest_tamer.index as usize].card_sources[0].handle(),
        hand_handle
    );
}
