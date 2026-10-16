//! Rule 4-21-3: cards cannot be stacked with tokens. Unexecuted staged TDD.
use digimon_engine::debug_runner::{DebugRunner,make_test_card};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor,CardSourceRef};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::token_registry::TokenDef;

fn board(token:bool)->(DebugRunner,PermanentHandle,PermanentHandle){
    let def=TokenDef{name:"blue-test".into(),card_id:"TOKEN_BLUE_TEST".into(),card_name:"Blue Token".into(),colors:vec![CardColor::Blue],dp:Some(3000),level:None,traits:vec![],keywords:vec![]};
    let observer="card: WATCH\nname: Watch\nkind: tamer\ncolor: [blue]\ncost: 0\neffects:\n  - when: on_add_digivolution_cards\n    process:\n      - gain_memory: 1\n";
    let mut r=DebugRunner::builder().from_dsl_yaml(observer).unwrap()
        .add_card(make_test_card("BODY","Body")).add_card(make_test_card("MAT","Material"))
        .add_card(def.to_card_data()).deck(0,&["MAT";20]).deck(1,&["MAT";20])
        .hand(0,&["MAT"]).security(0,&["MAT"]).memory(3).start();
    r.skip_mulligan();
    let watch=r.place_on_field(0,"WATCH",Some(0));
    let target=if token{
        r.game.token_registry.insert(def);
        let source=r.top_card(watch);
        EffectContext::new(&mut r.game,source,Some(watch),0).play_token(0,"blue-test").unwrap()
    }else{r.place_on_field(0,"BODY",Some(0))};
    r.game.drain_effect_queue();
    (r,watch,target)
}

#[test]
fn bottom_and_top_reject_token_before_hand_security_or_material_movement(){
    for token in [false,true]{for top in [false,true]{for zone in 0..3{
        let (mut r,_,target)=board(token);
        let donor=r.place_stack(0,&["MAT","BODY"]);
        let src=match zone{0=>CardSourceRef::Hand(0,0),1=>CardSourceRef::Security(0,0),_=>CardSourceRef::Material(donor,0)};
        let original=format!("{:?}",r.game.players[0]);
        let events=r.game.events.len();
        let memory=r.memory();
        let mut clone=DebugRunner::wrap(r.game.clone());
        for g in [&mut r,&mut clone]{
            let moved=if top{g.game.place_as_top_source(src,target,false)}else{g.game.place_as_bottom_source(src,target,false)};
            assert_eq!(moved,!token,"token={token}, top={top}, zone={zone}");
            g.game.drain_effect_queue();
            if token{
                assert_eq!(format!("{:?}",g.game.players[0]),original,"no source extraction, security loss, or donor change");
                assert_eq!(g.game.events.len(),events,"no movement or observer event");
                assert_eq!(g.memory(),memory,"no source-added observer");
            }else{
                assert_eq!(g.game.players[0].battle_area[target.index as usize].card_sources.len(),2);
                assert_eq!(g.memory(),memory+1,"positive observer control");
            }
        }
    }}}
}

#[test]
fn whole_permanent_and_field_top_reject_token_before_departure(){
    for token in [false,true]{for whole in [false,true]{
        let (mut r,_,target)=board(token);
        let donor=r.place_stack(0,&["MAT","BODY"]);
        let top=r.top_card(donor);
        let original=format!("{:?}",r.game.players[0]);
        let mut clone=DebugRunner::wrap(r.game.clone());
        for g in [&mut r,&mut clone]{
            let moved=if whole{g.game.place_permanent_as_bottom_sources(donor,target)}else{g.game.place_as_bottom_source(CardSourceRef::Material(donor,1),target,false)};
            assert_eq!(moved,!token);
            g.game.drain_effect_queue();
            if token{assert_eq!(format!("{:?}",g.game.players[0]),original);assert!(g.pending_selection().is_none());}
            else{assert!(g.game.players[0].battle_area[target.index as usize].card_sources.iter().any(|c|c.handle()==top));}
        }
    }}
}

#[test]
fn direct_card_and_training_deck_paths_preserve_token_and_source_zones(){
    for token in [false,true]{for training in [false,true]{
        let (mut r,watch,target)=board(token);
        let card=r.game.players[0].hand[0].handle();
        let source=r.top_card(watch);
        let original=format!("{:?}",r.game.players[0]);
        let memory=r.memory();
        let mut clone=DebugRunner::wrap(r.game.clone());
        for g in [&mut r,&mut clone]{
            let mut ctx=EffectContext::new(&mut g.game,source,Some(watch),0);
            if training{ctx.training_place_deck_top_under_self_face_down(target)}else{ctx.place_card_under_permanent_bottom(card,target,false)};
            g.game.drain_effect_queue();
            if token{assert_eq!(format!("{:?}",g.game.players[0]),original);assert_eq!(g.memory(),memory);}
            else{assert_eq!(g.game.players[0].battle_area[target.index as usize].card_sources.len(),2);assert_eq!(g.memory(),memory+1);}
        }
    }}
}

#[test]
fn mindlink_attach_rejects_token_before_tamer_cleanup(){
    for token in [false,true]{
        let (mut r,watch,target)=board(token);
        let source=r.top_card(watch);
        let original=format!("{:?}",r.game.players[0]);
        let mut clone=DebugRunner::wrap(r.game.clone());
        for g in [&mut r,&mut clone]{
            EffectContext::new(&mut g.game,source,Some(watch),0).attach_tamer_to_digimon(watch,target);
            g.game.drain_effect_queue();
            if token{assert_eq!(format!("{:?}",g.game.players[0]),original);}
            else{assert_eq!(g.game.players[0].battle_area.len(),1);assert_eq!(g.game.players[0].battle_area[0].card_sources.len(),2);}
        }
    }
}


// Adversarial continuation tests: deliberately change only the destination's
// token flag while a real engine window is parked. No legal card transformation
// to token is asserted. Genuine registry token entry paths are covered above.
use std::sync::Arc;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::card_source::CardHandle;
use digimon_engine::action::{build_action_mask,space::PASS};

fn board_deferred_security()->(DebugRunner,PermanentHandle,PermanentHandle) {
    let observer="card: WATCH\nname: Watch\nkind: tamer\ncolor: [blue]\ncost: 0\neffects:\n  - when: on_add_digivolution_cards\n    process:\n      - gain_memory: 1\n";
    let security=r#"
card: MAT
name: Material
kind: digimon
color: [blue]
level: 3
cost: 3
dp: 2000
effects:
  - when: on_lose_security
    scope: security
    process:
      - select_hand:
          of: you
          filter: {}
          optional: false
          bind_as: witness
          prompt: "Resume security destination"
"#;
    let mut r=DebugRunner::builder().from_dsl_yaml(observer).unwrap()
        .from_dsl_yaml(security).unwrap().add_card(make_test_card("BODY","Body"))
        .deck(0,&["MAT";20]).deck(1,&["MAT";20]).hand(0,&["MAT"])
        .security(0,&["MAT"]).memory(3).start();
    r.skip_mulligan();let watch=r.place_on_field(0,"WATCH",Some(0));
    let target=r.place_on_field(0,"BODY",Some(0));
    r.game.drain_effect_queue();(r,watch,target)
}
struct LeavePause;
impl CardEffect for LeavePause {
    fn effects(&self,card:CardHandle)->Vec<Effect> {
        vec![Effect::when_would_leave_battle_area(card).optional()
            .replacement_process(|ctx|ctx.cancel()).build()]
    }
}
fn physical_count(r:&DebugRunner,h:CardHandle)->usize {
    r.game.players.iter().map(|p| {
        [&p.hand,&p.trash,&p.deck,&p.security,&p.digitama_deck].into_iter()
            .map(|z|z.iter().filter(|c|c.handle()==h).count()).sum::<usize>()
        +p.battle_area.iter().map(|b|b.card_sources.iter().filter(|c|c.handle()==h).count()).sum::<usize>()
        +p.breeding_area.iter().map(|b|b.card_sources.iter().filter(|c|c.handle()==h).count()).sum::<usize>()
    }).sum::<usize>()
    +r.game.pending_security.as_ref().map_or(0,|p|usize::from(p.card.handle()==h))
}
fn resolve_first_legal(r:&mut DebugRunner) {
    let v=r.pending_selection_view().expect("real parked choice");
    let a=*v.valid_action_ids.iter().find(|&&a|a!=PASS).unwrap();
    assert_eq!(build_action_mask(&r.game,v.selecting_player)[a as usize],1.0);
    r.execute_action(v.selecting_player,a).unwrap();r.game.drain_effect_queue();
}
#[test]
fn deferred_security_destination_token_falls_back_once_to_owner_trash() {
    for mutate in [false,true] {for top in [false,true] {
        let (mut r,_,target)=board_deferred_security();
        let security=r.game.players[0].security[0].handle();
        let memory=r.memory();
        let source=CardSourceRef::Security(0,0);
        let _started=if top {r.game.place_as_top_source(source,target,false)}else{r.game.place_as_bottom_source(source,target,false)};
        assert!(r.pending_selection().is_some(),"must reach deferred security window");
        assert!(r.game.pending_security.is_some());
        // Resumable DSL selection survives cloning; destination mutation is
        // an external adversarial probe, not a raw callback effect.
        r.game.players[0].battle_area[target.index as usize]
            .card_sources.last_mut().unwrap().is_token=mutate;
        let mut cloned=DebugRunner::wrap(r.game.clone());
        for g in [&mut r,&mut cloned] {
            resolve_first_legal(g);
            assert!(g.pending_selection().is_none());assert!(g.game.pending_security.is_none());
            assert!(g.game.pending_effect_security_removal.is_empty());
            assert_eq!(physical_count(g,security),1,"no duplication or loss across continuation");
            let host=&g.game.players[0].battle_area[target.index as usize];
            assert_eq!(host.card_sources.len(),if mutate{1}else{2});
            assert_eq!(g.game.players[0].trash.iter().filter(|c|c.handle()==security).count(),usize::from(mutate));
            assert_eq!(g.memory(),memory+if mutate{0}else{1},"observer only when seated");
        }
    }}
}
#[test]
fn deferred_field_top_rechecks_token_before_donor_commit_in_clone() {
    for mutate in [false,true] {for top in [false,true] {
        let (mut r,_,target)=board(false);
        r.register_effect("MAT",Arc::new(LeavePause));
        let donor=r.place_stack(0,&["BODY","MAT"]);let top_card=r.top_card(donor);
        let lower=r.game.players[0].battle_area[donor.index as usize].card_sources[0].handle();
        let memory=r.memory();
        let source=CardSourceRef::Material(donor,1);
        let _started=if top {r.game.place_as_top_source(source,target,false)}else{r.game.place_as_bottom_source(source,target,false)};
        assert!(r.pending_selection().is_some(),"must reach real replacement gate");
        let mut cloned=DebugRunner::wrap(r.game.clone());
        for g in [&mut r,&mut cloned] {
            g.game.players[0].battle_area[target.index as usize].card_sources.last_mut().unwrap().is_token=mutate;
            let view=g.pending_selection_view().unwrap();
            assert_eq!(build_action_mask(&g.game,view.selecting_player)[PASS as usize],1.0);
            g.execute_action(view.selecting_player,PASS).unwrap();g.game.drain_effect_queue();
            assert!(g.pending_selection().is_none());
            assert_eq!(physical_count(g,top_card),1);assert_eq!(physical_count(g,lower),1);
            assert_eq!(g.game.players[0].battle_area[target.index as usize].card_sources.len(),if mutate{1}else{2});
            assert_eq!(g.game.players[0].battle_area.len(),if mutate{3}else{2});
            if mutate {
                assert_eq!(g.game.players[0].battle_area[donor.index as usize].top_card().handle(),top_card);
                assert_eq!(g.game.players[0].battle_area[donor.index as usize].card_sources.len(),2);
            } else {assert!(g.game.players[0].trash.iter().any(|c|c.handle()==lower));}
            assert_eq!(g.memory(),memory+if mutate{0}else{1},"observer only when physically seated");
        }
    }}
}
