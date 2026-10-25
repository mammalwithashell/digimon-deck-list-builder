//! Public Lobomon security-page boundaries, including complete inspection rows.
use digimon_engine::{action::{build_action_mask,space::PASS},card_choice_page::CARD_CHOICE_NEXT,
    debug_runner::DebugRunner,owned_card_choice::CardChoiceOperation,EffectTiming,TriggerSource};
use rand::RngCore;
fn select_at(index:usize) {
    for actor in [0,1] {
        let mut security=vec!["BT13-012";index];security.push("BT17-020");
        let mut r=DebugRunner::builder().dsl_card("BT18-037").unwrap().dsl_card("BT17-020").unwrap().dsl_card("BT13-012").unwrap()
            .security(actor,&security).deck(0,&["BT13-012";15]).deck(1,&["BT13-012";15]).memory(3).start();
        r.skip_mulligan();r.game.turn_player_idx=actor as usize;
        let host=r.place_on_field(actor,"BT18-037",Some(0));
        let selected=r.game.player(actor).security[index].handle();
        let before_order:Vec<_>=r.game.player(actor).security.iter().map(|card|card.handle()).collect();
        let before_deck=r.game.player(actor).deck.len();let before_hand=r.game.player(actor).hand.len();
        let mut expected=r.game.clone();expected.players[actor as usize].security.remove(index);
        let recovered=expected.players[actor as usize].deck.pop().unwrap();expected.players[actor as usize].put_on_security_top(recovered);expected.shuffle_security(actor);
        let rng_before=r.game.rng.clone().next_u64();
        r.game.enqueue_triggered(EffectTiming::WhenDigivolving,TriggerSource::Permanent(host));r.game.drain_effect_queue();
        for page in 0..=index/28 {
            let pending=r.game.pending_selection.as_ref().expect("full security inspection is player-visible");
            assert!(pending.is_optional);let owner=pending.card_choice.as_ref().expect("typed security page");
            let own=owner.view(&r.game,actor);let other=owner.view(&r.game,1-actor);
            assert!(other.rows.iter().all(|row|row.card_id.is_none()&&row.card_registry_index.is_none()),"other seat cannot inspect security identities");
            let mask=build_action_mask(&r.game,actor);assert_eq!(mask[PASS as usize],1.0);
            for row in &own.rows {
                match row.operation {
                    CardChoiceOperation::InspectCard=>assert_eq!(mask[row.action_id as usize],0.0,"ineligible inspected cards cannot be selected"),
                    CardChoiceOperation::CommitCard=>assert_eq!(mask[row.action_id as usize],1.0),_=>{}
                }
            }
            if page<index/28 {
                assert_eq!(mask[CARD_CHOICE_NEXT as usize],1.0);r.game.resolve_selection(actor,CARD_CHOICE_NEXT).unwrap();
                assert_eq!(r.game.player(actor).security.iter().map(|card|card.handle()).collect::<Vec<_>>(),before_order);
                assert_eq!(r.game.player(actor).deck.len(),before_deck);assert_eq!(r.game.memory,3);
                assert_eq!(r.game.rng.clone().next_u64(),rng_before,"navigation cannot shuffle");
            }
        }
        let view=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(&r.game,actor);
        let row=view.rows.iter().find(|row|matches!(row.operation,CardChoiceOperation::CommitCard)
            &&row.position.as_deref()==Some(index.to_string().as_str())).expect("chosen physical security index is reachable");
        assert_eq!(row.card_id.as_deref(),Some("BT17-020"));let action=row.action_id;
        assert_eq!(build_action_mask(&r.game,1-actor)[action as usize],0.0);
        let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            game.resolve_selection(actor,action).unwrap();assert!(game.pending_selection.is_none());
            assert_eq!(game.player(actor).hand.len(),before_hand+1);assert!(game.player(actor).hand.iter().any(|card|card.handle()==selected));
            assert_eq!(game.player(actor).security.len(),index+1);assert_eq!(game.player(actor).deck.len(),before_deck-1);
            assert_eq!(game.player(actor).security.iter().map(|card|card.handle()).collect::<Vec<_>>(),
                expected.player(actor).security.iter().map(|card|card.handle()).collect::<Vec<_>>(),"chosen card removed, deck top recovered, one shuffle");
            assert_eq!(game.rng.clone().next_u64(),expected.rng.clone().next_u64(),"exactly one shuffle after actual selection");
        }
    }
}
#[test] fn lobomon_security_index_9_is_selectable(){select_at(9);}
#[test] fn lobomon_security_index_10_is_selectable(){select_at(10);}
#[test] fn lobomon_security_index_39_is_selectable(){select_at(39);}
