//! Actual deletion commits, separated from attempts and replacement outcomes.
use digimon_dsl::compiled::CompiledStep;
use crate::dsl_cards::{bindings::Bindings,binding_ref::{resolve_binding_ref,ResolvedBinding}};
use crate::effect_context::EffectContext;
use crate::permanent::PermanentHandle;

pub(super) fn expand(step:&CompiledStep,ctx:&mut EffectContext<'_>,bindings:&Bindings)->Option<Vec<CompiledStep>> {
    let mut targets:Vec<PermanentHandle>=match step {
        CompiledStep::DeleteAllPermanents {over,selector} => {
            let mut scanned=super::permanent_scan::scan(ctx,over,Some(bindings));
            if let Some(selector)=selector {
                if let Some(extreme)=super::selections::selected_field_extreme(ctx.game,&scanned,*selector) {
                    scanned.retain(|h|super::selections::field_value(ctx.game,*h,*selector)==Some(extreme));
                }
            }
            scanned
        },
        CompiledStep::DeletePermanent {target} => match resolve_binding_ref(target,ctx,bindings) {Some(ResolvedBinding::Permanent(h))=>vec![h],_=>Vec::new()},
        CompiledStep::DeletePermanents {targets} => targets.iter().filter_map(|target|match resolve_binding_ref(target,ctx,bindings){Some(ResolvedBinding::Permanent(h))=>Some(h),_=>None}).collect(),
        // Preserve bound-list sequential semantics until separately migrated.
        _=>return None,
    };
    targets.sort_by_key(|h|(h.player,h.index));targets.dedup();
    targets.retain(|h|ctx.game.player(h.player).battle_area.get(h.index as usize).is_some() && ctx.can_affect_permanent(*h));
    if targets.is_empty(){return Some(Vec::new());}
    let captured=targets.into_iter().map(|h|(h.player,ctx.game.capture_digivolve_host_identity(h))).collect();
    let receipt=ctx.game.allocate_deletion_receipt();
    Some(vec![CompiledStep::DeleteCapturedBatch{receipt,targets:captured},CompiledStep::DeleteReceiptFinalize{receipt}])
}

pub(super) fn try_run(step:&CompiledStep,ctx:&mut EffectContext<'_>,bindings:&mut Bindings)->bool {
    match step {
        CompiledStep::DeleteCapturedBatch{receipt,targets}=>{
            let handles:Vec<_>=targets.iter().filter_map(|(p,id)|ctx.game.resolve_digivolve_host_identity(*p,*id)).collect();
            ctx.game.deletion_receipts.push((*receipt,Vec::new()));
            if let Some(first)=handles.first(){
                let cause=ctx.game.infer_deletion_cause(*first);
                ctx.game.delete_permanents_batch_with_receipt(handles,cause,Some(*receipt));
            }
            true
        },
        CompiledStep::DeleteReceiptFinalize{receipt}=>{
            if let Some(index)=ctx.game.deletion_receipts.iter().position(|(id,_)|id==receipt){
                let (_,completed)=ctx.game.deletion_receipts.remove(index);
                for (handle,dp,is_digimon,_) in completed {bindings.record_deleted(handle,dp,is_digimon);}
            }
            true
        },
        CompiledStep::PaidDeletionPreventionFinalize { receipt, payer } => {
            if take_paid_receipt(ctx,*receipt,*payer) {
                super::replacement_outcome::set_outcome(ctx, crate::replacement::ReplacementOutcome::Cancelled);
            }
            true
        },
        CompiledStep::PaidDeletionBatchPreventionFinalize { receipt,payer,members,filter,protect_all,cause,cause_player } => {
            if take_paid_receipt(ctx,*receipt,*payer) {
                // Payment has physically completed. Pending On Deletion effects
                // stay queued, but continuous and until-condition state already
                // follows the current zones before we expose target choices.
                ctx.game.tick_declarative_effects();
                ctx.game.reevaluate_until_condition_modifiers();
                let targets:Vec<_>=members.iter().filter_map(|m|resolve_member(ctx.game,*m))
                    .filter(|h|paid_target_matches(ctx,*h,filter,*cause,*cause_player)).collect();
                if *protect_all {
                    if !targets.is_empty(){super::replacement_outcome::set_outcome(ctx,crate::replacement::ReplacementOutcome::Cancelled);}
                } else { select_paid_prevention_member(ctx,members,targets,filter,*cause,*cause_player); }
            }
            true
        },
        CompiledStep::PreventDeletionMemberFinalize { binding,members,filter,cause,cause_player } => {
            if let Some(h)=bindings.get_permanent(binding) {
                if members.iter().any(|m|resolve_member(ctx.game,*m)==Some(h))
                    && paid_target_matches(ctx,h,filter,*cause,*cause_player) {
                    crate::replacement::prevent_chosen_deletion_member(ctx.game,h);
                }
            }
            true
        },
        _=>false,
    }
}

fn resolve_member(game:&crate::game::Game, identity:(u8,u16,u64))->Option<PermanentHandle> {
    game.resolve_digivolve_host_identity(identity.0,identity.2)
}
fn select_paid_prevention_member(ctx:&mut EffectContext<'_>,members:&[(u8,u16,u64)],targets:Vec<PermanentHandle>,filter:&digimon_dsl::compiled::CompiledPredicate,cause:digimon_dsl::compiled::CompiledReplacementCause,cause_player:Option<u8>) {
    use digimon_dsl::compiled::CompiledPredicate;
    if targets.is_empty(){return;}
    let mut bindings=Bindings::new();
    bindings.insert_permanent_list("__paid_prevention_candidates",targets);
    let selection_filter=CompiledPredicate{not:Some(Box::new(CompiledPredicate{not_in_binding:Some("__paid_prevention_candidates".into()),..Default::default()})),..Default::default()};
    let finalizer=CompiledStep::PreventDeletionMemberFinalize{binding:"__paid_prevention_target".into(),members:members.to_vec(),filter:filter.clone(),cause,cause_player};
    let choice=CompiledStep::SelectOwnPermanent {filter:selection_filter,bind_as:Some("__paid_prevention_target".into()),selector:None,
        prompt:"Choose which Digimon's deletion to prevent".into(),prompt_key:None,optional:false,continue_on_decline:false,then:vec![finalizer]};
    super::run_steps_with_runtime(&[choice],ctx,&mut bindings,&super::StepRuntime::default());
}

fn take_paid_receipt(ctx:&mut EffectContext<'_>,receipt:u64,payer:(u8,u16,u64))->bool {
    ctx.game.take_deletion_receipt_matching(receipt,payer.0,payer.2,Some(payer.1))
}
fn paid_target_matches(ctx:&EffectContext<'_>,h:PermanentHandle,filter:&digimon_dsl::compiled::CompiledPredicate,cause:digimon_dsl::compiled::CompiledReplacementCause,cause_player:Option<u8>)->bool {
    use digimon_dsl::compiled::CompiledReplacementCause as C;
    use crate::replacement::ReplacementCause as R;
    let cause=match cause {C::Battle=>R::Battle,C::OwnEffect=>R::OwnEffect,C::OpponentEffect=>R::OpponentEffect,
        C::SecurityCheck=>R::SecurityCheck,C::Cost=>R::Cost,C::Overclock=>R::Overclock};
    // The payer is gone. Never alias its previous slot to a compacted victim.
    let read=crate::effect_context::EffectReadContext::new(ctx.game,ctx.source_card,None,ctx.player)
        .with_replacement_context(crate::replacement::deletion_cause_for_controller(cause,cause_player,h.player),cause_player,Some(h.player));
    crate::dsl_cards::predicate::eval_predicate(filter,&read,crate::dsl_cards::predicate::PredicateSubject::Permanent(h))
}
