#![cfg(all(feature = "dsl-yaml-loader", feature = "test-helpers"))]
#[path = "support/named_material_kind_witness.rs"]
mod witness;
use serde_json::{json, Value};
fn row(id: &str, name: &str, kind: u8, index: u16) -> Value {
    json!({"card_id":id,"card_name_eng":name,"card_kind":kind,"index":index,"card_colors":[0]})
}
fn bytes(id: &str, name: &str, kind: u8) -> Vec<u8> {
    serde_json::to_vec(&json!({id:row(id,name,kind,1)})).unwrap()
}
#[test]
fn native_complete_sets_and_kind_exclusion() {
    for (id, k, equal) in [
        ("W-D", 0, true),
        ("W-T", 1, false),
        ("W-O", 2, false),
        ("W-DUAL", 4, true),
    ] {
        let b = bytes(id, "Titamon", k);
        let p = witness::prove(&b, &witness::sha(&b), &["Titamon"]).unwrap();
        assert_eq!(p["names"][0]["equal"], equal);
        assert_eq!(p["runtime_admission"], false);
    }
}
#[test]
fn malformed_original_wire_refused_before_runtime_fallback() {
    for k in [
        json!(5),
        json!(255),
        json!(true),
        json!(0.0),
        json!(null),
        json!("0"),
    ] {
        let mut v = json!({"W":row("W","Titamon",0,1)});
        v["W"]["card_kind"] = k;
        let b = serde_json::to_vec(&v).unwrap();
        assert!(witness::strict_records(&b, &witness::sha(&b)).is_err());
    }
    for b in [
        br#"{"W":{},"W":{}}"#.as_slice(),
        br#"{"W":{"card_id":"W","card_kind":0,"card_kind":1,"index":1}}"#.as_slice(),
    ] {
        assert!(witness::strict_records(b, &witness::sha(b)).is_err());
    }
    let b = serde_json::to_vec(&json!({"W":row("X","Titamon",0,1)})).unwrap();
    assert!(witness::strict_records(&b, &witness::sha(&b)).is_err());
    let b = serde_json::to_vec(&json!({"W":row("W","Titamon",0,1),"X":row("X","x",0,1)})).unwrap();
    assert!(witness::strict_records(&b, &witness::sha(&b)).is_err());
    let b = bytes("W", "Titamon", 0);
    assert!(witness::strict_records(&b, &"0".repeat(64)).is_err());
}
#[test]
#[ignore = "requires an explicit pinned runtime and a new diagnostic output path"]
fn actual_full_runtime_diagnostic() {
    let path = std::env::var("NAMED_MATERIAL_RUNTIME")
        .expect("explicit diagnostic requires NAMED_MATERIAL_RUNTIME");
    let b = std::fs::read(path).unwrap();
    let expected = std::env::var("NAMED_MATERIAL_RUNTIME_SHA").unwrap();
    let registry = digimon_engine::dsl_registry::from_embedded_cached().unwrap();
    use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCardKind, CompiledPredicate};
    let card = registry.lookup("BT24-081").unwrap();
    let assembly: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Assembly)
        .collect();
    assert_eq!(assembly.len(), 1);
    assert_eq!(assembly[0].materials.len(), 2);
    for (m, name) in assembly[0]
        .materials
        .iter()
        .zip(["Titamon", "SkullBaluchimon"])
    {
        assert_eq!(
            m.filter,
            CompiledPredicate {
                name_is: Some(name.into()),
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            }
        );
    }
    let result = witness::prove(&b, &expected, &["Titamon", "SkullBaluchimon"]);
    let succeeded = result.is_ok();
    let out = json!({"runtime_sha256":witness::sha(&b),"result":result,"binary_sha256":witness::sha(&std::fs::read(std::env::current_exe().unwrap()).unwrap()),"features":["dsl-yaml-loader","test-helpers"],"fresh_process_required":true,"admission":false});
    let dest = std::env::var("NAMED_MATERIAL_OUTPUT").unwrap();
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dest)
        .unwrap()
        .write_all(&serde_json::to_vec_pretty(&out).unwrap())
        .unwrap();
    // Keep the refusal artifact for diagnosis, but never report a failed proof
    // as a passing validation command.
    assert!(succeeded, "native proof refused; see saved diagnostic output");
}
#[test]
fn exact_alias_and_nonmatching_routes() {
    let cases = [
        ("W-SUB", "SuperTitamon", None, false),
        ("W-CASE", "titamon", None, false),
        ("AD1-021", "Marcus", None, true),
        ("W-XROS", "Other", Some("Marcus Damon"), false),
    ];
    for (id, name, alias, matched) in cases {
        let mut v = json!({id:row(id,name,1,1)});
        if let Some(a) = alias {
            v[id]["digixros_aliases"] = json!([a]);
        }
        let b = serde_json::to_vec(&v).unwrap();
        let target = if id.starts_with("W-S") || id == "W-CASE" {
            "Titamon"
        } else {
            "Marcus Damon"
        };
        let p = witness::prove(&b, &witness::sha(&b), &[target]).unwrap();
        assert_eq!(
            !p["names"][0]["name_only"].as_array().unwrap().is_empty(),
            matched
        );
        if matched {
            assert_eq!(p["names"][0]["equal"], false);
        }
    }
}
#[test]
fn cached_same_length_name_change_keeps_exact_name_sets_separate() {
    let a = bytes("W-CACHE-LENGTH", "Titamon", 0);
    let b = bytes("W-CACHE-LENGTH", "Abcdefg", 0);
    let original = witness::prove(&a, &witness::sha(&a), &["Titamon"]).unwrap();
    assert!(!original["names"][0]["name_only"].as_array().unwrap().is_empty());
    let changed = witness::prove(&b, &witness::sha(&b), &["Titamon"]).unwrap();
    assert!(changed["names"][0]["name_only"].as_array().unwrap().is_empty());
    assert_eq!(witness::prove(&a, &witness::sha(&a), &["Titamon"]).unwrap(), original);
}
#[test]
fn original_token_collision_is_refused() {
    let registry = digimon_engine::token_registry::build_registry();
    let token = registry.iter().next().unwrap();
    let b = bytes(&token.card_id, "Titamon", 0);
    assert!(witness::prove(&b, &witness::sha(&b), &["Titamon"]).is_err());
}
