//! Scapegoat rules16-31: carrier-only trigger; actual own-effect deletion payment.
use digimon_engine::{Game, debug_runner::{DebugRunner, make_test_card}};
use digimon_engine::action::{mask::build_action_mask, space::{PASS,REPLACEMENT_ACCEPT,encode_attack}};
use digimon_engine::enums::{Keyword,ModifierType,Expiry,CardKind};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::permanent::PermanentHandle;
fn board(ally_keyword:Option<Keyword>, reverse:bool)->(DebugRunner,PermanentHandle,PermanentHandle){
 let mut scap=make_test_card("SCAP","SCAP");scap.keywords=vec![Keyword::Scapegoat];
 let mut ally=make_test_card("ALLY","ALLY");ally.keywords=ally_keyword.into_iter().collect();
 let mut r=DebugRunner::builder().add_card(scap).add_card(ally).start();
 let (s,a)=if reverse {let a=r.place_on_field(0,"ALLY",Some(0));let s=r.place_on_field(0,"SCAP",Some(0));(s,a)}else{let s=r.place_on_field(0,"SCAP",Some(0));let a=r.place_on_field(0,"ALLY",Some(0));(s,a)};(r,s,a)
}
fn resolve(g:&mut Game,a:u16){let p=g.pending_selection.as_ref().expect("explicit pending decision").selecting_player;assert!(build_action_mask(g,p)[a as usize]>0.0,"action {a} must be legal");g.resolve_selection(p,a).unwrap();}
fn endpoint(g:&Game,field:&[&str],trash:&[&str]){
 assert!(g.pending_selection.is_none(),"unexpected unresolved prompt: {:?}",g.pending_selection);
 let mut f:Vec<_>=g.players[0].battle_area.iter().map(|p|p.top_card().card_id(&g.card_data)).collect();f.sort();let mut e=field.to_vec();e.sort();assert_eq!(f,e);
 let mut t:Vec<_>=g.players[0].trash.iter().map(|c|c.card_id(&g.card_data)).collect();t.sort();let mut e=trash.to_vec();e.sort();assert_eq!(t,e);
}
fn accept_and_check_picker(r:&mut DebugRunner,s:PermanentHandle,a:PermanentHandle)->u16{
 r.game.delete_permanent_with_cause(s,ReplacementCause::OpponentEffect);
 assert!(build_action_mask(&r.game,0)[PASS as usize]>0.0);
 resolve(&mut r.game,REPLACEMENT_ACCEPT);
 let action=encode_attack(0,a.index as u16);
 let p=r.game.pending_selection.as_ref().expect("mandatory physical payer choice");assert_eq!(p.selecting_player,0);assert!(!p.is_optional);assert_eq!(p.valid_action_ids,vec![action]);
 assert_eq!(build_action_mask(&r.game,0)[PASS as usize],0.0);action
}
#[test] fn unrelated_subject_never_offers_carriers_scapegoat(){for reverse in [false,true]{let(mut r,_s,a)=board(None,reverse);r.game.delete_permanent_with_cause(a,ReplacementCause::OpponentEffect);endpoint(&r.game,&["SCAP"],&["ALLY"]);}}
#[test] fn successful_payment_and_clone_finish_exactly(){for reverse in [false,true]{let(mut r,s,a)=board(None,reverse);let action=accept_and_check_picker(&mut r,s,a);let mut cloned=r.game.clone();resolve(&mut cloned,action);assert!(r.game.pending_selection.is_some());resolve(&mut r.game,action);endpoint(&cloned,&["SCAP"],&["ALLY"]);endpoint(&r.game,&["SCAP"],&["ALLY"]);}}
#[test] fn opponent_only_protection_does_not_block_own_effect_payment(){let(mut r,s,a)=board(None,false);let mut protection=ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect,0,Expiry::Permanent,0);protection.cause_filter=Some(ReplacementCause::OpponentEffect);r.game.modifiers.add(a,protection);let action=accept_and_check_picker(&mut r,s,a);r.game.set_effect_source_player_for_test(Some(1));let mut clone=r.game.clone();resolve(&mut clone,action);resolve(&mut r.game,action);endpoint(&clone,&["SCAP"],&["ALLY"]);endpoint(&r.game,&["SCAP"],&["ALLY"]);}
#[test] fn immunity_added_while_picker_parked_does_not_pay(){let(mut r,s,a)=board(None,false);let action=accept_and_check_picker(&mut r,s,a);r.game.modifiers.add(a,ModifierEntry::simple(ModifierType::CannotBeDestroyed,0,Expiry::Permanent,0));let mut cloned=r.game.clone();resolve(&mut cloned,action);resolve(&mut r.game,action);endpoint(&cloned,&["ALLY"],&["SCAP"]);endpoint(&r.game,&["ALLY"],&["SCAP"]);}
#[test] fn parked_payment_evade_both_branches_clone_to_actual_result(){for decline in [false,true]{let(mut r,s,a)=board(Some(Keyword::Evade),false);let action=accept_and_check_picker(&mut r,s,a);resolve(&mut r.game,action);assert_eq!(r.game.players[0].battle_area.len(),2,"payment still pending");let pending=r.game.pending_selection.as_ref().expect("ally Evade payment replacement");assert!(pending.is_optional);let mut clone=r.game.clone();let answer=if decline{PASS}else{REPLACEMENT_ACCEPT};resolve(&mut clone,answer);resolve(&mut r.game,answer);if decline{endpoint(&r.game,&["SCAP"],&["ALLY"]);endpoint(&clone,&["SCAP"],&["ALLY"]);}else{endpoint(&r.game,&["ALLY"],&["SCAP"]);endpoint(&clone,&["ALLY"],&["SCAP"]);assert!(r.game.players[0].battle_area[0].is_suspended);}}}
#[test] fn known_undeletable_payer_may_be_attempted_but_does_not_pay(){let(mut r,s,a)=board(None,false);r.game.modifiers.add(a,ModifierEntry::simple(ModifierType::CannotBeDestroyed,0,Expiry::Permanent,0));let action=accept_and_check_picker(&mut r,s,a);resolve(&mut r.game,action);endpoint(&r.game,&["ALLY"],&["SCAP"]);}
#[test] fn own_effect_payment_cannot_chain_another_scapegoat(){let(mut r,s,a)=board(Some(Keyword::Scapegoat),false);let action=accept_and_check_picker(&mut r,s,a);resolve(&mut r.game,action);endpoint(&r.game,&["SCAP"],&["ALLY"]);}

#[test]
fn rules_digimon_kind_can_pay_even_when_printed_kind_differs() {
    for kind in [CardKind::DigiEgg, CardKind::Tamer] {
        let mut scap = make_test_card("SCAP", "SCAP");
        scap.keywords = vec![Keyword::Scapegoat];
        let mut ally = make_test_card("ALLY", "ALLY");
        ally.card_kind = kind;
        let mut r = DebugRunner::builder().add_card(scap).add_card(ally).start();
        let s = r.place_on_field(0, "SCAP", Some(0));
        let a = r.place_on_field(0, "ALLY", Some(0));
        if kind == CardKind::Tamer {
            let mut entry = ModifierEntry::simple(ModifierType::TreatAsDigimon, 0, Expiry::Permanent, 0);
            entry.payload = digimon_engine::modifiers::ModifierPayload::AdditiveSynthIdentity {
                kind: CardKind::Digimon, level: 0, colors: vec![], traits: vec![], dp: 5000,
            };
            r.game.modifiers.add(a, entry);
        }
        assert!(r.game.permanent_is_digimon_for_rules(a));
        let action = accept_and_check_picker(&mut r, s, a);
        let mut cloned = r.game.clone();
        resolve(&mut cloned, action);
        resolve(&mut r.game, action);
        endpoint(&cloned, &["SCAP"], &["ALLY"]);
        endpoint(&r.game, &["SCAP"], &["ALLY"]);
    }
}

#[test]
fn token_disappearance_is_successful_deletion_payment_in_clones() {
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::CardColor;
    use digimon_engine::token_registry::TokenDef;
    let token = TokenDef { name: "payer-token".into(), card_id: "TOKEN_PAYER".into(), card_name: "Payer Token".into(), colors: vec![CardColor::Blue], dp: Some(3000), level: None, traits: vec![], keywords: vec![] };
    let mut scap = make_test_card("SCAP", "SCAP");
    scap.keywords = vec![Keyword::Scapegoat];
    let mut r = DebugRunner::builder().add_card(scap).add_card(token.to_card_data()).start();
    let s = r.place_on_field(0, "SCAP", Some(0));
    let source = r.top_card(s);
    r.game.token_registry.insert(token);
    let a = EffectContext::new(&mut r.game, source, Some(s), 0).play_token(0, "payer-token").unwrap();
    r.game.drain_effect_queue();
    assert!(r.game.permanent_is_digimon_for_rules(a));
    let action = accept_and_check_picker(&mut r, s, a);
    let mut cloned = r.game.clone();
    resolve(&mut cloned, action);
    resolve(&mut r.game, action);
    endpoint(&cloned, &["SCAP"], &[]);
    endpoint(&r.game, &["SCAP"], &[]);
}
