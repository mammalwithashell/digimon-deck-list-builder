//! Predicate evaluator. Phase 1c Task 3: leaf fields + combinators + existentials.

use digimon_dsl::compiled::{
    CompiledAggregateSelector, CompiledBindingCompare, CompiledCardKind, CompiledColor,
    CompiledCountAggregate, CompiledDpConstraint, CompiledEventCause, CompiledExistential,
    CompiledPlayerRef, CompiledPredicate, CompiledReplacementCause, CompiledZone,
};

use crate::card_source::{CardHandle, CardSource};
use crate::dsl_cards::bindings::{BindingValue, Bindings};
use crate::dsl_cards::formula_eval;
use crate::dsl_cards::modifier_map::lookup_keyword;
use crate::effect_context::EffectReadContext;
use crate::enums::{CardColor, CardKind, PlayerId};
use crate::permanent::PermanentHandle;
use crate::selection::AttackTarget;
use crate::trigger_context::AttackTargetChangeReason;

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
    // `all_turns: true` is an explicit "passes during any turn" marker
    // used in `active_when` scopes (the printed [All Turns] tag). It is
    // a no-op filter — the predicate passes regardless of whose turn
    // it is. `all_turns: false` is nonsensical (no turn would satisfy)
    // and is treated as a non-match.
    if let Some(want) = pred.all_turns {
        if !want {
            return false;
        }
    }
    if let Some(want) = pred.source_is_tamer {
        if rctx.source_is_tamer() != want {
            return false;
        }
    }
    if let Some(cap) = &pred.memory_lte {
        if (rctx.game.memory as i32) > eval_int_constraint_read(cap, rctx, None, bindings) {
            return false;
        }
    }
    if let Some(floor) = &pred.memory_gte {
        if (rctx.game.memory as i32) < eval_int_constraint_read(floor, rctx, None, bindings) {
            return false;
        }
    }
    if let Some(cap) = &pred.security_count_lte {
        if (rctx.security_count(rctx.player) as i32)
            > eval_int_constraint_read(cap, rctx, None, bindings)
        {
            return false;
        }
    }
    if let Some(floor) = &pred.security_count_gte {
        if (rctx.security_count(rctx.player) as i32)
            < eval_int_constraint_read(floor, rctx, None, bindings)
        {
            return false;
        }
    }
    if let Some(name) = &pred.binding_exists {
        if bindings.and_then(|b| b.get_ref(name)).is_none() {
            return false;
        }
    }
    if let Some(name) = &pred.binding_present {
        if bindings.and_then(|b| b.get_ref(name)).is_none() {
            return false;
        }
    }
    if let Some(name) = &pred.binding_absent {
        if bindings.and_then(|b| b.get_ref(name)).is_some() {
            return false;
        }
    }
    if !eval_result_bound_fields(pred, rctx, bindings) {
        return false;
    }
    if let Some(player_ref) = pred.can_hatch {
        let can_any = resolve_predicate_players(player_ref, rctx)
            .into_iter()
            .any(|player| {
                let p = rctx.game.player(player);
                p.breeding_area.is_none() && !p.digitama_deck.is_empty()
            });
        if !can_any {
            return false;
        }
    }
    if let Some(want) = pred.dna_origin {
        if rctx.dna_origin() != want {
            return false;
        }
    }
    // G-BEFORE-PAY-COST-DIGIVOLVE-TARGET: when a `cost_target` sub-
    // predicate is present, evaluate it as a Card predicate against the
    // card whose cost is currently being inspected. Fails outside any
    // BeforePayCost cost-calc dispatch (`cost_target_card == None`).
    if let Some(inner) = &pred.cost_target {
        let Some(target) = rctx.cost_target_card else {
            return false;
        };
        if !eval_predicate_with_bindings(
            inner,
            rctx,
            PredicateSubject::Card(target),
            bindings,
        ) {
            return false;
        }
    }
    // G-BEFORE-PAY-COST-DIGIVOLVE-TARGET: gate to "THIS permanent is
    // the digivolve target" — fires when the effect's source_permanent
    // is one of the digivolve target permanents (single entry for
    // normal digivolve, both materials for DNA).
    if let Some(want) = pred.source_is_cost_target_permanent {
        if rctx.source_is_cost_target_permanent() != want {
            return false;
        }
    }
    if !eval_event_fields(pred, rctx, subject) {
        return false;
    }
    if !eval_replacement_fields(pred, rctx) {
        return false;
    }
    if let Some(want) = pred.source_is_unsuspended {
        let Some(source) = rctx.source_permanent else {
            return false;
        };
        let Some(perm) = permanent_for_handle(rctx, source) else {
            return false;
        };
        if (!perm.is_suspended) != want {
            return false;
        }
    }
    if let Some(ref needle) = pred.self_digivolution_contains_name {
        let Some(perm) = subject_or_source_permanent(subject, rctx) else {
            return false;
        };
        if !perm.contains_card_name(needle, rctx.card_data()) {
            return false;
        }
    }
    if let Some(ref needle) = pred.source_name_contains {
        let Some(perm) = subject_or_source_permanent(subject, rctx) else {
            return false;
        };
        if !perm
            .top_card()
            .card_name(rctx.card_data())
            .to_lowercase()
            .contains(&needle.to_lowercase())
        {
            return false;
        }
    }
    if let Some(ref trait_name) = pred.source_permanent_trait_has {
        let Some(perm) = subject_or_source_permanent(subject, rctx) else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(perm.top_card().handle()) else {
            return false;
        };
        if !data.traits.iter().any(|x| x.eq_ignore_ascii_case(trait_name)) {
            return false;
        }
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
    // `of_permanent: <name>` constrains the subject to the named
    // permanent binding. The subject must be a `Permanent` and equal
    // to whatever `<name>` is bound to. Used by source-stack selection
    // step bodies (e.g. "select an inherited card on `carrier`").
    if let Some(binding_name) = &pred.of_permanent {
        let PredicateSubject::Permanent(handle) = subject else {
            return false;
        };
        let Some(target) = bindings.and_then(|b| b.get_permanent(binding_name)) else {
            return false;
        };
        if target != handle {
            return false;
        }
    }
    if let Some(binding_owner) = &pred.binding_owner {
        let Some(owner) = bindings
            .and_then(|b| b.get_permanent(&binding_owner.binding))
            .map(|handle| handle.player)
        else {
            return false;
        };
        if !player_ref_matches(binding_owner.of, owner, rctx) {
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
    if let Some(agg) = &pred.count_lte {
        let cap = eval_int_constraint_read(&agg.n, rctx, None, bindings).max(0) as u32;
        if count_matching(agg, rctx, bindings) > cap {
            return false;
        }
    }
    if let Some(agg) = &pred.count_gte {
        let floor = eval_int_constraint_read(&agg.n, rctx, None, bindings).max(0) as u32;
        if count_matching(agg, rctx, bindings) < floor {
            return false;
        }
    }

    match subject {
        PredicateSubject::Card(card) => eval_card_fields(pred, rctx, card, false, None, bindings),
        PredicateSubject::RevealedCard(card) => {
            eval_card_fields(pred, rctx, card, true, None, bindings)
        }
        PredicateSubject::Permanent(h) => eval_permanent_fields(pred, rctx, h, bindings),
        PredicateSubject::BreedingPermanent(player) => {
            eval_breeding_permanent_fields(pred, rctx, player, bindings)
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

fn eval_result_bound_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> bool {
    let Some(bindings) = bindings else {
        return pred.effect_suspended_any_own_digimon != Some(true)
            && pred.effect_returned_any_card != Some(true)
            && pred.effect_deleted_any_own_digimon != Some(true)
            && pred.effect_deleted_any_opponent_digimon != Some(true)
            && pred.effect_played_any_digimon != Some(true)
            && pred.effect_digivolved_any_digimon != Some(true)
            && pred.effect_added_any_card_to_hand != Some(true);
    };
    let log = bindings.result_log();
    if let Some(want) = pred.effect_suspended_any_own_digimon {
        let actual = log.suspended.iter().any(|h| h.player == rctx.player);
        if actual != want {
            return false;
        }
    }
    if let Some(want) = pred.effect_returned_any_card {
        let actual = !log.returned_to_deck.is_empty();
        if actual != want {
            return false;
        }
    }
    if let Some(want) = pred.effect_deleted_any_own_digimon {
        let actual = log.deleted.iter().any(|h| h.player == rctx.player);
        if actual != want {
            return false;
        }
    }
    if let Some(want) = pred.effect_deleted_any_opponent_digimon {
        let actual = log.deleted.iter().any(|h| h.player != rctx.player);
        if actual != want {
            return false;
        }
    }
    if let Some(want) = pred.effect_played_any_digimon {
        let actual = !log.played.is_empty();
        if actual != want {
            return false;
        }
    }
    if let Some(want) = pred.effect_digivolved_any_digimon {
        let actual = !log.digivolved.is_empty();
        if actual != want {
            return false;
        }
    }
    if let Some(want) = pred.effect_added_any_card_to_hand {
        let actual = !log.added_to_hand.is_empty();
        if actual != want {
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
        ) | (
            CompiledReplacementCause::Overclock,
            crate::replacement::ReplacementCause::Overclock
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
    resolve_predicate_players(of, rctx)
}

fn resolve_predicate_players(of: CompiledPlayerRef, rctx: &EffectReadContext<'_>) -> Vec<PlayerId> {
    match of {
        CompiledPlayerRef::You => vec![rctx.player],
        CompiledPlayerRef::Opponent => vec![rctx.opponent_id()],
        CompiledPlayerRef::Active => vec![rctx.game.turn_player()],
        CompiledPlayerRef::Any => (0..rctx.game.players.len() as PlayerId).collect(),
    }
}

fn count_matching(
    aggregate: &CompiledCountAggregate,
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> u32 {
    let filter = aggregate.filter.as_ref();
    let owners = existential_players(filter.owner.unwrap_or(CompiledPlayerRef::You), rctx);
    let zones: Vec<CompiledZone> = if filter.zone.is_empty() {
        vec![CompiledZone::BattleArea]
    } else {
        filter.zone.clone()
    };

    let mut subject_filter = filter.clone();
    subject_filter.zone.clear();
    subject_filter.owner = None;

    let mut count = 0;
    for owner in owners {
        let player = rctx.game.player(owner);
        for zone in &zones {
            match zone {
                CompiledZone::BattleArea => {
                    for index in 0..player.battle_area.len() {
                        let handle = PermanentHandle {
                            player: owner,
                            index: index as u8,
                        };
                        if eval_predicate_with_bindings(
                            &subject_filter,
                            rctx,
                            PredicateSubject::Permanent(handle),
                            bindings,
                        ) {
                            count += 1;
                        }
                    }
                }
                CompiledZone::Breeding => {
                    if player.breeding_area.is_some()
                        && eval_predicate_with_bindings(
                            &subject_filter,
                            rctx,
                            PredicateSubject::BreedingPermanent(owner),
                            bindings,
                        )
                    {
                        count += 1;
                    }
                }
                CompiledZone::Hand => {
                    count += count_card_sources(&player.hand, &subject_filter, rctx, bindings);
                }
                CompiledZone::Deck => {
                    count += count_card_sources(&player.deck, &subject_filter, rctx, bindings);
                }
                CompiledZone::Trash => {
                    count += count_card_sources(&player.trash, &subject_filter, rctx, bindings);
                }
                CompiledZone::Security => {
                    count += count_card_sources(&player.security, &subject_filter, rctx, bindings);
                }
                CompiledZone::DigiEggDeck => {
                    count +=
                        count_card_sources(&player.digitama_deck, &subject_filter, rctx, bindings);
                }
                CompiledZone::Reveal => {
                    count += rctx
                        .game
                        .revealed_cards
                        .iter()
                        .filter(|card| {
                            card.owner == owner
                                && eval_predicate_with_bindings(
                                    &subject_filter,
                                    rctx,
                                    PredicateSubject::RevealedCard(card.handle()),
                                    bindings,
                                )
                        })
                        .count() as u32;
                }
                CompiledZone::Material => {
                    if let Some(source) = rctx.source_permanent {
                        count += permanent_material_count(source, &subject_filter, rctx, bindings);
                    }
                }
            }
        }
    }
    count
}

fn count_card_sources(
    cards: &[CardSource],
    filter: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> u32 {
    cards
        .iter()
        .filter(|card| {
            eval_predicate_with_bindings(
                filter,
                rctx,
                PredicateSubject::Card(card.handle()),
                bindings,
            )
        })
        .count() as u32
}

fn permanent_material_count(
    handle: PermanentHandle,
    filter: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> u32 {
    let Some(perm) = permanent_for_handle(rctx, handle) else {
        return 0;
    };
    perm.card_sources
        .iter()
        .take(perm.card_sources.len().saturating_sub(1))
        .filter(|card| {
            eval_predicate_with_bindings(
                filter,
                rctx,
                PredicateSubject::Card(card.handle()),
                bindings,
            )
        })
        .count() as u32
}

fn eval_no_subject_fields(pred: &CompiledPredicate) -> bool {
    // If any subject-only field is set, this subjectless eval can't satisfy it.
    pred.kind.is_none()
        && pred.level_eq.is_none()
        && pred.level_eq_binding.is_none()
        && pred.level_lte.is_none()
        && pred.level_gte.is_none()
        && pred.level_matches_aggregate.is_none()
        && pred.color_is.is_none()
        && pred.color_only.is_none()
        && pred.color_matches_any_field_digimon.is_none()
        && pred.color_matches_binding.is_none()
        && pred.trait_has.is_none()
        && pred.form_is.is_none()
        && pred.attribute_is.is_none()
        && pred.name_is.is_none()
        && pred.name_contains.is_none()
        && pred.name_in.is_none()
        && pred.card_number_is.is_none()
        && pred.play_cost_lte.is_none()
        && pred.play_cost_gte.is_none()
        && pred.self_color_count_gte.is_none()
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

fn eval_event_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    subject: PredicateSubject,
) -> bool {
    if let Some(ref want) = pred.attack_target_change_reason {
        let Some(change) = rctx.attack_target_change() else {
            return false;
        };
        if !attack_target_change_reason_matches(want, change.reason) {
            return false;
        }
    }
    if let Some(want) = pred.event_target_is_player {
        let is_player = if let Some(change) = rctx.attack_target_change() {
            matches!(change.new_target, AttackTarget::Player(_))
        } else if let Some(target) = rctx.attack_target() {
            matches!(target, AttackTarget::Player(_))
        } else {
            return false;
        };
        if is_player != want {
            return false;
        }
    }
    if let Some(want) = pred.event_target_was_self {
        let Some(change) = rctx.attack_target_change() else {
            return false;
        };
        let source = match subject {
            PredicateSubject::Permanent(handle) => Some(handle),
            PredicateSubject::BreedingPermanent(player) => Some(PermanentHandle {
                player,
                index: crate::action::space::BREEDING_TARGET as u8,
            }),
            PredicateSubject::Card(_)
            | PredicateSubject::RevealedCard(_)
            | PredicateSubject::None => rctx
                .game
                .current_trigger_context
                .as_ref()
                .and_then(|trigger| trigger.target_permanent)
                .or(rctx.source_permanent),
        };
        let was_self = matches!(
            (change.old_target, source),
            (AttackTarget::Digimon(old_target), Some(source)) if old_target == source
        );
        if was_self != want {
            return false;
        }
    }
    if let Some(ref trait_name) = pred.attacker_trait_has {
        let Some(change) = rctx.attack_target_change() else {
            return false;
        };
        let Some(card) = rctx
            .game
            .player(change.attacker.player)
            .battle_area
            .get(change.attacker.index as usize)
            .map(|perm| perm.top_card().handle())
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
    if let Some(want) = pred.event_target_kind {
        if let Some(snapshot) = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.deleted_object.as_ref())
        {
            if !kind_matches_field(want, snapshot.card_kind) {
                return false;
            }
        } else {
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
    }
    if let Some(ref trait_name) = pred.event_target_trait_has {
        if let Some(snapshot) = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.deleted_object.as_ref())
        {
            if !snapshot
                .traits
                .iter()
                .any(|t| t.eq_ignore_ascii_case(trait_name))
            {
                return false;
            }
        } else {
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
    }
    if let Some(want) = pred.event_target_owner {
        let Some(owner) = event_target_owner(rctx) else {
            return false;
        };
        if !player_ref_matches(want, owner, rctx) {
            return false;
        }
    }
    if let Some(want) = pred.event_permanent_is_source {
        let Some(trigger) = rctx.game.current_trigger_context.as_ref() else {
            return false;
        };
        let Some(event_permanent) = trigger.event_permanent else {
            return false;
        };
        let Some(source_permanent) = rctx.source_permanent else {
            return false;
        };
        if (event_permanent == source_permanent) != want {
            return false;
        }
    }
    if let Some(want) = pred.event_is_effect_initiated {
        let Some(trigger) = rctx.game.current_trigger_context.as_ref() else {
            return false;
        };
        if trigger.effect_initiated != want {
            return false;
        }
    }
    if let Some(want) = pred.dna_origin {
        let actual = rctx
            .game
            .current_trigger_context
            .as_ref()
            .map(|trigger| trigger.dna_origin)
            .unwrap_or(false)
            || rctx.game.current_dna_origin.unwrap_or(false);
        if actual != want {
            return false;
        }
    }
    if let Some(ref trait_name) = pred.event_card_trait_has {
        let Some(card) = rctx
            .game
            .current_trigger_context
            .as_ref()
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
    if let Some(want) = pred.event_cause {
        let Some(actual) = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.cause)
        else {
            return false;
        };
        if !event_cause_matches(want, actual) {
            return false;
        }
    }
    if let Some(ref trait_name) = pred.host_permanent_trait_has {
        let live_host_matches = rctx
            .event_host_permanent()
            .and_then(|host| permanent_for_handle(rctx, host))
            .map(|perm| perm.has_trait(trait_name, rctx.card_data()))
            .unwrap_or(false);
        let host_snapshot_matches = rctx
            .event_host_card()
            .and_then(|card| rctx.game.card_data_for_handle(card))
            .map(|data| {
                data.traits
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(trait_name))
            })
            .unwrap_or(false);
        if !live_host_matches && !host_snapshot_matches {
            return false;
        }
    }
    if let Some(ref trait_name) = pred.trashed_source_trait_has {
        let Some(card) = rctx.event_source_card() else {
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
    if let Some(ref card_id) = pred.trashed_source_card_id_is {
        let Some(card) = rctx.event_source_card() else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(card) else {
            return false;
        };
        if data.card_id != *card_id {
            return false;
        }
    }
    true
}

fn event_cause_matches(
    want: CompiledEventCause,
    actual: crate::trigger_context::EventCause,
) -> bool {
    use crate::trigger_context::EventCause as A;
    matches!(
        (want, actual),
        (CompiledEventCause::BattleDeletion, A::BattleDeletion)
            | (CompiledEventCause::EffectDeletion, A::EffectDeletion)
            | (CompiledEventCause::OwnEffect, A::OwnEffect)
            | (CompiledEventCause::OpponentEffect, A::OpponentEffect)
            | (CompiledEventCause::Overclock, A::Overclock)
            | (CompiledEventCause::Return, A::Return)
            | (CompiledEventCause::DeckBottom, A::DeckBottom)
            | (CompiledEventCause::SecurityPlacement, A::SecurityPlacement)
            | (CompiledEventCause::SecurityRemoval, A::SecurityRemoval)
            | (CompiledEventCause::Cost, A::Cost)
            | (CompiledEventCause::Rule, A::Rule)
    )
}

fn attack_target_change_reason_matches(want: &str, actual: AttackTargetChangeReason) -> bool {
    let normalized = want
        .chars()
        .filter(|c| *c != '_' && *c != '-' && !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect::<String>();
    match actual {
        AttackTargetChangeReason::Raid => normalized == "raid",
        AttackTargetChangeReason::Collision => normalized == "collision",
        AttackTargetChangeReason::Blocker => normalized == "blocker" || normalized == "block",
        AttackTargetChangeReason::EffectRedirect(_) => {
            normalized == "effectredirect" || normalized == "redirect"
        }
        AttackTargetChangeReason::EffectForced => {
            normalized == "effectforced" || normalized == "forced"
        }
    }
}

fn subject_or_source_permanent<'a>(
    subject: PredicateSubject,
    rctx: &'a EffectReadContext<'_>,
) -> Option<&'a crate::permanent::Permanent> {
    match subject {
        PredicateSubject::Permanent(handle) => permanent_for_handle(rctx, handle),
        PredicateSubject::BreedingPermanent(player) => {
            rctx.game.player(player).breeding_area.as_ref()
        }
        PredicateSubject::Card(_) | PredicateSubject::RevealedCard(_) | PredicateSubject::None => {
            rctx.source_permanent()
        }
    }
}

fn permanent_for_handle<'a>(
    rctx: &'a EffectReadContext<'_>,
    handle: PermanentHandle,
) -> Option<&'a crate::permanent::Permanent> {
    if handle.index == crate::action::space::BREEDING_TARGET as u8 {
        return rctx.game.player(handle.player).breeding_area.as_ref();
    }
    rctx.game
        .player(handle.player)
        .battle_area
        .get(handle.index as usize)
}

fn event_target_owner(rctx: &EffectReadContext<'_>) -> Option<PlayerId> {
    let trigger = rctx.game.current_trigger_context.as_ref()?;
    if let Some(snapshot) = trigger.deleted_object.as_ref() {
        return Some(snapshot.former_controller);
    }
    if let Some(change) = trigger.attack_target_change.as_ref() {
        return match change.new_target {
            AttackTarget::Digimon(handle) => Some(handle.player),
            AttackTarget::Player(player) => Some(player),
        };
    }
    if let Some(handle) = trigger.event_permanent {
        return Some(handle.player);
    }
    if let Some(handle) = trigger.event_host_permanent {
        return Some(handle.player);
    }
    if let Some(handle) = trigger.target_permanent {
        return Some(handle.player);
    }
    for card in [trigger.event_card, trigger.target_card]
        .into_iter()
        .flatten()
    {
        if let Some(source) = rctx.game.card_source_for_handle(card) {
            return Some(source.owner);
        }
    }
    trigger.source_player
}

fn player_ref_matches(
    want: CompiledPlayerRef,
    actual: PlayerId,
    rctx: &EffectReadContext<'_>,
) -> bool {
    match want {
        CompiledPlayerRef::You => actual == rctx.player(),
        CompiledPlayerRef::Opponent => actual == rctx.opponent_id(),
        CompiledPlayerRef::Active => actual == rctx.game.turn_player(),
        CompiledPlayerRef::Any => true,
    }
}

fn event_target_card(rctx: &EffectReadContext<'_>) -> Option<CardHandle> {
    let trigger = rctx.game.current_trigger_context.as_ref()?;
    if let Some(snapshot) = trigger.deleted_object.as_ref() {
        return Some(snapshot.top_card);
    }
    if let Some(change) = trigger.attack_target_change.as_ref() {
        if let AttackTarget::Digimon(handle) = change.new_target {
            return rctx
                .game
                .player(handle.player)
                .battle_area
                .get(handle.index as usize)
                .map(|perm| perm.top_card().handle());
        }
        return None;
    }
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
    formula_target: Option<PermanentHandle>,
    bindings: Option<&Bindings>,
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
    if let Some(ref binding) = pred.level_eq_binding {
        let Some(want) = bindings.and_then(|b| b.get_literal(binding)) else {
            return false;
        };
        if data.level.map(i64::from) != Some(want) {
            return false;
        }
    }
    if let Some(cap) = &pred.level_lte {
        let cap = eval_int_constraint(cap, rctx, formula_target, bindings);
        if data.level.map_or(true, |l| i32::from(l) > cap) {
            return false;
        }
    }
    if let Some(floor) = &pred.level_gte {
        let floor = eval_int_constraint(floor, rctx, formula_target, bindings);
        if data.level.map_or(true, |l| i32::from(l) < floor) {
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
    if let Some(of) = pred.color_matches_any_field_digimon {
        if !card_shares_color_with_any_field_digimon(rctx, of, &data.colors) {
            return false;
        }
    }
    if let Some(ref binding) = pred.color_matches_binding {
        if !card_shares_color_with_bound_permanent(rctx, bindings, binding, data) {
            return false;
        }
    }
    if let Some(ref t) = pred.trait_has {
        if !data.traits.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            return false;
        }
    }
    if let Some(floor) = pred.self_color_count_gte {
        if distinct_color_count(&data.colors) < usize::from(floor) {
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
    if let Some(cap) = &pred.play_cost_lte {
        if i32::from(data.play_cost) > eval_int_constraint(cap, rctx, formula_target, bindings) {
            return false;
        }
    }
    if let Some(floor) = &pred.play_cost_gte {
        if i32::from(data.play_cost) < eval_int_constraint(floor, rctx, formula_target, bindings) {
            return false;
        }
    }
    // Card-subject DP filter. Cards in trash/hand/security carry their printed
    // DP via `CardData.dp`; permanents on the field route through
    // `eval_dp_constraints` against `effective_dp`. Options and other
    // non-Digimon cards have `dp = None` and cannot satisfy a DP constraint.
    if pred.dp_eq.is_some() || pred.dp_lte.is_some() || pred.dp_gte.is_some() {
        let Some(dp) = data.dp else {
            return false;
        };
        let perm_target = formula_target.or(rctx.source_permanent).unwrap_or(
            crate::permanent::PermanentHandle {
                player: rctx.player,
                index: 0,
            },
        );
        if let Some(want) = &pred.dp_eq {
            if dp != eval_dp_constraint(want, rctx, perm_target, bindings) {
                return false;
            }
        }
        if let Some(cap) = &pred.dp_lte {
            if dp > eval_dp_constraint(cap, rctx, perm_target, bindings) {
                return false;
            }
        }
        if let Some(floor) = &pred.dp_gte {
            if dp < eval_dp_constraint(floor, rctx, perm_target, bindings) {
                return false;
            }
        }
    }
    if let Some(want) = pred.can_digivolve_from_source {
        if can_card_digivolve_from_source(rctx, card) != want {
            return false;
        }
    }
    if let Some(ref alt_kind) = pred.has_alt_path {
        if !card_has_alt_path(rctx, &data.card_id, alt_kind) {
            return false;
        }
    }
    if let Some(ref inner) = pred.has_inherited {
        // For card subjects (in deck/hand/trash), `has_inherited` checks
        // whether the card's printed `inherited_text` is non-empty when
        // the inner predicate is empty/default. With a non-default
        // inner predicate, a card subject has no inherited card sources
        // to recurse into, so the predicate fails — only permanents on
        // the field have a digivolution stack to scan.
        if inner.as_ref() == &CompiledPredicate::default() {
            if data.inherited_text.is_empty() {
                return false;
            }
        } else {
            // Non-trivial inner predicate cannot match a card subject.
            return false;
        }
    }
    true
}

/// Returns true if the card with `card_id` has at least one DSL-registered
/// alt-path whose `kind` snake-cases to `kind_name`. Mirrors the YAML
/// vocabulary for `has_alt_path: <name>` (e.g. `digixros`, `digivolve`).
fn card_has_alt_path(rctx: &EffectReadContext<'_>, card_id: &str, kind_name: &str) -> bool {
    let registry = &rctx.game.alt_path_registry;
    let Some(paths) = registry.get(card_id) else {
        return false;
    };
    paths
        .iter()
        .any(|p| alt_path_kind_matches(&p.kind, kind_name))
}

fn alt_path_kind_matches(
    kind: &digimon_dsl::compiled::CompiledAltPathKind,
    name: &str,
) -> bool {
    use digimon_dsl::compiled::CompiledAltPathKind as K;
    let normalized = name.to_ascii_lowercase().replace('-', "_");
    let kind_str = match kind {
        K::Digivolve => "digivolve",
        K::DnaDigivolve => "dna_digivolve",
        K::BlastDnaDigivolve => "blast_dna_digivolve",
        K::DigiXros => "digixros",
        K::BurstDigivolve => "burst_digivolve",
        K::AppFusion => "app_fusion",
        K::Assembly => "assembly",
        K::ActivatedDigivolve => "activated_digivolve",
    };
    kind_str == normalized
}

fn can_card_digivolve_from_source(rctx: &EffectReadContext<'_>, card: CardHandle) -> bool {
    let Some(source_handle) = rctx.source_permanent else {
        return false;
    };
    let Some(source_permanent) = permanent_for_handle(rctx, source_handle) else {
        return false;
    };
    let Some(candidate) = rctx.game.card_source_for_handle(card) else {
        return false;
    };
    rctx.game.can_digivolve(candidate, source_permanent)
}

/// Phase 2 Track F (G-DSL-HAS-ON-DELETION-EFFECT) — true if `perm`'s top
/// card or any digivolution source carries any `OnDeletion`-timed effect.
/// Consulted by `has_on_deletion_effect: <bool>` predicate.
///
/// Walks both the registry-bound hand-written `CardEffect` impl and the
/// compiled DSL clauses (returned together by `Game::effects_for_card`)
/// and returns true on the first OnDeletion hit. Per printed text the
/// gate is on the existence of the printed timing, not on whether the
/// effect is currently runtime-active — `effects_for_card` already
/// expands keyword-derived auto-effects (Save, MaterialSave, Decoy,
/// etc.) so cards whose On Deletion text is keyword-shaped (e.g.
/// `<Save>`) also surface here.
fn permanent_has_on_deletion_effect(
    perm: &crate::permanent::Permanent,
    rctx: &EffectReadContext<'_>,
) -> bool {
    use crate::enums::EffectTiming;
    let data = rctx.card_data();
    for source in &perm.card_sources {
        let card_id = source.card_id(data);
        let Some(effects) = rctx.game.effects_for_card(card_id, source.handle()) else {
            continue;
        };
        if effects
            .iter()
            .any(|e| matches!(e.timing, EffectTiming::OnDeletion) || e.on_deletion)
        {
            return true;
        }
    }
    false
}

fn distinct_color_count(colors: &[CardColor]) -> usize {
    let mut seen = Vec::new();
    for color in colors {
        if !seen.contains(color) {
            seen.push(*color);
        }
    }
    seen.len()
}

fn card_shares_color_with_any_field_digimon(
    rctx: &EffectReadContext<'_>,
    of: CompiledPlayerRef,
    colors: &[CardColor],
) -> bool {
    if colors.is_empty() {
        return false;
    }
    for player in existential_players(of, rctx) {
        for permanent in &rctx.game.player(player).battle_area {
            let Some(data) = rctx
                .game
                .card_data_for_handle(permanent.top_card().handle())
            else {
                continue;
            };
            if !kind_matches_field(CompiledCardKind::Digimon, data.card_kind) {
                continue;
            }
            if data
                .digimon_colors()
                .iter()
                .any(|field_color| colors.iter().any(|card_color| card_color == field_color))
            {
                return true;
            }
        }
    }
    false
}

fn card_shares_color_with_bound_permanent(
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
    binding: &str,
    data: &crate::card_data::CardData,
) -> bool {
    let Some(bound) = bindings.and_then(|b| b.get_permanent(binding)) else {
        return false;
    };
    let Some(permanent) = permanent_for_handle(rctx, bound) else {
        return false;
    };
    let bound_colors = permanent.top_card().colors(rctx.card_data());
    if bound_colors.is_empty() {
        return false;
    }

    let candidate_colors = if kind_matches_card_search(CompiledCardKind::Digimon, data) {
        data.digimon_colors()
    } else if kind_matches_card_search(CompiledCardKind::Option, data) {
        data.option_colors()
    } else {
        data.colors.as_slice()
    };
    candidate_colors
        .iter()
        .any(|candidate| bound_colors.iter().any(|bound| candidate == bound))
}

fn eval_permanent_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
    bindings: Option<&Bindings>,
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
    let synth_identity = perm.synth_identity(rctx.card_data(), &rctx.game.modifiers, handle);
    // Track H × Track C — propagate synth-identity overlays into the
    // shared card-field checks (`trait_has`, `name_is`, `name_contains`,
    // `name_in`, `color_is`, `color_only`). `eval_card_fields` only
    // sees printed `CardData`; without this, Track C overlays
    // (`ChangeTraits`, `ChangeBaseCardName`, `ChangeBaseCardColor`)
    // were invisible to Track H aura filters. We pre-check each
    // overlay-able field against `synth_identity` and clear it from
    // the delegated predicate if the overlay matches. Pinned by
    // `aura_filter_includes_track_c_change_traits_overlay` (and follow-
    // up tests for name/color overlay propagation).
    let trait_overlay_match = pred.trait_has.as_ref().is_some_and(|t| {
        synth_identity
            .traits
            .iter()
            .any(|x| x.eq_ignore_ascii_case(t))
    });
    let name_is_overlay_match = pred
        .name_is
        .as_ref()
        .is_some_and(|n| synth_identity.card_name == *n);
    let name_contains_overlay_match = pred.name_contains.as_ref().is_some_and(|n| {
        synth_identity
            .card_name
            .to_lowercase()
            .contains(&n.to_lowercase())
    });
    let name_in_overlay_match = pred
        .name_in
        .as_ref()
        .is_some_and(|names| names.iter().any(|n| synth_identity.card_name == *n));
    let color_is_overlay_match = pred.color_is.is_some_and(|want| {
        synth_identity
            .colors
            .iter()
            .any(|c| color_matches(want, *c))
    });
    let color_only_overlay_match = pred.color_only.as_ref().is_some_and(|allowed| {
        !synth_identity.colors.is_empty()
            && synth_identity
                .colors
                .iter()
                .all(|c| allowed.iter().any(|a| color_matches(*a, *c)))
    });
    let delegated_pred_storage;
    let delegated_pred = if trait_overlay_match
        || name_is_overlay_match
        || name_contains_overlay_match
        || name_in_overlay_match
        || color_is_overlay_match
        || color_only_overlay_match
    {
        let mut p = pred.clone();
        if trait_overlay_match {
            p.trait_has = None;
        }
        if name_is_overlay_match {
            p.name_is = None;
        }
        if name_contains_overlay_match {
            p.name_contains = None;
        }
        if name_in_overlay_match {
            p.name_in = None;
        }
        if color_is_overlay_match {
            p.color_is = None;
        }
        if color_only_overlay_match {
            p.color_only = None;
        }
        delegated_pred_storage = p;
        &delegated_pred_storage
    } else {
        pred
    };
    if !eval_card_fields(
        delegated_pred,
        rctx,
        top_handle,
        false,
        Some(handle),
        bindings,
    ) {
        return false;
    }
    if let Some(want) = pred.kind {
        if !kind_matches_field(want, synth_identity.kind) {
            return false;
        }
    }
    if !eval_level_aggregate_match(pred, rctx, synth_identity.level) {
        return false;
    }
    if !eval_dp_constraints(pred, rctx, handle, bindings) {
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
    if let Some(ref keyword) = pred.has_keyword {
        let Some(kw) = lookup_keyword(keyword, None) else {
            return false;
        };
        if !rctx.game.has_keyword(handle, kw) {
            return false;
        }
    }
    if let Some(want) = pred.has_on_deletion_effect {
        let observed = permanent_has_on_deletion_effect(perm, rctx);
        if observed != want {
            return false;
        }
    }
    if let Some(floor) = pred.self_color_count_gte {
        if distinct_color_count(&synth_identity.colors) < usize::from(floor) {
            return false;
        }
    }
    if let Some(cap) = &pred.stack_size_lte {
        if perm.card_sources.len() as i32 > eval_int_constraint(cap, rctx, Some(handle), bindings) {
            return false;
        }
    }
    if let Some(floor) = &pred.stack_size_gte {
        if (perm.card_sources.len() as i32)
            < eval_int_constraint(floor, rctx, Some(handle), bindings)
        {
            return false;
        }
    }
    let materials_count = perm.card_sources.len().saturating_sub(1) as i32;
    if let Some(cap) = &pred.materials_count_lte {
        if materials_count > eval_int_constraint(cap, rctx, Some(handle), bindings) {
            return false;
        }
    }
    if let Some(floor) = &pred.materials_count_gte {
        if materials_count < eval_int_constraint(floor, rctx, Some(handle), bindings) {
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
    bindings: Option<&Bindings>,
) -> bool {
    if pred.dp_eq.is_none() && pred.dp_lte.is_none() && pred.dp_gte.is_none() {
        return true;
    }
    let Some(dp) = rctx.game.effective_dp(handle) else {
        return false;
    };
    if let Some(want) = &pred.dp_eq {
        if dp != eval_dp_constraint(want, rctx, handle, bindings) {
            return false;
        }
    }
    if let Some(cap) = &pred.dp_lte {
        if dp > eval_dp_constraint(cap, rctx, handle, bindings) {
            return false;
        }
    }
    if let Some(floor) = &pred.dp_gte {
        if dp < eval_dp_constraint(floor, rctx, handle, bindings) {
            return false;
        }
    }
    true
}

fn eval_level_aggregate_match(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    level: Option<u8>,
) -> bool {
    let Some((selector, of)) = pred.level_matches_aggregate else {
        return true;
    };
    let Some(level) = level else {
        return false;
    };
    let Some(aggregate_level) = aggregate_level(selector, of, rctx) else {
        return false;
    };
    i32::from(level) == aggregate_level
}

fn aggregate_level(
    selector: CompiledAggregateSelector,
    of: CompiledPlayerRef,
    rctx: &EffectReadContext<'_>,
) -> Option<i32> {
    let levels = existential_players(of, rctx)
        .into_iter()
        .flat_map(|player| {
            rctx.game
                .player(player)
                .battle_area
                .iter()
                .filter_map(|perm| perm.level(rctx.card_data()).map(i32::from))
        });
    match selector {
        CompiledAggregateSelector::LowestLevel => levels.min(),
        CompiledAggregateSelector::HighestLevel => levels.max(),
        CompiledAggregateSelector::LowestDp | CompiledAggregateSelector::HighestDp => None,
    }
}

fn eval_dp_constraint(
    constraint: &CompiledDpConstraint,
    rctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
    bindings: Option<&Bindings>,
) -> i32 {
    match constraint {
        CompiledDpConstraint::Literal(n) => *n,
        CompiledDpConstraint::Formula(f) => {
            formula_eval::evaluate_read_with_bindings(f, rctx, handle, bindings)
        }
    }
}

fn eval_int_constraint(
    constraint: &CompiledDpConstraint,
    rctx: &EffectReadContext<'_>,
    formula_target: Option<PermanentHandle>,
    bindings: Option<&Bindings>,
) -> i32 {
    match constraint {
        CompiledDpConstraint::Literal(n) => *n,
        CompiledDpConstraint::Formula(f) => {
            let target = formula_target
                .or(rctx.source_permanent)
                .unwrap_or(PermanentHandle {
                    player: rctx.player,
                    index: 0,
                });
            formula_eval::evaluate_read_with_bindings(f, rctx, target, bindings)
        }
    }
}

fn eval_int_constraint_read(
    constraint: &CompiledDpConstraint,
    rctx: &EffectReadContext<'_>,
    formula_target: Option<PermanentHandle>,
    bindings: Option<&Bindings>,
) -> i32 {
    eval_int_constraint(constraint, rctx, formula_target, bindings)
}

fn eval_breeding_permanent_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    player: PlayerId,
    bindings: Option<&Bindings>,
) -> bool {
    let Some(perm) = rctx.game.player(player).breeding_area.as_ref() else {
        return false;
    };
    let top_handle = perm.top_card().handle();

    let mut card_pred = pred.clone();
    card_pred.kind = None;
    if !eval_card_fields(
        &card_pred,
        rctx,
        top_handle,
        false,
        Some(PermanentHandle {
            player,
            index: crate::action::space::BREEDING_TARGET as u8,
        }),
        bindings,
    ) {
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
    if !eval_level_aggregate_match(pred, rctx, perm.level(rctx.card_data())) {
        return false;
    }
    let handle = PermanentHandle {
        player,
        index: crate::action::space::BREEDING_TARGET as u8,
    };
    if !eval_dp_constraints(pred, rctx, handle, bindings) {
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
        && pred.materials_count_lte.is_none()
        && pred.materials_count_gte.is_none()
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
