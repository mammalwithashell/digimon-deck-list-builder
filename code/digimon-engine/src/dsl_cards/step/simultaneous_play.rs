//! One activation per simultaneous event (a simultaneous play, or one deletion
//! batch, §15-5-2); singular event references ("that Digimon") remain
//! choices. The prompt and labels are event-neutral: deletion observers reach
//! this too (`Game::collapse_simultaneous_deletion_event_batch`).
use digimon_dsl::compiled::CompiledStep;
use crate::dsl_cards::bindings::{Bindings,BindingValue};
use crate::effect_context::EffectContext;
use super::{StepRuntime,RunOutcome,run_steps_with_runtime};

const SUBJECT_CHOICE_KEY:&str="__simultaneous_play_subject";
pub(super) fn expand_body(steps:&[CompiledStep],ctx:&mut EffectContext<'_>,bindings:&Bindings)->Option<Vec<CompiledStep>> {
    if matches!(steps.first(),Some(CompiledStep::WithSimultaneousPlaySubject{..}))
        || matches!((steps.first(),steps.get(1)),(Some(CompiledStep::SelectEffectChoice{bind_as:Some(selected),prompt_key:Some(key),..}),Some(CompiledStep::WithSimultaneousPlaySubject{binding,..})) if key==SUBJECT_CHOICE_KEY && selected==binding) { return None; }
    let members=&ctx.game.current_trigger_context.as_ref()?.simultaneous_play_members;
    if members.len()<2 || !uses_singular_event_subject(steps) { return None; }
    let labels=members.iter().enumerate().map(|(i,m)| {
        let name=m.event_card.and_then(|h|ctx.game.card_data_for_handle(h)).map(|d|d.card_name.as_str()).unwrap_or("event card");
        format!("Use simultaneous event subject {}: {}",i+1,name)
    }).collect();
    let mut n=0;
    let binding=loop {let s=format!("__simultaneous_play_subject_{n}"); if bindings.get_ref(&s).is_none(){break s;} n+=1;};
    Some(vec![CompiledStep::SelectEffectChoice {labels,legal_when:None,bind_as:Some(binding.clone()),prompt:"Choose the card referred to by this effect".into(),prompt_key:Some(SUBJECT_CHOICE_KEY.into())},
        CompiledStep::WithSimultaneousPlaySubject{binding,body:steps.to_vec()}])
}

pub(super) fn try_run(step:&CompiledStep,ctx:&mut EffectContext<'_>,bindings:&mut Bindings,runtime:&StepRuntime)->Option<RunOutcome> {
    let CompiledStep::WithSimultaneousPlaySubject{binding,body}=step else{return None;};
    let Some(BindingValue::Literal(index))=bindings.get_ref(binding) else{return Some(RunOutcome::Synchronous);};
    let selected=usize::try_from(*index).ok().and_then(|i|ctx.game.current_trigger_context.as_ref()?.simultaneous_play_members.get(i).cloned());
    let Some(selected)=selected else{return Some(RunOutcome::Synchronous);};
    bindings.remove(binding);
    let previous=ctx.game.current_trigger_context.replace(selected);
    let outcome=run_steps_with_runtime(body,ctx,bindings,runtime);
    ctx.game.current_trigger_context=previous;
    Some(outcome)
}

// Inspect enum identity, not JSON text: a card name, prompt, or named binding
// called "EventTarget" must never create a hidden extra decision.
fn uses_singular_event_subject(steps:&[CompiledStep])->bool {
    let mut probe=SubjectProbe::default();
    serde::Serialize::serialize(steps,&mut probe).expect("infallible structural probe");
    probe.found
}
#[derive(Default)]
struct SubjectProbe{found:bool,struct_names:Vec<&'static str>}
impl SubjectProbe {
 fn variant(&mut self,name:&str,variant:&str){
  self.found |= (name=="CompiledBindingRef" && matches!(variant,"EventTarget"|"EventCard"))
    || (name=="CompiledFormula" && variant=="EventTargetLevel");
 }
}
macro_rules! primitive {($($method:ident($ty:ty)),*)=>{$(fn $method(self,_v:$ty)->Result<(),Self::Error>{Ok(())})*};}
impl<'a> serde::Serializer for &'a mut SubjectProbe {
 type Ok=();type Error=serde_json::Error;
 type SerializeSeq=Self;type SerializeTuple=Self;type SerializeTupleStruct=Self;type SerializeTupleVariant=Self;
 type SerializeMap=Self;type SerializeStruct=Self;type SerializeStructVariant=Self;
 primitive!(serialize_bool(bool),serialize_i8(i8),serialize_i16(i16),serialize_i32(i32),serialize_i64(i64),serialize_u8(u8),serialize_u16(u16),serialize_u32(u32),serialize_u64(u64),serialize_f32(f32),serialize_f64(f64),serialize_char(char),serialize_str(&str),serialize_bytes(&[u8]));
 fn serialize_none(self)->Result<(),Self::Error>{Ok(())}
 fn serialize_some<T:serde::Serialize+?Sized>(self,v:&T)->Result<(),Self::Error>{v.serialize(self)}
 fn serialize_unit(self)->Result<(),Self::Error>{Ok(())}
 fn serialize_unit_struct(self,_n:&'static str)->Result<(),Self::Error>{Ok(())}
 fn serialize_unit_variant(self,n:&'static str,_i:u32,v:&'static str)->Result<(),Self::Error>{self.variant(n,v);Ok(())}
 fn serialize_newtype_struct<T:serde::Serialize+?Sized>(self,_n:&'static str,v:&T)->Result<(),Self::Error>{v.serialize(self)}
 fn serialize_newtype_variant<T:serde::Serialize+?Sized>(self,n:&'static str,_i:u32,v:&'static str,x:&T)->Result<(),Self::Error>{self.variant(n,v);x.serialize(self)}
 fn serialize_seq(self,_n:Option<usize>)->Result<Self,Self::Error>{Ok(self)}
 fn serialize_tuple(self,_n:usize)->Result<Self,Self::Error>{Ok(self)}
 fn serialize_tuple_struct(self,_s:&'static str,_n:usize)->Result<Self,Self::Error>{Ok(self)}
 fn serialize_tuple_variant(self,n:&'static str,_i:u32,v:&'static str,_len:usize)->Result<Self,Self::Error>{self.variant(n,v);Ok(self)}
 fn serialize_map(self,_n:Option<usize>)->Result<Self,Self::Error>{Ok(self)}
 fn serialize_struct(self,s:&'static str,_n:usize)->Result<Self,Self::Error>{self.struct_names.push(s);Ok(self)}
 fn serialize_struct_variant(self,n:&'static str,_i:u32,v:&'static str,_len:usize)->Result<Self,Self::Error>{self.variant(n,v);self.struct_names.push(n);Ok(self)}
}
macro_rules! sequence {($trait:ident,$method:ident)=>{impl serde::ser::$trait for &mut SubjectProbe {
 type Ok=();type Error=serde_json::Error;
 fn $method<T:serde::Serialize+?Sized>(&mut self,v:&T)->Result<(),Self::Error>{v.serialize(&mut **self)}
 fn end(self)->Result<(),Self::Error>{Ok(())}
}};}
sequence!(SerializeSeq,serialize_element);sequence!(SerializeTuple,serialize_element);
sequence!(SerializeTupleStruct,serialize_field);sequence!(SerializeTupleVariant,serialize_field);
impl serde::ser::SerializeMap for &mut SubjectProbe {
 type Ok=();type Error=serde_json::Error;
 fn serialize_key<T:serde::Serialize+?Sized>(&mut self,v:&T)->Result<(),Self::Error>{v.serialize(&mut **self)}
 fn serialize_value<T:serde::Serialize+?Sized>(&mut self,v:&T)->Result<(),Self::Error>{v.serialize(&mut **self)}
 fn end(self)->Result<(),Self::Error>{Ok(())}
}
macro_rules! structure {($trait:ident)=>{impl serde::ser::$trait for &mut SubjectProbe {
 type Ok=();type Error=serde_json::Error;
 fn serialize_field<T:serde::Serialize+?Sized>(&mut self,k:&'static str,v:&T)->Result<(),Self::Error>{
  let owner=self.struct_names.last().copied();
  if owner==Some("CompiledPredicate") && (k.starts_with("event_target_") || k.starts_with("event_card_")
      || k.starts_with("event_host_") || matches!(k,"event_permanent_is_source"|"is_event_host"|"host_permanent_trait_has")) {
   // Option(false) is still a subject-dependent test; only None is absent.
   self.found |= !serde_json::to_value(v)?.is_null();
  }
  if owner==Some("CompiledPlayCostBindingPredicate") && k=="binding" {
   self.found |= serde_json::to_value(v)?.as_str()==Some("event_target");
  }
  v.serialize(&mut **self)
 }
 fn end(self)->Result<(),Self::Error>{self.struct_names.pop();Ok(())}
}};}

structure!(SerializeStruct);structure!(SerializeStructVariant);

#[cfg(test)] mod tests {
 use super::*;use digimon_dsl::compiled::CompiledBindingRef;
 #[test] fn subject_probe_distinguishes_enum_from_lookalike_named_binding(){
  for (r,want) in [(CompiledBindingRef::EventTarget,true),(CompiledBindingRef::EventCard,true),(CompiledBindingRef::Named("EventTarget".into()),false)] {
   let mut p=SubjectProbe::default();serde::Serialize::serialize(&r,&mut p).unwrap();assert_eq!(p.found,want);
  }
 }
}

pub(super) fn annotate_selection(step:&CompiledStep,ctx:&mut EffectContext<'_>) {
 let CompiledStep::SelectEffectChoice{prompt_key:Some(key),..}=step else{return;};
 if key!=SUBJECT_CHOICE_KEY{return;}
 let members:Vec<_>=ctx.game.current_trigger_context.as_ref().map(|c|c.simultaneous_play_members.iter()
  .map(|m|(m.event_card,m.event_card.map(|h|ctx.game.effect_source_kind_for_handle(h)))).collect()).unwrap_or_default();
 if let Some(choices)=ctx.game.pending_selection.as_mut().and_then(|s|s.effect_choices.as_mut()) {
  for (row,(card,kind)) in choices.iter_mut().zip(members){row.source_card=card;row.source_kind=kind;}
 }
}
#[cfg(test)] mod predicate_probe_tests {
 use super::*;use digimon_dsl::compiled::CompiledPredicate;
 fn probe(key:&str,value:serde_json::Value)->bool {
  let mut pred=serde_json::to_value(CompiledPredicate::default()).unwrap();pred[key]=value;
  let pred:CompiledPredicate=serde_json::from_value(pred).unwrap();
  let mut p=SubjectProbe::default();serde::Serialize::serialize(&pred,&mut p).unwrap();p.found
 }
 #[test] fn all_singular_event_predicate_families_include_false_conditions(){
  for k in ["event_permanent_is_source","is_event_host","event_host_permanent_is_source","event_host_is_own_digimon","event_host_is_own_tamer","event_target_is_source"] {
   for v in [false,true]{assert!(probe(k,serde_json::json!(v)),"{k}={v}");}
  }
  assert!(probe("host_permanent_trait_has",serde_json::json!("Dragon")));
  assert!(!probe("name_is",serde_json::json!("event_target")));
 }
 #[test] fn special_cost_reference_is_typed_not_an_arbitrary_binding_string(){
  use digimon_dsl::compiled::{CompiledPlayCostBindingPredicate,CompiledComparatorOp};
  for (binding,want) in [("event_target",true),("ordinary_card",false)] {
   let v=CompiledPlayCostBindingPredicate{binding:binding.into(),offset:0,op:CompiledComparatorOp::Eq};
   let mut p=SubjectProbe::default();serde::Serialize::serialize(&v,&mut p).unwrap();assert_eq!(p.found,want);
  }
 }
}
