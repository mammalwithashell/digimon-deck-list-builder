//! Bounded source-predicate regression probes using complete copied subject YAML.
//! Synthetic non-Digimon/name/level witnesses distinguish printed grammar only;
//! they do not assert the existence or legality of corresponding real printings.
//! All actions use the public mask and both controller seats are separate tests.
use digimon_engine::action::{build_action_mask, space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START}};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, GamePhase};

fn yaml(id: &str) -> String {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards");
    let set = id.split('-').next().unwrap().to_lowercase();
    std::fs::read_to_string(std::path::Path::new(&root).join(set).join(format!("{id}.yaml"))).unwrap()
}
fn witness(id: &str, name: &str, kind: CardKind, level: Option<u8>, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = kind; c.level = level; c.traits = traits.iter().map(|s|s.to_string()).collect();
    c.play_cost = 3; c.dp = Some(4000); c
}
fn runner(id: &str, seat: u8, cards: Vec<CardData>) -> DebugRunner {
    let mut b = DebugRunner::builder().from_dsl_yaml(&yaml(id)).expect("complete subject YAML compiles");
    for c in cards { b = b.add_card(c); }
    let mut r = b.hand(seat, &[id]).memory(10).start();
    r.game.turn_player_idx = seat as usize; r.game.current_phase = GamePhase::Main; r.game.set_memory(10); r
}
fn trash(r: &mut DebugRunner, seat: u8, id: &str) {
    let data = r.game.card_data.iter().position(|c|c.card_id == id).unwrap();
    let idx = r.game.next_card_index(); r.game.players[seat as usize].trash.push(CardSource::new(data, seat, idx));
}
fn act(r: &mut DebugRunner, seat: u8, a: u16) {
    assert_eq!(build_action_mask(&r.game, seat)[a as usize],1.0,"seat={seat}, action={a}, pending={:?}",r.pending_selection_view());
    r.game.decode_action(a,seat);
}
fn prompt(r: &DebugRunner, seat: u8, expected: &[u16]) {
    let p = r.pending_selection_view().expect("printed Assembly eligibility must surface a material choice before payment");
    assert_eq!(p.selecting_player,seat); assert_eq!(p.valid_action_ids,expected);
    assert_eq!(r.memory(),10,"choose immediately before paying"); assert_eq!(r.battle_area_size(seat),0);
    assert!(build_action_mask(&r.game,1-seat).iter().all(|v|*v==0.0),"opponent cannot answer own Assembly");
}
fn sources(r: &DebugRunner, seat: u8) -> Vec<String> {
    r.game.players[seat as usize].battle_area[0].card_sources.iter().map(|s|s.card_id(&r.game.card_data).to_string()).collect()
}
fn one(id: &str, seat: u8, kind: CardKind, trait_name: &str) {
    let mut r = runner(id,seat,vec![witness("MAT","Material",kind,Some(4),&[trait_name])]);
    trash(&mut r,seat,"MAT"); act(&mut r,seat,PLAY_HAND_START); prompt(&r,seat,&[TRASH_EFFECT_START]);
    assert!(r.pending_selection_view().unwrap().is_optional);
    let mut clone = DebugRunner::wrap(r.game.clone()); act(&mut clone,seat,TRASH_EFFECT_START);
    assert!(clone.pending_selection().is_none()); assert_eq!(clone.memory(),5,"seven minus Assembly two");
    assert_eq!(sources(&clone,seat),vec!["MAT",id]); assert_eq!(clone.trash_size(seat),0);
    assert_eq!(r.trash_size(seat),1,"cloned selection leaves original unchanged");
    act(&mut r,seat,PASS); assert!(r.pending_selection().is_none()); assert_eq!(r.memory(),3);
    assert_eq!(sources(&r,seat),vec![id]); assert_eq!(r.trash_size(seat),1);
}
fn named(seat: u8, first: CardKind, second: CardKind) {
    let mut r = runner("BT24-081",seat,vec![witness("TITA","Titamon",first,Some(6),&[]),witness("SKULL","SkullBaluchimon",second,Some(5),&[])]);
    trash(&mut r,seat,"TITA"); trash(&mut r,seat,"SKULL"); act(&mut r,seat,PLAY_HAND_START);
    prompt(&r,seat,&[TRASH_EFFECT_START]); assert!(r.pending_selection_view().unwrap().is_optional);
    act(&mut r,seat,TRASH_EFFECT_START); prompt(&r,seat,&[TRASH_EFFECT_START+1]);
    act(&mut r,seat,TRASH_EFFECT_START+1); assert!(r.pending_selection().is_none());
    assert_eq!(r.memory(),2,"fourteen minus Assembly six"); assert_eq!(r.trash_size(seat),0);
    let actual = sources(&r,seat); assert_eq!(actual.last().unwrap(),"BT24-081");
    let mut material_ids=actual[..2].to_vec(); material_ids.sort(); assert_eq!(material_ids,vec!["SKULL","TITA"]);
    // Existing multi-material ordering/capacity obligations remain separately unresolved.
}
fn single_filters(id: &str, seat: u8, good_trait: &str) {
    let cases=[("GOOD",Some(4),good_trait),("HIGH",Some(5),good_trait),("NONE",None,good_trait),("WRONG",Some(4),"Dragon"),("PARTIAL",Some(4),if good_trait=="TB"{"TB-like"}else{"Puppet-like"})];
    let mut b=DebugRunner::builder().from_dsl_yaml(&yaml(id)).unwrap();
    for (name,level,t) in cases {b=b.add_card(witness(name,name,CardKind::Digimon,level,&[t]));}
    let mut r=b.hand(seat,&[id,"GOOD"]).hand(1-seat,&["GOOD"]).deck(seat,&["GOOD"]).security(seat,&["GOOD"]).memory(10).start();
    r.game.turn_player_idx=seat as usize;r.game.current_phase=GamePhase::Main;r.game.set_memory(10);
    for (name,_,_) in cases {trash(&mut r,seat,name);} trash(&mut r,1-seat,"GOOD");
    act(&mut r,seat,PLAY_HAND_START);prompt(&r,seat,&[TRASH_EFFECT_START]);
    let mask=build_action_mask(&r.game,seat);
    for n in 1..cases.len(){assert_eq!(mask[TRASH_EFFECT_START as usize+n],0.0,"excluded wrong material {n}");}
    assert_eq!(mask[PASS as usize],1.0,"printed Assembly may be declined");
}
fn named_filters(seat:u8){
    let names=[("TITA","Titamon"),("SKULL","SkullBaluchimon"),("PARTIAL-T","Titamon + SkullBaluchimon"),("PARTIAL-S","SkullBaluchimon-like"),("WRONG","Agumon")];
    let cards=names.iter().map(|(id,name)|witness(id,name,CardKind::Digimon,Some(6),&[])).collect();
    let mut r=runner("BT24-081",seat,cards);
    for (id,_) in names{trash(&mut r,seat,id);}trash(&mut r,1-seat,"TITA");trash(&mut r,1-seat,"SKULL");
    act(&mut r,seat,PLAY_HAND_START);prompt(&r,seat,&[TRASH_EFFECT_START]);
    let m=build_action_mask(&r.game,seat);for i in 1..names.len(){assert_eq!(m[TRASH_EFFECT_START as usize+i],0.0);}
    act(&mut r,seat,TRASH_EFFECT_START);prompt(&r,seat,&[TRASH_EFFECT_START+1]);
    let m=build_action_mask(&r.game,seat);for i in [0,2,3,4]{assert_eq!(m[TRASH_EFFECT_START as usize+i],0.0);}
}
fn named_decline(seat:u8){
    let mut r=runner("BT24-081",seat,vec![witness("TITA","Titamon",CardKind::Digimon,Some(6),&[]),witness("SKULL","SkullBaluchimon",CardKind::Digimon,Some(5),&[])]);
    trash(&mut r,seat,"TITA");trash(&mut r,seat,"SKULL");act(&mut r,seat,PLAY_HAND_START);prompt(&r,seat,&[TRASH_EFFECT_START]);
    act(&mut r,seat,PASS);assert!(r.pending_selection().is_none());assert_eq!(sources(&r,seat),vec!["BT24-081"]);assert_eq!(r.trash_size(seat),2);
}
macro_rules! single_test {($name:ident,$id:literal,$seat:expr,$kind:ident,$t:literal)=>{#[test]fn $name(){one($id,$seat,CardKind::$kind,$t);}};}
macro_rules! named_test {($name:ident,$seat:expr,$first:ident,$second:ident)=>{#[test]fn $name(){named($seat,CardKind::$first,CardKind::$second);}};}
named_test!(bt24_tamer_option_s0,0,Tamer,Option); named_test!(bt24_tamer_option_s1,1,Tamer,Option);
named_test!(bt24_option_tamer_s0,0,Option,Tamer); named_test!(bt24_option_tamer_s1,1,Option,Tamer);
named_test!(bt24_existing_digimon_s0,0,Digimon,Digimon); named_test!(bt24_existing_digimon_s1,1,Digimon,Digimon);
single_test!(ex046_tamer_s0,"EX12-046",0,Tamer,"TB");single_test!(ex046_tamer_s1,"EX12-046",1,Tamer,"TB");
single_test!(ex046_option_s0,"EX12-046",0,Option,"TB");single_test!(ex046_option_s1,"EX12-046",1,Option,"TB");
single_test!(ex046_existing_digimon_s0,"EX12-046",0,Digimon,"TB");single_test!(ex046_existing_digimon_s1,"EX12-046",1,Digimon,"TB");
single_test!(ex063_puppet_tamer_s0,"EX12-063",0,Tamer,"Puppet");single_test!(ex063_puppet_tamer_s1,"EX12-063",1,Tamer,"Puppet");
single_test!(ex063_tb_option_s0,"EX12-063",0,Option,"TB");single_test!(ex063_tb_option_s1,"EX12-063",1,Option,"TB");
single_test!(ex063_existing_digimon_s0,"EX12-063",0,Digimon,"Puppet");single_test!(ex063_existing_digimon_s1,"EX12-063",1,Digimon,"Puppet");
single_test!(ex031_reuse_aqua_s0,"EX12-031",0,Tamer,"Aquabeast");single_test!(ex031_reuse_aqua_s1,"EX12-031",1,Tamer,"Aquabeast");
single_test!(ex031_reuse_sea_s0,"EX12-031",0,Option,"Ancient Sea Animal");single_test!(ex031_reuse_sea_s1,"EX12-031",1,Option,"Ancient Sea Animal");
#[test]fn ex046_filters_s0(){single_filters("EX12-046",0,"TB");}#[test]fn ex046_filters_s1(){single_filters("EX12-046",1,"TB");}
#[test]fn ex063_puppet_filters_s0(){single_filters("EX12-063",0,"Puppet");}#[test]fn ex063_puppet_filters_s1(){single_filters("EX12-063",1,"Puppet");}
#[test]fn ex063_tb_filters_s0(){single_filters("EX12-063",0,"TB");}#[test]fn ex063_tb_filters_s1(){single_filters("EX12-063",1,"TB");}
#[test]fn bt24_name_filters_s0(){named_filters(0);}#[test]fn bt24_name_filters_s1(){named_filters(1);}
#[test]fn bt24_decline_s0(){named_decline(0);}#[test]fn bt24_decline_s1(){named_decline(1);}
