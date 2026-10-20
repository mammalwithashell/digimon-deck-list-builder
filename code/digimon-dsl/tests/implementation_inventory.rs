#[path = "../src/bin/implementation_inventory/core.rs"]
mod core;
use core::{build_inventory, FileInput};
fn f(path: &str, raw: &[u8]) -> FileInput { FileInput { path: path.into(), bytes: raw.to_vec() } }
const CARD: &[u8] = b"card: ST1-02\nname: Biyomon\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 2\ndp: 3000\neffects: []\n";
#[test]
fn parsed_empty_effects_are_never_vanilla_or_complete() {
 let r=build_inventory(vec!["ST1-02".into()],vec![f("st1/ST1-02.yaml",CARD)]).unwrap();
 assert!(!r.reference_visitor_complete);assert_eq!(r.cards[0].status,"COMPILED_REFERENCE_UNRESOLVED");
 assert!(r.files[0].parsed.is_some());assert!(r.files[0].compiled.is_some());
 assert_eq!(r.files[0].input.bytes,CARD);assert_eq!(r.requested_ids,["ST1-02"]);
}
#[test]
fn missing_and_duplicate_ids_remain_visible() {
 let r=build_inventory(vec!["ST1-02".into(),"ST1-03".into()],vec![f("a.yaml",CARD),f("b.yml",CARD)]).unwrap();
 assert_eq!(r.cards[0].status,"DUPLICATE_DEFINITION");assert_eq!(r.cards[0].file_indices.len(),2);
 assert_eq!(r.cards[1].status,"MISSING_DEFINITION");assert_eq!(r.files.len(),2);
}
#[test]
fn parse_failures_and_unselected_inventory_are_retained() {
 let r=build_inventory(vec!["ST1-03".into()],vec![f("a.yaml",CARD),f("b.yml",b"invalid: [")]).unwrap();
 assert!(r.files[0].compiled.is_none());assert!(r.files[0].parsed.is_some());
 assert!(r.files[1].error.is_some());assert_eq!(r.cards[0].status,"MISSING_DEFINITION");
 assert!(r.blockers.contains(&"NATIVE_PARSE_FAILURE".to_string()));
}
#[test]
fn ids_paths_and_duplicate_input_paths_refuse() {
 for path in ["../a.yaml","C:/a.yaml","a//b.yaml","./a.yaml","a.json","a\\b.yaml"] {
  assert!(build_inventory(vec!["ST1-02".into()],vec![f(path,CARD)]).is_err());
 }
 assert!(build_inventory(vec!["ST1-02".into(),"ST1-02".into()],vec![]).is_err());
 assert!(build_inventory(vec!["ST1-02".into()],vec![f("a.yaml",CARD),f("a.yaml",CARD)]).is_err());
}
#[test]
fn deterministic_order_preserves_requested_order() {
 let a=build_inventory(vec!["ST1-03".into(),"ST1-02".into()],vec![f("b.yaml",CARD),f("a.yml",CARD)]).unwrap();
 let b=build_inventory(vec!["ST1-03".into(),"ST1-02".into()],vec![f("a.yml",CARD),f("b.yaml",CARD)]).unwrap();
 assert_eq!(serde_json::to_value(a).unwrap(),serde_json::to_value(b).unwrap());
}
#[test]
fn actual_dual_both_faces_preserved_despite_empty_top_level() {
 let raw=include_bytes!("../../digimon-engine/cards/bt25/BT25-085.yaml");
 let r=build_inventory(vec!["BT25-085".into()],vec![f("bt25/BT25-085.yaml",raw)]).unwrap();
 assert_eq!(r.cards[0].status,"COMPILED_REFERENCE_UNRESOLVED");
 assert!(!r.files[0].parsed.as_ref().unwrap()["dual"].is_null());
 assert!(!r.files[0].compiled.as_ref().unwrap()["dual"].is_null());
}
