//! Bridge between engine-side `CardData` and DSL-side `CardDataDb`.
//! Lives in digimon-engine (not digimon-dsl) because it depends on the
//! engine's card_data + enums. digimon-dsl stays engine-agnostic so
//! build.rs can use it without a circular dependency.

use digimon_dsl::loader::{CardDataDb, CardDataRow};
use digimon_dsl::spec::{CardKind, ColorSpec};
use digimon_dsl::errors::DslError;
use std::path::Path as StdPath;
use std::collections::HashMap;

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
            cards.insert(card_id, RealRow {
                name: data.card_name,
                kind: engine_card_kind_to_dsl(data.card_kind),
                level: data.level,
                dp: data.dp,
                cost: Some(data.play_cost as i32),
                colors: data.colors.iter().map(|c| engine_color_to_dsl(*c)).collect(),
            });
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
