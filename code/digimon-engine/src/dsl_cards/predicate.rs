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
use crate::dsl_cards::source_stats::same_level_pairs_in_sources;
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
    /// A digivolution-stack source carrying source-stack metadata (face-down
    /// flag, host permanent, stack position). Source-subject leaves
    /// (e.g. `is_face_down`) are evaluated against this, after which the
    /// subject degrades to `Card` so card-identity leaves keep working.
    Source(crate::selection::SourceSelectionRef),
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
    // §4-6-1: a DUAL card can be treated as either a Digimon card or an Option
    // card. A reference that names neither category (no `kind:` Digimon /
    // Option anywhere in its conjunction) reads the DUAL card's Digimon
    // information only, as a `kind: digimon` reference does (§4-6-5-2; owner
    // ruling W5 (b), 2026-10-17: "This would be the Digimon colour/s"). The
    // Option information is read only under a `kind: option` qualifier
    // (§4-6-6-2), which `conjunct_dual_ref` resolves node by node -- also for
    // a DUAL card lying in a permanent's digivolution or link cards (official
    // Q&A Q6395). A DUAL card that is a permanent's TOP card is a Digimon
    // (§4-6-3); `eval_card_fields` enforces that from the card's location
    // whatever reference reaches it.
    eval_predicate_ref(pred, rctx, subject, bindings, DualRef::Digimon)
}

/// Which information of a DUAL card a card-zone search reads (§4-6-5-2,
/// §4-6-6-2). `kind: digimon` references the card as a Digimon card (Digimon
/// information only), `kind: option` as an Option card (Option information
/// only). `Either` is what `conjunct_dual_ref` returns for a node that names
/// neither category; such a node inherits its enclosing node's reference, and
/// the root reference (`eval_predicate_with_bindings`) is `Digimon`, so an
/// unqualified reference reads the Digimon information (§4-6-1, §4-6-5-2;
/// owner ruling W5 (b)) and a leaf only ever sees `Digimon` or `Option`. A
/// card that is not a DUAL has one set of information, read whatever the
/// reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DualRef {
    Either,
    Digimon,
    Option,
}

/// The category a predicate subtree references through its own `kind` or an
/// `all_of` conjunct (a `kind: option` sibling qualifies the colour / level /
/// cost leaves beside it). Never looks through `any_of` / `not` / `none_of`.
fn conjunct_dual_ref(pred: &CompiledPredicate) -> DualRef {
    match pred.kind {
        Some(CompiledCardKind::Option) => return DualRef::Option,
        Some(CompiledCardKind::Digimon) => return DualRef::Digimon,
        _ => {}
    }
    pred.all_of
        .iter()
        .map(conjunct_dual_ref)
        .find(|r| *r != DualRef::Either)
        .unwrap_or(DualRef::Either)
}

fn eval_predicate_ref(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    subject: PredicateSubject,
    bindings: Option<&Bindings>,
    inherited_ref: DualRef,
) -> bool {
    // A node's own category (its `kind` or an `all_of` conjunct) wins over the
    // one inherited from an enclosing node. A `Source` subject (a digivolution
    // card) reads like any card in a card zone: a DUAL card lying in the
    // digivolution cards is referenced as an Option card by an "Option card
    // from ... digivolution cards" reference (§4-6-6-2; official Q&A Q6395).
    // Only the TOP card of a permanent is a Digimon whatever the reference
    // (§4-6-3), which `eval_card_fields` enforces from the card's location.
    let dual_ref = match conjunct_dual_ref(pred) {
        DualRef::Either => inherited_ref,
        own => own,
    };
    // Source subjects: evaluate source-stack-metadata leaves here, then
    // degrade the subject to Card so every existing card-identity leaf
    // (trait_has / kind / name / etc.) continues to work unchanged.
    // `is_face_down` / `is_bottom_source` are only satisfiable for a Source
    // subject â€” on any non-Source subject a present source-subject leaf is an
    // unconditional non-match.
    //
    // The degrade is *node-local*: `subject` below is the degraded value
    // used by THIS node's leaf checks, but `original_subject` is preserved
    // un-degraded so the combinator arms (`all_of`/`any_of`/`none_of`/`not`)
    // recurse with it. Each recursive call then re-runs this same degrade
    // block for its own node â€” so a `Source` subject's source-stack leaves
    // (e.g. `is_face_down` / `is_bottom_source`) stay evaluable at any
    // combinator nesting depth.
    let original_subject = subject;
    let subject = if let PredicateSubject::Source(sref) = subject {
        if let Some(want) = pred.is_face_down {
            let actual = rctx
                .game
                .player(sref.permanent.player)
                .battle_area
                .get(sref.permanent.index as usize)
                .and_then(|perm| perm.card_sources.get(sref.source_index as usize))
                .map(|cs| cs.face_down);
            match actual {
                Some(face_down) if face_down == want => {}
                _ => return false,
            }
        }
        if let Some(want) = pred.is_bottom_source {
            let actual = sref.source_index == 0;
            if actual != want {
                return false;
            }
        }
        if let Some(want) = pred.host_kind_is {
            // The host permanent's top card is the visible Digimon/Tamer the
            // source sits beneath. If the host permanent cannot be resolved,
            // the predicate does not match. An in-battle_area permanent always
            // has a top card, so no empty-stack case is reachable (consistent
            // with the A3.1 `is_face_down` leaf). The top `CardSource`'s
            // `data_index` indexes `rctx.game.card_data` directly.
            let actual_kind = rctx
                .game
                .player(sref.permanent.player)
                .battle_area
                .get(sref.permanent.index as usize)
                .map(|perm| perm.synth_identity(rctx.card_data(), &rctx.game.modifiers, sref.permanent));
            match actual_kind {
                // `host_kind_is` reads a field permanent's top card, so it uses
                // the field-subject matcher (`Dual` coalesces to `Digimon`),
                // consistent with this file's other field-subject kind checks.
                Some(identity) if kind_matches_membership(want, identity.kind, identity.retained_kind) => {}
                _ => return false,
            }
        }
        PredicateSubject::Card(sref.card)
    } else {
        if pred.is_face_down.is_some()
            || pred.is_bottom_source.is_some()
            || pred.host_kind_is.is_some()
        {
            return false;
        }
        subject
    };

    // Game-state fields â€” independent of subject.
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
    // a no-op filter â€” the predicate passes regardless of whose turn
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
    // Controller-relative memory ("while YOU have N or less memory").
    // `game.memory` is stored from the turn player's perspective, so the
    // controller's signed memory is the gauge when it is their turn and the
    // negated gauge otherwise. G-DSL-OWN-MEMORY-PREDICATE (EX8-073 /
    // BT17-016 "while you have 0 or less memory" immunity gates).
    if pred.own_memory_lte.is_some() || pred.own_memory_gte.is_some() {
        let own_memory = if rctx.game.turn_player() == rctx.player {
            rctx.game.memory as i32
        } else {
            -(rctx.game.memory as i32)
        };
        if let Some(cap) = &pred.own_memory_lte {
            if own_memory > eval_int_constraint_read(cap, rctx, None, bindings) {
                return false;
            }
        }
        if let Some(floor) = &pred.own_memory_gte {
            if own_memory < eval_int_constraint_read(floor, rctx, None, bindings) {
                return false;
            }
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
    if let Some(want) = &pred.security_count_eq {
        if (rctx.security_count(rctx.player) as i32)
            != eval_int_constraint_read(want, rctx, None, bindings)
        {
            return false;
        }
    }
    if let Some(cap) = &pred.opponent_security_count_lte {
        if (rctx.security_count(rctx.opponent_id()) as i32)
            > eval_int_constraint_read(cap, rctx, None, bindings)
        {
            return false;
        }
    }
    if let Some(floor) = &pred.opponent_security_count_gte {
        if (rctx.security_count(rctx.opponent_id()) as i32)
            < eval_int_constraint_read(floor, rctx, None, bindings)
        {
            return false;
        }
    }
    // G-DSL-TOTAL-SECURITY-COUNT-PREDICATE: SUM of both players' security
    // stacks (controller + opponent). BT13-106 "if there're 6 or fewer total
    // cards in both players' security stacks". DCGO
    // `card.Owner.SecurityCards.Count + card.Owner.Enemy.SecurityCards.Count`.
    if pred.total_security_count_lte.is_some()
        || pred.total_security_count_gte.is_some()
        || pred.total_security_count_eq.is_some()
    {
        let total =
            (rctx.security_count(rctx.player) + rctx.security_count(rctx.opponent_id())) as i32;
        if let Some(cap) = &pred.total_security_count_lte {
            if total > eval_int_constraint_read(cap, rctx, None, bindings) {
                return false;
            }
        }
        if let Some(floor) = &pred.total_security_count_gte {
            if total < eval_int_constraint_read(floor, rctx, None, bindings) {
                return false;
            }
        }
        if let Some(want) = &pred.total_security_count_eq {
            if total != eval_int_constraint_read(want, rctx, None, bindings) {
                return false;
            }
        }
    }
    if let Some(cap) = &pred.face_up_security_count_lte {
        if face_up_security_count(rctx, rctx.player) as i32
            > eval_int_constraint_read(cap, rctx, None, bindings)
        {
            return false;
        }
    }
    if let Some(floor) = &pred.face_up_security_count_gte {
        if (face_up_security_count(rctx, rctx.player) as i32)
            < eval_int_constraint_read(floor, rctx, None, bindings)
        {
            return false;
        }
    }
    if let Some(spec) = &pred.no_face_up_security_named {
        // G-PRED-NO-FACE-UP-SECURITY-NAMED â€” fails if the named player has
        // ANY face-up security card matching the identity filter. Face-up
        // state lives in `Player.face_up_security` (a `card_index` set);
        // a card is counted only when both its identity matches and its
        // `card_index` is in that set.
        let players = resolve_predicate_players(spec.of, rctx);
        let card_data = rctx.card_data();
        let has_face_up_match = players.iter().any(|&player_id| {
            let player = rctx.game.player(player_id);
            player.security.iter().any(|card| {
                if !player.face_up_security.contains(&card.card_index) {
                    return false;
                }
                if let Some(card_number) = &spec.card_number_is {
                    card.card_id(card_data).eq_ignore_ascii_case(card_number)
                } else if let Some(name) = &spec.name_is {
                    card.card_name(card_data).eq_ignore_ascii_case(name)
                } else if let Some(want) = spec.color_is {
                    // Color arm (EX10-020 "no GREEN face-up security" â€” Q3).
                    card.colors(card_data)
                        .iter()
                        .any(|&got| color_matches(want, got))
                } else {
                    false
                }
            })
        });
        if has_face_up_match {
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
    if let Some((name, want)) = &pred.binding_count_eq {
        let count = bindings
            .and_then(|b| b.get_ref(name))
            .map(|value| match value {
                crate::dsl_cards::bindings::BindingValue::PermanentList(v) => v.len(),
                crate::dsl_cards::bindings::BindingValue::CardList(v) => v.len(),
                crate::dsl_cards::bindings::BindingValue::SourceRefs(v) => v.len(),
                // A scalar / single-target binding counts as one entry.
                _ => 1,
            })
            .unwrap_or(0);
        if count != usize::from(*want) {
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
    // G-DSL-DURING-ATTACK: "if during an attack" (DCGO
    // `attackProcess.IsAttacking`) â€” an attack is in flight.
    if let Some(want) = pred.during_attack {
        if rctx.game.pending_attack.is_some() != want {
            return false;
        }
    }
    if let Some(player_ref) = pred.digimon_attacked_this_turn {
        let attacked = resolve_predicate_players(player_ref, rctx)
            .into_iter()
            .any(|player| {
                rctx.game
                    .digimon_attacks_this_turn
                    .get(player as usize)
                    .copied()
                    .unwrap_or(0)
                    > 0
            });
        if !attacked {
            return false;
        }
    }
    if let Some(want) = pred.dna_origin {
        if rctx.dna_origin() != want {
            return false;
        }
    }
    if let Some(floor) = pred.distinct_tamer_colors_gte {
        // G-DSL-DISTINCT-TAMER-COLORS: count the distinct colors across
        // the observer's battle-area Tamer permanents. ST20-10's warp
        // alt-path condition ("your Tamers have 3 or more total colors").
        let mut colors: Vec<CardColor> = Vec::new();
        for perm in &rctx.game.player(rctx.player).battle_area {
            let top = perm.top_card();
            if top.card_kind(rctx.card_data()) != CardKind::Tamer {
                continue;
            }
            for c in top.colors(rctx.card_data()) {
                if !colors.contains(c) {
                    colors.push(*c);
                }
            }
        }
        if colors.len() < usize::from(floor) {
            return false;
        }
    }
    if let Some(spec) = &pred.distinct_named_count_gte {
        // G-DSL-DISTINCT-NAMED-PERMANENT-COUNT â€” count DISTINCT card names
        // among `of`'s battle-area permanents matching `filter`, then require
        // â‰¥ `n`. Names are synth-identity-aware (each permanent contributes its
        // synth `card_names`, so a `ChangeBaseCardName` overlay or a multi-name
        // card counts each distinct name). Driver: BT21-040 ("3 or more [Hero]
        // trait Tamers with different names"). No-subject global leaf.
        let mut names: Vec<String> = Vec::new();
        for player in resolve_predicate_players(spec.of, rctx) {
            let battle_len = rctx.game.player(player).battle_area.len();
            for index in 0..battle_len {
                let handle = PermanentHandle {
                    player,
                    index: index as u8,
                };
                if !eval_predicate_with_bindings(
                    &spec.filter,
                    rctx,
                    PredicateSubject::Permanent(handle),
                    bindings,
                ) {
                    continue;
                }
                let Some(perm) = permanent_for_handle(rctx, handle) else {
                    continue;
                };
                let synth = perm.synth_identity(rctx.card_data(), &rctx.game.modifiers, handle);
                for name in &synth.card_names {
                    if !names.iter().any(|n| n == name) {
                        names.push(name.clone());
                    }
                }
            }
        }
        if names.len() < usize::from(spec.n) {
            return false;
        }
    }
    if let Some(floor) = pred.face_down_sources_under_tamers_gte {
        // G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER: bottom face-down
        // digivolution sources across the observer's battle-area Tamers.
        // Gates BT25-035's optional free-digivolve on the trash-2 cost
        // actually being payable -- the same count as the cost's §15-7-4 guard
        // and its Tamer picks.
        let total =
            crate::dsl_cards::step::selections::face_down_sources_under_tamers(rctx, rctx.player);
        if total < usize::from(floor) {
            return false;
        }
    }
    if let Some(want) = pred.battle_opponent_no_sources {
        let Some(source) = rctx.source_permanent else {
            return false;
        };
        let Some(opponent) = rctx.battle_opponent_of(source) else {
            return false;
        };
        let Some(opponent_perm) = permanent_for_handle(rctx, opponent) else {
            return false;
        };
        let actual = opponent_perm.card_sources.len().saturating_sub(1) == 0;
        if actual != want {
            return false;
        }
    }
    if let Some(spec) = &pred.self_source_count {
        // G-DSL-SELF-SOURCE-COUNT-THRESHOLD: count the effect carrier's OWN
        // digivolution source cards (those beneath its top card) matching the
        // optional filter, then compare with `value` under `op`. Reads
        // `ctx.source_permanent` â€” a no-subject global predicate. Same
        // source-counting logic as the `source_stack_count` formula
        // (`card_sources.iter().rev().skip(1)` skips the top card), so the two
        // agree. DCGO `PermanentOfThisCard().DigivolutionCards.Count(cond)`.
        let count = rctx
            .source_permanent
            .and_then(|handle| permanent_for_handle(rctx, handle))
            .map(|perm| {
                perm.card_sources
                    .iter()
                    .rev()
                    .skip(1)
                    .filter(|source| {
                        spec.filter.as_deref().is_none_or(|filter| {
                            eval_predicate_with_bindings(
                                filter,
                                rctx,
                                PredicateSubject::Card(source.handle()),
                                bindings,
                            )
                        })
                    })
                    .count() as i32
            })
            .unwrap_or(0);
        let threshold = eval_int_constraint_read(&spec.value, rctx, None, bindings);
        if !spec.op.apply(count, threshold) {
            return false;
        }
    }
    if let Some(floor) = pred.self_same_level_source_pairs_gte {
        // G-DSL-SELF-SAME-LEVEL-SOURCE-PAIRS: count pairs among the effect
        // carrier's own source cards below its top card. This mirrors the
        // `same_level_pairs_in_sources` formula selector so trigger predicates
        // and formula counts agree.
        let count = rctx
            .source_permanent
            .and_then(|handle| permanent_for_handle(rctx, handle))
            .map(|perm| same_level_pairs_in_sources(perm, rctx.card_data()))
            .unwrap_or(0);
        if count < i32::from(floor) {
            return false;
        }
    }
    if let Some(floor) = pred.own_source_stack_color_count_gte {
        // G-DSL-OWN-SOURCE-STACK-COLOR-COUNT-THRESHOLD: distinct colors among
        // the effect CARRIER's NON-FLIPPED digivolution sources (top card
        // excluded, face-down sources excluded) â€” the SAME shared extraction
        // as `color_matches_own_source_stack` (`non_flipped_source_colors`),
        // so the branch gate and the Branch-A filter always agree. Reads
        // `ctx.source_permanent` â€” a no-subject, carrier-scoped global leaf.
        // The EX9-074 Kimeramon branch discriminant: "if this Digimon has 6
        // or more colors in its digivolution cards, instead â€¦" (DCGO
        // `DigivolutionCards.Filter(!IsFlipped).SelectMany(CardColors)
        // .Distinct().Count`). No carrier â†’ count 0 â†’ the floor fails.
        let count = rctx
            .source_permanent
            .and_then(|handle| permanent_for_handle(rctx, handle))
            .map(|perm| {
                crate::dsl_cards::formula_eval::distinct_non_flipped_source_color_count(
                    perm,
                    rctx.card_data(),
                )
            })
            .unwrap_or(0);
        if count < i32::from(floor) {
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
        if !eval_predicate_with_bindings(inner, rctx, PredicateSubject::Card(target), bindings) {
            return false;
        }
    }
    // G-BEFORE-PAY-COST-DIGIVOLVE-TARGET: gate to "THIS permanent is
    // the digivolve target" â€” fires when the effect's source_permanent
    // is one of the digivolve target permanents (single entry for
    // normal digivolve, both materials for DNA).
    if let Some(want) = pred.source_is_cost_target_permanent {
        if rctx.source_is_cost_target_permanent() != want {
            return false;
        }
    }
    // G-DSL-COST-TARGET-FROM-HAND: subject-free cost-calc gate â€” the card
    // whose cost is being computed comes from the hand (DCGO IsExistOnHand).
    if let Some(want) = pred.cost_target_from_hand {
        if rctx.cost_target_from_hand != want {
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
    if let Some(want) = pred.is_source_permanent {
        // `target: self` â€” true when the subject permanent IS the effect's
        // source permanent (the carrier in face_up scope, the host in inherited
        // scope). Lets flood_gate/aura install a modifier on the carrier itself.
        let is_src =
            matches!(subject, PredicateSubject::Permanent(h) if rctx.source_permanent == Some(h));
        if is_src != want {
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
    if let Some(ref needle) = pred.self_digivolution_sources_contain_name {
        // G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY: scan only the
        // digivolution source cards beneath the carrier, excluding the
        // carrier's own top card. `card_sources` stores the top card at
        // `last()`, so the sources are `[0 .. len-1]` â€” mirrors the
        // `.take(len-1)` idiom in `permanent_material_count`.
        let Some(perm) = subject_or_source_permanent(subject, rctx) else {
            return false;
        };
        let source_match = perm
            .card_sources
            .iter()
            .take(perm.card_sources.len().saturating_sub(1))
            .any(|card| card.contains_card_name(needle, rctx.card_data()));
        if !source_match {
            return false;
        }
    }
    if let Some(ref trait_name) = pred.self_digivolution_sources_trait_has {
        let Some(perm) = subject_or_source_permanent(subject, rctx) else {
            return false;
        };
        let source_match = perm
            .card_sources
            .iter()
            .take(perm.card_sources.len().saturating_sub(1))
            .filter_map(|card| rctx.game.card_data_for_handle(card.handle()))
            .any(|data| {
                data.traits
                    .iter()
                    .any(|x| x.eq_ignore_ascii_case(trait_name))
            });
        if !source_match {
            return false;
        }
    }
    if let Some(ref needle) = pred.source_name_contains {
        let Some(perm) = subject_or_source_permanent(subject, rctx) else {
            return false;
        };
        if !rctx.game.card_rules_name_contains(perm.top_card().handle(), needle)
        {
            return false;
        }
    }
    if let Some(ref want) = pred.source_name_is {
        // "this Digimon is [X]" / "this Digimon's name is [X]": an EXACT match
        // on one of the carrier's names -- its top card's synthesized rules
        // names (printed, "also treated as", gained by effects), the same
        // identity `source_name_contains` reads. Digivolution cards are not
        // read (§4-8-2; DCGO `TopCard.EqualsCardName`).
        // G-DSL-CARRIER-NAME-READS-DIGIVOLUTION-CARDS.
        let Some(perm) = subject_or_source_permanent(subject, rctx) else {
            return false;
        };
        if !rctx
            .game
            .card_rules_name_matches(perm.top_card().handle(), |name| name == want.as_str())
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
        if !data
            .traits
            .iter()
            .any(|x| x.eq_ignore_ascii_case(trait_name))
        {
            return false;
        }
    }
    // PUPPETS-G025: case-insensitive substring match against the carrier
    // permanent's printed rules text (effect_text + inherited_text +
    // security_text of the top card). Used by BT16-055 to gate its
    // inherited +1000 DP aura on "while this Digimon has [Pulsemon] in
    // its text." In an inherited while_condition context the subject is
    // the carrier permanent.
    if let Some(ref needle) = pred.rules_text_contains {
        let Some(perm) = subject_or_source_permanent(subject, rctx) else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(perm.top_card().handle()) else {
            return false;
        };
        let needle_lc = needle.to_lowercase();
        let found = data.effect_text.to_lowercase().contains(&needle_lc)
            || data.inherited_text.to_lowercase().contains(&needle_lc)
            || data.security_text.to_lowercase().contains(&needle_lc);
        if !found {
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

    // Combinators â€” short-circuit on first failure. Recurse with the
    // un-degraded `original_subject` so a `Source` subject keeps its
    // source-stack metadata; each child re-runs the degrade block above.
    for child in &pred.all_of {
        if !eval_predicate_ref(child, rctx, original_subject, bindings, dual_ref) {
            return false;
        }
    }
    if !pred.any_of.is_empty() {
        let any_match = pred
            .any_of
            .iter()
            .any(|c| eval_predicate_ref(c, rctx, original_subject, bindings, dual_ref));
        if !any_match {
            return false;
        }
    }
    for child in &pred.none_of {
        if eval_predicate_ref(child, rctx, original_subject, bindings, dual_ref) {
            return false;
        }
    }
    if let Some(inner) = &pred.not {
        if eval_predicate_ref(inner, rctx, original_subject, bindings, dual_ref) {
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
    if let Some(binding_kind) = &pred.binding_card_kind {
        // Resolve the named card binding and compare its printed category.
        // Used by LM-020 to test the revealed opponent deck-top against the
        // declared category. Fails closed when the binding is unset or the
        // card data can't be resolved. Also resolves an origin-tagged
        // `select_union_zone` binding, so a "play OR use 1 card from your hand
        // or its digivolution cards" body can branch Option â†’ `use_option_bound`
        // vs Digimon/Tamer â†’ `play_union_bound_free` (EX13-045 Examon).
        // G-DSL-BINDING-CARD-KIND-UNION. A single-card `CardList` (e.g. a
        // min:0/max:1 `select_reveal_buckets` bucket, BT26-084) resolves like
        // a `Card` binding.
        let Some(handle) = bindings.and_then(|b| {
            b.get_card(&binding_kind.binding)
                .or_else(|| b.get_union_card(&binding_kind.binding).map(|(c, _, _)| c))
                .or_else(|| {
                    b.get_card_list(&binding_kind.binding)
                        .filter(|l| l.len() == 1)
                        .map(|l| l[0])
                })
        }) else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(handle) else {
            return false;
        };
        if !kind_matches_card_search(binding_kind.kind, data) {
            return false;
        }
    }
    if let Some(binding_color) = &pred.binding_card_color {
        // G-DSL-BINDING-CARD-COLOR â€” resolve the named card binding and test
        // whether its printed color set contains the requested color. Sibling
        // of `binding_card_kind`. Driver: BT1-087 ("if THAT card is yellow,
        // trigger <Recovery +1>"). Fails closed when the binding is unset or
        // the card can't be resolved.
        let Some(handle) = bindings.and_then(|b| b.get_card(&binding_color.binding)) else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(handle) else {
            return false;
        };
        if !data
            .colors
            .iter()
            .any(|c| color_matches(binding_color.color_is, *c))
        {
            return false;
        }
    }
    if let Some((binding_name, want)) = &pred.binding_card_name_is {
        // G-DSL-BINDING-CARD-NAME-EQUALS â€” resolve the named card binding and
        // compare its effective name (printed `card_name` + static
        // `also_treated_as` aliases) to `want`. Mirrors the static `name_is`
        // leaf's exact match, extended over the identity aliases so a card
        // "also treated as [X]" satisfies `name_is: X`. Sibling of
        // `binding_card_kind`. Driver: BT21-087 ("play 1 [Vemmon]").
        let Some(handle) = bindings.and_then(|b| b.get_card(binding_name)) else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(handle) else {
            return false;
        };
        if !any_effective_name_matches(&data.card_name, None, &data.also_treated_as, |name| {
            name == want.as_str()
        }) {
            return false;
        }
    }

    // Existentials â€” scan battle areas.
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
    if let Some(agg) = &pred.level_sum_gte {
        let floor = eval_int_constraint_read(&agg.n, rctx, None, bindings).max(0) as u32;
        if level_sum_matching(agg, rctx, bindings) < floor {
            return false;
        }
    }

    // `has_face_down_source` is a permanent-subject-only leaf: it reads a
    // permanent's `card_sources` stack. Only `Permanent` / `BreedingPermanent`
    // can satisfy it â€” on any other subject (`Card`, `RevealedCard`, `None`,
    // and a `Source` subject already degraded to `Card` above) a present
    // `has_face_down_source` leaf is an unconditional non-match.
    if pred.has_face_down_source.is_some()
        && !matches!(
            subject,
            PredicateSubject::Permanent(_) | PredicateSubject::BreedingPermanent(_)
        )
    {
        return false;
    }

    match subject {
        PredicateSubject::Card(card) => {
            eval_card_fields(pred, rctx, card, false, None, bindings, dual_ref)
        }
        PredicateSubject::RevealedCard(card) => {
            eval_card_fields(pred, rctx, card, true, None, bindings, dual_ref)
        }
        PredicateSubject::Permanent(h) => eval_permanent_fields(pred, rctx, h, bindings),
        PredicateSubject::BreedingPermanent(player) => {
            eval_breeding_permanent_fields(pred, rctx, player, bindings)
        }
        // A `Source` subject is always degraded to `Card` by the
        // degrade-to-Card block at the top of this function, so this arm
        // is unreachable in practice; route it through the card path for
        // exhaustiveness and forward-compat safety.
        PredicateSubject::Source(sref) => {
            eval_card_fields(pred, rctx, sref.card, false, None, bindings, dual_ref)
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
    if let Some(traits) = pred.would_link_card_trait_any_of.as_ref() {
        // Read the card about to link in the active standing-Digimon
        // `WhenWouldLink` window and check it carries one of the listed traits
        // (Gap 5 â€” BT25-004 / BT25-045 host-side reducer trait gate).
        let Some(card) = rctx.would_link_subject_card() else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(card) else {
            return false;
        };
        if !data
            .traits
            .iter()
            .any(|t| traits.iter().any(|want| want == t))
        {
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
            && pred.effect_suspended_any_opponent_digimon != Some(true)
            && pred.effect_returned_any_card != Some(true)
            && pred.returned_card_matching.is_none()
            && pred.effect_trashed_any_hand_card != Some(true)
            && pred.trashed_hand_card_matching.is_none()
            && pred.effect_deleted_any_own_digimon != Some(true)
            && pred.effect_deleted_any_opponent_digimon != Some(true)
            && pred.effect_deleted_opponent_digimon_dp_gte.is_none()
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
    // G-DSL-EFFECT-SUSPENDED-RESULT â€” opponent-side sibling. The result
    // log records every suspend regardless of owner (`record_suspended`
    // is owner-agnostic); the own/opponent split happens here at read
    // time, mirroring `effect_deleted_any_opponent_digimon`.
    if let Some(want) = pred.effect_suspended_any_opponent_digimon {
        let actual = log.suspended.iter().any(|h| h.player != rctx.player);
        if actual != want {
            return false;
        }
    }
    if let Some(want) = pred.effect_returned_any_card {
        let actual = !log.returned_cards.is_empty();
        if actual != want {
            return false;
        }
    }
    // G-ANY-RETURNED-CARD-PREDICATE â€” filtered result-set predicate. True when
    // at least one card identity recorded by a preceding return / zone-move
    // step in this effect satisfies the inner card-shape predicate. Each
    // returned `CardHandle` is resolved to its identity via
    // `card_data_for_handle` (zone-agnostic) and evaluated as a `Card` subject.
    if let Some(inner) = &pred.returned_card_matching {
        let any_match = log.returned_cards.iter().any(|&handle| {
            eval_predicate_with_bindings(
                inner,
                rctx,
                PredicateSubject::Card(handle),
                Some(bindings),
            )
        });
        if !any_match {
            return false;
        }
    }
    // G-DSL-EFFECT-TRASHED-HAND-CARD â€” hand-trash siblings of the returned-card
    // result predicates. `trash_from_hand_by_index` / hand-origin
    // `trash_union_bound` record the trashed card's identity; the filtered
    // variant evaluates each recorded handle as a `Card` subject (zone-agnostic
    // identity read, so the card now sitting in the trash is what is tested).
    // Driver P-212 Asuna Shiroki ("If this effect trashed a card with the
    // [Three Musketeers] or [TS] trait, ...").
    if let Some(want) = pred.effect_trashed_any_hand_card {
        let actual = !log.trashed_from_hand.is_empty();
        if actual != want {
            return false;
        }
    }
    if let Some(inner) = &pred.trashed_hand_card_matching {
        let any_match = log.trashed_from_hand.iter().any(|&handle| {
            eval_predicate_with_bindings(
                inner,
                rctx,
                PredicateSubject::Card(handle),
                Some(bindings),
            )
        });
        if !any_match {
            return false;
        }
    }
    if let Some(want) = pred.effect_deleted_any_own_digimon {
        let actual = log
            .deleted
            .iter()
            .zip(&log.deleted_is_digimon)
            .any(|((h, _), digimon)| *digimon && h.player == rctx.player);
        if actual != want {
            return false;
        }
    }
    if let Some(want) = pred.effect_deleted_any_opponent_digimon {
        let actual = log
            .deleted
            .iter()
            .zip(&log.deleted_is_digimon)
            .any(|((h, _), digimon)| *digimon && h.player != rctx.player);
        if actual != want {
            return false;
        }
    }
    // G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD â€” true iff at least one OPPONENT
    // Digimon deleted by THIS effect had pre-removal effective DP >= N. The DP
    // was snapshotted at the delete-step call site (the carrier is in trash by
    // now, so the log's captured DP is the only faithful source). DCGO
    // EX4_065.cs:71 uses the identical `DPJustBeforeRemoveField >= 13000`.
    if let Some(floor) = &pred.effect_deleted_opponent_digimon_dp_gte {
        let n = eval_int_constraint_read(floor, rctx, rctx.source_permanent, Some(bindings));
        let actual = log
            .deleted
            .iter()
            .zip(&log.deleted_is_digimon)
            .any(|((h, dp), digimon)| {
                *digimon && h.player != rctx.player && dp.is_some_and(|d| d >= n)
            });
        if !actual {
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

fn face_up_security_count(rctx: &EffectReadContext<'_>, player_id: PlayerId) -> usize {
    let player = rctx.game.player(player_id);
    player
        .security
        .iter()
        .filter(|card| player.face_up_security.contains(&card.card_index))
        .count()
}

/// Summed level of every battle-area permanent matching `aggregate.filter`
/// (owner defaults to `you`, like `count_matching`). Level-less permanents
/// (Tamers, field Options) contribute 0. Only the battle area holds "Digimon
/// on the field", so a non-battle-area `zone` contributes nothing. Backs the
/// `level_sum_gte` predicate (BT25-077 Bacchusmon). G-DSL-BOARD-LEVEL-SUM.
fn level_sum_matching(
    aggregate: &CompiledCountAggregate,
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> u32 {
    let filter = aggregate.filter.as_ref();
    if !filter.zone.is_empty() && !filter.zone.contains(&CompiledZone::BattleArea) {
        return 0;
    }
    let owners = existential_players(filter.owner.unwrap_or(CompiledPlayerRef::You), rctx);
    let mut subject_filter = filter.clone();
    subject_filter.zone.clear();
    subject_filter.owner = None;

    let mut sum = 0u32;
    for owner in owners {
        let player = rctx.game.player(owner);
        for (index, perm) in player.battle_area.iter().enumerate() {
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
                sum += perm.level(rctx.card_data()).map(u32::from).unwrap_or(0);
            }
        }
    }
    sum
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
        && pred.can_play_from_effect_without_cost.is_none()
        && pred.can_play_to_breeding_from_hand.is_none()
        && pred.level_eq.is_none()
        && pred.level_eq_binding.is_none()
        && pred.level_lte_binding.is_none()
        && pred.level_gte_binding.is_none()
        && pred.level_lte.is_none()
        && pred.level_gte.is_none()
        && pred.level_matches_aggregate.is_none()
        && pred.materials_count_matches_aggregate.is_none()
        && pred.color_is.is_none()
        && pred.color_only.is_none()
        && pred.color_matches_any_field_digimon.is_none()
        && pred.color_matches_binding.is_none()
        && pred.color_matches_returned_card.is_none()
        && pred.color_matches_own_source_stack.is_none()
        && pred.trait_has.is_none()
        && pred.trait_contains.is_none()
        && pred.form_is.is_none()
        && pred.attribute_is.is_none()
        && pred.name_is.is_none()
        && pred.name_contains.is_none()
        && pred.effect_text_contains.is_none()
        && pred.printed_keyword.is_none()
        && pred.in_text_contains.is_none()
        && pred.name_in.is_none()
        && pred.name_not_shared_by_field_digimon.is_none()
        && pred.name_not_shared_by_field_tamer.is_none()
        && pred.card_number_is.is_none()
        && pred.play_cost_lte.is_none()
        && pred.play_cost_gte.is_none()
        && pred.play_cost_eq.is_none()
        && pred.play_cost_eq_binding.is_none()
        && pred.play_or_use_cost_lte.is_none()
        && pred.self_color_count_gte.is_none()
        && pred.self_color_count_lte.is_none()
        && pred.in_event_discarded_cards.is_none()
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
    let Some(value) = bindings.and_then(|b| b.get_ref(binding_name)) else {
        return false;
    };
    match subject {
        PredicateSubject::Permanent(handle) => match value {
            BindingValue::Permanent(bound) => *bound != handle,
            BindingValue::PermanentList(list) => !list.contains(&handle),
            _ => false,
        },
        PredicateSubject::Card(card) | PredicateSubject::RevealedCard(card) => match value {
            BindingValue::Card(bound) | BindingValue::UnionCard { card: bound, .. } => *bound != card,
            BindingValue::CardList(list) => !list.contains(&card),
            BindingValue::SourceRefs(list) => !list.iter().any(|entry| entry.card == card),
            _ => false,
        },
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
    if let Some(want) = pred.event_target_is_source {
        let event_target = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| {
                if trigger.event_permanent_identity.is_some() {
                    return trigger.resolve_event_permanent(rctx.game);
                }
                trigger
                    .event_permanent
                    .or(trigger.target_permanent)
                    .or(trigger.event_host_permanent)
            });
        let source = match subject {
            PredicateSubject::Permanent(handle) => Some(handle),
            PredicateSubject::BreedingPermanent(player) => Some(PermanentHandle {
                player,
                index: crate::action::space::BREEDING_TARGET as u8,
            }),
            PredicateSubject::Card(_)
            | PredicateSubject::RevealedCard(_)
            | PredicateSubject::Source(_)
            | PredicateSubject::None => rctx.source_permanent,
        };
        let is_source =
            matches!((event_target, source), (Some(event), Some(source)) if event == source);
        if is_source != want {
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
            | PredicateSubject::Source(_)
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
        let has_trait = |traits: &[String]| traits.iter().any(|t| t.eq_ignore_ascii_case(trait_name));
        let pinned_event = rctx
            .game
            .current_trigger_context
            .as_ref()
            .filter(|trigger| trigger.event_permanent_identity.is_some());
        let attacker = if let Some(trigger) = pinned_event {
            trigger.resolve_event_permanent(rctx.game).ok_or_else(|| {
                // The attacker has left the field: a clause-level gate reads
                // its last-known information (`Game::gate_trigger_context`
                // exposes the trigger's event snapshot as `deleted_object`).
                trigger.deleted_object.as_ref().map(|snapshot| has_trait(&snapshot.traits))
            })
        } else {
            rctx.attack_target_change()
                .map(|change| change.attacker)
                .or_else(|| {
                    rctx.game
                        .current_trigger_context
                        .as_ref()
                        .and_then(|trigger| trigger.event_permanent)
                })
                .or_else(|| rctx.attack_attacker())
                .ok_or(None)
        };
        let matched = match attacker {
            Err(snapshot_match) => snapshot_match.unwrap_or(false),
            Ok(attacker) => rctx
                .game
                .player(attacker.player)
                .battle_area
                .get(attacker.index as usize)
                .and_then(|perm| rctx.game.card_data_for_handle(perm.top_card().handle()))
                .is_some_and(|data| has_trait(&data.traits)),
        };
        if !matched {
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
            if !kind_matches_membership(want, snapshot.card_kind, snapshot.retained_kind) {
                return false;
            }
        } else {
            let Some(card) = event_target_card(rctx) else {
                return false;
            };
            let Some(data) = rctx.game.card_data_for_handle(card) else {
                return false;
            };
            // Event targets denote a surviving permanent when the captured
            // physical card is its current top. Card-only events (including a
            // source under a host) retain printed card-search classification.
            let field = rctx.game.players.iter().enumerate().find_map(|(player, p)| {
                p.battle_area.iter().enumerate().find_map(|(index, permanent)| {
                    (permanent.card_sources.last().map(|c| c.handle()) == Some(card))
                        .then_some(PermanentHandle { player: player as PlayerId, index: index as u8 })
                })
            });
            let matches = if let Some(handle) = field {
                eval_predicate(
                    &CompiledPredicate { kind: Some(want), ..Default::default() },
                    rctx,
                    PredicateSubject::Permanent(handle),
                )
            } else {
                kind_matches_card_search(want, data)
            };
            if !matches {
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
    if let Some(ref token) = pred.event_target_trait_contains {
        // G-DSL-EVENT-TARGET-TRAIT-CONTAINS â€” substring / root-trait sibling of
        // `event_target_trait_has`, tolerant of pluralization / compound traits
        // (BT11-089 Q&A: "regardless of other words or pluralizations"). Mirrors
        // the snapshot / live-target split. Case-insensitive substring match.
        let needle = token.to_lowercase();
        let matched = if let Some(snapshot) = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.deleted_object.as_ref())
        {
            snapshot
                .traits
                .iter()
                .any(|t| t.to_lowercase().contains(&needle))
        } else {
            let Some(card) = event_target_card(rctx) else {
                return false;
            };
            let Some(data) = rctx.game.card_data_for_handle(card) else {
                return false;
            };
            data.traits
                .iter()
                .any(|t| t.to_lowercase().contains(&needle))
        };
        if !matched {
            return false;
        }
    }
    if let Some(want) = pred.event_target_has_digivolution_cards {
        // G-DSL-EVENT-TARGET-SOURCE-COUNT â€” true when the event-target
        // permanent has â‰¥1 digivolution card. `event_target_stack_size` reads
        // the deletion snapshot's pre-removal count for a deletion event, or
        // the live source stack otherwise. "Has a digivolution card" â‡” stack
        // size â‰¥ 2 (top card + â‰¥1 source). Fails closed when unresolvable.
        let Some(stack_size) = event_target_stack_size(rctx) else {
            return false;
        };
        let has_sources = stack_size >= 2;
        if has_sources != want {
            return false;
        }
    }
    if let Some(min) = pred.event_target_stack_size_gte {
        // G-DSL-EVENT-TARGET-SOURCE-COUNT â€” the event-target permanent's full
        // stack size (top card + sources) must be â‰¥ N.
        let Some(stack_size) = event_target_stack_size(rctx) else {
            return false;
        };
        if stack_size < usize::from(min) {
            return false;
        }
    }
    if let Some(want) = pred.event_target_level_eq {
        let Some(level) = event_target_level(rctx) else {
            return false;
        };
        if level != want {
            return false;
        }
    }
    if let Some(max) = &pred.event_target_level_lte {
        let Some(level) = event_target_level(rctx) else {
            return false;
        };
        let max = eval_int_constraint(max, rctx, None, None);
        if i32::from(level) > max {
            return false;
        }
    }
    if let Some(min) = &pred.event_target_level_gte {
        let Some(level) = event_target_level(rctx) else {
            return false;
        };
        let min = eval_int_constraint(min, rctx, None, None);
        if i32::from(level) < min {
            return false;
        }
    }
    if let Some(want) = &pred.event_target_dp_eq {
        let Some(dp) = event_target_dp(rctx) else {
            return false;
        };
        if dp != eval_int_constraint(want, rctx, None, None) {
            return false;
        }
    }
    if let Some(max) = &pred.event_target_dp_lte {
        let Some(dp) = event_target_dp(rctx) else {
            return false;
        };
        if dp > eval_int_constraint(max, rctx, None, None) {
            return false;
        }
    }
    if let Some(min) = &pred.event_target_dp_gte {
        let Some(dp) = event_target_dp(rctx) else {
            return false;
        };
        if dp < eval_int_constraint(min, rctx, None, None) {
            return false;
        }
    }
    if let Some(ref needle) = pred.event_target_name_contains {
        // G-EVENT-TARGET-NAME-CONTAINS: case-insensitive substring scan
        // against the event-target permanent's card name (the digivolving /
        // played / deleted permanent on the triggered-effect context).
        let want = needle.to_lowercase();
        if let Some(snapshot) = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.deleted_object.as_ref())
        {
            // Deleted object is gone from the field â€” match its snapshot name:
            // only the last-known effective rules names captured in
            // `names_just_before` (synth identity â€” includes a
            // `ChangeBaseCardName` overlay live at deletion time).
            let name_match = snapshot
                    .names_just_before
                    .iter()
                    .any(|name| crate::card_data::rules_name_contains(name, &want));
            if !name_match {
                return false;
            }
        } else {
            let Some(card) = event_target_card(rctx) else {
                return false;
            };
            if !rctx.game.card_rules_name_contains(card, &want) {
                return false;
            }
        }
    }
    if let Some(ref needle) = pred.event_target_in_text_contains {
        let Some(card) = event_target_card(rctx) else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(card) else {
            return false;
        };
        if !card_in_text_contains(data, needle) {
            return false;
        }
    }
    if let Some(want) = pred.event_target_has_inherited {
        // G-DSL-EVENT-TARGET-HAS-INHERITED â€” DCGO `TopCard.HasInheritedEffect`.
        let Some(card) = event_target_card(rctx) else {
            return false;
        };
        let Some(data) = rctx.game.card_data_for_handle(card) else {
            return false;
        };
        if data.inherited_text.trim().is_empty() == want {
            return false;
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
    if let Some(want) = pred.event_add_to_hand_player {
        // OnAddToHand observer: the gaining player is carried in
        // `affected_player`. Compare it to the requested ref (you/opponent),
        // resolved relative to the observer. See G-ON-ADD-TO-HAND-OBSERVER.
        let Some(gaining_player) = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.affected_player)
        else {
            return false;
        };
        if !player_ref_matches(want, gaining_player, rctx) {
            return false;
        }
    }
    if let Some(want) = pred.event_winner_owner {
        // on_ally_won_battle: the winner's controller must match the ref. A tie
        // has no winner -> fail. The controller comes with the winner's
        // identity, so it holds after the winner has left the field.
        // G-DSL-BATTLE-WINNER-BOARDWIDE.
        let Some(owner) = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.battle_winner_controller())
        else {
            return false;
        };
        if !player_ref_matches(want, owner, rctx) {
            return false;
        }
    }
    if let Some(ref trait_name) = pred.event_winner_trait_has {
        // on_ally_won_battle: the winner must carry the trait. The winner is
        // found by identity, never by the slot named at enqueue (a lower-index
        // departure shifts it onto another Digimon); while it is on the field
        // it is read live (§15-4-4-5), and once it has left a clause-level
        // gate reads its last-known information (`Game::gate_trigger_context`
        // exposes the battle's winner snapshot as `deleted_object`):
        // §15-8-3-1 / §15-8-3-8. G-ENGINE-BATTLE-WINNER-QUALIFIER-STALE-INDEX.
        let matches = rctx
            .game
            .current_trigger_context
            .as_ref()
            .is_some_and(|trigger| trigger.battle_winner_has_trait(rctx.game, trait_name));
        if !matches {
            return false;
        }
    }
    if let Some(want) = pred.event_winner_is_source {
        // on_ally_won_battle: is the winner THIS effect's carrier? A tie has no
        // winner -> fail. Resolved by identity: a winner that has left the
        // field is not the carrier on the field. G-DSL-BATTLE-WINNER-IS-SOURCE.
        let Some(trigger) = rctx.game.current_trigger_context.as_ref() else {
            return false;
        };
        if trigger.battle_winner_controller().is_none() {
            return false;
        }
        let winner = trigger.resolve_battle_winner(rctx.game);
        if (winner.is_some() && winner == rctx.source_permanent) != want {
            return false;
        }
    }
    if let Some(inner) = &pred.event_battle_deleter {
        let matches = rctx
            .game
            .current_trigger_context
            .as_ref()
            .is_some_and(|trigger| {
                trigger
                    .battle_deleters
                    .iter()
                    .any(|snapshot| eval_battle_deleter_snapshot(inner, rctx, snapshot))
            });
        if !matches {
            return false;
        }
    }
    if let Some(inner) = &pred.event_battle_surviving_deleter {
        let matches = rctx
            .game
            .current_trigger_context
            .as_ref()
            .is_some_and(|trigger| {
                trigger
                    .battle_deleters
                    .iter()
                    .any(|snapshot| snapshot.survived && eval_battle_deleter_snapshot(inner, rctx, snapshot))
            });
        if !matches {
            return false;
        }
    }
    if let Some(want) = pred.event_discard_player {
        // on_discard_hand: the trashing player (whose hand lost cards) must
        // match the ref ("your hand is trashed from" â‡’ you). G-ENGINE-ON-DISCARD-HAND.
        let Some(trashing_player) = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.discard_hand_player)
        else {
            return false;
        };
        if !player_ref_matches(want, trashing_player, rctx) {
            return false;
        }
    }
    if let Some(want) = pred.event_caused_by_own_effect {
        // on_discard_hand (ST16-14 Matt Ishida "one of YOUR effects") and
        // on_add_digivolution_cards (BT7-056 Dorumon "when one of your effects
        // places a digivolution card under this Digimon"): true when the
        // causing effect belongs to the observer. Reads the discard batch's
        // `discard_cause_controller`, else the explicit `event_cause_effect`
        // captured by the source-placement facade. Mirrors DCGO
        // `cardEffect.EffectSourceCard.Owner == card.Owner`.
        let is_own = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| {
                trigger
                    .discard_cause_controller
                    .or_else(|| trigger.event_cause_effect.map(|cause| cause.controller))
            })
            .map(|controller| controller == rctx.player())
            .unwrap_or(false);
        if is_own != want {
            return false;
        }
    }
    if let Some(inner) = &pred.event_added_card_any {
        // on_add_digivolution_cards: ANY card of the just-placed batch matches
        // the inner card predicate (DCGO `CanTriggerOnAddDigivolutionCard`'s
        // `CardSources.Count(cardCondition) >= 1`). Outside that timing the
        // batch is empty and the gate fails. G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS.
        let added: Vec<CardHandle> = rctx
            .game
            .current_trigger_context
            .as_ref()
            .map(|trigger| trigger.added_source_cards().to_vec())
            .unwrap_or_default();
        let any_match = added.iter().any(|card| {
            eval_predicate_with_bindings(inner, rctx, PredicateSubject::Card(*card), None)
        });
        if !any_match {
            return false;
        }
    }
    if let Some(inner) = &pred.event_discarded_card_any {
        // on_discard_hand: ANY card of the just-trashed hand batch matches the
        // inner card predicate (DCGO `CanTriggerOnTrashHand(â€¦, cardCondition)`
        // over `DiscardedCards`). Outside that timing the batch is empty and
        // the gate fails. G-ENGINE-DISCARDED-HAND-CARDS.
        let discarded: Vec<CardHandle> = rctx
            .game
            .current_trigger_context
            .as_ref()
            .map(|trigger| trigger.discarded_hand_cards().to_vec())
            .unwrap_or_default();
        let any_match = discarded.iter().any(|card| {
            eval_predicate_with_bindings(inner, rctx, PredicateSubject::Card(*card), None)
        });
        if !any_match {
            return false;
        }
    }
    if let Some(want) = pred.played_by_effect {
        // BT25-080 main clause tail: the OnPlay firing forwards whether the
        // play was effect-driven into `effect_initiated`. G-ENGINE-ON-DISCARD-HAND.
        let is_by_effect = rctx
            .game
            .current_trigger_context
            .as_ref()
            .map(|trigger| trigger.effect_initiated)
            .unwrap_or(false);
        if is_by_effect != want {
            return false;
        }
    }
    if let Some(ref wanted_colors) = pred.event_target_color_any_of {
        // G-EVENT-TARGET-COLOR: the event-target permanent's printed color
        // set must intersect the requested list. Mirrors the snapshot /
        // live-card split used by `event_target_kind` â€” for a deletion
        // event the live slot is gone, so read colors from the deleted
        // top card's `CardData` (still present in `card_data`).
        let card = if let Some(snapshot) = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.deleted_object.as_ref())
        {
            snapshot.top_card
        } else {
            let Some(card) = event_target_card(rctx) else {
                return false;
            };
            card
        };
        let Some(data) = rctx.game.card_data_for_handle(card) else {
            return false;
        };
        let any_match = data
            .colors
            .iter()
            .any(|c| wanted_colors.iter().any(|w| color_matches(*w, *c)));
        if !any_match {
            return false;
        }
    }
    if let Some(want) = pred.event_permanent_is_source {
        let Some(trigger) = rctx.game.current_trigger_context.as_ref() else {
            return false;
        };
        let event_permanent = if trigger.deleted_object.is_some() {
            trigger.event_permanent
        } else {
            trigger.resolve_event_permanent(rctx.game)
        };
        let Some(event_permanent) = event_permanent else {
            return false;
        };
        let Some(source_permanent) = rctx.source_permanent else {
            return false;
        };
        // `PermanentHandle` is (player, battle-area INDEX), and the battle
        // area COMPACTS when a permanent is removed. A deletion trigger's
        // `event_permanent` is therefore a DANGLING handle by the time the
        // observer is evaluated: `enqueue_batch_on_any_deletion` enqueues the
        // pre-trash handle, but the surviving permanents have already slid
        // down. Every survivor that sat ABOVE the deleted one now answers to
        // a lower index â€” and the one that inherits the dead permanent's
        // index compares EQUAL to it.
        //
        // That alias is how `[All Turns] When other Digimon or Tamers are
        // deleted` observers silently lost a deletion: an ally deleted from a
        // LOWER index than the carrier made `event_permanent ==
        // source_permanent` true, so the clause's `event_permanent_is_source:
        // false` gate rejected the carrier's own trigger. Measured on exam
        // `BT19-075#effect#2` (MoonMillenniummon, oracle sidecar
        // `20260921T041928Z_40f7887a`: `p1.security ours=4 dcgo=3`).
        //
        // `general_rule.pdf` 15-8-3-1 / 15-8-3-2 / 15-8-3-5 / 15-8-3-6: the
        // trigger fires as soon as its conditions are met and waits as a
        // pending activation for the current processing to resolve; nothing
        // licenses discarding it. DCGO agrees â€” every deletion goes through
        // `CardEffectCommons.DeletePeremanentAndProcessAccordingToResult`,
        // which raises `OnDestroyedAnyone` regardless of seating order.
        //
        // So: handle equality alone is not identity once the event permanent
        // is gone. When the trigger carries a `deleted_object` snapshot and
        // THIS effect's own carrier is still standing at `source_permanent`
        // (it still holds `rctx.source_card`), the equality is the compaction
        // alias, not the observer watching itself â€” the deleted permanent was
        // someone else. Every other case keeps the plain handle comparison:
        // an observer whose carrier is itself the deleted permanent finds no
        // live carrier at that slot and still reads `is_source == true`.
        let is_source = event_permanent == source_permanent
            && !(trigger.deleted_object.is_some()
                && permanent_for_handle(rctx, source_permanent).is_some_and(|perm| {
                    perm.card_sources
                        .iter()
                        .any(|card| card.handle() == rctx.source_card)
                }));
        if is_source != want {
            return false;
        }
    }
    if let Some(want) = pred.source_deleted_battle_opponent {
        let actual = source_deleted_battle_opponent(rctx);
        if actual != want {
            return false;
        }
    }
    if let Some(want) = pred.event_host_permanent_is_source {
        let Some(trigger) = rctx.game.current_trigger_context.as_ref() else {
            return false;
        };
        let Some(event_host_permanent) = trigger.event_host_permanent else {
            return false;
        };
        let Some(source_permanent) = rctx.source_permanent else {
            return false;
        };
        if (event_host_permanent == source_permanent) != want {
            return false;
        }
    }
    if let Some(want) = pred.event_host_is_own_tamer {
        // G-SHARED-OPT-HETEROGENEOUS-TIMING: the trash event's host permanent is
        // a Tamer owned by the observer ("effects trash cards from under YOUR
        // Tamers"). The host handle's `player` is its controller; its top card's
        // kind must be Tamer.
        let host = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|t| t.event_host_permanent);
        let is_own_tamer = host
            .filter(|h| h.player == rctx.player)
            .and_then(|h| rctx.game.player(h.player).battle_area.get(h.index as usize))
            .map(|perm| perm.top_card().card_kind(rctx.card_data()) == CardKind::Tamer)
            .unwrap_or(false);
        if is_own_tamer != want {
            return false;
        }
    }
    if let Some(want) = pred.event_host_is_own_digimon {
        // G-DSL-ON-LINK-CARD-TRASHED-DELAY: the trash event's host is a
        // Digimon owned by the observer that still stands in the battle area.
        // Located by the host's TOP CARD (`event_host_card`), not its index:
        // the battle area compacts when a <Delay> carrier trashes itself.
        // When the host IS the event permanent (`OnTopCardTrashed`), it is
        // located by its stack identity instead: a LATER event can trash the
        // top card this event exposed (two separate <De-Digivolve 1>
        // instances, §16-12-7), and the earlier event's trigger is still met
        // on the same Digimon (§15-4-2-4; owner ruling 2026-10-10 #5).
        // G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS.
        let trigger = rctx.game.current_trigger_context.as_ref();
        let host_by_identity = trigger
            .filter(|t| {
                t.event_permanent_identity.is_some()
                    && t.event_host_permanent.is_some()
                    && t.event_host_permanent == t.event_permanent
            })
            .map(|t| {
                t.resolve_event_permanent(rctx.game)
                    .filter(|h| h.player == rctx.player)
                    .map(|h| h.index as usize)
            });
        let host_card = trigger.and_then(|t| t.event_host_card);
        let host_index = match host_by_identity {
            Some(index) => index,
            None => host_card.and_then(|card| {
                rctx.game
                    .player(rctx.player)
                    .battle_area
                    .iter()
                    .position(|perm| perm.top_card().handle() == card)
            }),
        };
        let is_own_digimon = host_index
            .map(|index| {
                rctx.game
                    .permanent_is_digimon_for_rules(crate::permanent::PermanentHandle {
                        player: rctx.player,
                        index: index as u8,
                    })
            })
            .unwrap_or(false);
        if is_own_digimon != want {
            return false;
        }
    }
    if let Some(want) = pred.is_event_host {
        // Subject permanent IS the event host ("1 of those Digimon").
        let host_card = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|t| t.event_host_card);
        let actual = match (subject, host_card) {
            (PredicateSubject::Permanent(handle), Some(card)) => rctx
                .game
                .player(handle.player)
                .battle_area
                .get(handle.index as usize)
                .map(|perm| perm.top_card().handle() == card)
                .unwrap_or(false),
            _ => false,
        };
        if actual != want {
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
    if let Some(want) = pred.event_card_kind {
        let Some(data) = rctx
            .event_card()
            .and_then(|card| rctx.game.card_data_for_handle(card))
        else {
            return false;
        };
        if !kind_matches_card_search(want, data) {
            return false;
        }
    }
    if let Some(ref want) = pred.event_card_name_is {
        if !rctx.event_card_name_matches(|name| name == want.as_str()) {
            return false;
        }
    }
    if let Some(ref needle) = pred.event_card_name_contains {
        if !rctx.event_card_name_contains(needle) {
            return false;
        }
    }
    if let Some(ref needle) = pred.event_card_text_contains {
        if !rctx.event_card_text_contains(needle) {
            return false;
        }
    }
    if let Some(ref needle) = pred.event_card_in_text_contains {
        // G-DSL-EVENT-CARD-IN-TEXT-CONTAINS â€” the broad whole-card scan
        // (`card_in_text_contains`, DCGO `CardSource.HasText`) applied to the
        // triggering event card. Resolves the event card's CardData exactly
        // like `event_card_trait_has` above.
        let matched = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.event_card)
            .and_then(|card| rctx.game.card_data_for_handle(card))
            .is_some_and(|data| card_in_text_contains(data, needle));
        if !matched {
            return false;
        }
    }
    if let Some(want) = pred.event_card_level_eq {
        let Some(level) = event_card_level(rctx) else {
            return false;
        };
        if level != want {
            return false;
        }
    }
    if let Some(floor) = &pred.event_card_level_gte {
        let Some(level) = event_card_level(rctx) else {
            return false;
        };
        let floor = eval_int_constraint(floor, rctx, None, None);
        if i32::from(level) < floor {
            return false;
        }
    }
    if let Some(ref allowed) = pred.event_card_color_only {
        // Every color of the triggering event card must appear in `allowed`.
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
        if data
            .colors
            .iter()
            .any(|c| !allowed.iter().any(|a| color_matches(*a, *c)))
        {
            return false;
        }
    }
    if let Some(ref allowed) = pred.event_card_color_has {
        // G-EVENT-CARD-COLOR-IS â€” at least one color of the triggering
        // event card must appear in `allowed` (intersection / "has"
        // semantics). Distinct from `event_card_color_only`'s subset test.
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
            .colors
            .iter()
            .any(|c| allowed.iter().any(|a| color_matches(*a, *c)))
        {
            return false;
        }
    }
    if let Some(want_count) = pred.event_card_color_count {
        // The triggering event card must have exactly `want_count` distinct colors.
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
        if distinct_color_count(&data.colors) as u8 != want_count {
            return false;
        }
    }
    if let Some(want) = pred.event_target_same_level_as_previous {
        if rctx
            .game
            .current_trigger_context
            .as_ref()
            .is_some_and(|trigger| {
                trigger.event_permanent_identity.is_some()
                    && trigger.resolve_event_permanent(rctx.game).is_none()
            })
        {
            return false;
        }
        let actual = event_target_same_level_as_previous(rctx).unwrap_or(false);
        if actual != want {
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
        // "a [X] trait Digimon's digivolution cards": the host's traits are
        // its top card's (with trait-changing effects), never another
        // digivolution card's (§4-7-7; DCGO `EX8_047.cs`
        // `trashedPermanent.TopCard.ContainsTraits`).
        // G-ENGINE-BATTLE-WINNER-TRAIT-READS-DIGIVOLUTION-CARDS.
        let live_host_matches = rctx
            .event_host_permanent()
            .and_then(|host| {
                permanent_for_handle(rctx, host).map(|perm| {
                    perm.has_trait_for_rules(
                        trait_name,
                        rctx.card_data(),
                        &rctx.game.modifiers,
                        host,
                    )
                })
            })
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

/// Evaluate historical identity without borrowing the former live slot. The
/// supported leaves are immutable identity/orientation fields and turn scope;
/// live-only predicates fail closed instead of consulting a replacement body.
fn eval_battle_deleter_snapshot(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    snapshot: &crate::trigger_context::BattleCombatantSnapshot,
) -> bool {
    let Some(data) = rctx.game.card_data.get(snapshot.data_index) else {
        return false;
    };
    if !pred
        .all_of
        .iter()
        .all(|p| eval_battle_deleter_snapshot(p, rctx, snapshot))
        || (!pred.any_of.is_empty()
            && !pred
                .any_of
                .iter()
                .any(|p| eval_battle_deleter_snapshot(p, rctx, snapshot)))
        || pred
            .none_of
            .iter()
            .any(|p| eval_battle_deleter_snapshot(p, rctx, snapshot))
        || pred
            .not
            .as_ref()
            .is_some_and(|p| eval_battle_deleter_snapshot(p, rctx, snapshot))
    {
        return false;
    }
    let mut remaining = pred.clone();
    remaining.all_of.clear();
    remaining.any_of.clear();
    remaining.none_of.clear();
    remaining.not = None;
    if let Some(owner) = remaining.owner.take() {
        if !player_ref_matches(owner, snapshot.controller, rctx) {
            return false;
        }
    }
    if let Some(kind) = remaining.kind.take() {
        if !kind_matches_membership(kind, snapshot.identity.kind, snapshot.identity.retained_kind) {
            return false;
        }
    }
    let names: Vec<&str> = snapshot
        .identity
        .card_names
        .iter()
        .map(String::as_str)
        .collect();
    if let Some(name) = remaining.name_is.take() {
        if !names.iter().any(|n| *n == name) {
            return false;
        }
    }
    if let Some(needle) = remaining.name_contains.take() {
        if !names
            .iter()
            .any(|n| crate::card_data::rules_name_contains(n, &needle))
        {
            return false;
        }
    }
    if let Some(wanted) = remaining.name_in.take() {
        if !names.iter().any(|n| wanted.iter().any(|want| *n == want)) {
            return false;
        }
    }
    if let Some(needle) = remaining.in_text_contains.take() {
        if !card_in_text_contains(data, &needle)
            && !snapshot
                .identity
                .card_names
                .iter()
                .chain(snapshot.identity.traits.iter())
                .any(|value| value.to_lowercase().contains(&needle.to_lowercase()))
        {
            return false;
        }
    }
    if let Some(needle) = remaining.effect_text_contains.take() {
        let needle = needle.to_lowercase();
        if ![&data.effect_text, &data.inherited_text, &data.security_text]
            .iter()
            .any(|text| text.to_lowercase().contains(&needle))
        {
            return false;
        }
    }
    if let Some(number) = remaining.card_number_is.take() {
        if data.card_id != number {
            return false;
        }
    }
    for wanted in [
        remaining.trait_has.take(),
        remaining.form_is.take(),
        remaining.attribute_is.take(),
    ]
    .into_iter()
    .flatten()
    {
        if !snapshot
            .identity
            .traits
            .iter()
            .chain(data.traits.iter())
            .any(|value| value.eq_ignore_ascii_case(&wanted))
        {
            return false;
        }
    }
    if let Some(needle) = remaining.trait_contains.take() {
        if !snapshot
            .identity
            .traits
            .iter()
            .chain(data.traits.iter())
            .any(|value| value.to_lowercase().contains(&needle.to_lowercase()))
        {
            return false;
        }
    }
    if let Some(wanted) = remaining.color_is.take() {
        if !snapshot
            .identity
            .colors
            .iter()
            .any(|c| color_matches(wanted, *c))
        {
            return false;
        }
    }
    if let Some(wanted) = remaining.color_only.take() {
        if snapshot.identity.colors.is_empty()
            || !snapshot
                .identity
                .colors
                .iter()
                .all(|c| wanted.iter().any(|want| color_matches(*want, *c)))
        {
            return false;
        }
    }
    if let Some(level) = remaining.level_eq.take() {
        if snapshot.identity.level != Some(level) {
            return false;
        }
    }
    if let Some(wanted) = remaining.is_suspended.take() {
        if snapshot.is_suspended != wanted {
            return false;
        }
    }
    if let Some(wanted) = remaining.is_unsuspended.take() {
        if snapshot.is_suspended == wanted {
            return false;
        }
    }
    if !remaining.zone.is_empty() && !remaining.zone.contains(&CompiledZone::BattleArea) {
        return false;
    }
    remaining.zone.clear();
    // Event-time identity is independent of current turn, but turn-scope
    // leaves still use the observer's controller just like ordinary predicates.
    let scope = CompiledPredicate {
        your_turn: remaining.your_turn.take(),
        opponents_turn: remaining.opponents_turn.take(),
        all_turns: remaining.all_turns.take(),
        ..CompiledPredicate::default()
    };
    remaining == CompiledPredicate::default()
        && eval_predicate(&scope, rctx, PredicateSubject::None)
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
        PredicateSubject::Card(_)
        | PredicateSubject::RevealedCard(_)
        | PredicateSubject::Source(_)
        | PredicateSubject::None => rctx.source_permanent(),
    }
}

fn permanent_for_handle<'a>(
    rctx: &'a EffectReadContext<'_>,
    handle: PermanentHandle,
) -> Option<&'a crate::permanent::Permanent> {
    if handle.index == crate::action::space::BREEDING_TARGET as u8 {
        return rctx.game.player(handle.player).breeding_area.as_ref();
    }
    // A would-leave event can retain its physical DigiXros material in the
    // engine's limbo holding area while an interruptive effect is chosen.
    // Revalidation must read that same permanent, never a shifted field slot.
    if crate::digixros::is_limbo_index(handle.index) {
        return rctx
            .game
            .digixros_leaving_limbo
            .get((handle.index - crate::digixros::LIMBO_INDEX_BASE) as usize)
            .filter(|(owner, _, _)| *owner == handle.player)
            .map(|(_, _, p)| p);
    }
    rctx.game
        .player(handle.player)
        .battle_area
        .get(handle.index as usize)
}

fn source_deleted_battle_opponent(rctx: &EffectReadContext<'_>) -> bool {
    let Some(source) = rctx.source_permanent else {
        return false;
    };
    let Some(source_perm) = permanent_for_handle(rctx, source) else {
        return false;
    };
    if !source_perm
        .card_sources
        .iter()
        .any(|card| card.handle() == rctx.source_card)
    {
        return false;
    }
    let Some(trigger) = rctx.game.current_trigger_context.as_ref() else {
        return false;
    };
    if trigger.cause != Some(crate::trigger_context::EventCause::BattleDeletion) {
        return false;
    }
    // The battle's own record first: the deleted opponent has left the battle
    // area, so it has no live handle to compare (§11-2-6: a removed attack
    // target is never read through its old slot).
    if let Some(deleted) = trigger.deleted_object.as_ref() {
        if let Some(was) = rctx.game.battle_record_opponent_was(source, deleted.top_card) {
            return was;
        }
    }
    let Some(event_permanent) = trigger.event_permanent.or(trigger.target_permanent) else {
        return false;
    };
    let Some(opponent) = rctx.battle_opponent_of(source) else {
        return false;
    };
    event_permanent == opponent
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
    if trigger.event_permanent_identity.is_some() {
        let handle = trigger.resolve_event_permanent(rctx.game)?;
        return permanent_for_handle(rctx, handle)?
            .card_sources
            .last()
            .map(|card| card.handle());
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

fn event_card_level(rctx: &EffectReadContext<'_>) -> Option<u8> {
    // Event-card predicates describe the captured physical card, while
    // event-target predicates describe its surviving Digimon's current top.
    let card = rctx
        .game
        .current_trigger_context
        .as_ref()
        .and_then(|trigger| {
            trigger
                .deleted_object
                .as_ref()
                .map(|snapshot| snapshot.top_card)
                .or(trigger.event_card)
        })
        .or_else(|| event_target_card(rctx))?;
    rctx.game
        .card_data_for_handle(card)
        .and_then(|data| data.level)
}

fn event_target_level(rctx: &EffectReadContext<'_>) -> Option<u8> {
    let card = event_target_card(rctx)?;
    rctx.game
        .card_data_for_handle(card)
        .and_then(|data| data.level)
}

/// Total digivolution-stack size (top card + sources) of the event-target
/// permanent. For a deletion event the field slot is already gone, so read the
/// rule-25 deletion snapshot's `source_count_just_before` (sources BELOW the
/// top) and add 1 for the top card. For a live event target, read the current
/// `card_sources` length (already inclusive of the top card). Returns `None`
/// when no event target resolves. G-DSL-EVENT-TARGET-SOURCE-COUNT.
fn event_target_stack_size(rctx: &EffectReadContext<'_>) -> Option<usize> {
    let trigger = rctx.game.current_trigger_context.as_ref()?;
    if let Some(snapshot) = trigger.deleted_object.as_ref() {
        return Some(snapshot.source_count_just_before + 1);
    }
    let handle = if trigger.event_permanent_identity.is_some() {
        trigger.resolve_event_permanent(rctx.game)?
    } else {
        trigger
            .event_permanent
            .or(trigger.target_permanent)
            .or(trigger.event_host_permanent)?
    };
    let perm = permanent_for_handle(rctx, handle)?;
    Some(perm.card_sources.len())
}

/// Resolve the reference play cost for `play_cost_eq_binding`. Order:
/// 1. A named card binding â†’ its printed play cost.
/// 2. A named permanent binding â†’ its top card's play cost.
/// 3. The special `event_target` handle â†’ the deletion snapshot's
///    pre-removal cost (`cost_just_before`) for a deletion event, else the
///    live event-target card's play cost.
///
/// Returns `None` when nothing resolves, so the caller fails closed rather
/// than silently comparing against 0. G-DSL-COST-RELATIVE-TO-EVENT-SUBJECT.
fn reference_play_cost(
    binding: &str,
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> Option<i32> {
    if let Some(bindings) = bindings {
        if let Some(card) = bindings.get_card(binding) {
            return rctx
                .game
                .card_data_for_handle(card)
                .map(|data| i32::from(data.play_cost));
        }
        if let Some(handle) = bindings.get_permanent(binding) {
            return permanent_for_handle(rctx, handle)
                .and_then(|perm| rctx.game.card_data_for_handle(perm.top_card().handle()))
                .map(|data| i32::from(data.play_cost));
        }
    }
    if binding == "event_target" {
        if let Some(snapshot) = rctx
            .game
            .current_trigger_context
            .as_ref()
            .and_then(|trigger| trigger.deleted_object.as_ref())
        {
            return snapshot.cost_just_before.map(i32::from);
        }
        let card = event_target_card(rctx)?;
        return rctx
            .game
            .card_data_for_handle(card)
            .map(|data| i32::from(data.play_cost));
    }
    None
}

fn event_target_dp(rctx: &EffectReadContext<'_>) -> Option<i32> {
    let trigger = rctx.game.current_trigger_context.as_ref()?;
    if let Some(snapshot) = trigger.deleted_object.as_ref() {
        return snapshot.dp_just_before;
    }
    if trigger.event_permanent_identity.is_some() {
        return rctx
            .game
            .effective_dp(trigger.resolve_event_permanent(rctx.game)?);
    }
    if let Some(handle) = trigger
        .event_permanent
        .or(trigger.event_host_permanent)
        .or(trigger.target_permanent)
    {
        return rctx.game.effective_dp(handle);
    }
    if let Some(change) = trigger.attack_target_change.as_ref() {
        if let AttackTarget::Digimon(handle) = change.new_target {
            return rctx.game.effective_dp(handle);
        }
    }
    None
}

fn event_target_same_level_as_previous(rctx: &EffectReadContext<'_>) -> Option<bool> {
    let trigger = rctx.game.current_trigger_context.as_ref()?;
    // Last-known information (a deletion snapshot, or a clause gate's view of
    // a departed event Digimon): its top card against the card directly
    // under it (`digisources_just_before` is bottom-most first).
    if let Some(snapshot) = trigger.deleted_object.as_ref() {
        let top_level = rctx.game.card_data_for_handle(snapshot.top_card)?.level?;
        let previous = *snapshot.digisources_just_before.last()?;
        let previous_level = rctx.game.card_data_for_handle(previous)?.level?;
        return Some(top_level == previous_level);
    }
    let handle = if trigger.event_permanent_identity.is_some() {
        trigger.resolve_event_permanent(rctx.game)?
    } else {
        trigger.event_permanent.or(trigger.target_permanent)?
    };
    let permanent = permanent_for_handle(rctx, handle)?;
    let top = permanent.card_sources.last()?;
    let previous = permanent.card_sources.iter().rev().nth(1)?;
    let top_level = rctx.game.card_data[top.data_index].level?;
    let previous_level = rctx.game.card_data[previous.data_index].level?;
    Some(top_level == previous_level)
}

/// True when any of a candidate card's effective names satisfies `matches`.
/// The effective names are the printed `card_name`, an optional
/// reveal-overlay name, and every static "also treated as" identity alias.
/// Shared by the `name_is` / `name_contains` / `name_in` leaves so a new
/// name source only has to be wired in one place.
fn any_effective_name_matches(
    printed: &str,
    overlay_name: Option<&str>,
    aliases: &[String],
    matches: impl Fn(&str) -> bool,
) -> bool {
    matches(printed)
        || overlay_name.is_some_and(&matches)
        || aliases.iter().any(|alias| matches(alias))
}

/// Whole-card "[X] in its text" scan (G-DSL-IN-TEXT-CONTAINS) â€” the broad DCGO
/// `CardSource.HasText` surface (`CardSource.cs`). Case-insensitive substring.
///
/// The scanned surface, per the official ruling reproduced on BT21-098's Q&A
/// ("name, traits, effects, inherited effects, (Rule), digivolution
/// requirements, DNA digivolution, DigiXros requirements, burst digivolve, App
/// Fusion, Link, or Assembly requirements"):
///
/// - **name** (`card_name`) and every static **also-treated-as** alias
///   (`ChangeCardNamesClass` / "(Rule) also treated as [X]") and **DigiXros
///   alias** (`digixros_aliases`) â€” DCGO scans only `CardName_ENG`, but our
///   engine records the extra printed identities the API drops, so scanning
///   them is a faithful superset.
/// - **traits** (`traits`) â€” the whole trait line (form, attribute and type;
///   DCGO scans `Attribute_ENG` and `Type_ENG`), INCLUDING the Rule-granted
///   traits the digimoncard.io ingest drops (e.g. the [Three Musketeers] trait
///   on BT6-017 / BT6-065 / ST14-09, which do NOT carry the literal string in
///   their effect text). This is the load-bearing difference from
///   `effect_text_contains`.
/// - **all printed text**: effect / inherited / security, and â€” for a Dual â€”
///   both faces (`text_for_search_all_faces`). Separately stored printed
///   requirement wording is retained in `requirement_text` by the loader.
/// Numeric `evo_costs` and `dna_costs` are not text and are not searched.
fn card_in_text_contains(data: &crate::card_data::CardData, needle: &str) -> bool {
    data.in_text_contains(needle)
}

/// Card-subject sibling including effect-granted physical identity aliases.
/// Printed-data-only callers retain the original helper and never infer live state.
fn any_physical_card_name_matches(
    printed: &str,
    overlay_name: Option<&str>,
    aliases: &[String],
    runtime_names: &[String],
    matches: impl Fn(&str) -> bool,
) -> bool {
    any_effective_name_matches(printed, overlay_name, aliases, &matches)
        || runtime_names.iter().any(|name| matches(name))
}

fn eval_card_fields(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    card: CardHandle,
    reveal_overlay_visible: bool,
    formula_target: Option<PermanentHandle>,
    bindings: Option<&Bindings>,
    dual_ref: DualRef,
) -> bool {
    let data = match rctx.game.card_data_for_handle(card) {
        Some(d) => d,
        None => return false,
    };
    // §4-6-3: a DUAL card placed on the field -- the TOP card of a battle-area
    // or breeding-area permanent, i.e. the permanent itself -- is treated as a
    // Digimon and can't be referenced as an Option card, whatever the caller
    // asked for. A DUAL card lying in a permanent's digivolution or link cards
    // keeps the caller's reference: as an Option card it reads its Option
    // information (§4-6-6-2; official Q&A Q6395, Q6404).
    let top_card_dual = data.dual.is_some() && rctx.game.card_is_permanent_top_card(card);
    let dual_ref = if top_card_dual {
        DualRef::Digimon
    } else {
        dual_ref
    };
    // §4-6-5-2 / §4-6-6-2: a DUAL referenced as an Option card has only its
    // Option information (color, use cost; no level, no DP); otherwise the
    // Digimon information (the top-level printed values) applies.
    let option_ref = data.dual.is_some() && dual_ref == DualRef::Option;
    let ref_level = if option_ref { None } else { data.level };
    let ref_dp = if option_ref { None } else { data.dp };
    let ref_play_cost: i32 = match (&data.dual, option_ref) {
        (Some(d), true) => effective_use_cost(rctx, card).unwrap_or(i32::from(d.option.use_cost)),
        _ => i32::from(data.play_cost),
    };
    let ref_colors: &[CardColor] = match (&data.dual, option_ref) {
        (Some(d), true) => d.option.colors.as_slice(),
        (Some(d), false) => d.digimon.colors.as_slice(),
        (None, _) => data.colors.as_slice(),
    };
    if let Some(want) = pred.can_play_to_breeding_from_hand {
        let can = rctx
            .game
            .card_source_for_handle(card)
            .is_some_and(|source| {
                let owner = source.owner;
                rctx.game
                    .players
                    .get(owner as usize)
                    .and_then(|player| {
                        player
                            .hand
                            .iter()
                            .position(|candidate| candidate.handle() == card)
                    })
                    .is_some_and(|index| {
                        rctx.game
                            .can_play_to_breeding_from_hand(owner, index, rctx.player)
                    })
            });
        if can != want {
            return false;
        }
    }
    if let Some(want) = pred.can_play_from_effect_without_cost {
        let can = rctx
            .game
            .card_source_for_handle(card)
            .is_some_and(|source| {
                rctx.game
                    .can_play_card_from_effect_without_cost_for_controller(
                        source.owner,
                        card,
                        1,
                        rctx.player,
                    )
            });
        if can != want {
            return false;
        }
    }
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
        if top_card_dual && matches!(want, CompiledCardKind::Option) {
            return false;
        }
        let overlay_match = overlay
            .and_then(|o| o.kind)
            .is_some_and(|kind| kind_matches(want, kind));
        if !kind_matches_card_search(want, data) && !overlay_match {
            return false;
        }
    }
    if let Some(want) = pred.level_eq {
        if ref_level != Some(want) {
            return false;
        }
    }
    if let Some(ref binding) = pred.level_eq_binding {
        let Some(want) = bindings.and_then(|b| b.get_literal(binding)) else {
            return false;
        };
        if ref_level.map(i64::from) != Some(want) {
            return false;
        }
    }
    if let Some(ref binding) = pred.name_not_in_binding {
        // BT26-086 â€” "with different names": reject a card that shares any
        // card name with a card already captured in the CardList binding.
        // Absent binding â‡’ nothing chosen yet â‡’ pass.
        if let Some(list) = bindings.and_then(|b| b.get_card_list(binding)) {
            let names = |d: &crate::card_data::CardData| {
                let mut v = vec![d.card_name.clone()];
                v.extend(d.also_treated_as.iter().cloned());
                v
            };
            let mine = names(data);
            let clash = list.iter().filter(|h| **h != card).any(|h| {
                rctx.game
                    .card_data_for_handle(*h)
                    .is_some_and(|other| names(other).iter().any(|n| mine.contains(n)))
            });
            if clash {
                return false;
            }
        }
    }
    if let Some(ref binding) = pred.level_lte_binding {
        // Subject level <= the literal bound to `binding`. Sibling of
        // `level_eq_binding`. Driver: BT8-107 ("level <= the deleted
        // Digimon's level"). Fails closed when the binding or level is absent.
        let Some(cap) = bindings.and_then(|b| b.get_literal(binding)) else {
            return false;
        };
        let Some(level) = ref_level else {
            return false;
        };
        if i64::from(level) > cap {
            return false;
        }
    }
    if let Some(ref binding) = pred.level_gte_binding {
        let Some(floor) = bindings.and_then(|b| b.get_literal(binding)) else {
            return false;
        };
        let Some(level) = ref_level else {
            return false;
        };
        if i64::from(level) < floor {
            return false;
        }
    }
    if let Some(cap) = &pred.level_lte {
        let cap = eval_int_constraint(cap, rctx, formula_target, bindings);
        if ref_level.map_or(true, |l| i32::from(l) > cap) {
            return false;
        }
    }
    if let Some(floor) = &pred.level_gte {
        let floor = eval_int_constraint(floor, rctx, formula_target, bindings);
        if ref_level.map_or(true, |l| i32::from(l) < floor) {
            return false;
        }
    }
    if let Some(want) = pred.color_is {
        if !ref_colors.iter().any(|c| color_matches(want, *c)) {
            return false;
        }
    }
    if let Some(ref allowed) = pred.color_only {
        for c in ref_colors {
            if !allowed.iter().any(|a| color_matches(*a, *c)) {
                return false;
            }
        }
    }
    if let Some(of) = pred.color_matches_any_field_digimon {
        if !card_shares_color_with_any_field_digimon(rctx, of, ref_colors) {
            return false;
        }
    }
    if let Some(ref binding) = pred.color_matches_binding {
        if !card_shares_color_with_bound_permanent(rctx, bindings, binding, ref_colors) {
            return false;
        }
    }
    if pred.color_matches_returned_card == Some(true)
        && !card_shares_color_with_returned_card(rctx, bindings, ref_colors)
    {
        return false;
    }
    if pred.color_matches_own_source_stack.is_some()
        && !card_shares_color_with_own_source_stack(rctx, ref_colors)
    {
        return false;
    }
    if let Some(ref t) = pred.trait_has {
        if !data.traits.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            return false;
        }
    }
    if let Some(ref t) = pred.trait_contains {
        let needle = t.to_lowercase();
        if !data
            .traits
            .iter()
            .any(|x| x.to_lowercase().contains(&needle))
        {
            return false;
        }
    }
    if let Some(floor) = pred.self_color_count_gte {
        if distinct_color_count(ref_colors) < usize::from(floor) {
            return false;
        }
    }
    if let Some(cap) = pred.self_color_count_lte {
        if distinct_color_count(ref_colors) > usize::from(cap) {
            return false;
        }
    }
    // A form or an attribute is a trait (general_rule.pdf 2-3-2-1), printed
    // text names either only as "the [X] trait", and `traits` is the whole
    // trait line (form + attribute + type) â€” so these match like `trait_has`.
    for want in [&pred.form_is, &pred.attribute_is].into_iter().flatten() {
        if !data.traits.iter().any(|x| x.eq_ignore_ascii_case(want)) {
            return false;
        }
    }
    // Physical runtime aliases are separate from printed data. Source/hand/
    // trash selectors must see the same identity as CardSource::card_names.
    let runtime_names =
        if pred.name_is.is_some() || pred.name_contains.is_some() || pred.name_in.is_some() {
            rctx.game
                .card_source_for_handle(card)
                .map(|source| source.also_treated_as.as_slice())
                .unwrap_or(&[])
        } else {
            &[]
        };
    // `name_is` / `name_contains` / `name_in` all match against the printed
    // name, an optional reveal-overlay name, and static "also treated as"
    // identity aliases â€” see `any_effective_name_matches`.
    if let Some(ref n) = pred.name_is {
        let overlay_name = overlay.and_then(|o| o.name.as_deref());
        if !any_physical_card_name_matches(
            &data.card_name,
            overlay_name,
            &data.also_treated_as,
            runtime_names,
            |name| name == n.as_str(),
        ) {
            return false;
        }
    }
    if let Some(ref n) = pred.name_contains {
        let needle = n.to_lowercase();
        let overlay_name = overlay.and_then(|o| o.name.as_deref());
        if !any_physical_card_name_matches(
            &data.card_name,
            overlay_name,
            &data.also_treated_as,
            runtime_names,
            |name| crate::card_data::rules_name_contains(name, &needle),
        ) {
            return false;
        }
    }
    if let Some(ref keyword) = pred.printed_keyword {
        // G-DSL-PREDICATE-PRINTED-KEYWORD: the candidate's face-up printed
        // keyword line (effect text only) â€” DCGO `CardSource.HasBlocker`
        // excludes inherited and security effects.
        let Some(kw) = lookup_keyword(keyword, None) else {
            return false;
        };
        // Name-only reference: matches every value of the keyword (§16-2).
        if !crate::card_data::parse_printed_keywords(&data.effect_text, "", "")
            .iter()
            .any(|k| k.same_type(&kw))
        {
            return false;
        }
    }
    if let Some(ref n) = pred.effect_text_contains {
        // G-DSL-PREDICATE-TEXT-CONTAINS: case-insensitive substring scan
        // against the candidate card's printed text â€” `effect_text`,
        // `inherited_text`, and `security_text`. DCGO `source.HasText(s)`.
        let needle = n.to_lowercase();
        let text_match = data.effect_text.to_lowercase().contains(&needle)
            || data.inherited_text.to_lowercase().contains(&needle)
            || data.security_text.to_lowercase().contains(&needle);
        if !text_match {
            return false;
        }
    }
    if let Some(ref n) = pred.in_text_contains {
        // G-DSL-IN-TEXT-CONTAINS: the broad whole-card "[X] in its text"
        // scan mirroring DCGO `CardSource.HasText` (`CardSource.cs`). Scans
        // the card NAME + also-treated-as / DigiXros aliases + TRAITS (the
        // Type line â€” this is where the [Three Musketeers]-trait regression
        // lives) + all printed text (effect / inherited / security, both
        // dual faces). Case-insensitive. This is the official-ruling surface
        // ("name, traits, effects, inherited effects, (Rule), digivolution
        // requirements, DNA digivolution, DigiXros requirements, burst
        // digivolve, App Fusion, Link, or Assembly requirements"): the
        // structured requirement strings the CardData model carries are
        // embedded in the printed text scanned here, and traits are scanned
        // directly. See `card_in_text_contains`.
        if !card_in_text_contains(data, n) {
            return false;
        }
    }
    if let Some(ref names) = pred.name_in {
        let overlay_name = overlay.and_then(|o| o.name.as_deref());
        if !any_physical_card_name_matches(
            &data.card_name,
            overlay_name,
            &data.also_treated_as,
            runtime_names,
            |name| names.iter().any(|n| n.as_str() == name),
        ) {
            return false;
        }
    }
    if let Some(of) = pred.name_not_shared_by_field_digimon {
        // The candidate card's effective name. NOTE: this is a deliberate
        // divergence from the sibling `name_is` / `name_contains` / `name_in`
        // leaves above. Those treat an overlay name as an *alternative* match
        // (`printed_match || overlay_match`) â€” a card can satisfy them via
        // either its printed name or its overlaid name. This leaf instead uses
        // the overlay name as a *replacement* for the printed name
        // (`overlay.name.unwrap_or(data.card_name)`). Replacement is correct
        // for exclusion semantics: a name-changed card's *effective / current*
        // name is the overlay name, and that single effective name is what
        // must be compared against field Digimon to decide exclusion. ORing in
        // the printed name would wrongly exclude on a name the card no longer
        // has.
        let candidate_name = overlay
            .and_then(|o| o.name.as_deref())
            .unwrap_or(data.card_name.as_str());
        if field_digimon_has_name(rctx, of, candidate_name) {
            return false;
        }
    }
    if let Some(of) = pred.name_not_shared_by_field_tamer {
        let candidate_name = overlay
            .and_then(|o| o.name.as_deref())
            .unwrap_or(data.card_name.as_str());
        if field_tamer_has_name(rctx, of, candidate_name) {
            return false;
        }
    }
    if let Some(ref cn) = pred.card_number_is {
        if data.card_id != *cn {
            return false;
        }
    }
    if let Some(cap) = &pred.play_cost_lte {
        if ref_play_cost > eval_int_constraint(cap, rctx, formula_target, bindings) {
            return false;
        }
    }
    if let Some(floor) = &pred.play_cost_gte {
        if ref_play_cost < eval_int_constraint(floor, rctx, formula_target, bindings) {
            return false;
        }
    }
    if let Some(want) = &pred.play_cost_eq {
        if ref_play_cost != eval_int_constraint(want, rctx, formula_target, bindings) {
            return false;
        }
    }
    if let Some(spec) = &pred.play_cost_eq_binding {
        // G-DSL-COST-RELATIVE-TO-EVENT-SUBJECT â€” compare the candidate's play
        // cost to a bound/event permanent's play cost plus an offset. Fails
        // closed when the reference cost cannot be resolved (binding unset).
        // Driver: BT19-099 ("play cost 1 higher than that [leaving] Digimon").
        let Some(reference) = reference_play_cost(&spec.binding, rctx, bindings) else {
            return false;
        };
        let threshold = reference + i32::from(spec.offset);
        let candidate = ref_play_cost;
        let ok = match spec.op {
            digimon_dsl::compiled::CompiledComparatorOp::Eq => candidate == threshold,
            digimon_dsl::compiled::CompiledComparatorOp::Gte => candidate >= threshold,
            digimon_dsl::compiled::CompiledComparatorOp::Lte => candidate <= threshold,
        };
        if !ok {
            return false;
        }
    }
    if let Some(cap) = &pred.play_or_use_cost_lte {
        // G-PLAY-OR-USE-COST-LTE: the larger of the candidate's play cost and
        // its Option-use cost must be at most the threshold. Mirrors DCGO
        // `CardSource.GetCostItself <= N` for "play or use 1 ... card with a
        // play or use cost of N or less". For a Digimon / Tamer
        // `option_use_cost()` is `None` so the bound is `play_cost`; for a pure
        // Option both coincide; for a Dual it is the max of the two faces.
        let play = i32::from(data.play_cost);
        // The effective use cost includes the card's own use-cost increase
        // (G-ENGINE-OPTION-SELF-USE-COST-INCREASE).
        let use_cost = if data.option_use_cost().is_some() {
            effective_use_cost(rctx, card).unwrap_or(play)
        } else {
            play
        };
        let cost = play.max(use_cost);
        if cost > eval_int_constraint(cap, rctx, formula_target, bindings) {
            return false;
        }
    }
    if let Some(reduce) = pred.affordable_with_cost_reduce {
        // BT26-084 â€” "play or use ... with the cost reduced by N": the
        // controller's own memory must stay at or above the floor after paying
        // max(0, cost - N). Option â†’ use cost; otherwise play cost.
        let cost = if data.card_kind == crate::enums::CardKind::Option {
            effective_use_cost(rctx, card)
                .map(|c| c as u16)
                .unwrap_or(data.option_use_cost().unwrap_or(data.play_cost))
        } else {
            data.play_cost
        };
        let pay = i32::from(cost.saturating_sub(u16::from(reduce)));
        let own = if rctx.game.turn_player() == rctx.player {
            i32::from(rctx.game.memory)
        } else {
            -i32::from(rctx.game.memory)
        };
        if own - pay < i32::from(rctx.game.rules.memory_range.0) {
            return false;
        }
    }
    // Card-subject DP filter. Cards in trash/hand/security carry their printed
    // DP via `CardData.dp`; permanents on the field route through
    // `eval_dp_constraints` against `effective_dp`. Options and other
    // non-Digimon cards have `dp = None` and cannot satisfy a DP constraint.
    if pred.dp_eq.is_some() || pred.dp_lte.is_some() || pred.dp_gte.is_some() {
        let Some(dp) = ref_dp else {
            return false;
        };
        let perm_target =
            formula_target
                .or(rctx.source_permanent)
                .unwrap_or(crate::permanent::PermanentHandle {
                    player: rctx.player,
                    index: 0,
                });
        if let Some(want) = &pred.dp_eq {
            if dp != eval_dp_constraint(want, rctx, perm_target, bindings) {
                return false;
            }
        }
        if let Some(cap) = &pred.dp_lte {
            if dp
                > eval_dp_constraint(cap, rctx, perm_target, bindings)
                    + deletion_cap_bonus(pred, rctx)
            {
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
    // `in_event_discarded_cards`: the candidate is one of the cards the
    // triggering effect just trashed from the hand ("play 1 of THEM" â€”
    // BT24-007 Tsunomon; DCGO `GetDiscardedCardsFromHashtable`).
    // G-ENGINE-DISCARDED-HAND-CARDS.
    if let Some(want) = pred.in_event_discarded_cards {
        let is_discarded = rctx
            .game
            .current_trigger_context
            .as_ref()
            .is_some_and(|trigger| trigger.discarded_hand_cards().contains(&card));
        if is_discarded != want {
            return false;
        }
    }
    // `can_digivolve_onto: <binding>` â€” the binding-targeted sibling of
    // `can_digivolve_from_source`: the candidate has â‰¥1 normal-digivolve
    // route onto the permanent bound under `binding` (DCGO
    // `CanPlayCardTargetFrame(selectedPermanent.PermanentFrame, â€¦)`).
    // Absent / non-permanent binding â†’ false (no target, no route).
    // G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST.
    if let Some(binding) = &pred.can_digivolve_onto {
        // `source` (when not shadowed by a real binding) is the effect's own
        // carrier permanent â€” "THIS Digimon may digivolve into ..." (EX13-055 /
        // EX13-057). G-DSL-CAN-DIGIVOLVE-ONTO-SOURCE.
        let target = bindings.and_then(|b| b.get_permanent(binding)).or_else(|| {
            if binding == "source" {
                rctx.source_permanent
            } else {
                None
            }
        });
        let Some(target) = target else {
            return false;
        };
        if !can_card_digivolve_onto(rctx, card, target) {
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
        // to recurse into, so the predicate fails â€” only permanents on
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

fn alt_path_kind_matches(kind: &digimon_dsl::compiled::CompiledAltPathKind, name: &str) -> bool {
    // Single source of truth for alt-path string keys lives on
    // `CompiledAltPathKind::as_key()` (see `digimon-dsl/src/compiled.rs`).
    // Reward profiles (`add-reward-profiles` change) reference these same
    // strings via the `play_named_card.via_alt_path` matcher.
    let normalized = name.to_ascii_lowercase().replace('-', "_");
    kind.as_key() == normalized
}

/// `can_digivolve_from_source: true` â€” the candidate card has at least one
/// normal-digivolve route onto the SOURCE permanent. Routes are enumerated by
/// the SAME machinery the effect-initiated digivolve commit path uses
/// (`Game::all_digivolve_routes_for_card`, App Fusion excluded â€” it is an
/// alt-PLAY mechanic the `effect_initiated_digivolve` step also skips): the
/// printed evo-cost circles, DSL `alt_paths: kind: digivolve` special circles
/// (e.g. BT25-005 Pagumon's "[TS] trait" targets whose only applicable circle
/// is "Lv.3 w/[TS] trait"), and the `CanOnlyDigivolveInto` restriction gate.
/// Mirrors DCGO `DigivolveIntoHandOrTrashCard`'s `CanSelectCardCondition`
/// (`cardSource.CanPlayCardTargetFrame(...)`) so the hand prompt offers
/// exactly the cards the commit would accept â€” a pick the commit would reject
/// is never surfaced. (Previously routed through `Game::can_digivolve`, which
/// scans printed `evo_costs` only and so hid alt-path-only candidates.)
fn can_card_digivolve_from_source(rctx: &EffectReadContext<'_>, card: CardHandle) -> bool {
    let Some(source_handle) = rctx.source_permanent else {
        return false;
    };
    can_card_digivolve_onto(rctx, card, source_handle)
}

/// Shared route probe behind `can_digivolve_from_source` (target = the
/// effect's source permanent) and `can_digivolve_onto` / `has_digivolve_candidate`
/// (target = an arbitrary bound permanent): true when `card` has at least one
/// non-App-Fusion normal-digivolve route onto `target` per
/// `Game::all_digivolve_routes_for_card` â€” printed evo circles, DSL
/// alt-digivolve paths, and the `CanOnlyDigivolveInto` gate, exactly what the
/// `effect_initiated_digivolve` commit path accepts. A target handle that no
/// longer addresses a permanent yields false.
fn can_card_digivolve_onto(
    rctx: &EffectReadContext<'_>,
    card: CardHandle,
    target: PermanentHandle,
) -> bool {
    if permanent_for_handle(rctx, target).is_none() {
        return false;
    }
    // This probe serves effect-driven evolution choices. Route compatibility
    // alone is insufficient: both the host and its controller must currently
    // be allowed to digivolve. Match the controller used by the DSL commit,
    // which can differ from the player owning the initiating effect.
    if rctx
        .game
        .modifiers
        .has(target, crate::enums::ModifierType::CannotDigivolve)
        || rctx.game.modifiers.player_has(
            target.player,
            crate::enums::ModifierType::CannotDigivolveDigimonByEffect,
        )
    {
        return false;
    }
    let Some(candidate) = rctx.game.card_source_for_handle(card) else {
        return false;
    };
    rctx.game
        .all_digivolve_routes_for_card(candidate, target)
        .iter()
        .any(|route| !route.app_fusion)
}

/// `has_digivolve_candidate` â€” permanent-subject probe: does any card in the
/// listed hand/trash zones of `of` satisfy `filter` AND have a normal-digivolve
/// route onto `handle`? DCGO BT25_092 `CanDigivolveDigimon(permanent)` =
/// `HasMatchConditionOwnersHand(ValidTarget(â€¦, Root.Hand, permanent)) ||
/// HasMatchConditionOwnersCardInTrash(ValidTarget(â€¦, Root.Trash, permanent))`.
/// The nested filter is evaluated as a card subject (same evaluator a
/// `select_union_zone` hand/trash pick uses), so the permanent gate and the
/// later result pick agree on the candidate set.
/// G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST.
fn permanent_has_digivolve_candidate(
    spec: &digimon_dsl::compiled::CompiledDigivolveCandidate,
    rctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
    bindings: Option<&Bindings>,
) -> bool {
    for player in resolve_predicate_players(spec.of, rctx) {
        let state = rctx.game.player(player);
        for zone in &spec.zones {
            let cards: &[crate::card_source::CardSource] = match zone {
                CompiledZone::Hand => &state.hand,
                CompiledZone::Trash => &state.trash,
                // Validated away at compile time; defensively contribute
                // nothing rather than scanning an unsupported zone.
                _ => continue,
            };
            for card in cards {
                let h = card.handle();
                // Full predicate eval (not the bare `eval_card_fields` leaf
                // path) so combinator filters â€” e.g. "a red OR blue Digimon
                // card" (`all_of` + `any_of`) â€” are honoured. Same fix as the
                // `source_count` leaf. G-HAS-DIGIVOLVE-CANDIDATE-COMPOSITE-FILTER.
                if let Some(filter) = &spec.filter {
                    if !eval_predicate_with_bindings(
                        filter,
                        rctx,
                        PredicateSubject::Card(h),
                        bindings,
                    ) {
                        continue;
                    }
                }
                if can_card_digivolve_onto(rctx, h, handle) {
                    return true;
                }
            }
        }
    }
    false
}

/// Phase 2 Track F (G-DSL-HAS-ON-DELETION-EFFECT) â€” true if `perm`'s top
/// card or any digivolution source carries any `OnDeletion`-timed effect.
/// Consulted by `has_on_deletion_effect: <bool>` predicate.
///
/// Walks both the registry-bound hand-written `CardEffect` impl and the
/// compiled DSL clauses (returned together by `Game::effects_for_card`)
/// and returns true on the first OnDeletion hit. Per printed text the
/// gate is on the existence of the printed timing, not on whether the
/// effect is currently runtime-active â€” `effects_for_card` already
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
        for (index, permanent) in rctx.game.player(player).battle_area.iter().enumerate() {
            let handle = PermanentHandle { player, index: index as u8 };
            let identity = permanent.synth_identity(&rctx.game.card_data, &rctx.game.modifiers, handle);
            if !kind_matches_membership(CompiledCardKind::Digimon, identity.kind, identity.retained_kind) {
                continue;
            }
            if identity.colors.iter().any(|field_color| colors.contains(field_color)) {
                return true;
            }
        }
    }
    false
}

/// True when at least one battle-area Digimon belonging to the scoped
/// player has the effective name `candidate_name`. Field names are read
/// through `synth_identity`, so a `ChangeBaseCardName` overlay on a field
/// Digimon is respected â€” consistent with how the `name_is` permanent
/// predicate resolves names. Name comparison is exact (case-sensitive),
/// matching `name_is` / `name_in`. Tamers and Options are skipped.
/// G-UNION-HAND-TRASH-NAME-EXCLUSION (Phase 2 Track J Task S2.2).
fn field_digimon_has_name(
    rctx: &EffectReadContext<'_>,
    of: CompiledPlayerRef,
    candidate_name: &str,
) -> bool {
    for player in existential_players(of, rctx) {
        for (index, permanent) in rctx.game.player(player).battle_area.iter().enumerate() {
            let handle = crate::permanent::PermanentHandle {
                player,
                index: index as u8,
            };
            let identity = permanent.synth_identity(rctx.card_data(), &rctx.game.modifiers, handle);
            if !kind_matches_membership(CompiledCardKind::Digimon, identity.kind, identity.retained_kind) {
                continue;
            }
            if identity
                .card_names
                .iter()
                .any(|name| name == candidate_name)
            {
                return true;
            }
        }
    }
    false
}

fn field_tamer_has_name(
    rctx: &EffectReadContext<'_>,
    of: CompiledPlayerRef,
    candidate_name: &str,
) -> bool {
    for player in existential_players(of, rctx) {
        for (index, permanent) in rctx.game.player(player).battle_area.iter().enumerate() {
            let handle = crate::permanent::PermanentHandle {
                player,
                index: index as u8,
            };
            let identity = permanent.synth_identity(rctx.card_data(), &rctx.game.modifiers, handle);
            if !kind_matches_membership(CompiledCardKind::Tamer, identity.kind, identity.retained_kind) {
                continue;
            }
            if identity
                .card_names
                .iter()
                .any(|name| name == candidate_name)
            {
                return true;
            }
        }
    }
    false
}

/// G-DSL-COLOR-MATCHES-OWN-SOURCE-STACK â€” true when the candidate card `data`
/// shares â‰¥1 color with the effect CARRIER's NON-FLIPPED digivolution-source
/// color set. The carrier is the effect's `source_permanent`; its source colors
/// are extracted by the shared `non_flipped_source_colors` (the same extraction
/// the branch-gating `DistinctColorsCount` uses). Empty carrier-color set (no
/// face-up sources) matches nothing â€” mirrors the sibling color leaves. The
/// candidate side is kind-aware exactly like `color_matches_binding`. Driver:
/// EX9-074 Kimeramon.
fn card_shares_color_with_own_source_stack(
    rctx: &EffectReadContext<'_>,
    candidate_colors: &[CardColor],
) -> bool {
    let Some(carrier) = rctx.source_permanent else {
        return false;
    };
    let Some(perm) = permanent_for_handle(rctx, carrier) else {
        return false;
    };
    let source_colors =
        crate::dsl_cards::formula_eval::non_flipped_source_colors(perm, rctx.card_data());
    if source_colors.is_empty() {
        return false;
    }
    candidate_colors
        .iter()
        .any(|candidate| source_colors.iter().any(|s| candidate == s))
}

fn card_shares_color_with_bound_permanent(
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
    binding: &str,
    candidate_colors: &[CardColor],
) -> bool {
    let Some(bound) = bindings.and_then(|b| b.get_permanent(binding)) else {
        return false;
    };
    let Some(permanent) = permanent_for_handle(rctx, bound) else {
        return false;
    };
    let bound_colors = permanent.colors_for_rules(rctx.card_data(), &rctx.game.modifiers, bound);
    if bound_colors.is_empty() {
        return false;
    }

    candidate_colors
        .iter()
        .any(|candidate| bound_colors.iter().any(|bound| candidate == bound))
}

/// G-RETURNED-CARD-COLOR-BINDING â€” true when the candidate card `data` shares
/// â‰¥1 color with ANY card recorded in this effect's `returned_to_deck` result
/// log. Unlike `color_matches_binding`, the returned card is never a permanent
/// (it moved trash â†’ deck bottom), so the comparison set is sourced from the
/// result log's stable `CardHandle`s resolved via `card_data_for_handle`, not a
/// permanent binding. The candidate side is kind-aware exactly like
/// `color_matches_binding`. Driver: EX10-068 Digimon Emperor On Play tail.
fn card_shares_color_with_returned_card(
    rctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
    candidate_colors: &[CardColor],
) -> bool {
    let Some(bindings) = bindings else {
        return false;
    };
    let returned = &bindings.result_log().returned_to_deck;
    if returned.is_empty() {
        return false;
    }

    if candidate_colors.is_empty() {
        return false;
    }

    returned.iter().any(|&handle| {
        let Some(returned_data) = rctx.game.card_data_for_handle(handle) else {
            return false;
        };
        returned_data
            .colors
            .iter()
            .any(|returned_color| candidate_colors.iter().any(|c| c == returned_color))
    })
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
    // Mirror of `other` â€” `is_source: true` requires the subject to BE the
    // effect's source permanent ("this Digimon" as a select filter).
    if pred.is_source == Some(true) && rctx.source_permanent != Some(handle) {
        return false;
    }

    let in_breeding = handle.index == crate::action::space::BREEDING_TARGET as u8;
    let Some(perm) = permanent_for_handle(rctx, handle) else {
        return false;
    };
    // Delegate the shared card fields to the card-handle path using the top card.
    let top_handle = perm.top_card().handle();
    let synth_identity = perm.synth_identity(rctx.card_data(), &rctx.game.modifiers, handle);
    // Track H Ã— Track C â€” propagate synth-identity overlays into the
    // shared card-field checks (`trait_has`, `name_is`, `name_contains`,
    // `name_in`, `color_is`, `color_only`). `eval_card_fields` only
    // sees printed `CardData`; without this, Track C overlays
    // (`ChangeTraits`, `ChangeBaseCardName`, `ChangeBaseCardColor`)
    // were invisible to Track H aura filters. We pre-check each
    // overlay-able field against `synth_identity` and clear it from
    // the delegated predicate if the overlay matches. Permanent DP
    // predicates are also cleared from the delegated card-field pass:
    // they are evaluated below through `effective_dp`, so same-effect
    // `ChangeDp` modifiers are visible before follow-up selections.
    // Pinned by `aura_filter_includes_track_c_change_traits_overlay`
    // and `dp_lte_selection_sees_same_effect_dp_modifier`.
    let trait_overlay_match = pred.trait_has.as_ref().is_some_and(|t| {
        synth_identity
            .traits
            .iter()
            .any(|x| x.eq_ignore_ascii_case(t))
    });
    let trait_contains_overlay_match = pred.trait_contains.as_ref().is_some_and(|t| {
        let needle = t.to_lowercase();
        synth_identity
            .traits
            .iter()
            .any(|x| x.to_lowercase().contains(&needle))
    });
    // `form_is` / `attribute_is` match the trait line like `trait_has`.
    let form_overlay_match = pred.form_is.as_ref().is_some_and(|t| {
        synth_identity
            .traits
            .iter()
            .any(|x| x.eq_ignore_ascii_case(t))
    });
    let attribute_overlay_match = pred.attribute_is.as_ref().is_some_and(|t| {
        synth_identity
            .traits
            .iter()
            .any(|x| x.eq_ignore_ascii_case(t))
    });
    // G-DSL-IN-TEXT-CONTAINS honors Track C overlays (`ChangeTraits` /
    // `ChangeBaseCardName`) the same way `trait_has` / `name_*` do: a
    // ChangeTraits-granted trait or a name overlay must be visible to the
    // whole-card text scan. The printed-text portion is unaffected by overlays,
    // so only the overlay's traits + names participate here; the delegated
    // card-field pass below still runs `card_in_text_contains` against the
    // printed `CardData` (which covers name/aliases/traits/text), so clearing
    // the field on an overlay match is correct only when the overlay itself
    // satisfies it.
    let in_text_contains_overlay_match = pred.in_text_contains.as_ref().is_some_and(|s| {
        let needle = s.to_lowercase();
        synth_identity
            .traits
            .iter()
            .chain(synth_identity.card_names.iter())
            .any(|x| x.to_lowercase().contains(&needle))
    });
    // G-DNA-MATERIAL-TREATED-AS-FOR-TARGET: while a DNA / Blast-DNA material
    // requirement is evaluated for a specific result card, the material's
    // result-scoped identity extras ("also treated as Lv.6 [Slayerdramon] for
    // [Examon]'s DNA digivolution") also satisfy `level_eq` / `name_*`.
    let dna_extras = if in_breeding {
        crate::dna_digivolve::DnaMaterialExtras::default()
    } else {
        rctx.dna_result_card
            .and_then(|h| rctx.game.card_source_for_handle(h))
            .map(|result| rctx.game.dna_material_extras(handle, result))
            .unwrap_or_default()
    };
    let level_eq_dna_match = pred
        .level_eq
        .is_some_and(|want| dna_extras.levels.contains(&want));
    let name_is_overlay_match = pred.name_is.as_ref().is_some_and(|n| {
        synth_identity.card_names.iter().any(|name| name == n)
            || dna_extras.names.iter().any(|name| name == n)
    });
    let name_contains_overlay_match = pred.name_contains.as_ref().is_some_and(|n| {
        let needle = n.to_lowercase();
        synth_identity
            .card_names
            .iter()
            .chain(dna_extras.names.iter())
            .any(|name| crate::card_data::rules_name_contains(name, &needle))
    });
    let name_in_overlay_match = pred.name_in.as_ref().is_some_and(|names| {
        names.iter().any(|n| {
            synth_identity.card_names.iter().any(|name| name == n)
                || dna_extras.names.iter().any(|name| name == n)
        })
    });
    // The synthesized names replace printed names when a base-name modifier
    // applies. A negative match must not fall through to printed card fields.
    if (pred.name_is.is_some() && !name_is_overlay_match)
        || (pred.name_contains.is_some() && !name_contains_overlay_match)
        || (pred.name_in.is_some() && !name_in_overlay_match)
    {
        return false;
    }
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
    // A replaced field color cannot fall back to the printed card color.
    // Card/search subjects still use their own printed-data evaluation path.
    if (pred.color_is.is_some() && !color_is_overlay_match)
        || (pred.color_only.is_some() && !color_only_overlay_match)
    {
        return false;
    }
    // Comparison leaves use effective colors for field candidates; physical
    // card searches retain their printed, kind-aware color path.
    let has_color_comparison = pred.color_matches_any_field_digimon.is_some()
        || pred.color_matches_binding.is_some()
        || pred.color_matches_returned_card.is_some()
        || pred.color_matches_own_source_stack.is_some();
    if pred.color_matches_any_field_digimon.is_some_and(|of|
        !card_shares_color_with_any_field_digimon(rctx, of, &synth_identity.colors))
        || pred.color_matches_binding.as_ref().is_some_and(|binding|
            !card_shares_color_with_bound_permanent(rctx, bindings, binding, &synth_identity.colors))
        || (pred.color_matches_returned_card == Some(true)
            && !card_shares_color_with_returned_card(rctx, bindings, &synth_identity.colors))
        || (pred.color_matches_own_source_stack.is_some()
            && !card_shares_color_with_own_source_stack(rctx, &synth_identity.colors))
    {
        return false;
    }
    let has_dp_constraint = pred.dp_eq.is_some() || pred.dp_lte.is_some() || pred.dp_gte.is_some();
    // `kind` for a field permanent is authoritatively checked below against
    // `synth_identity.kind` via `kind_matches_field` (which treats a battle-area
    // `CardKind::Token` as a Digimon â€” G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT). The
    // delegated `eval_card_fields` pass uses the printed-data card-search matcher
    // (`kind_matches_card_search`, no Tokenâ†’Digimon coalescing â€” correct for
    // deck/hand/trash search, where tokens never appear). So we strip `kind` from
    // the delegated predicate to avoid the card-search matcher wrongly rejecting a
    // token field permanent; the line-`kind_matches_field` check remains the sole
    // kind authority for permanent subjects.
    let has_kind_constraint = pred.kind.is_some();
    // `self_color_count_gte` on a permanent subject counts the SYNTHESIZED
    // colors (checked below against `synth_identity.colors`, which folds in
    // `ChangeBaseCardColor` / `AddColor`). The delegated card-field pass would
    // re-check it against the PRINTED colors and wrongly reject a permanent
    // whose color treatment raised its count (BT8-084 Kimeramon: printed white,
    // "[Your Turn] treated as also having the colors of its digivolution
    // cards", "while this Digimon has 4 or more colors" â€” DCGO reads
    // `TopCard.CardColors.Count` after ChangeCardColorClass). Strip it from the
    // delegated predicate; the synth check is the sole authority.
    // G-DSL-OWN-STACK-COLOR-COUNT-GTE.
    let has_self_color_count_constraint =
        pred.self_color_count_gte.is_some() || pred.self_color_count_lte.is_some();
    let delegated_pred_storage;
    let delegated_pred = if has_color_comparison
        || has_kind_constraint
        || level_eq_dna_match
        || trait_overlay_match
        || trait_contains_overlay_match
        || form_overlay_match
        || attribute_overlay_match
        || in_text_contains_overlay_match
        || name_is_overlay_match
        || name_contains_overlay_match
        || name_in_overlay_match
        || color_is_overlay_match
        || color_only_overlay_match
        || has_self_color_count_constraint
        || has_dp_constraint
    {
        let mut p = pred.clone();
        p.color_matches_any_field_digimon = None;
        p.color_matches_binding = None;
        p.color_matches_returned_card = None;
        p.color_matches_own_source_stack = None;
        if has_kind_constraint {
            // Authoritative permanent-kind check is `kind_matches_field`
            // (token-aware) below â€” don't let the card-search matcher re-reject.
            p.kind = None;
        }
        if level_eq_dna_match {
            p.level_eq = None;
        }
        if trait_overlay_match {
            p.trait_has = None;
        }
        if trait_contains_overlay_match {
            p.trait_contains = None;
        }
        if form_overlay_match {
            p.form_is = None;
        }
        if attribute_overlay_match {
            p.attribute_is = None;
        }
        if in_text_contains_overlay_match {
            p.in_text_contains = None;
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
        if has_self_color_count_constraint {
            p.self_color_count_gte = None;
            p.self_color_count_lte = None;
        }
        if has_dp_constraint {
            p.dp_eq = None;
            p.dp_lte = None;
            p.dp_gte = None;
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
        DualRef::Digimon,
    ) {
        return false;
    }
    if let Some(want) = pred.kind {
        if !kind_matches_membership(want, synth_identity.kind, synth_identity.retained_kind) {
            return false;
        }
    }
    if !eval_level_aggregate_match(pred, rctx, synth_identity.level) {
        return false;
    }
    if !eval_materials_aggregate_match(pred, rctx, handle) {
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
    if let Some(want) = pred.can_unsuspend {
        if !rctx.game.cannot_unsuspend(handle) != want {
            return false;
        }
    }
    if let Some(want) = pred.can_change_orientation {
        // DCGO `CanChangeOrientation`: IsSuspended ? CanUnsuspend : CanSuspend.
        let can = if perm.is_suspended {
            !rctx.game.cannot_unsuspend(handle)
        } else {
            !rctx
                .game
                .modifiers
                .has(handle, crate::enums::ModifierType::CannotSuspend)
        };
        if can != want {
            return false;
        }
    }
    if let Some(spec) = pred.can_attack {
        // G-DSL-CAN-ATTACK-PREDICATE â€” DCGO `Permanent.CanAttack`: false off
        // its owner's turn ("can not attack during opponent's turn"), then at
        // least one legal attack target exists under the same options the
        // consuming `may_attack_now` uses (identical target set, so a
        // candidate is never a dead pick). DCGO's `IsAttacking` gate is not
        // applied here.
        if rctx.game.turn_player() != handle.player {
            return false;
        }
        let restriction = match spec.targets {
            digimon_dsl::compiled::CompiledAttackTargetSpec::Any => {
                crate::effect_context::AttackTargetRestriction::Any
            }
            digimon_dsl::compiled::CompiledAttackTargetSpec::Player => {
                crate::effect_context::AttackTargetRestriction::PlayerOnly
            }
            digimon_dsl::compiled::CompiledAttackTargetSpec::Digimon => {
                crate::effect_context::AttackTargetRestriction::DigimonOnly
            }
        };
        let can = !crate::action::mask::effect_attack_target_action_ids_with_options(
            rctx.game,
            handle,
            restriction,
            spec.without_suspending,
            spec.ignore_summoning_sickness,
        )
        .is_empty();
        if !can {
            return false;
        }
    }
    if let Some(want) = pred.has_face_down_source {
        let has_face_down = perm.card_sources.iter().any(|cs| cs.face_down);
        if has_face_down != want {
            return false;
        }
    }
    if let Some(ref keyword) = pred.has_keyword {
        let Some(kw) = lookup_keyword(keyword, None) else {
            return false;
        };
        // A DSL keyword reference names the keyword with no parameter, so it
        // matches every value of it (§16-2); `lookup_keyword`'s default
        // parameter is only a placeholder for the type.
        if !rctx.game.has_keyword_type(handle, &kw) {
            return false;
        }
    }
    if let Some(want) = pred.has_security_attack_change {
        if rctx.game.has_security_attack_change(handle) != want {
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
    if let Some(cap) = pred.self_color_count_lte {
        if distinct_color_count(&synth_identity.colors) > usize::from(cap) {
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
    if let Some(want) = &pred.stack_size_eq {
        if perm.card_sources.len() as i32 != eval_int_constraint(want, rctx, Some(handle), bindings)
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
    if let Some(want) = &pred.materials_count_eq {
        if materials_count != eval_int_constraint(want, rctx, Some(handle), bindings) {
            return false;
        }
    }
    // `source_count` â€” count digivolution SOURCE cards (everything below the top
    // card) whose card satisfies the nested `filter`, and require the count to be
    // â‰¥ `at_least`. DCGO `DigivolutionCards.Count(predicate) >= N`. The top card
    // is `card_sources.last()`, so we scan `[..len-1]`. Each source is evaluated
    // as a card subject (`eval_card_fields`), which is exactly how a
    // `select_own_sources` filter reads a source. G-DSL-SOURCE-COUNT-FILTERED.
    if let Some((filter, at_least)) = &pred.source_count {
        let n = perm.card_sources.len();
        let matching = if n <= 1 {
            0
        } else {
            perm.card_sources[..n - 1]
                .iter()
                .filter(|s| {
                    // Full predicate eval (Card subject) so `all_of` / `any_of` /
                    // `none_of` / `not` in the nested filter are honoured â€” the
                    // bare `eval_card_fields` leaf path ignores combinators.
                    eval_predicate_ref(
                        filter,
                        rctx,
                        PredicateSubject::Card(s.handle()),
                        bindings,
                        DualRef::Digimon,
                    )
                })
                .count()
        };
        if matching < usize::from(*at_least) {
            return false;
        }
    }
    // `link_card_count` â€” the LINK-CARD sibling of `source_count`: count
    // `perm.linked_cards` satisfying the nested `filter` (each evaluated as a
    // card subject, like a `select_own_sources` filter) and require â‰¥
    // `at_least`. DCGO `permanent.LinkedCards.Count(predicate) >= N` â€” paired
    // with `source_count` it models `DigivolutionOrLinkCards.Any(...)`
    // (BT25-085). G-DSL-LINK-CARD-COUNT-FILTERED.
    if let Some((filter, at_least)) = &pred.link_card_count {
        let matching = perm
            .linked_cards
            .iter()
            .filter(|s| {
                // Full predicate eval (Card subject) so `all_of` / `any_of` /
                // `none_of` / `not` in the nested filter are honoured â€” the
                // bare `eval_card_fields` leaf path ignores combinators.
                eval_predicate_ref(
                    filter,
                    rctx,
                    PredicateSubject::Card(s.handle()),
                    bindings,
                    DualRef::Digimon,
                )
            })
            .count();
        if matching < usize::from(*at_least) {
            return false;
        }
    }
    // `has_digivolve_candidate` â€” DCGO's per-permanent `CanDigivolveDigimon`
    // gate: some hand/trash card matching the nested filter can digivolve
    // onto THIS permanent. G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST.
    if let Some(spec) = &pred.has_digivolve_candidate {
        if !permanent_has_digivolve_candidate(spec, rctx, handle, bindings) {
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
        if dp > eval_dp_constraint(cap, rctx, handle, bindings) + deletion_cap_bonus(pred, rctx) {
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
        CompiledAggregateSelector::LowestDp
        | CompiledAggregateSelector::HighestDp
        | CompiledAggregateSelector::FewestMaterials
        | CompiledAggregateSelector::LowestPlayCost => None,
    }
}

fn eval_materials_aggregate_match(
    pred: &CompiledPredicate,
    rctx: &EffectReadContext<'_>,
    handle: PermanentHandle,
) -> bool {
    let Some((selector, of)) = pred.materials_count_matches_aggregate else {
        return true;
    };
    let Some(perm) = permanent_for_handle(rctx, handle) else {
        return false;
    };
    let Some(aggregate_count) = aggregate_material_count(selector, of, rctx) else {
        return false;
    };
    let materials_count = perm.card_sources.len().saturating_sub(1) as i32;
    materials_count == aggregate_count
}

fn aggregate_material_count(
    selector: CompiledAggregateSelector,
    of: CompiledPlayerRef,
    rctx: &EffectReadContext<'_>,
) -> Option<i32> {
    let counts = existential_players(of, rctx)
        .into_iter()
        .flat_map(|player| {
            rctx.game
                .player(player)
                .battle_area
                .iter()
                .filter(|perm| perm.is_digimon(rctx.card_data()))
                .map(|perm| perm.card_sources.len().saturating_sub(1) as i32)
        });
    match selector {
        CompiledAggregateSelector::FewestMaterials => counts.min(),
        CompiledAggregateSelector::LowestDp
        | CompiledAggregateSelector::HighestDp
        | CompiledAggregateSelector::LowestLevel
        | CompiledAggregateSelector::HighestLevel
        | CompiledAggregateSelector::LowestPlayCost => None,
    }
}

/// "Add N to this Digimon's DP deletion effects' maximums": when `pred.dp_lte`
/// is a deletion effect's DP cap (flagged at compile time by
/// `digimon_dsl::deletion_cap`), the effect source permanent's summed
/// `ChangeDPDeleteEffectMaxDP` delta raises the cap. DCGO
/// `Player.MaxDP_DeleteEffect` â†’ `ChangeDPDeleteEffectMaxDPClass.ChangeMaxDP`
/// (adds only when `cardEffect.EffectSourceCard.PermanentOfThisCard()` is the
/// modifier's own permanent). 0 for non-deletion caps and for effects with no
/// source permanent (Options, rules processes).
/// G-ENGINE-DP-DELETION-MAX-MODIFIER.
fn deletion_cap_bonus(pred: &CompiledPredicate, rctx: &EffectReadContext<'_>) -> i32 {
    if !pred.dp_lte_deletion_cap {
        return 0;
    }
    deletion_max_bonus_for_source(rctx)
}

/// The effect source permanent's summed `ChangeDPDeleteEffectMaxDP` delta.
/// Shared by the predicate cap path and the deletion-flagged DP-budget select.
pub(crate) fn deletion_max_bonus_for_source(rctx: &EffectReadContext<'_>) -> i32 {
    rctx.source_permanent
        .map(|h| rctx.game.dp_delete_effect_max_bonus(h))
        .unwrap_or(0)
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
        DualRef::Digimon,
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
    if !eval_materials_aggregate_match(pred, rctx, handle) {
        return false;
    }
    if !eval_dp_constraints(pred, rctx, handle, bindings) {
        return false;
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
    if let Some(want) = &pred.stack_size_eq {
        if perm.card_sources.len() as i32 != eval_int_constraint(want, rctx, Some(handle), bindings)
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
    if let Some(want) = &pred.materials_count_eq {
        if materials_count != eval_int_constraint(want, rctx, Some(handle), bindings) {
            return false;
        }
    }
    // `source_count` on a breeding-area Digimon â€” same filtered-source-count gate
    // as the battle-area path (G-DSL-SOURCE-COUNT-FILTERED). Explicit here because
    // the shared `eval_card_fields` card-subject path does not inspect sources.
    if let Some((filter, at_least)) = &pred.source_count {
        let n = perm.card_sources.len();
        let matching = if n <= 1 {
            0
        } else {
            perm.card_sources[..n - 1]
                .iter()
                .filter(|s| {
                    // Full predicate eval (Card subject) so `all_of` / `any_of` /
                    // `none_of` / `not` in the nested filter are honoured â€” the
                    // bare `eval_card_fields` leaf path ignores combinators.
                    eval_predicate_ref(
                        filter,
                        rctx,
                        PredicateSubject::Card(s.handle()),
                        bindings,
                        DualRef::Digimon,
                    )
                })
                .count()
        };
        if matching < usize::from(*at_least) {
            return false;
        }
    }
    // `link_card_count` on a breeding-area Digimon: link cards only exist on
    // battle-area permanents (`Permanent.linked_cards` of a breeding permanent
    // is always empty), so any `at_least >= 1` gate is false here.
    // G-DSL-LINK-CARD-COUNT-FILTERED.
    if let Some((filter, at_least)) = &pred.link_card_count {
        let matching = perm
            .linked_cards
            .iter()
            .filter(|s| {
                // Full predicate eval (Card subject) so `all_of` / `any_of` /
                // `none_of` / `not` in the nested filter are honoured â€” the
                // bare `eval_card_fields` leaf path ignores combinators.
                eval_predicate_ref(
                    filter,
                    rctx,
                    PredicateSubject::Card(s.handle()),
                    bindings,
                    DualRef::Digimon,
                )
            })
            .count();
        if matching < usize::from(*at_least) {
            return false;
        }
    }
    // `has_digivolve_candidate` on a breeding permanent â€” probe routes onto
    // the BREEDING_TARGET sentinel handle (the route enumerator branches on
    // it). G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST.
    if let Some(spec) = &pred.has_digivolve_candidate {
        let sentinel = PermanentHandle {
            player,
            index: crate::action::space::BREEDING_TARGET as u8,
        };
        if !permanent_has_digivolve_candidate(spec, rctx, sentinel, bindings) {
            return false;
        }
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
    if let Some(want) = pred.has_face_down_source {
        let has_face_down = perm.card_sources.iter().any(|cs| cs.face_down);
        if has_face_down != want {
            return false;
        }
    }

    pred.is_suspended.is_none()
        && pred.is_unsuspended.is_none()
        && pred.can_change_orientation.is_none()
        && pred.can_unsuspend.is_none()
        && pred.can_attack.is_none()
}

fn kind_matches(want: CompiledCardKind, got: CardKind) -> bool {
    matches!(
        (want, got),
        (CompiledCardKind::Digimon, CardKind::Digimon)
            | (CompiledCardKind::Tamer, CardKind::Tamer)
            | (CompiledCardKind::Option, CardKind::Option)
            | (CompiledCardKind::DigiEgg, CardKind::DigiEgg)
            | (CompiledCardKind::Token, CardKind::Token)
            // `kind: dual` names a DUAL card in a card zone, so `not: { kind:
            // dual }` can exclude it â€” a DUAL has no play cost ("Play cost: D"),
            // so "a card with a play cost of N or less" never matches it
            // (BT26-075; DCGO `HasPlayCost`). G-DSL-KIND-DUAL-CARD-SEARCH.
            | (CompiledCardKind::Dual, CardKind::Dual)
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
        // A battle-area `CardKind::Token` permanent IS a Digimon (rules manual /
        // glossary: a Digimon token is a Digimon on the field; DCGO
        // `Permanent.IsDigimon` is true for token entities). So `kind: digimon`
        // field selectors/predicates must accept tokens
        // (G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT, judge-quiz Q12).
        (
            CompiledCardKind::Digimon,
            CardKind::Digimon | CardKind::Dual | CardKind::Token | CardKind::DigiEgg
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

/// Effective Option use cost of the concrete card `card` (printed + its own
/// use-cost increase), for the card's owner. `None` if the card can't be found.
fn effective_use_cost(rctx: &EffectReadContext<'_>, card: CardHandle) -> Option<i32> {
    let game = rctx.game;
    for player in game.players.iter() {
        for zone in [&player.hand, &player.trash, &player.security, &player.deck] {
            if let Some(c) = zone.iter().find(|c| c.handle() == card) {
                return Some(i32::from(game.option_use_cost(c, c.owner)));
            }
        }
    }
    game.revealed_cards
        .iter()
        .find(|c| c.handle() == card)
        .map(|c| i32::from(game.option_use_cost(c, c.owner)))
}

fn kind_matches_membership(want: CompiledCardKind, primary: CardKind, retained: Option<CardKind>) -> bool {
    kind_matches_field(want, primary) || retained.is_some_and(|kind| kind_matches_field(want, kind))
}

#[cfg(test)]
mod physical_binding_exclusion_tests {
    use super::*;
    use crate::selection::{SourceSelectionRef, UnionZoneOrigin};
    #[test]
    fn physical_card_exclusion_matches_all_supported_card_bindings() {
        let first = CardHandle(10);
        let twin = CardHandle(11);
        let host = PermanentHandle { player: 0, index: 0 };
        for value in [
            BindingValue::Card(first),
            BindingValue::UnionCard { card: first, origin: UnionZoneOrigin::Hand, owner: 0 },
            BindingValue::CardList(vec![first]),
            BindingValue::SourceRefs(vec![SourceSelectionRef { permanent: host, field_index: 0, source_index: 0, card: first }]),
        ] {
            let mut bindings = Bindings::new();
            bindings.insert("picked", value);
            for subject in [PredicateSubject::Card(first), PredicateSubject::RevealedCard(first)] {
                assert!(!subject_not_in_binding(subject, "picked", Some(&bindings)));
            }
            for subject in [PredicateSubject::Card(twin), PredicateSubject::RevealedCard(twin)] {
                assert!(subject_not_in_binding(subject, "picked", Some(&bindings)));
            }
        }
    }
    #[test]
    fn missing_wrong_type_and_permanent_exclusion_stay_fail_closed() {
        let mut bindings = Bindings::new();
        let card = PredicateSubject::Card(CardHandle(10));
        assert!(!subject_not_in_binding(card, "picked", None));
        assert!(!subject_not_in_binding(card, "picked", Some(&bindings)));
        bindings.insert_literal("picked", 10);
        assert!(!subject_not_in_binding(card, "picked", Some(&bindings)));
        let first = PermanentHandle { player: 0, index: 0 };
        let other = PermanentHandle { player: 0, index: 1 };
        for value in [BindingValue::Permanent(first), BindingValue::PermanentList(vec![first])] {
            bindings.insert("picked", value);
            assert!(!subject_not_in_binding(PredicateSubject::Permanent(first), "picked", Some(&bindings)));
            assert!(subject_not_in_binding(PredicateSubject::Permanent(other), "picked", Some(&bindings)));
            assert!(!subject_not_in_binding(card, "picked", Some(&bindings)));
        }
    }
}
