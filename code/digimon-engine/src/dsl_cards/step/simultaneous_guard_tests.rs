use super::*;
use crate::{
    action::build_action_mask, card_source::CardHandle,
    debug_runner::{DebugRunner, make_test_card},
    dsl_cards::raw_rust::EngineRawRustRegistry,
    trigger_context::TriggerContext,
};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn group(bits:u8)->TriggerContext {
    let mut result=TriggerContext::default();
    result.security_check_face_generation=Some(90);
    result.projected_reference_reads_guarded=bits&1!=0;
    for index in 0..2 {
        let mut member=TriggerContext::default();
        member.security_check_face_generation=Some(10+index);
        member.projected_reference_reads_guarded=bits&(2<<index)!=0;
        if index==1 {
            let mut deep=TriggerContext::default();
            deep.projected_reference_reads_guarded=bits&8!=0;
            member.simultaneous_play_members.push(deep);
        }
        result.simultaneous_play_members.push(member);
    }
    result
}
fn raw(name:&str)->CompiledStep {
    CompiledStep::RawRust{fn_name:name.into(),consumes:vec![],binds:vec![]}
}
fn setup(player:u8,bits:u8,index:usize,parked:bool)
    ->(DebugRunner,CardHandle,Bindings,StepRuntime,Vec<CompiledStep>,Arc<AtomicUsize>,Arc<AtomicUsize>) {
    let mut r=DebugRunner::builder().add_card(make_test_card("DS","Source"))
        .hand(player,&["DS"]).deck(0,&["DS";20]).deck(1,&["DS";20]).memory(2).start();
    r.skip_mulligan();
    let source=r.game.players[player as usize].hand[0].handle();
    r.game.current_trigger_context=Some(group(bits));
    let inner=Arc::new(AtomicUsize::new(0));let outer=Arc::new(AtomicUsize::new(0));
    let mut registry=EngineRawRustRegistry::new();
    let count=inner.clone();
    registry.register_step("inner",move |ctx,b| {
        count.fetch_add(1,Ordering::SeqCst);
        assert_eq!(ctx.player,player);
        assert_eq!(ctx.game.current_trigger_context.as_ref().unwrap().security_check_face_generation,Some(10+index as u64));
        assert!(b.get_ref("subject").is_none());
        if parked {assert!(matches!(b.get_ref("choice"),Some(BindingValue::Literal(0|1))));}
        ctx.game.memory+=4;
    });
    let count=outer.clone();
    registry.register_step("outer",move |ctx,_| {
        count.fetch_add(1,Ordering::SeqCst);
        assert_eq!(ctx.game.current_trigger_context.as_ref().unwrap().security_check_face_generation,Some(90));
        ctx.game.memory+=1;
    });
    let mut body=vec![];
    if parked {body.push(CompiledStep::SelectEffectChoice {
        labels:vec!["A".into(),"B".into()],legal_when:None,bind_as:Some("choice".into()),
        prompt:"Choose".into(),prompt_key:None,
    });}
    body.push(raw("inner"));
    let steps=vec![CompiledStep::WithSimultaneousPlaySubject{binding:"subject".into(),body},raw("outer")];
    let mut bindings=Bindings::new();bindings.insert_literal("subject",index as i64);
    (r,source,bindings,StepRuntime::new(Arc::new(registry)),steps,inner,outer)
}

#[test]
fn subject_transfer_rejects_raw_reader_before_body_for_every_marker_position() {
    for player in [0,1] {for bits in 0..16 {for index in 0..2 {
        let (mut r,source,mut bindings,runtime,steps,inner,outer)=setup(player,bits,index,false);
        let before=format!("{:?}",r.game.current_trigger_context);
        // Marked transfer is exercised below the now-strict runner entry.
        // This still must inherit every parent/sibling/deep obligation before
        // the nested runner can see the selected member.
        let result=catch_unwind(AssertUnwindSafe(|| {
            let mut ctx=EffectContext::new(&mut r.game,source,None,player);
            if bits!=0 {try_run(&steps[0],&mut ctx,&mut bindings,&runtime).expect("subject step")}
            else {run_steps_with_runtime(&steps,&mut ctx,&mut bindings,&runtime)}
        }));
        if bits!=0 {
            assert!(result.is_err(),"bits {bits}, selected {index}: must reject before raw body");
            assert_eq!(inner.load(Ordering::SeqCst),0);
            assert_eq!(outer.load(Ordering::SeqCst),0);
            assert_eq!(r.game.memory,2);
            let expected=crate::determinize::context_guard::inherited_member_context(&group(bits),&group(bits).simultaneous_play_members[index]);
            assert_eq!(format!("{:?}",r.game.current_trigger_context.as_ref().unwrap()),format!("{:?}",expected));
        } else {
            assert!(matches!(result.unwrap(),RunOutcome::Synchronous));
            assert_eq!(inner.load(Ordering::SeqCst),1);
            assert_eq!(outer.load(Ordering::SeqCst),1);
            assert_eq!(r.game.memory,7);
            assert_eq!(format!("{:?}",r.game.current_trigger_context),before);
        }
        assert!(bindings.get_ref("subject").is_none());
    }}}
}

// Test-only marker injection: preserve every captured payload field and shape.
fn copy_marker_bits(target:&mut TriggerContext,source:&TriggerContext) {
    assert_eq!(target.simultaneous_play_members.len(),source.simultaneous_play_members.len());
    target.projected_reference_reads_guarded=source.projected_reference_reads_guarded;
    for (to,from) in target.simultaneous_play_members.iter_mut().zip(&source.simultaneous_play_members) {
        copy_marker_bits(to,from);
    }
}

#[test]
fn subject_transfer_parked_clones_reject_late_saved_markers_from_unmarked_caller_before_raw_tail() {
    for player in [0,1] {for bits in 0..16 {for index in 0..2 {
        let (mut r,source,mut bindings,runtime,steps,inner,outer)=setup(player,0,index,true);
        let before=format!("{:?}",r.game.current_trigger_context);
        let outcome=run_steps_with_runtime(&steps,
            &mut EffectContext::new(&mut r.game,source,None,player),&mut bindings,&runtime);
        assert!(matches!(outcome,RunOutcome::Parked));
        assert_eq!(format!("{:?}",r.game.current_trigger_context),before,"restore enclosing group after park");
        assert!(r.game.pending_selection_resume.is_some(),"exercise structured resume, not raw callback");
        assert_eq!(inner.load(Ordering::SeqCst),0);
        assert_eq!(outer.load(Ordering::SeqCst),0);
        let actions=r.game.pending_selection.as_ref().unwrap().valid_action_ids.clone();
        assert_eq!(actions.len(),2);
        // The strict runner cannot park an already-marked context. Start
        // with the actual unmarked VM prompt, then introduce only marker bits
        // into its captured contexts to test a later obligation across clones.
        let marked_group=group(bits);
        let selected=crate::determinize::context_guard::inherited_member_context(&marked_group,&marked_group.simultaneous_play_members[index]);
        let mut frames=0;let mut outers=0;
        for frame in &mut r.game.pending_selection_resume.as_mut().unwrap().frames {
            let crate::resume::ResumeFrame::RunTail{trigger_context,outer_conts,..}=frame else {panic!("expected actual RunTail");};
            copy_marker_bits(trigger_context.as_mut().unwrap(),&selected);frames+=1;
            for cont in outer_conts {
                copy_marker_bits(cont.trigger_context.as_mut().unwrap(),&marked_group);outers+=1;
            }
        }
        assert_eq!(frames,1);assert_eq!(outers,1);
        let parked=r.game.clone();
        for _copy in 0..2 {for action in actions.iter().copied() {
            let mut game=parked.clone();
            let mut caller=TriggerContext::default();caller.security_check_face_generation=Some(99);
            game.current_trigger_context=Some(caller);
            assert_eq!(build_action_mask(&game,player)[action as usize],1.0);
            let inner_before=inner.load(Ordering::SeqCst);let outer_before=outer.load(Ordering::SeqCst);
            let result=catch_unwind(AssertUnwindSafe(||game.resolve_selection(player,action)));
            if bits!=0 {
                assert!(result.is_err(),"bits {bits}, selected {index}: restored tail must reject");
                assert_eq!(inner.load(Ordering::SeqCst),inner_before,"no partial inner execution");
                assert_eq!(outer.load(Ordering::SeqCst),outer_before,"no outer continuation execution");
                assert_eq!(game.memory,2);
            } else {
                assert!(result.unwrap().is_ok());
                assert_eq!(inner.load(Ordering::SeqCst),inner_before+1);
                assert_eq!(outer.load(Ordering::SeqCst),outer_before+1);
                assert_eq!(game.memory,7);
                assert!(game.pending_selection.is_none());
                assert!(game.pending_selection_resume.is_none());
                assert_eq!(game.current_trigger_context.as_ref().unwrap().security_check_face_generation,Some(99));
            }
        }}
    }}}
}

#[test]
fn subject_transfer_invalid_binding_preserves_context_and_binding_without_body() {
    for player in [0,1] {for bits in 0..16 {
        for value in [None,Some(BindingValue::Literal(-1)),Some(BindingValue::Literal(2)),
            Some(BindingValue::Literal(i64::MAX)),Some(BindingValue::Card(CardHandle(999)))] {
            let (mut r,source,mut bindings,runtime,steps,inner,outer)=setup(player,bits,0,false);
            bindings.remove("subject");
            if let Some(v)=value.clone(){bindings.insert("subject",v);}
            let before=format!("{:?}",r.game.current_trigger_context);
            let outcome=try_run(&steps[0],&mut EffectContext::new(&mut r.game,source,None,player),&mut bindings,&runtime);
            assert!(matches!(outcome,Some(RunOutcome::Synchronous)));
            assert_eq!(bindings.get_ref("subject"),value.as_ref());
            assert_eq!(format!("{:?}",r.game.current_trigger_context),before);
            assert_eq!(inner.load(Ordering::SeqCst),0);assert_eq!(outer.load(Ordering::SeqCst),0);
            assert_eq!(r.game.memory,2);assert!(r.game.pending_selection.is_none());
        }
    }}
}
