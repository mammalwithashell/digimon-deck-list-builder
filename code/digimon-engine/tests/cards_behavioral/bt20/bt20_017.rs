//! BT20-017 Jesmon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt20_017_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT20-017")
        .expect("BT20-017 must load from embedded DSL pack")
        .start();
}

// G-ALLY-PLAYED-MAY-ATTACK substrate is RESOLVED / already-composable
// (Phase 2 Track J Task S2.1, 2026-05-20 — see qa/resolved-gaps.md). The
// `may_attack_now` step already accepts `attacker: this | event_target |
// <named binding>`, proven by the DSL behavioral tests in
// `tests/dsl/effect_granted_attack.rs`. This card-shaped test stays ignored
// pending production BT20-017.yaml authoring (Track J PR 2): it additionally
// needs the Atho/René/Por token (registered, PR 1), a `select_own_permanent`
// step for "1 of your Digimon may attack", and a `dp_lte: 8000` delete
// sub-clause.
#[ignore = "card-authoring residual: production BT20-017.yaml not yet authored (Track J PR 2). Substrate gap G-ALLY-PLAYED-MAY-ATTACK is RESOLVED — see qa/resolved-gaps.md"]
#[test]
fn bt20_017_token_and_other_played_attack_observer() {}
