//! Complete hand-partner DNA feasibility for each stage of a normal selection.
use super::*;
use digimon_dsl::{compiled::CompiledDnaHandCandidate, predicate::DnaHandCandidateRole};

fn bound_hand_card(rctx: &EffectReadContext<'_>, bindings: Option<&Bindings>, name: &str) -> Option<CardHandle> {
    let bindings = bindings?;
    match bindings.get_ref(name)? {
        BindingValue::HandIndex(owner, original) if *owner == rctx.player => {
            let hand = &rctx.game.player(*owner).hand;
            let index = match bindings.resolve_pinned_hand_index(name, hand) {
                Some(index) => index?,
                None => *original,
            };
            hand.get(index as usize).map(|card| card.handle())
        }
        BindingValue::Card(card) if rctx.game.player(rctx.player).hand.iter().any(|live| live.handle() == *card) => Some(*card),
        _ => None,
    }
}

pub(super) fn dna_hand_candidate(
    spec: &CompiledDnaHandCandidate,
    rctx: &EffectReadContext<'_>,
    subject: PredicateSubject,
    bindings: Option<&Bindings>,
) -> bool {
    let hand = &rctx.game.player(rctx.player).hand;
    let result = match spec.role {
        DnaHandCandidateRole::Result => match subject {
            PredicateSubject::Card(card) => card,
            _ => return false,
        },
        _ => match spec.result.as_deref().and_then(|name| bound_hand_card(rctx, bindings, name)) {
            Some(card) => card,
            None => return false,
        },
    };
    if !hand.iter().any(|card| card.handle() == result) { return false; }
    let fixed_field = match spec.role {
        DnaHandCandidateRole::Result => None,
        DnaHandCandidateRole::Field => match subject {
            PredicateSubject::Permanent(field) => Some(field),
            _ => return false,
        },
        DnaHandCandidateRole::Partner => match spec.field.as_deref()
            .and_then(|name| resolve_cost_probe_target(rctx, bindings, name)) {
                Some(field) => Some(field),
                None => return false,
            },
    };
    let fixed_partner = if spec.role == DnaHandCandidateRole::Partner {
        match subject {
            PredicateSubject::Card(card) => Some(card),
            _ => return false,
        }
    } else { None };
    let delta = crate::dsl_cards::step::play_digivolve::lower_cost_delta_read(
        Some(&spec.cost), rctx, bindings,
    );
    for index in 0..rctx.game.player(rctx.player).battle_area.len() {
        let field = PermanentHandle { player: rctx.player, index: index as u8 };
        if fixed_field.is_some_and(|fixed| fixed != field) { continue; }
        if spec.field_filter.as_ref().is_some_and(|filter| !eval_predicate_with_bindings(
            filter, rctx, PredicateSubject::Permanent(field), bindings,
        )) { continue; }
        for partner in hand {
            let partner = partner.handle();
            if partner == result || fixed_partner.is_some_and(|fixed| fixed != partner) { continue; }
            if spec.partner_filter.as_ref().is_some_and(|filter| !eval_predicate_with_bindings(
                filter, rctx, PredicateSubject::Card(partner), bindings,
            )) { continue; }
            if !rctx.game.effect_dna_hand_routes(
                rctx.player, field, partner, result, delta, spec.ignore_requirements,
                rctx.player, rctx.source_kind, rctx.source_card, rctx.source_permanent,
            ).is_empty() { return true; }
        }
    }
    false
}
