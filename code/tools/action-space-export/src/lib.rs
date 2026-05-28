//! Library half of `action-space-export`. The binary (`src/main.rs`) is a thin
//! wrapper around [`build`] that pretty-prints the result to stdout. Tests in
//! `tests/` import [`build`] directly and assert the emitted JSON roundtrips
//! to the same values exported by `digimon_engine::action::space`.
//!
//! See the binary's module docs for the high-level rationale (codegen bridge
//! between the Rust action enum and external consumers like the DCGO mod).

use serde_json::{json, Value};

/// Build the JSON descriptor of the standard action space.
///
/// All values are pulled live from `digimon_engine::action::space`, so this
/// function cannot drift from the in-engine layout — any change to the engine
/// constants reshapes the emitted JSON automatically.
pub fn build() -> Value {
    use digimon_engine::action::space::*;

    // Sample encoder outputs at three points each. The Python emitter uses
    // these to sanity-check that the C# helpers it generates produce the
    // same numbers as the Rust source.
    let encode_attack_samples = json!({
        "(0, 0)": encode_attack(0, 0),
        "(1, 0)": encode_attack(1, 0),
        "(0, 1)": encode_attack(0, 1),
    });
    let encode_digivolve_samples = json!({
        "(0, 0)": encode_digivolve(0, 0),
        "(1, 0)": encode_digivolve(1, 0),
        "(0, 1)": encode_digivolve(0, 1),
    });
    let encode_source_select_samples = json!({
        "(0, 0)": encode_source_select(0, 0),
        "(1, 0)": encode_source_select(1, 0),
        "(0, 1)": encode_source_select(0, 1),
    });
    let encode_breeding_source_select_samples = json!({
        "(0, 0)": encode_breeding_source_select(0, 0),
        "(1, 0)": encode_breeding_source_select(1, 0),
        "(0, 1)": encode_breeding_source_select(0, 1),
    });

    json!({
        "schema_version": SCHEMA_VERSION,
        "action_space_size": ACTION_SPACE_SIZE,

        // Single-ID constants (no formula needed; emitted as plain C# consts).
        "constants": {
            "REPLACEMENT_ACCEPT":              REPLACEMENT_ACCEPT,
            "HATCH":                           HATCH,
            "MOVE_FROM_BREEDING":              MOVE_FROM_BREEDING,
            "PASS":                            PASS,
            "CONCEDE_GAME":                    CONCEDE_GAME,
            "PLAY_FIRST":                      PLAY_FIRST,
            "PLAY_SECOND":                     PLAY_SECOND,
            "BREEDING_SELECTION_TARGET":       BREEDING_SELECTION_TARGET,
            "BREEDING_SELECT_PLAYER_0":        BREEDING_SELECT_PLAYER_0,
            "BREEDING_SELECT_PLAYER_1":        BREEDING_SELECT_PLAYER_1,
            "SECURITY_TARGET":                 SECURITY_TARGET,
            "BREEDING_TARGET":                 BREEDING_TARGET,
            "MAX_SECURITY":                    MAX_SECURITY,
            "MAX_REVEALED":                    MAX_REVEALED,
            "TRASH_MAIN_LIMIT":                TRASH_MAIN_LIMIT,
            "HAND_MAIN_LIMIT":                 HAND_MAIN_LIMIT,
            "FIELD_EFFECT_SLOT_FOR_MAIN":      FIELD_EFFECT_SLOT_FOR_MAIN,
            "FIELD_EFFECT_SLOT_FOR_OVERCLOCK": FIELD_EFFECT_SLOT_FOR_OVERCLOCK,
            "BREEDING_SOURCE_CARRIERS":        BREEDING_SOURCE_CARRIERS,
        },

        // Stride / cardinality constants used by the C# encoder helpers and
        // by downstream consumers that need to iterate over a sub-range.
        "strides": {
            "TARGETS_PER_ATTACKER":   TARGETS_PER_ATTACKER,
            "FIELDS_PER_HAND":        FIELDS_PER_HAND,
            "EFFECTS_PER_PERMANENT":  EFFECTS_PER_PERMANENT,
            "SOURCES_PER_FIELD":      SOURCES_PER_FIELD,
            "MAX_FIELD_SLOTS":        MAX_FIELD_SLOTS,
        },

        // Half-open ID ranges [start, end). Listed in numeric start order so
        // a diff against the committed copy reflects layout changes obviously.
        "ranges": {
            "PLAY_HAND":              [PLAY_HAND_START,              PLAY_HAND_END],
            "HAND_EFFECT":            [HAND_EFFECT_START,            HAND_EFFECT_END],
            "SEL_REVEAL":             [SEL_REVEAL_START,             SEL_REVEAL_END],
            "SEL_MY_SECURITY":        [SEL_MY_SECURITY_START,        SEL_MY_SECURITY_END],
            "SEL_OPP_SECURITY":       [SEL_OPP_SECURITY_START,       SEL_OPP_SECURITY_END],
            "DNA_DIGIVOLVE":          [DNA_DIGIVOLVE_START,          DNA_DIGIVOLVE_END],
            "ATTACK":                 [ATTACK_START,                 ATTACK_END],
            "DIGIVOLVE":              [DIGIVOLVE_START,              DIGIVOLVE_END],
            "FIELD_EFFECT":           [FIELD_EFFECT_START,           FIELD_EFFECT_END],
            "TRASH_EFFECT":           [TRASH_EFFECT_START,           TRASH_EFFECT_END],
            "SOURCE_SELECT":          [SOURCE_SELECT_START,          SOURCE_SELECT_END],
            "BREEDING_SOURCE_SELECT": [BREEDING_SOURCE_SELECT_START, BREEDING_SOURCE_SELECT_END],
        },

        // For each 2D-formula range, the emitter needs: start, two stride
        // dimensions, and sample encodings the C# helpers must reproduce.
        // The formula is `id = start + a * stride_a + b` (stride_b is always 1).
        "formulas": {
            "encode_attack": {
                "start":    ATTACK_START,
                "stride_a": TARGETS_PER_ATTACKER,   // attacker_index stride
                "max_a":    MAX_FIELD_SLOTS,        // attackers are field slots
                "max_b":    TARGETS_PER_ATTACKER,   // target ∈ [0, 14] (14 = security)
                "samples":  encode_attack_samples,
            },
            "encode_digivolve": {
                "start":    DIGIVOLVE_START,
                "stride_a": FIELDS_PER_HAND,        // hand_index stride
                "max_a":    HAND_MAIN_LIMIT as u16, // hand_index ∈ [0, 30)
                "max_b":    FIELDS_PER_HAND,        // field ∈ [0, 14] (14 = breeding)
                "samples":  encode_digivolve_samples,
            },
            "encode_field_effect": {
                "start":    FIELD_EFFECT_START,
                "stride_a": EFFECTS_PER_PERMANENT,
                "max_a":    MAX_FIELD_SLOTS,
                "max_b":    EFFECTS_PER_PERMANENT,
                "samples":  json!({
                    "(2, 5)": FIELD_EFFECT_START + 2 * EFFECTS_PER_PERMANENT + 5,
                }),
            },
            "encode_source_select": {
                "start":    SOURCE_SELECT_START,
                "stride_a": SOURCES_PER_FIELD,
                "max_a":    MAX_FIELD_SLOTS,
                "max_b":    SOURCES_PER_FIELD,
                "samples":  encode_source_select_samples,
            },
            "encode_breeding_source_select": {
                "start":    BREEDING_SOURCE_SELECT_START,
                "stride_a": SOURCES_PER_FIELD,        // carrier_owner stride
                "max_a":    BREEDING_SOURCE_CARRIERS, // exactly two players in standard
                "max_b":    SOURCES_PER_FIELD,
                "samples":  encode_breeding_source_select_samples,
            },
        },
    })
}
