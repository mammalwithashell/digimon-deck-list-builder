use std::sync::{Arc, Mutex};

use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};

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

#[test]
fn place_selected_hand_card_under_source_tamer_moves_exact_card() {
    let mut runner = DebugRunner::builder()
        .add_card(card("SOURCE-TAMER", CardKind::Tamer))
        .add_card(card("MATCH-HAND", CardKind::Digimon))
        .add_card(card("OTHER-HAND", CardKind::Digimon))
        .memory(5)
        .start();

    let tamer = runner.place_on_field(0, "SOURCE-TAMER", None);
    let match_handle = push_to_hand(&mut runner, 0, "MATCH-HAND");
    let _other_handle = push_to_hand(&mut runner, 0, "OTHER-HAND");
    let source_card = runner.game.players[0].battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let moved = Arc::new(Mutex::new(None::<CardHandle>));

    {
        let moved = Arc::clone(&moved);
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(tamer), 0);
        ctx.select_hand(
            0,
            "Choose a matching card in hand to place under this Tamer",
            false,
            |game, index| game.players[0].hand[index].handle() == match_handle,
            move |ctx, hand_index| {
                *moved.lock().unwrap() = ctx.place_hand_card_under_source_tamer(hand_index, false);
            },
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("matching hand card installs a prompt");
    assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START]);

    runner
        .game
        .resolve_selection(pending.selecting_player, PLAY_HAND_START)
        .expect("resolve hand placement selection");

    assert_eq!(*moved.lock().unwrap(), Some(match_handle));
    assert_eq!(runner.game.players[0].hand.len(), 1);
    let tamer_perm = &runner.game.players[0].battle_area[tamer.index as usize];
    assert_eq!(tamer_perm.card_sources[0].handle(), match_handle);
    assert_eq!(tamer_perm.top_card().handle(), source_card);
}

#[test]
fn place_selected_hand_card_under_chosen_tamer_rejects_non_tamer_target() {
    let mut runner = DebugRunner::builder()
        .add_card(card("SOURCE-TAMER", CardKind::Tamer))
        .add_card(card("DEST-TAMER", CardKind::Tamer))
        .add_card(card("DEST-DIGIMON", CardKind::Digimon))
        .add_card(card("MATCH-HAND", CardKind::Digimon))
        .memory(5)
        .start();

    let source_tamer = runner.place_on_field(0, "SOURCE-TAMER", None);
    let dest_tamer = runner.place_on_field(0, "DEST-TAMER", None);
    let dest_digimon = runner.place_on_field(0, "DEST-DIGIMON", None);
    let match_handle = push_to_hand(&mut runner, 0, "MATCH-HAND");
    let source_card = runner.game.players[0].battle_area[source_tamer.index as usize]
        .top_card()
        .handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source_tamer), 0);
        assert_eq!(
            ctx.place_hand_card_under_tamer(0, dest_digimon, false),
            None,
            "non-Tamer destinations are rejected without moving the hand card"
        );
        assert_eq!(
            ctx.place_hand_card_under_tamer(0, dest_tamer, false),
            Some(match_handle)
        );
    }

    assert_eq!(runner.game.players[0].hand.len(), 0);
    assert_eq!(
        runner.game.players[0].battle_area[dest_tamer.index as usize].card_sources[0].handle(),
        match_handle
    );
    assert_eq!(
        runner.game.players[0].battle_area[dest_digimon.index as usize]
            .card_sources
            .len(),
        1,
        "rejected non-Tamer destination should remain unchanged"
    );
}
