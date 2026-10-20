#[path = "../src/bin/implementation_inventory/core.rs"]
mod core;
use core::{build_inventory_with_references,FileInput};
use serde_json::Value;
fn graph(suffix:&str)->Value {
 let raw=format!("card: TEST-01\nname: Test\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 2\ndp: 3000\n{suffix}");
 serde_json::to_value(build_inventory_with_references(vec!["TEST-01".into()],vec![FileInput{path:"card.yaml".into(),bytes:raw.into_bytes()}]).unwrap()).unwrap()
}
fn has(g:&Value,phase:&str,role:&str,value:&str)->bool {
 g["reference_graph"]["files"][0][phase]["nodes"].as_array().unwrap().iter().any(|n|n["role"]==role&&n["value"]==value)
}
#[test]fn identity_categories_are_not_card_references(){
 let g=graph("form: ST1-02\nattribute: [ST1-03, Vaccine]\neffects: []\n");
 for p in ["authored","compiled"] {assert!(has(&g,p,"CARD_FORM_DEFINITION","ST1-02"));assert!(has(&g,p,"CARD_ATTRIBUTE_DEFINITION","ST1-03"));assert!(!has(&g,p,"MATCH_CARD_ID","ST1-02"));}
}
#[test]fn keyword_modifier_and_expiry_names_require_engine_witness(){
 let g=graph("effects:\n  - kind: grant_keyword\n    keyword: ST1-02\n  - when: on_play\n    process:\n      - add_modifier: {target: self, modifier: ST1-03, value: 1, expiry: ST1-04}\n      - grant_keyword: {target: self, keyword: Rush, expiry: end_of_turn}\n");
 for p in ["authored","compiled"] {for (role,v) in [("ENGINE_KEYWORD_NAME","ST1-02"),("ENGINE_MODIFIER_NAME","ST1-03"),("ENGINE_EXPIRY_NAME","ST1-04")] {assert!(has(&g,p,role,v),"{role} {p} {g}");assert!(g["reference_graph"]["files"][0][p]["blockers"].as_array().unwrap().iter().any(|b| b==&format!("UNRESOLVED_{role}")));}}
 assert_eq!(g["reference_graph"]["reference_closure_complete"],false);
}
#[test]fn binding_predicates_are_instance_slots_even_if_card_shaped(){
 let g=graph("use_requirement: {all_of: [{binding_present: ST1-02}, {binding_exists: ST1-02}, {binding_absent: ST1-03}, {can_digivolve_onto: ST1-04}]}\neffects: []\n");
 for p in ["authored","compiled"] {assert!(has(&g,p,"EXISTING_INSTANCE_BINDING","ST1-02"));assert!(has(&g,p,"EXISTING_INSTANCE_BINDING","ST1-04"));assert!(!has(&g,p,"MATCH_CARD_ID","ST1-02"));}
}
#[test]fn opaque_hook_token_and_actual_card_predicates_remain_distinct(){
 let g=graph("use_requirement: {card_number_is: ST1-02}\neffects:\n  - when: on_play\n    process:\n      - raw_rust: {fn: ST1-02, consumes: [], binds: []}\n      - play_token: {controller: you, token_name: petrification}\n");
 assert!(has(&g,"compiled","MATCH_CARD_ID","ST1-02"));assert!(has(&g,"compiled","HOOK_STEP","ST1-02"));assert!(has(&g,"compiled","TOKEN_CONSTRUCTION","petrification"));let opaque=graph("effects:\n  - when: on_play\n    process:\n      - if:\n          condition: {future_unknown: ST1-02}\n          then: [{gain_memory: 1}]\n");assert_eq!(opaque["reference_graph"]["traversal_complete"],false);assert_eq!(g["reference_graph"]["runtime_admission"],false);
}
