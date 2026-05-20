//! ST19-08 ShoeShoemon.
//! Printed text:
//! - [Security] You may play 1 [LIBERATOR] card with play cost 4 or less
//!   from hand or trash free.
//! - <Overclock ([Puppet] Trait)>.
//! - Inherited [Your Turn] all opponent security Digimon get -3000 DP.
//!
//! Partial: the Security hand-or-trash union play is blocked by the current
//! union-zone DSL lowering, which binds only a CardHandle and ignores filters,
//! so it cannot faithfully play the selected card from its original zone.
//! The inherited opponent security Digimon DP aura remains blocked by
//! G-OPPONENT-SECURITY-DP-AURA / PUPPETS-G008.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledDeclarativeClause};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

#[test]
fn st19_08_yaml_loads() {
    let _runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .start();
}

#[test]
fn st19_08_grants_overclock_with_puppet_cost_filter() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .start();

    let shoe = runner.place_on_field(0, "ST19-08", Some(0));

    assert!(runner.game.has_keyword(shoe, Keyword::Overclock));

    let compiled = runner
        .compiled_card("ST19-08")
        .expect("ST19-08 must be compiled");
    let overclock_clause = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                overclock_cost_filter,
                ..
            }) => keyword
                .eq_ignore_ascii_case("Overclock")
                .then_some(overclock_cost_filter),
            _ => None,
        })
        .expect("ST19-08 must grant Overclock");
    let filter = overclock_clause
        .as_ref()
        .expect("Overclock must carry a Puppet/token sacrifice filter");

    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.kind == Some(CompiledCardKind::Token)),
        "Overclock cost allows deleting one of your Tokens"
    );
    assert!(
        filter.any_of.iter().any(|branch| {
            branch
                .all_of
                .iter()
                .any(|leaf| leaf.trait_has.as_deref() == Some("Puppet"))
        }),
        "Overclock cost allows other Puppet trait Digimon"
    );
}

#[test]
fn st19_08_security_may_play_liberator_cost_4_or_less_from_hand_or_trash() {
    let runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("ST19-08")
        .expect("ST19-08 must be compiled");

    assert!(
        compiled.effects.iter().any(|clause| match clause {
            CompiledClause::Triggered(triggered) => {
                triggered
                    .when
                    .contains(&digimon_dsl::compiled::CompiledTiming::OnSecurity)
                    && triggered.optional
            }
            _ => false,
        }),
        "Security text should compile to an optional on_security union-zone play"
    );
}

#[test]
#[ignore = "pending: G-OPPONENT-SECURITY-DP-AURA / PUPPETS-G008 - DSL cannot express inherited applies_to_opponent_security_dp"]
fn st19_08_inherited_reduces_opponent_security_digimon_dp_during_your_turn() {
    let runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("ST19-08")
        .expect("ST19-08 must be compiled");

    assert!(
        compiled.effects.iter().any(|clause| match clause {
            CompiledClause::Declarative(aura) => format!("{aura:?}").contains("opponent_security"),
            _ => false,
        }),
        "Inherited aura should lower to opponent-security-Digimon DP adjustment"
    );
}
