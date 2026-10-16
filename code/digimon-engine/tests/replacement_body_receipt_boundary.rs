//! Paid-deletion result must run after Decode's payer commit and before parent leave.
use digimon_engine::{Game,debug_runner::{DebugRunner,make_test_card}};
use digimon_engine::action::{mask::build_action_mask,space::{PASS,REPLACEMENT_ACCEPT,encode_source_select,encode_attack}};
use digimon_engine::selection::SelectionKind;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::enums::Keyword;
fn board(guard:bool,evade:bool)->DebugRunner { board_detail(guard,evade,false) }
fn board_detail(guard:bool,evade:bool,repark:bool)->DebugRunner {
 let grant=if guard {"  - kind: grant_keyword\n    keyword: Guard\n    summary: '<Guard>'\n"}else{""};
 let extra=if evade {"  - kind: grant_keyword\n    keyword: Evade\n    summary: '<Evade>'\n"}else{""};
 let yaml=format!(r#"card: PAYER
name: Payer
kind: digimon
level: 5
color: [blue]
cost: 7
dp: 7000
effects:
{grant}{extra}  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - none_of:
            - replacement_cause: battle
    summary: Decode source
    process:
      - select_material:
          of_permanent: replacement_subject
          bind_as: picked
          filter: {{ name_is: Source }}
          prompt: Decode source
      - play_from_materials:
          target: replacement_subject
          source_index: picked
          cost_delta: free
"#);
 let yaml=if repark {yaml.replace("          cost_delta: free", "          cost_delta: free\n      - select_own_permanent:\n          filter: { name_is: Source }\n          bind_as: finished\n          prompt: Finish Decode body") }else{yaml};
 let mut subject=make_test_card("SUBJECT","Subject");if !guard {subject.keywords=vec![Keyword::Scapegoat];}
 let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap().add_card(make_test_card("SOURCE","Source")).add_card(subject).start();
 r.place_stack(0,&["SOURCE","PAYER"]);r.place_on_field(0,"SUBJECT",Some(0));r.game.tick_declarative_effects();r
}
fn choose(g:&mut Game,a:u16){let s=g.pending_selection.as_ref().expect("explicit decision");assert_eq!(s.selecting_player,0);if a!=PASS {assert!(s.valid_action_ids.contains(&a));}else{assert!(s.is_optional);}assert!(build_action_mask(g,0)[a as usize]>0.0);g.resolve_selection(0,a).unwrap();}
fn replacement(g:&Game,label:&str){let s=g.pending_selection.as_ref().expect(label);assert_eq!(s.kind,SelectionKind::Replacement);assert!(s.prompt.contains(label),"{}",s.prompt);assert!(s.is_optional);assert!(build_action_mask(g,0)[PASS as usize]>0.0);assert!(s.valid_action_ids.contains(&REPLACEMENT_ACCEPT));}
fn endpoint(g:&Game,field:&[&str],trash:&[&str]){assert!(g.pending_selection.is_none());let mut f:Vec<_>=g.players[0].battle_area.iter().map(|p|p.top_card().card_id(&g.card_data)).collect();f.sort();let mut want=field.to_vec();want.sort();assert_eq!(f,want);let mut t:Vec<_>=g.players[0].trash.iter().map(|c|c.card_id(&g.card_data)).collect();t.sort();let mut want=trash.to_vec();want.sort();assert_eq!(t,want);}
fn begin(r:&mut DebugRunner,guard:bool){let subject=digimon_engine::permanent::PermanentHandle{player:0,index:1};r.game.delete_permanent_with_cause(subject,ReplacementCause::OpponentEffect);replacement(&r.game,if guard {"<Guard>"}else{"<Scapegoat>"});choose(&mut r.game,REPLACEMENT_ACCEPT);if !guard {choose(&mut r.game,encode_attack(0,0));}}
#[test]
fn guard_decode_accept_and_decline_settle_payment_before_parent_in_clones(){
 for decline in [false,true] {let mut r=board(true,false);begin(&mut r,true);replacement(&r.game,"when_would_leave_battle_area");let clone=r.game.clone();for mut g in [r.game,clone] {if decline {choose(&mut g,PASS);endpoint(&g,&["SUBJECT"],&["PAYER","SOURCE"]);}else{choose(&mut g,REPLACEMENT_ACCEPT);let pick=encode_source_select(0,0).unwrap();let clone=g.clone();for mut h in [g,clone]{choose(&mut h,pick);endpoint(&h,&["SUBJECT","SOURCE"],&["PAYER"]);}}}}
}
#[test]
fn scapegoat_selected_decode_payer_saves_original_in_clones(){let mut r=board(false,false);begin(&mut r,false);replacement(&r.game,"when_would_leave_battle_area");choose(&mut r.game,REPLACEMENT_ACCEPT);let clone=r.game.clone();for mut g in [r.game,clone]{choose(&mut g,encode_source_select(0,0).unwrap());endpoint(&g,&["SUBJECT","SOURCE"],&["PAYER"]);}}
#[test]
fn guard_decode_then_evade_failed_payment_does_not_save_parent(){let mut r=board(true,true);begin(&mut r,true);let s=r.game.pending_selection.as_ref().unwrap();assert_eq!(s.kind,SelectionKind::EffectChoice);assert!(!s.is_optional);let rows=s.effect_choices.as_ref().unwrap();assert_eq!(rows.len(),2);let choices:Vec<_>=rows.iter().filter(|x|x.label.contains("when_would_leave_battle_area")||x.label.contains("Decode")).collect();assert_eq!(choices.len(),1);let action=choices[0].action_id;choose(&mut r.game,action);replacement(&r.game,"when_would_leave_battle_area");choose(&mut r.game,REPLACEMENT_ACCEPT);choose(&mut r.game,encode_source_select(0,0).unwrap());replacement(&r.game,"<Evade>");let clone=r.game.clone();for mut g in [r.game,clone]{choose(&mut g,REPLACEMENT_ACCEPT);endpoint(&g,&["PAYER","SOURCE"],&["SUBJECT"]);assert!(g.players[0].battle_area.iter().find(|p|p.top_card().card_id(&g.card_data)=="PAYER").unwrap().is_suspended);}}

#[test]
fn guard_payment_waits_for_further_decode_body_prompt_in_clones(){
 let mut r=board_detail(true,false,true);begin(&mut r,true);replacement(&r.game,"when_would_leave_battle_area");choose(&mut r.game,REPLACEMENT_ACCEPT);choose(&mut r.game,encode_source_select(0,0).unwrap());
 let s=r.game.pending_selection.as_ref().expect("further body choice");assert!(s.prompt.contains("Finish Decode body"));assert_eq!(s.kind,SelectionKind::OwnField);assert!(!s.is_optional);let id=r.game.players[0].battle_area.iter().position(|p|p.top_card().card_id(&r.game.card_data)=="SOURCE").unwrap();let action=encode_attack(0,id as u16);assert_eq!(s.valid_action_ids,vec![action]);
 let before:Vec<_>=r.game.players[0].battle_area.iter().map(|p|p.top_card().card_id(&r.game.card_data)).collect();assert!(before.contains(&"PAYER")&&before.contains(&"SUBJECT"),"payment and original departure wait for the body");
 let clone=r.game.clone();for mut g in [r.game,clone]{choose(&mut g,action);endpoint(&g,&["SOURCE","SUBJECT"],&["PAYER"]);}
}
#[test]
fn ordinary_deletion_receipt_counts_decode_deletion_after_material_selection(){
 const DRIVER:&str=r#"
card: DRIVER
name: Driver
kind: digimon
color: [red]
level: 6
cost: 12
dp: 12000
effects:
  - when: on_play
    process:
      - delete_all_permanents:
          over: { of: opponent, kind: digimon, zone: [battle_area] }
      - if:
          condition: { effect_deleted_any_opponent_digimon: true }
          then:
            - draw: { of: you, count: 1 }
          else:
            - draw: { of: you, count: 2 }
"#;
 const DECODE:&str=r#"
card: DECODE
name: Decode
kind: digimon
color: [blue]
level: 5
cost: 7
dp: 7000
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when: { replacement_subject_is_mine: true }
    process:
      - select_material:
          of_permanent: replacement_subject
          bind_as: picked
          filter: { name_is: Source }
          prompt: Ordinary Decode source
      - play_from_materials:
          target: replacement_subject
          source_index: picked
          cost_delta: free
"#;
 let mut r=DebugRunner::builder().from_dsl_yaml(DRIVER).unwrap().from_dsl_yaml(DECODE).unwrap().add_card(make_test_card("SOURCE","Source")).add_card(make_test_card("FILL","Fill")).deck(0,&["FILL";20]).deck(1,&["FILL";20]).start();r.skip_mulligan();let driver=r.place_on_field(0,"DRIVER",None);r.place_stack(1,&["SOURCE","DECODE"]);let hand=r.game.players[0].hand.len();r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::OnPlay,digimon_engine::selection::TriggerSource::Permanent(driver));r.game.drain_effect_queue();
 let s=r.game.pending_selection.as_ref().unwrap();assert_eq!(s.selecting_player,1);assert_eq!(s.kind,SelectionKind::Replacement);assert!(build_action_mask(&r.game,1)[REPLACEMENT_ACCEPT as usize]>0.0);r.game.resolve_selection(1,REPLACEMENT_ACCEPT).unwrap();let a=encode_source_select(0,0).unwrap();assert_eq!(r.game.players[0].hand.len(),hand,"result branch waits for payer body");let clone=r.game.clone();for mut g in [r.game,clone]{let s=g.pending_selection.as_ref().unwrap();assert_eq!(s.selecting_player,1);assert!(s.prompt.contains("Ordinary Decode source"));assert!(s.valid_action_ids.contains(&a));assert!(build_action_mask(&g,1)[a as usize]>0.0);g.resolve_selection(1,a).unwrap();assert!(g.pending_selection.is_none());assert_eq!(g.players[0].hand.len(),hand+1,"actual receipt records original Decode deletion exactly once");assert_eq!(g.players[1].battle_area.len(),1);assert_eq!(g.players[1].battle_area[0].top_card().card_id(&g.card_data),"SOURCE");let trash:Vec<_>=g.players[1].trash.iter().map(|c|c.card_id(&g.card_data)).collect();assert_eq!(trash,vec!["DECODE"]);}
}
