//! Printed “If you did” actual-play results across replacements and clones.
use digimon_engine::{action::{build_action_mask,space::PASS},debug_runner::{make_test_card,DebugRunner},
 enums::{CardColor,CardKind,Expiry,ModifierType},modifiers::PlayerModifierEntry,owned_card_choice::CardChoiceOperation,
 EffectTiming,TriggerSource};
#[test]
fn geogreymon_does_not_recover_when_selected_tamer_cannot_actually_be_played() {
    for actor in [0,1] {
        let mut tamer=make_test_card("AUDIT-TAMER","Yellow Tamer");tamer.card_kind=CardKind::Tamer;
        tamer.colors=vec![CardColor::Yellow];tamer.level=None;tamer.dp=None;
        let mut r=DebugRunner::builder().dsl_card("BT13-012").unwrap().add_card(tamer)
            .add_card(make_test_card("AUDIT-PAD","Pad")).security(actor,&["AUDIT-TAMER"])
            .deck(0,&["AUDIT-PAD";5]).deck(1,&["AUDIT-PAD";5]).memory(3).start();
        r.skip_mulligan();r.game.turn_player_idx=actor as usize;
        let host=r.place_on_field(actor,"BT13-012",None);
        r.game.enqueue_triggered(EffectTiming::WhenDigivolving,TriggerSource::Permanent(host));r.game.drain_effect_queue();
        r.game.modifiers.add_player_modifier(actor,PlayerModifierEntry::simple(
            ModifierType::CannotPlayTamerByEffect,0,Expiry::Permanent,None,actor));
        let faceup=r.game.player(actor).security[0].card_index;
        r.game.player_mut(actor).face_up_security.insert(faceup);
        let before_deck=r.game.player(actor).deck.len();let before_security=r.game.player(actor).security.len();
        let pending=r.game.pending_selection.as_ref().expect("printed search remains an explicit inspection");
        let page=pending.card_choice.as_ref().unwrap().view(&r.game,actor);let mask=build_action_mask(&r.game,actor);
        let action=page.rows.iter().find(|row|matches!(row.operation,CardChoiceOperation::CommitCard)&&mask[row.action_id as usize]>0.5)
            .map(|row|row.action_id).unwrap_or(PASS);
        r.game.resolve_selection(actor,action).unwrap();
        assert!(!r.game.player(actor).battle_area.iter().any(|p|p.top_card().card_id(&r.game.card_data)=="AUDIT-TAMER"));
        assert_eq!(r.game.player(actor).deck.len(),before_deck,"no actual Tamer play means printed If-you-did Recovery must not draw from deck");
        assert_eq!(r.game.player(actor).security.len(),before_security);
        assert!(r.game.pending_selection.is_none());
    }
}

struct OptionalCancelAuditTamer;
impl digimon_engine::effect::CardEffect for OptionalCancelAuditTamer {
    fn effects(&self,card:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::Effect::when_permanent_would_play(card)
            .name("Audit optional play cancellation").optional()
            .replacement_condition(|ctx,subject|matches!(subject,
                digimon_engine::replacement::ReplacementSubject::Card(handle,_)
                    if ctx.game.card_data_for_handle(*handle).is_some_and(|data|data.card_id=="AUDIT-TAMER")))
            .replacement_process(|rctx|rctx.cancel()).build()]
    }
}
#[test]
fn geogreymon_recovery_tracks_canceled_and_resumed_tamer_play_in_clones() {
    use std::sync::Arc;
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    for actor in [0,1] { for cancel in [true,false] {
        let mut tamer=make_test_card("AUDIT-TAMER","Yellow Tamer");tamer.card_kind=CardKind::Tamer;
        tamer.colors=vec![CardColor::Yellow];tamer.level=None;tamer.dp=None;
        let mut r=DebugRunner::builder().dsl_card("BT13-012").unwrap().add_card(tamer)
            .add_card(make_test_card("AUDIT-WATCHER","Watcher")).add_card(make_test_card("AUDIT-PAD","Pad"))
            .security(actor,&["AUDIT-TAMER","AUDIT-PAD"]).deck(0,&["AUDIT-PAD";5]).deck(1,&["AUDIT-PAD";5]).memory(3).start();
        r.skip_mulligan();r.game.turn_player_idx=actor as usize;
        r.register_effect("AUDIT-WATCHER",Arc::new(OptionalCancelAuditTamer));
        let host=r.place_on_field(actor,"BT13-012",None);r.place_on_field(actor,"AUDIT-WATCHER",None);
        let before_deck=r.game.player(actor).deck.len();
        r.game.enqueue_triggered(EffectTiming::WhenDigivolving,TriggerSource::Permanent(host));r.game.drain_effect_queue();
        let page=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(&r.game,actor);
        let action=page.rows.iter().find(|row|matches!(row.operation,CardChoiceOperation::CommitCard)).unwrap().action_id;
        assert_eq!(build_action_mask(&r.game,actor)[action as usize],1.0);r.game.resolve_selection(actor,action).unwrap();
        let pending=r.game.pending_selection.as_ref().expect("optional pre-play replacement pauses before actual Tamer entry");
        assert!(pending.is_optional);let action=if cancel {REPLACEMENT_ACCEPT}else{PASS};
        assert_eq!(build_action_mask(&r.game,actor)[action as usize],1.0);
        let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            game.resolve_selection(actor,action).unwrap();assert!(game.pending_selection.is_none());
            assert_eq!(game.player(actor).battle_area.iter().any(|p|p.top_card().card_id(&game.card_data)=="AUDIT-TAMER"),!cancel);
            assert_eq!(game.player(actor).deck.len(),before_deck-if cancel{0}else{1},"Recovery is correlated with actual resumed Tamer play, not selection or an attempted play");
            assert_eq!(game.player(actor).security.len(),2);assert_eq!(game.memory,3);
        }
    }}
}

const CORRELATED_PLAY: &str = r#"
card: AUDIT-PLAYER
name: Audit Player
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 4000
effects:
  - when: when_digivolving
    process:
      - search_own_security_stack:
          bind_as: chosen
          optional: true
          filter: { kind: tamer }
          prompt: Choose a Tamer
          on_select: []
      - play_security_card: { of: you, card: chosen }
      - if:
          condition: { effect_played_any_digimon: true }
          then:
            - gain_memory: 2
"#;
#[test]
fn canceled_middle_security_play_restores_origin_and_clone_branches_are_independent() {
    use std::sync::Arc;
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    for actor in [0,1] {
        let mut tamer=make_test_card("AUDIT-TAMER","Yellow Tamer");tamer.card_kind=CardKind::Tamer;
        tamer.colors=vec![CardColor::Yellow];tamer.level=None;tamer.dp=None;
        let mut r=DebugRunner::builder().from_dsl_yaml(CORRELATED_PLAY).unwrap().add_card(tamer)
            .add_card(make_test_card("AUDIT-WATCHER","Watcher"))
            .add_card(make_test_card("AUDIT-LEFT","Left")).add_card(make_test_card("AUDIT-RIGHT","Right"))
            .security(actor,&["AUDIT-LEFT","AUDIT-TAMER","AUDIT-RIGHT"]).memory(3).start();
        r.skip_mulligan();r.game.turn_player_idx=actor as usize;
        r.register_effect("AUDIT-WATCHER",Arc::new(OptionalCancelAuditTamer));
        let host=r.place_on_field(actor,"AUDIT-PLAYER",None);r.place_on_field(actor,"AUDIT-WATCHER",None);
        let original:Vec<_>=r.game.player(actor).security.iter().map(|c|c.handle()).collect();
        let marked_index=r.game.player(actor).security[1].card_index;
        r.game.player_mut(actor).face_up_security.insert(marked_index);
        r.game.enqueue_triggered(EffectTiming::WhenDigivolving,TriggerSource::Permanent(host));r.game.drain_effect_queue();
        let page=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(&r.game,actor);
        let action=page.rows.iter().find(|row|matches!(row.operation,CardChoiceOperation::CommitCard)).unwrap().action_id;
        assert_eq!(build_action_mask(&r.game,actor)[action as usize],1.0);
        let mut blocked = r.game.clone();
        blocked.modifiers.add_player_modifier(actor,PlayerModifierEntry::simple(ModifierType::CannotPlayTamerByEffect,0,Expiry::Permanent,None,actor));
        blocked.resolve_selection(actor,action).unwrap();
        assert_eq!(blocked.player(actor).security.iter().map(|c|c.handle()).collect::<Vec<_>>(),original);
        assert!(blocked.player(actor).face_up_security.contains(&marked_index));
        assert_eq!(blocked.memory,3);assert!(blocked.pending_selection.is_none());
        r.game.resolve_selection(actor,action).unwrap();
        assert!(r.game.pending_selection.is_some());let mut successful=r.game.clone();
        r.game.resolve_selection(actor,REPLACEMENT_ACCEPT).unwrap();
        assert_eq!(r.game.player(actor).security.iter().map(|c|c.handle()).collect::<Vec<_>>(),original,"cancel restores the selected middle physical slot");
        assert!(r.game.player(actor).face_up_security.contains(&marked_index));
        assert_eq!(r.game.memory,3);assert!(r.game.pending_selection.is_none());
        successful.resolve_selection(actor,PASS).unwrap();
        assert_eq!(successful.memory,5,"only successful clone receives actual-play benefit");
        assert_eq!(successful.player(actor).security.iter().map(|c|c.handle()).collect::<Vec<_>>(),vec![original[0],original[2]]);
        assert!(successful.player(actor).battle_area.iter().any(|p|p.top_card().handle()==original[1]));
        assert!(successful.pending_selection.is_none());
    }
}

#[test]
fn security_play_result_is_credited_once_before_entrant_on_play_choices() {
    const PARKING_TAMER: &str = r#"
card: AUDIT-TAMER
name: Yellow Tamer
kind: tamer
color: [yellow]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: witness
          filter: {}
          prompt: Choose the entry witness
"#;
    for actor in [0,1] {
        let mut r=DebugRunner::builder().from_dsl_yaml(CORRELATED_PLAY).unwrap().from_dsl_yaml(PARKING_TAMER).unwrap()
            .security(actor,&["AUDIT-TAMER"]).memory(3).start();
        r.skip_mulligan();r.game.turn_player_idx=actor as usize;
        let host=r.place_on_field(actor,"AUDIT-PLAYER",None);
        r.game.enqueue_triggered(EffectTiming::WhenDigivolving,TriggerSource::Permanent(host));r.game.drain_effect_queue();
        let page=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(&r.game,actor);
        let action=page.rows.iter().find(|row|matches!(row.operation,CardChoiceOperation::CommitCard)).unwrap().action_id;
        assert_eq!(build_action_mask(&r.game,actor)[action as usize],1.0);r.game.resolve_selection(actor,action).unwrap();
        let pending=r.game.pending_selection.as_ref().expect("entrant OnPlay decision remains public");
        assert!(pending.prompt.contains("entry witness"));
        assert_eq!(r.game.memory,5,"the current effect finishes before queued entrant OnPlay activates");
        let action=pending.valid_action_ids[0];assert_eq!(build_action_mask(&r.game,actor)[action as usize],1.0);
        let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            game.resolve_selection(actor,action).unwrap();assert!(game.pending_selection.is_none());
            assert_eq!(game.memory,5,"entry credits one outer actual-play result across an OnPlay pause");
            assert_eq!(game.player(actor).battle_area.len(),2);
        }
    }
}

#[test]
fn live_stack_play_from_security_uses_the_same_correlated_top_card_receipt() {
    use std::sync::Arc;
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    let search_start=CORRELATED_PLAY.find("      - search_own_security_stack:").unwrap();
    let search_end=CORRELATED_PLAY.find("      - if:").unwrap();
    let yaml=format!("{}      - play_from_security: {{}}\n{}",&CORRELATED_PLAY[..search_start],&CORRELATED_PLAY[search_end..]);
    for actor in [0,1] {
        let mut tamer=make_test_card("AUDIT-TAMER","Yellow Tamer");tamer.card_kind=CardKind::Tamer;
        tamer.colors=vec![CardColor::Yellow];tamer.level=None;tamer.dp=None;
        let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap().add_card(tamer)
            .add_card(make_test_card("AUDIT-WATCHER","Watcher")).add_card(make_test_card("AUDIT-PAD","Pad"))
            .security(actor,&["AUDIT-PAD","AUDIT-TAMER"]).memory(3).start();
        r.skip_mulligan();r.game.turn_player_idx=actor as usize;
        r.register_effect("AUDIT-WATCHER",Arc::new(OptionalCancelAuditTamer));
        let host=r.place_on_field(actor,"AUDIT-PLAYER",None);r.place_on_field(actor,"AUDIT-WATCHER",None);
        let original:Vec<_>=r.game.player(actor).security.iter().map(|c|c.handle()).collect();
        r.game.enqueue_triggered(EffectTiming::WhenDigivolving,TriggerSource::Permanent(host));r.game.drain_effect_queue();
        assert!(r.game.pending_selection.is_some());let mut success=r.game.clone();
        assert_eq!(build_action_mask(&r.game,actor)[REPLACEMENT_ACCEPT as usize],1.0);
        r.game.resolve_selection(actor,REPLACEMENT_ACCEPT).unwrap();
        assert_eq!(r.game.memory,3);assert_eq!(r.game.player(actor).security.iter().map(|c|c.handle()).collect::<Vec<_>>(),original);
        assert_eq!(build_action_mask(&success,actor)[PASS as usize],1.0);success.resolve_selection(actor,PASS).unwrap();
        assert_eq!(success.memory,5);assert_eq!(success.player(actor).security.iter().map(|c|c.handle()).collect::<Vec<_>>(),vec![original[0]]);
        assert!(success.player(actor).battle_area.iter().any(|p|p.top_card().handle()==original[1]));
        assert!(success.pending_selection.is_none());assert!(r.game.pending_selection.is_none());
    }
}

struct RawSecurityPlay;
impl digimon_engine::effect::CardEffect for RawSecurityPlay {
    fn effects(&self,card:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect> {
        vec![digimon_engine::effect::Effect::on_play(card).process(|ctx| {
            let target=ctx.game.player(ctx.player).security.iter().find(|c|c.card_id(&ctx.game.card_data)=="AUDIT-TAMER").unwrap().handle();
            ctx.play_from_security_card(ctx.player,target);
        }).build()]
    }
}
fn raw_security_cancel_origin(selected_index: usize) {
    use std::sync::Arc;
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    for actor in [0,1] {
        let mut tamer=make_test_card("AUDIT-TAMER","Yellow Tamer");tamer.card_kind=CardKind::Tamer;
        tamer.colors=vec![CardColor::Yellow];tamer.level=None;tamer.dp=None;
        let mut r=DebugRunner::builder().add_card(tamer).add_card(make_test_card("AUDIT-RAW","Raw Player"))
            .add_card(make_test_card("AUDIT-WATCHER","Watcher")).add_card(make_test_card("AUDIT-LEFT","Left"))
            .add_card(make_test_card("AUDIT-RIGHT","Right"))
            .security(actor,if selected_index==1 { &["AUDIT-LEFT","AUDIT-TAMER","AUDIT-RIGHT"] } else { &["AUDIT-LEFT","AUDIT-RIGHT","AUDIT-TAMER"] }).memory(3).start();
        r.skip_mulligan();r.game.turn_player_idx=actor as usize;
        r.register_effect("AUDIT-WATCHER",Arc::new(OptionalCancelAuditTamer));r.register_effect("AUDIT-RAW",Arc::new(RawSecurityPlay));
        let host=r.place_on_field(actor,"AUDIT-RAW",None);r.place_on_field(actor,"AUDIT-WATCHER",None);
        let original:Vec<_>=r.game.player(actor).security.iter().map(|c|c.handle()).collect();
        let marker=r.game.player(actor).security[selected_index].card_index;r.game.player_mut(actor).face_up_security.insert(marker);
        r.fire_on_play(actor,host.index as usize);assert!(r.game.pending_selection.is_some());
        assert_eq!(build_action_mask(&r.game,actor)[REPLACEMENT_ACCEPT as usize],1.0);
        let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            game.resolve_selection(actor,REPLACEMENT_ACCEPT).unwrap();assert!(game.pending_selection.is_none());
            assert_eq!(game.player(actor).security.iter().map(|c|c.handle()).collect::<Vec<_>>(),original,"unrecorded SecurityTop cancellation preserves its established top origin");
            assert!(game.player(actor).face_up_security.contains(&marker));assert_eq!(game.player(actor).battle_area.len(),2);assert_eq!(game.memory,3);
        }
    }
}

#[test]
fn raw_unrecorded_security_play_cancel_restores_top_and_face_up_in_clones() { raw_security_cancel_origin(2); }
#[test]
fn raw_unrecorded_security_play_cancel_restores_selected_middle_and_face_up_in_clones() { raw_security_cancel_origin(1); }
