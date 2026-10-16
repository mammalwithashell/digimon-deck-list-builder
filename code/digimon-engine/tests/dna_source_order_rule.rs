//! Official rules8-2-2-2/3: left requirement above right; preserve internal order.
//! Scratch test proposal only; not compiled or run.
use digimon_engine::action::{build_action_mask,DNA_DIGIVOLVE_START};
use digimon_engine::debug_runner::{make_test_card,make_test_card_with_level,make_test_dna_card,DebugRunner};
use digimon_engine::enums::GamePhase;

fn runner(reverse_field:bool)->DebugRunner {
    let mut r=DebugRunner::builder()
        .add_card(make_test_card_with_level("LEFT","Left",5))
        .add_card(make_test_card_with_level("RIGHT","Right",6))
        .add_card(make_test_card("LEFT-LOW","Left lower"))
        .add_card(make_test_card("RIGHT-LOW","Right lower"))
        .add_card(make_test_card("DRAW","Draw"))
        .add_card(make_test_dna_card("RESULT","Result",5,6,0))
        .hand(0,&["RESULT"]).deck(0,&["DRAW","DRAW","DRAW"])
        .memory(5).start();
    if reverse_field {
        r.place_stack(0,&["RIGHT-LOW","RIGHT"]);
        r.place_stack(0,&["LEFT-LOW","LEFT"]);
    } else {
        r.place_stack(0,&["LEFT-LOW","LEFT"]);
        r.place_stack(0,&["RIGHT-LOW","RIGHT"]);
    }
    r.game.current_phase=GamePhase::Main;
    r
}
fn native_order(reverse_field:bool,choose_right_first:bool) {
    let mut r=runner(reverse_field);
    assert!(build_action_mask(&r.game,0)[DNA_DIGIVOLVE_START as usize]>0.5);
    assert!(r.game.initiate_dna_digivolve(0,0));
    let left=if reverse_field{1}else{0};
    let right=1-left;
    let expected_handles:Vec<_>=r.game.players[0].battle_area[right].card_sources.iter()
        .chain(r.game.players[0].battle_area[left].card_sources.iter())
        .chain(r.game.players[0].hand.iter().take(1)).map(|c|c.handle()).collect();
    let (first,second)=if choose_right_first{(right,left)}else{(left,right)};
    assert!(build_action_mask(&r.game,0)[first]>0.5);
    r.game.resolve_selection(0,first as u16).unwrap();
    for mut g in [r.game.clone(),r.game] {
        assert!(build_action_mask(&g,0)[second]>0.5);
        g.resolve_selection(0,second as u16).unwrap();
        assert_eq!(g.players[0].battle_area.len(),1);
        let ids:Vec<_>=g.players[0].battle_area[0].card_sources.iter()
            .map(|c|c.card_id(&g.card_data).to_owned()).collect();
        assert_eq!(ids,["RIGHT-LOW","RIGHT","LEFT-LOW","LEFT","RESULT"],
            "bottom-to-top must put printed left requirement above right without reversing either material");
        assert_eq!(g.players[0].battle_area[0].card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>(),expected_handles,
            "moving whole material blocks preserves exact physical cards, including on parked clones");
    }
}
#[test]fn native_left_then_right_preserves_printed_stack_order(){native_order(false,false);}
#[test]fn native_right_then_left_preserves_printed_stack_order(){native_order(false,true);}
#[test]fn compacted_left_carrier_preserves_printed_stack_order(){native_order(true,false);}
#[test]fn reversed_field_and_pick_order_preserves_printed_stack_order(){native_order(true,true);}

fn effect_order(choose_right_first:bool) {
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::permanent::PermanentHandle;
    let r=runner(false);
    for mut g in [r.game.clone(),r.game] {
        let card=g.players[0].hand[0].handle();
        let a=PermanentHandle{player:0,index:if choose_right_first{1}else{0}};
        let b=PermanentHandle{player:0,index:1-a.index};
        let result=EffectContext::new(&mut g,card,None,0)
            .effect_initiated_dna_digivolve(a,b,card,0,false)
            .expect("the pair satisfies the printed recipe in either argument order");
        let ids:Vec<_>=g.players[0].battle_area[result.index as usize].card_sources.iter()
            .map(|c|c.card_id(&g.card_data).to_owned()).collect();
        assert_eq!(ids,["RIGHT-LOW","RIGHT","LEFT-LOW","LEFT","RESULT"]);
    }
}
#[test]fn effect_left_then_right_uses_printed_stack_order(){effect_order(false);}
#[test]fn effect_right_then_left_uses_printed_stack_order(){effect_order(true);}

#[test]
fn actual_examon_material_identities_keep_green_above_blue() {
    let mut r=DebugRunner::builder().dsl_card("BT23-047").unwrap()
        .dsl_card("BT20-042").unwrap().dsl_card("BT20-025").unwrap()
        .add_card(make_test_card("DRAW","Draw"))
        .hand(0,&["BT23-047"]).deck(0,&["DRAW","DRAW","DRAW"])
        .memory(5).start();
    r.place_on_field(0,"BT20-025",None);
    r.place_on_field(0,"BT20-042",None);
    r.game.current_phase=GamePhase::Main;
    r.game.tick_declarative_effects();
    assert!(build_action_mask(&r.game,0)[DNA_DIGIVOLVE_START as usize]>0.5);
    assert!(r.game.initiate_dna_digivolve(0,0));
    assert!(build_action_mask(&r.game,0)[1]>0.5);
    r.game.resolve_selection(0,1).unwrap();
    assert!(build_action_mask(&r.game,0)[0]>0.5);
    r.game.resolve_selection(0,0).unwrap();
    let ids:Vec<_>=r.game.players[0].battle_area[0].card_sources.iter()
        .map(|c|c.card_id(&r.game.card_data).to_owned()).collect();
    assert_eq!(ids,["BT20-025","BT20-042","BT23-047"],
        "green left requirement is nearest Examon, even with result-scoped Lv6 treatment");
}
