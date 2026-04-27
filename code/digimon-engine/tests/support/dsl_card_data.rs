use digimon_dsl::compiled::{CompiledCard, CompiledCardKind, CompiledColor};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, CardKind};

pub fn compiled(card_id: &str) -> CompiledCard {
    digimon_engine::dsl_registry::from_embedded()
        .expect("embedded DSL registry loads")
        .lookup(card_id)
        .unwrap_or_else(|| panic!("{card_id} not found in embedded DSL pack"))
        .clone()
}

pub fn card_data_from_compiled(card_id: &str) -> CardData {
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
                CompiledColor::Red => CardColor::Red,
                CompiledColor::Blue => CardColor::Blue,
                CompiledColor::Yellow => CardColor::Yellow,
                CompiledColor::Green => CardColor::Green,
                CompiledColor::Black => CardColor::Black,
                CompiledColor::Purple => CardColor::Purple,
                CompiledColor::White => CardColor::White,
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
