//! Translate DSL modifier/keyword strings into engine enums.

use crate::enums::{Keyword, ModifierType};

pub fn lookup_modifier_type(name: &str) -> Option<ModifierType> {
    Some(match name {
        "CannotActivateSecurityEffects" => ModifierType::CannotActivateSecurityEffects,
        "CannotPlayDigimonByEffect" => ModifierType::CannotPlayDigimonByEffect,
        "CannotPlayTamerByEffect" => ModifierType::CannotPlayTamerByEffect,
        "IgnoreColorRequirement" => ModifierType::IgnoreColorRequirement,
        "CannotActivateMainEffects" => ModifierType::CannotActivateMainEffects,
        "CannotActivateWhenDigivolvingEffects" => {
            ModifierType::CannotActivateWhenDigivolvingEffects
        }
        "CannotActivateWhenAttackingEffects" => ModifierType::CannotActivateWhenAttackingEffects,
        "CannotDigivolveDigimonByEffect" => ModifierType::CannotDigivolveDigimonByEffect,
        "CannotGainMemoryByEffect" => ModifierType::CannotGainMemoryByEffect,
        "CannotGainMemoryExceptFromTamers" => ModifierType::CannotGainMemoryExceptFromTamers,
        "CannotReducePlayCost" => ModifierType::CannotReducePlayCost,
        "CannotReduceDigivolveCost" => ModifierType::CannotReduceDigivolveCost,
        "CannotDrawByEffect" => ModifierType::CannotDrawByEffect,
        "CannotAddSecurityByEffect" => ModifierType::CannotAddSecurityByEffect,
        "CannotTrashOpponentSecurity" => ModifierType::CannotTrashOpponentSecurity,
        "CannotReduceOpponentSecurity" => ModifierType::CannotReduceOpponentSecurity,
        "CannotPlayFromHand" => ModifierType::CannotPlayFromHand,
        "CannotBeDestroyed" => ModifierType::CannotBeDestroyed,
        "CannotBeDestroyedByBattle" => ModifierType::CannotBeDestroyedByBattle,
        "CannotBeDestroyedByEffect" => ModifierType::CannotBeDestroyedByEffect,
        "CannotBeRemoved" => ModifierType::CannotBeRemoved,
        "CannotAttack" => ModifierType::CannotAttack,
        "CannotAttackPlayer" => ModifierType::CannotAttackPlayer,
        "CannotSuspend" => ModifierType::CannotSuspend,
        "CannotUnsuspend" => ModifierType::CannotUnsuspend,
        "CannotBeSelectedByEffect" => ModifierType::CannotBeSelectedByEffect,
        "CannotBeAffected" => ModifierType::CannotBeAffected,
        "CannotReduceCost" => ModifierType::CannotReduceCost,
        _ => return None,
    })
}

pub fn lookup_keyword(name: &str, value: Option<i32>) -> Option<Keyword> {
    Some(match name {
        "Blocker" => Keyword::Blocker,
        "Rush" => Keyword::Rush,
        "Jamming" => Keyword::Jamming,
        "Piercing" => Keyword::Piercing,
        "Reboot" => Keyword::Reboot,
        "Blitz" => Keyword::Blitz,
        "Raid" => Keyword::Raid,
        "Alliance" => Keyword::Alliance,
        "BlastDigivolve" => Keyword::BlastDigivolve,
        "Save" => Keyword::Save,
        "MaterialSave" => Keyword::MaterialSave(value.unwrap_or(1) as u8),
        "Fortitude" => Keyword::Fortitude,
        "Overclock" => Keyword::Overclock,
        "Barrier" => Keyword::Barrier,
        "Decoy" => Keyword::Decoy,
        "Partition" => Keyword::Partition,
        "Vortex" => Keyword::Vortex,
        "Collision" => Keyword::Collision,
        "Evade" => Keyword::Evade,
        "Decode" => Keyword::Decode,
        "ArmorPurge" => Keyword::ArmorPurge,
        "SecurityAttackPlus" => Keyword::SecurityAttackPlus(value.unwrap_or(1) as i8),
        "SecurityAttackMinus" => Keyword::SecurityAttackMinus(value.unwrap_or(1) as i8),
        "DeDigivolve" => Keyword::DeDigivolve(value.unwrap_or(1) as u8),
        "DrawX" => Keyword::DrawX(value.unwrap_or(1) as u8),
        "Fragment" => Keyword::Fragment(value.unwrap_or(1) as u8),
        "Progress" => Keyword::Progress,
        _ => return None,
    })
}
