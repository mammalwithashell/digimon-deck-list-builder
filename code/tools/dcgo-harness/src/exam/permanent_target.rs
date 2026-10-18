//! Public, state-bound permanent target descriptors. No slot-order equivalence.
use digimon_engine::{Game,permanent::PermanentHandle,selection::AttackTarget};
#[derive(Debug,Clone,PartialEq,Eq,serde::Serialize)]
pub struct PermanentTarget {
    pub version:u8,
    #[serde(skip_serializing_if="Option::is_none")]
    pub play_ref:Option<String>,
    pub owner:u8,
    pub card_id:String,
    pub sources:Vec<String>,
    pub linked:Vec<String>,
    pub suspended:bool,
    pub has_dp:bool,
    pub dp:i64,
    pub attacker:bool,
    pub attack_target:bool,
}
pub fn snapshot(game:&Game,handle:PermanentHandle)->Result<PermanentTarget,String>{
    let p=game.players.get(handle.player as usize).and_then(|p|p.battle_area.get(handle.index as usize)).ok_or("physical target no longer exists")?;
    let (top,lower)=p.card_sources.split_last().ok_or("empty permanent")?;
    let public_id=|c:&digimon_engine::card_source::CardSource|->Result<String,String>{
        if c.is_token{return Err("physical target includes unsupported token identity".into());}
        if c.face_down{return Ok("<face-down>".into());}
        if c.is_opaque_placeholder{return Err("physical target includes unsupported opaque face-up identity".into());}
        game.card_data.get(c.data_index).map(|d|d.card_id.trim()).filter(|s|!s.is_empty()).map(str::to_owned).ok_or_else(||"physical target identity unavailable".into())
    };
    if top.face_down {return Err("physical target has hidden top identity".into());}
    // §11-2-6: once the attack target is removed it stays the target (DCGO's
    // `DefendingPermanent` is the removed object), so the Digimon that compacted
    // into its old slot is never the attack target on the wire.
    let attack=game.pending_attack.as_ref().filter(|a|a.declaration_committed);
    Ok(PermanentTarget{version:2,play_ref:None,owner:handle.player as u8,card_id:public_id(top)?,sources:lower.iter().map(&public_id).collect::<Result<_,_>>()?,linked:p.linked_cards.iter().map(public_id).collect::<Result<_,_>>()?,suspended:p.is_suspended,has_dp:game.effective_dp(handle).is_some(),dp:game.effective_dp(handle).map(i64::from).unwrap_or(-1),attacker:attack.is_some_and(|a|a.attacker==handle),attack_target:attack.is_some_and(|a|!a.battle_defender_deleted&&!a.effective_target_removed&&a.effective_target==AttackTarget::Digimon(handle))})
}
