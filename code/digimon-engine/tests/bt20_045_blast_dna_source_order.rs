//! Source-derived scratch tests; unexecuted. Rules8-2-2-2/3 and16-30-4.
use digimon_engine::action::{build_action_mask,space::{DNA_DIGIVOLVE_START,PLAY_HAND_START}};
use digimon_engine::debug_runner::{make_test_card,DebugRunner};
use digimon_engine::game::Game;
fn pick(g:&mut Game,a:u16) {
    let actor=g.pending_selection.as_ref().expect("real pending decision").selecting_player;
    assert!(build_action_mask(g,actor)[a as usize]>0.5,"chosen action must be legal");
    g.resolve_selection(actor,a).unwrap();
}
fn ordered_blast(field:&str,partner:&str,field_is_upper:bool) {
    let mut r=DebugRunner::builder().dsl_card("BT20-045").unwrap()
        .dsl_card(field).unwrap().dsl_card(partner).unwrap()
        .add_card(make_test_card("LOWER","Lower inert source"))
        .add_card(make_test_card("DRAW","Draw"))
        .add_card(make_test_card("ATTACKER","Attacker"))
        .hand(1,&["BT20-045",partner]).deck(1,&["DRAW","DRAW","DRAW"])
        .security(1,&["DRAW"]).start();
    let a=r.place_on_field(0,"ATTACKER",Some(0));
    let base=r.place_stack(1,&["LOWER",field]);
    r.game.players[1].battle_area[0].is_suspended=true;
    r.game.tick_declarative_effects();
    let mut field_handles=r.game.players[1].battle_area[0].card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>();
    let result=r.game.players[1].hand[0].handle();let material=r.game.players[1].hand[1].handle();
    let mut expected=Vec::new();
    if field_is_upper {expected.push(material);expected.append(&mut field_handles);}else{expected.append(&mut field_handles);expected.push(material);}
    expected.push(result);
    r.attack_digimon(a,base,false);
    pick(&mut r.game,DNA_DIGIVOLVE_START);
    pick(&mut r.game,0);
    for mut game in [r.game.clone(),r.game] {
        pick(&mut game,PLAY_HAND_START+1);
        let carrier=game.players[1].battle_area.iter().find(|p|p.top_card().card_id(&game.card_data)=="BT20-045").expect("actual Examon result");
        assert_eq!(carrier.card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>(),expected,
            "printed left material is upper; preserve all existing lower source identities and their order");
        let ids=carrier.card_sources.iter().map(|c|c.card_id(&game.card_data)).collect::<Vec<_>>();
        let expected_ids=if field_is_upper{vec![partner,"LOWER",field,"BT20-045"]}else{vec!["LOWER",field,partner,"BT20-045"]};
        assert_eq!(ids,expected_ids);
        assert_eq!(game.players[1].deck.len(),2,"one DNA bonus draw only");
    }
}
#[test]fn actual_breakdramon_field_goes_above_hand_slayerdramon(){ordered_blast("EX13-044","EX3-024",true);}
#[test]fn actual_slayerdramon_field_stays_below_hand_breakdramon(){ordered_blast("EX3-024","EX13-044",false);}
#[test]fn result_scoped_groundramon_field_goes_above_hand_slayerdramon(){ordered_blast("EX13-041","EX3-024",true);}
#[test]fn result_scoped_wingdramon_field_stays_below_hand_breakdramon(){ordered_blast("EX13-021","EX13-044",false);}
