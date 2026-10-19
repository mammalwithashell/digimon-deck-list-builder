include!("dual_effect_source_kind.rs");
fn granted_changes_before_activation(gain:bool){
 let (mut r,host)=setup(!gain);let seen=Arc::new(Mutex::new(vec![]));let witness=seen.clone();
 let grantor=PermanentHandle{player:0,index:1};let card=r.top_card(grantor);
 EffectContext::new(&mut r.game,card,Some(grantor),0).grant_triggered_effect(host,
 digimon_engine::enums::EffectTiming::WhenAttacking,Expiry::Permanent,move|ctx|witness.lock().unwrap().push(ctx.source_kind()));
 r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenAttacking,digimon_engine::selection::TriggerSource::Permanent(host));
 if gain {r.game.modifiers.add(host,ModifierEntry::simple(digimon_engine::enums::ModifierType::TreatAsDigimon,0,Expiry::EndOfTurn,0).with_payload(digimon_engine::modifiers::ModifierPayload::AdditiveSynthIdentity{kind:digimon_engine::enums::CardKind::Digimon,level:0,colors:vec![],traits:vec![],dp:3000}));}else{r.game.modifiers.expire_end_of_turn(0);}
 let state=r.game.clone();let mut digest=None;
 for _ in 0..2 {r.game=state.clone();seen.lock().unwrap().clear();r.game.drain_effect_queue();
 assert_eq!(*seen.lock().unwrap(),vec![if gain{EffectSourceKind::DigimonAndTamer}else{EffectSourceKind::Tamer}]);
 let current=r.game.verification_digest();if let Some(old)=digest{assert_eq!(old,current);}digest=Some(current);}
}
#[test]fn q2229_review24_granted_gains_dual_kind_at_activation(){granted_changes_before_activation(true);}
#[test]fn q2229_review24_granted_loses_dual_kind_at_activation(){granted_changes_before_activation(false);}
#[test]fn q2229_review24_explicit_queued_option_attribution_stays_option(){
 let(mut r,h)=setup(true);let card=r.top_card(h);
 r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::OnSuspend,digimon_engine::selection::TriggerSource::EventObserved{player:0,permanent:h,card,effect_initiated:true});
 // A refire entry carries explicit attribution; the native Marcus body is
 // reused only as a real queued body discriminator, not as printed Option text.
 let entry=r.game.effect_queue.front_mut().unwrap();entry.source_kind=EffectSourceKind::Option;entry.attribution_source_card=Some(card);
 r.game.drain_effect_queue();r.accept_optional_trigger().unwrap();
 assert_eq!(r.pending_selection().unwrap().source_kind,EffectSourceKind::Option);
}
#[test]fn q2229_review24_literal_membership_matrix(){
 use EffectSourceKind::*;
 let categories=[Digimon,Tamer,Option,Rule,DigimonAndTamer];
 let expected=[[true,false,false,false,false],[false,true,false,false,false],[false,false,true,false,false],[false,false,false,true,false],[true,true,false,false,true]];
 for (i,actual) in categories.iter().enumerate(){for(j,query)in categories.iter().enumerate(){assert_eq!(actual.includes(*query),expected[i][j],"{actual:?} queried as {query:?}");}}
}
