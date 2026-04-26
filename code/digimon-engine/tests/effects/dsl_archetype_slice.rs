//! Mechanic tests for the first real DSL archetype slice.

use digimon_dsl::compiled::{CompiledCard, CompiledCardKind};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};

fn compiled(card_id: &str) -> CompiledCard {
    digimon_engine::dsl_registry::from_embedded()
        .expect("embedded DSL registry loads")
        .lookup(card_id)
        .unwrap_or_else(|| panic!("{card_id} present in embedded DSL pack"))
        .clone()
}

fn card_data_from_compiled(card_id: &str) -> CardData {
    let card = compiled(card_id);
    CardData {
        card_id: card.card.clone(),
        card_name: card.name,
        card_kind: match card.kind {
            CompiledCardKind::Digimon => CardKind::Digimon,
            CompiledCardKind::Tamer => CardKind::Tamer,
            CompiledCardKind::Option => CardKind::Option,
            CompiledCardKind::DigiEgg => CardKind::DigiEgg,
            CompiledCardKind::Token => CardKind::Token,
        },
        level: card.level,
        dp: card.dp,
        play_cost: card.cost.unwrap_or(0) as u16,
        colors: card
            .color
            .iter()
            .map(|c| match c {
                digimon_dsl::compiled::CompiledColor::Red => CardColor::Red,
                digimon_dsl::compiled::CompiledColor::Blue => CardColor::Blue,
                digimon_dsl::compiled::CompiledColor::Yellow => CardColor::Yellow,
                digimon_dsl::compiled::CompiledColor::Green => CardColor::Green,
                digimon_dsl::compiled::CompiledColor::Black => CardColor::Black,
                digimon_dsl::compiled::CompiledColor::Purple => CardColor::Purple,
                digimon_dsl::compiled::CompiledColor::White => CardColor::White,
            })
            .collect(),
        traits: card.traits,
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn nokia_aura_grants_plus_1000_dp_to_own_greymon_name() {
    let mut runner = DebugRunner::builder()
        .add_card(card_data_from_compiled("BT22-084"))
        .add_card(card_data_from_compiled("BT17-015"))
        .build();

    let nokia = runner.place_on_field(0, "BT22-084", Some(0));
    let wargreymon = runner.place_on_field(0, "BT17-015", Some(0));
    let base_dp = runner
        .effective_dp(wargreymon)
        .expect("WarGreymon has printed DP");

    let source_card = runner.game.players[0].battle_area[nokia.index as usize]
        .top_card()
        .handle();
    let aura = runner
        .game
        .effects_for_card("BT22-084", source_card)
        .expect("Nokia DSL effects are registered")
        .into_iter()
        .find(|effect| effect.timing == EffectTiming::Declarative)
        .expect("Nokia has a declarative aura");
    let process = aura.process.as_ref().expect("aura has process");

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(nokia), 0);
        process(&mut ctx);
    }

    assert_eq!(
        runner.effective_dp(wargreymon),
        Some(base_dp + 1000),
        "Nokia's DSL aura should buff the Greymon-name permanent"
    );
}
