//! Fixed native constructor coverage; no runtime-admission exemption.
use super::{keyword_to_auto_effect, keyword_to_auto_effect_inner};
use crate::{
    card_source::CardHandle,
    debug_runner::DebugRunner,
    effect::Effect,
    effect_context::{EffectContext, EffectReadContext},
    enums::Keyword,
    game::Game,
    permanent::PermanentHandle,
    selection::SourceSelectionRef,
    replacement::{ReplacementCause, ReplacementContext, ReplacementOutcome, ReplacementSubject},
    trigger_context::TriggerContext,
};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn keywords(exhaustive: bool) -> Vec<Keyword> {
    let mut out = vec![Keyword::Blocker, Keyword::Rush, Keyword::Jamming, Keyword::Piercing, Keyword::Reboot, Keyword::Blitz, Keyword::Raid, Keyword::Alliance, Keyword::BlastDigivolve, Keyword::Save, Keyword::Fortitude, Keyword::Overclock, Keyword::Barrier, Keyword::Partition, Keyword::Vortex, Keyword::Collision, Keyword::Evade, Keyword::Decode, Keyword::ArmorPurge, Keyword::Progress, Keyword::Retaliation, Keyword::Scapegoat, Keyword::Execute, Keyword::Engage, Keyword::Guard, Keyword::Iceclad, Keyword::MindLink, Keyword::Training, Keyword::ArtsDigivolve, Keyword::Ascension];
    if exhaustive { out.extend((i8::MIN..=i8::MAX).map(Keyword::SecurityAttackPlus)); } else { out.extend([i8::MIN, 1, i8::MAX].into_iter().map(Keyword::SecurityAttackPlus)); }
    if exhaustive { out.extend((i8::MIN..=i8::MAX).map(Keyword::SecurityAttackMinus)); } else { out.extend([i8::MIN, 1, i8::MAX].into_iter().map(Keyword::SecurityAttackMinus)); }
    if exhaustive { out.extend((u8::MIN..=u8::MAX).map(Keyword::DeDigivolve)); } else { out.extend([u8::MIN, 1, u8::MAX].into_iter().map(Keyword::DeDigivolve)); }
    if exhaustive { out.extend((u8::MIN..=u8::MAX).map(Keyword::DrawX)); } else { out.extend([u8::MIN, 1, u8::MAX].into_iter().map(Keyword::DrawX)); }
    if exhaustive { out.extend((u8::MIN..=u8::MAX).map(Keyword::MaterialSave)); } else { out.extend([u8::MIN, 1, u8::MAX].into_iter().map(Keyword::MaterialSave)); }
    if exhaustive { out.extend((u8::MIN..=u8::MAX).map(Keyword::DigiBurst)); } else { out.extend([u8::MIN, 1, u8::MAX].into_iter().map(Keyword::DigiBurst)); }
    if exhaustive { out.extend((u8::MIN..=u8::MAX).map(Keyword::Digisorption)); } else { out.extend([u8::MIN, 1, u8::MAX].into_iter().map(Keyword::Digisorption)); }
    if exhaustive { out.extend((u8::MIN..=u8::MAX).map(Keyword::Decoy)); } else { out.extend([u8::MIN, 1, u8::MAX].into_iter().map(Keyword::Decoy)); }
    if exhaustive { out.extend((u8::MIN..=u8::MAX).map(Keyword::Fragment)); } else { out.extend([u8::MIN, 1, u8::MAX].into_iter().map(Keyword::Fragment)); }
    out
}
fn expected_count(keyword: Keyword) -> usize {
    // No wildcard: a newly added enum variant must receive an explicit review.
    match keyword {
        Keyword::Execute => 2,
        Keyword::Alliance | Keyword::ArmorPurge | Keyword::Ascension | Keyword::Barrier | Keyword::BlastDigivolve | Keyword::Blitz | Keyword::Decoy(_) | Keyword::Engage | Keyword::Evade | Keyword::Fortitude | Keyword::Fragment(_) | Keyword::Guard | Keyword::MaterialSave(_) | Keyword::MindLink | Keyword::Overclock | Keyword::Partition | Keyword::Piercing | Keyword::Raid | Keyword::Retaliation | Keyword::Save | Keyword::Scapegoat | Keyword::Training | Keyword::Vortex => 1,
        Keyword::Blocker | Keyword::SecurityAttackPlus(_) | Keyword::SecurityAttackMinus(_) | Keyword::Rush | Keyword::Jamming | Keyword::Reboot | Keyword::DeDigivolve(_) | Keyword::DrawX(_) | Keyword::DigiBurst(_) | Keyword::Digisorption(_) | Keyword::Collision | Keyword::Decode | Keyword::Progress | Keyword::Iceclad | Keyword::ArtsDigivolve => 0,
    }
}
fn callback_shape(e: &Effect) -> [bool; 16] {
    [e.condition.is_some(),e.trigger_qualifier.is_some(),e.outer_optional_guard.is_some(),
     e.option_color_requirement_bypass.is_some(),e.cost_reduction_fn.is_some(),
     e.use_cost_increase_fn.is_some(),e.replacement_condition.is_some(),
     e.dp_modifier_fn.is_some(),e.security_attack_fn.is_some(),e.link_filter.is_some(),
     e.succession_filter.is_some(),e.overclock_cost_filter.is_some(),
     e.process.is_some(),e.pay_cost_fn.is_some(),e.activation_cost_fn.is_some(),
     e.replacement_process.is_some()]
}
#[test]
fn every_keyword_payload_constructs_and_preserves_reviewed_metadata() {
    let values=keywords(true);
    assert_eq!(values.len(),2334);
    let mut cases=0;let mut effects=0;
    for keyword in values { for card in [CardHandle(0),CardHandle(42),CardHandle(u16::MAX)] {
        let original=keyword_to_auto_effect_inner(keyword,card);
        let guarded=keyword_to_auto_effect(keyword,card);
        assert_eq!(original.len(),expected_count(keyword),"{keyword:?}");
        assert_eq!(guarded.len(),original.len());
        for (a,b) in original.iter().zip(&guarded) {
            assert_eq!(a.source_card,card);assert_eq!(b.source_card,card);
            assert_eq!(a.keyword_source,None);assert_eq!(b.keyword_source,Some(keyword));
            assert_eq!(a.name,b.name);assert_eq!(a.timing,b.timing);
            assert_eq!(a.optional,b.optional);assert_eq!(a.declarative,b.declarative);
            assert_eq!(a.blast_digivolve,b.blast_digivolve);
            assert_eq!(a.paid_self_prevention,b.paid_self_prevention);
            assert_eq!(a.paid_prevention_all,b.paid_prevention_all);
            assert_eq!(a.paid_prevention_filter,b.paid_prevention_filter);
            assert_eq!(a.granted_keyword,b.granted_keyword);
            assert_eq!(callback_shape(a),callback_shape(b));
            assert!(!b.ignores_event_card_identity, "construction is not a liveness certificate");
            effects+=1;
        }
        cases+=1;
    } }
    assert_eq!(cases,7002);assert_eq!(effects,2370);
    println!("constructor_cases={cases} returned_effects={effects} all39variants all_payload_values");
}
fn reject<T>(run:impl FnOnce()->T) {
    let error=match catch_unwind(AssertUnwindSafe(run)) {
        Err(error)=>error,Ok(_)=>panic!("guarded native keyword callback entered its body"),
    };
    let message=error.downcast_ref::<String>().map(String::as_str)
        .or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("");
    assert!(message.contains("closed-list projected context reached unproven callback"),"{message}");
}
fn mark(game:&mut Game,mode:u8) {
    if mode==0 {game.require_projected_world_read_guard_for_test();return;}
    let mut ctx=TriggerContext::default();ctx.projected_reference_reads_guarded=true;
    if mode==2 {
        let mut parent=TriggerContext::default();parent.simultaneous_play_members.push(ctx);
        let mut outer=TriggerContext::default();outer.simultaneous_play_members.push(parent);ctx=outer;
    }
    game.current_trigger_context=Some(ctx);
}
#[test]
fn every_emitted_keyword_callback_rejects_world_direct_and_nested_contexts() {
    let mut cases=0;let mut invocations=0;
    for keyword in keywords(false) {for card in [CardHandle(0),CardHandle(u16::MAX)] {
        // Factory constructs before any Game exists; returned callbacks retained.
        let effects=keyword_to_auto_effect(keyword,card);
        for seat in 0..2 {for mode in 0..3 {
            let mut game=DebugRunner::builder().memory(5).start().game;
            mark(&mut game,mode);
            let before=(game.memory,game.current_phase,game.pending_selection.is_some());
            let permanent=PermanentHandle{player:seat,index:255};
            let source=SourceSelectionRef{permanent,field_index:0,source_index:0,card};
            for e in &effects {
                let read=EffectReadContext::new(&game,card,None,seat);
                macro_rules! one {($($field:ident),+) => {$(
                    if let Some(f)=&e.$field {reject(||f(&read));invocations+=1;}
                )+};}
                macro_rules! two {($field:ident,$arg:expr) => {
                    if let Some(f)=&e.$field {reject(||f(&read,$arg));invocations+=1;}
                };}
                one!(condition,trigger_qualifier,outer_optional_guard,
                    option_color_requirement_bypass,cost_reduction_fn,use_cost_increase_fn);
                two!(replacement_condition,&ReplacementSubject::Player(seat));
                two!(dp_modifier_fn,permanent);two!(security_attack_fn,permanent);
                two!(link_filter,permanent);two!(succession_filter,source);two!(overclock_cost_filter,permanent);
                let mut ctx=EffectContext::new(&mut game,card,None,seat);
                if let Some(f)=&e.process {reject(||f(&mut ctx));invocations+=1;}
                if let Some(f)=&e.pay_cost_fn {reject(||f(&mut ctx));invocations+=1;}
                if let Some(f)=&e.activation_cost_fn {reject(||f(&mut ctx));invocations+=1;}
                let mut replacement=ReplacementContext{effect:&mut ctx,cause:ReplacementCause::OwnEffect,
                    subject:ReplacementSubject::Player(seat),original_destination:None,
                    outcome:ReplacementOutcome::None,cancel_if_deleted:None};
                if let Some(f)=&e.replacement_process {reject(||f(&mut replacement));invocations+=1;}
            }
            assert_eq!((game.memory,game.current_phase,game.pending_selection.is_some()),before);
            cases+=1;
        }}
    }}
    assert_eq!(cases,684);assert!(invocations>0);
    println!("guard_factory_cases={cases} callback_invocations={invocations} all39variants payload_min_one_max");
}
