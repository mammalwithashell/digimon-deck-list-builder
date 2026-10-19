//! Runtime expansion of source plays into cloneable, individually parked steps.
use digimon_dsl::compiled::{CompiledStep};
use crate::dsl_cards::{bindings::Bindings,binding_ref::{resolve_binding_ref,ResolvedBinding}};
use crate::effect_context::EffectContext;
use crate::card_source::CardHandle;
use crate::permanent::PermanentHandle;
use super::{StepRuntime,run_steps_with_runtime};

pub(super) fn try_run(step:&CompiledStep,ctx:&mut EffectContext<'_>,bindings:&mut Bindings,runtime:&StepRuntime)->bool {
    match step {
        CompiledStep::PlayFromMaterials{..}=>{
            if let Some(expanded)=expand(step,ctx,bindings){run_steps_with_runtime(&expanded,ctx,bindings,runtime);}
            true
        }
        CompiledStep::SourcePlayCaptured{player,host_identity,breeding,card,receipt,cost_delta,suppress_on_play,suspended}=>{
            let captured=PermanentHandle{player:*player,index:if *breeding{crate::action::space::BREEDING_TARGET as u8}else{0}};
            let host=ctx.game.resolve_source_play_host_identity(captured,*host_identity);
            let Some(host)=host else{return true;};
            let Some(index)=crate::effect_context::material_carrier_permanent(ctx.game,host).and_then(|p|p.card_sources.iter().take(p.card_sources.len().saturating_sub(1)).position(|c|c.handle()==CardHandle(*card))) else{return true;};
            let delta=super::play_digivolve::lower_cost_delta(cost_delta.as_ref(),ctx,bindings);
            ctx.game.source_play_receipts.push((*receipt,CardHandle(*card),None));
            ctx.play_from_materials_with_entry_receipt(host,index,delta,*suppress_on_play,*suspended,Some(*receipt));
            true
        }
        CompiledStep::SourcePlayBatchCommit { receipts } => {
            ctx.game.commit_prepared_source_plays(receipts);
            true
        }
        CompiledStep::SourcePlayFinalize{receipt,card,bind_as}=>{
            ctx.game.commit_prepared_source_play(*receipt);
            if let Some(index)=ctx.game.source_play_receipts.iter().position(|r|r.0==*receipt){
                let (_,_,committed)=ctx.game.source_play_receipts.remove(index);
                if let Some((played,identity))=committed{
                    bindings.record_played(played);
                    if let Some(name)=bind_as{bindings.insert_played_permanent_with_identity(name,crate::trigger_context::ProvenanceToken::from(CardHandle(*card)),played,identity);}
                }
            }
            true
        }
        _=>false,
    }
}

pub(super) fn expand(step:&CompiledStep,ctx:&mut EffectContext<'_>,bindings:&Bindings)->Option<Vec<CompiledStep>> {
    let CompiledStep::PlayFromMaterials{target,source_index,cost_delta,bind_as,suppress_on_play,suspended}=step else{return None;};

            let Some(ResolvedBinding::Permanent(host))=resolve_binding_ref(target,ctx,bindings) else{return Some(Vec::new());};
            let Some(carrier)=crate::effect_context::material_carrier_permanent(ctx.game,host) else{return Some(Vec::new());};
            let cards=match resolve_binding_ref(source_index,ctx,bindings){
                Some(ResolvedBinding::CardList(v))=>v,
                Some(ResolvedBinding::Card(h))=>vec![h],
                Some(ResolvedBinding::Literal(i)) if i>=0=>carrier.card_sources.get(i as usize).map(|c|vec![c.handle()]).unwrap_or_default(),
                _=>return Some(Vec::new()),
            };
            let breeding=host.index==crate::action::space::BREEDING_TARGET as u8;
            let identity=ctx.game.capture_source_play_host_identity(host)?;
            let mut expanded=Vec::new();
            let mut finalizers=Vec::new();
            let mut receipts=Vec::new();
            for card in cards{
                let receipt=ctx.game.next_source_play_receipt;
                ctx.game.next_source_play_receipt=receipt.checked_add(1).expect("source play receipt exhausted");
                expanded.push(CompiledStep::SourcePlayCaptured{player:host.player,host_identity:identity,breeding,card:card.0,receipt,cost_delta:cost_delta.clone(),suppress_on_play:*suppress_on_play,suspended:*suspended});
                ctx.game.prepared_source_plays.push((receipt,None));
                receipts.push(receipt);
                finalizers.push(CompiledStep::SourcePlayFinalize{receipt,card:card.0,bind_as:bind_as.clone()});
            }
            expanded.push(CompiledStep::SourcePlayBatchCommit { receipts });
            expanded.extend(finalizers);
            Some(expanded)

}
