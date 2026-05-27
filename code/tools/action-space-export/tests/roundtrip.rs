//! Roundtrip tests for the action-space JSON exporter.
//!
//! These tests are the contract that protects downstream consumers from
//! silent breakage. If the engine's action layout changes and the exporter
//! disagrees with `digimon_engine::action::space`, these tests fail loudly.

use digimon_engine::action::space as space;
use serde_json::Value;

fn descriptor() -> Value {
    action_space_export::build()
}

#[test]
fn schema_version_matches_engine() {
    let v = descriptor();
    assert_eq!(v["schema_version"].as_u64().unwrap(), space::SCHEMA_VERSION as u64);
}

#[test]
fn action_space_size_matches_engine() {
    let v = descriptor();
    assert_eq!(v["action_space_size"].as_u64().unwrap(), space::ACTION_SPACE_SIZE as u64);
}

#[test]
fn single_id_constants_match_engine() {
    let v = descriptor();
    let consts = &v["constants"];

    // Spot-check every single-ID constant. If any of these drift the
    // recorder will mis-encode silently — these asserts are the wall.
    assert_eq!(consts["REPLACEMENT_ACCEPT"].as_u64().unwrap(), space::REPLACEMENT_ACCEPT as u64);
    assert_eq!(consts["HATCH"].as_u64().unwrap(), space::HATCH as u64);
    assert_eq!(consts["MOVE_FROM_BREEDING"].as_u64().unwrap(), space::MOVE_FROM_BREEDING as u64);
    assert_eq!(consts["PASS"].as_u64().unwrap(), space::PASS as u64);
    assert_eq!(consts["CONCEDE_GAME"].as_u64().unwrap(), space::CONCEDE_GAME as u64);
    assert_eq!(consts["PLAY_FIRST"].as_u64().unwrap(), space::PLAY_FIRST as u64);
    assert_eq!(consts["PLAY_SECOND"].as_u64().unwrap(), space::PLAY_SECOND as u64);
    assert_eq!(consts["BREEDING_SELECTION_TARGET"].as_u64().unwrap(), space::BREEDING_SELECTION_TARGET as u64);
    assert_eq!(consts["BREEDING_SELECT_PLAYER_0"].as_u64().unwrap(), space::BREEDING_SELECT_PLAYER_0 as u64);
    assert_eq!(consts["BREEDING_SELECT_PLAYER_1"].as_u64().unwrap(), space::BREEDING_SELECT_PLAYER_1 as u64);
    assert_eq!(consts["SECURITY_TARGET"].as_u64().unwrap(), space::SECURITY_TARGET as u64);
    assert_eq!(consts["BREEDING_TARGET"].as_u64().unwrap(), space::BREEDING_TARGET as u64);
    assert_eq!(consts["MAX_SECURITY"].as_u64().unwrap(), space::MAX_SECURITY as u64);
    assert_eq!(consts["MAX_REVEALED"].as_u64().unwrap(), space::MAX_REVEALED as u64);
    assert_eq!(consts["TRASH_MAIN_LIMIT"].as_u64().unwrap(), space::TRASH_MAIN_LIMIT as u64);
    assert_eq!(consts["HAND_MAIN_LIMIT"].as_u64().unwrap(), space::HAND_MAIN_LIMIT as u64);
    assert_eq!(consts["FIELD_EFFECT_SLOT_FOR_MAIN"].as_u64().unwrap(), space::FIELD_EFFECT_SLOT_FOR_MAIN as u64);
    assert_eq!(consts["FIELD_EFFECT_SLOT_FOR_OVERCLOCK"].as_u64().unwrap(), space::FIELD_EFFECT_SLOT_FOR_OVERCLOCK as u64);
    assert_eq!(consts["BREEDING_SOURCE_CARRIERS"].as_u64().unwrap(), space::BREEDING_SOURCE_CARRIERS as u64);
}

#[test]
fn strides_match_engine() {
    let v = descriptor();
    let s = &v["strides"];
    assert_eq!(s["TARGETS_PER_ATTACKER"].as_u64().unwrap(), space::TARGETS_PER_ATTACKER as u64);
    assert_eq!(s["FIELDS_PER_HAND"].as_u64().unwrap(), space::FIELDS_PER_HAND as u64);
    assert_eq!(s["EFFECTS_PER_PERMANENT"].as_u64().unwrap(), space::EFFECTS_PER_PERMANENT as u64);
    assert_eq!(s["SOURCES_PER_FIELD"].as_u64().unwrap(), space::SOURCES_PER_FIELD as u64);
    assert_eq!(s["MAX_FIELD_SLOTS"].as_u64().unwrap(), space::MAX_FIELD_SLOTS as u64);
}

#[test]
fn ranges_match_engine() {
    let v = descriptor();
    let r = &v["ranges"];

    fn pair(v: &Value) -> (u64, u64) {
        let arr = v.as_array().unwrap();
        (arr[0].as_u64().unwrap(), arr[1].as_u64().unwrap())
    }
    assert_eq!(pair(&r["PLAY_HAND"]),              (space::PLAY_HAND_START as u64,              space::PLAY_HAND_END as u64));
    assert_eq!(pair(&r["HAND_EFFECT"]),            (space::HAND_EFFECT_START as u64,            space::HAND_EFFECT_END as u64));
    assert_eq!(pair(&r["SEL_REVEAL"]),             (space::SEL_REVEAL_START as u64,             space::SEL_REVEAL_END as u64));
    assert_eq!(pair(&r["SEL_MY_SECURITY"]),        (space::SEL_MY_SECURITY_START as u64,        space::SEL_MY_SECURITY_END as u64));
    assert_eq!(pair(&r["SEL_OPP_SECURITY"]),       (space::SEL_OPP_SECURITY_START as u64,       space::SEL_OPP_SECURITY_END as u64));
    assert_eq!(pair(&r["DNA_DIGIVOLVE"]),          (space::DNA_DIGIVOLVE_START as u64,          space::DNA_DIGIVOLVE_END as u64));
    assert_eq!(pair(&r["ATTACK"]),                 (space::ATTACK_START as u64,                 space::ATTACK_END as u64));
    assert_eq!(pair(&r["DIGIVOLVE"]),              (space::DIGIVOLVE_START as u64,              space::DIGIVOLVE_END as u64));
    assert_eq!(pair(&r["FIELD_EFFECT"]),           (space::FIELD_EFFECT_START as u64,           space::FIELD_EFFECT_END as u64));
    assert_eq!(pair(&r["TRASH_EFFECT"]),           (space::TRASH_EFFECT_START as u64,           space::TRASH_EFFECT_END as u64));
    assert_eq!(pair(&r["SOURCE_SELECT"]),          (space::SOURCE_SELECT_START as u64,          space::SOURCE_SELECT_END as u64));
    assert_eq!(pair(&r["BREEDING_SOURCE_SELECT"]), (space::BREEDING_SOURCE_SELECT_START as u64, space::BREEDING_SOURCE_SELECT_END as u64));
}

#[test]
fn formula_samples_match_engine_encoders() {
    // The C# emitter trusts the `samples` blocks as the authoritative
    // ground truth for what its generated `EncodeAttack` / `EncodeDigivolve`
    // / etc. must reproduce. Pin every sample to the actual encoder output
    // so the emitter can't ship a wrong formula.
    let v = descriptor();
    let f = &v["formulas"];

    assert_eq!(f["encode_attack"]["samples"]["(0, 0)"].as_u64().unwrap(), space::encode_attack(0, 0) as u64);
    assert_eq!(f["encode_attack"]["samples"]["(1, 0)"].as_u64().unwrap(), space::encode_attack(1, 0) as u64);
    assert_eq!(f["encode_attack"]["samples"]["(0, 1)"].as_u64().unwrap(), space::encode_attack(0, 1) as u64);

    assert_eq!(f["encode_digivolve"]["samples"]["(0, 0)"].as_u64().unwrap(), space::encode_digivolve(0, 0) as u64);
    assert_eq!(f["encode_digivolve"]["samples"]["(1, 0)"].as_u64().unwrap(), space::encode_digivolve(1, 0) as u64);
    assert_eq!(f["encode_digivolve"]["samples"]["(0, 1)"].as_u64().unwrap(), space::encode_digivolve(0, 1) as u64);

    // Source-select and breeding-source-select encoders return Option<u16>;
    // unwrap for the in-range samples we pinned.
    assert_eq!(
        f["encode_source_select"]["samples"]["(0, 0)"].as_u64().unwrap(),
        space::encode_source_select(0, 0).unwrap() as u64
    );
    assert_eq!(
        f["encode_source_select"]["samples"]["(1, 0)"].as_u64().unwrap(),
        space::encode_source_select(1, 0).unwrap() as u64
    );
    assert_eq!(
        f["encode_source_select"]["samples"]["(0, 1)"].as_u64().unwrap(),
        space::encode_source_select(0, 1).unwrap() as u64
    );

    assert_eq!(
        f["encode_breeding_source_select"]["samples"]["(0, 0)"].as_u64().unwrap(),
        space::encode_breeding_source_select(0, 0).unwrap() as u64
    );
    assert_eq!(
        f["encode_breeding_source_select"]["samples"]["(1, 0)"].as_u64().unwrap(),
        space::encode_breeding_source_select(1, 0).unwrap() as u64
    );
    assert_eq!(
        f["encode_breeding_source_select"]["samples"]["(0, 1)"].as_u64().unwrap(),
        space::encode_breeding_source_select(0, 1).unwrap() as u64
    );
}

#[test]
fn ranges_do_not_overlap_in_their_top_level_groups() {
    // Sanity check the descriptor itself — top-level macro ranges should
    // never overlap. (Sub-ranges like SEL_REVEAL deliberately share IDs
    // with HAND_EFFECT via phase disambiguation, so we test the macro
    // ranges only: PLAY_HAND, HAND_EFFECT, DNA_DIGIVOLVE, ATTACK,
    // DIGIVOLVE, FIELD_EFFECT, TRASH_EFFECT, SOURCE_SELECT, BREEDING_SOURCE_SELECT.)
    let names = [
        "PLAY_HAND", "HAND_EFFECT", "DNA_DIGIVOLVE",
        "ATTACK", "DIGIVOLVE", "FIELD_EFFECT", "TRASH_EFFECT",
        "SOURCE_SELECT", "BREEDING_SOURCE_SELECT",
    ];
    let v = descriptor();
    let ranges = &v["ranges"];
    let mut pairs: Vec<(u64, u64)> = names.iter().map(|n| {
        let arr = ranges[*n].as_array().unwrap();
        (arr[0].as_u64().unwrap(), arr[1].as_u64().unwrap())
    }).collect();
    pairs.sort();
    for w in pairs.windows(2) {
        assert!(
            w[0].1 <= w[1].0,
            "macro ranges overlap: [{},{}) and [{},{})",
            w[0].0, w[0].1, w[1].0, w[1].1
        );
    }
}

#[test]
fn no_id_in_descriptor_exceeds_action_space_size() {
    let v = descriptor();
    let size = v["action_space_size"].as_u64().unwrap();

    // Range endpoints
    for (_name, r) in v["ranges"].as_object().unwrap() {
        let end = r.as_array().unwrap()[1].as_u64().unwrap();
        assert!(end <= size, "range end {} exceeds action_space_size {}", end, size);
    }

    // Reserved single IDs
    for (name, val) in v["constants"].as_object().unwrap() {
        // Skip non-action-ID constants like MAX_SECURITY, TRASH_MAIN_LIMIT,
        // BREEDING_SOURCE_CARRIERS, etc. — they're array sizes, not action IDs.
        let is_action_id = matches!(name.as_str(),
            "REPLACEMENT_ACCEPT" | "HATCH" | "MOVE_FROM_BREEDING" | "PASS"
            | "CONCEDE_GAME" | "PLAY_FIRST" | "PLAY_SECOND"
        );
        if is_action_id {
            let id = val.as_u64().unwrap();
            assert!(id < size, "{} = {} >= action_space_size {}", name, id, size);
        }
    }
}
