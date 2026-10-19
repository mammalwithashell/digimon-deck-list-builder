// Included in replacement.rs. One immediate window owns the entire deletion event.
#[derive(Debug, Clone)]
struct DeletionWindowMember {
    original: crate::permanent::PermanentHandle,
    identity: (PlayerId, CardHandle, u64),
    prevented: bool,
    terminal: bool,
    substituted: bool,
}
#[derive(Clone)]
struct DeletionWindowBatch {
    members: Vec<DeletionWindowMember>,
    pending_subject: Option<(WindowCandidate, Vec<(u16, usize)>)>,
    substitution_depth: u8,
    initiating_source_permanent: Option<(PlayerId, u64)>,
    active_paid_all: Option<(digimon_dsl::compiled::CompiledPredicate,Vec<(PlayerId,u16,u64)>,CardHandle,PlayerId,Option<(PlayerId,u64)>)>,
}
/// Effect causes are relative to each affected controller, not the first
/// member of a mixed-controller batch. Non-effect causes retain their kind.
pub(crate) fn deletion_cause_for_controller(cause:ReplacementCause, actor:Option<PlayerId>, controller:PlayerId)->ReplacementCause {
    match (cause,actor) {
        (ReplacementCause::OwnEffect|ReplacementCause::OpponentEffect,Some(actor)) =>
            if actor==controller {ReplacementCause::OwnEffect}else{ReplacementCause::OpponentEffect},
        _=>cause,
    }
}
fn deletion_member_handle(game: &crate::game::Game, member: &DeletionWindowMember) -> Option<crate::permanent::PermanentHandle> {
    game.resolve_digivolve_host_identity(member.identity.0, member.identity.2)
}
fn deletion_member_candidates(game: &mut crate::game::Game, window: &ReplacementWindow, index: usize, eligibility: bool) -> Vec<Candidate> {
    let Some(member)=window.deletion_batch.as_ref().and_then(|b|b.members.get(index)) else{return Vec::new();};
    if member.terminal || (member.prevented && window.departure.as_ref().is_some_and(|d|d.route_stage)){return Vec::new();}
    let Some(handle)=deletion_member_handle(game,member) else{return Vec::new();};
    let old_player=game.effect_source_player;
    let old_context=game.current_trigger_context.clone();
    game.effect_source_player=window.cause_player;
    game.current_trigger_context=window.trigger_context.clone();
    game.tick_declarative_effects();
    let candidates=collect_event_candidates(game,window.timing,ReplacementSubject::Permanent(handle),deletion_cause_for_controller(window.cause,window.cause_player,handle.player),eligibility);
    game.effect_source_player=old_player;
    game.current_trigger_context=old_context;
    candidates
}
fn eligible_deletion_candidates(game: &mut crate::game::Game, window: &mut ReplacementWindow) -> Vec<(WindowCandidate,Vec<usize>)> {
    let mut grouped:Vec<(WindowCandidate,Vec<usize>)>=Vec::new();
    let count=window.deletion_batch.as_ref().unwrap().members.len();
    for index in 0..count {
        let candidates=deletion_member_candidates(game,window,index,true);
        if candidates.iter().any(|c|matches!(c.kind,CandidateKind::PassiveCancel)) {
            let member=&mut window.deletion_batch.as_mut().unwrap().members[index];
            member.prevented=true;member.terminal=true;
            continue;
        }
        for candidate in candidates {
            let Some(captured)=window.remaining.iter().find(|old|window_candidate_matches(game,old,&candidate)).cloned() else{continue;};
            if let Some((_,members))=grouped.iter_mut().find(|(old,_)|window_candidate_matches(game,old,&candidate)) {members.push(index);}
            else {grouped.push((WindowCandidate{candidate,host_identity:captured.host_identity},vec![index]));}
        }
    }
    grouped
}
fn paid_self_prevention_candidate(game:&crate::game::Game,c:&Candidate)->bool {
    match &c.kind {
        CandidateKind::EffectClosure{card_id,effect_slot}=>game.effects_for_card(card_id,c.source_card)
            .and_then(|effects|effects.get(*effect_slot as usize).map(|e|e.paid_self_prevention)).unwrap_or(false),
        CandidateKind::GrantedKeywordEffect{keyword,effect_slot}=>crate::cards::keyword_effects::keyword_to_auto_effect(*keyword,c.source_card)
            .get(*effect_slot as usize).is_some_and(|e|e.paid_self_prevention),
        CandidateKind::PassiveCancel=>false,
    }
}
/// Returns false only when the ordinary single-member/no-candidate path should run.
/// A true result transfers all members to this owned continuation, including on park.
pub(crate) fn start_deletion_replacement_batch(game:&mut crate::game::Game)->bool {
    let Some(batch)=game.active_deletion_batch.as_ref() else{return false;};
    if batch.kill_list.len()<2 || game.in_replacement_commit || game.replacement_depth>=MAX_REPLACEMENT_DEPTH{return false;}
    let cause=batch.cause;let mut handles=batch.kill_list.clone();
    handles.sort_by_key(|h|(h.player,h.index));handles.dedup();
    game.tick_declarative_effects();
    // No eligible effect means no identity allocation or new hidden continuation.
    if !handles.iter().any(|h|!collect_event_candidates(game,EffectTiming::WhenWouldBeDeleted,ReplacementSubject::Permanent(*h),deletion_cause_for_controller(cause,game.effect_source_player,h.player),true).is_empty()){return false;}
    let members:Vec<_>=handles.iter().filter_map(|h|capture_subject_card(game,ReplacementSubject::Permanent(*h))
        .map(|identity|DeletionWindowMember{original:*h,identity,prevented:false,terminal:false,substituted:false})).collect();
    if members.len()<2{return false;}
    let initiating_source_permanent=game.effect_source_permanent.and_then(|h| capture_subject_card(game,ReplacementSubject::Permanent(h))).map(|(player,_,identity)|(player,identity));
    let first=members[0].clone();
    let mut window=ReplacementWindow{
        timing:EffectTiming::WhenWouldBeDeleted,subject:ReplacementSubject::Permanent(first.original),stable_subject:Some(first.identity),
        cause,destination:Some(Zone::Trash),cause_player:game.effect_source_player,initiating_source_card:game.effect_source_card,
        effect_digivolve_card:None,play_declaration:None,event_cause:game.current_deletion_event_cause_override,
        trigger_context:game.current_trigger_context.clone(),deletion_receipt:game.current_deletion_receipt,turn_player:game.turn_player(),
        remaining:Vec::new(),cancelled:false,departure:None,deletion_batch:Some(DeletionWindowBatch{members,pending_subject:None,substitution_depth:0,initiating_source_permanent,active_paid_all:None}),
    };
    for i in 0..window.deletion_batch.as_ref().unwrap().members.len(){
        for c in deletion_member_candidates(game,&window,i,false){
            if !window.remaining.iter().any(|old|window_candidate_matches(game,old,&c)){
                window.remaining.push(capture_window_candidate(game,c));
            }
        }
    }
    // The synchronous caller still owns the returned batch summary. Parked
    // callbacks instead restore whatever unrelated deletion scope they interrupt.
    game.active_deletion_batch=None;
    advance_deletion_window(game,window,None,true);
    true
}
fn advance_deletion_window(game:&mut crate::game::Game,mut window:ReplacementWindow,chosen:Option<WindowCandidate>,retain_summary:bool){
    if game.game_over{return;}
    let eligible=eligible_deletion_candidates(game,&mut window);
    if eligible.is_empty(){finish_deletion_window(game,window,retain_summary);return;}
    let chooser=if eligible.iter().any(|(c,_)|c.candidate.source_controller==window.turn_player){window.turn_player}else{1-window.turn_player};
    let choices:Vec<_>=eligible.iter().filter(|(c,_)|c.candidate.source_controller==chooser).map(|(c,_)|c.clone()).collect();
    let explicit=chosen.is_some();
    let selected=chosen.and_then(|old|choices.iter().find(|c|window_candidate_matches(game,&old,&c.candidate)).cloned());
    if explicit && selected.is_none(){install_window_order(game,window,choices,0);return;}
    let selected=match selected {Some(c)=>c,None if choices.len()==1=>choices[0].clone(),None=>{install_window_order(game,window,choices,0);return;}};
    window.remaining.retain(|old|!window_candidate_matches(game,old,&selected.candidate));
    let member=eligible.iter().find(|(c,_)|window_candidate_matches(game,c,&selected.candidate)).unwrap().1[0];
    select_deletion_member(game,&mut window,member);
    if !selected.candidate.is_mandatory{
        let c=&selected.candidate;
        install_optional_selection(game,window.timing,c,c.source_controller,window.subject,window.cause,window.destination);
        if let Some(crate::resume::ResumeFrame::OptionalReplacement(state))=game.pending_selection_resume.as_mut().and_then(|s|s.frames.last_mut()){
            state.window=Some(window);state.source_identity=selected.host_identity;
        }
        return;
    }
    activate_deletion_candidate(game,window,selected,retain_summary);
}
fn select_deletion_member(game:&crate::game::Game,window:&mut ReplacementWindow,index:usize)->bool{
    let member=&window.deletion_batch.as_ref().unwrap().members[index];
    let Some(h)=deletion_member_handle(game,member)else{return false;};
    window.subject=ReplacementSubject::Permanent(h);window.stable_subject=Some(member.identity);window.cancelled=member.prevented;window.cause=deletion_cause_for_controller(window.cause,window.cause_player,h.player);true
}

fn current_deletion_candidate_members(game:&mut crate::game::Game,window:&ReplacementWindow,chosen:&WindowCandidate)->Vec<(usize,Candidate)> {
    (0..window.deletion_batch.as_ref().unwrap().members.len()).filter_map(|i|{
        deletion_member_candidates(game,window,i,true).into_iter()
            .find(|c|window_candidate_matches(game,chosen,c)).map(|c|(i,c))
    }).collect()
}
fn install_deletion_subject_choice(game:&mut crate::game::Game,mut window:ReplacementWindow,chosen:WindowCandidate,members:&[(usize,Candidate)]) {
    let c=&chosen.candidate;
    install_optional_selection(game,window.timing,c,c.source_controller,window.subject,window.cause,window.destination);
    let choices:Vec<_>=members.iter().filter_map(|(i,_)|{
        let h=deletion_member_handle(game,&window.deletion_batch.as_ref().unwrap().members[*i])?;
        Some((crate::action::space::encode_attack(h.player as u16,h.index as u16),*i))
    }).collect();
    if let Some(p)=game.pending_selection.as_mut(){
        p.kind=SelectionKind::AnyField;p.is_optional=false;p.effect_choices=None;
        p.prompt="Choose the impending deletion this effect processes".into();
        p.valid_action_ids=choices.iter().map(|(a,_)|*a).collect();
    }
    window.deletion_batch.as_mut().unwrap().pending_subject=Some((chosen.clone(),choices));
    if let Some(crate::resume::ResumeFrame::OptionalReplacement(s))=game.pending_selection_resume.as_mut().and_then(|s|s.frames.last_mut()){
        s.source_identity=chosen.host_identity;s.window=Some(window);
    }
}
fn activate_deletion_candidate(game:&mut crate::game::Game,mut window:ReplacementWindow,chosen:WindowCandidate,retain_summary:bool){
    let matches=current_deletion_candidate_members(game,&window,&chosen);
    if matches.is_empty(){advance_deletion_window(game,window,None,retain_summary);return;}
    let paid=paid_self_prevention_candidate(game,&chosen.candidate)
        || paid_prevention_properties(game,&chosen.candidate).is_some_and(|(_,all)|all);
    if matches.len()>1 && !paid {
        install_deletion_subject_choice(game,window,chosen,&matches);return;
    }
    select_deletion_member(game,&mut window,matches[0].0);
    let candidate=matches[0].1.clone();
    run_deletion_candidate_body(game,window,candidate,if paid{matches.iter().map(|(i,_)|*i).collect()}else{Vec::new()},retain_summary);
}
fn run_deletion_candidate_body(game:&mut crate::game::Game,mut window:ReplacementWindow,candidate:Candidate,paid_members:Vec<usize>,retain_summary:bool){
    let base=game.parked_replacement.len();
    let mut execution=window.clone();
    if let Some(c)=execution.trigger_context.as_mut(){c.replacement_deletion_members.clear(); c.replacement_paid_prevention=None; c.replacement_deletion_cause=None;}
    if !paid_members.is_empty(){
        let context=execution.trigger_context.get_or_insert_with(Default::default);
        context.replacement_paid_prevention=match &candidate.kind {
            CandidateKind::EffectClosure{card_id,effect_slot}=>Some((candidate.source_card,card_id.clone(),*effect_slot,None)),
            CandidateKind::GrantedKeywordEffect{keyword,effect_slot}=>Some((candidate.source_card,String::new(),*effect_slot,Some(*keyword))),
            CandidateKind::PassiveCancel=>None,
        };
        context.replacement_deletion_cause=Some((window.cause,window.cause_player));
        context.replacement_deletion_members=window.deletion_batch.as_ref().unwrap().members.iter()
            .filter(|m|!m.prevented && !m.terminal).map(|m|{let(p,c,id)=m.identity;(p,c.0,id)}).collect();
        if let Some((filter,true))=paid_prevention_properties(game,&candidate){
            let source_identity = candidate.source_permanent
                .and_then(|h| capture_subject_card(game, ReplacementSubject::Permanent(h)))
                .map(|(player, _, identity)| (player, identity));
            window.deletion_batch.as_mut().unwrap().active_paid_all=Some((filter,context.replacement_deletion_members.clone(),candidate.source_card,candidate.source_controller,source_identity));
        }
    }
    let outcome=run_window_candidate(game,&execution,&candidate);
    if game.pending_selection.is_some(){
        if let Some(parked)=game.parked_replacement.get_mut(base){parked.window=Some(window);}
        wrap_replacement_body(game);return;
    }
    finish_deletion_candidate(game,window,outcome,retain_summary);
}
fn run_deletion_window_step(game:&mut crate::game::Game,mut window:ReplacementWindow,mut state:OptionalReplacementState,action:u16,is_pass:bool){
    if window.departure.as_ref().is_some_and(|d|!d.pending_order.is_empty()) { resume_departure_order(game,window,action); return; }
    if let Some((chosen,choices))=window.deletion_batch.as_mut().unwrap().pending_subject.take(){
        let Some((_,index))=choices.iter().find(|(a,_)|*a==action)else{return;};
        if !select_deletion_member(game,&mut window,*index){advance_deletion_window(game,window,None,false);return;}
        let candidate=deletion_member_candidates(game,&window,*index,true).into_iter().find(|c|window_candidate_matches(game,&chosen,c));
        if let Some(c)=candidate{run_deletion_candidate_body(game,window,c,Vec::new(),false);}
        else{advance_deletion_window(game,window,None,false);}
        return;
    }
    if let Some(order)=state.order.take(){
        let Some(i)=action.checked_sub(30).filter(|i|*i<28).map(|i|order.page*28+i as usize)else{return;};
        if let Some(chosen)=order.choices.get(i).cloned(){advance_deletion_window(game,window,Some(chosen),false);}
        return;
    }
    if is_pass{advance_deletion_window(game,window,None,false);return;}
    let chosen=WindowCandidate{candidate:Candidate{
        source_card:state.source_card,source_permanent:state.source_permanent,source_controller:state.controller,
        is_mandatory:false,effect_name:String::new(),kind:state.candidate_kind},host_identity:state.source_identity};
    activate_deletion_candidate(game,window,chosen,false);
}
fn finish_deletion_candidate(game:&mut crate::game::Game,mut window:ReplacementWindow,outcome:ReplacementOutcome,retain_summary:bool){
    let Some(index)=window.deletion_batch.as_ref().unwrap().members.iter().position(|m|Some(m.identity)==window.stable_subject)else{return;};
    // Plural intent belongs only to the candidate that just completed, even
    // when its payment failed and it produced no cancellation.
    let active_paid_all=window.deletion_batch.as_mut().unwrap().active_paid_all.take();
    match outcome {
        ReplacementOutcome::None=>{},
        ReplacementOutcome::Cancelled=>{
            if let Some((filter,members,card,player,source_identity))=active_paid_all{
                // Preserve the paying effect carrier for relative predicates such as
                // `other`; never retarget a replacement body at its old field slot.
                let source_permanent = source_identity.and_then(|(owner, identity)|
                    game.resolve_digivolve_host_identity(owner, identity));
                let valid:Vec<_>=members.iter().filter_map(|m|game.resolve_digivolve_host_identity(m.0,m.2))
                    .filter(|h|{
                        let read=crate::effect_context::EffectReadContext::new(game,card,source_permanent,player)
                            .with_replacement_context(deletion_cause_for_controller(window.cause,window.cause_player,h.player),window.cause_player,Some(h.player));
                        crate::dsl_cards::predicate::eval_predicate(&filter,&read,crate::dsl_cards::predicate::PredicateSubject::Permanent(*h))
                    }).filter_map(|h|game.player(h.player).battle_area[h.index as usize].battle_identity.map(|id|(h.player,id))).collect();
                for m in &mut window.deletion_batch.as_mut().unwrap().members {
                    if valid.contains(&(m.identity.0,m.identity.2)){m.prevented=true;}
                }
            }else{window.deletion_batch.as_mut().unwrap().members[index].prevented=true;}
        },
        ReplacementOutcome::CustomHandled=>{
            let m=&mut window.deletion_batch.as_mut().unwrap().members[index];
            if !m.prevented{m.terminal=true;}m.prevented=true;
        },
        ReplacementOutcome::Substituted(ReplacementSubject::Permanent(h))=>{
            let m=&mut window.deletion_batch.as_mut().unwrap().members[index];m.prevented=true;m.terminal=true;
            if let Some(identity)=capture_subject_card(game,ReplacementSubject::Permanent(h)){
                let b=window.deletion_batch.as_mut().unwrap();
                if !b.members.iter().any(|m|m.identity==identity) && b.substitution_depth<16{
                    b.substitution_depth+=1;
                    b.members.push(DeletionWindowMember{original:h,identity,prevented:false,terminal:false,substituted:true});
                    let i=b.members.len()-1;
                    for c in deletion_member_candidates(game,&window,i,false){
                        if !window.remaining.iter().any(|old|window_candidate_matches(game,old,&c)){window.remaining.push(capture_window_candidate(game,c));}
                    }
                }
            }
        },
        other=>{
            let m=&mut window.deletion_batch.as_mut().unwrap().members[index];m.prevented=true;m.terminal=true;
            let mut single=window.clone();single.deletion_batch=None;
            game.replacement_fired.insert((window.timing,window.subject));
            game.replacement_fired.insert((EffectTiming::WhenWouldLeaveBattleArea,window.subject));
            commit_window(game,single,other);
            if game.pending_selection.is_some(){
                game.after_selection_resume_hooks.0.push(crate::resume::AfterSelectionHook::DeletionWindowContinue{window:Box::new(window)});
                return;
            }
        }
    }
    advance_deletion_window(game,window,None,retain_summary);
}
pub(crate) fn resume_deletion_window(game:&mut crate::game::Game,window:ReplacementWindow){
    if game.game_over{return;}
    if game.pending_selection.is_some(){
        game.after_selection_resume_hooks.0.push(crate::resume::AfterSelectionHook::DeletionWindowContinue{window:Box::new(window)});
    }else{advance_deletion_window(game,window,None,false);}
}
fn finish_deletion_window(game:&mut crate::game::Game,window:ReplacementWindow,retain_summary:bool){
    if window.departure.is_some() { finish_departure_window(game,window); return; }
    use crate::deletion_batch::DeletionBatch;
    let b=window.deletion_batch.as_ref().unwrap();
    let kill_list=b.members.iter().filter(|m|!m.prevented && !m.terminal).filter_map(|m|deletion_member_handle(game,m)).collect();
    let mut batch=DeletionBatch::new(kill_list,window.cause);
    batch.cancelled=b.members.iter().filter(|m|m.prevented||m.terminal).map(|m|m.original).collect();
    batch.substituted_in=b.members.iter().filter(|m|m.substituted).map(|m|deletion_member_handle(game,m).unwrap_or(m.original)).collect();
    let previous=game.active_deletion_batch.replace(batch);
    let cause=game.current_deletion_cause.replace(window.cause);
    let player=game.effect_source_player;game.effect_source_player=window.cause_player;
    let source_permanent=game.effect_source_permanent;game.effect_source_permanent=b.initiating_source_permanent.and_then(|(player,id)|game.resolve_digivolve_host_identity(player,id));
    let source_card=game.effect_source_card;game.effect_source_card=window.initiating_source_card;
    let trigger=game.current_trigger_context.clone();game.current_trigger_context=window.trigger_context.clone();
    let event_cause=game.current_deletion_event_cause_override;game.current_deletion_event_cause_override=window.event_cause;
    game.with_deletion_receipt(window.deletion_receipt,|g|g.complete_deletion_batch_after_replacements());
    game.current_deletion_cause=cause;game.effect_source_player=player;game.current_deletion_event_cause_override=event_cause;
    game.effect_source_permanent=source_permanent;game.effect_source_card=source_card;game.current_trigger_context=trigger;
    if !retain_summary{game.active_deletion_batch=previous;}
}
pub(crate) fn prevent_chosen_deletion_member(game:&mut crate::game::Game,h:PermanentHandle){
    let Some(current)=game.player(h.player).battle_area.get(h.index as usize).and_then(|p|p.battle_identity)else{return;};
    let Some(parked)=game.active_parked_replacement_mut()else{return;};
    let Some(window)=parked.window.as_mut().filter(|w|w.deletion_batch.is_some())else{return;};
    let Some(identity)=window.deletion_batch.as_ref().unwrap().members.iter().find(|m|m.identity.0==h.player && m.identity.2==current).map(|m|m.identity)else{return;};
    window.subject=ReplacementSubject::Permanent(h);window.stable_subject=Some(identity);
    parked.subject=window.subject;parked.stable_subject_card=Some(identity);parked.outcome=ReplacementOutcome::Cancelled;
}

fn paid_prevention_properties(game:&crate::game::Game,c:&Candidate)->Option<(digimon_dsl::compiled::CompiledPredicate,bool)> {
    let effects=match &c.kind {
        CandidateKind::EffectClosure{card_id,..}=>game.effects_for_card(card_id,c.source_card)?,
        CandidateKind::GrantedKeywordEffect{keyword,..}=>std::sync::Arc::new(crate::cards::keyword_effects::keyword_to_auto_effect(*keyword,c.source_card)),
        CandidateKind::PassiveCancel=>return None,
    };
    let slot=match c.kind{CandidateKind::EffectClosure{effect_slot,..}|CandidateKind::GrantedKeywordEffect{effect_slot,..}=>effect_slot,CandidateKind::PassiveCancel=>return None};
    let e=effects.get(slot as usize)?;
    Some((e.paid_prevention_filter.clone().unwrap_or_default(),e.paid_prevention_all))
}

// Preserve the historical four-field active_paid_all Debug payload. The fifth
// field is gameplay state and is separately hashed for current verification.
impl std::fmt::Debug for DeletionWindowBatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let legacy = self.active_paid_all.as_ref().map(|(filter,members,card,player,_)|
            (filter,members,card,player));
        f.debug_struct("DeletionWindowBatch")
            .field("members", &self.members)
            .field("pending_subject", &self.pending_subject)
            .field("substitution_depth", &self.substitution_depth)
            .field("initiating_source_permanent", &self.initiating_source_permanent)
            .field("active_paid_all", &legacy).finish()
    }
}

/// Path-tagged physical source identities for current digests. LegacyV2 never
/// calls this extension; None within an active plural window is significant.
pub(crate) fn verification_plural_carriers(game: &crate::game::Game)
    -> Vec<(String,Option<(PlayerId,u64)>)>
{
    fn window(path:String,w:&ReplacementWindow,out:&mut Vec<(String,Option<(PlayerId,u64)>)>) {
        if let Some((_,_,_,_,source))=w.deletion_batch.as_ref().and_then(|b|b.active_paid_all.as_ref()) {
            out.push((path,*source));
        }
    }
    fn stack(path:&str,s:&crate::resume::ResumeStack,out:&mut Vec<(String,Option<(PlayerId,u64)>)>) {
        use crate::resume::ResumeFrame as F;
        for (i,frame) in s.frames.iter().enumerate() {
            let p=format!("{path}/{i}");
            match frame {
                F::OptionalReplacement(s)=>if let Some(w)=&s.window {window(format!("{p}/OptionalReplacement"),w,out);},
                F::FieldTopSourceBatch{inner,..}=>stack(&format!("{p}/FieldTopSourceBatch"),inner,out),
                F::ReplacementBody{inner,..}=>stack(&format!("{p}/ReplacementBody"),inner,out),
                F::DelayCancelAfterSelection{inner,..}=>stack(&format!("{p}/DelayCancelAfterSelection"),inner,out),
                F::DelayHandDigivolveAfterSelection{inner,..}=>stack(&format!("{p}/DelayHandDigivolveAfterSelection"),inner,out),
                F::DelayDnaAfterSelection{inner,..}=>stack(&format!("{p}/DelayDnaAfterSelection"),inner,out),
                F::DelayFieldDnaAfterPayment{inner,..}=>stack(&format!("{p}/DelayFieldDnaAfterPayment"),inner,out),
                F::DelayPlayFromHandAfterSelection{inner,..}=>stack(&format!("{p}/DelayPlayFromHandAfterSelection"),inner,out),
                F::DelayPlayFromUnionAfterSelection{inner,..}=>stack(&format!("{p}/DelayPlayFromUnionAfterSelection"),inner,out),
                F::DelayBodyAfterCost{inner,..}=>stack(&format!("{p}/DelayBodyAfterCost"),inner,out),
                _=>{},
            }
        }
    }
    let mut out=Vec::new();
    if let Some(s)=&game.pending_selection_resume {stack("pending",s,&mut out);}
    for (i,p) in game.parked_replacement.iter().enumerate() {
        if let Some(w)=&p.window {window(format!("parked/{i}"),w,&mut out);}
    }
    for (i,h) in game.after_selection_resume_hooks.0.iter().enumerate() {
        if let crate::resume::AfterSelectionHook::DeletionWindowContinue{window:w}=h {
            window(format!("hook/{i}/DeletionWindowContinue"),w,&mut out);
        }
    }
    out
}

#[cfg(test)]
mod plural_carrier_digest_compatibility {
    use super::*;
    use crate::{debug_runner::DebugRunner,recorder::VerificationDigestVersion as V,resume::{ResumeFrame as F,ResumeStack,AfterSelectionHook}};
    fn window(source:Option<(PlayerId,u64)>)->ReplacementWindow {
        ReplacementWindow {
            timing:EffectTiming::WhenWouldLeaveBattleArea,
            subject:ReplacementSubject::Permanent(PermanentHandle{player:0,index:0}),
            stable_subject:None,cause:ReplacementCause::OpponentEffect,destination:Some(Zone::Trash),
            cause_player:Some(1),initiating_source_card:None,effect_digivolve_card:None,
            play_declaration:None,event_cause:None,trigger_context:None,deletion_receipt:None,
            turn_player:0,remaining:Vec::new(),cancelled:false,departure:None,
            deletion_batch:Some(DeletionWindowBatch{members:Vec::new(),pending_subject:None,
                substitution_depth:0,initiating_source_permanent:None,
                active_paid_all:Some((Default::default(),vec![(0,7,99)],CardHandle(7),0,source))}),
        }
    }
    fn state(w:ReplacementWindow)->OptionalReplacementState {
        OptionalReplacementState{window:Some(w),order:None,source_identity:None,deletion_receipt:None,
            timing:EffectTiming::WhenWouldLeaveBattleArea,candidate_kind:CandidateKind::PassiveCancel,
            source_card:CardHandle(7),source_permanent:None,controller:0,
            subject:ReplacementSubject::Permanent(PermanentHandle{player:0,index:0}),stable_subject_card:None,
            cause:ReplacementCause::OpponentEffect,event_cause_override:None,original_destination:Some(Zone::Trash),outer_conts:Vec::new()}
    }
    fn game(source:Option<(PlayerId,u64)>,owner:u8)->crate::game::Game {
        let mut g=DebugRunner::builder().start().game;
        let w=window(source);
        if owner==2 {
            g.after_selection_resume_hooks.0.push(AfterSelectionHook::DeletionWindowContinue{window:Box::new(w)});
        } else if owner==3 {
            g.parked_replacement.push(ParkedReplacement{window:Some(w),outer_conts:Vec::new(),deletion_receipt:None,
                timing:Some(EffectTiming::WhenWouldLeaveBattleArea),subject:ReplacementSubject::Permanent(PermanentHandle{player:0,index:0}),
                stable_subject_card:None,cause:ReplacementCause::OpponentEffect,event_cause_override:None,
                original_destination:Some(Zone::Trash),source_card:CardHandle(7),source_permanent:None,controller:0,
                outcome:ReplacementOutcome::None,cancel_if_deleted:None});
        } else {
            let mut s=ResumeStack{frames:vec![F::OptionalReplacement(state(w))]};
            if owner==1 {s=ResumeStack{frames:vec![F::ReplacementBody{inner:Box::new(s),outer_conts:Vec::new()}]};}
            g.pending_selection_resume=Some(s);
        }
        g
    }
    #[test]
    fn populated_batch_debug_preserves_exact_pre_extension_four_tuple() {
        for source in [None,Some((0,99))] {
            let w=window(source);let b=w.deletion_batch.as_ref().unwrap();
            let old=b.active_paid_all.as_ref().map(|(f,m,c,p,_)|(f,m,c,p));
            let expected=format!("DeletionWindowBatch {{ members: {:?}, pending_subject: {:?}, substitution_depth: {:?}, initiating_source_permanent: {:?}, active_paid_all: {:?} }}",b.members,b.pending_subject,b.substitution_depth,b.initiating_source_permanent,old);
            assert_eq!(format!("{b:?}"),expected);
        }
        let mut w=window(None);w.deletion_batch.as_mut().unwrap().active_paid_all=None;
        assert_eq!(format!("{:?}",w.deletion_batch.unwrap()),"DeletionWindowBatch { members: [], pending_subject: None, substitution_depth: 0, initiating_source_permanent: None, active_paid_all: None }");
    }
    #[test]
    fn legacy_v2_retains_projection_while_current_hashes_source_for_each_owner() {
        for owner in 0..4 {
            let a=game(Some((0,99)),owner);
            for source in [None,Some((0,100)),Some((1,99))] {
                let b=game(source,owner);
                assert_eq!(a.verification_digest_with_version(V::LegacyV2),b.verification_digest_with_version(V::LegacyV2),"legacy owner {owner}");
                assert_ne!(a.verification_digest_with_version(V::V3),b.verification_digest_with_version(V::V3),"current owner {owner}");
            }
            assert_eq!(a.verification_digest(),a.clone().verification_digest());
        }
    }
    #[test]
    fn current_digest_retains_hook_owner_order_and_same_card_new_incarnation() {
        let mut a=game(Some((0,99)),2);
        a.after_selection_resume_hooks.0.push(AfterSelectionHook::DeletionWindowContinue{window:Box::new(window(Some((0,100))))});
        let mut b=a.clone();b.after_selection_resume_hooks.0.reverse();
        assert_ne!(a.verification_digest(),b.verification_digest());
        assert_eq!(a.verification_digest_with_version(V::LegacyV2),b.verification_digest_with_version(V::LegacyV2));
    }
}
