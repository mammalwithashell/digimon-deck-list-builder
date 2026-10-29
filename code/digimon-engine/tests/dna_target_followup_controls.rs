//! Behavior-only controls for consumers beyond Counter-phase target DNA.
use std::sync::Arc;
use digimon_engine::{debug_runner::{DebugRunner,make_test_card,make_test_dna_card},
    card_source::CardHandle, effect::{CardEffect,Effect},
    effect_context::EffectContext, enums::{Expiry,Keyword,ModifierType,GamePhase},
    modifiers::ModifierEntry, permanent::PermanentHandle,
    action::{mask::build_action_mask,space::encode_attack},
    selection::AttackState, combat::AttackResult};

fn merge_target(ctx:&mut EffectContext<'_>)->PermanentHandle {
    let index=ctx.game.players[1].battle_area.iter()
        .position(|p|p.top_card().card_id(&ctx.game.card_data)=="MAT").expect("old material exists");
    let result=ctx.game.players[1].hand.iter().find(|c|c.card_id(&ctx.game.card_data)=="RESULT").unwrap().handle();
    let partner=ctx.game.players[1].hand.iter().find(|c|c.card_id(&ctx.game.card_data)=="PARTNER").unwrap().handle();
    ctx.effect_initiated_dna_digivolve_with_hand_partner(
        PermanentHandle{player:1,index:index as u8},partner,result,0,false).expect("valid hand-partner DNA")
}
struct BeforeRaid;
impl CardEffect for BeforeRaid {
    fn effects(&self,card:CardHandle)->Vec<Effect>{vec![
        Effect::when_attacking(card).name("test target DNA before Raid")
            .process(|ctx|{ merge_target(ctx); }).build()
    ]}
}
struct AfterProtectedBattle;
impl CardEffect for AfterProtectedBattle {
    fn effects(&self,card:CardHandle)->Vec<Effect>{vec![
        Effect::end_of_battle(card).name("test new-target effect battle")
            .condition(|ctx|ctx.game.players[1].hand.iter().any(|c|c.card_id(&ctx.game.card_data)=="RESULT"))
            .process(|ctx|{
                let attacker=ctx.source_permanent.expect("attacker witness remains");
                let pa=ctx.game.pending_attack.as_ref().expect("attack remains during EndOfBattle");
                assert_eq!(pa.state,AttackState::Battle);
                assert!(!pa.battle_defender_deleted,"initial material was protected from battle deletion");
                let result=merge_target(ctx);
                assert!(!ctx.game.modifiers.has(result,ModifierType::CannotBeDestroyedByBattle),
                    "DNA must remove material's gained protection before this consumer test");
                assert_eq!(ctx.battle_digimon(attacker,result),AttackResult::AttackerWins);
                assert!(!ctx.game.players[1].battle_area.iter().any(|p|p.top_card().card_id(&ctx.game.card_data)=="RESULT"));
                let pa=ctx.game.pending_attack.as_ref().expect("outer attack remains");
                assert!(!pa.battle_defender_deleted,
                    "effect battle with a new DNA Digimon must not overwrite old attack battle receipt");
                // The effect battle queues its BattleResult/Piercing trigger;
                // activation waits until this enclosing effect finishes.
                assert!(!pa.effect_battle_piercing_pending,
                    "effect-battle Piercing must not activate inside the resolving effect");
            }).build()
    ]}
}
fn setup(effect:Arc<dyn CardEffect>,keyword:Keyword)->(DebugRunner,PermanentHandle,PermanentHandle){
    let mut mat=make_test_card("MAT","Material");mat.level=Some(6);mat.dp=Some(2000);
    let mut partner=make_test_card("PARTNER","Partner");partner.level=Some(5);
    let mut result=make_test_dna_card("RESULT","Result",6,5,0);result.level=Some(7);result.dp=Some(5000);
    let mut attacker=make_test_card("ATT","Attacker");attacker.dp=Some(12000);
    let mut r=DebugRunner::builder().add_card(mat).add_card(partner).add_card(result).add_card(attacker)
        .add_card(make_test_card("SEC","Security")).add_card(make_test_card("DRAW","Draw"))
        .hand(1,&["RESULT","PARTNER"]).deck(0,&["DRAW";4]).deck(1,&["DRAW";4])
        .security(1,&["SEC";3]).memory(5).start();
    r.register_effect("ATT",effect);
    assert!(r.game.mulligan_current_player().is_none());r.game.current_phase=GamePhase::Main;
    let attacker=r.place_on_field(0,"ATT",Some(0));let target=r.place_on_field(1,"MAT",Some(0));
    r.game.players[1].battle_area[0].is_suspended=true;
    r.game.modifiers.grant_keyword(attacker,keyword,Expiry::Permanent,0);
    (r,attacker,target)
}
#[test]
fn pre_raid_dna_result_is_eligible_in_original_switch_window(){
    let (mut r,attacker,target)=setup(Arc::new(BeforeRaid),Keyword::Raid);
    let mut clone=r.game.clone();
    for g in [&mut r.game,&mut clone] {
        let declaration=encode_attack(attacker.index as u16,target.index as u16);
        assert!(build_action_mask(g,0)[declaration as usize]>0.0);g.decode_action(declaration,0);
        let pa=g.pending_attack.as_ref().expect("Raid attack remains");
        // §16-23-2: <Raid> resolves in the declaration's trigger pool (state
        // still Declared), not the post-Block rescue window (PostBlock).
        assert_eq!(pa.state,AttackState::Declared,"must be printed Raid switch, not later rescue window");
        let choice=g.pending_selection.as_ref().expect("new same-slot result must be offered for Raid");
        assert_eq!(choice.selecting_player,0);assert!(choice.is_optional);
        assert_eq!(choice.prompt,"Raid - switch attack target");
        let action=encode_attack(0,target.index as u16);
        assert!(choice.valid_action_ids.contains(&action));assert!(build_action_mask(g,0)[action as usize]>0.0);
        g.resolve_selection(0,action).unwrap();
        assert!(g.pending_selection.is_none());assert!(g.pending_attack.is_none());
        assert!(g.players[1].battle_area.is_empty(),"selected new result must be battled");
    }
    assert_eq!(r.game.verification_digest(),clone.verification_digest());
}
#[test]
fn effect_battle_after_target_dna_does_not_claim_outer_attack_battle_receipt(){
    let (mut r,attacker,target)=setup(Arc::new(AfterProtectedBattle),Keyword::Piercing);
    r.game.modifiers.add(target,ModifierEntry::simple(ModifierType::CannotBeDestroyedByBattle,1,Expiry::Permanent,1));
    let mut clone=r.game.clone();
    for g in [&mut r.game,&mut clone] {
        let declaration=encode_attack(attacker.index as u16,target.index as u16);
        assert!(build_action_mask(g,0)[declaration as usize]>0.0);g.decode_action(declaration,0);
        assert!(g.pending_selection.is_none());assert!(g.pending_attack.is_none());
        assert!(g.players[1].battle_area.is_empty());
        assert_eq!(g.players[1].security.len(),2,"one effect-battle Piercing check must occur");
    }
    assert_eq!(r.game.verification_digest(),clone.verification_digest());
}
