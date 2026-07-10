use std::collections::BTreeSet;

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use serde::Deserialize;

const MATRIX_JSON: &str = include_str!("../../../../qa/rules/keyword_semantics_matrix.json");
const ENUMS_RS: &str = include_str!("../../src/enums.rs");

#[derive(Debug, Deserialize)]
struct KeywordMatrix {
    rows: Vec<KeywordMatrixRow>,
}

#[derive(Debug, Deserialize)]
struct KeywordMatrixRow {
    keyword_id: String,
    kind: String,
    engine_keyword: bool,
    optional_choice: bool,
    cost_optional: bool,
    effect_mandatory_after_cost: bool,
}

fn matrix() -> KeywordMatrix {
    serde_json::from_str(MATRIX_JSON).expect("keyword semantics matrix JSON must parse")
}

fn engine_keyword_variants() -> BTreeSet<String> {
    let block = ENUMS_RS
        .split("pub enum Keyword {")
        .nth(1)
        .expect("Keyword enum present")
        .split("\n}")
        .next()
        .expect("Keyword enum closes");
    block
        .lines()
        .filter_map(|line| {
            let line = line.split("//").next().unwrap_or("").trim();
            if line.is_empty() || !line.ends_with(',') {
                return None;
            }
            let name = line
                .split(['(', ','])
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if name.chars().next().is_some_and(char::is_uppercase) {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

fn keyword_for_id(keyword_id: &str) -> Keyword {
    match keyword_id {
        "Blocker" => Keyword::Blocker,
        "SecurityAttackPlus" => Keyword::SecurityAttackPlus(1),
        "SecurityAttackMinus" => Keyword::SecurityAttackMinus(1),
        "Rush" => Keyword::Rush,
        "Jamming" => Keyword::Jamming,
        "Piercing" => Keyword::Piercing,
        "Reboot" => Keyword::Reboot,
        "DeDigivolve" => Keyword::DeDigivolve(1),
        "DrawX" => Keyword::DrawX(1),
        "Blitz" => Keyword::Blitz,
        "Raid" => Keyword::Raid,
        "Alliance" => Keyword::Alliance,
        "BlastDigivolve" => Keyword::BlastDigivolve,
        "Save" => Keyword::Save,
        "MaterialSave" => Keyword::MaterialSave(1),
        "DigiBurst" => Keyword::DigiBurst(1),
        "Fortitude" => Keyword::Fortitude,
        "Overclock" => Keyword::Overclock,
        "Barrier" => Keyword::Barrier,
        "Decoy" => Keyword::Decoy(0),
        "Partition" => Keyword::Partition,
        "Vortex" => Keyword::Vortex,
        "Collision" => Keyword::Collision,
        "Evade" => Keyword::Evade,
        "Fragment" => Keyword::Fragment(1),
        "Decode" => Keyword::Decode,
        "ArmorPurge" => Keyword::ArmorPurge,
        "Progress" => Keyword::Progress,
        "Retaliation" => Keyword::Retaliation,
        "Scapegoat" => Keyword::Scapegoat,
        "Execute" => Keyword::Execute,
        "Iceclad" => Keyword::Iceclad,
        "MindLink" => Keyword::MindLink,
        "Training" => Keyword::Training,
        "ArtsDigivolve" => Keyword::ArtsDigivolve,
        "Ascension" => Keyword::Ascension,
        other => panic!("no representative Keyword mapping for matrix row {other}"),
    }
}

fn synthetic_card(card_id: &str, keyword: Keyword) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: vec![keyword],
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

#[test]
fn matrix_has_rows_for_every_engine_keyword_variant() {
    let matrix_ids: BTreeSet<_> = matrix()
        .rows
        .into_iter()
        .filter(|row| row.engine_keyword)
        .map(|row| row.keyword_id)
        .collect();
    let engine_ids = engine_keyword_variants();

    let missing: Vec<_> = engine_ids.difference(&matrix_ids).cloned().collect();
    assert!(
        missing.is_empty(),
        "engine Keyword variants missing semantics matrix rows: {missing:?}"
    );
}

#[test]
fn matrix_optionality_fields_match_kind() {
    for row in matrix().rows {
        match row.kind.as_str() {
            "persistent" | "mandatory" => {
                assert!(!row.optional_choice, "{:?}", row);
                assert!(!row.cost_optional, "{:?}", row);
                assert!(!row.effect_mandatory_after_cost, "{:?}", row);
            }
            "optional" => {
                assert!(row.optional_choice, "{:?}", row);
                assert!(!row.cost_optional, "{:?}", row);
                assert!(!row.effect_mandatory_after_cost, "{:?}", row);
            }
            "optional_cost_then_mandatory" => {
                assert!(row.optional_choice, "{:?}", row);
                assert!(row.cost_optional, "{:?}", row);
                assert!(row.effect_mandatory_after_cost, "{:?}", row);
            }
            other => panic!("unknown matrix kind {other} for {}", row.keyword_id),
        }
    }
}

#[test]
fn synthetic_cards_instantiate_every_engine_keyword_row() {
    for row in matrix().rows.into_iter().filter(|row| row.engine_keyword) {
        let keyword = keyword_for_id(&row.keyword_id);
        let card_id = format!("KW-{}", row.keyword_id);
        let mut runner = DebugRunner::builder()
            .add_card(synthetic_card(&card_id, keyword))
            .start();
        let handle = runner.place_on_field(0, &card_id, Some(0));
        assert!(
            runner.game.has_keyword(handle, keyword),
            "synthetic card for {} did not expose {:?}",
            row.keyword_id,
            keyword
        );
    }
}
