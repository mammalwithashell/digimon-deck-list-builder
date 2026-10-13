//! Explicit offline review-v2 loader; never a live verdict or admission surface.
//!
//! The byte pin is external review input, not self-authentication. This reader
//! verifies wire/occurrence consistency; Python's source compiler remains the
//! segmentation authority. No old Book, verdict setter or admission API uses it.
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

fn require(ok: bool, message: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn sha(raw: &[u8]) -> String {
    format!("{:x}", Sha256::digest(raw))
}
fn array(v: &Value) -> Result<&Vec<Value>, String> {
    v.as_array().ok_or_else(|| "expected array".into())
}
fn object(v: &Value) -> Result<&Map<String, Value>, String> {
    v.as_object().ok_or_else(|| "expected object".into())
}
fn text(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or_else(|| "expected string".into())
}
fn index(v: &Value) -> Result<usize, String> {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| "expected nonnegative integer".into())
}
fn strict_integer(v: &Value, n: u64) -> bool {
    v.as_u64() == Some(n)
}
fn canonical(v: &Value) -> Result<Vec<u8>, String> {
    fn sorted(v: &Value) -> Value {
        match v {
            Value::Object(m) => {
                let order: BTreeMap<_, _> = m.iter().map(|(k, v)| (k.clone(), sorted(v))).collect();
                Value::Object(order.into_iter().collect())
            }
            Value::Array(a) => Value::Array(a.iter().map(sorted).collect()),
            _ => v.clone(),
        }
    }
    serde_json::to_vec(&sorted(v)).map_err(|e| e.to_string())
}
/// Exact arrays/string/integer/null codec. No newline or Unicode normalization.
pub fn wire_bytes_v2(v: &Value) -> Result<Vec<u8>, String> {
    fn check(v: &Value) -> Result<(), String> {
        match v {
            Value::Null | Value::String(_) => Ok(()),
            Value::Number(n) if n.is_i64() || n.is_u64() => Ok(()),
            Value::Array(a) => {
                for x in a {
                    check(x)?;
                }
                Ok(())
            }
            _ => Err("wire allows only arrays, strings, integers and null".into()),
        }
    }
    check(v)?;
    serde_json::to_vec(v).map_err(|e| e.to_string())
}

// Value's normal deserializer silently replaces duplicate keys. Refuse them
// before constructing any object, including nested diagnostic evidence.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("unique-key JSON")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(json!(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(json!(v)))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(json!(v)))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(json!(v)))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(json!(v)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut out = Vec::new();
                while let Some(Unique(v)) = a.next_element()? {
                    out.push(v);
                }
                Ok(Unique(Value::Array(out)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut out = Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if out.contains_key(&k) {
                        return Err(de::Error::custom("duplicate JSON key"));
                    }
                    let Unique(v) = a.next_value()?;
                    out.insert(k, v);
                }
                Ok(Unique(Value::Object(out)))
            }
        }
        d.deserialize_any(V)
    }
}

#[derive(Debug)]
pub struct ReviewBindingV2 {
    generation: String,
    semantic_sha: String,
}
impl ReviewBindingV2 {
    pub fn semantic_version(&self) -> u64 {
        2
    }
    pub fn source_generation(&self) -> &str {
        &self.generation
    }
    pub fn semantic_sha(&self) -> &str {
        &self.semantic_sha
    }
}
#[derive(Debug)]
pub struct ReviewBookV2 {
    document: Value,
    bindings: BTreeMap<String, ReviewBindingV2>,
    counts: BTreeMap<String, Option<usize>>,
    generation: String,
}
impl ReviewBookV2 {
    pub fn card_count(&self) -> usize {
        self.counts.len()
    }
    pub fn clause_ids(&self) -> Vec<String> {
        self.bindings.keys().cloned().collect()
    }
    pub fn is_recordable(&self) -> bool {
        false
    }
    pub fn card_clause_count(&self, id: &str) -> Result<Option<usize>, String> {
        self.counts
            .get(id)
            .copied()
            .ok_or_else(|| "card absent from inventory".into())
    }
    pub fn binding(&self, id: &str) -> Result<&ReviewBindingV2, String> {
        self.bindings
            .get(id)
            .ok_or_else(|| "unknown v2 clause".into())
    }
    pub fn source_generation(&self) -> &str {
        &self.generation
    }
    pub fn evidence(&self) -> &Value {
        &self.document
    }
}

fn span(v: &Value) -> Result<Value, String> {
    require(
        matches!(text(&v["kind"])?, "timing" | "keyword" | "untimed"),
        "unknown span kind",
    )?;
    for timing in array(&v["timings"])? {
        text(timing)?;
    }
    require(object(v)?.contains_key("keyword"), "missing keyword")?;
    if !v["keyword"].is_null() {
        text(&v["keyword"])?;
    }
    text(&v["text"])?;
    Ok(json!([v["kind"], v["timings"], v["keyword"], v["text"]]))
}
fn identity(v: &Value) -> Result<String, String> {
    text(&v["observation_id"])?;
    index(&v["section_index"])?;
    Ok(String::from_utf8(wire_bytes_v2(&json!([
        v["observation_id"],
        v["section_index"]
    ]))?)
    .map_err(|e| e.to_string())?)
}
fn multiset(a: &[Value]) -> Result<Vec<Vec<u8>>, String> {
    let mut r = a.iter().map(canonical).collect::<Result<Vec<_>, _>>()?;
    r.sort();
    Ok(r)
}

// These checks establish retained wire correspondence, not printed-source
// authority or an independent replay of the Python segmentation grammar.
fn folded_link_dp(source: &Value, unit: &Value, zone: &str) -> Result<(), String> {
    require(
        zone == "effect" && unit["role"] == "folded-link-dp",
        "invalid folded DP zone",
    )?;
    let oid = text(&unit["observation_id"])?;
    let derivation = &source["link_derivations"][oid];
    require(
        derivation["status"] == "DERIVED_LINK_DIAGNOSTIC",
        "missing Link derivation",
    )?;
    let matching = array(&derivation["occurrences"])?
        .iter()
        .filter(|o| o["role"] == "dp" && o["section_index"] == unit["section_index"])
        .collect::<Vec<_>>();
    require(matching.len() == 1, "exact DP derivation required")?;
    let occurrence = matching[0];
    require(
        occurrence["executable"] == false
            && occurrence["text"] == unit["text"]
            && unit["projected_text"]
                == json!(format!(
                    "[{}] {}",
                    text(&unit["label"])?,
                    text(&unit["text"])?
                )),
        "folded DP projection differs",
    )?;
    let mut expected = object(occurrence)?.clone();
    expected.insert("observation_id".into(), unit["observation_id"].clone());
    expected.insert("printing_id".into(), unit["printing_id"].clone());
    require(
        array(&source["structural_obligations"])?
            .iter()
            .filter(|o| **o == Value::Object(expected.clone()))
            .count()
            == 1,
        "exact nonexecuting DP obligation required",
    )?;
    require(
        unit.get("spans")
            .is_none_or(|s| s.as_array().is_some_and(Vec::is_empty)),
        "folded DP must not execute",
    )
}

fn link_correspondence(row: &Value) -> Result<(), String> {
    let source = &row["source_composition"];
    let binding = &source["condition_binding"];
    let officials = object(&row["original_observations"])?
        .iter()
        .filter(|(_, o)| o["source"] == "official")
        .collect::<BTreeMap<_, _>>();
    let derivations = object(&source["link_derivations"])?;
    require(
        !officials.is_empty()
            && officials.keys().copied().collect::<BTreeSet<_>>() == derivations.keys().collect(),
        "complete Link printing set required",
    )?;
    let mut contributions = Map::new();
    let mut obligations = Vec::new();
    let labels = [
        "special digivolution condition",
        "special play condition",
        "special evolution condition",
        "dna digivolution",
        "digixros",
    ];
    for (oid, observation) in &officials {
        let derived = &derivations[*oid];
        let sections = array(&observation["extracted"]["text_sections"])?;
        require(
            derived["observation"] == **observation
                && derived["section_inventory"] == json!(sections)
                && derived["status"] == "DERIVED_LINK_DIAGNOSTIC",
            "Link original derivation differs",
        )?;
        let inventory = array(&source["inventory"])?
            .iter()
            .filter(|r| r["observation_id"] == json!(oid))
            .collect::<Vec<_>>();
        require(
            inventory.len() == sections.len(),
            "Link section inventory differs",
        )?;
        let mut pieces = Vec::new();
        for (n, (section, item)) in sections.iter().zip(&inventory).enumerate() {
            require(
                item["section_index"] == json!(n)
                    && item["label"] == section["label"]
                    && item["text"] == section["text"],
                "Link original section changed",
            )?;
            if labels.contains(&text(&section["label"])?.to_lowercase().as_str()) {
                pieces.push(
                    format!("[{}] {}", text(&section["label"])?, text(&section["text"])?)
                        .trim()
                        .to_owned(),
                );
            }
        }
        contributions.insert((*oid).clone(), json!(pieces.join("\n")));
        for occurrence in array(&derived["occurrences"])? {
            let section = sections
                .get(index(&occurrence["section_index"])?)
                .ok_or("foreign Link section")?;
            let original = text(&section["text"])?;
            let start = index(&occurrence["utf8_start"])?;
            let end = index(&occurrence["utf8_end"])?;
            require(
                original.get(start..end) == Some(text(&occurrence["text"])?),
                "Link occurrence source slice differs",
            )?;
            if occurrence["role"] != "effect" {
                let mut obligation = object(occurrence)?.clone();
                obligation.insert("observation_id".into(), json!(oid));
                obligation.insert(
                    "printing_id".into(),
                    observation["extracted"]["printing"]["printing_id"].clone(),
                );
                obligations.push(Value::Object(obligation));
            }
        }
    }
    let evidence = array(&binding["evidence_observation_ids"])?
        .iter()
        .map(text)
        .collect::<Result<Vec<_>, _>>()?;
    require(
        binding["producer"] == "official_condition_projection"
            && binding["a2_section_coverage_claimed"] == false
            && binding["source_revision"] == source["source_revision"]
            && binding["candidate_values"] == Value::Object(contributions.clone())
            && evidence.len() == officials.len()
            && evidence.iter().copied().collect::<BTreeSet<_>>()
                == officials.keys().map(|k| k.as_str()).collect()
            && contributions.values().all(|v| *v == binding["projection"]),
        "Link condition binding differs",
    )?;
    require(
        source["structural_obligations"] == json!(obligations),
        "Link structural obligations differ",
    )?;
    require(
        source["zones"]["conditions"]["source_binding"] == *binding
            && source["zones"]["conditions"]["value"] == binding["projection"],
        "Link condition zone differs",
    )?;
    let first = officials.keys().next().ok_or("empty Link sources")?;
    let selected = array(&source["inventory"])?
        .iter()
        .filter(|r| r["observation_id"] == json!(first))
        .filter_map(|r| match text(&r["label"]) {
            Ok(label) if labels.contains(&label.to_lowercase().as_str()) => Some(Ok(r)),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let condition_indices = array(&source["zones"]["conditions"]["unit_indices"])?;
    require(
        condition_indices.len() == selected.len(),
        "Link condition occurrence set differs",
    )?;
    for (number, original) in condition_indices.iter().zip(selected) {
        let mut expected = object(original)?.clone();
        expected.insert("kind".into(), json!("official-section"));
        expected.insert("field".into(), json!("xros_req"));
        expected.insert(
            "condition_type".into(),
            json!(text(&original["label"])?.to_lowercase().replace(' ', "_")),
        );
        expected.insert("projected_text".into(), original["text"].clone());
        expected.insert("role".into(), json!("condition"));
        expected.insert(
            "transformation".into(),
            json!("unanimous-producer-condition"),
        );
        require(
            array(&source["units"])?.get(index(number)?) == Some(&Value::Object(expected)),
            "Link condition unit differs",
        )?;
    }
    for u in array(&source["units"])? {
        let oid = text(&u["observation_id"])?;
        let is_dp = array(&derivations.get(oid).ok_or("foreign Link unit")?["occurrences"])?
            .iter()
            .any(|o| o["role"] == "dp" && o["section_index"] == u["section_index"]);
        require(
            is_dp == (u["role"] == "folded-link-dp"),
            "Link DP role differs",
        )?;
        require(
            matches!(text(&u["role"])?, "effect" | "condition" | "folded-link-dp"),
            "unknown Link role",
        )?;
    }
    Ok(())
}

fn paired_main_mirror(row: &Value, units: &[Value]) -> Result<(), String> {
    let source = &row["source_composition"];
    let binding = &source["correction_binding"];
    let corrected = units
        .iter()
        .enumerate()
        .filter(|(_, u)| u["kind"] == "reviewed-paired-main-image-correction")
        .collect::<Vec<_>>();
    require(
        corrected.len() == 1,
        "one paired main correction unit required",
    )?;
    let (i, u) = corrected[0];
    let mut expected_unit = object(&binding["unit"])?.clone();
    expected_unit.insert("projected_text".into(), binding["unit"]["value"].clone());
    expected_unit.insert(
        "transformation".into(),
        json!("reviewed-paired-main-image-correction"),
    );
    expected_unit.insert("sidecar_sha256".into(), binding["sidecar_sha256"].clone());
    require(
        *u == Value::Object(expected_unit) && u["field_path"] == "effect_description_eng",
        "paired unit differs from binding",
    )?;
    let mirror = &binding["face_mirror_obligation"];
    require(
        mirror["field_path"] == "faces.0.effect_text"
            && mirror["value"] == u["value"]
            && mirror["executable"] == false,
        "paired face must be exact nonexecuting mirror",
    )?;
    let mut expected = object(mirror)?.clone();
    expected.insert("kind".into(), json!("paired-main-face-consistency"));
    expected.insert("source_unit_index".into(), json!(i));
    expected.insert(
        "correction_sidecar_sha256".into(),
        binding["sidecar_sha256"].clone(),
    );
    let mirrors = array(&source["structural_obligations"])?
        .iter()
        .filter(|o| o["kind"] == "paired-main-face-consistency")
        .collect::<Vec<_>>();
    require(
        mirrors.len() == 1 && *mirrors[0] == Value::Object(expected),
        "paired mirror obligation differs",
    )?;
    require(
        source["zones"]["effect"]["unit_indices"] == json!([i])
            && source["zones"]["effect"]["source_binding"] == *binding
            && source["zones"]["effect"]["value"] == u["value"],
        "paired effect zone binding differs",
    )?;
    let observations = object(&binding["source_observations"])?;
    require(
        binding["source_observations"] == row["original_observations"],
        "paired complete observations differ",
    )?;
    let corrections = array(&binding["corrections"])?;
    require(
        corrections.len() == 2
            && corrections
                .iter()
                .map(|c| text(&c["field_path"]))
                .collect::<Result<BTreeSet<_>, _>>()?
                == BTreeSet::from(["effect_description_eng", "faces.0.effect_text"])
            && corrections
                .iter()
                .all(|c| c["card_id"] == u["card_id"] && c["value"] == u["value"]),
        "paired correction values differ",
    )?;
    let images = array(&binding["images"])?;
    require(images.len() == 2, "exact two image associations required")?;
    for key in ["artifact_name", "official_observation_id", "printing_id"] {
        let values = images
            .iter()
            .map(|image| text(&image[key]))
            .collect::<Result<BTreeSet<_>, _>>()?;
        require(
            values.len() == 2 && values.iter().all(|v| !v.is_empty()),
            "distinct image association identities required",
        )?;
    }
    let image_ids = images
        .iter()
        .map(|image| text(&image["official_observation_id"]))
        .collect::<Result<BTreeSet<_>, _>>()?;
    for correction in corrections {
        let ids = array(&correction["evidence_observation_ids"])?;
        require(
            ids.len() == 2
                && ids.iter().map(text).collect::<Result<BTreeSet<_>, _>>()? == image_ids,
            "image/correction evidence membership differs",
        )?;
    }
    for image in images {
        require(
            object(image)?
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                == BTreeSet::from([
                    "artifact_name",
                    "official_observation_id",
                    "printing_id",
                    "sha256",
                ]),
            "exact image association schema required",
        )?;
        let hash = text(&image["sha256"])?;
        require(
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "strict image byte pin required",
        )?;
        let observation = observations
            .get(text(&image["official_observation_id"])?)
            .ok_or("missing image observation")?;
        require(
            observation["source"] == "official"
                && observation["extracted"]["printing"]["printing_id"] == image["printing_id"],
            "paired image source association differs",
        )?;
    }
    let mut originals = BTreeSet::new();
    for original in array(&binding["original_sections"])? {
        let observation = observations
            .get(text(&original["observation_id"])?)
            .ok_or("missing original observation")?;
        let section = array(&observation["extracted"]["text_sections"])?
            .get(index(&original["section_index"])?)
            .ok_or("missing original section")?;
        require(
            *section == json!({"label":original["label"],"text":original["text"]})
                && original["source_url"] == observation["source_url"]
                && original["raw_payload_digest"] == observation["raw_payload_digest"]
                && original["printing_id"] == observation["extracted"]["printing"]["printing_id"],
            "paired original section differs",
        )?;
        require(
            originals.insert(identity(original)?),
            "duplicate corrected original",
        )?;
    }
    let replaced = array(&source["consumption"])?
        .iter()
        .filter(|r| r["disposition"] == "superseded-by-reviewed-correction")
        .collect::<Vec<_>>();
    require(
        replaced.len() == originals.len()
            && replaced
                .iter()
                .map(|r| identity(r))
                .collect::<Result<BTreeSet<_>, _>>()?
                == originals
            && replaced.iter().all(|r| {
                r["unit_indices"] == json!([]) && r["replacement_unit_indices"] == json!([i])
            }),
        "paired original source replacement inventory differs",
    )
}

// Rebuild occurrence descriptors and projected span choice from the retained
// source composition, rather than trusting submitted clause semantic_payload.
fn occurrences(cid: &str, row: &Value) -> Result<Vec<Value>, String> {
    let source = &row["source_composition"];
    let units = array(&source["units"])?;
    let inventory = array(&source["inventory"])?;
    let consumption = array(&source["consumption"])?;
    let mut by_key = BTreeMap::new();
    for item in inventory {
        require(
            by_key.insert(identity(item)?, item).is_none(),
            "duplicate source occurrence",
        )?;
    }
    require(
        consumption.len() == inventory.len(),
        "consumption inventory differs",
    )?;
    let mut consumed = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for item in consumption {
        let key = identity(item)?;
        require(consumed.insert(key.clone()), "duplicate consumption")?;
        let original = by_key.get(&key).ok_or("foreign consumption")?;
        for (k, v) in object(original)? {
            require(item.get(k) == Some(v), "source container changed")?;
        }
        for name in ["unit_indices", "replacement_unit_indices"] {
            if let Some(indices) = item.get(name) {
                for i in array(indices)? {
                    let n = index(i)?;
                    require(n < units.len(), "foreign unit")?;
                    seen.insert(n);
                }
            }
        }
    }
    require(
        seen == (0..units.len()).collect(),
        "unaccounted source unit",
    )?;
    let mut wrapped = Vec::new();
    let mut views = Vec::new();
    let mut obligations = source
        .get("structural_obligations")
        .map(array)
        .transpose()?
        .cloned()
        .unwrap_or_default();
    for (i, u) in units.iter().enumerate() {
        let key = if u.get("observation_id").is_some() {
            require(by_key.contains_key(&identity(u)?), "foreign unit source")?;
            json!([
                cid,
                "official-occurrence",
                u["observation_id"],
                u["section_index"],
                i
            ])
        } else {
            require(
                u["kind"] == "reviewed-canonical-image-correction"
                    || (source["status"] == "COMPOSED_PAIRED_MAIN_CARD"
                        && u["kind"] == "reviewed-paired-main-image-correction"),
                "unknown sourceless unit",
            )?;
            text(&u["sidecar_sha256"])?;
            json!([cid, "reviewed-correction", u["sidecar_sha256"], i])
        };
        let face = u.get("face_id").cloned().unwrap_or(json!("primary"));
        text(&face)?;
        wrapped.push(json!({"key":key,"unit_index":i,"face":face,"source_unit":u}));
        if u["role"] == "arts-reminder" {
            require(array(&u["spans"])?.is_empty(), "executable Arts reminder")?;
            obligations.push(json!({"purpose":"arts-reminder","executable":false,"unit_key":key,"source_unit":u}));
        }
        if let Some(spans) = u.get("spans") {
            for (n, s) in array(spans)?.iter().enumerate() {
                span(s)?;
                views.push(json!({"unit_key":key,"face":face,"zone":u["role"],"occurrence_span_index":n,"span":s}));
            }
        }
    }
    if let Some(zones) = source.get("zones") {
        for (zone, z) in object(zones)? {
            for s in array(&z["spans"])? {
                let i = index(&s["unit_index"])?;
                require(i < units.len(), "foreign span unit")?;
                index(&s["span_index"])?;
                span(s)?;
                views.push(json!({"unit_key":wrapped[i]["key"],"face":wrapped[i]["face"],"zone":zone,"occurrence_span_index":s["span_index"],"span":s}));
            }
        }
    }
    let mut groups: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
    for v in &views {
        groups
            .entry(wire_bytes_v2(&json!([v["unit_key"], v["zone"]]))?)
            .or_default()
            .push(index(&v["occurrence_span_index"])?);
    }
    for indices in groups.values() {
        require(
            *indices == (0..indices.len()).collect::<Vec<_>>(),
            "invalid local span sequence",
        )?;
    }
    for o in &obligations {
        require(
            o.get("executable") == Some(&Value::Bool(false)),
            "executable structural obligation",
        )?;
    }
    require(
        Value::Array(wrapped.clone()) == row["source_unit_descriptors"]
            && multiset(&views)? == multiset(array(&row["span_descriptors"])?)?,
        "source descriptors differ",
    )?;
    require(
        json!(obligations) == row["structural_obligations"],
        "structural obligations differ",
    )?;
    require(
        row["conservation"]
            == json!({"occurrences":inventory.len(),"units":units.len(),"spans":views.len(),"structural_obligations":obligations.len()})
            && row["span_count"] == json!(views.len()),
        "conservation inventory differs",
    )?;
    let mut selected: BTreeMap<usize, (String, Vec<Value>)> = BTreeMap::new();
    let mut used = BTreeSet::new();
    if source["status"] == "COMPOSED_LINK_CARD" {
        link_correspondence(row)?;
    }
    if source["status"] == "COMPOSED_PAIRED_MAIN_CARD" {
        paired_main_mirror(row, units)?;
    }
    if source["status"] == "COMPOSED_DUAL_SOURCE" {
        require(
            source
                .get("zones")
                .is_none_or(|v| v.as_object().is_some_and(|m| m.is_empty())),
            "Dual has ordinary zones",
        )?;
        for (i, u) in units.iter().enumerate() {
            let role = text(&u["role"])?;
            require(
                matches!(
                    role,
                    "effect" | "inherited" | "security" | "condition" | "arts-reminder"
                ),
                "unknown Dual role",
            )?;
            used.insert(i);
            if role != "arts-reminder" {
                selected.insert(
                    i,
                    (
                        role.into(),
                        array(&u["spans"])?
                            .iter()
                            .map(span)
                            .collect::<Result<_, _>>()?,
                    ),
                );
            }
        }
    } else {
        require(
            matches!(
                text(&source["status"])?,
                "COMPOSED_RULE_CARD"
                    | "COMPOSED_CORRECTED_RULE_CARD"
                    | "COMPOSED_LINK_CARD"
                    | "COMPOSED_PAIRED_MAIN_CARD"
            ),
            "unsupported composition",
        )?;
        for (zone, z) in object(&source["zones"])? {
            require(
                matches!(
                    zone.as_str(),
                    "effect" | "inherited" | "security" | "conditions"
                ) || (source["status"] == "COMPOSED_LINK_CARD" && zone == "link"),
                "unknown zone",
            )?;
            let indices = array(&z["unit_indices"])?;
            for v in indices {
                let i = index(v)?;
                require(i < units.len() && used.insert(i), "invalid duplicate unit")?;
                if units[i]["role"] == "folded-link-dp" {
                    require(
                        source["status"] == "COMPOSED_LINK_CARD",
                        "folded DP outside Link route",
                    )?;
                    folded_link_dp(source, &units[i], zone)?;
                }
                selected.insert(i, (zone.clone(), Vec::new()));
            }
            let mut sequence = Vec::new();
            for s in array(&z["spans"])? {
                let i = index(&s["unit_index"])?;
                let n = index(&s["span_index"])?;
                require(
                    units.get(i).is_some_and(|u| u["role"] != "folded-link-dp"),
                    "folded DP became executable",
                )?;
                let (selected_zone, spans) = selected.get_mut(&i).ok_or("span outside zone")?;
                require(
                    selected_zone == zone && n == spans.len(),
                    "invalid projected local index",
                )?;
                spans.push(span(s)?);
                sequence.push((i, n));
            }
            let expected = indices
                .iter()
                .map(index)
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flat_map(|i| (0..selected[&i].1.len()).map(move |n| (i, n)))
                .collect::<Vec<_>>();
            require(
                sequence == expected,
                "projected occurrence ordering differs",
            )?;
        }
    }
    require(used == (0..units.len()).collect(), "missing source unit")?;
    let mut out = Vec::new();
    for (i, (zone, spans)) in selected {
        for (n, s) in spans.into_iter().enumerate() {
            let u = &wrapped[i];
            let matches = views
                .iter()
                .filter(|v| v["unit_key"] == u["key"] && v["occurrence_span_index"] == json!(n))
                .cloned()
                .collect::<Vec<_>>();
            require(!matches.is_empty(), "missing span view")?;
            out.push(json!({"unit_key":u["key"],"face":u["face"],"zone":zone,"local_span_index":n,"span":s,"descriptor_views":matches}));
        }
    }
    Ok(out)
}

fn validate_inventory(d: &Value) -> Result<(), String> {
    let cards = object(&d["cards"])?;
    require(
        !cards.is_empty() && cards.keys().all(|k| !k.is_empty()),
        "complete card inventory required",
    )?;
    let totals = object(&d["membership_counts"])?;
    let axes = ["official", "canonical", "runtime"];
    require(
        totals.len() == 3 && axes.iter().all(|k| totals.contains_key(*k)),
        "membership count fields differ",
    )?;
    for n in totals.values() {
        index(n)?;
    }
    let selected = array(&d["evaluated_card_ids"])?;
    let ids = selected.iter().map(text).collect::<Result<Vec<_>, _>>()?;
    require(
        !ids.is_empty()
            && ids.windows(2).all(|p| p[0] < p[1])
            && ids.iter().all(|id| cards.contains_key(*id)),
        "unique present evaluated IDs required",
    )?;
    let mut observed = [0usize; 3];
    let flags = [
        "official",
        "canonical",
        "runtime",
        "provisional",
        "runtime_only",
        "test_only",
        "qa",
        "qa_only",
    ];
    for (cid, row) in cards {
        object(row)?;
        let member = object(&row["membership"])?;
        require(
            member.len() == flags.len()
                && flags
                    .iter()
                    .all(|k| member.get(*k).is_some_and(Value::is_boolean)),
            "strict membership flags required",
        )?;
        let official = member["official"].as_bool().ok_or("official Boolean")?;
        let canonical = member["canonical"].as_bool().ok_or("canonical Boolean")?;
        let runtime = member["runtime"].as_bool().ok_or("runtime Boolean")?;
        let qa = member["qa"].as_bool().ok_or("qa Boolean")?;
        require(
            member["provisional"] == json!(canonical && !official)
                && member["runtime_only"] == json!(runtime && !canonical)
                && member["test_only"] == json!(cid.starts_with("TEST-") && !official)
                && member["qa_only"] == json!(qa && !(official || canonical || runtime)),
            "derived membership flags differ",
        )?;
        for (i, axis) in axes.iter().enumerate() {
            if member[*axis] == Value::Bool(true) {
                observed[i] += 1;
            }
        }
        let blockers = array(&row["blockers"])?;
        for blocker in blockers {
            object(blocker)?;
            require(!text(&blocker["code"])?.is_empty(), "blocker code required")?;
        }
        let status = text(&row["status"])?;
        require(
            matches!(status, "COMPOSED_DIAGNOSTIC" | "NOT_EVALUATED" | "BLOCKED"),
            "unknown diagnostic card status",
        )?;
        require(
            row.get("gating_interaction_count") == Some(&Value::Null),
            "unknown gating count must remain null",
        )?;
        let evaluated = ids.contains(&cid.as_str());
        if evaluated {
            require(
                official && canonical && runtime && status != "NOT_EVALUATED",
                "incompatible evaluated card",
            )?;
        }
        if status == "COMPOSED_DIAGNOSTIC" {
            require(
                evaluated && blockers.is_empty(),
                "invalid composed blockers/evaluation",
            )?;
            index(&row["span_count"])?;
        } else {
            if row.get("span_count") != Some(&Value::Null) {
                require(
                    status == "BLOCKED" && evaluated,
                    "only evaluated blocked source spans may be known",
                )?;
                index(&row["span_count"])?;
                // Reconcile all retained descriptors, but never emit clauses or
                // assign a known executable count for this blocked row.
                occurrences(cid, row)?;
            }
            require(
                !blockers.is_empty() == (status == "BLOCKED"),
                "blocked status and blockers differ",
            )?;
            if status == "NOT_EVALUATED" {
                require(
                    official && canonical && runtime,
                    "unevaluated card lacks source membership",
                )?;
            }
        }
    }
    for (i, axis) in axes.iter().enumerate() {
        require(
            index(&totals[*axis])? == observed[i],
            "membership total differs from retained inventory",
        )?;
    }
    Ok(())
}

pub fn load_review_v2(raw: &[u8], expected_sha: &str) -> Result<ReviewBookV2, String> {
    require(
        expected_sha.len() == 64
            && expected_sha
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && sha(raw) == expected_sha,
        "review byte pin mismatch",
    )?;
    let mut decoder = serde_json::Deserializer::from_slice(raw);
    let Unique(book) = Unique::deserialize(&mut decoder).map_err(|e| e.to_string())?;
    decoder.end().map_err(|e| e.to_string())?;
    require(
        strict_integer(&book["book_schema_version"], 2)
            && strict_integer(&book["semantic_fingerprint_version"], 2),
        "unknown version",
    )?;
    let d = &book["diagnostic_evidence"];
    validate_inventory(d)?;
    require(
        strict_integer(&d["schema_version"], 1)
            && d["kind"] == "bounded-source-composition-diagnostic-v1"
            && d["status"] == "DIAGNOSTIC_ONLY_NOT_ADMISSION",
        "unknown diagnostic envelope",
    )?;
    require(
        d.get("full_span_count") == Some(&Value::Null)
            && d.get("full_gating_interaction_count") == Some(&Value::Null),
        "false diagnostic completeness",
    )?;
    for key in [
        "snapshot_sha256",
        "input_manifest_sha256",
        "source_revision",
    ] {
        text(&d[key])?;
    }
    let payload = json!([
        "digimon-source-generation",
        2,
        d["snapshot_sha256"],
        d["input_manifest_sha256"],
        d["source_revision"],
        sha(&canonical(&d["dependency_sha256"])?),
        sha(&canonical(&book["compiler_dependencies"])?)
    ]);
    let generation = sha(&wire_bytes_v2(&payload)?);
    let mut cards = Map::new();
    let mut clauses = Vec::new();
    let mut counts = BTreeMap::new();
    let mut bindings = BTreeMap::new();
    let ordered: BTreeMap<_, _> = object(&d["cards"])?.iter().collect();
    for (cid, row) in ordered {
        let mut card = json!({"status":row["status"],"membership":row["membership"],"blockers":row["blockers"],"clause_count":null,"clause_ids":[],"gating_interaction_count":null});
        if row["status"] == "COMPOSED_DIAGNOSTIC" {
            require(
                array(&d["evaluated_card_ids"])?.contains(&json!(cid))
                    && array(&row["blockers"])?.is_empty(),
                "unselected or blocked success",
            )?;
            let occurrence = occurrences(cid, row)?;
            let closure = sha(&canonical(row)?);
            let siblings = occurrence
                .iter()
                .map(|o| {
                    json!([
                        o["unit_key"],
                        o["face"],
                        o["zone"],
                        o["local_span_index"],
                        o["span"]
                    ])
                })
                .collect::<Vec<_>>();
            card["status"] = json!("COMPILED_REVIEW_ONLY");
            card["clause_count"] = json!(occurrence.len());
            card["source_closure_sha256"] = json!(closure);
            card["structural_obligations"] = row["structural_obligations"].clone();
            card["superseded_source_occurrences"] =
                json!(array(&row["source_composition"]["consumption"])?
                    .iter()
                    .filter(|c| c["disposition"] == "superseded-by-reviewed-correction")
                    .count());
            counts.insert(cid.clone(), Some(occurrence.len()));
            for o in occurrence {
                let locator = json!([o["face"], o["unit_key"], o["zone"], o["local_span_index"]]);
                let id = format!("{cid}#v2#{}", sha(&wire_bytes_v2(&locator)?));
                let semantic = json!([
                    "digimon-clause-semantics",
                    2,
                    generation,
                    cid,
                    locator,
                    o["span"],
                    siblings,
                    closure
                ]);
                let hash = sha(&wire_bytes_v2(&semantic)?);
                let s = &o["span"];
                clauses.push(json!({"id":id,"card_id":cid,"face":o["face"],"unit_key":o["unit_key"],"zone":o["zone"],"local_span_index":o["local_span_index"],"kind":s[0],"timings":s[1],"keyword":s[2],"text":s[3],"descriptor_views":o["descriptor_views"],"semantic_fingerprint_version":2,"semantic_payload":semantic,"semantic_sha256":hash}));
                card["clause_ids"]
                    .as_array_mut()
                    .ok_or("clause ID array")?
                    .push(json!(id));
                require(
                    bindings
                        .insert(
                            id,
                            ReviewBindingV2 {
                                generation: generation.clone(),
                                semantic_sha: hash,
                            },
                        )
                        .is_none(),
                    "duplicate clause identity",
                )?;
            }
        } else {
            require(
                matches!(text(&row["status"])?, "BLOCKED" | "NOT_EVALUATED"),
                "unknown card status",
            )?;
            counts.insert(cid.clone(), None);
        }
        cards.insert(cid.clone(), card);
    }
    let expected = json!({"book_schema_version":2,"semantic_fingerprint_version":2,"status":"REVIEW_ONLY_NOT_ADMISSION","source_generation_sha256":generation,"source_generation_payload":payload,"compiler_dependencies":book["compiler_dependencies"],"membership_counts":d["membership_counts"],"full_clause_count":null,"full_gating_interaction_count":null,"cards":cards,"clauses":clauses,"diagnostic_evidence":d});
    require(
        canonical(&expected)? == canonical(&book)?,
        "review-v2 contents differ from occurrence reconstruction",
    )?;
    Ok(ReviewBookV2 {
        document: book,
        bindings,
        counts,
        generation,
    })
}
#[cfg(test)]
#[path = "source_book_v2_composition_routes_tests.rs"]
mod composition_routes_tests;
#[cfg(test)]
#[path = "source_book_v2_tests.rs"]
mod tests;
