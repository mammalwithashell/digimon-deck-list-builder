//! Diagnostic only: full-population native name/kind correspondence. No admission.
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

pub fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// Duplicate-key rejection applies recursively, before the permissive runtime loader.
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "unique-key JSON")
            }
            fn visit_bool<E: de::Error>(self, x: bool) -> Result<Strict, E> {
                Ok(Strict(json!(x)))
            }
            fn visit_i64<E: de::Error>(self, x: i64) -> Result<Strict, E> {
                Ok(Strict(json!(x)))
            }
            fn visit_u64<E: de::Error>(self, x: u64) -> Result<Strict, E> {
                Ok(Strict(json!(x)))
            }
            fn visit_f64<E: de::Error>(self, x: f64) -> Result<Strict, E> {
                Ok(Strict(json!(x)))
            }
            fn visit_str<E: de::Error>(self, x: &str) -> Result<Strict, E> {
                Ok(Strict(json!(x)))
            }
            fn visit_string<E: de::Error>(self, x: String) -> Result<Strict, E> {
                Ok(Strict(json!(x)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = vec![];
                while let Some(x) = a.next_element::<Strict>()? {
                    v.push(x.0)
                }
                Ok(Strict(Value::Array(v)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut m = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if m.contains_key(&k) {
                        return Err(de::Error::custom("duplicate key"));
                    }
                    m.insert(k, a.next_value::<Strict>()?.0);
                }
                Ok(Strict(Value::Object(m)))
            }
        }
        d.deserialize_any(V)
    }
}
pub fn strict_records(bytes: &[u8], expected: &str) -> Result<Value, String> {
    if sha(bytes) != expected {
        return Err("runtime byte pin".into());
    }
    let root = serde_json::from_slice::<Strict>(bytes)
        .map_err(|e| e.to_string())?
        .0;
    let rows = root.as_object().ok_or("runtime must be object")?;
    if rows.is_empty() {
        return Err("empty population".into());
    }
    let mut indices = BTreeSet::new();
    for (id, row) in rows {
        let r = row.as_object().ok_or("record object")?;
        if id.is_empty() || r.get("card_id").and_then(Value::as_str) != Some(id.as_str()) {
            return Err("record identity".into());
        }
        let k = r
            .get("card_kind")
            .and_then(Value::as_u64)
            .ok_or("strict original kind")?;
        if k > 4 {
            return Err("unknown original kind".into());
        }
        let index = r
            .get("index")
            .and_then(Value::as_u64)
            .ok_or("strict index")?;
        if index == 0 || index >= 20000 || !indices.insert(index) {
            return Err("invalid/duplicate index".into());
        }
    }
    Ok(root)
}
pub fn prove(bytes: &[u8], expected: &str, names: &[&str]) -> Result<Value, String> {
    use digimon_dsl::compiled::{CompiledCardKind, CompiledPredicate};
    use digimon_engine::{
        card_data::CardData,
        card_store::{build_shared_card_store, shared_card_store},
        debug_runner::DebugRunner,
        dsl_cards::predicate::{eval_predicate, PredicateSubject},
        effect_context::EffectReadContext,
    };
    use std::collections::BTreeMap;
    let original = strict_records(bytes, expected)?;
    if names.is_empty()
        || names.iter().any(|n| n.is_empty())
        || names.iter().collect::<BTreeSet<_>>().len() != names.len()
    {
        return Err("exact unique nonempty names required".into());
    }
    let registry = digimon_engine::dsl_registry::from_embedded_cached()
        .map_err(|e| format!("embedded registry required: {e}"))?;
    let compiled: BTreeMap<_, _> = registry
        .iter()
        .map(|(id, c)| (id.clone(), c.clone()))
        .collect();
    let loaded = CardData::load_from_str(std::str::from_utf8(bytes).map_err(|e| e.to_string())?)?;
    if loaded.len() != original.as_object().unwrap().len() {
        return Err("loader membership".into());
    }
    let tokens = digimon_engine::token_registry::build_registry();
    for t in tokens.iter() {
        if loaded.contains_key(&t.card_id) {
            return Err(format!("token collision: {}", t.card_id));
        }
    }
    let fresh = build_shared_card_store(&loaded);
    if fresh.index.len() != fresh.data.len() {
        return Err("store index collision".into());
    }
    for (i, d) in fresh.data.iter().enumerate() {
        if fresh.index.get(&d.card_id) != Some(&i) {
            return Err("store index bijection".into());
        }
    }
    let cached = shared_card_store(&loaded);
    let store_json = serde_json::to_value(fresh.data.as_ref()).map_err(|e| e.to_string())?;
    if serde_json::to_value(cached.data.as_ref()).map_err(|e| e.to_string())? != store_json
        || cached.index != fresh.index
        || cached.alt_paths != fresh.alt_paths
    {
        return Err("cached versus fresh enriched store mismatch".into());
    }
    let mut runner = DebugRunner::builder().with_card_data(loaded).build();
    if serde_json::to_value(&*runner.game.card_data).map_err(|e| e.to_string())? != store_json {
        return Err("Game store mismatch".into());
    }
    let mut rows = vec![];
    for (i, d) in fresh.data.iter().enumerate() {
        runner.game.stage_inject_card(0, &d.card_id, "trash")?;
        let h = runner.game.players[0]
            .trash
            .last()
            .ok_or("trash insertion")?
            .handle();
        if serde_json::to_value(runner.game.card(h)).map_err(|e| e.to_string())? != store_json[i] {
            return Err("trash handle points at different card".into());
        }
        let rctx = EffectReadContext::new(&runner.game, h, None, 0);
        let mut matches = vec![];
        for name in names {
            let plain = CompiledPredicate {
                name_is: Some((*name).into()),
                ..Default::default()
            };
            let typed = CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..plain.clone()
            };
            let n = eval_predicate(&plain, &rctx, PredicateSubject::Card(h));
            let k = eval_predicate(&typed, &rctx, PredicateSubject::Card(h));
            let reason = if d.card_name == *name {
                Some("printed_name")
            } else if d.also_treated_as.iter().any(|a| a == name) {
                Some("static_alias")
            } else {
                None
            };
            if n != reason.is_some() {
                return Err("native name matcher and recorded reason disagree".into());
            }
            matches.push(json!({"name":name,"name_only":n,"name_plus_digimon":k,"reason":reason}));
        }
        rows.push(json!({"id":d.card_id,"store_index":i,"kind":d.card_kind,"name":d.card_name,"static_aliases":d.also_treated_as,"origin":if original.get(&d.card_id).is_some(){"original_runtime"}else{"generated_token"},"matches":matches}));
    }
    let summaries:Vec<_>=names.iter().enumerate().map(|(i,n)|{
        let a:Vec<_>=rows.iter().filter(|r|r["matches"][i]["name_only"]==true).map(|r|r["id"].clone()).collect();
        let b:Vec<_>=rows.iter().filter(|r|r["matches"][i]["name_plus_digimon"]==true).map(|r|r["id"].clone()).collect();
        json!({"name":n,"equal":a==b,"zero_matches":a.is_empty(),"name_only":a,"name_plus_digimon":b})
    }).collect();
    let blocked = summaries
        .iter()
        .any(|s| s["equal"] != true || s["zero_matches"] == true);
    Ok(
        json!({"version":1,"runtime_sha256":expected,"original_records":original,"enriched_store":store_json,"compiled_registry":compiled,"alt_paths":fresh.alt_paths.as_ref(),"members":rows,"names":summaries,"set_correspondence_blocked":blocked,"source_authority":false,"runtime_admission":false,"executable_closure":false,"engine_behavior_proven":false,"cache_comparison":"fresh_equals_cached_and_game","subject":"ordinary_own_trash_card"}),
    )
}
