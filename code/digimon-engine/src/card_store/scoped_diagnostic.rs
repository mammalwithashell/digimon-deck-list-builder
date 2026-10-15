//! Inspection only. An externally accepted binding is an operand, not authority
//! discovered by this constructor. No Game, dispatch, default pack or token cache.
use super::diagnostic_wire::*;
use crate::card_data::CardData;
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_dsl::compiled::{CompiledAltPath, CompiledCardKind};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, String>;
const FAMILIES: &[&str] = &[
    "traits",
    "numeric_sentinels",
    "kind_colors",
    "registry_f32",
    "printed_keywords",
    "raw_digixros_aliases",
    "dual_face_lookup",
    "dsl_enrichment",
    "dsl_dna_restrictions",
    "tokens",
];

/// Uncached, immutable inspection of exactly the binding's executable membership.
///
/// The caller must obtain `expected_binding_sha256` from independent accepted
/// evidence. A caller-authored hash does not establish source or runtime authority.
/// The complete binding bytes retain semantic unknowns and witness references.
/// This type deliberately has no Deref, mutable map, SharedCardStore or Game path.
#[derive(Clone)]
pub struct ScopedDiagnosticStore {
    binding: Vec<u8>,
    cards: Vec<CardData>,
    offsets: BTreeMap<String, usize>,
    alt_paths: BTreeMap<String, Vec<CompiledAltPath>>,
}

fn sha(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn digest(v: &Value) -> bool {
    v.as_str().is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}
fn pin(v: &Value, raw: &[u8]) -> Result<()> {
    keys(v, &["bytes", "sha256"], &["bytes", "sha256"], "byte pin")?;
    require(
        v["bytes"].as_u64() == Some(raw.len() as u64)
            && v["sha256"].as_str() == Some(sha(raw).as_str()),
        "bound input differs",
    )
}
fn ids(v: &Value) -> Result<BTreeSet<String>> {
    strings(v, "scope IDs")?;
    let mut out = BTreeSet::new();
    for id in v.as_array().ok_or("scope array")? {
        let id = id.as_str().ok_or("scope ID string")?;
        card_suffix(id)?;
        require(out.insert(id.to_owned()), "duplicate scope ID")?;
    }
    Ok(out)
}

impl ScopedDiagnosticStore {
    /// Validate exact supplied bytes, decode only the supplied pack, and enrich
    /// only selected rows. Raw-function presence is checked; functions never run.
    pub fn from_bound_bytes(
        binding: &[u8],
        expected_binding_sha256: &str,
        runtime: &[u8],
        registry: &[u8],
        pack: &[u8],
        raw: &EngineRawRustRegistry,
    ) -> Result<Self> {
        require(
            expected_binding_sha256.len() == 64 && sha(binding) == expected_binding_sha256,
            "binding digest differs",
        )?;
        require(pack.len() as u64 <= MAX_JSON, "pack byte limit")?;
        let b = strict_json(binding)?;
        let fields = [
            "kind",
            "schema_version",
            "status",
            "is_recordable",
            "contract_sha256",
            "input_pins",
            "accepted_evidence",
            "consumer_dependencies",
            "source_generation_sha256",
            "source_revision",
            "required_card_ids",
            "executable_card_ids",
            "cards",
            "native_executable",
            "native_pack",
            "native_request_sha256",
            "derived_families",
            "field_slots",
            "counts",
            "retained_witness_roles",
            "unresolved",
            "admission",
            "projection_epoch",
            "full_clause_count",
            "full_gating_interaction_count",
        ];
        keys(&b, &fields, &fields, "diagnostic binding")?;
        require(
            b["kind"] == json!("focused-native-binding-v1")
                && b["schema_version"] == json!(1)
                && b["status"] == json!("DIAGNOSTIC_ONLY_NOT_RUNTIME")
                && b["is_recordable"] == json!(false),
            "binding kind/policy",
        )?;
        for key in [
            "contract_sha256",
            "source_generation_sha256",
            "source_revision",
            "native_request_sha256",
        ] {
            require(digest(&b[key]), format!("binding digest {key}"))?;
        }
        let gates = ["source", "native", "runtime", "oracle", "training"];
        keys(&b["admission"], &gates, &gates, "admission")?;
        require(
            gates.iter().all(|k| b["admission"][k] == json!(false)),
            "admission must remain false",
        )?;
        for k in [
            "projection_epoch",
            "full_clause_count",
            "full_gating_interaction_count",
        ] {
            require(b[k].is_null(), format!("unknown must remain null: {k}"))?;
        }
        let slots: Vec<_> = RAW
            .iter()
            .chain(COMPAT)
            .chain(META)
            .copied()
            .chain(["card_index"])
            .collect();
        require(
            b["field_slots"] == json!(slots) && b["derived_families"] == json!(FAMILIES),
            "native obligations differ",
        )?;
        require(
            b["retained_witness_roles"]
                == json!(["book", "metadata", "records", "runtime", "registry", "native"]),
            "witness roles differ",
        )?;
        strings(&b["unresolved"], "unresolved")?;
        for marker in [
            "ALL_CAPTURED_SEMANTIC_GATES_RETAINED",
            "NO_RUNTIME_HANDLE",
            "TASK_2_3_INCOMPLETE",
        ] {
            require(
                b["unresolved"].as_array().unwrap().contains(&json!(marker)),
                "missing unresolved boundary",
            )?;
        }
        let required = ids(&b["required_card_ids"])?;
        let selected = ids(&b["executable_card_ids"])?;
        require(
            !required.is_empty() && selected.is_subset(&required),
            "E/required membership",
        )?;
        let card_bindings = object(&b["cards"], "card bindings")?;
        require(
            card_bindings.keys().cloned().collect::<BTreeSet<_>>() == required,
            "required card bindings differ",
        )?;
        for card in card_bindings.values() {
            require(
                card["native_semantics_proven"] == json!(false),
                "native semantics must remain unproven",
            )?;
        }
        pin(&b["input_pins"]["runtime"], runtime)?;
        pin(&b["input_pins"]["registry"], registry)?;
        pin(&b["native_pack"], pack)?;
        let rows = strict_json(runtime)?;
        let rows = object(&rows, "runtime")?;
        require(
            rows.keys().cloned().collect::<BTreeSet<_>>() == selected,
            "runtime must equal exact E",
        )?;
        let registry = strict_json(registry)?;
        let assignments = registry_wire(&registry)?;
        let counts = [
            "required",
            "executable",
            "inventory",
            "registry",
            "field_slots",
        ];
        keys(&b["counts"], &counts, &counts, "counts")?;
        require(
            b["counts"]["required"].as_u64() == Some(required.len() as u64)
                && b["counts"]["executable"].as_u64() == Some(selected.len() as u64)
                && b["counts"]["registry"].as_u64() == Some(assignments.len() as u64)
                && b["counts"]["field_slots"].as_u64()
                    == Some((selected.len() * slots.len()) as u64)
                && b["counts"]["inventory"]
                    .as_u64()
                    .is_some_and(|n| n >= required.len() as u64),
            "binding denominator differs",
        )?;
        for (id, row) in rows {
            wire_row(id, row, assignments)?;
        }
        // Complete-pack checks include outside-E entries and required raw names;
        // only selected metadata and paths enter this inspection store.
        let checked = checked_pack(pack, raw)?;
        let dsl = digimon_dsl::CardRegistry::from_pack_bytes_with_raw_registry(pack, raw)?;
        for id in &selected {
            let compiled = dsl
                .lookup(id)
                .ok_or_else(|| format!("selected card absent from explicit pack: {id}"))?;
            require(
                compiled.kind != CompiledCardKind::Token,
                "empty token allowlist",
            )?;
        }
        drop(checked);
        let text = std::str::from_utf8(runtime).map_err(|e| e.to_string())?;
        let mut metadata = CardData::load_from_str(text)?;
        require(
            metadata.keys().cloned().collect::<BTreeSet<_>>() == selected,
            "decoded membership differs",
        )?;
        for (id, card) in &metadata {
            let global = assignments[id].as_u64().ok_or("global index")? as u16;
            require(
                card.index == global
                    && card.norm_id.to_bits() == ((global as f64 / 20000.0) as f32).to_bits(),
                "native registry/f32 differs",
            )?;
        }
        crate::dsl_bridge::enrich_card_data_with_dsl_alt_paths(&mut metadata, &dsl);
        let mut cards = Vec::new();
        let mut offsets = BTreeMap::new();
        let mut alt_paths = BTreeMap::new();
        for id in selected {
            offsets.insert(id.clone(), cards.len());
            cards.push(metadata.remove(&id).ok_or("selected metadata missing")?);
            alt_paths.insert(
                id.clone(),
                dsl.lookup(&id)
                    .ok_or("selected IR missing")?
                    .alt_paths
                    .clone(),
            );
        }
        Ok(Self {
            binding: binding.to_vec(),
            cards,
            offsets,
            alt_paths,
        })
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
    /// The retained accepted operand, including all unknowns; no new admission.
    pub fn binding_bytes(&self) -> &[u8] {
        &self.binding
    }
    /// Diagnostic vector offset, explicitly distinct from CardData.index.
    pub fn internal_offset(&self, id: &str) -> Result<usize> {
        self.offsets
            .get(id)
            .copied()
            .ok_or_else(|| format!("outside executable scope: {id}"))
    }
    pub fn card(&self, id: &str) -> Result<&CardData> {
        Ok(&self.cards[self.internal_offset(id)?])
    }
    pub fn alt_paths(&self, id: &str) -> Result<&[CompiledAltPath]> {
        self.internal_offset(id)?;
        Ok(&self.alt_paths[id])
    }
}
