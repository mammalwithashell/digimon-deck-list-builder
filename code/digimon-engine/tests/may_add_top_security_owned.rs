use digimon_engine::card_choice_page::CARD_CHOICE_START;
use digimon_engine::{debug_runner::{DebugRunner,make_test_card},effect_context::EffectContext,dsl_cards::{bindings::Bindings,step::run_steps},action::{build_action_mask,space::PASS},owned_card_choice::{CardChoiceOperation,CardChoiceZone},enums::EffectSourceKind};
use digimon_dsl::compiled::{CompiledStep,CompiledPlayerRef};
fn fixture(actor:u8,owner:u8,length:usize,override_actor:Option<u8>)->DebugRunner {
    let cards=vec!["SAME";length];
    let mut r=DebugRunner::builder().add_card(make_test_card("EFFECT","Effect")).add_card(make_test_card("SAME","Same"))
        .hand(0,&["EFFECT","SAME"]).hand(1,&["EFFECT","SAME"]).security(owner,&cards).memory(0).start();
    r.skip_mulligan();r.game.enter_main_phase();r.game.turn_player_idx=actor as usize;
    let source=r.game.players[actor as usize].hand[0].handle();
    let of=if owner==actor {CompiledPlayerRef::You}else{CompiledPlayerRef::Opponent};
    run_steps(&[CompiledStep::MayAddTopSecurityToHand{of},CompiledStep::GainMemory(3)],
        &mut EffectContext::new_with_source_kind_and_override(&mut r.game,source,None,EffectSourceKind::Digimon,actor,override_actor),&mut Bindings::new());
    r
}
fn memory(_actor:u8)->i16 {3}
#[test]
fn top_security_owned_choice_reaches_indices_9_10_39_with_private_identity_and_clone_tail() {
    for actor in [0,1] {for owner in [actor,1-actor] {for override_actor in [None,Some(1-actor)] {for index in [9,10,39] {
        let r=fixture(actor,owner,index+1,override_actor);
        let selecting=override_actor.unwrap_or(actor);
        let top=r.game.players[owner as usize].security.last().unwrap().handle();
        let pending=r.game.pending_selection.as_ref().unwrap();
        assert_eq!(pending.selecting_player,selecting);assert_eq!(pending.zone_owner,Some(owner));
        assert!(pending.is_optional);
        let owned=pending.card_choice.as_ref().expect("physical top security owner");
        for observer in [0,1] {
            let rows=owned.view(&r.game,observer).rows;
            let commits:Vec<_>=rows.iter().filter(|row|matches!(row.operation,CardChoiceOperation::CommitCard)).collect();
            assert_eq!(commits.len(),1);let row=commits[0];
            assert!(matches!(row.zone,Some(CardChoiceZone::Security)));assert_eq!(row.owner,Some(owner));
            assert_eq!(row.position.as_deref(),Some(index.to_string().as_str()));
            assert!(!row.identity_visible);assert!(row.card_id.is_none());assert!(row.card_registry_index.is_none());
        }
        let mask=build_action_mask(&r.game,selecting);
        for (action,legal) in mask.iter().enumerate(){assert_eq!(*legal,if [CARD_CHOICE_START,PASS].contains(&(action as u16)){1.0}else{0.0});}
        assert!(build_action_mask(&r.game,1-selecting).iter().all(|v|*v==0.0));
        for mut game in [r.game.clone(),r.game.clone()] {
            game.resolve_selection(selecting,CARD_CHOICE_START).unwrap();
            assert_eq!(game.players[owner as usize].security.len(),index);
            assert_eq!(game.players[owner as usize].hand.last().unwrap().handle(),top);
            assert_eq!(game.memory,memory(actor));assert!(game.pending_selection.is_none());
        }
    }}}}
}
#[test]
fn stale_top_snapshot_rejects_same_number_replacement_and_decline_keeps_tail() {
    for actor in [0,1] {let mut r=fixture(actor,actor,40,None);
        let old_action=r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        let original=r.game.players[actor as usize].security.pop().unwrap();
        r.game.players[actor as usize].put_in_trash(original);
        let replacement=r.game.players[actor as usize].hand.remove(1);
        let replacement_handle=replacement.handle();r.game.players[actor as usize].put_on_security_top(replacement);
        for mut game in [r.game.clone(),r.game.clone()] {
            let before:Vec<_>=game.players[actor as usize].security.iter().map(|c|c.handle()).collect();
            assert_eq!(build_action_mask(&game,actor)[old_action as usize],0.0,"stale prompt cannot consume replacement top");
            assert!(game.resolve_selection(actor,old_action).is_err());assert_eq!(game.memory,0);
            game.resolve_selection(actor,PASS).unwrap();
            assert_eq!(game.players[actor as usize].security.iter().map(|c|c.handle()).collect::<Vec<_>>(),before);
            assert_eq!(game.players[actor as usize].security.last().unwrap().handle(),replacement_handle);
            assert_eq!(game.memory,memory(actor));
        }
    }
}
#[test]
fn shuffled_top_snapshot_only_offers_decline_without_inspection_and_preserves_following_effect() {
    for actor in [0,1] {let mut r=fixture(actor,actor,11,None);
        let old_action=r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        r.game.shuffle_security(actor);
        let mask=build_action_mask(&r.game,actor);
        assert_eq!(mask[old_action as usize],0.0);assert_eq!(mask[PASS as usize],1.0);
        let before:Vec<_>=r.game.players[actor as usize].security.iter().map(|c|c.handle()).collect();
        for mut game in [r.game.clone(),r.game.clone()] {
            assert!(game.resolve_selection(actor,old_action).is_err());game.resolve_selection(actor,PASS).unwrap();
            assert_eq!(game.players[actor as usize].security.iter().map(|c|c.handle()).collect::<Vec<_>>(),before);
            assert_eq!(game.memory,memory(actor));
        }
    }
}

#[test]
fn fresh_decline_and_empty_security_preserve_tail_and_faceup_permissions() {
    for actor in [0,1] {
        let mut r=fixture(actor,actor,11,None);
        let top=r.game.players[actor as usize].security.last().unwrap().handle();
        r.game.players[actor as usize].face_up_security.insert(top.0);
        let pending=r.game.pending_selection.as_ref().unwrap();
        for observer in [0,1] {
            let rows=pending.card_choice.as_ref().unwrap().view(&r.game,observer).rows;
            let row=rows.iter().find(|row|matches!(row.operation,CardChoiceOperation::CommitCard)).unwrap();
            assert!(row.identity_visible);assert_eq!(row.card_id.as_deref(),Some("SAME"));
        }
        for mut game in [r.game.clone(),r.game.clone()] {
            game.resolve_selection(actor,PASS).unwrap();
            assert_eq!(game.players[actor as usize].security.last().unwrap().handle(),top);
            assert_eq!(game.players[actor as usize].hand.len(),2);assert_eq!(game.memory,memory(actor));
        }
        let empty=fixture(actor,actor,0,None);
        assert!(empty.game.pending_selection.is_none());assert_eq!(empty.game.memory,memory(actor));
    }
}
#[test]
fn legacy_security_post_add_top_to_hand_frame_remains_compatible() {
    for actor in [0,1] {for owner in [actor,1-actor] {
        let mut r=fixture(actor,owner,11,None);
        let top=r.game.players[owner as usize].security.last().unwrap().handle();
        let legacy=if actor==owner {digimon_engine::action::space::SEL_MY_SECURITY_START}else{digimon_engine::action::space::SEL_OPP_SECURITY_START}+9;
        let pending=r.game.pending_selection.as_mut().unwrap();
        pending.card_choice=None;pending.zone_owner=None;pending.valid_action_ids=vec![legacy];
        for mut game in [r.game.clone(),r.game.clone()] {
            game.resolve_selection(actor,legacy).unwrap();
            assert_eq!(game.players[owner as usize].hand.last().unwrap().handle(),top);
            assert_eq!(game.players[owner as usize].security.len(),10);assert_eq!(game.memory,memory(actor));
        }
    }}
}

// Retain the complete existing canonical consumers beside the primitive probe.
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "dsl/security_stack_steps.rs"]
mod security_stack_steps;
#[path = "cards_behavioral/bt24/bt24_031.rs"]
mod bt24_031;
#[path = "cards_behavioral/bt25/bt25_030.rs"]
mod bt25_030;
#[path = "cards_behavioral/bt26/bt26_025.rs"]
mod bt26_025;

#[path = "cards_behavioral/bt26/support.rs"]
mod support;
