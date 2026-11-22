//! Explicit-input, fail-closed diagnostic. No registrations, training or admission.
//! Run with baseline cards.json, candidate cards.json and private exact attestation.
use std::{collections::HashMap, env, fs};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn hash(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn plain(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id); c.card_kind = CardKind::Digimon;
    c.dp = Some(20000); c.level = Some(5); c.play_cost = cost; c
}
fn cards(rows: &Value) -> HashMap<String, CardData> {
    CardData::load_from_str(&rows.to_string()).expect("actual catalog must load")
}
fn assert_metadata(r: &DebugRunner, actual: &CardData) {
    let loaded = r.game.card_data.iter().find(|c| c.card_id == actual.card_id).unwrap();
    assert_eq!(loaded.effect_text, actual.effect_text, "DSL must not replace loaded main text");
    assert_eq!(loaded.inherited_text, actual.inherited_text);
    assert_eq!(loaded.security_text, actual.security_text);
    assert_eq!(loaded.requirement_text, actual.requirement_text);
}
fn security_check(actual: CardData, implemented: bool) -> bool {
    let id = actual.card_id.clone();
    let mut b = DebugRunner::builder();
    if implemented { b = b.dsl_card(&id).expect("real security positive-control DSL"); }
    let mut r = b.add_card(actual.clone()).add_card(plain("ATTACKER", 5))
        .add_card(plain("FUEL", 3)).deck(0, &["FUEL", "FUEL", "FUEL"])
        .deck(1, &["FUEL", "FUEL", "FUEL"]).security(1, &[&id]).start();
    assert_metadata(&r, &actual);
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let _ = r.attack_player(attacker, 1, false);
    r.auto_resolve().expect("security processing settles");
    assert_eq!(r.security_count(1), 0);
    assert!(r.pending_selection().is_none());
    let played = r.game.player(1).battle_area.iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id);
    assert_eq!(played, implemented, "metadata relocation cannot synthesize a missing card implementation");
    played
}
fn frigimon(actual: CardData, mode: &str) -> Value {
    let mut tamer = plain("TAMER", 3); tamer.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder().dsl_card("EX8-022").unwrap()
        .add_card(actual.clone()).add_card(plain("HOST", 5)).add_card(plain("OPP", 4))
        .add_card(plain("DUMMY", 3)).add_card(tamer).add_card(plain("FUEL", 3))
        .deck(0, &["FUEL", "FUEL", "FUEL"]).deck(1, &["FUEL", "FUEL", "FUEL"])
        .security(0, &["DUMMY", "DUMMY", "DUMMY"]).security(1, &["DUMMY", "DUMMY"]).start();
    assert_metadata(&r, &actual);
    let host = match mode {
        "face" => r.place_on_field(0, "EX8-022", Some(0)),
        "wrong_source" => r.place_stack(0, &["DUMMY", "HOST"]),
        _ => r.place_stack(0, &["EX8-022", "HOST"]),
    };
    let opp = if mode == "no_digimon" { r.place_on_field(1, "TAMER", Some(0)) }
              else { r.place_on_field(1, "OPP", Some(0)) };
    if mode == "positive" { let _ = r.attack_player(host, 1, false); }
    else { r.game.enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host)); r.game.drain_effect_queue(); }
    if mode != "positive" {
        assert!(r.pending_selection().is_none(), "{mode}: no inherited opponent-Digimon target");
        assert_eq!(r.game.modifiers.sum(opp, ModifierType::SecurityAttackChange), 0);
        return json!({"mode":mode,"modifier":0});
    }
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    assert!(!r.pending_is_optional()); assert_eq!(r.pending_action_count(), 1);
    r.auto_resolve().expect("mandatory opponent selection");
    assert_eq!(r.game.modifiers.sum(opp, ModifierType::SecurityAttackChange), -1);
    r.end_turn();
    assert_eq!(r.game.modifiers.sum(opp, ModifierType::SecurityAttackChange), -1);
    let before = r.security_count(0); let _ = r.attack_player(opp, 0, false);
    r.auto_resolve().unwrap(); assert_eq!(r.security_count(0), before, "-1 means zero checks, not +1");
    r.end_turn(); assert_eq!(r.game.modifiers.sum(opp, ModifierType::SecurityAttackChange), 0);
    json!({"mode":mode,"modifier":-1,"actual_host_attack":true,"zero_checks":true,"expires_after_opponent_turn":true})
}
fn landramon(actual: CardData, mode: &str) -> Value {
    let mut host_data = plain("HOST", 5);
    let mut strong = plain("STRONG", 8); strong.dp = Some(30000);
    host_data.traits = vec![if mode == "wrong_trait" { "Dragon" } else if mode == "rock" { "Rock" } else { "Mineral" }.into()];
    let mut tamer = plain("TAMER", 3); tamer.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder().dsl_card("EX8-048").unwrap().add_card(actual.clone())
        .add_card(host_data).add_card(strong).add_card(plain("CHEAP4", 4)).add_card(plain("EXPENSIVE5", 5))
        .add_card(plain("DUMMY", 3)).add_card(tamer).build();
    assert_metadata(&r, &actual);
    let host = r.place_on_field(0, "HOST", None);
    let source = r.push_source(host, "EX8-048");
    let removed = if mode == "wrong_source" { r.push_source(host, "DUMMY") } else { source };
    r.place_on_field(0, "CHEAP4", None); // own cheap Digimon is never a target
    if mode != "no_legal_target" { r.place_on_field(1, "CHEAP4", None); }
    r.place_on_field(1, "EXPENSIVE5", None); r.place_on_field(1, "TAMER", None);
    let top = r.top_card(host);
    if mode == "battle_deletion" {
        let enemy = r.place_on_field(1, "STRONG", None);
        let _ = r.battle_digimon(host, enemy);
        r.auto_resolve().expect("battle deletion settles");
        assert!(!r.game.player(0).battle_area.iter().any(|p| p.top_card().card_id(&r.game.card_data) == "HOST"));
        assert!(r.game.player(0).trash.iter().any(|c| c.card_id(&r.game.card_data) == "EX8-048"));
    } else { let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0); ctx.trash_card_source(host, removed); }
    let positive = mode == "mineral" || mode == "rock";
    if positive {
        assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
        assert_eq!(r.pending_action_count(), 1);
        r.auto_resolve().expect("cost-four target deletion");
    } else { assert!(r.pending_selection().is_none(), "{mode}: forbidden source/host/target"); }
    let remaining: Vec<_> = r.game.player(1).battle_area.iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string()).collect();
    assert!(remaining.contains(&"EXPENSIVE5".into())); assert!(remaining.contains(&"TAMER".into()));
    assert_eq!(remaining.contains(&"CHEAP4".into()), !positive && mode != "no_legal_target");
    assert!(r.game.player(0).battle_area.iter().any(|p| p.top_card().card_id(&r.game.card_data) == "CHEAP4"));
    json!({"mode":mode,"opponent_remaining":remaining,"own_target_preserved":true})
}
// Enumerate the actual embedded pack, rather than searching uncompiled YAML.
fn consumer_inventory(old: &HashMap<String, CardData>, new: &HashMap<String, CardData>, controlled: &HashMap<String, CardData>) -> Value {
    fn walk(owner: &str, path: &str, value: &Value, out: &mut Vec<Value>) {
        match value {
            Value::Object(map) => for (key, child) in map {
                let p = format!("{path}/{key}");
                if key.contains("text_contains") && !child.is_null() {
                    assert!(matches!(key.as_str(), "text_contains" | "in_text_contains" | "effect_text_contains" | "rules_text_contains" | "event_card_text_contains" | "event_card_in_text_contains" | "event_target_in_text_contains"), "unreviewed text consumer: {key}");
                    let needle = child.as_str().expect("literal consumer must be a string");
                    out.push(json!({"owner":owner,"path":p,"kind":key,"needle":needle}));
                }
                walk(owner, &p, child, out);
            },
            Value::Array(items) => for (i, child) in items.iter().enumerate() { walk(owner, &format!("{path}/{i}"), child, out); },
            _ => ()
        }
    }
    fn matched(card: &CardData, kind: &str, needle: &str) -> bool {
        if matches!(kind, "effect_text_contains" | "rules_text_contains" | "event_card_text_contains") {
            let lower = needle.to_lowercase();
            [&card.effect_text, &card.inherited_text, &card.security_text].iter().any(|s| s.to_lowercase().contains(&lower))
        } else { card.in_text_contains(needle) }
    }
    let registry = digimon_engine::dsl_registry::from_embedded_cached().expect("genuine compiled pack");
    let mut pack: Vec<_> = registry.iter().collect();
    pack.sort_by(|a,b| a.0.cmp(&b.0));
    let mut inventory = Vec::new();
    let mut pack_digest = Sha256::new();
    for (id, card) in &pack {
        let value = serde_json::to_value(card).unwrap();
        pack_digest.update(serde_json::to_vec(&(id, &value)).unwrap()); pack_digest.update(b"\n");
        walk(id, "", &value, &mut inventory);
    }
    assert!(!inventory.is_empty(), "non-vacuous compiled consumer inventory");
    let mut differences = Vec::new();
    for entry in &inventory {
        let kind = entry["kind"].as_str().unwrap(); let needle = entry["needle"].as_str().unwrap();
        for id in ["EX8-011", "EX8-022", "EX8-048"] {
            let a = matched(&old[id],kind,needle); let b = matched(&new[id],kind,needle); let c = matched(&controlled[id],kind,needle);
            if a != b || a != c { differences.push(json!({"card_id":id,"consumer":entry,"baseline":a,"candidate":b,"controlled":c})); }
        }
    }
    json!({"scope":"compiled literal predicates evaluated on the three actual card rows; not whole-game equivalence",
        "compiled_card_count":pack.len(),"pack_sha256":format!("{:x}", pack_digest.finalize()),
        "literal_consumer_count":inventory.len(),"inventory":inventory,"differences":differences})
}
fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 4, "required: baseline.json candidate.json attestation.json; no fallback fixtures");
    let base_bytes = fs::read(&args[1]).expect("baseline input");
    let candidate_bytes = fs::read(&args[2]).expect("candidate input");
    let att_bytes = fs::read(&args[3]).expect("exact field attestation input");
    let att: Value = serde_json::from_slice(&att_bytes).unwrap();
    assert_eq!(att["baseline_sha256"], hash(&base_bytes)); assert_eq!(att["candidate_sha256"], hash(&candidate_bytes));
    let base: Value = serde_json::from_slice(&base_bytes).unwrap();
    let candidate: Value = serde_json::from_slice(&candidate_bytes).unwrap();
    let targets = [("EX8-011","effect_description_eng"),("EX8-011","security_effect_description_eng"),
                   ("EX8-022","inherited_effect_description_eng"),("EX8-048","inherited_effect_description_eng")];
    assert_eq!(att["fields"].as_array().unwrap().len(), targets.len());
    let mut controlled = base.clone();
    for (id, field) in targets {
        let entries: Vec<_> = att["fields"].as_array().unwrap().iter()
            .filter(|x| x["card_id"] == id && x["field_path"] == field).collect();
        assert_eq!(entries.len(), 1); let e = entries[0];
        assert_eq!(e["old_value"], base[id][field]); assert_eq!(e["new_value"], candidate[id][field]);
        assert_eq!(e["old_digest"], hash(format!("{}\n", e["old_value"]).as_bytes()));
        assert_eq!(e["new_digest"], hash(format!("{}\n", e["new_value"]).as_bytes()));
        controlled[id][field] = candidate[id][field].clone();
    }
    assert!(DebugRunner::builder().dsl_card("EX8-011").is_err(), "gap changed: reassess EX8-011 instead of claiming this diagnostic as qualification");
    let mut outcomes = Vec::new();
    for (label, rows) in [("baseline", &base), ("candidate", &candidate), ("controlled_four_fields_not_production", &controlled)] {
        let loaded = cards(rows);
        let security_gap = security_check(loaded["EX8-011"].clone(), false);
        let security_positive = security_check(loaded["EX8-066"].clone(), true);
        let a: Vec<_> = ["positive","face","wrong_source","no_digimon"].iter().map(|m| frigimon(loaded["EX8-022"].clone(), m)).collect();
        let b: Vec<_> = ["mineral","rock","wrong_trait","wrong_source","no_legal_target","battle_deletion"].iter().map(|m| landramon(loaded["EX8-048"].clone(), m)).collect();
        outcomes.push(json!({"catalog":label,"EX8-011_played":security_gap,"EX8-066_positive_control_played":security_positive,"EX8-022":a,"EX8-048":b}));
    }
    let old = cards(&base); let new = cards(&candidate);
    let consumers = consumer_inventory(&old, &new, &cards(&controlled));
    let needles = [("EX8-011", "cost.\r\n[Start of Your Main Phase]"),
                   ("EX8-022", "1 additional security card"),("EX8-048", "[Rock]\u{a0}trait")];
    let searches: Vec<_> = needles.iter().map(|(id, needle)| {
        assert!(old[*id].in_text_contains(needle), "baseline literal witness");
        assert!(!new[*id].in_text_contains(needle), "candidate differs on literal search");
        json!({"card_id":id,"needle":needle,"baseline":true,"candidate":false})
    }).collect();
    println!("{}", serde_json::to_string_pretty(&json!({"status":"SCOPED_RUNTIME_DIAGNOSTIC_NOT_ADMISSION",
        "baseline_sha256":hash(&base_bytes),"candidate_sha256":hash(&candidate_bytes),"attestation_sha256":hash(&att_bytes),
        "EX8-011":"BLOCKED_NO_EXECUTABLE_CARD_IMPLEMENTATION", "general_text_equivalence":false,
        "runtime_adjudications_emitted":false,"compiled_literal_consumers":consumers,"outcomes":outcomes,"literal_search_counterexamples":searches})).unwrap());
}
