//! Map string trigger names (as encoded in `CompiledDeclarativeClause::Replacement`)
//! to their corresponding `EffectTiming` variants.

use crate::enums::EffectTiming;

/// Return the `EffectTiming` for a replacement trigger name, or `None` if the
/// string is not a known "would" trigger.
pub fn lookup_replacement_trigger(s: &str) -> Option<EffectTiming> {
    Some(match s {
        "when_would_be_deleted" => EffectTiming::WhenWouldBeDeleted,
        "when_would_leave_battle_area" => EffectTiming::WhenWouldLeaveBattleArea,
        "when_would_be_returned_to_hand" => EffectTiming::WhenWouldBeReturnedToHand,
        "when_would_be_returned_to_deck" => EffectTiming::WhenWouldBeReturnedToDeck,
        "when_would_be_trashed" => EffectTiming::WhenWouldBeTrashed,
        "when_would_be_de_digivolved" => EffectTiming::WhenWouldBeDeDigivolved,
        "when_would_lose_security" => EffectTiming::WhenWouldLoseSecurity,
        "when_would_draw" => EffectTiming::WhenWouldDraw,
        "when_would_place_in_security" => EffectTiming::WhenWouldPlaceInSecurity,
        _ => return None,
    })
}
