//! Public source-page boundaries for the exact LightHybrid inherited cards.
use digimon_engine::{action::{build_action_mask,space::{PASS,REPLACEMENT_ACCEPT}},
    card_choice_page::CARD_CHOICE_NEXT,debug_runner::{make_test_card,DebugRunner},enums::CardKind,
    owned_card_choice::CardChoiceOperation,replacement::ReplacementCause};
use rand::RngCore;
fn run(card_id:&str,tamer_index:usize) {
    for actor in [0,1] {
        let mut tamer=make_test_card("TEST-INHERITED-TAMER","Inherited Tamer");tamer.card_kind=CardKind::Tamer;
        tamer.inherited_text="[Your Turn] This Digimon gets +1000 DP.".into();
        let mut filler=tamer.clone();filler.card_id="TEST-TAMER-FILLER".into();filler.card_name="Other inherited Tamer".into();
        let mut r=DebugRunner::builder().dsl_card(card_id).unwrap().add_card(tamer).add_card(filler)
            .add_card(make_test_card("TEST-SOURCE-FILLER","Source filler")).add_card(make_test_card("TEST-CARRIER","Carrier"))
            .deck(0,&["TEST-SOURCE-FILLER";20]).deck(1,&["TEST-SOURCE-FILLER";20]).memory(3).start();
        r.skip_mulligan();r.game.turn_player_idx=actor as usize;
        let mut stack=vec![card_id];stack.extend(std::iter::repeat("TEST-TAMER-FILLER").take(tamer_index-1));
        stack.extend(["TEST-INHERITED-TAMER","TEST-CARRIER"]);
        let host=r.place_stack(actor,&stack);
        let selected=r.game.player(actor).battle_area[0].card_sources[tamer_index].handle();
        let source_order:Vec<_>=r.game.player(actor).battle_area[0].card_sources.iter().map(|card|card.handle()).collect();
        r.game.delete_permanent_with_cause(host,ReplacementCause::OpponentEffect);
        assert_eq!(build_action_mask(&r.game,actor)[REPLACEMENT_ACCEPT as usize],1.0);
        r.game.resolve_selection(actor,REPLACEMENT_ACCEPT).unwrap();
        let rng_before=r.game.rng.clone().next_u64();
        for _ in 0..(tamer_index-1)/28 {
            assert_eq!(build_action_mask(&r.game,actor)[CARD_CHOICE_NEXT as usize],1.0);
            r.game.resolve_selection(actor,CARD_CHOICE_NEXT).unwrap();
            assert_eq!(r.game.player(actor).battle_area[0].card_sources.iter().map(|card|card.handle()).collect::<Vec<_>>(),source_order);
            assert_eq!(r.game.memory,3);assert_eq!(r.game.rng.clone().next_u64(),rng_before,"navigation consumes no RNG");
        }
        let view=r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(&r.game,actor);
        let row=view.rows.iter().find(|row|matches!(row.operation,CardChoiceOperation::CommitCard)
            &&row.position.as_deref()==Some(tamer_index.to_string().as_str())).expect("requested physical depth is reachable on its page");
        assert_eq!(row.card_id.as_deref(),Some("TEST-INHERITED-TAMER"));assert_eq!(row.owner,Some(actor));
        let action=row.action_id;
        assert_eq!(build_action_mask(&r.game,actor)[action as usize],1.0);
        assert_eq!(build_action_mask(&r.game,1-actor)[action as usize],0.0);
        assert_eq!(build_action_mask(&r.game,actor)[PASS as usize],1.0,"printed optional processing remains a surfaced choice");
        let mut clone=r.game.clone();
        for game in [&mut r.game,&mut clone] {
            game.resolve_selection(actor,action).unwrap();
            assert!(game.pending_selection.is_none());
            let played=game.player(actor).battle_area.iter().find(|p|p.top_card().handle()==selected).expect("chosen Tamer is actually played without cost");
            assert_eq!(played.top_card().card_id(&game.card_data),"TEST-INHERITED-TAMER");
            assert!(!game.player(actor).battle_area.iter().any(|p|p.top_card().card_id(&game.card_data)=="TEST-CARRIER"),"original leave still completes");
            assert_eq!(game.memory,3);
        }
    }
}
#[test] fn beetlemon_tamer_at_source_index_11_is_selectable(){run("BT18-063",11);}
#[test] fn loweemon_tamer_at_source_index_11_is_selectable(){run("BT18-076",11);}
#[test] fn beetlemon_tamer_at_source_index_12_is_selectable(){run("BT18-063",12);}
#[test] fn loweemon_tamer_at_source_index_12_is_selectable(){run("BT18-076",12);}
#[test] fn beetlemon_tamer_at_source_index_39_and_300_is_selectable(){run("BT18-063",39);run("BT18-063",300);}
#[test] fn loweemon_tamer_at_source_index_39_and_300_is_selectable(){run("BT18-076",39);run("BT18-076",300);}
