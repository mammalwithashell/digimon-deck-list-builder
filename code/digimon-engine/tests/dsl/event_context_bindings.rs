use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::{Permanent, PermanentHandle};

fn digimon_card(id: &str, name: &str, traits: &[&str], dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(dp),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: traits.iter().map(|t| t.to_string()).collect(),
        evo_costs: Vec::<EvoCost>::new(),
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

#[test]
fn source_trashed_event_exposes_trashed_source_card_to_followup_predicate() {
    let yaml = r#"
card: DSL-SOURCE-TRASH-BINDING
name: Source Trash Binding
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_digivolution_card_trashed
    condition:
      all_of:
        - event_target_owner: you
        - host_permanent_trait_has: Rock
        - trashed_source_trait_has: Mineral
        - trashed_source_card_id_is: UNDER-MINERAL
    process:
      - gain_memory: 4
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon_card("BASE", "Base", &[], 1000))
        .add_card(digimon_card(
            "UNDER-MINERAL",
            "Under Mineral",
            &["Mineral"],
            1000,
        ))
        .add_card(digimon_card("HOST-ROCK", "Host Rock", &["Rock"], 4000))
        .add_card(digimon_card("EFFECT", "Effect", &[], 2000))
        .memory(0)
        .build();

    runner.place_on_field(0, "DSL-SOURCE-TRASH-BINDING", None);
    let (target, trashed_source) = {
        let game = runner.game_mut();
        let turn = game.turn_count;
        let base_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BASE")
            .expect("BASE registered");
        let under_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER-MINERAL")
            .expect("UNDER-MINERAL registered");
        let host_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "HOST-ROCK")
            .expect("HOST-ROCK registered");
        let base = CardSource::new(base_idx, 0, game.next_card_index());
        let under = CardSource::new(under_idx, 0, game.next_card_index());
        let host = CardSource::new(host_idx, 0, game.next_card_index());
        let trashed_source = under.handle();
        let mut permanent = Permanent::new(base, turn);
        permanent.card_sources.push(under);
        permanent.card_sources.push(host);
        game.players[0].battle_area.push(permanent);
        (
            PermanentHandle {
                player: 0,
                index: (game.players[0].battle_area.len() - 1) as u8,
            },
            trashed_source,
        )
    };

    let effect_perm = runner.place_on_field(0, "EFFECT", None);
    let effect_card = runner.top_card(effect_perm);
    {
        let mut ctx = EffectContext::new(&mut runner.game, effect_card, Some(effect_perm), 0);
        ctx.trash_card_source(target, trashed_source);
    }

    assert_eq!(
        runner.memory(),
        4,
        "source-trash DSL predicates must read the exact trashed source card from event context"
    );
}
