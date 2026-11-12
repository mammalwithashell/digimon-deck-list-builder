//! Complete hand-partner DNA feasibility for each stage of a normal selection.
use super::*;
use digimon_dsl::{
    compiled::{CompiledDnaFieldPair, CompiledDnaHandCandidate},
    predicate::{DnaFieldPairRole, DnaHandCandidateRole},
};

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

/// Both-on-field DNA feasibility (G-DSL-DNA-FIELD-PAIR-CANDIDATE): "2 of your
/// Digimon may DNA digivolve into X in the hand". A pair qualifies when the two
/// distinct own battle-area Digimon satisfy one of the result's printed DNA
/// requirements (`matching_dna_cost_for_handles`, the same oracle the
/// `effect_initiated_dna_digivolve` commit backstop uses) and the printed DNA
/// cost is payable. DCGO `CanPlayJogress(true)` / `CanJogressFromTargetPermanents`.
///
/// Affordability reads the printed cost without speculative reducers; a
/// reducer can only make the cost cheaper, so this never hides a pair that a
/// reducer-free payment would allow.
pub(super) fn dna_field_pair(
    spec: &CompiledDnaFieldPair,
    rctx: &EffectReadContext<'_>,
    subject: PredicateSubject,
    bindings: Option<&Bindings>,
) -> bool {
    let player = rctx.player;
    let game = rctx.game;
    if game
        .modifiers
        .player_has(player, crate::enums::ModifierType::CannotDigivolveDigimonByEffect)
    {
        return false;
    }
    let result = match spec.role {
        DnaFieldPairRole::Result => match subject {
            PredicateSubject::Card(card) => card,
            _ => return false,
        },
        _ => match spec.result.as_deref().and_then(|name| bound_hand_card(rctx, bindings, name)) {
            Some(card) => card,
            None => return false,
        },
    };
    let Some(result_card) = game.player(player).hand.iter().find(|c| c.handle() == result) else {
        return false;
    };
    let Some(meta) = game.card_data.get(result_card.data_index) else {
        return false;
    };
    if !matches!(meta.card_kind, crate::enums::CardKind::Digimon | crate::enums::CardKind::Dual) {
        return false;
    }
    let fixed_first = match spec.role {
        DnaFieldPairRole::Result => None,
        DnaFieldPairRole::First => match subject {
            PredicateSubject::Permanent(handle) => Some(handle),
            _ => return false,
        },
        DnaFieldPairRole::Second => match spec
            .first
            .as_deref()
            .and_then(|name| resolve_cost_probe_target(rctx, bindings, name))
        {
            Some(handle) => Some(handle),
            None => return false,
        },
    };
    let fixed_second = match spec.role {
        DnaFieldPairRole::Second => match subject {
            PredicateSubject::Permanent(handle) => Some(handle),
            _ => return false,
        },
        _ => None,
    };
    let field_len = game.player(player).battle_area.len();
    let own = |index: usize| PermanentHandle { player, index: index as u8 };
    let is_digimon = |handle: PermanentHandle| {
        handle.player == player
            && game
                .player(player)
                .battle_area
                .get(handle.index as usize)
                .is_some_and(|perm| perm.is_digimon(&game.card_data))
    };
    for a in 0..field_len {
        let a = own(a);
        if fixed_first.is_some_and(|fixed| fixed != a) || !is_digimon(a) {
            continue;
        }
        for b in 0..field_len {
            let b = own(b);
            if a == b || fixed_second.is_some_and(|fixed| fixed != b) || !is_digimon(b) {
                continue;
            }
            if let Some(cost) = game.matching_dna_cost_for_handles(result_card, meta, a, b) {
                if game.can_pay_memory_for(player, cost.memory_cost.max(0) as u16) {
                    return true;
                }
            }
        }
    }
    false
}
