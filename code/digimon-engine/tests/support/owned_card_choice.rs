//! Fixture-only physical menu lookup. Never send an old action alias to gameplay.
use digimon_engine::{action::build_action_mask,game::Game,owned_card_choice::{CardChoiceOperation,CardChoiceRowView,CardChoiceZone},permanent::PermanentHandle};
pub fn commits(game:&Game)->Vec<CardChoiceRowView> {
    let pending=game.pending_selection.as_ref().expect("physical card selection is installed");
    let actor=pending.selecting_player;
    let mask=build_action_mask(game,actor);
    pending.card_choice.as_ref().expect("selection exposes its owned physical menu").view(game,actor).rows.into_iter()
        .filter(|row|matches!(row.operation,CardChoiceOperation::CommitCard)&&mask[row.action_id as usize]>0.5).collect()
}
pub fn card_action(game:&Game,id:&str,zone:CardChoiceZone)->u16 {
    commits(game).into_iter().find(|row|row.card_id.as_deref()==Some(id)
        &&row.zone.as_ref().is_some_and(|actual|std::mem::discriminant(actual)==std::mem::discriminant(&zone)))
        .unwrap_or_else(||panic!("printed card {id} must be a legal commit in requested zone")).action_id
}
pub fn source_action(game:&Game,host:PermanentHandle,index:usize)->u16 {
    let row=commits(game).into_iter().find(|row|matches!(row.zone,Some(CardChoiceZone::BattleSource|CardChoiceZone::BreedingSource))
        &&row.owner==Some(host.player)&&row.carrier_slot==Some(host.index)&&row.position.as_deref()==Some(index.to_string().as_str()))
        .expect("specified original carrier and physical source position must be selectable");
    let stack=if host.index==digimon_engine::action::space::BREEDING_TARGET as u8 {&game.player(host.player).breeding_area.as_ref().unwrap().card_sources}
        else {&game.player(host.player).battle_area[host.index as usize].card_sources};
    assert_eq!(row.card_id.as_deref(),Some(stack[index].card_id(&game.card_data)),"row printed identity matches the physical source");
    row.action_id
}
pub fn battle_top_action(game:&Game,host:PermanentHandle)->u16 {
    let row=commits(game).into_iter().find(|row|matches!(row.zone,Some(CardChoiceZone::BattleTop))
        &&row.owner==Some(host.player)&&row.carrier_slot==Some(host.index)).expect("specified battle top must be selectable");
    assert_eq!(row.card_id.as_deref(),Some(game.player(host.player).battle_area[host.index as usize].top_card().card_id(&game.card_data)));
    row.action_id
}

/// Pick a physical security position without granting inspection permission.
pub fn security_action(game:&Game,owner:u8,index:usize)->u16 {
    let position=index.to_string();
    let row=commits(game).into_iter().find(|row|matches!(row.zone,Some(CardChoiceZone::Security))
        &&row.owner==Some(owner)&&row.position.as_deref()==Some(position.as_str()))
        .expect("the requested physical security position is selectable");
    if row.identity_visible {
        assert_eq!(row.card_id.as_deref(),Some(game.player(owner).security[index].card_id(&game.card_data)));
    } else {assert!(row.card_id.is_none());assert!(row.card_registry_index.is_none());}
    row.action_id
}

/// Resolve a known fixture position using only its public menu row and mask.
pub fn zone_action(game:&Game,owner:u8,index:usize,zone:CardChoiceZone)->u16 {
    let position=index.to_string();
    let row=commits(game).into_iter().find(|row| row.owner==Some(owner)
        &&row.position.as_deref()==Some(position.as_str())
        &&row.zone.as_ref().is_some_and(|actual|std::mem::discriminant(actual)==std::mem::discriminant(&zone)))
        .expect("requested physical owner/zone/position must be selectable");
    let card=match zone { CardChoiceZone::Hand=>&game.player(owner).hand[index],CardChoiceZone::Trash=>&game.player(owner).trash[index],
        _=>panic!("zone_action serves hand/trash fixtures") };
    if row.identity_visible {assert_eq!(row.card_id.as_deref(),Some(card.card_id(&game.card_data)));}
    else {assert!(row.card_id.is_none());assert!(row.card_registry_index.is_none());}
    row.action_id
}
