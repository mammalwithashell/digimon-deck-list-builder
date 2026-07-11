//! `scope: both` cost-reduction — shared once-per-turn accounting across the
//! two lowered copies (G-ENGINE-SHARED-OPT-SCOPE-BOTH-REDUCER).
//!
//! A `scope: both` cost-reduction clause must emanate from BOTH the active-top
//! (FaceUp) and digivolution-source (Inherited) positions, so a card whose
//! reducer reads "[All Turns] … reduce … by N" applies whether it is the top
//! card or a source beneath a later digivolution. When that clause is
//! once-per-turn, the two lowered copies MUST share ONE OPT accounting slot
//! (DCGO `SetHashString`) — otherwise a card could reduce a cost once as the
//! active top and AGAIN the same turn after being buried as a source (the
//! double-reduction window).
//!
//! Before this fix `scope: both` on a cost reducer silently collapsed to ONE
//! FaceUp-only effect (the `if matches!(scope, Inherited)` guard never fired
//! for `Both`), so it worked only when the card was the top and never as a
//! source — a latent authoring bug that also meant the double-reduction window
//! was unreachable. These tests lock in the corrected lowering.

use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::enums::EffectTiming;
use digimon_engine::CardEffect;

fn lower(yaml: &str) -> Vec<digimon_engine::effect::Effect> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("YAML compiles");
    let dsl = DslCardEffect::new(Arc::new(compiled));
    dsl.effects(CardHandle(99))
}

/// A once-per-turn `scope: both` cost reducer.
const BOTH_OPT_REDUCER: &str = r#"
card: TEST-BOTH-OPT
name: Test Both OPT Reducer
kind: digimon
level: 5
color: [black]
cost: 8
dp: 8000
effects:
  - kind: cost_reduction
    scope: both
    once_per_turn: true
    when_playing_this: false
    amount: 2
    active_when: { all_turns: true }
"#;

/// A once-per-turn `scope: inherited` cost reducer (the BT18-060 / BT11-061
/// Vemmon shape) — single copy, no shared group needed.
const INHERITED_OPT_REDUCER: &str = r#"
card: TEST-INH-OPT
name: Test Inherited OPT Reducer
kind: digimon
level: 5
color: [black]
cost: 8
dp: 8000
effects:
  - kind: cost_reduction
    scope: inherited
    once_per_turn: true
    when_any_ally_digivolves_into: { name_contains: "Galacticmon" }
    amount: 1
"#;

/// A `scope: both` reducer that is NOT once-per-turn — two copies, but no
/// shared group (nothing to lock out).
const BOTH_NON_OPT_REDUCER: &str = r#"
card: TEST-BOTH-NOOPT
name: Test Both Non-OPT Reducer
kind: digimon
level: 5
color: [black]
cost: 8
dp: 8000
effects:
  - kind: cost_reduction
    scope: both
    once_per_turn: false
    when_playing_this: false
    amount: 2
    active_when: { all_turns: true }
"#;

fn reducers(effects: &[digimon_engine::effect::Effect]) -> Vec<&digimon_engine::effect::Effect> {
    effects
        .iter()
        .filter(|e| e.timing == EffectTiming::BeforePayCost)
        .collect()
}

#[test]
fn scope_both_opt_reducer_lowers_to_two_copies_sharing_one_opt_group() {
    let effects = lower(BOTH_OPT_REDUCER);
    let reducers = reducers(&effects);
    assert_eq!(
        reducers.len(),
        2,
        "scope: both must emit BOTH a FaceUp and an Inherited cost-reduction copy"
    );

    // One copy is face-up (inherited == false), the other is inherited.
    let face_up = reducers.iter().find(|e| !e.inherited);
    let inherited = reducers.iter().find(|e| e.inherited);
    assert!(
        face_up.is_some(),
        "one copy must be the FaceUp (top) position"
    );
    assert!(
        inherited.is_some(),
        "the other copy must be the Inherited (source) position"
    );

    // Both must carry the SAME shared_opt_group so they lock out together.
    let fg = face_up.unwrap().shared_opt_group;
    let ig = inherited.unwrap().shared_opt_group;
    assert!(
        fg.is_some(),
        "the FaceUp copy must carry a shared OPT group"
    );
    assert_eq!(
        fg, ig,
        "both scope:both copies must share the SAME OPT group id (single per-turn lockout)"
    );

    // Both are still once-per-turn (max_per_turn == 1).
    assert_eq!(face_up.unwrap().max_per_turn, 1);
    assert_eq!(inherited.unwrap().max_per_turn, 1);
}

#[test]
fn scope_inherited_opt_reducer_has_no_shared_group() {
    // The Vemmon shape: a single inherited-only reducer. No sibling to share
    // with, so no shared group — keeps the default per-slot OPT keying.
    let effects = lower(INHERITED_OPT_REDUCER);
    let reducers = reducers(&effects);
    assert_eq!(reducers.len(), 1, "scope: inherited emits exactly one copy");
    assert!(reducers[0].inherited, "the single copy is inherited");
    assert_eq!(
        reducers[0].shared_opt_group, None,
        "a single-scope reducer needs no shared OPT group"
    );
    assert_eq!(reducers[0].max_per_turn, 1);
}

#[test]
fn scope_both_non_opt_reducer_emits_two_copies_without_shared_group() {
    // Without once_per_turn there is nothing to lock out, so the two copies
    // carry no shared group (each just applies whenever eligible).
    let effects = lower(BOTH_NON_OPT_REDUCER);
    let reducers = reducers(&effects);
    assert_eq!(
        reducers.len(),
        2,
        "scope: both still emits both positions even when not OPT"
    );
    for r in &reducers {
        assert_eq!(
            r.shared_opt_group, None,
            "a non-OPT scope:both reducer needs no shared group"
        );
        assert_eq!(r.max_per_turn, 0, "not once-per-turn");
    }
    // Still one FaceUp + one Inherited.
    assert!(reducers.iter().any(|e| !e.inherited));
    assert!(reducers.iter().any(|e| e.inherited));
}
