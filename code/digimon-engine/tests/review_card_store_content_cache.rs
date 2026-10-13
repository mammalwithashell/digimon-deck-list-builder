use digimon_engine::card_data::{CardData,DualCardData,DualDigimonFace,DualOptionFace,EvoCost};
use digimon_engine::card_store::{shared_card_store,build_shared_card_store};
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::{CardColor,Keyword};
use std::{collections::HashMap,sync::{Arc,Barrier}};
fn db(c:CardData)->HashMap<String,CardData>{HashMap::from([(c.card_id.clone(),c)])}
#[test]
fn reviewer_signed_zero_identity_and_live_a_b_a(){
 let mut a=make_test_card("REVIEW-CACHE-ZERO","Alpha");a.norm_id=0.0;let mut b=a.clone();b.norm_id=-0.0;
 let sa=shared_card_store(&db(a.clone()));let sb=shared_card_store(&db(b.clone()));
 assert!(!Arc::ptr_eq(&sa,&sb));assert_eq!(sa.data[sa.index[&a.card_id]].norm_id.to_bits(),0.0f32.to_bits());
 assert_eq!(sb.data[sb.index[&b.card_id]].norm_id.to_bits(),(-0.0f32).to_bits());
 assert!(Arc::ptr_eq(&sa,&shared_card_store(&db(a))));assert!(Arc::ptr_eq(&sb,&shared_card_store(&db(b))));
}
#[test]
fn reviewer_nested_dual_metadata_preserved_and_distinguished(){
 let mut a=make_test_card("REVIEW-CACHE-NESTED","Alpha");a.dual=Some(DualCardData{
 digimon:DualDigimonFace{level:3,dp:2000,colors:vec![CardColor::Red],traits:vec!["Alpha".into()],evo_costs:vec![EvoCost{card_color:0,level:2,memory_cost:1}],effect_text:"Alpha".into(),inherited_text:"Alpha".into(),keywords:vec![]},
 option:DualOptionFace{use_cost:4,colors:vec![CardColor::Blue],effect_text:"Alpha".into(),security_text:"Alpha".into(),keywords:vec![]}});
 let original=shared_card_store(&db(a.clone()));let held=original.data[original.index[&a.card_id]].clone();
 for n in 0..7 {let mut b=a.clone();let d=b.dual.as_mut().unwrap();match n {
 0=>d.digimon.traits[0]="Bravo".into(),1=>d.digimon.inherited_text="Bravo".into(),2=>d.digimon.evo_costs[0].memory_cost=2,
 3=>d.option.security_text="Bravo".into(),4=>d.option.colors=vec![CardColor::Yellow],5=>d.option.keywords=vec![Keyword::Rush],_=>d.digimon.keywords=vec![Keyword::Blocker]}
 let input=db(b);let fresh=build_shared_card_store(&input);let changed=shared_card_store(&input);
 assert!(!Arc::ptr_eq(&original,&changed));assert_eq!(changed.data[changed.index[&a.card_id]],fresh.data[fresh.index[&a.card_id]]);
 assert_eq!(original.data[original.index[&a.card_id]],held);assert!(Arc::ptr_eq(&changed,&shared_card_store(&input)));}
 assert!(Arc::ptr_eq(&original,&shared_card_store(&db(a))));
}
#[test]
fn reviewer_many_same_bucket_entries_survive_live_and_roundtrip(){
 let stores:Vec<_>=(0..24).map(|i|{let c=make_test_card("REVIEW-CACHE-BUCKET",&format!("N{i:03}"));let input=db(c);let s=shared_card_store(&input);(input,s)}).collect();
 for (input,s) in &stores {assert!(Arc::ptr_eq(s,&shared_card_store(input)));assert_eq!(s.data[s.index["REVIEW-CACHE-BUCKET"]].card_name,input["REVIEW-CACHE-BUCKET"].card_name);}
 for i in 1..stores.len(){assert!(!Arc::ptr_eq(&stores[0].1,&stores[i].1));}
}
#[test]
fn reviewer_concurrent_identical_and_colliding_inputs(){
 let gate=Arc::new(Barrier::new(8));let threads:Vec<_>=(0..8).map(|i|{let gate=gate.clone();std::thread::spawn(move||{let name=if i%2==0{"Alpha"}else{"Bravo"};let input=db(make_test_card("REVIEW-CACHE-RACE",name));gate.wait();(i%2,shared_card_store(&input))})}).collect();
 let results:Vec<_>=threads.into_iter().map(|t|t.join().unwrap()).collect();
 for (kind,s) in &results {assert_eq!(s.data[s.index["REVIEW-CACHE-RACE"]].card_name,if *kind==0{"Alpha"}else{"Bravo"});
 for (other,t) in &results{assert_eq!(Arc::ptr_eq(s,t),kind==other);}}
}
