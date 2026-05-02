//! Predicate evaluator. Phase 1c Task 3: leaf fields + combinators + existentials.

use digimon_dsl::compiled::{
    CompiledBindingCompare, CompiledCardKind, CompiledColor, CompiledDpConstraint,
    CompiledExistential, CompiledPlayerRef, CompiledPredicate, CompiledReplacementCause,
    CompiledZone,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::{BindingValue, Bindings};
use crate::dsl_cards::formula_eval;
use crate::effect_context::EffectReadContext;
use crate::enums::{CardColor, CardKind, PlayerId};
use crate::permanent::PermanentHandle;

/// The subject a predicate is applied to.
#[derive(Debug, Clone, Copy)]
pub enum PredicateSubject {
    Permanent(PermanentHandle),
    BreedingPermanent(PlayerId),
    Card(CardHandle),
    RevealedCard(CardHandle),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateContext {
    CardSearchAny,
    DigimonCardSearch,
    OptionCardSearch,
    FieldDigimon,
    OptionUse,
    DigivolutionRequirement,
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
    if !eval_replacement_fields(pred, rctx) {
        return false;
    }
    if let Some(want) = pred.in_breeding {
        let is_in_breeding = match subject {
            PredicateSubject::Permanent(h) => {
                h.index == crate::action::space::BREEDING_TARGET as u8
            }
            _ => rctx
                .source_permanent
                .is_some_and(|h| h.index == crate::action::space::BREEDING_TARGET as u8),
        };
        if is_in_breeding != want {
            return false;
        }
    }
    if let Some(want) = pred.on_field {
        let is_on_field = match subject {
            PredicateSubject::Permanent(h) => {
                h.index != crate::action::space::BREEDING_TARGET as u8
            }
            _ => rctx
                .source_permanent
                .is_some_and(|h| h.index != crate::action::space::BREEDING_TARGET as u8),
        };
        if is_on_field != want {
            return false;
        }
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
    if let Some(binding_name) = &pred.not_in_binding {
        if !subject_not_in_binding(subject, binding_name, bindings) {
            return false;
        }
    }

    // Existentials — scan battle areas.
    if let Some(ex) = &pred.any_permanent {
        if !existential_any(ex, rctx, bindings) {
            return false;
        }
    }
    if let Some(ex) = &pred.any_field_permanent {
        if !field_existential_any(ex, rctx, bindings) {
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
        PredicateSubject::Card(card) => eval_card_fields(pred, rctx, card, false),
        PredicateSubject::RevealedCard(card) => eval_card_fields(pred, rctx, card, true),
        PredicateSubject::Permanent(h) => eval_permanent_fields(pred, rctx, h),
        PredicateSubject::BreedingPermanent(player) => {
            eval_breeding_permanent_fields(pred, rctx, player)
        }
        PredicateSubject::None => eval_no_subject_fields(pred),
    }
}

fn eval_replacement_fields(pred: &CompiledPredicate, rctx: &EffectReadContext<'_>) -> bool {
    if let Some(want) = pred.replacement_cause {
        let Some(actual) = rctx.replacement_cause() else {
            return false;
        };
        if !replacement_cause_matches(want, actual) {
            return false;
        }
    }
    if let Some(want) = pred.replacement_source_is_opponent {
        let Some(controller) = rctx.replacement_source_controller() else {
            return false;
        };
        let is_opponent = controller != rctx.player();
        if is_opponent != want {
            return false;
        }
    }
    if let Some(want) = pred.replacement_subject_is_mine {
        let Some(controller) = rctx.replacement_subject_controller() else {
            return false;
        };
        let is_mine = controller == rctx.player();
        if is_mine != want {
            return false;
        }
    }
    true
}

fn replacement_cause_matches(
    want: CompiledReplacementCause,
    actual: crate::replacement::ReplacementCause,
) -> bool {
    matches!(
        (want, actual),
        (
            CompiledReplacementCause::Battle,
            crate::replacement::ReplacementCause::Battle
        ) | (
            CompiledReplacementCause::OwnEffect,
            crate::replacement::ReplacementCause::OwnEffect
        ) | (
            CompiledReplacementCause::OpponentEffect,
            crate::replacement::ReplacementCause::OpponentEffect
        ) | (
            CompiledReplacementCause::SecurityCheck,
            crate::replacement::ReplacementCause::SecurityCheck
        ) | (
            CompiledReplacementCause::Cost,
            crate::replacement::ReplacementCause::Cost
        )
    )
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

fn field_existential_any(
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
        if rctx.game.player(p).breeding_area.is_some()
            && eval_predicate_with_bindings(
                &ex.predicate,
                rctx,
                PredicateSubject::BreedingPermanent(p),
                bindings,
            )
        {
            return true;
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
        && pred.play_cost_lte.is_none()
        && pred.dp_eq.is_none()
        && pred.dp_lte.is_none()
        && pred.dp_gte.is_none()
        && pred.not_in_binding.is_none()
}

fn subject_not_in_binding(
    subject: PredicateSubject,
    binding_name: &str,
    bindings: Option<&Bindings>,
) -> bool {
    let PredicateSubject::Permanent(handle) = subject else {
        return false;
    };
    let Some(value) = bindings.and_then(|b| b.get_ref(binding_name)) else {
        return false;
    };
    match value {
        BindingValue::Permanent(bound) => *bound != handle,
        BindingValue::PermanentList(list) => !list.contains(&handle),
        _ => false,
    }
}

fn eval_event_fields(pred: &CompiledPredicate, rctx: &EffectReadContext<'_>) -> bool {
    if let Some(want) = pred.event_target_kind {
        let Some(card) = event_target_card(rctx) else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(card) else {
            return false;
        };
        if !kind_matches_card_search(want, data) {
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
    if let Some(ref needle) = pred.event_card_name_contains {
        if !rctx.event_card_name_contains(needle) {
            return false;
        }
    }
    true
}

fn event_target_card(rctx: &EffectReadContext<'_>) -> Option<CardHandle> {
    let trigger = rctx.game.current_trigger_context?;
    if let Some(handle) = trigger.event_permanent {
        if let Some(card) = live_event_permanent_card(rctx, handle, trigger.event_card) {
            return Some(card);
        }
        return trigger.event_card;
    }
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
    if let Some(card) = trigger.target_card {
        return Some(card);
    }
    trigger.event_card
}

fn live_event_permanent_card(
    rctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
    expected: Option<CardHandle>,
) -> Option<CardHandle> {
    let card = rctx
        .game
        .player(handle.player)
        .battle_area
        .get(handle.index as usize)
        .map(|perm| perm.top_card().handle())?;
    match expected {
        Some(expected) if card != expected => None,
        _ => Some(card),
    }
}

fn eval_card_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    card: CardHandle,
    reveal_overlay_visible: bool,
) -> bool {
    let data = match rctx.game.card_data_for_handle(card) {
        Some(d) => d,
        None => return false,
    };
    if reveal_overlay_visible && !pred.zone.is_empty() && !pred.zone.contains(&CompiledZone::Reveal)
    {
        return false;
    }
    let overlay = reveal_overlay_visible
        .then(|| {
            rctx.game
                .card_source_for_handle(card)
                .and_then(|source| source.reveal_overlay.as_ref())
        })
        .flatten();

    if let Some(want) = pred.kind {
        let overlay_match = overlay
            .and_then(|o| o.kind)
            .is_some_and(|kind| kind_matches(want, kind));
        if !kind_matches_card_search(want, data) && !overlay_match {
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
        let overlay_match = overlay
            .and_then(|o| o.name.as_ref())
            .is_some_and(|name| name == n);
        if data.card_name != *n && !overlay_match {
            return false;
        }
    }
    if let Some(ref n) = pred.name_contains {
        let needle = n.to_lowercase();
        let overlay_match = overlay
            .and_then(|o| o.name.as_ref())
            .is_some_and(|name| name.to_lowercase().contains(&needle));
        if !data.card_name.to_lowercase().contains(&needle) && !overlay_match {
            return false;
        }
    }
    if let Some(ref names) = pred.name_in {
        let overlay_match = overlay
            .and_then(|o| o.name.as_ref())
            .is_some_and(|name| names.iter().any(|n| n == name));
        if !names.iter().any(|n| n == &data.card_name) && !overlay_match {
            return false;
        }
    }
    if let Some(ref cn) = pred.card_number_is {
        if data.card_id != *cn {
            return false;
        }
    }
    if let Some(cap) = pred.play_cost_lte {
        if i32::from(data.play_cost) > cap {
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
    if pred.other == Some(true) && rctx.source_permanent == Some(handle) {
        return false;
    }

    let in_breeding = handle.index == crate::action::space::BREEDING_TARGET as u8;
    let perm = if in_breeding {
        match rctx.game.player(handle.player).breeding_area.as_ref() {
            Some(p) => p,
            None => return false,
        }
    } else {
        match rctx
            .game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
        {
            Some(p) => p,
            None => return false,
        }
    };
    // Delegate the shared card fields to the card-handle path using the top card.
    let top_handle = perm.top_card().handle();
    if !eval_card_fields(pred, rctx, top_handle, false) {
        return false;
    }
    if let Some(want) = pred.kind {
        let data = match rctx.game.card_data_for_handle(top_handle) {
            Some(d) => d,
            None => return false,
        };
        if !kind_matches_field(want, data.card_kind) {
            return false;
        }
    }
    if !eval_dp_constraints(pred, rctx, handle) {
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
    if !pred.zone.is_empty()
        && !pred.zone.contains(if in_breeding {
            &CompiledZone::Breeding
        } else {
            &CompiledZone::BattleArea
        })
    {
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

fn eval_dp_constraints(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
) -> bool {
    if pred.dp_eq.is_none() && pred.dp_lte.is_none() && pred.dp_gte.is_none() {
        return true;
    }
    let Some(dp) = rctx.game.effective_dp(handle) else {
        return false;
    };
    if let Some(want) = &pred.dp_eq {
        if dp != eval_dp_constraint(want, rctx, handle) {
            return false;
        }
    }
    if let Some(cap) = &pred.dp_lte {
        if dp > eval_dp_constraint(cap, rctx, handle) {
            return false;
        }
    }
    if let Some(floor) = &pred.dp_gte {
        if dp < eval_dp_constraint(floor, rctx, handle) {
            return false;
        }
    }
    true
}

fn eval_dp_constraint(
    constraint: &CompiledDpConstraint,
    rctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
) -> i32 {
    match constraint {
        CompiledDpConstraint::Literal(n) => *n,
        CompiledDpConstraint::Formula(f) => formula_eval::evaluate_read(f, rctx, handle),
    }
}

fn eval_breeding_permanent_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    player: PlayerId,
) -> bool {
    let Some(perm) = rctx.game.player(player).breeding_area.as_ref() else {
        return false;
    };
    let top_handle = perm.top_card().handle();

    let mut card_pred = pred.clone();
    card_pred.kind = None;
    if !eval_card_fields(&card_pred, rctx, top_handle, false) {
        return false;
    }

    if let Some(want) = pred.kind {
        let data = match rctx.game.card_data_for_handle(top_handle) {
            Some(d) => d,
            None => return false,
        };
        let matches_kind = match (want, data.card_kind) {
            (CompiledCardKind::Digimon, CardKind::DigiEgg) => true,
            _ => kind_matches_field(want, data.card_kind),
        };
        if !matches_kind {
            return false;
        }
    }
    let handle = PermanentHandle {
        player,
        index: crate::action::space::BREEDING_TARGET as u8,
    };
    if !eval_dp_constraints(pred, rctx, handle) {
        return false;
    }
    if let Some(want) = pred.in_breeding {
        if !want {
            return false;
        }
    }
    if !pred.zone.is_empty() && !pred.zone.contains(&CompiledZone::Breeding) {
        return false;
    }
    if let Some(want) = pred.owner {
        let matches = match want {
            CompiledPlayerRef::You => player == rctx.player,
            CompiledPlayerRef::Opponent => player == rctx.opponent_id(),
            CompiledPlayerRef::Active => player == rctx.game.turn_player(),
            CompiledPlayerRef::Any => true,
        };
        if !matches {
            return false;
        }
    }

    pred.is_suspended.is_none()
        && pred.is_unsuspended.is_none()
        && pred.stack_size_lte.is_none()
        && pred.stack_size_gte.is_none()
}

fn kind_matches(want: CompiledCardKind, got: CardKind) -> bool {
    matches!(
        (want, got),
        (CompiledCardKind::Digimon, CardKind::Digimon)
            | (CompiledCardKind::Tamer, CardKind::Tamer)
            | (CompiledCardKind::Option, CardKind::Option)
            | (CompiledCardKind::DigiEgg, CardKind::DigiEgg)
            | (CompiledCardKind::Token, CardKind::Token)
    )
}

fn kind_matches_card_search(want: CompiledCardKind, data: &crate::card_data::CardData) -> bool {
    match want {
        CompiledCardKind::Digimon => matches!(data.card_kind, CardKind::Digimon | CardKind::Dual),
        CompiledCardKind::Option => data.is_option_card_for_search(),
        _ => kind_matches(want, data.card_kind),
    }
}

fn kind_matches_field(want: CompiledCardKind, got: CardKind) -> bool {
    matches!(
        (want, got),
        (
            CompiledCardKind::Digimon,
            CardKind::Digimon | CardKind::Dual
        ) | (CompiledCardKind::Tamer, CardKind::Tamer)
            | (CompiledCardKind::Option, CardKind::Option)
            | (CompiledCardKind::DigiEgg, CardKind::DigiEgg)
            | (CompiledCardKind::Token, CardKind::Token)
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
