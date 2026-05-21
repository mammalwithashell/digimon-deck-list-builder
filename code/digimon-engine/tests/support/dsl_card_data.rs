use digimon_dsl::compiled::{CompiledCard, CompiledCardKind, CompiledColor};
use digimon_engine::card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace};
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
            CompiledCardKind::Dual => CardKind::Dual,
        },
        level: card.level,
        dp: card.dp,
        play_cost: card.cost.unwrap_or(0) as u16,
        colors: card.color.iter().copied().map(compiled_color).collect(),
        traits: card.traits,
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: card.dual.as_ref().map(|dual| DualCardData {
            digimon: DualDigimonFace {
                level: dual.digimon.level,
                dp: dual.digimon.dp,
                colors: dual
                    .digimon
                    .colors
                    .iter()
                    .copied()
                    .map(compiled_color)
                    .collect(),
                traits: dual.digimon.traits.clone(),
                evo_costs: Vec::new(),
                effect_text: dual.digimon.effect_text.clone(),
                inherited_text: dual.digimon.inherited_text.clone(),
                keywords: Vec::new(),
            },
            option: DualOptionFace {
                use_cost: dual.option.use_cost,
                colors: dual
                    .option
                    .colors
                    .iter()
                    .copied()
                    .map(compiled_color)
                    .collect(),
                effect_text: dual.option.effect_text.clone(),
                security_text: dual.option.security_text.clone(),
                keywords: Vec::new(),
            },
        }),
        ace_overflow: card.ace_overflow,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        digixros_aliases: card.digixros_aliases,
        also_treated_as: card.also_treated_as,
    }
}

fn compiled_color(color: CompiledColor) -> CardColor {
    match color {
        CompiledColor::Red => CardColor::Red,
        CompiledColor::Blue => CardColor::Blue,
        CompiledColor::Yellow => CardColor::Yellow,
        CompiledColor::Green => CardColor::Green,
        CompiledColor::Black => CardColor::Black,
        CompiledColor::Purple => CardColor::Purple,
        CompiledColor::White => CardColor::White,
    }
}
