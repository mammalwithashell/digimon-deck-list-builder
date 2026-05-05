//! BT20-083 Omekamon - Digimon, Lv.4, White.
//!
//! Supported slice:
//! - <Blocker>.
//! - [On Play] If you have 1 or fewer security cards, may digivolve this
//!   Digimon into [Omnimon (X Antibody)] in hand ignoring requirements/free.
//!
//! Gap-routed:
//! - [On Deletion] place this card under [King Drasil_7D6] in breeding needs a
//!   filtered breeding permanent target.
//! - Inherited [Breeding][Opponent's Turn] security-removed trigger needs
//!   breeding inherited observer dispatch plus source-play from materials.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledCostDelta, CompiledDeclarativeClause,
    CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT20-083")
        .expect("BT20-083 YAML loads")
        .memory(10)
        .start()
}

#[test]
fn bt20_083_has_printed_metadata() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-083")
        .expect("BT20-083 compiled card present");

    assert_eq!(card.name, "Omekamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "Puppet"));
    assert!(card.traits.iter().any(|name| name == "X Antibody"));
    assert!(card.traits.iter().any(|name| name == "LIBERATOR"));
    assert_eq!(card.attribute.as_deref(), Some("Data"));
}

#[test]
fn bt20_083_has_blocker_grant_and_low_security_on_play_digivolve() {
    let runner = runner();
    let card = runner
        .compiled_card("BT20-083")
        .expect("BT20-083 compiled card present");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            ..
        }) if keyword == "Blocker"
    )));

    let on_play = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("low-security On Play digivolve clause exists");

    assert!(on_play.optional, "printed digivolve says 'may'");
    assert_eq!(
        on_play
            .condition
            .as_ref()
            .and_then(|predicate| predicate.security_count_lte),
        Some(1),
        "On Play clause is gated to 1 or fewer own security"
    );
    assert!(
        on_play.process.iter().any(|step| matches!(
            step,
            CompiledStep::SelectHand { filter, .. }
                if filter.name_is.as_deref() == Some("Omnimon (X Antibody)")
        )),
        "On Play must select [Omnimon (X Antibody)] from hand"
    );
    assert!(
        on_play.process.iter().any(|step| matches!(
            step,
            CompiledStep::EffectInitiatedDigivolve {
                cost: CompiledCostDelta::Literal(0),
                ignore_requirements: true,
                ..
            }
        )),
        "selected hand card must digivolve this Omekamon for free, ignoring requirements"
    );
}

#[test]
#[ignore = "pending: RK-G001 — filtered select_own_breeding_permanent target for [King Drasil_7D6]"]
fn bt20_083_on_deletion_places_self_under_king_drasil_only() {
    panic!("requires filtered breeding permanent selection before this clause can be authored");
}

#[test]
#[ignore = "pending: G-BREEDING-TRIGGER-DISPATCH plus source-play from materials"]
fn bt20_083_inherited_breeding_security_removed_suspends_carrier_and_plays_omekamon_source() {
    panic!("requires breeding inherited security-removed observer and source-play support");
}
