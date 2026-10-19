// Declining the enclosing payment after its nested evolution finishes revokes only enclosing credit.
include!("bt4_011_nested_abort_scope.rs");
fn outer_tail_decline(native:bool) {
    use digimon_engine::action::space::{ATTACK_START,PLAY_HAND_START,PASS};
    let observer=OBSERVER.replace("      - select_own_permanent:\n          filter: {card_number_is: TEST-Q1157-OBSERVER}\n          bind_as: tail_marker_again", "      - select_hand:\n          of: you\n          filter: {card_number_is: ST1-03}\n          optional: true\n          cost: true\n          bind_as: tail_marker_again");
    let mut r=setup_observer(false,&observer);
    let outer=r.place_on_field(0,"ST1-12",Some(0));
    r.place_on_field(0,"ST1-03",Some(0));
    r.place_on_field(0,"TEST-Q1157-OBSERVER",Some(0));
    r.place_on_field(0,"TEST-INNER-COST",Some(0));
    let inner=r.add_to_hand(0,"TEST-Q1157-INNER");r.game.tick_declarative_effects();
    if native {let a=encode_digivolve(0,0);assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.game.decode_action(a,0);}
    else {r.game.effect_initiated_digivolve(0,0,outer,digimon_engine::enums::CostDelta::Reduce(0),false,digimon_engine::enums::PlaySource::ByEffect);}
    // The first selector is mandatory, so the enclosing reducer retains its consent gate.
    accept(&mut r);choose(&mut r,ATTACK_START+1);choose(&mut r,PLAY_HAND_START+inner as u16);
    choose(&mut r,PASS); // Inner reducer declined, ordinary evolution still completes.
    assert_eq!(r.game.players[0].battle_area[1].top_card().card_id(&r.game.card_data),"TEST-Q1157-INNER");
    choose(&mut r,ATTACK_START+2); // A subsequent enclosing mandatory choice remains real.
    let parked=r.game.clone();let mut digest=None;
    for _ in 0..2 {
        r.game=parked.clone();choose(&mut r,PASS);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"BT4-011");
        assert_eq!(r.memory(),1,"5 minus inner ordinary cost2 minus outer unreduced cost2; aborted tail earns no memory");
        assert_eq!(r.game.players[0].hand.len(),2);
        let d=r.game.verification_digest();if let Some(previous)=digest {assert_eq!(d,previous);}digest=Some(d);
    }
}
#[test]fn native_enclosing_payment_decline_after_inner_scope_removed(){outer_tail_decline(true);}
#[test]fn effect_enclosing_payment_decline_after_inner_scope_removed(){outer_tail_decline(false);}
