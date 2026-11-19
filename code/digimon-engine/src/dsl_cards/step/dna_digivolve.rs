//! G-DSL-EOT-DNA-INLINE — step lowering for `CompiledStep::MayDnaDigivolveNow`.
//!
//! Bridges the DSL `CompiledPredicate` filters into runtime closures that
//! capture `Bindings`, `source_card / source_permanent / source_kind`,
//! and the controller's player id — then calls the
//! `EffectContext::may_dna_digivolve_now` orchestrator which owns the
//! three-stage selection chain (partner pick → target pick → DNA digivolve).
//!
//! Used by the 6 cards that print `[End of Your Turn] This Digimon and any
//! of your other Digimon may DNA digivolve into a Digimon card in the hand`
//! (BT12-021, BT12-047, BT17-007, BT17-019, BT22-008, BT22-017). See the
//! YAML migration for the authoring shape.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use crate::effect_context::{EffectContext, EffectReadContext};

pub fn try_run(step: &CompiledStep, ctx: &mut EffectContext<'_>, bindings: &Bindings) -> bool {
    match step {
        CompiledStep::MayDnaDigivolveNow {
            anchor,
            partner_filter,
            target_filter,
            cost,
            ignore_requirements,
            optional,
            prompt,
        } => {
            // Resolve anchor binding to a permanent handle. Defaults to the
            // trigger's source permanent (`anchor: source` → `CompiledBindingRef::Source`).
            let anchor_handle = match resolve_binding_ref(anchor, ctx, bindings) {
                Some(ResolvedBinding::Permanent(h)) => h,
                _ => {
                    // Anchor binding unresolved → silent no-op.
                    return true;
                }
            };

            // Snapshot context the predicate-eval closures need to rebuild
            // `EffectReadContext` at install/callback time.
            let source_card = ctx.source_card;
            let source_permanent = ctx.source_permanent;
            let source_kind = ctx.source_kind;
            let player = ctx.player;
            let bindings_snapshot = bindings.clone();
            let (partner_closure, target_closure) = dna_filter_closures(
                partner_filter,
                target_filter,
                source_card,
                source_permanent,
                source_kind,
                player,
                &bindings_snapshot,
            );

            ctx.may_dna_digivolve_now(
                anchor_handle,
                partner_closure,
                target_closure,
                *cost,
                *ignore_requirements,
                *optional,
                prompt.as_deref(),
                None,
            );
            true
        }
        _ => false,
    }
}

type PartnerClosure =
    Arc<dyn Fn(&crate::game::Game, crate::permanent::PermanentHandle) -> bool + Send + Sync>;
type TargetClosure = Arc<dyn Fn(&crate::game::Game, usize) -> bool + Send + Sync>;

/// Runtime closures for a `may_dna_digivolve_now` step's partner / hand-target
/// predicates. Shared by the step itself and by [`DnaSubjectProbe`], so the
/// probe asks exactly the question the step will ask.
fn dna_filter_closures(
    partner_filter: &CompiledPredicate,
    target_filter: &CompiledPredicate,
    source_card: crate::card_source::CardHandle,
    source_permanent: Option<crate::permanent::PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
    player: crate::enums::PlayerId,
    bindings: &Bindings,
) -> (PartnerClosure, TargetClosure) {
    let partner_pred = partner_filter.clone();
    let target_pred = target_filter.clone();
    let bindings_for_partner = bindings.clone();
    let bindings_for_target = bindings.clone();

    let partner_closure: PartnerClosure = Arc::new(move |game, h| {
        let read_ctx = EffectReadContext::new_with_source_kind(
            game,
            source_card,
            source_permanent,
            source_kind,
            player,
        );
        eval_predicate_with_bindings(
            &partner_pred,
            &read_ctx,
            PredicateSubject::Permanent(h),
            Some(&bindings_for_partner),
        )
    });

    let target_closure: TargetClosure = Arc::new(move |game, hand_idx| {
        let card = match game.player(player).hand.get(hand_idx).map(|c| c.handle()) {
            Some(c) => c,
            None => return false,
        };
        let read_ctx = EffectReadContext::new_with_source_kind(
            game,
            source_card,
            source_permanent,
            source_kind,
            player,
        );
        eval_predicate_with_bindings(
            &target_pred,
            &read_ctx,
            PredicateSubject::Card(card),
            Some(&bindings_for_target),
        )
    });
    (partner_closure, target_closure)
}

/// What a replacement-window DNA digivolve (`may_dna_digivolve_now` anchored on
/// the leaving subject, e.g. Kapurimon EX12-003) can reach from a given subject.
/// The deletion batch uses it to decide whether the choice of WHICH leaving
/// Digimon anchors the digivolve is a real choice (see
/// [`DnaSubjectProbe::partners_of`]).
#[derive(Clone, Debug)]
pub struct DnaSubjectProbe {
    pub partner_filter: CompiledPredicate,
    pub target_filter: CompiledPredicate,
    pub ignore_requirements: bool,
}

impl DnaSubjectProbe {
    /// The finder used at lowering time: the first `MayDnaDigivolveNow` step
    /// anchored on `replacement_subject` in a replacement's process (top level
    /// or inside an `if`).
    pub fn from_process(process: &[CompiledStep]) -> Option<Self> {
        for step in process {
            match step {
                CompiledStep::MayDnaDigivolveNow {
                    anchor,
                    partner_filter,
                    target_filter,
                    ignore_requirements,
                    ..
                } if matches!(
                    anchor,
                    digimon_dsl::compiled::CompiledBindingRef::Named(n)
                        | digimon_dsl::compiled::CompiledBindingRef::Binding(n)
                        if n == "replacement_subject"
                ) =>
                {
                    return Some(Self {
                        partner_filter: partner_filter.clone(),
                        target_filter: target_filter.clone(),
                        ignore_requirements: *ignore_requirements,
                    });
                }
                CompiledStep::If {
                    then, else_branch, ..
                } => {
                    if let Some(p) =
                        Self::from_process(then).or_else(|| Self::from_process(else_branch))
                    {
                        return Some(p);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Every own battle-area permanent `partner` such that `{subject, partner}`
    /// is a legal DNA pair for at least one filtered hand card — the partner
    /// pick the step would offer with `subject` as the anchor.
    #[allow(clippy::too_many_arguments)]
    pub fn partners_of(
        &self,
        game: &crate::game::Game,
        source_card: crate::card_source::CardHandle,
        source_permanent: Option<crate::permanent::PermanentHandle>,
        source_kind: crate::enums::EffectSourceKind,
        controller: crate::enums::PlayerId,
        subject: crate::permanent::PermanentHandle,
    ) -> Vec<crate::permanent::PermanentHandle> {
        let (partner, target) = dna_filter_closures(
            &self.partner_filter,
            &self.target_filter,
            source_card,
            source_permanent,
            source_kind,
            controller,
            &Bindings::new(),
        );
        (0..game.player(controller).battle_area.len())
            .map(|i| crate::permanent::PermanentHandle {
                player: controller,
                index: i as u8,
            })
            .filter(|&p| {
                crate::effect_context::dna_partner_has_target(
                    game,
                    controller,
                    subject,
                    p,
                    self.ignore_requirements,
                    partner.as_ref(),
                    target.as_ref(),
                )
            })
            .collect()
    }
}
