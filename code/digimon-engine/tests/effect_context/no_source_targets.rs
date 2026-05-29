use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, StackPosition};
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

#[test]
fn select_opponent_no_source_digimon_filters_and_returns_to_deck() {
    let mut runner = DebugRunner::builder()
        .add_card(card("EFFECT-SOURCE"))
        .add_card(card("NO-SOURCE"))
        .add_card(card("HAS-SOURCE"))
        .add_card(card("STACKED"))
        .memory(5)
        .start();

    let effect_source = runner.place_on_field(0, "EFFECT-SOURCE", None);
    let no_source = runner.place_on_field(1, "NO-SOURCE", None);
    let has_source = runner.place_on_field(1, "HAS-SOURCE", None);
    insert_source_below_top(&mut runner, has_source, "STACKED");
    let source_card = runner.top_card(effect_source);

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(effect_source), 0);
        ctx.select_opponent_no_source_digimon(
            "Choose an opponent Digimon with no sources",
            false,
            move |ctx, target| {
                assert!(ctx.return_to_deck(target, StackPosition::Bottom));
            },
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("no-source opponent target selection installs");
    let no_source_action = encode_attack(0, no_source.index as u16);
    let has_source_action = encode_attack(0, has_source.index as u16);
    assert_eq!(pending.valid_action_ids, vec![no_source_action]);
    assert!(!pending.valid_action_ids.contains(&has_source_action));

    runner
        .game
        .resolve_selection(pending.selecting_player, no_source_action)
        .expect("resolve no-source target selection");

    assert_eq!(runner.game.players[1].battle_area.len(), 1);
    assert_eq!(
        runner.game.players[1].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "HAS-SOURCE"
    );
    assert_eq!(
        runner.game.players[1]
            .deck
            .first()
            .expect("returned card at bottom of deck")
            .card_id(&runner.game.card_data),
        "NO-SOURCE"
    );
}
