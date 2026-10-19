//! A singular paid prevention must retain its post-payment subject choice.
use digimon_engine::{
    action::{build_action_mask, space::{encode_attack, PASS, REPLACEMENT_ACCEPT}},
    debug_runner::{DebugRunner, make_test_card},
    permanent::PermanentHandle,
    selection::SelectionKind,
};

fn fixture() -> DebugRunner {
    let yaml = r#"
card: SINGULAR-PAYER
name: Singular paid protection
kind: tamer
color: [green]
cost: 3
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - other: true
        - replacement_cause: opponent_effect
    cost: { delete_self: true }
    outcome: prevent
"#;
    let mut r=DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
        .add_card(make_test_card("SINGULAR-ALLY", "Ally"))
        .deck(0,&["SINGULAR-ALLY";20]).deck(1,&["SINGULAR-ALLY";20])
        .memory(3).start();
    r.skip_mulligan();r.game.enter_main_phase();
    r.place_on_field(0,"SINGULAR-ALLY",None);
    r.place_on_field(0,"SINGULAR-ALLY",None);
    r.place_on_field(0,"SINGULAR-PAYER",None);
    r.game.set_effect_source_player_for_test(Some(1));
    r.game.delete_permanents_batch(vec![
        PermanentHandle{player:0,index:0}, PermanentHandle{player:0,index:1}],
        digimon_engine::replacement::ReplacementCause::OpponentEffect);
    r
}
fn answer(r:&mut DebugRunner, action:u16) {
    let player=r.game.pending_selection.as_ref().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game,player)[action as usize],1.0);
    r.game.resolve_selection(player,action).unwrap();
}
#[test]
fn singular_paid_prevention_chooses_one_after_payment_and_clones() {
    let mut r=fixture();
    answer(&mut r,REPLACEMENT_ACCEPT);
    assert_eq!(r.game.players[0].battle_area.len(),2,"payer must leave before subject choice");
    let view=r.pending_selection_view().expect("singular prevention requires a subject choice");
    assert_eq!(view.kind,SelectionKind::OwnField);
    assert!(!view.is_optional);
    let first=r.game.players[0].battle_area[0].top_card().handle();
    let second=r.game.players[0].battle_area[1].top_card().handle();
    assert_ne!(first,second,"duplicate printed IDs retain distinct physical cards");
    let mask=build_action_mask(&r.game,0);
    assert_eq!(mask[PASS as usize],0.0);
    let saved=r.game.clone();
    for (index,card) in [(0,first),(1,second)] {
        let mut digest=None;
        for _ in 0..2 {
            r.game=saved.clone();answer(&mut r,encode_attack(0,index));
            r.auto_resolve().unwrap();
            assert!(r.game.pending_selection.is_none());
            assert_eq!(r.game.players[0].battle_area.len(),1);
            assert_eq!(r.game.players[0].battle_area[0].top_card().handle(),card);
            let current=r.game.verification_digest();
            if let Some(prior)=digest {assert_eq!(prior,current);} else {digest=Some(current);}
        }
    }
}
