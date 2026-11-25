//! Explicit-input section-placement diagnostic, never admission or equivalence.
//! Genuine rows run against the default production registry from the embedded DSL pack.
//! Avoid DebugRunner::dsl_card duplicate full-pack cache; require actual registry entries.
use std::{collections::HashMap,env,fs};
use digimon_engine::{card_data::CardData,debug_runner::{DebugRunner,make_test_card},enums::{CardKind,Expiry,ModifierType},modifiers::PlayerModifierEntry,selection::SelectionKind};
use serde_json::{json,Value};
use sha2::{Digest,Sha256};
fn sha(b:&[u8])->String {format!("{:x}",Sha256::digest(b))}
fn load(v:&Value, ids:&[&str])->HashMap<String,CardData>{
 let mut selected=serde_json::Map::new();for id in ids {selected.insert((*id).into(),v[*id].clone());}
 CardData::load_from_str(&Value::Object(selected).to_string()).unwrap()
}
fn metadata(r:&DebugRunner,c:&CardData){let x=r.game.card_data.iter().find(|x|x.card_id==c.card_id).unwrap();assert_eq!(x.effect_text,c.effect_text);assert_eq!(x.inherited_text,c.inherited_text);assert_eq!(x.security_text,c.security_text);}
fn reveal(cards:&HashMap<String,CardData>,target:&str,want:bool)->Value{
 let mut r=DebugRunner::builder()
 .add_card(cards["BT17-009"].clone()).add_card(cards["BT7-090"].clone()).add_card(cards[target].clone())
 .deck(0,&["BT7-090",target,"BT17-009"]).hand(0,&["BT17-009"]).memory(10).start();
 assert!(r.game.effect_registry.get("BT17-009").is_some(),"actual production embedded consumer required");
 metadata(&r,&cards[target]);r.play(0,0).expect("real Flamemon play");
 assert!(r.pending_selection().is_some(),"real Hybrid bucket");
 r.auto_resolve().expect("real reveal picks settle");
 let picked=r.game.player(0).hand.iter().any(|c|c.card_id(&r.game.card_data)==target);
 assert_eq!(picked,want,"{target}: has_inherited real reveal consumer");
 json!({"subject":target,"added_to_hand":picked})
}
fn event(cards:&HashMap<String,CardData>,target:&str,want:bool)->Value{
 let mut fuel=make_test_card("FUEL","Fuel");fuel.card_kind=CardKind::Digimon;fuel.dp=Some(1000);
 let mut r=DebugRunner::builder().add_card(cards["BT21-091"].clone())
 .add_card(cards["BT17-009"].clone()).add_card(cards[target].clone()).add_card(fuel)
 .deck(0,&["FUEL","FUEL","FUEL","FUEL"]).deck(1,&["FUEL","FUEL","FUEL","FUEL"])
 .hand(0,&["BT21-091",target]).memory(10).start();
 assert!(r.game.effect_registry.get("BT21-091").is_some(),"actual production embedded event consumer required");
 metadata(&r,&cards[target]);r.place_on_field(0,"BT17-009",Some(0));r.game.enter_main_phase();
 r.game.play_option_from_hand(0,0);r.auto_resolve().expect("park real Delay Option");
 assert!(r.game.player(0).battle_area.iter().any(|p|p.top_card().card_id(&r.game.card_data)=="BT21-091"));
 r.end_turn();r.game.enter_main_phase();r.end_turn();r.game.enter_main_phase();r.game.memory=10;
 let i=r.game.player(0).hand.iter().position(|c|c.card_id(&r.game.card_data)==target).unwrap();r.play(0,i).expect("play actual Tamer metadata subject");
 let offered=r.pending_kind()==Some(SelectionKind::Replacement);
 assert_eq!(offered,want,"{target}: event_target_has_inherited real Delay consumer");
 if offered {assert!(r.pending_is_optional());}
 json!({"subject":target,"delay_offered":offered})
}
fn security(cards:&HashMap<String,CardData>,id:&str,blocked:bool)->Value{
 let mut atk=make_test_card("ATTACKER","Attacker");atk.card_kind=CardKind::Digimon;atk.dp=Some(20000);
 let mut r=DebugRunner::builder()
 .add_card(cards[id].clone()).add_card(atk).security(1,&[id,id]).start();assert!(r.game.effect_registry.get(id).is_some(),"actual production embedded Tamer implementation required");metadata(&r,&cards[id]);
 let checked=r.game.player(1).security.last().unwrap().handle();let memory=r.memory();
 if blocked {r.game.modifiers.add_player_modifier(1,PlayerModifierEntry::simple(ModifierType::CannotPlayTamerByEffect,0,Expiry::Permanent,None,0));}
 let h=r.place_on_field(0,"ATTACKER",Some(0));r.attack_player(h,1,false);r.auto_resolve().expect("real security effects settle");
 let played=r.game.player(1).battle_area.iter().any(|p|p.top_card().card_id(&r.game.card_data)==id);
 assert_eq!(played,!blocked);assert_eq!(r.security_count(1),1);assert_eq!(r.memory(),memory,"security self-play pays no memory");
 if played {assert!(r.game.player(1).battle_area.iter().any(|p|p.top_card().handle()==checked));}
 let trashed=r.game.player(1).trash.iter().any(|c|c.card_id(&r.game.card_data)==id);assert_eq!(trashed,blocked);
 json!({"card_id":id,"blocked":blocked,"played":played,"trashed":trashed,"exact_physical_card":true,"remaining_security":1,"memory_unchanged":true})
}
fn main(){
 let args:Vec<_>=env::args().collect();assert_eq!(args.len(),5,"required baseline.json candidate.json attestation.json exact-case");let case=&args[4];
 let bb=fs::read(&args[1]).unwrap();let cb=fs::read(&args[2]).unwrap();let ab=fs::read(&args[3]).unwrap();let att:Value=serde_json::from_slice(&ab).unwrap();
 assert_eq!(att["baseline_sha256"],sha(&bb));assert_eq!(att["candidate_sha256"],sha(&cb));
 let base:Value=serde_json::from_slice(&bb).unwrap();let candidate:Value=serde_json::from_slice(&cb).unwrap();let mut controlled=base.clone();
 let fields=att["fields"].as_array().expect("exact field witnesses required");
 let tamers=att["security_tamers"].as_array().expect("explicit reviewed Tamer subset");
 assert!(!tamers.is_empty(),"security scope cannot silently be empty");
 let mut ids=vec!["EX8-065"];for t in tamers {let id=t.as_str().unwrap();assert!(!ids.contains(&id));ids.push(id);}
 assert_eq!(fields.len(),ids.len()*2,"exact section pairs only");
 for &id in &ids {
  for field in ["inherited_effect_description_eng","security_effect_description_eng"] {
   let es:Vec<_>=fields.iter().filter(|x|x["card_id"]==id&&x["field_path"]==field).collect();assert_eq!(es.len(),1);let e=es[0];
   assert_eq!(e["old_value"],base[id][field]);assert_eq!(e["new_value"],candidate[id][field]);
   assert_eq!(e["old_digest"],sha(format!("{}\n",e["old_value"]).as_bytes()));assert_eq!(e["new_digest"],sha(format!("{}\n",e["new_value"]).as_bytes()));controlled[id][field]=candidate[id][field].clone();
  }
  assert_eq!(candidate[id]["inherited_effect_description_eng"],"");
  assert_eq!(candidate[id]["security_effect_description_eng"],"[Security] Play this card without paying the cost.");
  assert_eq!(base[id]["security_effect_description_eng"],"");
  let before=base[id]["inherited_effect_description_eng"].as_str().unwrap();
  assert!(before=="[Security] Play this card without paying the cost."||before=="Security Effect [Security] Play this card without paying the cost.");
 }
 let mut needed=ids.clone();needed.extend(["BT17-009","BT17-079","BT7-090","BT21-091"]);
 let mut outcomes=Vec::new();
 for (label,rows,want) in [("baseline",&base,true),("candidate",&candidate,false),("controlled_reviewed_sections_only_not_production",&controlled,false)] {
  let presence_case=format!("presence:{label}");
  let security_case=tamers.iter().find(|t|case==&format!("security:{label}:{}",t.as_str().unwrap()));
  if case!=&presence_case && security_case.is_none(){continue;}
  let cards=load(rows,&needed);assert_eq!(cards["EX8-065"].card_kind,CardKind::Tamer);
  if case==&presence_case {
   let searches:Vec<_>=[("EX8-065",want),("BT17-079",true),("BT7-090",false)].iter().map(|(id,w)|reveal(&cards,id,*w)).collect();
   let events:Vec<_>=[("EX8-065",want),("BT17-079",true),("BT7-090",false)].iter().map(|(id,w)|event(&cards,id,*w)).collect();
   outcomes.push(json!({"catalog":label,"has_inherited_reveal":searches,"event_target_has_inherited":events}));
  } else {
   let id=security_case.unwrap().as_str().unwrap();assert_eq!(cards[id].card_kind,CardKind::Tamer);
   let search=reveal(&cards,id,want);let positive=security(&cards,id,false);let negative=security(&cards,id,true);
   outcomes.push(json!({"catalog":label,"tamer_has_inherited_reveal":search,"security":[positive,negative]}));
  }
 }
 assert_eq!(outcomes.len(),1,"unknown or ambiguous exact-case selector");
 println!("{}",serde_json::to_string_pretty(&json!({"case":case,"status":"INTENTIONAL_SECTION_BEHAVIOR_DIAGNOSTIC_NOT_ADMISSION","general_equivalence":false,"EX8-065_own_implementation_qualified":false,"source_sha256":sha(include_bytes!("section_presence_runtime.rs")),"baseline_sha256":sha(&bb),"candidate_sha256":sha(&cb),"attestation_sha256":sha(&ab),"outcomes":outcomes})).unwrap());
}
