//! First offline slice: native inventory, explicitly not reference resolution.
use digimon_dsl::CardSpec;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize)]
pub struct FileInput {
    pub path: String,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Serialize)]
pub struct FileEvidence {
    pub input: FileInput,
    pub card_id: Option<String>,
    pub parsed: Option<serde_json::Value>,
    pub compiled: Option<serde_json::Value>,
    pub error: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct CardInventory {
    pub card_id: String,
    pub status: String,
    pub file_indices: Vec<usize>,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub schema_version: u32,
    pub status: String,
    pub reference_visitor_complete: bool,
    pub requested_ids: Vec<String>,
    pub files: Vec<FileEvidence>,
    pub cards: Vec<CardInventory>,
    pub blockers: Vec<String>,
}
fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':'])
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && (path.ends_with(".yaml") || path.ends_with(".yml"))
}
pub fn build_inventory(
    requested_ids: Vec<String>,
    mut inputs: Vec<FileInput>,
) -> Result<Inventory, String> {
    let selected: BTreeSet<_> = requested_ids.iter().cloned().collect();
    if requested_ids.is_empty()
        || selected.len() != requested_ids.len()
        || requested_ids
            .iter()
            .any(|id| id.is_empty() || id.chars().any(char::is_whitespace))
    {
        return Err("nonempty unique explicit card IDs required".into());
    }
    let mut paths = BTreeSet::new();
    for input in &inputs {
        if !safe_path(&input.path) || !paths.insert(input.path.clone()) {
            return Err("noncanonical or duplicate YAML input path".into());
        }
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let mut files = Vec::new();
    let mut by_id: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut blockers = BTreeSet::from([
        "REFERENCE_VISITOR_NOT_IMPLEMENTED".to_string(),
        "SOURCE_BACKED_ABSENCE_OR_VANILLA_WITNESSES_NOT_VALIDATED".to_string(),
    ]);
    for input in inputs {
        let mut row = FileEvidence {
            input,
            card_id: None,
            parsed: None,
            compiled: None,
            error: None,
        };
        match serde_yml::from_slice::<CardSpec>(&row.input.bytes) {
            Err(error) => {
                row.error = Some(error.to_string());
                blockers.insert("NATIVE_PARSE_FAILURE".into());
            }
            Ok(spec) => {
                row.card_id = Some(spec.card.clone());
                by_id
                    .entry(spec.card.clone())
                    .or_default()
                    .push(files.len());
                row.parsed = Some(serde_json::to_value(&spec).map_err(|e| e.to_string())?);
                if selected.contains(&spec.card) {
                    match digimon_dsl::compile::compile(&spec) {
                        Ok(compiled) => {
                            row.compiled =
                                Some(serde_json::to_value(compiled).map_err(|e| e.to_string())?)
                        }
                        Err(errors) => {
                            row.error = Some(format!("{errors:?}"));
                            blockers.insert("NATIVE_COMPILE_FAILURE".into());
                        }
                    }
                }
            }
        }
        files.push(row);
    }
    if by_id.values().any(|indices| indices.len() > 1) {
        blockers.insert("DUPLICATE_CARD_DEFINITION".into());
    }
    let cards = requested_ids
        .iter()
        .map(|id| {
            let indices = by_id.get(id).cloned().unwrap_or_default();
            let status = if indices.is_empty() {
                blockers.insert("MISSING_DEFINITION".into());
                "MISSING_DEFINITION"
            } else if indices.len() > 1 {
                "DUPLICATE_DEFINITION"
            } else if files[indices[0]].compiled.is_none() {
                "COMPILE_FAILED"
            } else {
                "COMPILED_REFERENCE_UNRESOLVED"
            };
            CardInventory {
                card_id: id.clone(),
                status: status.into(),
                file_indices: indices,
            }
        })
        .collect();
    Ok(Inventory {
        schema_version: 1,
        status: "OFFLINE_NATIVE_INVENTORY_NOT_REFERENCE_CLOSURE".into(),
        reference_visitor_complete: false,
        requested_ids,
        files,
        cards,
        blockers: blockers.into_iter().collect(),
    })
}

// Opt-in evidence wrapper. The original function and v1 wire above are unchanged.
#[path = "references.rs"]
pub mod references;
#[derive(Debug, Serialize)]
pub struct ReferenceFile {
    pub file_index: usize,
    pub owner_id: String,
    pub traversal_complete: bool,
    pub authored: references::Pass,
    pub compiled: references::Pass,
}
#[derive(Debug, Serialize)]
pub struct ReferenceGraph {
    pub schema_version: u32,
    pub scope: String,
    pub source_authority: bool,
    pub runtime_admission: bool,
    pub traversal_complete: bool,
    pub reference_closure_complete: bool,
    pub files: Vec<ReferenceFile>,
    pub blockers: BTreeSet<String>,
}
#[derive(Debug, Serialize)]
pub struct ReferenceInventory {
    pub schema_version: u32,
    pub inventory: Inventory,
    pub reference_graph: ReferenceGraph,
}
pub fn build_inventory_with_references(
    ids: Vec<String>,
    inputs: Vec<FileInput>,
) -> Result<ReferenceInventory, String> {
    let inventory = build_inventory(ids, inputs)?;
    let mut files = vec![];
    let mut blockers = BTreeSet::from([
        "ENGINE_DEPENDENCY_WITNESSES_NOT_VALIDATED".into(),
        "SOURCE_BACKED_ABSENCE_OR_VANILLA_WITNESSES_NOT_VALIDATED".into(),
    ]);
    let mut complete = true;
    for card in &inventory.cards {
        if card.status != "COMPILED_REFERENCE_UNRESOLVED" {
            complete = false;
            blockers.insert(card.status.clone());
            continue;
        }
        let i = card.file_indices[0];
        // Reparse immutable captured bytes, never reacquire disk or trust serialized caller flags.
        let spec: CardSpec =
            serde_yml::from_slice(&inventory.files[i].input.bytes).map_err(|e| e.to_string())?;
        let compiled = digimon_dsl::compile::compile(&spec).map_err(|e| format!("{e:?}"))?;
        let (a, b) = references::visit(&spec, &compiled);
        let done = a.traversal_complete && b.traversal_complete;
        complete &= done;
        blockers.extend(a.blockers.clone());
        blockers.extend(b.blockers.clone());
        files.push(ReferenceFile {
            file_index: i,
            owner_id: card.card_id.clone(),
            traversal_complete: done,
            authored: a,
            compiled: b,
        });
    }
    Ok(ReferenceInventory {
        schema_version: 1,
        inventory,
        reference_graph: ReferenceGraph {
            schema_version: 1,
            scope: "requested_compiled_definitions_only".into(),
            source_authority: false,
            runtime_admission: false,
            traversal_complete: complete,
            reference_closure_complete: false,
            files,
            blockers,
        },
    })
}
