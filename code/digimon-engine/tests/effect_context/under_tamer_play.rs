use std::sync::Arc;

use digimon_engine::action::space::encode_source_select;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::{EffectContext, SourceSelectionRef};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

fn card(id: &str, kind: CardKind) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: kind,
        level: matches!(kind, CardKind::Digimon).then_some(4),
        dp: matches!(kind, CardKind::Digimon).then_some(4000),
        play_cost: 6,
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

fn insert_bottom_source(
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
    runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources
        .insert(0, card);
    handle
}

fn is_playable_source(game: &digimon_engine::game::Game, source_ref: SourceSelectionRef) -> bool {
    game.card_data_for_handle(source_ref.card)
        .is_some_and(|data| data.card_id == "PLAY-ME")
}

struct OnPlayGainMemory;

impl CardEffect for OnPlayGainMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Gain 1 memory")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn play_under_tamer_source_without_cost_routes_through_on_play() {
    let mut runner = DebugRunner::builder()
        .add_card(card("SOURCE-TAMER", CardKind::Tamer))
        .add_card(card("PLAY-ME", CardKind::Digimon))
        .memory(3)
        .start();
    runner.register_effect("PLAY-ME", Arc::new(OnPlayGainMemory));

    let tamer = runner.place_on_field(0, "SOURCE-TAMER", None);
    let play_me = insert_bottom_source(&mut runner, tamer, "PLAY-ME");
    let source_card = runner.game.players[0].battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let memory_before = runner.game.memory;

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(tamer), 0);
        ctx.select_own_tamer_sources(
            "Choose a card under a Tamer to play without paying the cost",
            1,
            1,
            is_playable_source,
            move |ctx, refs| {
                assert_eq!(refs.len(), 1);
                let played = ctx
                    .play_under_tamer_source_without_cost(refs[0])
                    .expect("source under own Tamer should play for free");
                assert_eq!(played.player, 0);
            },
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("under-Tamer source play installs source selection");
    let source_action = encode_source_select(tamer.index as u16, 0).unwrap();
    assert_eq!(pending.valid_action_ids, vec![source_action]);

    runner
        .game
        .resolve_selection(pending.selecting_player, source_action)
        .expect("resolve under-Tamer source play selection");

    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "free play pays no printed cost, but OnPlay gain-memory trigger fires"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        1,
        "played source is removed from under the Tamer"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().handle() == play_me),
        "the selected source becomes a new permanent"
    );
}

#[test]
fn play_under_tamer_source_rejects_non_tamer_origin() {
    let mut runner = DebugRunner::builder()
        .add_card(card("DIGI-HOST", CardKind::Digimon))
        .add_card(card("PLAY-ME", CardKind::Digimon))
        .memory(3)
        .start();

    let host = runner.place_on_field(0, "DIGI-HOST", None);
    let play_me = insert_bottom_source(&mut runner, host, "PLAY-ME");
    let source_card = runner.game.players[0].battle_area[host.index as usize]
        .top_card()
        .handle();
    let source_ref = SourceSelectionRef {
        permanent: host,
        field_index: host.index,
        source_index: 0,
        card: play_me,
    };

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(host), 0);
        ctx.play_under_tamer_source_without_cost(source_ref)
    };

    assert_eq!(result, None);
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        2,
        "non-Tamer source stack is left untouched"
    );
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
}

#[test]
fn play_under_tamer_source_with_cost_reduction_pays_reduced_cost() {
    let mut runner = DebugRunner::builder()
        .add_card(card("SOURCE-TAMER", CardKind::Tamer))
        .add_card(card("PLAY-ME", CardKind::Digimon))
        .memory(5)
        .start();

    let tamer = runner.place_on_field(0, "SOURCE-TAMER", None);
    let play_me = insert_bottom_source(&mut runner, tamer, "PLAY-ME");
    let source_card = runner.game.players[0].battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let source_ref = SourceSelectionRef {
        permanent: tamer,
        field_index: tamer.index,
        source_index: 0,
        card: play_me,
    };

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(tamer), 0);
        ctx.play_under_tamer_source_with_cost_reduction(source_ref, 4)
    };

    assert!(result.is_some());
    assert_eq!(
        runner.game.memory, 3,
        "printed cost 6 reduced by 4 should pay exactly 2 memory"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        1,
        "paid play consumes the selected source"
    );
}

#[test]
fn play_under_tamer_source_with_cost_reduction_rolls_back_when_payment_fails() {
    let mut runner = DebugRunner::builder()
        .add_card(card("SOURCE-TAMER", CardKind::Tamer))
        .add_card(card("PLAY-ME", CardKind::Digimon))
        .memory(-9)
        .start();

    let tamer = runner.place_on_field(0, "SOURCE-TAMER", None);
    let play_me = insert_bottom_source(&mut runner, tamer, "PLAY-ME");
    let source_card = runner.game.players[0].battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let source_ref = SourceSelectionRef {
        permanent: tamer,
        field_index: tamer.index,
        source_index: 0,
        card: play_me,
    };

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(tamer), 0);
        ctx.play_under_tamer_source_with_cost_reduction(source_ref, 4)
    };

    assert_eq!(result, None);
    assert_eq!(
        runner.game.memory, -9,
        "failed payment leaves memory unchanged"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].handle(),
        play_me,
        "failed payment restores the source at its original under-Tamer index"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "failed payment does not create a new permanent"
    );
}
