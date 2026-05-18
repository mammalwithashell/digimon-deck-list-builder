//! EX7-024 Shoemon.
//!
//! Printed text from `data/cards.json`:
//! - [Your Turn] When this Digimon would digivolve into a Digimon card with the
//!   [Puppet] trait, reduce the digivolution cost by 1.
//! - Inherited: [Your Turn] All of your opponent's Security Digimon get -3000 DP.
//!
//! Current verdict: PARTIAL. The card's printed metadata and yellow Lv2
//! digivolution path are implemented in production DSL. Both printed effects are
//! blocked by reusable DSL/engine vocabulary gaps already used by adjacent
//! Puppet cards:
//! - no `when_this_digivolves_into` + target-trait cost-reduction hook.
//! - no YAML bridge to `EffectBuilder::applies_to_opponent_security_dp()`.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCardKind, CompiledColor, CompiledCost};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ex7_024_compiles_with_printed_stats_and_lv2_yellow_path() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card("EX7-024")
        .expect("EX7-024 in compiled cards");

    assert_eq!(compiled.card, "EX7-024");
    assert_eq!(compiled.name, "Shoemon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    for trait_name in ["Puppet", "LIBERATOR"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(2)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(2)))
                        && (from.color_is == Some(CompiledColor::Yellow)
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.color_is == Some(CompiledColor::Yellow)))
                })
        }),
        "Shoemon must digivolve from a yellow Lv2 for cost 0"
    );

    assert_eq!(
        compiled.effects.len(),
        1,
        "EX7-024 should only encode the inherited opponent-security DP aura; \
         the source-scoped digivolve-into-trait cost reducer remains blocked"
    );
}

#[test]
#[ignore = "pending: DSL/engine gap - no when_this_digivolves_into + target_trait_has cost-reduction hook"]
fn ex7_024_cost_reduction_reduces_digivolving_into_puppet_by_one() {
    todo!("unignore when the DSL can express source-scoped digivolve-into-trait cost reduction")
}

#[test]
#[ignore = "pending: DSL/engine gap - same as ex7_024_cost_reduction_reduces_digivolving_into_puppet_by_one"]
fn ex7_024_cost_reduction_does_not_apply_to_non_puppet_target() {
    todo!("unignore when target-trait filtering is available to digivolution cost reduction")
}

#[test]
#[ignore = "pending: DSL/engine gap - same as ex7_024_cost_reduction_reduces_digivolving_into_puppet_by_one"]
fn ex7_024_cost_reduction_only_applies_on_your_turn() {
    todo!("unignore when source-scoped [Your Turn] digivolution cost reduction is available")
}

#[test]
#[ignore = "pending: G-OPPONENT-SECURITY-DP-AURA / PUPPETS-G008 - no DSL bridge to applies_to_opponent_security_dp"]
fn ex7_024_inherited_security_dp_aura_drops_opponent_security_digimon_by_3000() {
    todo!("unignore when inherited opponent security DP aura can be authored in YAML")
}

#[test]
#[ignore = "pending: G-OPPONENT-SECURITY-DP-AURA / PUPPETS-G008 - same as positive security DP aura test"]
fn ex7_024_inherited_security_dp_aura_only_applies_on_your_turn() {
    todo!("unignore when inherited opponent security DP aura can be authored in YAML")
}
