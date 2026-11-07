//! Direct raw-declarative construction requests, NOT a safety certificate.
//! Runtime steps/formulas and transitive helper behavior are separate facts.
use super::raw_rust::EngineRawRustRegistry;
use digimon_dsl::compiled::CompiledCard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClauseRegion {
    Card,
    DigimonFace,
    OptionFace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawFactoryRequest {
    pub(crate) region: ClauseRegion,
    pub(crate) clause_index: usize,
    pub(crate) name: String,
    pub(crate) registered_declarative: bool,
}

pub(crate) fn direct_raw_factory_requests(
    card: &CompiledCard,
    raw: &EngineRawRustRegistry,
) -> Vec<RawFactoryRequest> {
    use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause as D};

    fn collect(
        clauses: &[CompiledClause],
        region: ClauseRegion,
        raw: &EngineRawRustRegistry,
        out: &mut Vec<RawFactoryRequest>,
    ) {
        for (clause_index, clause) in clauses.iter().enumerate() {
            // Enumerate variants explicitly: additions require reviewing their
            // direct factory-construction semantics. These negative arms say
            // nothing about runtime callbacks or transitive helper safety.
            let name = match clause {
                CompiledClause::Triggered(_) => None,
                CompiledClause::Declarative(declarative) => match declarative {
                    D::RawRust { fn_name, .. } => Some(fn_name),
                    D::Aura { .. }
                    | D::CostReduction { .. }
                    | D::Replacement { .. }
                    | D::Partition { .. }
                    | D::AceOverflow { .. }
                    | D::GrantKeyword { .. }
                    | D::Delay { .. }
                    | D::LinkRequirement { .. }
                    | D::LinkCondition { .. }
                    | D::UseCostIncrease { .. }
                    | D::Succession { .. }
                    | D::FloodGate { .. }
                    | D::AltPathRegistration { .. } => None,
                },
            };
            if let Some(name) = name {
                out.push(RawFactoryRequest {
                    region,
                    clause_index,
                    name: name.clone(),
                    // Name membership in a step/formula namespace is NOT a
                    // declarative factory. Lookup never invokes the callback.
                    registered_declarative: raw.declarative_fn(name).is_some(),
                });
            }
        }
    }

    let mut out = Vec::new();
    collect(&card.effects, ClauseRegion::Card, raw, &mut out);
    // Match effects() construction order and presence checks, including IR
    // supplied directly without validated-pack authorship.
    if let Some(dual) = &card.dual {
        collect(&dual.digimon.effects, ClauseRegion::DigimonFace, raw, &mut out);
        collect(&dual.option.effects, ClauseRegion::OptionFace, raw, &mut out);
    }
    out
}

#[cfg(test)]
#[path = "../determinize/context_guard/construction_facts_tests.rs"]
mod tests;
