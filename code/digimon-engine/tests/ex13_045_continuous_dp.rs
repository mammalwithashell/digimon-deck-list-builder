//! EX13-045 overall processing: official rule 15-11-2-2 includes later entrants.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn setup(dna: bool) -> DebugRunner {
    let mut builder = DebugRunner::builder().dsl_card("EX13-045").unwrap();
    for (id, level, dp, color) in [("GREEN",6,12000,CardColor::Green),("BLUE",6,12000,CardColor::Blue),("LATER",3,5000,CardColor::Red),("PAD",3,1000,CardColor::Red)] {
        let mut card=make_test_card(id,id);card.level=Some(level);card.dp=Some(dp);card.colors=vec![color];card.play_cost=3;
        builder=builder.add_card(card);
    }
    let mut tamer=make_test_card("TAMER","Tamer");tamer.card_kind=CardKind::Tamer;tamer.dp=None;tamer.level=None;
    let mut r=builder.add_card(tamer).hand(0,&["EX13-045"]).deck(0,&["PAD";20]).deck(1,&["PAD";20]).security(1,&["PAD";5]).memory(10).start();
    r.game.current_phase=GamePhase::Main;
    if dna {
        r.place_on_field(0,"GREEN",Some(0));r.place_on_field(0,"BLUE",Some(0));
        assert!(r.game.initiate_dna_digivolve(0,0));
        r.game.resolve_selection(0,0).unwrap();r.game.resolve_selection(0,1).unwrap();
        let prompt=r.pending_selection_view().expect("mandatory DNA attack target");
        assert!(!prompt.is_optional);
        let action=*prompt.valid_action_ids.first().unwrap();
        r.execute_action(prompt.selecting_player,action).unwrap();
        r.auto_resolve().unwrap();
        assert!(r.game.pending_selection.is_none());
    } else {
        let h=r.place_on_field(0,"EX13-045",Some(0));
        r.game.enqueue_triggered(EffectTiming::WhenDigivolving,TriggerSource::Permanent(h));r.game.drain_effect_queue();
    }
    r
}

fn later_play(r: &mut DebugRunner) -> PermanentHandle {
    let hand=r.add_to_hand(0,"LATER");
    let index=r.play(0,hand).expect("late Digimon play legal");
    PermanentHandle{player:0,index:index as u8}
}

#[test]
fn dna_overall_dp_applies_to_later_play_after_source_leaves_and_in_clones() {
    let mut r=setup(true);let saved=r.game.clone();
    for remove_source in [false,true] {
        r.game=saved.clone();
        if remove_source {assert!(r.game.return_to_hand(PermanentHandle{player:0,index:0}).is_some());}
        let late=later_play(&mut r);
        assert_eq!(r.effective_dp(late),Some(15000),"overall buff includes later entry, source_removed={remove_source}");
        let enemy=r.place_on_field(1,"LATER",Some(0));
        assert_eq!(r.effective_dp(enemy),Some(5000),"opponent excluded");
        let tamer=r.place_on_field(0,"TAMER",Some(0));
        assert_eq!(r.effective_dp(tamer),None,"ordinary Tamer excluded");
    }
}

#[test]
fn ordinary_digivolution_does_not_grant_later_entry_dp() {
    let mut r=setup(false);let late=later_play(&mut r);
    assert_eq!(r.effective_dp(late),Some(5000));
}

#[test]
fn overall_dp_expires_after_opponent_turn_for_existing_and_future_entries() {
    let mut r=setup(true);let late=later_play(&mut r);
    assert_eq!(r.effective_dp(late),Some(15000));
    for target in [1,0] {
        for _ in 0..8 {
            if r.turn_player()==target {break;}
            r.auto_resolve().unwrap();r.pass_turn();r.auto_resolve().unwrap();
            if r.game.current_phase==GamePhase::EndOfTurnAction {r.game.pass_end_of_turn_action();r.auto_resolve().unwrap();}
        }
        assert_eq!(r.turn_player(),target);
        assert_eq!(r.effective_dp(late),Some(if target==1 {15000} else {5000}));
    }
    r.game.current_phase=GamePhase::Main;
    let next=later_play(&mut r);assert_eq!(r.effective_dp(next),Some(5000));
}

// Official 15-1-2: resolve the printed attack declaration, then overall DP.
// DCGO EX13_045 activates SelectAttackEffect before ChangeDigimonDPPlayerEffect;
// AttackProcess.Attack returns at Counter, before the attack battle resolves.
fn declaration_order_setup(cannot_attack: bool) -> DebugRunner { declaration_order_setup_with_replacement(cannot_attack,false) }
fn declaration_order_setup_with_replacement(cannot_attack: bool, replacement:bool) -> DebugRunner {
    use digimon_engine::action::build_action_mask;
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut b=DebugRunner::builder().dsl_card("EX13-045").unwrap();
    for (id,level,dp,color) in [("GREEN",6,12000,CardColor::Green),("BLUE",6,12000,CardColor::Blue),("ALLY",3,5000,CardColor::Red),("ENEMY20",6,20000,CardColor::Black),("PAD",3,1000,CardColor::Red)] {
        let mut c=make_test_card(id,id);c.level=Some(level);c.dp=Some(dp);c.colors=vec![color];c.play_cost=3;b=b.add_card(c);
    }
    let mut r=b.hand(0,&["EX13-045"]).deck(0,&["PAD";20]).deck(1,&["PAD";20]).security(1,&["PAD";5]).memory(10).start();
    if replacement {r.register_effect("ALLY",std::sync::Arc::new(OrderOptionalCancel));}
    r.skip_mulligan();r.set_phase(GamePhase::Main);
    r.place_on_field(0,"GREEN",Some(0));r.place_on_field(0,"BLUE",Some(0));r.place_on_field(0,"ALLY",Some(0));
    r.place_on_field(1,"ENEMY20",Some(0));r.game.players[1].battle_area[0].is_suspended=true;
    if cannot_attack {r.game.modifiers.add_player_modifier(0,PlayerModifierEntry::simple(ModifierType::CannotAttack,1,Expiry::EndOfTurn,None,0));}
    assert_eq!(build_action_mask(&r.game,0)[63],1.0);
    r.game.decode_action(63,0);order_act(&mut r,0);order_act(&mut r,1);r
}
fn order_act(r:&mut DebugRunner,a:u16) {
    let v=r.pending_selection_view().expect("authored decision");
    assert_eq!(digimon_engine::action::build_action_mask(&r.game,v.selecting_player)[a as usize],1.0,"{}: action {a}",v.prompt);
    r.execute_action(v.selecting_player,a).unwrap();
}
fn order_examon(r:&DebugRunner)->PermanentHandle {
    let index=r.game.players[0].battle_area.iter().position(|p|p.top_card().card_id(&r.game.card_data)=="EX13-045").unwrap();
    PermanentHandle{player:0,index:index as u8}
}
#[test]
fn dna_target_declaration_precedes_overall_dp_in_clones() {
    let mut r=declaration_order_setup(false);let g=r.game.clone();
    for _ in 0..2 {r.game=g.clone();let v=r.pending_selection_view().unwrap();assert!(v.prompt.contains("attacks"));
        assert_eq!(r.effective_dp(order_examon(&r)),Some(15000),"attack target is declared before printed next processing grants DP");
        let ally=r.game.players[0].battle_area.iter().position(|p|p.top_card().card_id(&r.game.card_data)=="ALLY").unwrap();
        assert_eq!(r.effective_dp(PermanentHandle{player:0,index:ally as u8}),Some(5000));
    }
}
#[test]
fn dna_dp_is_installed_before_direct_and_attack_battles_against_twenty_thousand() {
    use digimon_engine::action::space::{encode_attack,PASS};
    let mut r=declaration_order_setup(false);let g=r.game.clone();
    for direct_battle in [false,true] {for _ in 0..2 {
        r.game=g.clone();let me=order_examon(&r);order_act(&mut r,encode_attack(me.index as u16,0));
        let v=r.pending_selection_view().expect("WD direct battle choice before attack battle");
        assert!(v.prompt.contains("may battle"),"{}",v.prompt);
        assert_eq!(r.effective_dp(order_examon(&r)),Some(25000), "floating={:?}", r.game.floating_mass_modifiers);
        assert_eq!(r.game.players[1].battle_area.len(),1,"attack battle has not resolved");
        order_act(&mut r,if direct_battle {100} else {PASS});
        r.auto_resolve().unwrap();
        assert!(r.game.players[0].battle_area.iter().any(|p|p.top_card().card_id(&r.game.card_data)=="EX13-045"));
        assert!(r.game.players[1].battle_area.is_empty(),"25000 Examon must defeat 20000 enemy, direct={direct_battle}");
        assert!(r.game.players[1].trash.iter().any(|c|c.card_id(&r.game.card_data)=="ENEMY20"));
    }}
}
#[test]
fn dna_unable_to_attack_still_grants_dp_and_offers_direct_battle_in_clones() {
    use digimon_engine::action::space::PASS;
    let mut r=declaration_order_setup(true);let g=r.game.clone();
    for _ in 0..2 {r.game=g.clone();let v=r.pending_selection_view().expect("later optional processing still runs");assert!(v.prompt.contains("may battle"));
        assert_eq!(r.effective_dp(order_examon(&r)),Some(25000), "floating={:?}", r.game.floating_mass_modifiers);
        order_act(&mut r,PASS);r.auto_resolve().unwrap();
        assert_eq!(r.security_count(1),5);assert_eq!(r.game.players[1].battle_area.len(),1);
        assert!(!r.game.players[0].battle_area[order_examon(&r).index as usize].is_suspended,"failed attack did not suspend");
    }
}

#[test]
fn dna_would_attack_replacement_parks_before_dp_then_resumes_inner_before_outer_in_clones() {
    use digimon_engine::action::space::{encode_attack, PASS, REPLACEMENT_ACCEPT};
    let mut r=declaration_order_setup_with_replacement(false,true);
    let me=order_examon(&r);order_act(&mut r,encode_attack(me.index as u16,0));
    assert_eq!(r.effective_dp(order_examon(&r)),Some(15000),"replacement precedes next printed processing");
    let g=r.game.clone();
    for accept in [false,true] {for _ in 0..2 {
        r.game=g.clone();order_act(&mut r,if accept {REPLACEMENT_ACCEPT} else {PASS});
        assert!(r.pending_selection_view().unwrap().prompt.contains("may battle"));
        assert_eq!(r.effective_dp(order_examon(&r)),Some(25000),"inner DP tail precedes outer direct battle");
        order_act(&mut r,PASS);r.auto_resolve().unwrap();
        assert_eq!(r.game.players[1].battle_area.len(),if accept {1} else {0});
    }}
}

struct OrderOptionalCancel;
impl digimon_engine::effect::CardEffect for OrderOptionalCancel {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::EffectBuilder::new(c,EffectTiming::WhenWouldAttack)
   .name("optional attack cancellation control").optional()
   .replacement_process(|ctx|ctx.cancel()).build()]
 }
}
