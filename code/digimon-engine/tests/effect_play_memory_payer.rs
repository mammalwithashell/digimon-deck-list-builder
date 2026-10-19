use digimon_engine::debug_runner::{DebugRunner,make_test_card};
use digimon_engine::enums::{CostDelta,PlaySource};
fn setup(actor:u8, active:u8, memory:i16, trash:bool)->DebugRunner {
 let mut card=make_test_card("PAYER-CARD","Payment witness");card.play_cost=3;
 let mut r=DebugRunner::builder().add_card(card).add_card(make_test_card("FILL","Deck witness")).deck(0,&["FILL";10]).deck(1,&["FILL";10]).memory(0).start();
 r.skip_mulligan();r.game.turn_player_idx=active as usize;r.game.memory=memory;
 if trash {r.inject_trash(actor,"PAYER-CARD");}else{r.add_to_hand(actor,"PAYER-CARD");}r
}
fn play(r:&mut DebugRunner,actor:u8,trash:bool)->bool{if trash{r.game.play_from_trash_with_cost(actor,0,CostDelta::Reduce(0),PlaySource::ByEffect).is_some()}else{let i=r.game.players[actor as usize].hand.iter().position(|c|c.card_id(&r.game.card_data)=="PAYER-CARD").unwrap();r.game.play_from_hand_with_cost(actor,i,CostDelta::Reduce(0),PlaySource::ByEffect).is_some()}}
#[test]fn paid_effect_play_moves_memory_relative_to_payer(){for actor in [0,1]{for active in [0,1]{for trash in [false,true]{let mut r=setup(actor,active,0,trash);assert!(play(&mut r,actor,trash));assert_eq!(r.memory(),if actor==active{-3}else{3},"actor={actor} active={active} trash={trash}");}}}}
#[test]fn inactive_payer_can_pay_at_negative_edge(){for actor in [0,1]{for trash in [false,true]{let mut r=setup(actor,1-actor,-10,trash);assert!(play(&mut r,actor,trash));assert_eq!(r.memory(),-7);}}}
#[test]fn inactive_payer_overdraw_fails_without_moving_card_or_memory(){for actor in [0,1]{for trash in [false,true]{let mut r=setup(actor,1-actor,8,trash);let hand=r.hand_size(actor);let discard=r.game.players[actor as usize].trash.len();assert!(!play(&mut r,actor,trash));assert_eq!(r.memory(),8);assert_eq!(r.hand_size(actor),hand);assert_eq!(r.game.players[actor as usize].trash.len(),discard);assert_eq!(r.battle_area_size(actor),0);}}}
#[test]fn active_payer_overdraw_remains_rejected(){for actor in [0,1]{for trash in [false,true]{let mut r=setup(actor,actor,-8,trash);assert!(!play(&mut r,actor,trash));assert_eq!(r.memory(),-8);assert_eq!(r.battle_area_size(actor),0);}}}

