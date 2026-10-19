//! Rules8-2-2-1 new-object DNA semantics. Synthetic cards isolate shared engine behavior.
use digimon_engine::{debug_runner::{DebugRunner,make_test_card,make_test_dna_card}, effect_context::EffectContext, game::Game, permanent::PermanentHandle, enums::{Expiry,Keyword,ModifierType}, modifiers::ModifierEntry};

fn seed(g:&mut Game,h:PermanentHandle) {
    let link=g.players[0].hand.pop().expect("link fixture");
    let mut lower=g.players[0].hand.pop().expect("face-down source fixture");
    assert_eq!(lower.card_id(&g.card_data),"SOURCE");
    lower.face_down=true;lower.owner=1;
    let p=&mut g.players[h.player as usize].battle_area[h.index as usize];
    p.card_sources.insert(0,lower);
    p.linked_cards.push(link); p.is_suspended=true; p.attacks_this_turn=3; p.is_attacking=true;
    p.record_activation(p.top_card().handle(),0);
    p.top_card_mut().also_treated_as.push("Temporary granted name".into());
    g.modifiers.add(h,ModifierEntry::simple(ModifierType::ChangeDp,5000,Expiry::EndOfTurn,0));
    g.modifiers.grant_keyword(h,Keyword::Blocker,Expiry::EndOfTurn,0);
}
fn verify(g:&Game,h:PermanentHandle,links:usize) {
    let p=&g.players[0].battle_area[h.index as usize];
    assert_eq!(p.top_card().card_id(&g.card_data),"RESULT");
    assert!(!p.is_suspended,"DNA must not inherit suspension");
    assert!(g.can_attack(h,false),"DNA must not inherit fresh-play attack restriction");
    assert_eq!(p.attacks_this_turn,0); assert!(!p.is_attacking);
    assert!(p.effect_activations.is_empty(),"OPT state belongs to old objects");
    assert!(p.linked_cards.is_empty());
    assert!(p.card_sources.iter().all(|c|c.also_treated_as.is_empty()));
    for lower in p.card_sources.iter().filter(|c|c.card_id(&g.card_data)=="SOURCE") {
        assert!(lower.face_down);assert_eq!(lower.owner,1);
        assert!(g.card_data[lower.data_index].also_treated_as.contains(&"Static source alias".into()));
    }
    assert_eq!(g.modifiers.sum(h,ModifierType::ChangeDp),0);
    assert!(!g.has_keyword(h,Keyword::Blocker),"granted keyword must end");
    assert_eq!(g.players[0].trash.iter().filter(|c|c.card_id(&g.card_data)=="LINK").count(),links);
}
fn exercise(route:u8) {
    let mut a=make_test_card("A","A");a.level=Some(6);
    let mut b=make_test_card("B","B");b.level=Some(6);
    let mut lower=make_test_card("SOURCE","Source");lower.also_treated_as=vec!["Static source alias".into()];
    let mut link=make_test_card("LINK","Link");link.ace_overflow=Some(-2);
    let mut result=make_test_dna_card("RESULT","Result",6,6,0);result.level=Some(7);
    let mut r=DebugRunner::builder().add_card(a).add_card(b).add_card(result).add_card(lower)
        .add_card(link).add_card(make_test_card("DRAW","Draw"))
        .hand(0,&["RESULT","B","SOURCE","LINK","SOURCE","LINK"]).deck(0,&["DRAW";4]).memory(10).start();
    let a=r.place_on_field(0,"A",None);
    let field_b=if route==0 || route==3 {Some(r.place_on_field(0,"B",None))}else{None};
    seed(&mut r.game,a);if let Some(b)=field_b {seed(&mut r.game,b);}
    r.game.modifiers.add_player_modifier(0,digimon_engine::modifiers::PlayerModifierEntry::simple(
        ModifierType::ChangeOwnSecurityDigimonDp,1000,Expiry::EndOfTurn,Some(a),0));
    for card in &mut r.game.players[0].hand {
        card.also_treated_as.push("Old hand effect alias".into());
    }
    assert!(r.game.mulligan_current_player().is_none(),"builder must finish mulligan bookkeeping");
    r.game.current_phase=digimon_engine::enums::GamePhase::Main;
    assert_eq!(r.game.turn_player(),0);
    let physical_before = physical_cards(&r.game);
    // Both routes begin from exact same physical state, including pending-independent clone.
    let mut clone=r.game.clone();
    for g in [&mut r.game,&mut clone] {
        let card=g.players[0].battle_area[a.index as usize].top_card().handle();
        let result=g.players[0].hand[0].handle();
        let partner=g.players[0].hand.iter().find(|c|c.card_id(&g.card_data)=="B").unwrap().handle();
        if route==2 {let i=g.players[0].hand.iter().position(|c|c.handle()==partner).unwrap();let c=g.players[0].hand.remove(i);g.players[0].trash.push(c);}
        let merged=if route==3 {
            let declaration=digimon_engine::action::space::DNA_DIGIVOLVE_START;
            assert!(digimon_engine::action::mask::build_action_mask(g,0)[declaration as usize]>0.0,
                "native DNA declaration must be exposed to the agent");
            g.decode_action(declaration,0);
            for action in [a.index as u16,field_b.unwrap().index as u16] {
                let p=g.pending_selection.as_ref().expect("native DNA material prompt");
                assert!(p.valid_action_ids.contains(&action));
                assert!(digimon_engine::action::mask::build_action_mask(g,0)[action as usize]>0.0);
                g.resolve_selection(0,action).unwrap();
            }
            a
        }else{
            let mut ctx=EffectContext::new(g,card,Some(a),0);
            match route {
                0=>ctx.effect_initiated_dna_digivolve(a,field_b.unwrap(),result,0,false),
                1=>ctx.effect_initiated_dna_digivolve_with_hand_partner(a,partner,result,0,false),
                2=>ctx.effect_initiated_dna_digivolve_trash_partner(a,partner,result,0,false),
                _=>unreachable!(),
            }.expect("valid DNA route commits")
        };
        let link_count=if field_b.is_some(){2}else{1};
        verify(g,merged,link_count);
        assert_eq!(g.memory,10-2*link_count as i16,"linked ACE Overflow applies exactly once each");
        assert_eq!(g.modifiers.player_modifier_value(0,ModifierType::ChangeOwnSecurityDigimonDp),1000,
            "independent player-scoped effects survive material reset");
        assert_eq!(physical_cards(g),physical_before,"every physical card is conserved");
    }
    assert_eq!(r.game.verification_digest(),clone.verification_digest());
}
#[test] fn effect_field_pair_resets_material_objects(){exercise(0)}
#[test] fn effect_hand_partner_resets_material_object(){exercise(1)}
#[test] fn effect_trash_partner_resets_material_object(){exercise(2)}
#[test] fn native_field_pair_resets_material_objects(){exercise(3)}

fn physical_cards(g:&Game)->Vec<u16>{
    let mut ids=Vec::new();
    for p in &g.players {
        for c in p.hand.iter().chain(p.deck.iter()).chain(p.trash.iter()).chain(p.security.iter()) {ids.push(c.handle().0);}
        for perm in &p.battle_area {for c in perm.card_sources.iter().chain(perm.linked_cards.iter()){ids.push(c.handle().0);}}
    }
    ids.sort(); ids
}
#[test]
fn rejected_dna_does_not_reset_materials(){
    let mut a=make_test_card("A","A");a.level=Some(6);
    let result=make_test_dna_card("RESULT","Result",7,7,0);
    let mut r=DebugRunner::builder().add_card(a).add_card(result).add_card(make_test_card("SOURCE","Source")).add_card(make_test_card("LINK","Link"))
        .hand(0,&["RESULT","SOURCE","LINK"]).memory(10).start();
    let a=r.place_on_field(0,"A",None);let b=r.place_on_field(0,"A",None);seed(&mut r.game,a);
    let before=r.game.verification_digest();
    let source=r.game.players[0].battle_area[0].top_card().handle();let result=r.game.players[0].hand[0].handle();
    let mut ctx=EffectContext::new(&mut r.game,source,Some(a),0);
    assert!(ctx.effect_initiated_dna_digivolve(a,b,result,0,false).is_none());
    assert_eq!(r.game.verification_digest(),before);
}


// Ordinary evolution is the same object: the DNA reset must not leak here.
#[test]
fn ordinary_evolution_retains_suspension_links_opt_and_gained_effects() {
    use digimon_engine::{card_data::EvoCost,enums::{CostDelta,PlaySource}};
    let mut base=make_test_card("BASE","Base");base.level=Some(6);
    let mut result=make_test_card("ORDINARY","Ordinary");result.level=Some(7);
    result.evo_costs=vec![EvoCost{card_color:0,level:6,memory_cost:0}];
    let mut r=DebugRunner::builder().add_card(base).add_card(result)
        .add_card(make_test_card("LINK","Link")).add_card(make_test_card("SOURCE","Source"))
        .add_card(make_test_card("DRAW","Draw"))
        .hand(0,&["ORDINARY","SOURCE","LINK"]).deck(0,&["DRAW";4]).memory(5).start();
    let host=r.place_on_field(0,"BASE",None);seed(&mut r.game,host);
    let old=r.game.players[0].battle_area[0].top_card().handle();
    let link=r.game.players[0].battle_area[0].linked_cards[0].handle();
    let old_turn=r.game.players[0].battle_area[0].turn_played;
    let before=physical_cards(&r.game);let mut clone=r.game.clone();
    for g in [&mut r.game,&mut clone] {
        assert!(g.effect_initiated_digivolve(0,0,host,CostDelta::Free,false,PlaySource::ByEffect));
        let p=&g.players[0].battle_area[0];
        assert_eq!(p.top_card().card_id(&g.card_data),"ORDINARY");
        assert!(p.is_suspended);assert_eq!(p.turn_played,old_turn);
        assert_eq!(p.attacks_this_turn,3);assert!(p.is_attacking);
        assert_eq!(p.linked_cards[0].handle(),link);
        assert!(p.effect_activations.keys().any(|(card,_)|*card==old));
        assert_eq!(g.modifiers.sum(host,ModifierType::ChangeDp),5000);
        assert!(g.has_keyword(host,Keyword::Blocker));
        assert_eq!(physical_cards(g),before);
    }
    assert_eq!(r.game.verification_digest(),clone.verification_digest());
}

#[test]
fn dna_cross_owner_link_overflow_applies_turn_owner_first_at_memory_limit() {
    // Deliberately seat the non-turn-owner ACE on the first material. Applying
    // each material's Overflow separately gives 8 (clamp 9+4 to10, then-2);
    // simultaneous turn-owner-first processing gives10 (9-2+4 clamped).
    let mut a=make_test_card("A","A");a.level=Some(6);
    let mut b=make_test_card("B","B");b.level=Some(6);
    let mut own=make_test_card("OWN-ACE","Own ACE");own.ace_overflow=Some(-2);
    let mut foreign=make_test_card("FOREIGN-ACE","Foreign ACE");foreign.ace_overflow=Some(-4);
    let result=make_test_dna_card("RESULT","Result",6,6,0);
    let mut r=DebugRunner::builder().add_card(a).add_card(b).add_card(own).add_card(foreign).add_card(result)
        .add_card(make_test_card("DRAW","Draw")).deck(0,&["DRAW";4])
        .hand(0,&["RESULT","OWN-ACE"]).hand(1,&["FOREIGN-ACE"]).memory(9).start();
    let a=r.place_on_field(0,"A",None);let b=r.place_on_field(0,"B",None);
    let own=r.game.players[0].hand.pop().unwrap();let own_id=own.handle();
    let foreign=r.game.players[1].hand.pop().unwrap();let foreign_id=foreign.handle();
    assert_eq!(own.owner,0);assert_eq!(foreign.owner,1);
    r.game.players[0].battle_area[0].linked_cards.push(foreign);
    r.game.players[0].battle_area[1].linked_cards.push(own);
    let before=physical_cards(&r.game);let mut clone=r.game.clone();
    for g in [&mut r.game,&mut clone] {
        let source=g.players[0].battle_area[0].top_card().handle();let result=g.players[0].hand[0].handle();
        assert!(EffectContext::new(g,source,Some(a),0).effect_initiated_dna_digivolve(a,b,result,0,false).is_some());
        assert_eq!(g.memory,10,"simultaneous owner-relative Overflow order");
        assert_eq!(g.players[0].trash.iter().filter(|c|c.handle()==own_id).count(),1);
        assert_eq!(g.players[1].trash.iter().filter(|c|c.handle()==foreign_id).count(),1);
        assert!(!g.players[0].trash.iter().any(|c|c.handle()==foreign_id));
        assert_eq!(physical_cards(g),before);
    }
    assert_eq!(r.game.verification_digest(),clone.verification_digest());
}

#[test]
fn dna_invalidates_queued_old_material_but_preserves_bystander_and_new_inherited_effect() {
    use digimon_engine::{enums::EffectTiming,selection::TriggerSource};
    const MATERIAL:&str=r#"
card: QUEUED-A
name: Queued material
kind: digimon
level: 6
color: [red]
cost: 10
dp: 10000
effects:
  - when: on_play
    process:
      - gain_memory: 1
  - scope: inherited
    when: when_attacking
    process:
      - gain_memory: 4
"#;
    const BYSTANDER:&str=r#"
card: QUEUED-OTHER
name: Queued bystander
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - gain_memory: 2
"#;
    let mut b=make_test_card("B","B");b.level=Some(6);
    let result=make_test_dna_card("RESULT","Result",6,6,0);
    let mut r=DebugRunner::builder().from_dsl_yaml(MATERIAL).unwrap().from_dsl_yaml(BYSTANDER).unwrap()
        .add_card(b).add_card(result).add_card(make_test_card("DRAW","Draw"))
        .hand(0,&["RESULT"]).deck(0,&["DRAW";4]).memory(0).start();
    let a=r.place_on_field(0,"QUEUED-A",None);let b=r.place_on_field(0,"B",None);
    let other=r.place_on_field(0,"QUEUED-OTHER",None);
    // Explicit dispatcher substrate: collect, then DNA before activation.
    // This is not a claim that OnPlay is newly triggered by field placement.
    r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(a));
    r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(other));
    let mut control=DebugRunner::wrap(r.game.clone());
    control.game.drain_effect_queue();let _=control.auto_resolve();
    assert_eq!(control.game.memory,3,"both original queued effects must exist");
    let mut clone=r.game.clone();
    for g in [&mut r.game,&mut clone] {
        let source=g.players[0].battle_area[0].top_card().handle();let result=g.players[0].hand[0].handle();
        let merged=EffectContext::new(g,source,Some(a),0).effect_initiated_dna_digivolve(a,b,result,0,false).unwrap();
        g.drain_effect_queue();
        assert!(g.pending_selection.is_none());
        assert_eq!(g.memory,2,"old material activation invalidated; bystander survives compaction");
        g.enqueue_triggered(EffectTiming::WhenAttacking,TriggerSource::Permanent(merged));
        g.drain_effect_queue();
        assert_eq!(g.memory,6,"new inherited effect remains available after reset");
    }
    assert_eq!(r.game.verification_digest(),clone.verification_digest());
}
