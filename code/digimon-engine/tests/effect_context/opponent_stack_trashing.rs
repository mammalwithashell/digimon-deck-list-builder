use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

fn card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
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

fn insert_source_below_top(runner: &mut DebugRunner, host: PermanentHandle, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|data| data.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown test card {card_id}"));
    let card = CardSource::new(data_idx, host.player, runner.game.next_card_index());
    let sources = &mut runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources;
    let insert_at = sources.len().saturating_sub(1);
    sources.insert(insert_at, card);
}

fn source_ids(runner: &DebugRunner, host: PermanentHandle) -> Vec<String> {
    runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn source_count(game: &digimon_engine::game::Game, handle: PermanentHandle) -> bool {
    game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .len()
        > 1
}

#[test]
fn selected_opponent_digimon_trashes_top_n_stacked_sources_below_top() {
    let mut runner = DebugRunner::builder()
        .add_card(card("EFFECT-SOURCE"))
        .add_card(card("OPP-HOST"))
        .add_card(card("SRC-A"))
        .add_card(card("SRC-B"))
        .add_card(card("SRC-C"))
        .memory(5)
        .start();

    let effect_source = runner.place_on_field(0, "EFFECT-SOURCE", None);
    let opponent_host = runner.place_on_field(1, "OPP-HOST", None);
    insert_source_below_top(&mut runner, opponent_host, "SRC-A");
    insert_source_below_top(&mut runner, opponent_host, "SRC-B");
    insert_source_below_top(&mut runner, opponent_host, "SRC-C");
    let source_card: CardHandle = runner.top_card(effect_source);
    assert_eq!(
        source_ids(&runner, opponent_host),
        vec!["SRC-A", "SRC-B", "SRC-C", "OPP-HOST"]
    );

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(effect_source), 0);
        ctx.select_opponent_permanent(
            "Choose an opponent Digimon to trash top stacked cards",
            false,
            source_count,
            move |ctx, target| {
                assert_eq!(ctx.trash_top_n_stacked_sources(target, 2), 2);
            },
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("opponent stack trash target selection installs");
    let action = encode_attack(0, opponent_host.index as u16);
    assert_eq!(pending.valid_action_ids, vec![action]);

    runner
        .game
        .resolve_selection(pending.selecting_player, action)
        .expect("resolve opponent stack trash target");

    assert_eq!(
        source_ids(&runner, opponent_host),
        vec!["SRC-A", "OPP-HOST"],
        "top two stacked source cards below the visible top are trashed"
    );
    let opp_trash_ids: Vec<_> = runner.game.players[1]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(opp_trash_ids, vec!["SRC-C", "SRC-B"]);
}
