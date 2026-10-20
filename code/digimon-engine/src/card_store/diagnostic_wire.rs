//! Shared finite wire/pack checks; no filesystem, globals, or runtime construction.
use super::DiagnosticKeyword;
use digimon_dsl::{raw_rust_registry::RawRustRegistry, CardPack};
use serde::{Deserialize, Deserializer};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
type Result<T> = std::result::Result<T, String>;

pub(super) const MAX_JSON: u64 = 64 * 1024 * 1024;
pub(super) const MAX_JSON_NODES: usize = 1_000_000;
std::thread_local! {static JSON_NODES:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}

pub(super) const RAW: &[&str] = &[
    "card_id",
    "card_name_eng",
    "card_effect_class_name",
    "play_cost",
    "dp",
    "level",
    "card_kind",
    "card_colors",
    "type_eng",
    "form_eng",
    "attribute_eng",
    "effect_description_eng",
    "inherited_effect_description_eng",
    "security_effect_description_eng",
    "link_effect_description_eng",
    "evo_costs",
    "dna_costs",
    "index",
    "norm_id",
    "ace_overflow",
    "dual",
    "digixros_aliases",
];
pub(super) const OPTIONAL: &[&str] = &[
    "link_effect_description_eng",
    "dna_costs",
    "ace_overflow",
    "dual",
    "digixros_aliases",
];
pub(super) const COMPAT: &[&str] = &["xros_req", "digixros_costs", "max_count_in_deck"];
pub(super) const META: &[&str] = &["card_name_jpn", "rarity"];
pub(super) fn require(ok: bool, message: impl Into<String>) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(message.into())
    }
}
// serde_json's ordinary map reader accepts duplicate keys. Reject them before
// the actual engine deserializer can silently discard an earlier value.
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let nodes = JSON_NODES.with(|n| {
            let v = n.get() + 1;
            n.set(v);
            v
        });
        if nodes > MAX_JSON_NODES {
            return Err(serde::de::Error::custom("JSON node limit"));
        }
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Strict;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("finite JSON without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Strict, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Strict(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Strict, E> {
                Ok(Strict(json!(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut out = Vec::new();
                while let Some(v) = a.next_element::<Strict>()? {
                    out.push(v.0)
                }
                Ok(Strict(Value::Array(out)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut out = Map::new();
                while let Some((k, v)) = a.next_entry::<String, Strict>()? {
                    if out.insert(k.clone(), v.0).is_some() {
                        return Err(serde::de::Error::custom(format!("duplicate key {k}")));
                    }
                }
                Ok(Strict(Value::Object(out)))
            }
        }
        d.deserialize_any(Visitor)
    }
}
pub(super) fn strict_json(raw: &[u8]) -> Result<Value> {
    require(raw.len() as u64 <= MAX_JSON, "JSON byte limit")?;
    JSON_NODES.with(|n| n.set(0));
    let mut d = serde_json::Deserializer::from_slice(raw);
    let result = Strict::deserialize(&mut d).map_err(|e| e.to_string())?.0;
    d.end().map_err(|e| e.to_string())?;
    Ok(result)
}
pub(super) fn object<'a>(v: &'a Value, label: &str) -> Result<&'a Map<String, Value>> {
    v.as_object()
        .ok_or_else(|| format!("object required: {label}"))
}
pub(super) fn keys(v: &Value, required: &[&str], allowed: &[&str], label: &str) -> Result<()> {
    let o = object(v, label)?;
    require(
        required.iter().all(|k| o.contains_key(*k)),
        format!("missing field: {label}"),
    )?;
    require(
        o.keys().all(|k| allowed.contains(&k.as_str())),
        format!("unknown field: {label}"),
    )
}
pub(super) fn integer(v: &Value, lo: i64, hi: i64, label: &str) -> Result<i64> {
    v.as_i64()
        .filter(|n| *n >= lo && *n <= hi)
        .ok_or_else(|| format!("integer range/type: {label}"))
}
pub(super) fn strings(v: &Value, label: &str) -> Result<()> {
    require(
        v.as_array().is_some_and(|a| a.iter().all(Value::is_string)),
        format!("string array: {label}"),
    )
}
pub(super) fn colors(v: &Value, names: bool) -> Result<()> {
    let a = v.as_array().ok_or("colors must be array")?;
    for c in a {
        if names {
            require(
                c.as_str().is_some_and(|s| {
                    ["Red", "Blue", "Yellow", "Green", "White", "Black", "Purple"].contains(&s)
                }),
                "color name",
            )?;
        } else {
            integer(c, 0, 6, "color")?;
        }
    }
    Ok(())
}
pub(super) fn evo(v: &Value) -> Result<()> {
    for e in v.as_array().ok_or("evo array")? {
        keys(
            e,
            &["card_color", "level", "memory_cost"],
            &["card_color", "level", "memory_cost"],
            "evo",
        )?;
        integer(&e["card_color"], 0, 6, "evo color")?;
        integer(&e["level"], 0, 255, "evo level")?;
        integer(&e["memory_cost"], 0, 65535, "evo cost")?;
    }
    Ok(())
}
pub(super) fn dna_wire(v: &Value) -> Result<()> {
    for d in v.as_array().ok_or("dna array")? {
        keys(
            d,
            &["requirement1", "requirement2", "memory_cost"],
            &["requirement1", "requirement2", "memory_cost"],
            "dna",
        )?;
        integer(&d["memory_cost"], -32768, 32767, "dna cost")?;
        for name in ["requirement1", "requirement2"] {
            let p = &d[name];
            keys(
                p,
                &["level", "card_colors", "name_contains", "text_contains"],
                &["level", "card_colors", "name_contains", "text_contains"],
                "dna requirement",
            )?;
            integer(&p["level"], 0, 255, "dna level")?;
            let cs = p["card_colors"].as_array().ok_or("dna colors array")?;
            colors(&p["card_colors"], cs.first().is_some_and(Value::is_string))?;
            require(
                p["name_contains"].is_string() && p["text_contains"].is_string(),
                "dna text",
            )?;
        }
    }
    Ok(())
}
pub(super) fn dual_wire(v: &Value) -> Result<()> {
    if v.is_null() {
        return Ok(());
    }
    keys(v, &["digimon", "option"], &["digimon", "option"], "dual")?;
    let d = &v["digimon"];
    let o = &v["option"];
    let dk = [
        "level",
        "dp",
        "colors",
        "traits",
        "evo_costs",
        "effect_text",
        "inherited_text",
        "keywords",
    ];
    let ok = [
        "use_cost",
        "colors",
        "effect_text",
        "security_text",
        "keywords",
    ];
    keys(d, &dk, &dk, "dual digimon")?;
    keys(o, &ok, &ok, "dual option")?;
    integer(&d["level"], 0, 255, "dual level")?;
    integer(&d["dp"], 0, i32::MAX.into(), "dual dp")?;
    integer(&o["use_cost"], 0, 65535, "dual cost")?;
    strings(&d["traits"], "dual traits")?;
    evo(&d["evo_costs"])?;
    for face in [d, o] {
        colors(&face["colors"], true)?;
        require(face["effect_text"].is_string(), "dual text")?;
        let _: Vec<DiagnosticKeyword> =
            serde_json::from_value(face["keywords"].clone()).map_err(|e| e.to_string())?;
    }
    require(
        d["inherited_text"].is_string() && o["security_text"].is_string(),
        "dual text",
    )
}
pub(super) fn card_suffix(id: &str) -> Result<u16> {
    let (prefix, suffix) = id.split_once('-').ok_or("card ID separator")?;
    require(
        prefix
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_uppercase)
            && prefix
                .bytes()
                .all(|x| x.is_ascii_uppercase() || x.is_ascii_digit())
            && !suffix.is_empty()
            && suffix.bytes().all(|x| x.is_ascii_digit()),
        "canonical card ID",
    )?;
    suffix.parse().map_err(|_| "card suffix range".into())
}
pub(super) fn registry_wire(registry: &Value) -> Result<&Map<String, Value>> {
    let fields = [
        "schema_version",
        "registry_version",
        "capacity",
        "padding_index",
        "assignment_count",
        "assignments",
    ];
    keys(registry, &fields, &fields, "registry")?;
    require(
        registry["schema_version"] == json!(1)
            && registry["registry_version"] == json!(1)
            && registry["capacity"] == json!(20000)
            && registry["padding_index"] == json!(0),
        "registry contract",
    )?;
    let assignments = object(&registry["assignments"], "assignments")?;
    require(
        integer(&registry["assignment_count"], 1, 20000, "assignment_count")? as usize
            == assignments.len(),
        "registry count",
    )?;
    let mut indices = BTreeSet::new();
    for (id, n) in assignments {
        card_suffix(id)?;
        require(
            indices.insert(integer(n, 1, 20000, "registry index")?),
            "duplicate global index",
        )?;
    }
    Ok(assignments)
}
pub(super) fn wire_row(id: &str, row: &Value, assignments: &Map<String, Value>) -> Result<()> {
    let required: Vec<_> = RAW
        .iter()
        .copied()
        .filter(|k| !OPTIONAL.contains(k))
        .chain(["card_index"])
        .collect();
    let allowed: Vec<_> = RAW
        .iter()
        .chain(COMPAT)
        .chain(META)
        .copied()
        .chain(["card_index"])
        .collect();
    keys(row, &required, &allowed, id)?;
    require(row["card_id"].as_str() == Some(id), "key/card_id differs")?;
    for k in [
        "card_name_eng",
        "card_effect_class_name",
        "effect_description_eng",
        "inherited_effect_description_eng",
        "security_effect_description_eng",
        "link_effect_description_eng",
        "card_name_jpn",
        "xros_req",
    ] {
        if let Some(v) = row.get(k) {
            require(v.is_string(), format!("string required {id}.{k}"))?;
        }
    }
    for k in ["type_eng", "form_eng", "attribute_eng", "digixros_aliases"] {
        if let Some(v) = row.get(k) {
            strings(v, k)?;
        }
    }
    for (k, hi) in [
        ("level", 255),
        ("dp", i32::MAX.into()),
        ("play_cost", 65535),
    ] {
        if !row[k].is_null() {
            integer(&row[k], -1, hi, k)?;
        }
    }
    integer(&row["card_kind"], 0, 4, "kind")?;
    colors(&row["card_colors"], false)?;
    evo(&row["evo_costs"])?;
    if let Some(v) = row.get("dna_costs") {
        dna_wire(v)?;
    }
    if let Some(v) = row.get("dual") {
        dual_wire(v)?;
    }
    if let Some(v) = row.get("ace_overflow") {
        if !v.is_null() {
            integer(v, i32::MIN.into(), i32::MAX.into(), "overflow")?;
        }
    }
    if let Some(v) = row.get("rarity") {
        integer(v, 0, 5, "rarity")?;
    }
    if let Some(v) = row.get("max_count_in_deck") {
        integer(v, 0, 65535, "max count")?;
    }
    if let Some(v) = row.get("digixros_costs") {
        require(v.is_array(), "digixros_costs array")?;
    }
    let index = integer(&row["index"], 1, 20000, "index")?;
    require(
        assignments.get(id) == Some(&json!(index)),
        "registry identity differs",
    )?;
    require(
        integer(&row["card_index"], 0, 65535, "card_index")? == i64::from(card_suffix(id)?),
        "numeric suffix differs",
    )?;
    require(
        row["norm_id"].as_f64() == Some(index as f64 / 20000.0),
        "normal producer norm differs",
    )
}
pub(super) fn checked_pack(bytes: &[u8], raw: &dyn RawRustRegistry) -> Result<CardPack> {
    let pack = CardPack::from_bytes(bytes)?;
    require(
        pack.manifest.pack_version == digimon_dsl::pack::PACK_FORMAT_VERSION,
        "pack format version",
    )?;
    // This reviewed profile has one concrete engine version. Unknown ranges are
    // retained as refusals instead of adding a new permissive semver evaluator.
    require(
        pack.manifest.min_engine_version == env!("CARGO_PKG_VERSION")
            && pack
                .manifest
                .max_engine_version
                .as_deref()
                .is_none_or(|v| v == env!("CARGO_PKG_VERSION")),
        "unreviewed pack engine range",
    )?;
    let ids: BTreeSet<_> = pack.cards.iter().map(|c| &c.card).collect();
    require(ids.len() == pack.cards.len(), "duplicate pack card ID")?;
    require(
        pack.manifest.required_raw_rust_fns
            == digimon_dsl::pack::required_raw_rust_fns_for_cards(&pack.cards),
        "raw function manifest differs from complete IR",
    )?;
    digimon_dsl::CardRegistry::from_pack_bytes_with_raw_registry(bytes, raw)?;
    Ok(pack)
}
