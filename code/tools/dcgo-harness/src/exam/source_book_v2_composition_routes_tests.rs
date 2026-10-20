use super::*;
const LINK: &str = include_str!("../../../../tests/fixtures/card_data/source_book_v2_composition_routes/BT21-074-original-row.json");
const BLOCKED: &str = include_str!("../../../../tests/fixtures/card_data/source_book_v2_composition_routes/BT24-081-original-row.json");
const PAIRED: &str = include_str!(
    "../../../../tests/fixtures/card_data/source_book_v2_composition_routes/EX1-066-fresh-row.json"
);
const FRESH: &str = include_str!("../../../../tests/fixtures/card_data/source_book_v2_composition_routes/fresh-six-review-book.json");

#[test]
fn shared_fresh_six_book_keeps_full_unknown_inventory() {
    let book = load_review_v2(FRESH.as_bytes(), &sha(FRESH.as_bytes())).unwrap();
    assert_eq!(book.card_count(), 4484);
    assert_eq!(book.clause_ids().len(), 22);
    assert_eq!(book.card_clause_count("BT24-081").unwrap(), None);
    assert_eq!(book.card_clause_count("BT21-071").unwrap(), Some(4));
    assert_eq!(book.card_clause_count("EX1-066").unwrap(), Some(3));
    assert!(!book.is_recordable());
}

#[test]
fn paired_original_consistency_rejects_rehashed_binding_mutations() {
    for change in [
        "original_text",
        "correction_value",
        "image_association",
        "mirror_duplicate",
        "sidecar",
        "images_empty",
        "image_single",
        "image_duplicate",
    ] {
        let mut row: Value = serde_json::from_str(PAIRED).unwrap();
        let source = &mut row["source_composition"];
        match change {
            "original_text" => {
                source["correction_binding"]["original_sections"][0]["text"] = json!("changed")
            }
            "correction_value" => {
                source["correction_binding"]["corrections"][0]["value"] = json!("changed")
            }
            "image_association" => {
                source["correction_binding"]["images"][0]["official_observation_id"] =
                    json!("foreign")
            }
            "sidecar" => source["correction_binding"]["sidecar_sha256"] = json!("0".repeat(64)),
            "images_empty" => source["correction_binding"]["images"] = json!([]),
            "image_single" => {
                source["correction_binding"]["images"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            "image_duplicate" => {
                source["correction_binding"]["images"][1] =
                    source["correction_binding"]["images"][0].clone()
            }
            "mirror_duplicate" => {
                let mirror = source["structural_obligations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|o| o["kind"] == "paired-main-face-consistency")
                    .unwrap()
                    .clone();
                source["structural_obligations"]
                    .as_array_mut()
                    .unwrap()
                    .push(mirror);
            }
            _ => unreachable!(),
        }
        source["zones"]["effect"]["source_binding"] = source["correction_binding"].clone();
        assert!(
            paired_main_mirror(&row, row["source_composition"]["units"].as_array().unwrap())
                .is_err(),
            "{change}"
        );
    }
}

#[test]
fn link_condition_and_folded_dp_bindings_reject_mutations() {
    for change in [
        "members",
        "projection",
        "observation",
        "dp_value",
        "dp_role",
        "obligation",
    ] {
        let mut row: Value = serde_json::from_str(LINK).unwrap();
        let source = &mut row["source_composition"];
        match change {
            "members" => source["condition_binding"]["candidate_values"] = json!({}),
            "projection" => source["condition_binding"]["projection"] = json!("free"),
            "observation" => {
                source["condition_binding"]["evidence_observation_ids"] = json!(["foreign"])
            }
            "dp_value" => source["structural_obligations"][0]["value"] = json!(3000),
            "dp_role" => source["units"][1]["role"] = json!("effect"),
            "obligation" => {
                source["structural_obligations"]
                    .as_array_mut()
                    .unwrap()
                    .remove(0);
            }
            _ => unreachable!(),
        }
        source["zones"]["conditions"]["source_binding"] = source["condition_binding"].clone();
        assert!(link_correspondence(&row).is_err(), "{change}");
    }
}

#[test]
fn original_link_occurrences_keep_dp_structural() {
    let row: Value = serde_json::from_str(LINK).unwrap();
    let found = occurrences("BT21-074", &row).unwrap();
    assert_eq!(found.len(), 4);
    assert_eq!(found.iter().filter(|o| o["zone"] == "link").count(), 1);
    assert!(found
        .iter()
        .all(|o| !o["span"][3].as_str().unwrap().contains("+4000 DP")));
}

#[test]
fn retained_blocked_span_count_is_not_a_clause_count() {
    let row: Value = serde_json::from_str(BLOCKED).unwrap();
    assert_eq!(row["span_count"], 7);
    let mut book: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/card_data/source_book_v2_rust/fresh-four-review-book.json"
    ))
    .unwrap();
    let diagnostic = &mut book["diagnostic_evidence"];
    diagnostic["cards"]["BT24-081"] = row;
    let selected = diagnostic["evaluated_card_ids"].as_array_mut().unwrap();
    selected.push(json!("BT24-081"));
    selected.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    validate_inventory(diagnostic).unwrap();
    // Only the inventory validator is invoked; no review book is fabricated.
    assert_eq!(diagnostic["cards"]["BT24-081"]["status"], "BLOCKED");
}
