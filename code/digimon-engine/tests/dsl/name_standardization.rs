//! English exclusions in Bandai Language Standardization Rules (2026-05-29).
//! The denominator is twelve exact-name/query pairs, not all languages.
use digimon_engine::action::build_action_mask;
use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    EffectTiming, TriggerSource,
};
use digimon_engine::{Expiry, ModifierType};

const PAIRS: &[(&str, &str)] = &[
    ("Pagumon", "Agumon"),
    ("DemiVeemon", "Vee"),
    ("DemiVeemon", "Veemon"),
    ("KendoGarurumon", "Garurumon"),
    ("BurningGreymon", "Greymon"),
    ("DoruGreymon", "Greymon"),
    ("DexDoruGreymon", "Greymon"),
    ("Indramon", "Dramon"),
    ("BeelStarmon", "Starmon"),
    ("BeelStarmon (X Antibody)", "Starmon"),
    ("Blimpmon", "Impmon"),
    ("MasterBlimpmon", "Impmon"),
];
const SELECT: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick matching name
          bind_as: pick
          optional: true
          filter: { name_contains: QUERY }
      - gain_memory: 1
"#;
const FIELD: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    condition: { any_permanent: { of: you, name_contains: QUERY } }
    process: [{ gain_memory: 1 }]
"#;
const DELETION: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_any_deletion
    trigger_if: { event_target_name_contains: QUERY }
    process: [{ gain_memory: 1 }]
"#;
const EVENT_CARD: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_any_deletion
    trigger_if: { event_card_name_contains: QUERY }
    process: [{ gain_memory: 1 }]
"#;
const DNA: &str = r#"
card: RESULT
name: Result
kind: digimon
level: 6
color: [red]
cost: 10
dp: 10000
alt_paths:
  - kind: dna_digivolve
    materials:
      - { name_contains: Greymon }
      - { name_contains: Garurumon }
    cost: 0
"#;

fn runner(yaml: &str, name: &str, query: &str, static_alias: bool) -> DebugRunner {
    let mut excluded = make_test_card("CANDIDATE", name);
    if name.contains(" / ") {
        excluded.card_kind = digimon_engine::CardKind::Dual;
        excluded.dual = Some(serde_json::from_value(serde_json::json!({
            "digimon": {"level": 6, "dp": 12000, "colors": ["Purple"], "traits": [], "evo_costs": [], "effect_text": "", "inherited_text": ""},
            "option": {"use_cost": 7, "colors": ["Purple"], "effect_text": "", "security_text": "", "keywords": []}
        })).unwrap());
    }
    if static_alias {
        excluded.also_treated_as.push(query.into());
    }
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml.replace("QUERY", query))
        .unwrap()
        .add_card(excluded)
        .add_card(make_test_card("YES", query))
        .add_card(make_test_card("YES2", &format!("Mega{query}")))
        .hand(0, &["CANDIDATE", "YES", "YES2"])
        .deck(0, &["YES"; 10])
        .deck(1, &["YES"; 10])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn fire(r: &mut DebugRunner) {
    let h = r.place_on_field(0, "PROBE", Some(0));
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn select(r: &mut DebugRunner, expected: &[u16]) {
    fire(r);
    let p = r.game.pending_selection.as_ref().expect("real choice");
    assert_eq!(p.valid_action_ids, expected);
    let mask = build_action_mask(&r.game, 0);
    for a in 0..3 {
        assert_eq!(
            mask[a],
            if expected.contains(&(a as u16)) {
                1.0
            } else {
                0.0
            }
        );
    }
    r.execute_action(0, *expected.last().unwrap()).unwrap();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 6);
}
#[test]
fn all_twelve_english_exclusions_filter_real_hand_choices_case_insensitively() {
    for &(name, query) in PAIRS {
        for (name, query) in [
            (name.to_string(), query.to_string()),
            (name.to_uppercase(), query.to_lowercase()),
        ] {
            select(&mut runner(SELECT, &name, &query, false), &[1, 2]);
            // One-fact delta: an ordinary containing name is not the excluded whole name.
            select(
                &mut runner(SELECT, &format!("Mega{name}"), &query, false),
                &[0, 1, 2],
            );
        }
    }
}
#[test]
fn explicit_static_and_runtime_aliases_rescue_excluded_base_names() {
    for &(name, query) in PAIRS {
        select(&mut runner(SELECT, name, query, true), &[0, 1, 2]);
        let mut r = runner(SELECT, name, query, false);
        r.game.players[0].hand[0].also_treated_as.push(query.into());
        select(&mut r, &[0, 1, 2]);
    }
}
#[test]
fn permanent_effective_name_and_deleted_snapshot_apply_exclusions() {
    for yaml in [FIELD, DELETION, EVENT_CARD] {
        for name in ["BurningGreymon", "MetalGreymon"] {
            let mut r = runner(yaml, "Agumon", "Greymon", false);
            let h = r.place_on_field(0, "CANDIDATE", Some(0));
            r.game.modifiers.add(
                h,
                ModifierEntry::simple(ModifierType::ChangeBaseCardName, 0, Expiry::Permanent, 0)
                    .with_payload(ModifierPayload::Name {
                        value: name.into(),
                        base: true,
                    }),
            );
            if yaml == FIELD {
                fire(&mut r);
            } else {
                r.place_on_field(0, "PROBE", Some(0));
                r.game.delete_permanents_batch(
                    vec![h],
                    digimon_engine::replacement::ReplacementCause::OpponentEffect,
                );
            }
            assert_eq!(
                r.memory(),
                if name == "MetalGreymon" { 6 } else { 5 },
                "{yaml}: {name}"
            );
        }
    }
}
#[test]
fn exact_name_equality_is_unchanged() {
    let yaml = SELECT.replace("name_contains: QUERY", "name_is: BurningGreymon");
    select(
        &mut runner(&yaml, "BurningGreymon", "BurningGreymon", false),
        &[0, 1],
    );
}
#[test]
fn combined_dual_name_does_not_leak_excluded_digimon_face() {
    select(
        &mut runner(SELECT, "BeelStarmon / Fly Bullet", "Starmon", false),
        &[1, 2],
    );
    select(
        &mut runner(SELECT, "BeelStarmon / Fly Bullet", "Fly Bullet", false),
        &[0, 1, 2],
    );
    select(
        &mut runner(SELECT, "BeelStarmon / Starmon", "Starmon", false),
        &[0, 1, 2],
    );
}

#[test]
fn legacy_digixros_name_helpers_share_the_exclusions() {
    use digimon_engine::digixros::{
        matches_digixros_name_requirement_for_test, matches_generic_name_requirement_for_test,
    };
    for &(name, query) in PAIRS {
        let mut card = make_test_card("MATERIAL", name);
        assert!(!matches_digixros_name_requirement_for_test(&card, query));
        assert!(!matches_generic_name_requirement_for_test(&card, query));
        card.digixros_aliases.push(query.into());
        assert!(matches_digixros_name_requirement_for_test(&card, query));
        assert!(!matches_generic_name_requirement_for_test(&card, query));
    }
}

#[test]
fn dna_name_requirements_reject_excluded_material_and_resolve_valid_pair() {
    use digimon_engine::action::space::DNA_DIGIVOLVE_START;
    for name in ["BurningGreymon", "MetalGreymon"] {
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(DNA)
            .unwrap()
            .add_card(make_test_card("FIRST", name))
            .add_card(make_test_card("SECOND", "Garurumon"))
            .hand(0, &["RESULT"])
            .deck(0, &["SECOND"; 10])
            .deck(1, &["SECOND"; 10])
            .memory(5)
            .start();
        r.skip_mulligan();
        r.game.enter_main_phase();
        r.place_on_field(0, "FIRST", Some(0));
        r.place_on_field(0, "SECOND", Some(0));
        let valid = name == "MetalGreymon";
        assert_eq!(
            build_action_mask(&r.game, 0)[DNA_DIGIVOLVE_START as usize],
            if valid { 1.0 } else { 0.0 }
        );
        if valid {
            r.game.decode_action(DNA_DIGIVOLVE_START, 0);
            while let Some(p) = &r.game.pending_selection {
                let a = p.valid_action_ids[0];
                assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
                r.execute_action(0, a).unwrap();
            }
            assert_eq!(r.battle_area_size(0), 1);
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                "RESULT"
            );
        }
    }
}
