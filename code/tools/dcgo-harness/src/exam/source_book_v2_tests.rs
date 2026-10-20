use super::*;
use serde_json::{json, Value};
const BOOK: &str = include_str!(
    "../../../../tests/fixtures/card_data/source_book_v2_rust/fresh-four-review-book.json"
);
const VECTORS: &str =
    include_str!("../../../../tests/fixtures/card_data/source_book_v2/semantic_vectors_v2.json");
fn hash(raw: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(raw))
}
fn load(v: &Value) -> Result<ReviewBookV2, String> {
    let raw = serde_json::to_vec(v).unwrap();
    load_review_v2(&raw, &hash(&raw))
}
#[test]
fn actual_book_retains_every_card_and_is_never_recordable() {
    let b = load_review_v2(BOOK.as_bytes(), &hash(BOOK.as_bytes())).unwrap();
    assert_eq!(b.card_count(), 4484);
    assert_eq!(b.clause_ids().len(), 22);
    assert!(!b.is_recordable());
    assert_eq!(b.card_clause_count("BT24-031").unwrap(), Some(3));
    assert_eq!(b.card_clause_count("BT26-033").unwrap(), Some(9));
    assert_eq!(b.card_clause_count("EX12-031").unwrap(), Some(5));
    assert_eq!(b.card_clause_count("BT1-001").unwrap(), None);
    assert_eq!(b.card_clause_count("LM-063").unwrap(), None);
    assert!(b.card_clause_count("MISSING-1").is_err());
    for id in b.clause_ids() {
        let binding = b.binding(&id).unwrap();
        assert_eq!(binding.semantic_version(), 2);
        assert_eq!(binding.source_generation(), b.source_generation());
    }
    assert_eq!(
        b.evidence()["cards"]["EX12-031"]["superseded_source_occurrences"],
        2
    );
}
#[test]
fn all_twenty_seven_shared_vectors_match_exact_bytes() {
    let v: Value = serde_json::from_str(VECTORS).unwrap();
    assert_eq!(v["vectors"].as_array().unwrap().len(), 27);
    for row in v["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .chain(v["scalar_vectors"].as_array().unwrap())
    {
        let raw = wire_bytes_v2(&row["payload"]).unwrap();
        let hex: String = raw.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, row["utf8_hex"].as_str().unwrap());
        assert_eq!(hash(&raw), row["sha256"].as_str().unwrap());
    }
    for bad in v["rejected_payloads"].as_array().unwrap() {
        assert!(wire_bytes_v2(bad).is_err());
    }
}
#[test]
fn exact_external_byte_pin_is_required() {
    assert!(load_review_v2(BOOK.as_bytes(), &"0".repeat(64)).is_err());
}
#[test]
fn mutated_clause_fields_fail_even_with_new_external_byte_pin() {
    for field in [
        "text",
        "zone",
        "face",
        "kind",
        "unit_key",
        "local_span_index",
        "timings",
        "keyword",
        "semantic_sha256",
        "id",
    ] {
        let mut v: Value = serde_json::from_str(BOOK).unwrap();
        v["clauses"][0][field] = json!("forged");
        assert!(load(&v).is_err(), "{field}");
    }
}
#[test]
fn unknown_and_empty_counts_cannot_be_promoted() {
    for mutation in [0, 1, 2, 3] {
        let mut v: Value = serde_json::from_str(BOOK).unwrap();
        match mutation {
            0 => v["cards"]["BT1-001"]["clause_count"] = json!(0),
            1 => {
                v["cards"].as_object_mut().unwrap().remove("BT1-001");
            }
            2 => v["full_clause_count"] = json!(22),
            _ => v["cards"]["LM-063"]["status"] = json!("COMPILED_REVIEW_ONLY"),
        };
        assert!(load(&v).is_err());
    }
}
#[test]
fn strict_versions_and_duplicate_or_missing_clauses_fail() {
    for mutation in 0..7 {
        let mut v: Value = serde_json::from_str(BOOK).unwrap();
        match mutation {
            0 => v["book_schema_version"] = json!(true),
            1 => v["semantic_fingerprint_version"] = json!(2.0),
            2 => v["semantic_fingerprint_version"] = json!(99),
            3 => {
                v["clauses"].as_array_mut().unwrap().pop();
            }
            4 => {
                let c = v["clauses"][0].clone();
                v["clauses"].as_array_mut().unwrap().push(c);
            }
            5 => v["status"] = json!("ADMITTED"),
            _ => v["source_generation_sha256"] = json!("0".repeat(64)),
        };
        assert!(load(&v).is_err(), "{mutation}");
    }
}
#[test]
fn source_runtime_structural_and_correction_changes_invalidate_closure() {
    for mutation in 0..5 {
        let mut v: Value = serde_json::from_str(BOOK).unwrap();
        let row = &mut v["diagnostic_evidence"]["cards"]["EX12-031"];
        match mutation {
            0 => {
                row["composition_evidence"]["runtime_projection"]["projected_claimed_values"]
                    ["play_cost"] = json!(0)
            }
            1 => row["structural_obligations"] = json!([]),
            2 => row["original_observations"] = json!({}),
            3 => row["source_composition"]["consumption"] = json!([]),
            _ => row["source_composition"]["units"][0]["text"] = json!("changed"),
        };
        assert!(load(&v).is_err(), "{mutation}");
    }
}
#[test]
fn unicode_and_crlf_have_distinct_exact_wire_bytes() {
    assert_ne!(
        wire_bytes_v2(&json!(["x\r\ny"])).unwrap(),
        wire_bytes_v2(&json!(["x\ny"])).unwrap()
    );
    assert!(wire_bytes_v2(&json!([false])).is_err());
}

#[test]
fn duplicate_json_keys_are_refused_before_value_construction() {
    for raw in [
        format!("{{\"book_schema_version\":2,{}", &BOOK[1..]),
        BOOK.replacen(
            "\"clause_count\":",
            "\"clause_count\":null,\"clause_count\":",
            1,
        ),
    ] {
        assert!(load_review_v2(raw.as_bytes(), &hash(raw.as_bytes())).is_err());
    }
}
#[test]
fn malformed_json_and_nested_diagnostic_never_panic() {
    for raw in ["[]", "null", "1", "false", "{}", "{", "{}{}", "{\"x\":NaN}"] {
        assert!(load_review_v2(raw.as_bytes(), &hash(raw.as_bytes())).is_err());
    }
    for bad in [
        Value::Null,
        json!(false),
        json!(2),
        json!([]),
        json!("invalid"),
    ] {
        let mut v: Value = serde_json::from_str(BOOK).unwrap();
        v["diagnostic_evidence"] = bad;
        assert!(load(&v).is_err());
    }
}
#[test]
fn supplied_payload_and_hash_are_not_the_semantic_authority() {
    let mut v: Value = serde_json::from_str(BOOK).unwrap();
    v["clauses"][0]["semantic_payload"][5][3] = json!("forged payload");
    v["clauses"][0]["semantic_sha256"] = json!(hash(
        &wire_bytes_v2(&v["clauses"][0]["semantic_payload"]).unwrap()
    ));
    assert!(load(&v).is_err());
}
#[test]
fn strict_nulls_unknown_fields_and_foreign_occurrences_refuse() {
    for n in 0..5 {
        let mut v: Value = serde_json::from_str(BOOK).unwrap();
        match n {
            0 => v["full_gating_interaction_count"] = json!(false),
            1 => v["recordable"] = json!(true),
            2 => v["cards"]["BT1-001"]["clause_count"] = json!(false),
            3 => {
                v["diagnostic_evidence"]["cards"]["BT24-031"]["source_composition"]["units"][0]
                    ["observation_id"] = json!("foreign")
            }
            _ => v["clauses"][0]["local_span_index"] = json!(false),
        };
        assert!(load(&v).is_err());
    }
}

fn invalid_inventory(case: usize) {
    let mut v: Value = serde_json::from_str(BOOK).unwrap();
    match case {
        0 => {
            v["membership_counts"]["official"] = json!(true);
            v["diagnostic_evidence"]["membership_counts"]["official"] = json!(true);
        }
        1 => {
            v["membership_counts"]["official"] = json!(4473);
            v["diagnostic_evidence"]["membership_counts"]["official"] = json!(4473);
        }
        2 => {
            v["cards"].as_object_mut().unwrap().remove("BT1-001");
            v["diagnostic_evidence"]["cards"]
                .as_object_mut()
                .unwrap()
                .remove("BT1-001");
        }
        3 => {
            v["cards"]["BT1-001"]["membership"]["official"] = json!("yes");
            v["diagnostic_evidence"]["cards"]["BT1-001"]["membership"]["official"] = json!("yes");
        }
        4 => {
            v["cards"]["BT1-001"]["blockers"] = json!("unknown");
            v["diagnostic_evidence"]["cards"]["BT1-001"]["blockers"] = json!("unknown");
        }
        5 => {
            let first = v["diagnostic_evidence"]["evaluated_card_ids"][0].clone();
            v["diagnostic_evidence"]["evaluated_card_ids"]
                .as_array_mut()
                .unwrap()
                .push(first);
        }
        _ => v["diagnostic_evidence"]["evaluated_card_ids"]
            .as_array_mut()
            .unwrap()
            .push(json!("MISSING-999")),
    };
    assert!(load(&v).is_err(), "inventory case {case} accepted");
}
#[test]
fn inventory_count_bool() {
    invalid_inventory(0);
}
#[test]
fn inventory_count_mismatch() {
    invalid_inventory(1);
}
#[test]
fn inventory_drop_unknown() {
    invalid_inventory(2);
}
#[test]
fn inventory_membership_type() {
    invalid_inventory(3);
}
#[test]
fn inventory_blockers_type() {
    invalid_inventory(4);
}
#[test]
fn inventory_duplicate_eval() {
    invalid_inventory(5);
}
#[test]
fn inventory_foreign_eval() {
    invalid_inventory(6);
}
