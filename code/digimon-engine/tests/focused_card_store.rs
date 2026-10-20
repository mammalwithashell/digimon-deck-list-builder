//! Finite inspection fixtures, not a Game/dispatch or semantic admission test.
#![cfg(feature = "dsl-yaml-loader")]

use digimon_dsl::{compiled::*, CardPack};
use digimon_engine::card_store::ScopedDiagnosticStore;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}
fn descriptor(b: &[u8]) -> Value {
    json!({"bytes":b.len(),"sha256":sha(b)})
}

fn row(id: &str, index: u16) -> Value {
    json!({"card_id":id,"card_name_eng":"Fixture","card_effect_class_name":"Fixture",
        "play_cost":3,"dp":2000,"level":3,"card_kind":0,"card_colors":[2,0],
        "type_eng":["Beast","Beast"],"form_eng":["Rookie"],"attribute_eng":["Free"],
        "effect_description_eng":"","inherited_effect_description_eng":"",
        "security_effect_description_eng":"","evo_costs":[],
        "index":index,"norm_id":f64::from(index)/20000.0,
        "card_index":id.split('-').last().unwrap().parse::<u16>().unwrap()})
}
fn compiled(id: &str) -> CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&format!(
        "card: {id}\nname: Fixture\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 2000\nalt_paths:\n  - kind: digivolve\n    from: {{level_eq: 2}}\n    cost: 0\neffects: []\n"
    )).unwrap();
    digimon_dsl::CardRegistry::from_specs("fixture", &[spec])
        .unwrap()
        .lookup(id)
        .unwrap()
        .clone()
}
struct Fixture {
    runtime: Vec<u8>,
    registry: Vec<u8>,
    pack: Vec<u8>,
    binding: Value,
}
impl Fixture {
    fn new() -> Self {
        let runtime = bytes(&json!({"FIX-002":row("FIX-002",91),"FIX-001":row("FIX-001",7)}));
        let registry = bytes(
            &json!({"schema_version":1,"registry_version":1,"capacity":20000,
            "padding_index":0,"assignment_count":4,
            "assignments":{"FIX-001":7,"FIX-002":91,"OUT-001":999,"TOK-001":111}}),
        );
        let pack = CardPack::new(
            "explicit-fixture",
            vec![
                compiled("FIX-002"),
                compiled("FIX-001"),
                compiled("OUT-001"),
            ],
        )
        .to_bytes()
        .unwrap();
        let mut f = Self {
            runtime,
            registry,
            pack,
            binding: json!({
                "kind":"focused-native-binding-v1","schema_version":1,
                "status":"DIAGNOSTIC_ONLY_NOT_RUNTIME","is_recordable":false,
                "contract_sha256":"a".repeat(64),"source_generation_sha256":"b".repeat(64),
                "source_revision":"c".repeat(64),"native_request_sha256":"d".repeat(64),
                "required_card_ids":["FIX-001","FIX-002","QA-001"],
                "executable_card_ids":["FIX-001","FIX-002"],
                "cards":{"FIX-001":{"native_semantics_proven":false},"FIX-002":{"native_semantics_proven":false},"QA-001":{"native_semantics_proven":false}},
                "input_pins":{},"native_pack":{},"native_executable":{},
                "accepted_evidence":{},"consumer_dependencies":{},
                "derived_families":["traits","numeric_sentinels","kind_colors","registry_f32","printed_keywords","raw_digixros_aliases","dual_face_lookup","dsl_enrichment","dsl_dna_restrictions","tokens"],
                "field_slots":["card_id","card_name_eng","card_effect_class_name","play_cost","dp","level","card_kind","card_colors","type_eng","form_eng","attribute_eng","effect_description_eng","inherited_effect_description_eng","security_effect_description_eng","link_effect_description_eng","evo_costs","dna_costs","index","norm_id","ace_overflow","dual","digixros_aliases","xros_req","digixros_costs","max_count_in_deck","card_name_jpn","rarity","card_index"],
                "counts":{"required":3,"executable":2,"registry":4,"inventory":4,"field_slots":56},
                "retained_witness_roles":["book","metadata","records","runtime","registry","native"],
                "unresolved":["ALL_CAPTURED_SEMANTIC_GATES_RETAINED","NO_RUNTIME_HANDLE","TASK_2_3_INCOMPLETE"],
                "admission":{"source":false,"native":false,"runtime":false,"oracle":false,"training":false},
                "projection_epoch":null,"full_clause_count":null,"full_gating_interaction_count":null
            }),
        };
        f.repin();
        f
    }
    // Re-pinning is only synthetic fixture authoring, never an approval operation.
    fn repin(&mut self) {
        self.binding["input_pins"]["runtime"] = descriptor(&self.runtime);
        self.binding["input_pins"]["registry"] = descriptor(&self.registry);
        self.binding["native_pack"] = descriptor(&self.pack);
    }
    fn build(&self) -> Result<ScopedDiagnosticStore, String> {
        self.build_with(&EngineRawRustRegistry::new())
    }
    fn build_with(&self, raw: &EngineRawRustRegistry) -> Result<ScopedDiagnosticStore, String> {
        let b = bytes(&self.binding);
        ScopedDiagnosticStore::from_bound_bytes(
            &b,
            &sha(&b),
            &self.runtime,
            &self.registry,
            &self.pack,
            raw,
        )
    }
    fn runtime_change(&mut self, f: impl FnOnce(&mut Value)) {
        let mut v: Value = serde_json::from_slice(&self.runtime).unwrap();
        f(&mut v);
        self.runtime = bytes(&v);
    }
    fn pack_change(&mut self, f: impl FnOnce(&mut CardPack)) {
        let mut v = CardPack::from_bytes(&self.pack).unwrap();
        f(&mut v);
        self.pack = v.to_bytes().unwrap();
    }
}

#[test]
fn explicit_sparse_store_has_separate_offsets_and_scoped_paths() {
    let f = Fixture::new();
    let s = f.build().unwrap();
    assert_eq!(s.len(), 2);
    assert!(!s.is_empty());
    assert_eq!(s.internal_offset("FIX-001").unwrap(), 0);
    assert_eq!(s.internal_offset("FIX-002").unwrap(), 1);
    assert_eq!(s.card("FIX-002").unwrap().index, 91);
    assert_eq!(
        s.card("FIX-002").unwrap().norm_id.to_bits(),
        (91f32 / 20000f32).to_bits()
    );
    assert_eq!(s.alt_paths("FIX-001").unwrap().len(), 1);
    assert_eq!(s.binding_bytes(), bytes(&f.binding));
}
#[test]
fn q_only_outside_pack_token_and_unknown_all_refuse() {
    let s = Fixture::new().build().unwrap();
    for id in ["QA-001", "OUT-001", "TOK-001", "NOT-001", "option_token"] {
        assert!(s.card(id).is_err(), "{id}");
        assert!(s.alt_paths(id).is_err(), "{id}");
        assert!(s.internal_offset(id).is_err(), "{id}");
    }
}
#[test]
fn same_length_metadata_is_uncached_and_clones_are_read_isolated() {
    let mut f = Fixture::new();
    let first = f.build().unwrap();
    f.runtime_change(|v| v["FIX-001"]["card_name_eng"] = json!("Changed"));
    f.repin();
    let second = f.build().unwrap();
    let clone = first.clone();
    assert_eq!(first.card("FIX-001").unwrap().card_name, "Fixture");
    assert_eq!(second.card("FIX-001").unwrap().card_name, "Changed");
    let mut detached = clone.card("FIX-001").unwrap().clone();
    detached.card_name = "Mutated".into();
    assert_eq!(clone.card("FIX-001").unwrap().card_name, "Fixture");
}
#[test]
fn reordered_input_keeps_internal_offsets_and_global_indices() {
    let mut f = Fixture::new();
    let first = f.build().unwrap();
    f.runtime = format!(
        "{{\"FIX-002\":{},\"FIX-001\":{}}}",
        row("FIX-002", 91),
        row("FIX-001", 7)
    )
    .into_bytes();
    f.repin();
    let second = f.build().unwrap();
    for id in ["FIX-001", "FIX-002"] {
        assert_eq!(first.internal_offset(id), second.internal_offset(id));
        assert_eq!(first.card(id).unwrap(), second.card(id).unwrap());
    }
}
#[test]
fn independently_drifted_inputs_and_binding_refuse() {
    let f = Fixture::new();
    let b = bytes(&f.binding);
    assert!(ScopedDiagnosticStore::from_bound_bytes(
        &b,
        &"0".repeat(64),
        &f.runtime,
        &f.registry,
        &f.pack,
        &EngineRawRustRegistry::new()
    )
    .is_err());
    for role in 0..3 {
        let mut g = Fixture::new();
        match role {
            0 => g.runtime.push(b' '),
            1 => g.registry.push(b' '),
            _ => g.pack.push(0),
        };
        assert!(g.build().is_err(), "role {role}");
    }
}
#[test]
fn runtime_scope_is_exact_not_subset() {
    for add in [false, true] {
        let mut f = Fixture::new();
        f.runtime_change(|v| {
            if add {
                v["OUT-001"] = row("OUT-001", 999)
            } else {
                v.as_object_mut().unwrap().remove("FIX-002");
            }
        });
        f.repin();
        assert!(f.build().is_err());
    }
}
#[test]
fn binding_membership_and_diagnostic_policy_are_not_coerced() {
    for (key, value) in [
        ("executable_card_ids", json!(["FIX-001", "FIX-001"])),
        ("required_card_ids", json!(["FIX-001", "QA-001"])),
        ("is_recordable", json!(true)),
        ("projection_epoch", json!(1)),
        ("full_clause_count", json!(0)),
        ("status", json!("APPROVED")),
        ("derived_families", json!([])),
        ("field_slots", json!([])),
    ] {
        let mut f = Fixture::new();
        f.binding[key] = value;
        assert!(f.build().is_err(), "{key}");
    }
    let mut f = Fixture::new();
    f.binding["admission"]["native"] = json!(true);
    assert!(f.build().is_err());
}
#[test]
fn strict_wire_rejects_duplicates_boolean_sentinels_and_unknown_fields() {
    let mut f = Fixture::new();
    f.runtime = String::from_utf8(f.runtime)
        .unwrap()
        .replacen("\"play_cost\":3", "\"play_cost\":3,\"play_cost\":4", 1)
        .into_bytes();
    f.repin();
    assert!(f.build().is_err());
    for (k, v) in [
        ("level", json!(true)),
        ("play_cost", json!(-2)),
        ("card_kind", json!(7)),
        ("card_colors", json!([true])),
        ("unknown", json!(1)),
        ("norm_id", json!(0.0)),
        ("card_index", json!(99)),
        ("card_id", json!("FIX-002")),
    ] {
        let mut f = Fixture::new();
        f.runtime_change(|r| r["FIX-001"][k] = v);
        f.repin();
        assert!(f.build().is_err(), "{k}");
    }
}
#[test]
fn full_registry_rejects_duplicate_indices_even_outside_scope() {
    let mut f = Fixture::new();
    let mut r: Value = serde_json::from_slice(&f.registry).unwrap();
    r["assignments"]["TOK-001"] = json!(999);
    f.registry = bytes(&r);
    f.repin();
    assert!(f.build().is_err());
}
#[test]
fn wrong_global_index_or_norm_refuses_without_renumbering() {
    for key in ["index", "norm_id"] {
        let mut f = Fixture::new();
        f.runtime_change(|r| r["FIX-002"][key] = json!(1));
        f.repin();
        assert!(f.build().is_err());
    }
}
#[test]
fn corrupt_and_missing_selected_pack_refuse_without_fallback() {
    let mut f = Fixture::new();
    f.pack = b"not a pack".to_vec();
    f.repin();
    assert!(f.build().is_err());
    let mut f = Fixture::new();
    f.pack_change(|p| p.cards.retain(|c| c.card != "FIX-002"));
    f.repin();
    assert!(f.build().is_err());
}
#[test]
fn duplicate_pack_unknown_version_and_selected_token_refuse() {
    for mode in 0..3 {
        let mut f = Fixture::new();
        f.pack_change(|p| match mode {
            0 => p.cards.push(p.cards[0].clone()),
            1 => p.manifest.min_engine_version = "99.0.0".into(),
            _ => {
                p.cards
                    .iter_mut()
                    .find(|c| c.card == "FIX-001")
                    .unwrap()
                    .kind = CompiledCardKind::Token
            }
        });
        f.repin();
        assert!(f.build().is_err(), "mode {mode}");
    }
}
#[test]
fn explicit_real_raw_registry_is_required_and_no_callback_is_dispatched() {
    let mut f = Fixture::new();
    f.pack_change(|p| {
        p.cards[0].alt_paths[0]
            .extra_cost
            .push(CompiledStep::RawRust {
                fn_name: "inspection_only".into(),
                consumes: vec![],
                binds: vec![],
            });
        p.manifest.required_raw_rust_fns =
            digimon_dsl::pack::required_raw_rust_fns_for_cards(&p.cards);
    });
    f.repin();
    assert!(f.build().is_err());
    let mut raw = EngineRawRustRegistry::new();
    raw.register_step("inspection_only", |_, _| {
        panic!("inspection dispatched a raw callback")
    });
    assert!(f.build_with(&raw).is_ok());
    f.pack_change(|p| p.manifest.required_raw_rust_fns.clear());
    f.repin();
    assert!(f.build_with(&raw).is_err());
}
#[test]
fn selected_enrichment_never_adopts_outside_paths_or_tokens() {
    let mut f = Fixture::new();
    f.pack_change(|p| {
        let c = p.cards.iter_mut().find(|c| c.card == "FIX-001").unwrap();
        c.also_treated_as = vec!["Alias".into()];
        c.ace_overflow = Some(-3);
        p.cards
            .iter_mut()
            .find(|c| c.card == "OUT-001")
            .unwrap()
            .kind = CompiledCardKind::Token;
    });
    f.repin();
    let s = f.build().unwrap();
    assert_eq!(s.card("FIX-001").unwrap().also_treated_as, vec!["Alias"]);
    assert_eq!(s.card("FIX-001").unwrap().ace_overflow, Some(-3));
    assert!(s.card("OUT-001").is_err());
    assert_eq!(s.len(), 2);
}

#[test]
fn q_only_binding_builds_an_empty_store_without_importing_pack_or_tokens() {
    let mut f = Fixture::new();
    f.runtime = bytes(&json!({}));
    f.binding["executable_card_ids"] = json!([]);
    f.binding["counts"]["executable"] = json!(0);
    f.binding["counts"]["field_slots"] = json!(0);
    f.repin();
    let s = f.build().unwrap();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
    for id in ["FIX-001", "OUT-001", "TOK-001"] {
        assert!(s.card(id).is_err());
    }
}
