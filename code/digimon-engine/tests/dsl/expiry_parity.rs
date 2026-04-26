//! Parity test: the validator's `KNOWN_EXPIRY_KEYS` and the engine's
//! `lookup_expiry` must accept exactly the same set of strings.
//!
//! Why this matters: when the validator green-lights an expiry string the
//! engine doesn't recognize, the modifier step silently no-ops at runtime
//! (`modifiers.rs`: `let Some(expiry) = lookup_expiry(expiry) else { return true; };`).
//! That's a silent miscompile — the YAML parses, the validator passes, and
//! the modifier never applies. This test fails the build the moment the
//! two lists drift.

use std::collections::BTreeSet;

use digimon_dsl::validator::KNOWN_EXPIRY_KEYS;
use digimon_engine::dsl_cards::expiry_map::{all_engine_expiry_keys, lookup_expiry};

#[test]
fn validator_keys_and_engine_keys_are_equal_sets() {
    let validator: BTreeSet<&str> = KNOWN_EXPIRY_KEYS.iter().copied().collect();
    let engine: BTreeSet<&str> = all_engine_expiry_keys().iter().copied().collect();

    let only_validator: Vec<_> = validator.difference(&engine).collect();
    let only_engine: Vec<_> = engine.difference(&validator).collect();

    assert!(
        only_validator.is_empty() && only_engine.is_empty(),
        "expiry-key drift between digimon-dsl validator and digimon-engine lookup_expiry. \
         Only in validator (would silently no-op at runtime): {only_validator:?}. \
         Only in engine (validator would reject valid YAML): {only_engine:?}."
    );
}

#[test]
fn every_validator_key_resolves_in_engine() {
    for &key in KNOWN_EXPIRY_KEYS {
        assert!(
            lookup_expiry(key).is_some(),
            "validator accepts {key:?} but lookup_expiry returns None — \
             would silently drop the modifier at runtime"
        );
    }
}

#[test]
fn every_engine_key_is_in_validator_list() {
    let validator: BTreeSet<&str> = KNOWN_EXPIRY_KEYS.iter().copied().collect();
    for &key in all_engine_expiry_keys() {
        assert!(
            validator.contains(key),
            "engine recognizes {key:?} but validator's KNOWN_EXPIRY_KEYS omits it — \
             card authors writing this key would get spurious validation failures"
        );
    }
}
