//! Versioned projection of outstanding security-play results.
use super::{debug_field, Sha256};
use crate::{game::{Game, PendingWouldPlayOrigin}, card_source::CardHandle};
use digimon_dsl::compiled::CompiledStep;
#[derive(Debug)]
pub(super) enum Reference { Live(usize), Missing(u64) }
#[derive(Debug)]
pub(super) struct StepReference { operation: &'static str, reference: Reference, player: u8, card: u16 }
#[derive(Debug)]
pub(super) struct OriginReference { reference: Option<Reference>, player: u8, card: Option<CardHandle>, index: usize, was_face_up: bool, pub(super) context: Option<String> }
fn reference(g:&Game,id:u64,player:u8,card:Option<CardHandle>)->Reference {
    g.security_play_receipts.iter().position(|r|r.id==id && r.player==player && card.is_none_or(|card|card==r.card))
        .map(Reference::Live).unwrap_or(Reference::Missing(id))
}
pub(super) fn origin(g:&Game,value:PendingWouldPlayOrigin,player:u8,card:Option<CardHandle>)->Option<OriginReference> {
    let (id,index,was_face_up)=match value {
        PendingWouldPlayOrigin::SecurityRecorded { receipt,index,was_face_up } => (Some(receipt),index,was_face_up),
        PendingWouldPlayOrigin::SecurityAt { index,was_face_up } => (None,index,was_face_up),
        _=>return None,
    };
    Some(OriginReference{reference:id.map(|id|reference(g,id,player,card)),player,card,index,was_face_up,context:None})
}
pub(super) fn is_runtime_step(step:&CompiledStep)->bool { matches!(step,CompiledStep::SecurityPlayCaptured{..}|CompiledStep::SecurityPlayFinalize{..}) }
pub(super) fn is_runtime_value(value:&serde_json::Value)->bool { value.get("SecurityPlayCaptured").is_some() || value.get("SecurityPlayFinalize").is_some() }
pub(super) fn step(g:&Game,value:&serde_json::Value)->Option<StepReference> {
    for operation in ["SecurityPlayCaptured","SecurityPlayFinalize"] {
        if let Some(value)=value.get(operation) {
            let id=value.get("receipt")?.as_u64()?;
            let player=u8::try_from(value.get("player")?.as_u64()?).ok()?;
            let card=u16::try_from(value.get("card")?.as_u64()?).ok()?;
            return Some(StepReference{operation,reference:reference(g,id,player,Some(CardHandle(card))),player,card});
        }
    }
    None
}
pub(super) fn write(h:&mut Sha256,g:&Game) {
    let refs=super::cost_receipt_hash::security_references(g);
    if g.security_play_receipts.is_empty() && refs.is_empty() {return;}
    // Allocation IDs are correlated through live-table ordinals, not history.
    let rows:Vec<_>=g.security_play_receipts.iter().map(|r|(r.player,r.card,r.original_index,r.committed)).collect();
    debug_field(h,"security_play_receipts",&rows);
    debug_field(h,"security_play_references",&refs);
}

#[cfg(test)]
mod tests {
    use crate::{card_source::CardHandle,debug_runner::DebugRunner,game::{Game,PendingWouldPlayOrigin as O,PendingWouldPlayResume,SecurityPlayReceipt},recorder::VerificationDigestVersion as V,resume::{ResumeStack,ResumeFrame,ResumeProvenance,ResumeSelectKind,ResumeDecline}};
    use digimon_dsl::compiled::CompiledStep as S;
    use std::sync::Arc;
    fn receipt(g:&mut Game,id:u64) { g.security_play_receipts.push(SecurityPlayReceipt{id,player:0,card:CardHandle(1),original_index:1,committed:None}); }
    fn frame(steps:Vec<S>)->ResumeStack { ResumeStack{frames:vec![ResumeFrame::RunTail{
        prov:ResumeProvenance{source_card:CardHandle(0),source_permanent:None,source_kind:crate::enums::EffectSourceKind::Digimon,controller:0,override_pin:None},
        select_kind:ResumeSelectKind::Hand{of_player:0},bind_as:None,inner_tail:Arc::new(steps),outer_conts:vec![],bindings:Default::default(),runtime:Default::default(),trigger_context:None,decline:ResumeDecline::None}]}}
    fn pending(g:&mut Game,id:u64) { g.pending_would_play_resume=Some(PendingWouldPlayResume{player:0,card:CardHandle(1),effective_cost:0,origin:O::SecurityRecorded{was_face_up:true,index:1,receipt:id},effect_initiated:true,play_controller:0,suppress_on_play:false}); }
    fn runtime(id:u64)->Vec<S> { vec![S::SecurityPlayCaptured{receipt:id,player:0,card:1},S::SecurityPlayFinalize{receipt:id,player:0,card:1},S::CompleteCostPayment{receipt:77}] }
    #[test]
    fn security_receipt_store_is_v4_only_and_counter_is_not_state() {
        let base=DebugRunner::builder().start().game;
        let mut g=base.clone();receipt(&mut g,8);
        assert_ne!(g.verification_digest(),base.verification_digest());
        for version in [V::LegacyV2,V::V3] {assert_eq!(g.verification_digest_with_version(version),base.verification_digest_with_version(version));}
        let mut committed=g.clone();committed.security_play_receipts[0].committed=Some((crate::permanent::PermanentHandle{player:0,index:0},99));
        assert_ne!(g.verification_digest(),committed.verification_digest());
        assert_eq!(committed.verification_digest(),committed.clone().verification_digest());
        committed.security_play_receipts.clear();committed.next_security_play_receipt=900;
        for version in [V::LegacyV2,V::V3,V::V4] {assert_eq!(committed.verification_digest_with_version(version),base.verification_digest_with_version(version));}
    }
    #[test]
    fn security_origin_keeps_archival_debug_and_hashes_current_correlation() {
        let base=DebugRunner::builder().start().game;
        let mut recorded=base.clone();pending(&mut recorded,8);receipt(&mut recorded,8);
        let mut archival=recorded.clone();archival.pending_would_play_resume.as_mut().unwrap().origin=O::SecurityTop{was_face_up:true};
        assert_eq!(format!("{:?}",archival.pending_would_play_resume),format!("{:?}",recorded.pending_would_play_resume));
        for version in [V::LegacyV2,V::V3] {assert_eq!(archival.verification_digest_with_version(version),recorded.verification_digest_with_version(version));}
        let mut wrong=recorded.clone();wrong.pending_would_play_resume.as_mut().unwrap().origin=O::SecurityRecorded{was_face_up:true,index:1,receipt:9};
        assert_ne!(recorded.verification_digest(),wrong.verification_digest());
        let mut wrong_index=recorded.clone();wrong_index.pending_would_play_resume.as_mut().unwrap().origin=O::SecurityRecorded{was_face_up:true,index:2,receipt:8};
        assert_ne!(recorded.verification_digest(),wrong_index.verification_digest());
    }
    #[test]
    fn security_runtime_steps_preserve_legacy_tail_and_cost_reference_positions() {
        let mut base=DebugRunner::builder().start().game;receipt(&mut base,8);
        base.pending_selection_resume=Some(frame(vec![S::CompleteCostPayment{receipt:77}]));
        let mut added=base.clone();added.pending_selection_resume=Some(frame(runtime(8)));
        for version in [V::LegacyV2,V::V3] {assert_eq!(base.verification_digest_with_version(version),added.verification_digest_with_version(version));}
        assert_ne!(base.verification_digest(),added.verification_digest());
    }
    #[test]
    fn security_active_receipt_alpha_rename_preserves_v4_but_wrong_reference_does_not() {
        let mut base=DebugRunner::builder().start().game;receipt(&mut base,8);pending(&mut base,8);
        base.pending_selection_resume=Some(frame(runtime(8)));
        let mut renamed=base.clone();renamed.security_play_receipts[0].id=100;renamed.next_security_play_receipt=101;pending(&mut renamed,100);
        renamed.pending_selection_resume=Some(frame(runtime(100)));
        assert_eq!(base.verification_digest(),renamed.verification_digest());
        let mut wrong=renamed.clone();let ResumeFrame::RunTail{inner_tail,..}=&mut wrong.pending_selection_resume.as_mut().unwrap().frames[0] else {panic!()};
        if let S::SecurityPlayFinalize{receipt,..}=&mut Arc::make_mut(inner_tail)[1] {*receipt=102;}else{panic!()}
        assert_ne!(renamed.verification_digest(),wrong.verification_digest());
    }
    #[test]
    fn raw_security_origin_index_is_current_state_but_archival_projection_is_stable() {
        let mut base=DebugRunner::builder().start().game;pending(&mut base,8);
        base.pending_would_play_resume.as_mut().unwrap().origin=O::SecurityTop{was_face_up:true};
        let mut middle=base.clone();middle.pending_would_play_resume.as_mut().unwrap().origin=O::SecurityAt{was_face_up:true,index:1};
        for version in [V::LegacyV2,V::V3] {assert_eq!(base.verification_digest_with_version(version),middle.verification_digest_with_version(version));}
        let mut other=middle.clone();other.pending_would_play_resume.as_mut().unwrap().origin=O::SecurityAt{was_face_up:true,index:2};
        assert_ne!(middle.verification_digest(),other.verification_digest());
    }
    #[test]
    fn security_steps_in_global_and_nested_outer_tails_are_versioned() {
        for nested in [false,true] {
            let mut base=DebugRunner::builder().start().game;receipt(&mut base,8);
            let original=vec![S::CompleteCostPayment{receipt:77}];
            if nested {
                let mut stack=frame(vec![]);
                let ResumeFrame::RunTail{outer_conts,..}=&mut stack.frames[0] else {panic!()};
                outer_conts.push(crate::resume::OuterContinuation{tail:Arc::new(original),bindings:Default::default(),runtime:Default::default(),trigger_context:None,source_card:CardHandle(0),source_permanent:None,player:0});
                base.pending_selection_resume=Some(stack);
            } else {base.dsl_outer_tail=Some((original,Default::default(),Default::default()));}
            let mut added=base.clone();
            if nested {
                let ResumeFrame::RunTail{outer_conts,..}=&mut added.pending_selection_resume.as_mut().unwrap().frames[0] else {panic!()};
                outer_conts[0].tail=Arc::new(runtime(8));
            } else {added.dsl_outer_tail.as_mut().unwrap().0=runtime(8);}
            for version in [V::LegacyV2,V::V3] {assert_eq!(base.verification_digest_with_version(version),added.verification_digest_with_version(version));}
            assert_ne!(base.verification_digest(),added.verification_digest());
        }
    }

}
