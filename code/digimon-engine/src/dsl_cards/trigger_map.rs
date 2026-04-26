//! Map string trigger names (as encoded in `CompiledDeclarativeClause::Replacement`)
//! to their corresponding `EffectTiming` variants.

use crate::enums::EffectTiming;

/// Return the `EffectTiming` for a replacement trigger name, or `None` if the
/// string is not a known "would" trigger.
pub fn lookup_replacement_trigger(s: &str) -> Option<EffectTiming> {
    match s {
        "when_would_be_deleted" => Some(EffectTiming::WhenWouldBeDeleted),
        "when_would_leave_battle_area" => Some(EffectTiming::WhenWouldLeaveBattleArea),
        "when_would_be_returned_to_hand" => Some(EffectTiming::WhenWouldBeReturnedToHand),
        "when_would_be_returned_to_deck" => Some(EffectTiming::WhenWouldBeReturnedToDeck),
        "when_would_be_trashed" => Some(EffectTiming::WhenWouldBeTrashed),
        "when_would_be_de_digivolved" => Some(EffectTiming::WhenWouldBeDeDigivolved),
        "when_would_lose_security" => Some(EffectTiming::WhenWouldLoseSecurity),
        "when_would_draw" => Some(EffectTiming::WhenWouldDraw),
        "when_would_place_in_security" => Some(EffectTiming::WhenWouldPlaceInSecurity),
        _ => None,
    }
}
