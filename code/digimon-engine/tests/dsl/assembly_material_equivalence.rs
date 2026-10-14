//! Source-shape equivalence, not a behavioral defect repro. Same YAML effect origin;
//! two explicit material metadata loaders. Runtime fixtures are synthetic exact unions.
use digimon_engine::action::{build_action_mask,PLAY_HAND_START,TRASH_EFFECT_START,PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card,DebugRunner};
use digimon_engine::enums::{CardKind,GamePhase};
const ORIGINAL:&str=include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ex12_076_assembly_equivalence/original.yaml"));
const PROPOSED:&str=include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/cards/ex12/EX12-076.yaml"));
#[derive(Debug,PartialEq)]struct Trace{masks:Vec<Vec<f32>>,memory:i16,stack:Vec<String>,trash:Vec<String>}
// Cases: 0 printed Hybrid form; 1 Mega + Rule-granted Hybrid (BT18-102);
// 2 Shambala Tamer; 3 Hybrid name only; 4 no matching trait; 5 duplicate name;
// 6 Mega without the Rule grant (negative propagation control).
fn material(i:usize,case:usize,runtime:bool)->CardData {
 let id=format!("EQ-MAT-{i}");let mut name=format!("Unique Material {i}");
 let (kind,forms,types)=if i<8{(0,vec!["Hybrid"],vec![])}else{match case{
  0=>(0,vec!["Hybrid"],vec![]),
  1=>(0,vec!["Mega","Hybrid"],vec!["Shaman"]),
  2=>(1,vec![],vec!["Shambala"]),
  3=>{name="Hybrid in name only".into();(0,vec!["Rookie"],vec!["Reptile"])},
  4=>(0,vec!["Mega"],vec!["Shaman"]),
  5=>{name="Unique Material 0".into();(0,vec!["Hybrid"],vec![])},
  6=>(0,vec!["Mega"],vec!["Shaman"]),_=>unreachable!()}};
 if runtime {
  let raw=serde_json::json!({ "card_id":id,"card_name_eng":name,"card_kind":kind,
    "card_colors":[0],"level":7,"play_cost":9,"dp":16000,
    "form_eng":forms,"type_eng":types,"attribute_eng":["Vaccine"]});
  let mut map=serde_json::Map::new();map.insert(id.clone(),raw);let raw=serde_json::Value::Object(map);
  CardData::load_from_str(&raw.to_string()).unwrap().remove(&id).unwrap()
 }else{
  let mut c=make_test_card(&id,&name);c.card_kind=if kind==1{CardKind::Tamer}else{CardKind::Digimon};
  c.traits=forms.into_iter().chain(types).chain(["Vaccine"]).map(str::to_owned).collect();c
 }
}
fn setup(yaml:&str,seat:u8,case:usize,runtime:bool)->DebugRunner{
 let mut b=DebugRunner::builder().from_dsl_yaml(yaml).unwrap().hand(seat,&["EX12-076"]);
 for i in 0..9 {b=b.add_card(material(i,case,runtime));}
 let mut r=b.start();r.skip_mulligan();r.game.turn_player_idx=seat as usize;r.game.memory_pair=(seat,1-seat);
 r.game.current_phase=GamePhase::Main;r.game.set_memory(10);
 for i in 0..9{let id=format!("EQ-MAT-{i}");let di=r.game.card_data.iter().position(|c|c.card_id==id).unwrap();
  let ci=r.game.next_card_index();r.game.players[seat as usize].trash.push(CardSource::new(di,seat,ci));}
 r
}
fn act(r:&mut DebugRunner,seat:u8,a:u16){assert_eq!(build_action_mask(&r.game,seat)[a as usize],1.0);r.game.decode_action(a,seat);}
fn resolve(yaml:&str,seat:u8,case:usize,runtime:bool,decline:bool)->Trace{
 let mut r=setup(yaml,seat,case,runtime);act(&mut r,seat,PLAY_HAND_START);let mut masks=vec![build_action_mask(&r.game,seat)];
 let expected=matches!(case,0|1|2|5);assert_eq!(masks[0][(TRASH_EFFECT_START+8)as usize],if expected{1.0}else{0.0});
 assert_eq!(masks[0][PASS as usize],1.0,"decline at zero remains optional");
 if decline{act(&mut r,seat,PASS);}else{
  let picks:Vec<u16>=if expected&&case!=5{std::iter::once(TRASH_EFFECT_START+8).chain((0..7).map(|i|TRASH_EFFECT_START+i)).collect()}else{(0..8).map(|i|TRASH_EFFECT_START+i).collect()};
  for (n,a)in picks.into_iter().enumerate(){act(&mut r,seat,a);
   if n<7{let m=build_action_mask(&r.game,seat);assert_eq!(m[PASS as usize],0.0,"partial eight cannot finish");assert!(r.game.resolve_selection(seat,PASS).is_err());
    if case==5{assert_eq!(m[(TRASH_EFFECT_START+8)as usize],0.0,"effective names must differ");}
    masks.push(m);assert_eq!(r.game.memory,10);assert!(r.game.players[seat as usize].battle_area.is_empty());}
  }
 }
 let p=&r.game.players[seat as usize];assert_eq!(p.battle_area.len(),1);
 let stack=p.battle_area[0].card_sources.iter().map(|c|c.card_id(&r.game.card_data).to_string()).collect::<Vec<_>>();
 let trash=p.trash.iter().map(|c|c.card_id(&r.game.card_data).to_string()).collect::<Vec<_>>();
 assert_eq!(stack.len(),if decline{1}else{9});assert_eq!(trash.len(),if decline{9}else{1});
 assert_eq!(r.game.memory,if decline{6}else{3},"seven payment remains own turn; declining pays16 and crosses turn");
 assert_eq!(r.game.turn_player_idx,if decline{(1-seat)as usize}else{seat as usize});
 assert!(r.game.pending_selection.is_none());Trace{masks,memory:r.game.memory,stack,trash}
}
fn eq(case:usize,seat:u8,runtime:bool,decline:bool){let a=resolve(ORIGINAL,seat,case,runtime,decline);let b=resolve(PROPOSED,seat,case,runtime,decline);assert_eq!(a,b);}
macro_rules! checks{($name:ident,$case:expr)=>{mod $name{use super::*;
 #[test]fn seat0_constructed(){eq($case,0,false,false)}#[test]fn seat1_constructed(){eq($case,1,false,false)}
 #[test]fn seat0_runtime_metadata(){eq($case,0,true,false)}#[test]fn seat1_runtime_metadata(){eq($case,1,true,false)}
}};}
checks!(printed_hybrid_form,0);checks!(mega_rule_granted_hybrid,1);checks!(shambala_tamer,2);
checks!(hybrid_name_only_refused,3);checks!(nonmatching_refused,4);checks!(duplicate_effective_name,5);checks!(missing_rule_grant_refused,6);
#[test]fn decline_zero_s0_constructed(){eq(1,0,false,true)}#[test]fn decline_zero_s1_constructed(){eq(1,1,false,true)}
#[test]fn decline_zero_s0_runtime(){eq(1,0,true,true)}#[test]fn decline_zero_s1_runtime(){eq(1,1,true,true)}
