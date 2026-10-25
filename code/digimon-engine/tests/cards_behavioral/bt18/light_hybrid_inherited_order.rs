//! Official BT18-063 Q2994 / BT18-076 Q3021: the played Tamer's [On Play]
//! and original Digimon's [On Deletion] trigger simultaneously, with order chosen
//! by their controller. Real inherited YAML cards drive the actual leave flow.
use std::sync::Arc;
use digimon_engine::{action::{build_action_mask, space::{PASS, REPLACEMENT_ACCEPT}}, card_source::CardHandle,
    debug_runner::{make_test_card,DebugRunner}, effect::{CardEffect,Effect}, enums::{CardKind,EffectTiming},
    events::GameEvent, game::Game, owned_card_choice::CardChoiceOperation, replacement::ReplacementCause,
    selection::SelectionKind};
struct OrderedTamer;
impl CardEffect for OrderedTamer {
    fn effects(&self, card:CardHandle)->Vec<Effect> {
        vec![Effect::on_play(card).name("Ordered Tamer On Play").process(|ctx|ctx.gain_memory(1)).build()]
    }
}
struct OrderedDigimon;
impl CardEffect for OrderedDigimon {
    fn effects(&self, card:CardHandle)->Vec<Effect> {
        vec![Effect::on_deletion(card).name("Ordered Digimon On Deletion").process(|ctx|ctx.gain_memory(2)).build()]
    }
}
fn choose(game:&mut Game,actor:u8,action:u16) {
    assert_eq!(build_action_mask(game,actor)[action as usize],1.0,"submitted action is currently legal");
    assert_eq!(build_action_mask(game,1-actor)[action as usize],0.0,"other seat cannot choose this activation");
    game.resolve_selection(actor,action).unwrap();
}
pub fn prove_simultaneous_order(card_id:&str) {
    for actor in [0,1] {
        let mut tamer=make_test_card("ORDER-TAMER","Ordered Tamer");tamer.card_kind=CardKind::Tamer;tamer.level=None;tamer.dp=None;
        tamer.inherited_text="[Your Turn] This Digimon gets +1000 DP.".into();tamer.effect_text="[On Play] Gain 1 memory.".into();
        let mut top=make_test_card("ORDER-TOP","Ordered Digimon");top.effect_text="[On Deletion] Gain 2 memory.".into();
        let mut r=DebugRunner::builder().dsl_card(card_id).unwrap().add_card(tamer).add_card(top)
            .add_card(make_test_card("ORDER-PAD","Pad")).deck(0,&["ORDER-PAD";20]).deck(1,&["ORDER-PAD";20])
            .memory(3).start();
        r.skip_mulligan();r.game.turn_player_idx=actor as usize;
        r.register_effect("ORDER-TAMER",Arc::new(OrderedTamer));r.register_effect("ORDER-TOP",Arc::new(OrderedDigimon));
        let host=r.place_stack(actor,&["ORDER-TAMER",card_id,"ORDER-TOP"]);
        r.game.delete_permanent_with_cause(host,ReplacementCause::OpponentEffect);
        choose(&mut r.game,actor,REPLACEMENT_ACCEPT);
        let pending=r.game.pending_selection.as_ref().expect("printed inherited effect offers source Tamer choice");
        let source=pending.card_choice.as_ref().unwrap().view(&r.game,actor).rows.into_iter()
            .find(|row|matches!(row.operation,CardChoiceOperation::CommitCard)&&row.card_id.as_deref()==Some("ORDER-TAMER"))
            .expect("choose the qualifying physical Tamer");
        choose(&mut r.game,actor,source.action_id);
        let view=r.game.pending_selection.as_ref().expect("joint On Play / On Deletion activation order");
        assert_eq!(view.kind,SelectionKind::TriggerOrder,"{card_id}: Q2994/Q3021 require one simultaneous trigger window");
        assert_eq!(view.selecting_player,actor);
        let choices=view.effect_choices.as_ref().expect("player-visible activation identities");
        assert_eq!(choices.len(),2);
        for timing in [EffectTiming::OnPlay,EffectTiming::OnDeletion] {
            let choice=choices.iter().find(|choice|choice.timing==Some(timing)).expect("both timing families are pending together");
            assert_eq!(build_action_mask(&r.game,actor)[choice.action_id as usize],1.0);
        }
        assert_eq!(build_action_mask(&r.game,actor)[PASS as usize],0.0);
        assert_eq!(r.game.memory,3,"neither trigger may resolve before the chosen order");
        assert!(r.game.player(actor).battle_area.iter().any(|p|p.top_card().card_id(&r.game.card_data)=="ORDER-TAMER"));
        assert!(!r.game.player(actor).battle_area.iter().any(|p|p.top_card().card_id(&r.game.card_data)=="ORDER-TOP"),"free Tamer play does not cancel original deletion");
        let event_start=r.game.events.len();
        for first in [EffectTiming::OnPlay,EffectTiming::OnDeletion] {
            for _ in 0..2 {
                let mut game=r.game.clone();
                let action=game.pending_selection.as_ref().unwrap().effect_choices.as_ref().unwrap().iter()
                    .find(|choice|choice.timing==Some(first)).unwrap().action_id;
                choose(&mut game,actor,action);
                if let Some(pending)=game.pending_selection.as_ref() {
                    assert_eq!(pending.kind,SelectionKind::TriggerOrder);
                    let remaining=pending.effect_choices.as_ref().unwrap();assert_eq!(remaining.len(),1);
                    assert_ne!(remaining[0].timing,Some(first));let action=remaining[0].action_id;
                    choose(&mut game,actor,action);
                }
                assert!(game.pending_selection.is_none());
                let actual:Vec<_>=game.events[event_start..].iter().filter_map(|event|match event {
                    GameEvent::MemoryChange {source_card_id:Some(id),delta,..} => Some((id.as_str(),*delta)),_=>None
                }).collect();
                let expected=if first==EffectTiming::OnPlay {vec![("ORDER-TAMER",1),("ORDER-TOP",2)]}
                    else {vec![("ORDER-TOP",2),("ORDER-TAMER",1)]};
                assert_eq!(actual,expected,"{card_id}: only the chosen order may determine activation trace");
                assert_eq!(game.memory,6);
            }
        }
    }
}
