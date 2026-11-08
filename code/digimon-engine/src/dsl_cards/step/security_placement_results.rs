//! Actual selected/top-card placement receipts, captured before replacements.
use crate::dsl_cards::{
    binding_ref::{resolve_binding_ref, ResolvedBinding},
    bindings::Bindings,
};
use crate::effect_context::EffectContext;
use digimon_dsl::compiled::CompiledStep;

pub(super) fn expand(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> Option<Vec<CompiledStep>> {
    if let CompiledStep::PlacePermanentOnSecurity {
        target,
        bind_placed_as: Some(name),
        ..
    } = step
    {
        // Pin the exact physical top before any would-leave/placement window.
        // A successful rule-routed egg/token movement still pays (CR 15-1-9).
        let selected = resolve_binding_ref(target, ctx, bindings);
        bindings.remove(name);
        let Some(ResolvedBinding::Permanent(handle)) = selected else {
            return Some(Vec::new());
        };
        let Some(card) = ctx
            .game
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .map(|p| p.top_card().handle())
        else {
            return Some(Vec::new());
        };
        let source_breeding = ctx
            .source_permanent
            .is_some_and(|h| h.index == crate::action::space::BREEDING_TARGET as u8);
        let source_anchor = ctx
            .source_permanent
            .and_then(|h| ctx.game.source_move_anchor(h));
        let rebind = bindings
            .permanent_slots()
            .into_iter()
            .filter(|(n, h)| {
                n != name
                    && h.player == handle.player
                    && h.index != crate::action::space::BREEDING_TARGET as u8
            })
            .filter_map(|(n, h)| ctx.game.source_move_anchor(h).map(|a| (n, a)))
            .collect();
        let mut receipt = 0;
        while ctx
            .game
            .permanent_security_placement_receipts
            .iter()
            .any(|(id, _)| *id == receipt)
        {
            receipt += 1;
        }
        ctx.game
            .permanent_security_placement_receipts
            .push((receipt, false));
        let mut operation = step.clone();
        if let CompiledStep::PlacePermanentOnSecurity {
            bind_placed_as,
            result_receipt,
            ..
        } = &mut operation
        {
            *bind_placed_as = None;
            *result_receipt = Some(receipt);
        }
        return Some(vec![
            operation,
            CompiledStep::SecurityPermanentPlacementFinalize {
                receipt,
                card: card.0,
                bind_as: name.clone(),
                source_anchor,
                source_breeding,
                rebind,
            },
        ]);
    }
    let (carrier, name) = match step {
        CompiledStep::SecurityPlaceStackedCard {
            carrier,
            bind_success_as: Some(name),
            ..
        }
        | CompiledStep::SecurityPlaceTopStackedCard {
            carrier,
            bind_success_as: Some(name),
            ..
        }
        | CompiledStep::SecurityPlaceTopCard {
            carrier,
            bind_success_as: Some(name),
            ..
        } => (carrier, name),
        _ => return None,
    };
    // Every attempt overwrites the old result, including invalid/no-op attempts.
    bindings.insert_literal(name, 0);
    let Some(ResolvedBinding::Permanent(handle)) = resolve_binding_ref(carrier, ctx, bindings)
    else {
        return Some(vec![]);
    };
    let Some(p) = ctx
        .game
        .player(handle.player)
        .battle_area
        .get(handle.index as usize)
    else {
        return Some(vec![]);
    };
    let card = match step {
        CompiledStep::SecurityPlaceStackedCard {
            source,
            source_index_from_top,
            ..
        } => {
            let Some(card) = super::zone_moves::selected_security_source(
                ctx,
                bindings,
                handle,
                source,
                *source_index_from_top,
            ) else {
                return Some(vec![]);
            };
            if !p.card_sources.iter().any(|c| c.handle() == card) {
                return Some(vec![]);
            }
            card.0
        }
        _ => {
            if p.card_sources.len() < 2 {
                return Some(vec![]);
            }
            p.top_card().handle().0
        }
    };
    // Preserve the permanent itself when its effect source moves off the top.
    // Existing identity pins survive clone/compaction and fail closed on departure.
    if ctx.source_permanent.is_some()
        && p.card_sources
            .iter()
            .any(|source| source.handle() == ctx.source_card)
    {
        let identity = ctx.game.capture_digivolve_host_identity(handle);
        bindings.insert_selected_permanent_with_identity(
            &crate::dsl_cards::binding_ref::source_carrier_pin_key(ctx.source_card),
            handle,
            identity,
        );
    }
    ctx.game.security_placement_receipts.push((card, false));
    let mut operation = step.clone();
    match &mut operation {
        CompiledStep::SecurityPlaceStackedCard {
            bind_success_as, ..
        }
        | CompiledStep::SecurityPlaceTopStackedCard {
            bind_success_as, ..
        }
        | CompiledStep::SecurityPlaceTopCard {
            bind_success_as, ..
        } => *bind_success_as = None,
        _ => unreachable!(),
    }
    Some(vec![
        operation,
        CompiledStep::SecurityPlacementSuccessFinalize {
            card,
            bind_as: name.clone(),
        },
    ])
}

pub(super) fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &mut Bindings,
) -> bool {
    if let CompiledStep::SecurityPermanentPlacementFinalize {
        receipt,
        card,
        bind_as,
        source_anchor,
        source_breeding,
        rebind,
    } = step
    {
        for (name, anchor) in rebind {
            // Do not overwrite bindings an interrupting effect deliberately changed.
            if !matches!(
                bindings.get(name),
                Some(crate::dsl_cards::bindings::BindingValue::Permanent(_))
            ) {
                continue;
            }
            match ctx.game.resolve_source_move_anchor(*anchor, false) {
                Some(h) => bindings.insert_permanent(name, h),
                None => bindings.remove(name),
            }
        }
        let paid = ctx
            .game
            .permanent_security_placement_receipts
            .iter()
            .rposition(|(id, _)| id == receipt)
            .map(|i| ctx.game.permanent_security_placement_receipts.remove(i).1)
            .unwrap_or(false);
        if paid {
            bindings.insert_card(bind_as, crate::card_source::CardHandle(*card));
        } else {
            bindings.remove(bind_as);
        }
        if let Some(anchor) = source_anchor {
            ctx.source_permanent = ctx
                .game
                .resolve_source_move_anchor(*anchor, *source_breeding);
        }
        return true;
    }
    let CompiledStep::SecurityPlacementSuccessFinalize { card, bind_as } = step else {
        return false;
    };
    let success = ctx
        .game
        .security_placement_receipts
        .iter()
        .rposition(|(c, _)| c == card)
        .map(|i| ctx.game.security_placement_receipts.remove(i).1)
        .unwrap_or(false);
    bindings.insert_literal(bind_as, i64::from(success));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_runner::{make_test_card, DebugRunner};
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPlayerRef, CompiledStackPosition};

    fn runner() -> DebugRunner {
        DebugRunner::builder()
            .add_card(make_test_card("LOWER", "Lower"))
            .add_card(make_test_card("TOP", "Top"))
            .start()
    }
    #[test]
    fn success_finalize_consumes_only_innermost_physical_receipt() {
        let mut r = runner();
        let h = r.place_stack(0, &["LOWER", "TOP"]);
        let card = r.game.players[0].battle_area[0].top_card().handle();
        r.game.security_placement_receipts = vec![(card.0, false), (card.0, true)];
        let mut b = Bindings::new();
        let mut ctx = EffectContext::new(&mut r.game, card, Some(h), 0);
        let step = CompiledStep::SecurityPlacementSuccessFinalize {
            card: card.0,
            bind_as: "paid".into(),
        };
        assert!(try_run(&step, &mut ctx, &mut b));
        assert_eq!(b.get_literal("paid"), Some(1));
        assert_eq!(ctx.game.security_placement_receipts, vec![(card.0, false)]);
        assert!(try_run(&step, &mut ctx, &mut b));
        assert_eq!(b.get_literal("paid"), Some(0));
        assert!(ctx.game.security_placement_receipts.is_empty());
    }
    #[test]
    fn actual_sync_operation_cleans_receipt() {
        let mut r = runner();
        let h = r.place_stack(0, &["LOWER", "TOP"]);
        let card = r.game.players[0].battle_area[0].top_card().handle();
        let mut b = Bindings::new();
        let step = CompiledStep::SecurityPlaceTopStackedCard {
            carrier: CompiledBindingRef::Source,
            of: CompiledPlayerRef::You,
            position: CompiledStackPosition::Top,
            face_up: false,
            to_owner: false,
            bind_success_as: Some("paid".into()),
        };
        let mut ctx = EffectContext::new(&mut r.game, card, Some(h), 0);
        super::super::run_steps(&[step], &mut ctx, &mut b);
        assert_eq!(b.get_literal("paid"), Some(1));
        assert!(ctx.game.security_placement_receipts.is_empty());
    }
    #[test]
    fn aborted_clause_consumes_only_its_own_lifo_receipt() {
        let mut r = runner();
        let h = r.place_stack(0, &["LOWER", "TOP"]);
        let card = r.game.players[0].battle_area[0].top_card().handle();
        r.game.security_placement_receipts = vec![(card.0, true), (card.0, false)];
        r.game.dsl_clause_aborted = true;
        let mut ctx = EffectContext::new(&mut r.game, card, Some(h), 0);
        let mut b = Bindings::new();
        super::super::run_steps(
            &[CompiledStep::SecurityPlacementSuccessFinalize {
                card: card.0,
                bind_as: "paid".into(),
            }],
            &mut ctx,
            &mut b,
        );
        assert_eq!(ctx.game.security_placement_receipts, vec![(card.0, true)]);
        assert!(b.get_literal("paid").is_none());
    }
    #[test]
    fn actual_pending_operation_clone_consumes_receipt_on_accept_and_decline() {
        use crate::action::space::{PASS, REPLACEMENT_ACCEPT};
        use crate::{
            card_source::CardHandle,
            effect::{CardEffect, Effect, EffectBuilder},
            enums::EffectTiming,
        };
        struct Block;
        impl CardEffect for Block {
            fn effects(&self, card: CardHandle) -> Vec<Effect> {
                vec![
                    EffectBuilder::new(card, EffectTiming::WhenWouldPlaceInSecurity)
                        .optional()
                        .replacement_process(|ctx| ctx.cancel())
                        .build(),
                ]
            }
        }
        let mut r = runner();
        r.register_effect("LOWER", std::sync::Arc::new(Block));
        let h = r.place_stack(0, &["LOWER", "TOP"]);
        r.place_on_field(0, "LOWER", None);
        let card = r.game.players[0].battle_area[0].top_card().handle();
        let mut b = Bindings::new();
        let step = CompiledStep::SecurityPlaceTopStackedCard {
            carrier: CompiledBindingRef::Source,
            of: CompiledPlayerRef::You,
            position: CompiledStackPosition::Top,
            face_up: false,
            to_owner: false,
            bind_success_as: Some("paid".into()),
        };
        super::super::run_steps(
            &[step],
            &mut EffectContext::new(&mut r.game, card, Some(h), 0),
            &mut b,
        );
        assert!(r.game.pending_selection.is_some());
        assert_eq!(r.game.security_placement_receipts, vec![(card.0, false)]);
        let saved = r.game.clone();
        for action in [PASS, REPLACEMENT_ACCEPT] {
            r.game = saved.clone();
            r.game.resolve_selection(0, action).unwrap();
            r.auto_resolve().unwrap();
            assert!(r.game.security_placement_receipts.is_empty());
            assert!(r.game.pending_security_moves.is_empty());
        }
    }
    #[test]
    fn abandoned_security_receipt_finalizer_consumes_only_its_own_lifo_receipt() {
        use crate::dsl_cards::{bindings::Bindings, step::run_steps};
        use digimon_dsl::compiled::{
            CompiledBindingRef, CompiledCountBound, CompiledPredicate, CompiledStep,
        };
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("RECOVERY-SOURCE", "Source"))
            .add_card(make_test_card("RECOVERY-TOP", "Top"))
            .deck(0, &["RECOVERY-SOURCE"; 20])
            .deck(1, &["RECOVERY-SOURCE"; 20])
            .memory(3)
            .start();
        r.skip_mulligan();
        let host = r.place_stack(0, &["RECOVERY-SOURCE", "RECOVERY-TOP"]);
        let top = r.game.player(0).battle_area[0].top_card().handle();
        r.game.security_placement_receipts = vec![(top.0, true), (top.0, false)];
        run_steps(
            &[
                CompiledStep::SelectMaterials {
                    named_groups: Vec::new(),
                    require_max: false,
                    of_permanent: CompiledBindingRef::Source,
                    min: 1,
                    max: CompiledCountBound::Literal(1),
                    filter: CompiledPredicate::default(),
                    uniqueness: None,
                    bind_as: Some("picked".into()),
                    prompt: "Cost".into(),
                    prompt_key: None,
                    optional_zero: false,
                    same_by: None,
                },
                CompiledStep::SecurityPlacementSuccessFinalize {
                    card: top.0,
                    bind_as: "paid".into(),
                },
                CompiledStep::GainMemory(4),
            ],
            &mut EffectContext::new(&mut r.game, top, Some(host), 0),
            &mut Bindings::new(),
        );
        r.game.players[0].battle_area[0].card_sources.remove(0);
        r.game
            .resolve_selection(0, crate::action::space::PASS)
            .unwrap();
        assert_eq!(r.game.memory, 3);
        assert_eq!(r.game.security_placement_receipts, vec![(top.0, true)]);
    }
    #[test]
    fn abandoned_permanent_payment_consumes_only_its_correlated_receipt() {
        use crate::dsl_cards::{bindings::Bindings, step::run_steps};
        use digimon_dsl::compiled::{
            CompiledBindingRef, CompiledCountBound, CompiledPredicate, CompiledStep,
        };
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("PAY-LOW", "Lower"))
            .add_card(make_test_card("PAY-TOP", "Top"))
            .memory(3)
            .start();
        let host = r.place_stack(0, &["PAY-LOW", "PAY-TOP"]);
        let top = r.game.player(0).battle_area[0].top_card().handle();
        r.game.permanent_security_placement_receipts = vec![(0, true), (1, false)];
        run_steps(
            &[
                CompiledStep::SelectMaterials {
                    named_groups: Vec::new(),
                    require_max: false,
                    of_permanent: CompiledBindingRef::Source,
                    min: 1,
                    max: CompiledCountBound::Literal(1),
                    filter: CompiledPredicate::default(),
                    uniqueness: None,
                    bind_as: Some("picked".into()),
                    prompt: "Cost".into(),
                    prompt_key: None,
                    optional_zero: false,
                    same_by: None,
                },
                CompiledStep::SecurityPermanentPlacementFinalize {
                    receipt: 1,
                    card: top.0,
                    bind_as: "paid".into(),
                    source_anchor: None,
                    source_breeding: false,
                    rebind: Vec::new(),
                },
                CompiledStep::GainMemory(4),
            ],
            &mut EffectContext::new(&mut r.game, top, Some(host), 0),
            &mut Bindings::new(),
        );
        r.game.players[0].battle_area[0].card_sources.remove(0);
        r.game
            .resolve_selection(0, crate::action::space::PASS)
            .unwrap();
        assert_eq!(r.game.memory, 3);
        assert_eq!(
            r.game.permanent_security_placement_receipts,
            vec![(0, true)]
        );
    }
}
