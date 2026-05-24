use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCostDelta, CompiledPlayerRef, CompiledPredicate, CompiledStep,
    CompiledZone,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};

fn evo_lv4(card_id: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: vec![EvoCost {
            card_color: 0,
            level: 3,
            memory_cost: 0,
        }],
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn selected_union_zone_card_can_feed_effect_digivolve_from_card_handle() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(evo_lv4("EVO4"))
        .hand(0, &["EVO4"])
        .memory(5)
        .start();

    let target = runner.place_on_field(0, "BASE3", Some(0));
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let steps = vec![
        CompiledStep::SelectUnionZone {
            of: CompiledPlayerRef::You,
            zones: vec![CompiledZone::Hand, CompiledZone::Trash],
            material_of: None,
            filter: CompiledPredicate::default(),
            bind_as: Some("evo".to_string()),
            prompt: "Pick evolution card".to_string(),
            prompt_key: None,
            optional: false,
            cost: false,
            then: vec![],
        },
        CompiledStep::EffectInitiatedDigivolve {
            target: CompiledBindingRef::Permanent("base".to_string()),
            from_hand: CompiledBindingRef::Binding("evo".to_string()),
            cost: CompiledCostDelta::Free,
            ignore_requirements: false,
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        let mut bindings = Bindings::new();
        bindings.insert_permanent("base", target);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("union selection should park");
    let action = pending.valid_action_ids[0];
    let selecting_player = pending.selecting_player;
    runner
        .game
        .resolve_selection(selecting_player, action)
        .expect("resolve union pick");

    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "EVO4"
    );
    assert!(runner.game.players[0].hand.is_empty());
}
