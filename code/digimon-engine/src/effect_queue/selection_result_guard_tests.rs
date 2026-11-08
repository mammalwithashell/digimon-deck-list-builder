use crate::{
 action::space::PASS,
 card_source::CardHandle,
 debug_runner::{DebugRunner, make_test_card},
 enums::{GamePhase, EffectSourceKind},
 selection::{PendingSelection, SelectionKind, SelectionError},
 trigger_context::TriggerContext,
};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn exercise(payment_probe: bool, decline: bool) {
 for player in [0,1] { for nested in [false,true] { for marked in [false,true] {
    let mut r=DebugRunner::builder().add_card(make_test_card("SG-1","Source"))
        .deck(0,&["SG-1";20]).deck(1,&["SG-1";20]).memory(3).start();
    r.skip_mulligan();
    let game=&mut r.game;
    let calls=Arc::new(AtomicUsize::new(0));let accept_calls=calls.clone();let decline_calls=calls.clone();
    game.pending_selection=Some(PendingSelection{
        card_choice:None,kind:SelectionKind::EffectChoice,selecting_player:player,
        previous_phase:GamePhase::Main,valid_action_ids:vec![0],is_optional:true,
        prompt:"Guard result".into(),effect_choices:None,source_card:CardHandle(0),
        source_permanent:None,source_kind:EffectSourceKind::Digimon,zone_owner:None,
        callback:Box::new(move |game, action| {assert_eq!(action,0);assert!(!decline);
            accept_calls.fetch_add(1,Ordering::SeqCst);game.memory=4;}),
        on_decline:Some(Box::new(move |game| {assert!(decline);
            decline_calls.fetch_add(1,Ordering::SeqCst);game.memory=4;})),
    });
    game.current_phase=GamePhase::EffectChoice;
    let mut context=TriggerContext::default();context.projected_reference_reads_guarded=marked;
    if nested {let mut parent=TriggerContext::default();parent.simultaneous_play_members.push(context);context=parent;}
    game.current_trigger_context=Some(context);
    let before_depth=game.draining_deferred;
    let before_resolution_depth=game.selection_resolution_depth;
    let action=if decline {PASS} else {0};
    let result=catch_unwind(AssertUnwindSafe(|| if payment_probe {
        game.resolve_payment_probe_selection(player,action)
    } else {game.resolve_selection(player,action)}));
    if marked {
        assert!(result.is_err(),"marked result must reject");
        assert_eq!(calls.load(Ordering::SeqCst),0);
        assert_eq!(game.memory,3);
        assert!(game.pending_selection.is_some(),"reject before consuming prompt");
        assert_eq!(game.current_phase,GamePhase::EffectChoice);
        assert_eq!(game.draining_deferred,before_depth);
        assert_eq!(game.selection_resolution_depth,before_resolution_depth);
    } else {
        assert!(result.unwrap().is_ok());
        assert_eq!(calls.load(Ordering::SeqCst),1);
        assert_eq!(game.memory,4);
        assert!(game.pending_selection.is_none());
        assert_eq!(game.draining_deferred,before_depth);
        assert_eq!(game.selection_resolution_depth,before_resolution_depth);
    }
 } } }
}
#[test] fn raw_accept_checks_before_prompt_consumption(){exercise(false,false);}
#[test] fn raw_decline_checks_before_prompt_consumption(){exercise(false,true);}
#[test] fn payment_probe_accept_checks_before_prompt_consumption(){exercise(true,false);}
#[test] fn payment_probe_decline_checks_before_prompt_consumption(){exercise(true,true);}

#[test]
fn invalid_actions_still_use_selection_errors_without_invoking_callbacks(){
 let mut game=DebugRunner::builder().memory(3).start().game;
 game.pending_selection=Some(PendingSelection{
    card_choice:None,kind:SelectionKind::EffectChoice,selecting_player:0,
    previous_phase:GamePhase::Main,valid_action_ids:vec![0],is_optional:false,
    prompt:"Guard invalid".into(),effect_choices:None,source_card:CardHandle(0),
    source_permanent:None,source_kind:EffectSourceKind::Digimon,zone_owner:None,
    callback:Box::new(|_,_|panic!("invalid action invoked callback")),on_decline:None,
 });
 let mut context=TriggerContext::default();context.projected_reference_reads_guarded=true;
 game.current_trigger_context=Some(context);
 for probe in [false,true] {
   for (player,action,expected) in [(1,0,SelectionError::WrongPlayer),(0,1,SelectionError::InvalidAction),(0,PASS,SelectionError::InvalidAction)] {
    let result=if probe {game.resolve_payment_probe_selection(player,action)}else{game.resolve_selection(player,action)};
    assert_eq!(result,Err(expected)); assert!(game.pending_selection.is_some());
   }
 }
}



#[test]
fn physical_page_navigation_does_not_enter_result_callbacks() {
 use crate::{card_choice_page::CARD_CHOICE_NEXT, owned_card_choice::OwnedCardChoice};
 for probe in [false,true] { for nested in [false,true] {
    let mut game=DebugRunner::builder().add_card(make_test_card("SG-N","Navigation"))
        .security(0,&["SG-N";40]).memory(3).start().game;
    let owner=OwnedCardChoice::security(&mut game,0,(0..40).collect(),false).unwrap();
    let token=owner.token();let valid=owner.valid_action_ids(&game);assert!(valid.contains(&CARD_CHOICE_NEXT));
    game.pending_selection=Some(PendingSelection{
        card_choice:Some(owner),kind:SelectionKind::Security,selecting_player:0,
        previous_phase:GamePhase::Main,valid_action_ids:valid,is_optional:false,
        prompt:"Navigate".into(),effect_choices:None,source_card:CardHandle(0),
        source_permanent:None,source_kind:EffectSourceKind::Digimon,zone_owner:Some(0),
        callback:Box::new(|_,_|panic!("navigation invoked result callback")),
        on_decline:Some(Box::new(|_|panic!("navigation invoked decline callback"))),
    });
    game.current_phase=GamePhase::SelectSecurity;
    let mut context=TriggerContext::default();context.projected_reference_reads_guarded=true;
    if nested {let mut parent=TriggerContext::default();parent.simultaneous_play_members.push(context);context=parent;}
    game.current_trigger_context=Some(context);
    let result=if probe {game.resolve_payment_probe_selection(0,CARD_CHOICE_NEXT)}
               else {game.resolve_selection(0,CARD_CHOICE_NEXT)};
    assert!(result.is_ok());
    assert_ne!(game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().token(),token);
    assert_eq!(game.current_phase,GamePhase::SelectSecurity);
    assert_eq!(game.memory,3);assert_eq!(game.draining_deferred,0);assert_eq!(game.selection_resolution_depth,0);
 } }
}
