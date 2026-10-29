//! Scoped canonical-fixture physical choice lookup.
use digimon_engine::{game::Game,action::build_action_mask,owned_card_choice::{CardChoiceOperation,CardChoiceZone}};
pub fn action(game:&Game, owner:u8,id:&str,zone:&str)->u16 {
 let p=game.pending_selection.as_ref().expect("pending choice");assert_eq!(p.selecting_player,owner);
 let page=p.card_choice.as_ref().expect("physical choice owner").view(game,owner);
 let row=page.rows.iter().find(|r|matches!(r.operation,CardChoiceOperation::CommitCard)&&r.owner==Some(owner)&&r.card_id.as_deref()==Some(id)&&match zone{"trash"=>matches!(r.zone,Some(CardChoiceZone::Trash)),"hand"=>matches!(r.zone,Some(CardChoiceZone::Hand)),"breeding"=>matches!(r.zone,Some(CardChoiceZone::BreedingSource)),_=>panic!("unknown fixture zone")}).unwrap_or_else(||panic!("no legal physical {zone} row for {id}"));
 assert!(row.identity_visible);assert_eq!(build_action_mask(game,owner)[row.action_id as usize],1.0);assert!(build_action_mask(game,1-owner).iter().all(|v|*v==0.0));row.action_id
}
