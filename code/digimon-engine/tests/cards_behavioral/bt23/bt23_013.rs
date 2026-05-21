//! BT23-013 Jesmon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt23_013_loads_keyword_slice() {
    DebugRunner::builder()
        .dsl_card("BT23-013")
        .expect("BT23-013 must load from embedded DSL pack")
        .start();
}

// Substrate is RESOLVED for both halves of this card. The hand+trash
// name-excluded Sistermon play (`G-UNION-HAND-TRASH-NAME-EXCLUSION`) was
// closed by Phase 2 Track J Task S2.2 — `select_union_zone` now applies
// its `filter`, and the `name_not_shared_by_field_digimon` predicate leaf
// models "can't play cards with the same names as any of your Digimon".
// See `tests/dsl/s2_2_union_hand_trash_name_exclusion.rs` and
// `qa/resolved-gaps.md`. The ally-played may-attack observer was closed by
// Task S2.1 (`G-ALLY-PLAYED-MAY-ATTACK`, already-composable). This
// card-shaped test stays ignored pending production BT23-013.yaml
// authoring (Track J PR 3): it additionally needs the Atho/René/Por token
// (registered, PR 1), the `<Rush>` / `<Alliance>` keyword slices, and an
// effect-choice between the token play and the Sistermon union play.
#[ignore = "card-authoring residual: production BT23-013.yaml not yet authored (Track J PR 3). Substrate gaps G-UNION-HAND-TRASH-NAME-EXCLUSION and G-ALLY-PLAYED-MAY-ATTACK are RESOLVED — see qa/resolved-gaps.md"]
#[test]
fn bt23_013_token_sistermon_union_play_and_attack_observer() {}
