use super::*;
use serde_json::json;

fn retained_operational_fixture(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().canonicalize().unwrap().join(name);
    fs::create_dir(&path).unwrap();
    path
}

#[cfg(windows)]
fn junction(link: &Path, target: &Path) {
    use std::os::windows::process::CommandExt;
    let result = std::process::Command::new("cmd.exe")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "junction fixture setup: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn operational_r2_same_byte_owner_replacement_refuses() {
    operational_same_byte_replacement("OWNER.json", "owner-replacement");
}
#[test]
fn operational_r2_same_byte_start_replacement_refuses() {
    operational_same_byte_replacement("START.json", "start-replacement");
}
fn operational_same_byte_replacement(member: &str, name: &str) {
    let base = retained_operational_fixture(name);
    let out = base.join("out");
    let result = publish_diagnostic(
        &out,
        "fixture",
        || Ok(()),
        || {
            let original = base.join(format!("original-{member}"));
            fs::rename(out.join(member), &original).unwrap();
            fs::copy(original, out.join(member)).unwrap();
            Ok(json!({"fixture":true}))
        },
    );
    fs::write(base.join("RESULT.json"),json!({"accepted":result.is_ok(),"completed":out.join("COMPLETED.json").exists(),"error":result.as_ref().err()}).to_string()).unwrap();
    assert!(
        result.is_err(),
        "same-byte physical ownership replacement accepted"
    );
    assert!(!out.join("COMPLETED.json").exists());
}

#[test]
fn operational_r2_foreign_member_refuses() {
    let base = retained_operational_fixture("foreign-member");
    let out = base.join("out");
    let result = publish_diagnostic(
        &out,
        "fixture",
        || Ok(()),
        || {
            fs::write(out.join("FOREIGN.json"), b"foreign").unwrap();
            Ok(json!({"fixture":true}))
        },
    );
    fs::write(
        base.join("RESULT.json"),
        json!({"accepted":result.is_ok(),"completed":out.join("COMPLETED.json").exists()})
            .to_string(),
    )
    .unwrap();
    assert!(result.is_err(), "foreign member accepted");
    assert!(!out.join("COMPLETED.json").exists());
}

#[cfg(windows)]
#[test]
fn operational_r2_junction_descriptor_refuses() {
    let base = retained_operational_fixture("descriptor-junction");
    let real = base.join("real");
    fs::create_dir(&real).unwrap();
    fs::write(real.join("payload"), b"finite fixture").unwrap();
    let mut descriptor = Descriptor::from_path(&real.join("payload")).unwrap();
    junction(&base.join("alias"), &real);
    descriptor.path = base.join("alias/payload").to_string_lossy().into();
    let result = descriptor.read();
    fs::write(
        base.join("RESULT.json"),
        json!({"accepted":result.is_ok()}).to_string(),
    )
    .unwrap();
    assert!(result.is_err(), "junction descriptor accepted");
}

#[cfg(windows)]
#[test]
fn operational_r2_dangling_stop_and_reparse_ancestor_refuse() {
    let base = retained_operational_fixture("stop-boundaries");
    let absent = base.join("absent/STOP");
    assert!(stop_absent(&absent).is_ok());
    for name in ["STOP", "out"] {
        junction(&base.join(name), &base.join("missing-target"));
    }
    assert!(stop_absent(&base.join("STOP")).is_err());
    assert!(stop_absent(&base.join("out/STOP")).is_err());
    let real = base.join("real");
    fs::create_dir(&real).unwrap();
    junction(&base.join("alias"), &real);
    assert!(stop_absent(&base.join("alias/STOP")).is_err());
}

#[test]
fn operational_r2_input_identity_traversal_and_hardlinks_refuse() {
    let base = retained_operational_fixture("input-identity");
    let path = base.join("payload");
    fs::write(&path, b"original").unwrap();
    let descriptor = Descriptor::from_path(&path).unwrap();
    let bound = BoundFile::bind(&descriptor).unwrap();
    bound.check(None).unwrap();
    fs::rename(&path, base.join("original")).unwrap();
    fs::copy(base.join("original"), &path).unwrap();
    assert!(
        bound.check(None).is_err(),
        "same-byte physical input replacement accepted"
    );
    let mut traversal = descriptor.clone();
    traversal.path = format!("{}/sub/../payload", base.to_string_lossy());
    assert!(traversal.read().is_err());
    fs::hard_link(&path, base.join("hardlink")).unwrap();
    assert!(descriptor.read().is_err());
}

#[test]
fn operational_r2_same_byte_root_replacement_refuses_without_failure_publication() {
    let base = retained_operational_fixture("root-replacement");
    let out = base.join("out");
    let result = publish_diagnostic(
        &out,
        "fixture",
        || Ok(()),
        || {
            fs::rename(&out, base.join("original-root")).unwrap();
            fs::create_dir(&out).unwrap();
            for name in ["OWNER.json", "START.json"] {
                fs::copy(base.join("original-root").join(name), out.join(name)).unwrap();
            }
            Ok(json!({"fixture":true}))
        },
    );
    assert!(result.is_err());
    assert!(!out.join("COMPLETED.json").exists());
    assert!(!out.join("FAILED.json").exists());
}

#[test]
fn operational_r2_after_completion_stop_is_failure_and_retains_orphan() {
    let base = retained_operational_fixture("terminal-stop");
    let out = base.join("out");
    let stop = base.join("STOP");
    let result = publish_with_controls(
        &out,
        "fixture",
        || stop_absent(&stop),
        || {
            if out.join("COMPLETED.json").exists() && !stop.exists() {
                fs::write(&stop, b"stop after durable completion").unwrap();
            }
            stop_absent(&stop)
        },
        || Ok(json!({"fixture":true})),
    );
    fs::write(base.join("RESULT.json"),json!({"accepted":result.is_ok(),"completion_retained":out.join("COMPLETED.json").exists()}).to_string()).unwrap();
    assert!(result.is_err());
    assert!(out.join("COMPLETED.json").exists());
    assert!(!out.join("FAILED.json").exists());
}

#[test]
fn operational_r2_payload_replacement_and_late_foreign_member_refuse() {
    for mode in ["payload", "foreign"] {
        let base = retained_operational_fixture(&format!("late-{mode}"));
        let out = base.join("out");
        let changed = std::cell::Cell::new(false);
        let result = publish_with_controls(
            &out,
            "fixture",
            || Ok(()),
            || {
                if out.join("DIAGNOSTIC.json").exists() && !changed.replace(true) {
                    if mode == "payload" {
                        fs::rename(out.join("DIAGNOSTIC.json"), base.join("original-payload"))
                            .unwrap();
                        fs::copy(base.join("original-payload"), out.join("DIAGNOSTIC.json"))
                            .unwrap();
                    } else {
                        fs::write(out.join("FOREIGN"), b"foreign").unwrap();
                    }
                }
                Ok(())
            },
            || Ok(json!({"fixture":true})),
        );
        assert!(result.is_err());
        assert!(!out.join("COMPLETED.json").exists());
        assert!(!out.join("FAILED.json").exists());
    }
}

#[test]
fn operational_r2_finite_resource_limits_and_repeated_role_conflicts() {
    let base = retained_operational_fixture("resource-bounds");
    let descriptor = |name: &str, bytes: u64| Descriptor {
        path: base.join(name).to_string_lossy().into(),
        bytes,
        sha256: "0".repeat(64),
    };
    let a = descriptor("a", MAX_FILE);
    let b = descriptor("b", MAX_FILE);
    let c = descriptor("c", 1);
    assert_eq!(
        bounded_plan(&[("a".into(), &a), ("again".into(), &a), ("b".into(), &b)])
            .unwrap()
            .len(),
        2
    );
    assert!(bounded_plan(&[("a".into(), &a), ("b".into(), &b), ("c".into(), &c)]).is_err());
    assert!(BoundFile::bind(&descriptor("too-big", MAX_FILE + 1)).is_err());
    let conflict = descriptor("a", 1);
    assert!(bounded_plan(&[("a".into(), &a), ("conflict".into(), &conflict)]).is_err());
    let over_roles = vec![("duplicate".into(), &c); MAX_ROLES + 1];
    assert!(bounded_plan(&over_roles).is_err());
    let declarations: Vec<_> = (0..MAX_DESCRIPTORS + 1)
        .map(|i| descriptor(&format!("file-{i}"), 0))
        .collect();
    let roles: Vec<_> = declarations.iter().map(|d| ("source".into(), d)).collect();
    assert!(bounded_plan(&roles).is_err());
    assert!(encode_limited(&json!({"text":"x".repeat(4096)}), 128).is_err());
    assert!(encode_limited(&json!({"text":"small"}), 128).is_ok());
}

fn tiny_operational_request(base: &Path) -> (PathBuf, String, PathBuf) {
    let inputs = base.join("inputs");
    let launch = base.join("launch");
    fs::create_dir(&inputs).unwrap();
    fs::create_dir(&launch).unwrap();
    let put = |name: &str, v: &Value| {
        let path = inputs.join(name);
        write_new(&path, &encode(v).unwrap()).unwrap();
        Descriptor::from_path(&path).unwrap()
    };
    let (rows, mut report, global, comparison) = source_fixture();
    let records = put("records.json", &rows);
    report["generated_cards_sha256"] = json!(records.sha256);
    let source_report = put("report.json", &report);
    let registry = put("registry.json", &global);
    let comparison_runtime = put("comparison.json", &comparison);
    let empty = put("metadata.json", &json!({}));
    let executable = Descriptor::from_path(&std::env::current_exe().unwrap()).unwrap();
    let packpath = inputs.join("cards.pack");
    write_new(&packpath, PACK).unwrap();
    let pack = Descriptor::from_path(&packpath).unwrap();
    let body = json!({"schema_version":1,"kind":"native-conversion-diagnostic-request-v1","stop_path":base.join("STOP"),"records":records,
        "source_report":source_report,"registry":registry,"closure_manifest":empty,"comparison_runtime":comparison_runtime,"generation_completion":empty,
        "generation_natural_exit":empty,"independent_source_review":empty,"build_receipt":empty,"executable":executable,"pack":pack,
        "dependencies":[empty],"yaml_inputs":[]});
    let path = launch.join("request.json");
    let bytes = encode(&body).unwrap();
    write_new(&path, &bytes).unwrap();
    (path, sha(&bytes), base.join("out"))
}

#[cfg(windows)]
#[test]
fn operational_r2_real_entrypoint_dangling_stop_refuses_before_small_profile_gate() {
    let base = retained_operational_fixture("entrypoint-stop");
    let (path, pin, out) = tiny_operational_request(&base);
    let absent = execute(&path, &pin, &out).unwrap_err();
    assert!(
        absent.contains("complete99/4483"),
        "invalid positive fixture: {absent}"
    );
    assert!(!out.exists());
    junction(&base.join("STOP"), &base.join("missing-target"));
    let stopped = execute(&path, &pin, &out).unwrap_err();
    assert!(
        stopped.contains("reparse"),
        "dangling STOP escaped: {stopped}"
    );
    assert!(!out.exists());
    fs::write(base.join("RESULT.json"),json!({"absent_stop_error":absent,"dangling_stop_error":stopped,"actual99_calls":0,"conversion_reached":false}).to_string()).unwrap();
}

#[test]
fn operational_r2_canonical_siblings_and_repeated_roles_bind_but_ancestry_refuses() {
    let base = retained_operational_fixture("input-layout");
    let (path, _, out) = tiny_operational_request(&base);
    let d = Descriptor::from_path(&path).unwrap();
    let req_file = BoundFile::bind(&d).unwrap();
    let req: Request = serde_json::from_value(strict_json(&d.read().unwrap()).unwrap()).unwrap();
    let bindings = InputBindings::bind(&req, &req_file, &out).unwrap();
    assert!(bindings.roles.len() > bindings.files.len());
    assert_eq!(
        bindings.report().as_array().unwrap().len(),
        req.inputs().len()
    );
    bindings.verify().unwrap();
    assert!(InputBindings::bind(&req, &req_file, &base.join("inputs/output")).is_err());
    assert!(InputBindings::bind(&req, &req_file, &base.join("launch/output")).is_err());
    assert!(InputBindings::bind(
        &req,
        &req_file,
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("diagnostic-out")
            .as_path()
    )
    .is_err());
}

fn wire(id: &str, index: u16) -> Value {
    json!({"card_id":id,"card_name_eng":"Fixture","card_effect_class_name":"Fixture",
        "play_cost":3,"dp":2000,"level":3,"card_kind":0,"card_colors":[2,0],
        "type_eng":["Beast","Beast"],"form_eng":["Rookie"],"attribute_eng":["Free"],
        "effect_description_eng":"","inherited_effect_description_eng":"",
        "security_effect_description_eng":"","evo_costs":[{"card_color":2,"level":2,"memory_cost":0}],
        "index":index,"norm_id":f64::from(index)/20000.0,"card_index":id.split('-').last().unwrap().parse::<u16>().unwrap()})
}

fn fixture() -> (Value, Value, BTreeSet<String>, CardPack) {
    let rows = json!({"FIX-001":wire("FIX-001",7),"FIX-002":wire("FIX-002",91)});
    let registry = json!({"schema_version":1,"registry_version":1,"capacity":20000,"padding_index":0,
        "assignment_count":4,"assignments":{"FIX-001":7,"FIX-002":91,"OUT-001":42,"OUT-002":999}});
    (
        rows,
        registry,
        ["FIX-001".into(), "FIX-002".into()].into(),
        CardPack::new("fixture", vec![compiled("FIX-001"), compiled("FIX-002")]),
    )
}

fn compiled(id: &str) -> CompiledCard {
    CompiledCard {
        card: id.into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(3),
        color: vec![CompiledColor::Yellow, CompiledColor::Red],
        cost: Some(3),
        dp: Some(2000),
        traits: vec![],
        form: None,
        attribute: vec![],
        ace_overflow: None,
        identity: None,
        digixros_aliases: vec![],
        also_treated_as: vec![],
        dual: None,
        use_requirement: None,
        alt_paths: vec![],
        effects: vec![],
    }
}

fn dna() -> CompiledAltPath {
    let material = CompiledMaterial {
        filter: CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            level_eq: Some(4),
            color_is: Some(CompiledColor::Blue),
            ..Default::default()
        },
        repeat: None,
        distinct_by: None,
        zones: vec![],
        cost_delta: None,
        stack_under: false,
    };
    CompiledAltPath {
        kind: CompiledAltPathKind::DnaDigivolve,
        from: None,
        materials: vec![material.clone(), material],
        cost: Some(CompiledCost::Literal(2)),
        stacks_unsuspended: false,
        ignore_requirements: false,
        source_treated_as: None,
        extra_cost: vec![],
        on_burst_turn_end: vec![],
        marker: false,
        condition: None,
        direction: CompiledAltPathDirection::From,
        extra_material_zones: vec![],
    }
}

#[test]
fn sparse_registry_never_renumbers_and_retains_outside_assignments() {
    let (rows, registry, scope, pack) = fixture();
    let out = diagnose(&rows, &registry, &scope, &pack).unwrap();
    assert_eq!(out["cards"]["FIX-002"]["registry"]["index"], 91);
    assert_eq!(out["full_registry"]["assignments"]["OUT-001"], 42);
    assert_eq!(out["selected_registry_count"], 2);
    assert_eq!(
        out["cards"]["FIX-001"]["raw_native"]["traits"],
        json!(["Rookie", "Free", "Beast", "Beast"])
    );
    assert_eq!(
        out["cards"]["FIX-001"]["raw_native"]["colors"],
        json!(["Yellow", "Red"])
    );
    assert_eq!(out["derived_families"].as_array().unwrap().len(), 10);
    assert_eq!(
        out["admission"],
        json!({"source":false,"runtime":false,"oracle":false,"training":false})
    );
    assert_eq!(out["projection_epoch"], Value::Null);
}

#[test]
fn omissions_extras_identity_and_registry_collisions_refuse() {
    let (rows, registry, scope, pack) = fixture();
    for field in [
        "missing",
        "extra",
        "card_id",
        "zero",
        "duplicate",
        "wrong_norm",
        "outside_registry",
    ] {
        let mut r = rows.clone();
        let mut g = registry.clone();
        match field {
            "missing" => {
                r.as_object_mut().unwrap().remove("FIX-002");
            }
            "extra" => {
                r["OUT-001"] = wire("OUT-001", 42);
            }
            "card_id" => r["FIX-001"]["card_id"] = json!("FIX-002"),
            "zero" => r["FIX-001"]["index"] = json!(0),
            "duplicate" => g["assignments"]["OUT-001"] = json!(7),
            "wrong_norm" => r["FIX-001"]["norm_id"] = json!(0.1),
            "outside_registry" => g["assignment_count"] = json!(3),
            _ => unreachable!(),
        }
        assert!(diagnose(&r, &g, &scope, &pack).is_err(), "accepted {field}");
    }
}

#[test]
fn permissive_loader_fallbacks_are_rejected_by_diagnostic_wire_gate() {
    let (rows, registry, scope, pack) = fixture();
    for (key, value) in [
        ("card_kind", json!(5)),
        ("card_colors", json!([7])),
        ("level", json!(256)),
        ("play_cost", json!(65536)),
        ("level", json!(-2)),
        ("index", json!(true)),
        ("mystery", json!(1)),
    ] {
        let mut r = rows.clone();
        r["FIX-001"][key] = value;
        assert!(
            diagnose(&r, &registry, &scope, &pack).is_err(),
            "accepted {key}"
        );
    }
    let mut direct = rows.clone();
    direct["FIX-001"]["card_kind"] = json!(5);
    direct["FIX-001"]["card_colors"] = json!([7]);
    let actual = CardData::load_from_str(&direct.to_string()).unwrap();
    assert_eq!(
        actual["FIX-001"].card_kind,
        digimon_engine::enums::CardKind::Digimon
    );
    assert_eq!(
        actual["FIX-001"].colors,
        vec![digimon_engine::enums::CardColor::Red]
    );
}

#[test]
fn strict_json_rejects_duplicate_keys_nested_keys_and_nonfinite() {
    for raw in [r#"{"a":1,"a":2}"#, r#"{"a":{"b":1,"b":2}}"#, r#"{"a":NaN}"#] {
        assert!(strict_json(raw.as_bytes()).is_err());
    }
    assert_eq!(
        strict_json(br#"{"a":[null,{},1]}"#).unwrap(),
        json!({"a":[null,{},1]})
    );
}

#[test]
fn absence_null_empty_and_sentinels_do_not_collapse_in_report() {
    let (mut rows, registry, scope, pack) = fixture();
    rows["FIX-001"]["level"] = Value::Null;
    rows["FIX-001"]["dp"] = json!(-1);
    rows["FIX-001"]["play_cost"] = Value::Null;
    rows["FIX-001"]["ace_overflow"] = Value::Null;
    rows["FIX-002"]["digixros_aliases"] = json!([]);
    let out = diagnose(&rows, &registry, &scope, &pack).unwrap();
    let c = &out["cards"]["FIX-001"];
    assert_eq!(c["presence"]["level"]["presence"], "null");
    assert_eq!(c["presence"]["dp"]["presence"], "value");
    assert_eq!(c["presence"]["ace_overflow"]["presence"], "null");
    assert_eq!(c["presence"]["digixros_aliases"]["presence"], "absent");
    assert_eq!(
        out["cards"]["FIX-002"]["presence"]["digixros_aliases"]["presence"],
        "empty"
    );
    assert_eq!(c["raw_native"]["level"], Value::Null);
    assert_eq!(c["raw_native"]["dp"], Value::Null);
    assert_eq!(c["raw_native"]["play_cost"], 0);
}

#[test]
fn keyword_origins_and_grants_are_not_confused() {
    let (mut rows, registry, scope, pack) = fixture();
    rows["FIX-001"]["effect_description_eng"] =
        json!("＜Blocker＞ [Your Turn] This Digimon gains ＜Rush＞.");
    rows["FIX-001"]["inherited_effect_description_eng"] = json!("Inherited Effect ＜Raid＞");
    rows["FIX-001"]["security_effect_description_eng"] =
        json!("[Security] ＜Draw 1＞. Then, trash 1 card.");
    let out = diagnose(&rows, &registry, &scope, &pack).unwrap();
    assert_eq!(
        out["cards"]["FIX-001"]["raw_native"]["keywords"],
        json!(["Blocker", "Raid"])
    );
    assert_eq!(
        out["cards"]["FIX-001"]["keyword_parsing"]["security"],
        json!([])
    );
    assert!(out["cards"]["FIX-001"]["keyword_text_spans"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["text"] == "Rush"));
}

#[test]
fn enrichment_overwrite_is_explicit_even_when_it_discards_raw_metadata() {
    let (mut rows, registry, scope, mut pack) = fixture();
    rows["FIX-001"]["ace_overflow"] = json!(-3);
    rows["FIX-001"]["digixros_aliases"] = json!(["Raw alias"]);
    pack.cards[0].also_treated_as = vec!["Identity".into()];
    let out = diagnose(&rows, &registry, &scope, &pack).unwrap();
    let c = &out["cards"]["FIX-001"];
    assert_eq!(c["raw_native"]["ace_overflow"], -3);
    assert!(c["enriched_native"].get("ace_overflow").is_none());
    assert_eq!(c["enriched_native"]["also_treated_as"], json!(["Identity"]));
    let changes = c["enrichment_changes"].as_array().unwrap();
    assert!(changes.iter().any(|v| v["field"] == "ace_overflow"));
    assert!(changes.iter().any(|v| v["field"] == "digixros_aliases"));
}

#[test]
fn dna_emission_omission_dedup_and_name_widening_stay_distinct() {
    let (rows, registry, scope, mut pack) = fixture();
    let good = dna();
    let mut unsupported = good.clone();
    unsupported.materials.push(unsupported.materials[0].clone());
    let mut wider = good.clone();
    wider.materials[0].filter.name_is = Some("Exact Name".into());
    pack.cards[0].alt_paths = vec![good.clone(), good, unsupported, wider];
    let out = diagnose(&rows, &registry, &scope, &pack).unwrap();
    assert_eq!(out["dna_inserted"], 2);
    let paths = &out["cards"]["FIX-001"]["alt_path_diagnostics"];
    assert_eq!(paths[0]["status"], "EMITTED");
    assert_eq!(paths[1]["full_pass_disposition"], "DEDUPLICATED");
    assert_eq!(paths[2]["status"], "OMITTED_UNSUPPORTED");
    assert!(paths[3]["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "NAME_IS_REPRESENTED_AS_SUBSTRING"));
    assert_eq!(
        paths[3]["emitted"][0]["requirement1"]["name_contains"],
        "Exact Name"
    );
}

#[test]
fn dual_lookup_keeps_face_fields_and_excludes_link_text() {
    let (mut rows, registry, scope, pack) = fixture();
    rows["FIX-001"]["card_kind"] = json!(4);
    rows["FIX-001"]["link_effect_description_eng"] = json!("LINK ONLY");
    rows["FIX-001"]["dual"] = json!({"digimon":{"level":5,"dp":7000,"colors":["Blue"],"traits":["Sea"],"evo_costs":[],
        "effect_text":"DIGIMON","inherited_text":"INHERITED","keywords":["Blocker"]},
        "option":{"use_cost":8,"colors":["Red"],"effect_text":"OPTION","security_text":"SECURITY","keywords":["ArtsDigivolve"]}});
    let out = diagnose(&rows, &registry, &scope, &pack).unwrap();
    let lookup = &out["cards"]["FIX-001"]["raw_lookup"];
    assert_eq!(lookup["digimon_level"], 5);
    assert_eq!(lookup["option_use_cost"], 8);
    assert_eq!(lookup["digimon_colors"], json!(["Blue"]));
    assert_eq!(lookup["option_colors"], json!(["Red"]));
    assert_eq!(
        lookup["text_for_search_all_faces"],
        "DIGIMON\nINHERITED\nOPTION\nSECURITY"
    );
    assert_eq!(out["cards"]["FIX-001"]["raw_native"]["keywords"], json!([]));
}

#[test]
fn compatibility_is_retained_but_never_native_proven() {
    let (mut rows, registry, scope, pack) = fixture();
    rows["FIX-001"]["digixros_costs"] = json!([{"any_of":[{"trait_contains":"Aqua"}]}]);
    rows["FIX-001"]["max_count_in_deck"] = json!(50);
    let out = diagnose(&rows, &registry, &scope, &pack).unwrap();
    assert_eq!(
        out["cards"]["FIX-001"]["ignored_compatibility"]["digixros_costs"]["value"],
        rows["FIX-001"]["digixros_costs"]
    );
    assert_eq!(
        out["cards"]["FIX-001"]["compatibility_semantics_proven"],
        false
    );
}

#[test]
fn pack_duplicates_and_raw_manifest_disagreement_refuse() {
    let (_, _, _, mut pack) = fixture();
    let raw = digimon_engine::cards::raw_rust::build_registry();
    pack.cards.push(pack.cards[0].clone());
    assert!(checked_pack(&pack.to_bytes().unwrap(), &raw).is_err());
    pack.cards.pop();
    pack.manifest.required_raw_rust_fns = vec!["invented".into()];
    assert!(checked_pack(&pack.to_bytes().unwrap(), &raw).is_err());
}

#[test]
fn missing_selected_pack_and_outside_cards_are_reported_without_filling_rows() {
    let (rows, registry, scope, mut pack) = fixture();
    pack.cards.pop();
    pack.cards.push(compiled("OUT-001"));
    let out = diagnose(&rows, &registry, &scope, &pack).unwrap();
    assert_eq!(out["missing_selected_pack"], json!(["FIX-002"]));
    assert_eq!(out["outside_e_pack"], json!(["OUT-001"]));
    assert_eq!(out["cards"].as_object().unwrap().len(), 2);
    assert!(out["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .all(|x| x["template_source_proven"] == false));
}

#[test]
fn token_occurrence_inventory_recurses_and_preserves_unknown_names() {
    let ir = json!({"effects":[{"Process":{"then":[{"PlayToken":{"controller":"You","token_name":"unknown-template","bind_as":null}}]}}],
        "dual":{"option":{"effects":[{"PlayToken":{"controller":"You","token_name":"familiar","bind_as":null}}]}}});
    let refs = token_occurrences(&ir).unwrap();
    assert_eq!(refs.len(), 2);
    assert!(refs
        .iter()
        .any(|r| r["token_name"] == "unknown-template" && r["registered"] == false));
    assert!(refs
        .iter()
        .any(|r| r["pointer"] == "/dual/option/effects/0/PlayToken"));
}

#[test]
fn same_length_metadata_uses_fresh_conversion_not_shared_store() {
    let (mut rows, registry, scope, pack) = fixture();
    rows["FIX-001"]["effect_description_eng"] = json!("AAAA");
    let a = diagnose(&rows, &registry, &scope, &pack).unwrap();
    rows["FIX-001"]["effect_description_eng"] = json!("BBBB");
    let b = diagnose(&rows, &registry, &scope, &pack).unwrap();
    assert_eq!(a["cards"]["FIX-001"]["raw_native"]["effect_text"], "AAAA");
    assert_eq!(b["cards"]["FIX-001"]["raw_native"]["effect_text"], "BBBB");
}

struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let base = std::env::temp_dir().canonicalize().unwrap();
        let p = base.join(format!(
            "native-diagnostic-fixture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        assert!(p.starts_with(&base));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let base = std::env::temp_dir().canonicalize().unwrap();
        let p = self.0.canonicalize().unwrap();
        assert!(
            p.starts_with(base)
                && p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("native-diagnostic-fixture-")
        );
        fs::remove_dir_all(p).unwrap();
    }
}

#[test]
fn owner_replacement_during_input_guard_prevents_first_payload() {
    let t = Temp::new();
    let out = t.0.join("out");
    let calls = std::cell::Cell::new(0);
    let result = publish_diagnostic(
        &out,
        "fixture",
        || {
            calls.set(calls.get() + 1);
            if calls.get() == 2 {
                fs::write(out.join("OWNER.json"), b"replacement").unwrap();
            }
            Ok(())
        },
        || Ok(json!({"fixture":true})),
    );
    assert!(result.is_err());
    assert!(
        !out.join("DIAGNOSTIC.json").exists(),
        "payload escaped after owner replacement during guard"
    );
    assert!(!out.join("COMPLETED.json").exists());
    assert!(!out.join("FAILED.json").exists());
}

#[test]
fn stop_before_save_and_replaced_start_refuse_and_do_not_publish_failure() {
    for mode in ["stop", "start"] {
        let t = Temp::new();
        let out = t.0.join("out");
        let result = publish_diagnostic(
            &out,
            "fixture",
            || require(!t.0.join("STOP").exists(), "STOP"),
            || {
                if mode == "stop" {
                    fs::write(t.0.join("STOP"), b"stop").unwrap();
                } else {
                    fs::write(out.join("START.json"), b"replacement").unwrap();
                }
                Ok(json!({"fixture":true}))
            },
        );
        assert!(result.is_err());
        assert!(!out.join("DIAGNOSTIC.json").exists());
        assert!(!out.join("COMPLETED.json").exists());
        assert!(!out.join("FAILED.json").exists());
    }
}

#[test]
fn saved_payload_mutation_and_duplicate_resume_never_complete() {
    let t = Temp::new();
    let out = t.0.join("out");
    let calls = std::cell::Cell::new(0);
    assert!(publish_diagnostic(
        &out,
        "fixture",
        || {
            calls.set(calls.get() + 1);
            if calls.get() == 3 {
                fs::write(out.join("DIAGNOSTIC.json"), b"mutated").unwrap();
            }
            Ok(())
        },
        || Ok(json!({"fixture":true}))
    )
    .is_err());
    assert!(!out.join("COMPLETED.json").exists());
    assert!(publish_diagnostic(
        &out,
        "fixture",
        || Ok(()),
        || panic!("duplicate invocation reached producer")
    )
    .is_err());
}

#[test]
fn payload_replacement_during_final_input_guard_prevents_completion() {
    let t = Temp::new();
    let out = t.0.join("out");
    let calls = std::cell::Cell::new(0);
    let result = publish_diagnostic(
        &out,
        "fixture",
        || {
            calls.set(calls.get() + 1);
            if calls.get() == 4 {
                fs::write(out.join("DIAGNOSTIC.json"), b"late replacement").unwrap();
            }
            Ok(())
        },
        || Ok(json!({"fixture":true})),
    );
    assert!(
        result.is_err(),
        "completion accepted payload changed during final guard"
    );
    assert!(!out.join("COMPLETED.json").exists());
}

fn source_fixture() -> (Value, Value, Value, Value) {
    let (rows, global, _, _) = fixture();
    let comparison = json!({"FIX-001":{"index":7},"FIX-002":{"index":91},"OUT-001":{"index":42},"OUT-002":{"index":999}});
    let mut cards = Map::new();
    for (id, row) in rows.as_object().unwrap() {
        let fields: Map<_, _> = row
            .as_object()
            .unwrap()
            .keys()
            .map(|k| (k.clone(), presence(row, k)))
            .collect();
        cards.insert(id.clone(), json!({"normal_projection":fields}));
    }
    let report = json!({"generated_cards_sha256":"fixture-byte-pin","registry":global,
        "closure":{"E":["FIX-001"],"Q":["FIX-002"],"original_runtime_indices":comparison},"cards":cards});
    (rows, report, global, comparison)
}

#[test]
fn source_witness_checks_every_projection_and_full_outside_registry() {
    let (rows, report, global, comparison) = source_fixture();
    assert_eq!(
        source_binding(&rows, &report, &global, &comparison, "fixture-byte-pin")
            .unwrap()
            .len(),
        2
    );
    for change in [
        "hash",
        "scope",
        "outside-index",
        "outside-omission",
        "projection",
        "absent-null",
        "report-omission",
    ] {
        let (mut r, mut p, g, mut c) = source_fixture();
        match change {
            "hash" => p["generated_cards_sha256"] = json!("other"),
            "scope" => p["closure"]["E"] = json!(["FIX-001", "FIX-001"]),
            "outside-index" => c["OUT-002"]["index"] = json!(998),
            "outside-omission" => {
                c.as_object_mut().unwrap().remove("OUT-001");
            }
            "projection" => r["FIX-001"]["dp"] = json!(3000),
            "absent-null" => {
                p["cards"]["FIX-001"]["normal_projection"]["dual"] =
                    json!({"presence":"null","value":null})
            }
            "report-omission" => {
                p["cards"].as_object_mut().unwrap().remove("FIX-002");
            }
            _ => unreachable!(),
        }
        assert!(
            source_binding(&r, &p, &g, &c, "fixture-byte-pin").is_err(),
            "accepted {change}"
        );
    }
}

#[test]
fn successful_publication_is_exact_and_failure_has_no_completion() {
    let t = Temp::new();
    let out = t.0.join("ok");
    publish_diagnostic(&out, "fixture", || Ok(()), || Ok(json!({"n":1}))).unwrap();
    let bytes = fs::read(out.join("DIAGNOSTIC.json")).unwrap();
    let done = strict_json(&fs::read(out.join("COMPLETED.json")).unwrap()).unwrap();
    assert_eq!(done["diagnostic_sha256"], sha(&bytes));
    assert_eq!(done["native_admission"], false);
    let failed = t.0.join("failed");
    assert!(publish_diagnostic(
        &failed,
        "fixture",
        || Ok(()),
        || Err("fixture error".into())
    )
    .is_err());
    assert!(failed.join("FAILED.json").exists());
    assert!(!failed.join("COMPLETED.json").exists());
}

#[test]
fn actual_embedded_pack_matches_production_registry_without_loading_source_records() {
    let raw = digimon_engine::cards::raw_rust::build_registry();
    let pack = checked_pack(PACK, &raw).unwrap();
    let registry = digimon_engine::dsl_registry::from_embedded_with_raw_registry(&raw).unwrap();
    assert!(!pack.cards.is_empty());
    assert_eq!(registry.len(), pack.cards.len());
    assert_eq!(registry.manifest, pack.manifest);
    let mut occurrences = 0;
    for card in &pack.cards {
        assert_eq!(registry.lookup(&card.card), Some(card));
        occurrences += token_occurrences(&value(card).unwrap()).unwrap().len();
    }
    assert!(
        occurrences > 0,
        "recursive token inventory reached no actual compiled token steps"
    );
    println!(
        "embedded pack: {} cards, {} required raw functions, {} token occurrences; metadata only",
        pack.cards.len(),
        pack.manifest.required_raw_rust_fns.len(),
        occurrences
    );
}

#[test]
fn actual_embedded_pack_membership_matches_current_build_yaml_inputs() {
    let cards = Path::new(env!("CARGO_MANIFEST_DIR")).join("cards");
    let mut directories: Vec<_> = fs::read_dir(cards)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    directories.sort();
    let mut files = Vec::new();
    for d in directories {
        let mut stack = vec![d];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(dir).unwrap() {
                let p = entry.unwrap().path();
                if p.is_dir() {
                    stack.push(p)
                } else if p.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                    files.push(Descriptor::from_path(&p).unwrap());
                }
            }
        }
    }
    let declarations =
        read_yaml_in_build_order(&files, &Path::new(env!("CARGO_MANIFEST_DIR")).join("cards"))
            .unwrap();
    let pack = CardPack::from_bytes(PACK).unwrap();
    let binding = yaml_pack_binding(&declarations, &pack).unwrap();
    assert_eq!(
        binding["declaration_count"].as_u64().unwrap() as usize,
        declarations.len()
    );
    assert_eq!(
        binding["unique_pack_card_count"].as_u64().unwrap() as usize,
        pack.cards.len()
    );
    assert_eq!(binding["full_winning_ir_equal_to_embedded"], true);
    write_new(
        &std::env::temp_dir().join("NATIVE-PRODUCTION-PACK-METADATA.json"),
        &encode(&binding).unwrap(),
    )
    .unwrap();
    println!("current YAML/pack exact IR: {} declarations, {} unique cards, {} shadowed; authority unproven",declarations.len(),pack.cards.len(),binding["shadowed_declarations"].as_array().unwrap().len());
}

#[test]
fn shadowed_yaml_is_preserved_and_same_membership_wrong_winner_refuses() {
    let t = Temp::new();
    let mut declarations = Vec::new();
    for (i, name) in ["Earlier", "Later"].iter().enumerate() {
        let path = t.0.join(format!("{i}.yaml"));
        fs::write(&path,format!("card: FIX-001\nname: {name}\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 2000\n")).unwrap();
        let spec = digimon_dsl::loader::load_file(&path).unwrap();
        declarations.push((Descriptor::from_path(&path).unwrap(), spec));
    }
    let compiled = digimon_dsl::compile::compile(&declarations[1].1).unwrap();
    let pack = CardPack::new("fixture", vec![compiled]);
    let binding = yaml_pack_binding(&declarations, &pack).unwrap();
    assert_eq!(binding["declaration_count"], 2);
    assert_eq!(binding["unique_pack_card_count"], 1);
    assert_eq!(
        binding["shadowed_declarations"][0]["compiled"]["name"],
        "Earlier"
    );
    assert_eq!(
        binding["shadowed_declarations"][0]["same_ir_as_winner"],
        false
    );
    assert_eq!(binding["duplicate_source_authority"], "UNPROVEN");
    declarations.reverse();
    assert!(
        yaml_pack_binding(&declarations, &pack).is_err(),
        "same IDs hid changed winning IR"
    );
}

#[test]
fn formula_and_extra_predicate_dna_are_retained_as_unsupported() {
    let (rows, registry, scope, mut pack) = fixture();
    let mut formula = dna();
    formula.cost = Some(CompiledCost::Formula(CompiledFormula::Literal(2)));
    let mut extra = dna();
    extra.materials[0].filter.name_is = Some("Exact".into());
    extra.materials[0].filter.trait_has = Some("Sea".into());
    pack.cards[0].alt_paths = vec![formula, extra];
    let out = diagnose(&rows, &registry, &scope, &pack).unwrap();
    assert_eq!(out["dna_inserted"], 0);
    for p in out["cards"]["FIX-001"]["alt_path_diagnostics"]
        .as_array()
        .unwrap()
    {
        assert_eq!(p["status"], "OMITTED_UNSUPPORTED");
    }
}

#[test]
fn build_binding_rejects_partial_sources_wrong_pack_and_test_executable() {
    let t = Temp::new();
    for name in ["code.rs", "card.yaml", "program.exe", "cards.pack"] {
        fs::write(t.0.join(name), name).unwrap();
    }
    let code = Descriptor::from_path(&t.0.join("code.rs")).unwrap();
    let yaml = Descriptor::from_path(&t.0.join("card.yaml")).unwrap();
    let executable = Descriptor::from_path(&t.0.join("program.exe")).unwrap();
    let pack = Descriptor::from_path(&t.0.join("cards.pack")).unwrap();
    let dependencies = vec![code.clone(), yaml.clone()];
    let build = json!({"mode":"build","actual_natural_exit":0,"source_readset":dependencies,"yaml_inputs":[yaml],
        "compiler_artifacts":[{"target":{"name":"native_conversion_diagnostic"},"profile":{"test":false},"executable":executable}],"build_scripts":[{"pack":pack}]});
    build_binding(&build, &dependencies, &[yaml.clone()], &executable, &pack).unwrap();
    for kind in [
        "missing-source",
        "duplicate-source",
        "yaml-substitution",
        "test-executable",
        "exit",
        "pack",
        "source-pin",
    ] {
        let mut b = build.clone();
        let mut d = dependencies.clone();
        let mut y = vec![yaml.clone()];
        match kind {
            "missing-source" => {
                d.remove(0);
            }
            "duplicate-source" => d.push(code.clone()),
            "yaml-substitution" => y = vec![code.clone()],
            "test-executable" => b["compiler_artifacts"][0]["profile"]["test"] = json!(true),
            "exit" => b["actual_natural_exit"] = json!(101),
            "pack" => b["build_scripts"][0]["pack"]["sha256"] = json!("0".repeat(64)),
            "source-pin" => d[0].sha256 = "0".repeat(64),
            _ => unreachable!(),
        }
        assert!(
            build_binding(&b, &d, &y, &executable, &pack).is_err(),
            "accepted {kind}"
        );
    }
}

#[test]
fn request_or_stop_change_during_input_pass_is_rechecked() {
    for mode in ["request", "stop"] {
        let t = Temp::new();
        let request = t.0.join("request.json");
        let stop = t.0.join("STOP");
        fs::write(&request, b"original").unwrap();
        let result = guarded_inputs(
            || {
                require(!stop.exists(), "STOP")?;
                require(
                    fs::read(&request).unwrap() == b"original",
                    "request replaced",
                )
            },
            || {
                if mode == "request" {
                    fs::write(&request, b"replacement").unwrap();
                } else {
                    fs::write(&stop, b"stop").unwrap();
                }
                Ok(())
            },
        );
        assert!(result.is_err());
    }
}

#[test]
fn dna_context_reduction_and_precedence_remain_visible() {
    let (rows, registry, scope, mut pack) = fixture();
    let mut path = dna();
    path.stacks_unsuspended = true;
    path.ignore_requirements = true;
    path.materials[0].filter.name_is = Some("Exact".into());
    path.materials[0].filter.name_contains = Some("Substring".into());
    path.materials[0].filter.color_only = Some(vec![CompiledColor::Red]);
    pack.cards[0].alt_paths = vec![path];
    let out = diagnose(&rows, &registry, &scope, &pack).unwrap();
    let p = &out["cards"]["FIX-001"]["alt_path_diagnostics"][0];
    assert_eq!(
        p["emitted"][0]["requirement1"]["name_contains"],
        "Substring"
    );
    assert_eq!(
        p["emitted"][0]["requirement1"]["card_colors"],
        json!(["Red"])
    );
    for flag in [
        "NAME_CONTAINS_TAKES_PRECEDENCE_OVER_NAME_IS",
        "COLOR_ONLY_PREDICATE_REDUCED_TO_COLOR_LIST",
        "COLOR_ONLY_TAKES_PRECEDENCE_OVER_COLOR_IS",
        "PATH_CONTEXT_NOT_REPRESENTED_BY_DNA_COST",
    ] {
        assert!(p["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x == flag));
    }
    assert_eq!(p["material_semantics_proven"], false);
}
