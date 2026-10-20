//! Wire contract shared with tools.clause_coverage.semantic_fingerprint.
use serde_json::Value;

pub const VERSION: u64 = 1;

pub fn normalize(value: &Value) -> Result<Value, String> {
    match value {
        Value::String(text) => Ok(Value::String(text.replace("\r\n", "\n").replace('\r', "\n"))),
        Value::Array(values) => values.iter().map(normalize).collect::<Result<Vec<_>, _>>().map(Value::Array),
        Value::Null => Ok(Value::Null),
        Value::Number(n) if n.is_i64() || n.is_u64() => Ok(value.clone()),
        _ => Err("semantic payload must contain only arrays, strings, integers and null".into()),
    }
}

pub fn digest(payload: &Value) -> Result<String, String> {
    let bytes = serde_json::to_string(&normalize(payload)?).map_err(|e| e.to_string())?;
    Ok(super::verdict::sha256_hex(&bytes))
}

/// Fresh synthetic source fixtures only. Never called by production or by the
/// legacy-preservation tests; archived evidence is never rebound this way.
#[cfg(test)]
pub(crate) fn fresh_clause_json(text: &str) -> String {
    let mut doc: Value = serde_json::from_str(text).unwrap();
    let rows = doc["clauses"].as_array_mut().unwrap();
    for row in rows.iter_mut() {
        let id = row["id"].as_str().unwrap().to_string();
        let parts: Vec<_> = id.split('#').collect();
        let obj = row.as_object_mut().unwrap();
        obj.entry("card_id").or_insert(serde_json::json!(parts[0]));
        obj.entry("zone").or_insert(serde_json::json!(parts[1]));
        obj.entry("kind").or_insert(serde_json::json!("untimed"));
        obj.entry("timings").or_insert(serde_json::json!([]));
        obj.entry("keyword").or_insert(Value::Null);
        obj.entry("text").or_insert(serde_json::json!(""));
    }
    let source = rows.clone();
    for row in rows.iter_mut() {
        let mut zone: Vec<_> = source.iter().filter(|other|
            other["card_id"] == row["card_id"] && other["zone"] == row["zone"]).collect();
        zone.sort_by_key(|other| other["id"].as_str().unwrap().split('#').last().unwrap().parse::<usize>().unwrap());
        let span = |r: &Value| serde_json::json!([r["kind"],r["timings"],r["keyword"],r["text"]]);
        let index = row["id"].as_str().unwrap().split('#').last().unwrap().parse::<usize>().unwrap();
        let spans: Vec<_> = zone.into_iter().map(span).collect();
        let payload = normalize(&serde_json::json!(["digimon-clause-semantics",1,row["card_id"],row["zone"],index,span(row),spans])).unwrap();
        row["semantic_fingerprint_version"] = serde_json::json!(1);
        row["semantic_sha256"] = serde_json::json!(digest(&payload).unwrap());
        row["semantic_payload"] = payload;
    }
    doc.to_string()
}

#[cfg(test)]
pub(crate) fn fresh_interaction_json(text: &str) -> String {
    let mut doc: Value = serde_json::from_str(text).unwrap();
    for (id, row) in doc["interactions"].as_object_mut().unwrap() {
        let mut cards: Vec<_> = row["card_ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        cards.sort();
        let cards = serde_json::json!(cards);
        let payload = if row["source"] == "qa" {
            row["q_id"] = serde_json::json!(id.strip_prefix("qa:").unwrap());
            serde_json::json!(["digimon-qa-semantics",1,row["q_id"],cards,row["kind"],row["text_sha256"]])
        } else {
            serde_json::json!(["digimon-interaction-semantics",1,id,cards,"probe",row["kind"],row["family"],row["family_version"],super::verdict::sha256_hex(row["text_sha256"].as_str().unwrap_or(""))])
        };
        row["semantic_fingerprint_version"] = serde_json::json!(1);
        row["semantic_sha256"] = serde_json::json!(digest(&payload).unwrap());
        row["semantic_payload"] = payload;
    }
    doc.to_string()
}

#[cfg(test)]
mod tests {
    fn vectors() -> serde_json::Value {
        serde_json::from_str(include_str!("../../../../tests/fixtures/semantic_fingerprints_v1.json")).unwrap()
    }

    fn book_json() -> serde_json::Value {
        let vectors = vectors();
        let rows: Vec<_> = vectors.as_array().unwrap().iter().take(2).map(|v| {
            let p = &v["payload"];
            serde_json::json!({"id": format!("EX12-062#inherited#{}", p[4]),
                "card_id": p[2], "zone": p[3], "kind": p[5][0], "timings": p[5][1],
                "keyword": p[5][2], "text": p[5][3], "semantic_fingerprint_version": 1,
                "semantic_sha256": v["sha256"], "semantic_payload": p})
        }).collect();
        serde_json::json!({"cards": ["EX12-062"], "clauses": rows})
    }

    #[test]
    fn shared_golden_vectors() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/semantic_fingerprints_v1.json"
        )).unwrap();
        for vector in vectors.as_array().unwrap() {
            assert_eq!(super::digest(&vector["payload"]).unwrap(), vector["sha256"].as_str().unwrap());
        }
    }

    #[test]
    fn producer_recomputes_complete_zone_and_preserves_diagnostic_text_hash() {
        use crate::exam::verdict::*;
        let json = book_json();
        let book = ClauseTextBook::from_json(&json.to_string(), "frozen vector").unwrap();
        let mut store = VerdictStore::default();
        let id = "EX12-062#inherited#1";
        record_scenario_verdict(&mut store, &book, id, Verdict::Confirmed, None, None, "now".into()).unwrap();
        let sha = book.semantic_sha(id).unwrap();
        store.set_current_semantic_sha_v1(id, &sha);
        assert!(!store.is_invalidated(id));
        assert!(store.get_semantically_validated_v1(id, &sha).is_some());
        assert_eq!(store.get(id).unwrap().text_sha256, sha256_hex(book.get(id).unwrap().text.as_str()));
        let mut changed = json.clone();
        changed["clauses"][0]["timings"] = serde_json::json!(["On Play"]);
        let stale = ClauseTextBook::from_json(&changed.to_string(), "changed timing").unwrap();
        assert!(stale.semantic_sha(id).is_err());
        let before = store.to_json().unwrap();
        assert!(record_scenario_verdict(&mut store, &stale, id, Verdict::Confirmed, None, None, "later".into()).is_err());
        assert_eq!(store.to_json().unwrap(), before);
    }

    #[test]
    fn incomplete_unknown_or_duplicate_source_books_never_bind() {
        use crate::exam::verdict::ClauseTextBook;
        for damage in ["unknown_version", "missing_payload", "missing_timing", "gap", "duplicate"] {
            let mut source = book_json();
            match damage {
                "unknown_version" => source["clauses"][0]["semantic_fingerprint_version"] = serde_json::json!(99),
                "missing_payload" => { source["clauses"][0].as_object_mut().unwrap().remove("semantic_payload"); },
                "missing_timing" => { source["clauses"][0].as_object_mut().unwrap().remove("timings"); },
                "gap" => { source["clauses"].as_array_mut().unwrap().remove(0); },
                _ => { let duplicate = source["clauses"][0].clone(); source["clauses"].as_array_mut().unwrap().push(duplicate); },
            }
            let outcome = ClauseTextBook::from_json(&source.to_string(), damage)
                .and_then(|book| book.semantic_sha("EX12-062#inherited#1"));
            assert!(outcome.is_err(), "{damage}");
        }
    }

    #[test]
    fn historical_rows_remain_readable_but_are_unmeasured_with_unknown_fields_preserved() {
        use crate::exam::verdict::*;
        for verdict in ["confirmed", "diverged", "unreachable", "unavailable", "unmeasured"] {
            let json = serde_json::json!({"version": 1, "clauses": {"EX12-062#inherited#1": {
                "clause_id": "EX12-062#inherited#1", "card_id": "EX12-062", "verdict": verdict,
                "label": "Draw", "text_sha256": "old-body", "recorded_at": "old", "reason": "original",
                "review_metadata": {"preserve": true}}}});
            let store = VerdictStore::from_json(&json.to_string()).unwrap();
            assert!(store.is_invalidated("EX12-062#inherited#1"));
            let summary = store.summary(&["EX12-062#inherited#1".into()]);
            assert_eq!(summary.total, 1);
            assert_eq!(summary.unmeasured, 1);
            let saved: serde_json::Value = serde_json::from_str(&store.to_json().unwrap()).unwrap();
            assert_eq!(saved["version"], 3);
            assert_eq!(saved["clauses"], json["clauses"]);
        }
    }
}
