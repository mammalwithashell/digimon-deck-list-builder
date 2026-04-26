//! Predicate evaluator. Phase 1c Task 3: leaf fields + combinators + existentials.

use digimon_dsl::compiled::{
    CompiledBindingCompare, CompiledCardKind, CompiledColor, CompiledExistential,
    CompiledPlayerRef, CompiledPredicate, CompiledZone,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectReadContext;
use crate::enums::{CardColor, CardKind, PlayerId};
use crate::permanent::PermanentHandle;

/// The subject a predicate is applied to.
#[derive(Debug, Clone, Copy)]
pub enum PredicateSubject {
    Permanent(PermanentHandle),
    Card(CardHandle),
    None,
}

pub fn eval_predicate(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    subject: PredicateSubject,
) -> bool {
    eval_predicate_with_bindings(pred, rctx, subject, None)
}

pub fn eval_predicate_with_bindings(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    subject: PredicateSubject,
    bindings: Option<&Bindings>,
) -> bool {
    // Game-state fields — independent of subject.
    if let Some(want) = pred.your_turn {
        let is_my = rctx.game.turn_player() == rctx.player;
        if is_my != want {
            return false;
        }
    }
    if let Some(want) = pred.opponents_turn {
        let is_opp = rctx.game.turn_player() != rctx.player;
        if is_opp != want {
            return false;
        }
    }
    if let Some(cap) = pred.memory_lte {
        if (rctx.game.memory as i32) > cap {
            return false;
        }
    }
    if let Some(floor) = pred.memory_gte {
        if (rctx.game.memory as i32) < floor {
            return false;
        }
    }
    if let Some(cap) = pred.security_count_lte {
        if rctx.security_count(rctx.player) as u8 > cap {
            return false;
        }
    }
    if let Some(floor) = pred.security_count_gte {
        if (rctx.security_count(rctx.player) as u8) < floor {
            return false;
        }
    }
    if !eval_event_fields(pred, rctx) {
        return false;
    }

    // Combinators — short-circuit on first failure.
    for child in &pred.all_of {
        if !eval_predicate_with_bindings(child, rctx, subject, bindings) {
            return false;
        }
    }
    if !pred.any_of.is_empty() {
        let any_match = pred
            .any_of
            .iter()
            .any(|c| eval_predicate_with_bindings(c, rctx, subject, bindings));
        if !any_match {
            return false;
        }
    }
    for child in &pred.none_of {
        if eval_predicate_with_bindings(child, rctx, subject, bindings) {
            return false;
        }
    }
    if let Some(inner) = &pred.not {
        if eval_predicate_with_bindings(inner, rctx, subject, bindings) {
            return false;
        }
    }

    if let Some(values) = &pred.equals {
        if !compare_binding_values(values, bindings, |a, b| a == b) {
            return false;
        }
    }
    if let Some(values) = &pred.not_equals {
        if !compare_binding_values(values, bindings, |a, b| a != b) {
            return false;
        }
    }

    // Existentials — scan battle areas.
    if let Some(ex) = &pred.any_permanent {
        if !existential_any(ex, rctx, bindings) {
            return false;
        }
    }
    if let Some(ex) = &pred.no_permanent {
        if existential_any(ex, rctx, bindings) {
            return false;
        }
    }
    if let Some(ex) = &pred.all_permanents {
        if !existential_all(ex, rctx, bindings) {
            return false;
        }
    }

    match subject {
        PredicateSubject::Card(card) => eval_card_fields(pred, rctx, card),
        PredicateSubject::Permanent(h) => eval_permanent_fields(pred, rctx, h),
        PredicateSubject::None => eval_no_subject_fields(pred),
    }
}

fn compare_binding_values(
    values: &[CompiledBindingCompare],
    bindings: Option<&Bindings>,
    cmp: impl Fn(i64, i64) -> bool,
) -> bool {
    let Some((first, rest)) = values.split_first() else {
        return false;
    };
    let Some(left) = resolve_compare_value(first, bindings) else {
        return false;
    };
    rest.iter()
        .all(|right| resolve_compare_value(right, bindings).is_some_and(|r| cmp(left, r)))
}

fn resolve_compare_value(
    value: &CompiledBindingCompare,
    bindings: Option<&Bindings>,
) -> Option<i64> {
    match value {
        CompiledBindingCompare::Literal(n) => Some(*n),
        CompiledBindingCompare::Binding(name) => bindings?.get_literal(name),
    }
}

fn existential_any(
    ex: &CompiledExistential,
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> bool {
    for p in existential_players(ex.of, rctx) {
        let n = rctx.game.player(p).battle_area.len();
        for i in 0..n {
            let handle = PermanentHandle {
                player: p,
                index: i as u8,
            };
            if eval_predicate_with_bindings(
                &ex.predicate,
                rctx,
                PredicateSubject::Permanent(handle),
                bindings,
            ) {
                return true;
            }
        }
    }
    false
}

fn existential_all(
    ex: &CompiledExistential,
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> bool {
    let mut any_seen = false;
    for p in existential_players(ex.of, rctx) {
        let n = rctx.game.player(p).battle_area.len();
        for i in 0..n {
            any_seen = true;
            let handle = PermanentHandle {
                player: p,
                index: i as u8,
            };
            if !eval_predicate_with_bindings(
                &ex.predicate,
                rctx,
                PredicateSubject::Permanent(handle),
                bindings,
            ) {
                return false;
            }
        }
    }
    any_seen
}

fn existential_players(of: CompiledPlayerRef, rctx: &EffectReadContext<'_>) -> Vec<PlayerId> {
    match of {
        CompiledPlayerRef::You => vec![rctx.player],
        CompiledPlayerRef::Opponent => vec![rctx.opponent_id()],
        CompiledPlayerRef::Active => vec![rctx.game.turn_player()],
        CompiledPlayerRef::Any => (0..rctx.game.players.len() as PlayerId).collect(),
    }
}

fn eval_no_subject_fields(pred: &CompiledPredicate) -> bool {
    // If any subject-only field is set, this subjectless eval can't satisfy it.
    pred.kind.is_none()
        && pred.level_eq.is_none()
        && pred.level_lte.is_none()
        && pred.level_gte.is_none()
        && pred.color_is.is_none()
        && pred.color_only.is_none()
        && pred.trait_has.is_none()
        && pred.form_is.is_none()
        && pred.attribute_is.is_none()
        && pred.name_is.is_none()
        && pred.name_contains.is_none()
        && pred.name_in.is_none()
        && pred.card_number_is.is_none()
}

fn eval_event_fields(pred: &CompiledPredicate, rctx: &EffectReadContext<'_>) -> bool {
    if let Some(want) = pred.event_target_kind {
        let Some(card) = event_target_card(rctx) else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(card) else {
            return false;
        };
        if !kind_matches(want, data.card_kind) {
            return false;
        }
    }
    if let Some(ref trait_name) = pred.event_target_trait_has {
        let Some(card) = event_target_card(rctx) else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(card) else {
            return false;
        };
        if !data
            .traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case(trait_name))
        {
            return false;
        }
    }
    if let Some(ref trait_name) = pred.event_card_trait_has {
        let Some(card) = rctx
            .game
            .current_trigger_context
            .and_then(|trigger| trigger.event_card)
        else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(card) else {
            return false;
        };
        if !data
            .traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case(trait_name))
        {
            return false;
        }
    }
    true
}

fn event_target_card(rctx: &EffectReadContext<'_>) -> Option<CardHandle> {
    let trigger = rctx.game.current_trigger_context?;
    if let Some(handle) = trigger.target_permanent {
        if let Some(card) = rctx
            .game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|perm| perm.top_card().handle())
        {
            return Some(card);
        }
    }
    trigger.target_card
}

fn eval_card_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    card: CardHandle,
) -> bool {
    let data = match rctx.game.card_data_for_handle(card) {
        Some(d) => d,
        None => return false,
    };

    if let Some(want) = pred.kind {
        if !kind_matches(want, data.card_kind) {
            return false;
        }
    }
    if let Some(want) = pred.level_eq {
        if data.level != Some(want) {
            return false;
        }
    }
    if let Some(cap) = pred.level_lte {
        if data.level.map_or(true, |l| l > cap) {
            return false;
        }
    }
    if let Some(floor) = pred.level_gte {
        if data.level.map_or(true, |l| l < floor) {
            return false;
        }
    }
    if let Some(want) = pred.color_is {
        if !data.colors.iter().any(|c| color_matches(want, *c)) {
            return false;
        }
    }
    if let Some(ref allowed) = pred.color_only {
        for c in &data.colors {
            if !allowed.iter().any(|a| color_matches(*a, *c)) {
                return false;
            }
        }
    }
    if let Some(ref t) = pred.trait_has {
        if !data.traits.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            return false;
        }
    }
    if pred.form_is.is_some() {
        // CardData has no `form` field yet; engine doesn't track form.
        // Phase 1c: treat as always-false when set (mirrors "no card matches").
        return false;
    }
    if pred.attribute_is.is_some() {
        // Same as form — attribute not yet tracked on CardData.
        return false;
    }
    if let Some(ref n) = pred.name_is {
        if data.card_name != *n {
            return false;
        }
    }
    if let Some(ref n) = pred.name_contains {
        if !data.card_name.to_lowercase().contains(&n.to_lowercase()) {
            return false;
        }
    }
    if let Some(ref names) = pred.name_in {
        if !names.iter().any(|n| n == &data.card_name) {
            return false;
        }
    }
    if let Some(ref cn) = pred.card_number_is {
        if data.card_id != *cn {
            return false;
        }
    }
    true
}

fn eval_permanent_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
) -> bool {
    let perm = match rctx
        .game
        .player(handle.player)
        .battle_area
        .get(handle.index as usize)
    {
        Some(p) => p,
        None => return false,
    };
    // Delegate the shared card fields to the card-handle path using the top card.
    let top_handle = perm.top_card().handle();
    if !eval_card_fields(pred, rctx, top_handle) {
        return false;
    }
    if let Some(want) = pred.is_suspended {
        if perm.is_suspended != want {
            return false;
        }
    }
    if let Some(want) = pred.is_unsuspended {
        if perm.is_suspended == want {
            return false;
        }
    }
    if let Some(cap) = pred.stack_size_lte {
        if perm.card_sources.len() as u8 > cap {
            return false;
        }
    }
    if let Some(floor) = pred.stack_size_gte {
        if (perm.card_sources.len() as u8) < floor {
            return false;
        }
    }
    if !pred.zone.is_empty() && !pred.zone.contains(&CompiledZone::BattleArea) {
        // Permanents always live in BattleArea — any zone list missing it fails.
        return false;
    }
    if let Some(want) = pred.owner {
        let matches = match want {
            CompiledPlayerRef::You => handle.player == rctx.player,
            CompiledPlayerRef::Opponent => handle.player == rctx.opponent_id(),
            CompiledPlayerRef::Active => handle.player == rctx.game.turn_player(),
            CompiledPlayerRef::Any => true,
        };
        if !matches {
            return false;
        }
    }
    true
}

fn kind_matches(want: CompiledCardKind, got: CardKind) -> bool {
    matches!(
        (want, got),
        (CompiledCardKind::Digimon, CardKind::Digimon)
            | (CompiledCardKind::Tamer, CardKind::Tamer)
            | (CompiledCardKind::Option, CardKind::Option)
            | (CompiledCardKind::DigiEgg, CardKind::DigiEgg)
    )
}

fn color_matches(want: CompiledColor, got: CardColor) -> bool {
    matches!(
        (want, got),
        (CompiledColor::Red, CardColor::Red)
            | (CompiledColor::Blue, CardColor::Blue)
            | (CompiledColor::Yellow, CardColor::Yellow)
            | (CompiledColor::Green, CardColor::Green)
            | (CompiledColor::Black, CardColor::Black)
            | (CompiledColor::Purple, CardColor::Purple)
            | (CompiledColor::White, CardColor::White)
    )
}
