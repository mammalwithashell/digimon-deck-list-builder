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
    // A material named on a `result` stage was picked before the result
    // (G-DSL-DNA-FIELD-PAIR-FIXED-MATERIALS: BT20-036 / EX3-008 bind
    // `source` + `dna_partner`): the result must fit exactly that pair. A
    // named material that is gone fits nothing.
    let named = |name: Option<&str>| -> Result<Option<PermanentHandle>, ()> {
        match name {
            None => Ok(None),
            Some(name) => resolve_cost_probe_target(rctx, bindings, name).map(Some).ok_or(()),
        }
    };
    let fixed_first = match spec.role {
        DnaFieldPairRole::Result => match named(spec.first.as_deref()) {
            Ok(handle) => handle,
            Err(()) => return false,
        },
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
        DnaFieldPairRole::Result => match named(spec.second.as_deref()) {
            Ok(handle) => handle,
            Err(()) => return false,
        },
        DnaFieldPairRole::First => None,
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
                // `cost` mirrors the consuming step; omitted = printed.
                let pay = match spec.cost.as_ref() {
                    None => i32::from(cost.memory_cost).max(0),
                    Some(delta) => crate::dsl_cards::step::play_digivolve::lower_dna_cost_read(
                        delta,
                        rctx,
                        bindings,
                        || Some(i32::from(cost.memory_cost)),
                    ),
                };
                if game.can_pay_memory_for(player, pay.clamp(0, i32::from(u16::MAX)) as u16) {
                    return true;
                }
            }
        }
    }
    false
}

/// Field + TRASH DNA feasibility (G-DSL-DNA-TRASH-PARTNER-CANDIDATE), the
/// candidate oracle for `effect_initiated_dna_digivolve_trash_partner`
/// ("1 of your [Machinedramon] in play and 1 [Kimeramon] in the trash may DNA
/// digivolve into [Millenniummon] in the hand": BT18-015, BT18-073). True when
/// some (own battle-area Digimon, own trash card) pair, consistent with the
/// stage's prior bindings and filters, passes every check the commit makes
/// (`EffectContext::effect_initiated_dna_digivolve_trash_partner`): the owner
/// is not under `CannotDigivolveDigimonByEffect`, the pair satisfies one of
/// the result's printed DNA requirements unless `ignore_requirements`, and
/// the lowered `cost` is payable. A pick that offers a card no pair can use
/// would be a choice with no legal follow-up (CR 4.3 1-3-2).
pub(super) fn dna_trash_candidate(
    spec: &CompiledDnaHandCandidate,
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
        DnaHandCandidateRole::Result => match subject {
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
    let fixed_field = match spec.role {
        DnaHandCandidateRole::Result => None,
        DnaHandCandidateRole::Field => match subject {
            PredicateSubject::Permanent(field) => Some(field),
            _ => return false,
        },
        DnaHandCandidateRole::Partner => match spec
            .field
            .as_deref()
            .and_then(|name| resolve_cost_probe_target(rctx, bindings, name))
        {
            Some(field) => Some(field),
            None => return false,
        },
    };
    let fixed_partner = match spec.role {
        DnaHandCandidateRole::Partner => match subject {
            PredicateSubject::Card(card) => Some(card),
            _ => return false,
        },
        _ => None,
    };
    let trash = &game.player(player).trash;
    for index in 0..game.player(player).battle_area.len() {
        let field = PermanentHandle { player, index: index as u8 };
        if fixed_field.is_some_and(|fixed| fixed != field) {
            continue;
        }
        if spec.field_filter.as_ref().is_some_and(|filter| {
            !eval_predicate_with_bindings(filter, rctx, PredicateSubject::Permanent(field), bindings)
        }) {
            continue;
        }
        for partner in trash {
            if fixed_partner.is_some_and(|fixed| fixed != partner.handle()) {
                continue;
            }
            if spec.partner_filter.as_ref().is_some_and(|filter| {
                !eval_predicate_with_bindings(
                    filter,
                    rctx,
                    PredicateSubject::Card(partner.handle()),
                    bindings,
                )
            }) {
                continue;
            }
            let Some(partner_meta) = game.card_data.get(partner.data_index) else {
                continue;
            };
            let printed = game
                .matching_dna_cost_for_handle_and_card(result_card, meta, field, partner_meta)
                .map(|cost| i32::from(cost.memory_cost));
            if printed.is_none() && !spec.ignore_requirements {
                continue;
            }
            let pay = crate::dsl_cards::step::play_digivolve::lower_dna_cost_read(
                &spec.cost,
                rctx,
                bindings,
                || printed,
            );
            if game.can_pay_memory_for(player, pay.clamp(0, i32::from(u16::MAX)) as u16) {
                return true;
            }
        }
    }
    false
}
