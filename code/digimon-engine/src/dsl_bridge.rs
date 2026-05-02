//! Bridge between engine-side `CardData` and DSL-side `CardDataDb`.
//! Lives in digimon-engine (not digimon-dsl) because it depends on the
//! engine's card_data + enums. digimon-dsl stays engine-agnostic so
//! build.rs can use it without a circular dependency.

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledCard, CompiledCardKind, CompiledColor,
    CompiledCost, CompiledPredicate,
};
use digimon_dsl::errors::DslError;
use digimon_dsl::loader::{CardDataDb, CardDataRow};
use digimon_dsl::spec::{CardKind, ColorSpec};
use std::collections::HashMap;
use std::path::Path as StdPath;

use crate::card_data::{CardData, DnaCost, DnaRequirement};

pub struct RealCardDataAdapter {
    cards: HashMap<String, RealRow>,
}

struct RealRow {
    name: String,
    kind: CardKind,
    level: Option<u8>,
    dp: Option<i32>,
    cost: Option<i32>,
    colors: Vec<ColorSpec>,
}

impl RealCardDataAdapter {
    pub fn from_path(path: &StdPath) -> Result<Self, DslError> {
        let raw = std::fs::read_to_string(path).map_err(|e| DslError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let parsed = crate::card_data::CardData::load_from_str(&raw).map_err(|e| DslError::Io {
            path: path.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e}")),
        })?;
        let mut cards = HashMap::new();
        for (card_id, data) in parsed {
            let kind = engine_card_kind_to_dsl(data.card_kind);
            let level = data.digimon_level();
            let dp = data.digimon_dp();
            let cost = data
                .option_use_cost()
                .map(i32::from)
                .or(Some(data.play_cost as i32));
            let colors = data
                .digimon_colors()
                .iter()
                .map(|c| engine_color_to_dsl(*c))
                .collect();
            cards.insert(
                card_id,
                RealRow {
                    name: data.card_name,
                    kind,
                    level,
                    dp,
                    cost,
                    colors,
                },
            );
        }
        Ok(Self { cards })
    }
}

fn engine_card_kind_to_dsl(k: crate::enums::CardKind) -> CardKind {
    use crate::enums::CardKind as E;
    match k {
        E::Digimon => CardKind::Digimon,
        E::Tamer => CardKind::Tamer,
        E::Option => CardKind::Option,
        E::DigiEgg => CardKind::DigiEgg,
        E::Token => CardKind::Token,
        E::Dual => CardKind::Dual,
    }
}

fn engine_color_to_dsl(c: crate::enums::CardColor) -> ColorSpec {
    use crate::enums::CardColor as E;
    match c {
        E::Red => ColorSpec::Red,
        E::Blue => ColorSpec::Blue,
        E::Yellow => ColorSpec::Yellow,
        E::Green => ColorSpec::Green,
        E::Black => ColorSpec::Black,
        E::Purple => ColorSpec::Purple,
        E::White => ColorSpec::White,
    }
}

impl CardDataDb for RealCardDataAdapter {
    fn lookup(&self, card_id: &str) -> Option<CardDataRow<'_>> {
        self.cards.get(card_id).map(|r| CardDataRow {
            name: &r.name,
            kind: r.kind,
            level: r.level,
            dp: r.dp,
            cost: r.cost,
            colors: &r.colors,
        })
    }
}

/// Thread compiled DSL DNA alt-path metadata into engine `CardData`.
///
/// The engine already decides whether a hand card can DNA digivolve from
/// `CardData::dna_costs`; this adapter lets DSL-authored `alt_paths:
/// kind: dna_digivolve` populate that same runtime surface without adding a
/// second DNA metadata model.
pub fn enrich_card_data_with_dsl_alt_paths(
    cards: &mut HashMap<String, CardData>,
    registry: &digimon_dsl::CardRegistry,
) -> usize {
    let mut inserted = 0;
    for (card_id, compiled) in registry.iter() {
        let Some(card) = cards.get_mut(card_id) else {
            continue;
        };
        card.ace_overflow = compiled.ace_overflow;
        card.digixros_aliases = compiled.digixros_aliases.clone();
        for dna_cost in compiled_dna_costs(compiled) {
            if !card.dna_costs.contains(&dna_cost) {
                card.dna_costs.push(dna_cost);
                inserted += 1;
            }
        }
    }
    inserted
}

fn compiled_dna_costs(card: &CompiledCard) -> impl Iterator<Item = DnaCost> + '_ {
    card.alt_paths
        .iter()
        .filter_map(|path| compiled_dna_cost(path))
}

fn compiled_dna_cost(path: &CompiledAltPath) -> Option<DnaCost> {
    if !matches!(path.kind, CompiledAltPathKind::DnaDigivolve) {
        return None;
    }
    if path.materials.len() != 2 {
        return None;
    }
    let memory_cost = match &path.cost {
        Some(CompiledCost::Literal(n)) => i16::try_from(*n).ok()?,
        None => 0,
        Some(CompiledCost::Formula(_)) => return None,
    };

    Some(DnaCost {
        requirement1: compiled_dna_requirement(&path.materials[0].filter)?,
        requirement2: compiled_dna_requirement(&path.materials[1].filter)?,
        memory_cost,
    })
}

fn compiled_dna_requirement(pred: &CompiledPredicate) -> Option<DnaRequirement> {
    if let Some(kind) = pred.kind {
        if !matches!(kind, CompiledCardKind::Digimon) {
            return None;
        }
    }

    let mut supported = CompiledPredicate::default();
    supported.kind = pred.kind;
    supported.level_eq = pred.level_eq;
    supported.color_is = pred.color_is;
    supported.color_only = pred.color_only.clone();
    supported.name_is = pred.name_is.clone();
    supported.name_contains = pred.name_contains.clone();
    if &supported != pred {
        return None;
    }

    let card_colors = pred
        .color_only
        .clone()
        .unwrap_or_else(|| pred.color_is.into_iter().collect())
        .into_iter()
        .map(compiled_color_to_engine)
        .collect();

    Some(DnaRequirement {
        level: pred.level_eq.unwrap_or(0),
        card_colors,
        name_contains: pred
            .name_contains
            .clone()
            .or_else(|| pred.name_is.clone())
            .unwrap_or_default(),
        text_contains: String::new(),
    })
}

fn compiled_color_to_engine(color: CompiledColor) -> crate::enums::CardColor {
    match color {
        CompiledColor::Red => crate::enums::CardColor::Red,
        CompiledColor::Blue => crate::enums::CardColor::Blue,
        CompiledColor::Yellow => crate::enums::CardColor::Yellow,
        CompiledColor::Green => crate::enums::CardColor::Green,
        CompiledColor::Black => crate::enums::CardColor::Black,
        CompiledColor::Purple => crate::enums::CardColor::Purple,
        CompiledColor::White => crate::enums::CardColor::White,
    }
}
