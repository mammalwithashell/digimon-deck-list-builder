//! Translate DSL modifier/keyword strings into engine enums.

use crate::enums::{Keyword, ModifierType};

pub fn lookup_modifier_type(name: &str) -> Option<ModifierType> {
    Some(match name {
        "ChangeDp" => ModifierType::ChangeDp,
        "ChangeBaseDp" => ModifierType::ChangeBaseDp,
        "DpFloor" => ModifierType::DpFloor,
        "DontHaveDp" => ModifierType::DontHaveDp,
        "ChangePlayCost" => ModifierType::ChangePlayCost,
        "ChangeDigivolveCost" => ModifierType::ChangeDigivolveCost,
        "CannotReduceCost" => ModifierType::CannotReduceCost,
        "CannotBeDestroyed" => ModifierType::CannotBeDestroyed,
        "CannotBeDestroyedByBattle" => ModifierType::CannotBeDestroyedByBattle,
        "CannotBeDestroyedByEffect" => ModifierType::CannotBeDestroyedByEffect,
        "CannotBeRemoved" => ModifierType::CannotBeRemoved,
        "CannotBeReturnedToDeck" => ModifierType::CannotBeReturnedToDeck,
        "CannotBeReturnedToHand" => ModifierType::CannotBeReturnedToHand,
        "CannotBeTrashedByEffect" => ModifierType::CannotBeTrashedByEffect,
        "CannotBeDeDigivolved" => ModifierType::CannotBeDeDigivolved,
        "CannotAttack" => ModifierType::CannotAttack,
        "CannotAttackPlayer" => ModifierType::CannotAttackPlayer,
        "VortexCanAttackPlayer" => ModifierType::VortexCanAttackPlayer,
        "CanAttackUnsuspended" => ModifierType::CanAttackUnsuspended,
        "CanAttackActivePlayer" => ModifierType::CanAttackActivePlayer,
        "CannotAttackTarget" => ModifierType::CannotAttackTarget,
        "CannotBeRedirectedAsAttackTarget" => ModifierType::CannotBeRedirectedAsAttackTarget,
        "CanNotSwitchAttackTarget" => ModifierType::CanNotSwitchAttackTarget,
        "CannotSuspend" => ModifierType::CannotSuspend,
        "CannotUnsuspend" => ModifierType::CannotUnsuspend,
        "CannotBeSelectedByEffect" => ModifierType::CannotBeSelectedByEffect,
        "CannotBeAffected" => ModifierType::CannotBeAffected,
        "GrantBlocker" => ModifierType::GrantBlocker,
        "GrantRush" => ModifierType::GrantRush,
        "GrantJamming" => ModifierType::GrantJamming,
        "GrantPiercing" => ModifierType::GrantPiercing,
        "GrantReboot" => ModifierType::GrantReboot,
        "GrantBlitz" => ModifierType::GrantBlitz,
        "GrantAlliance" => ModifierType::GrantAlliance,
        "GrantRaid" => ModifierType::GrantRaid,
        "GrantDecoy" => ModifierType::GrantDecoy,
        "GrantVortex" => ModifierType::GrantVortex,
        "GrantOverclock" => ModifierType::GrantOverclock,
        "MayAttack" => ModifierType::MayAttack,
        "ForceAttack" => ModifierType::ForceAttack,
        "SecurityAttackChange" => ModifierType::SecurityAttackChange,
        "ImmunityToOpponentEffects" => ModifierType::ImmunityToOpponentEffects,
        "DontBattleSecurityDigimon" => ModifierType::DontBattleSecurityDigimon,
        "CannotDigivolve" => ModifierType::CannotDigivolve,
        "ChangeColor" => ModifierType::ChangeColor,
        "AddColor" => ModifierType::AddColor,
        "ChangeLevel" => ModifierType::ChangeLevel,
        "CannotReturnToHand" => ModifierType::CannotReturnToHand,
        "CannotTrash" => ModifierType::CannotTrash,
        "CannotBlock" => ModifierType::CannotBlock,
        "CannotCounter" => ModifierType::CannotCounter,
        "DrawBlock" => ModifierType::DrawBlock,
        "MemoryBlock" => ModifierType::MemoryBlock,
        "CannotPlayFromHand" => ModifierType::CannotPlayFromHand,
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
        "CannotPlayFromTrash" => ModifierType::CannotPlayFromTrash,
        "CannotReducePlayCost" => ModifierType::CannotReducePlayCost,
        "CannotReduceDigivolveCost" => ModifierType::CannotReduceDigivolveCost,
        "OpponentCannotReduceDigivolveCost" => ModifierType::OpponentCannotReduceDigivolveCost,
        "CannotDrawByEffect" => ModifierType::CannotDrawByEffect,
        "CannotAddSecurityByEffect" => ModifierType::CannotAddSecurityByEffect,
        "CannotTrashOpponentSecurity" => ModifierType::CannotTrashOpponentSecurity,
        "CannotReduceOpponentSecurity" => ModifierType::CannotReduceOpponentSecurity,
        // Track C taxonomy completion (2026-05-06).
        "MayAttackPlayerOnly" => ModifierType::MayAttackPlayerOnly,
        "CannotMove" => ModifierType::CannotMove,
        "CannotSwitchAttackTarget" => ModifierType::CannotSwitchAttackTarget,
        "CanAttackTargetDefendingPermanent" => ModifierType::CanAttackTargetDefendingPermanent,
        "CannotAddMemory" => ModifierType::CannotAddMemory,
        "CannotAddSecurity" => ModifierType::CannotAddSecurity,
        "ChangeEndTurnMinMemory" => ModifierType::ChangeEndTurnMinMemory,
        "ImmuneFromDPMinus" => ModifierType::ImmuneFromDPMinus,
        "ImmuneFromStackTrashing" => ModifierType::ImmuneFromStackTrashing,
        "DisableEffect" => ModifierType::DisableEffect,
        "TreatAsDigimon" => ModifierType::TreatAsDigimon,
        "ChangeCardDP" => ModifierType::ChangeCardDP,
        "ChangeOriginDP" => ModifierType::ChangeOriginDP,
        "ChangeSAttack" => ModifierType::ChangeSAttack,
        "ChangeLinkCost" => ModifierType::ChangeLinkCost,
        "ChangeLinkMax" => ModifierType::ChangeLinkMax,
        "ChangePermanentLevel" => ModifierType::ChangePermanentLevel,
        "ChangeTraits" => ModifierType::ChangeTraits,
        "ChangeBaseCardName" => ModifierType::ChangeBaseCardName,
        "ChangeBaseCardColor" => ModifierType::ChangeBaseCardColor,
        "ChangeCardLevelForAssembly" => ModifierType::ChangeCardLevelForAssembly,
        "ChangeCardNamesForDigiXros" => ModifierType::ChangeCardNamesForDigiXros,
        _ => return None,
    })
}

/// Compile-time guard: every `ModifierType` variant must appear in this
/// match. Adding a new variant breaks the exhaustiveness check, prompting
/// the engineer to also add a row to `lookup_modifier_type` AND a string
/// entry in `digimon_dsl::validator::KNOWN_MODIFIER_KEYS`. The runtime
/// parity test `validator_keys_match_engine_table` catches the latter.
#[allow(dead_code)]
const fn _modifier_variant_exhaustiveness_check(m: ModifierType) {
    match m {
        ModifierType::ChangeDp
        | ModifierType::ChangeBaseDp
        | ModifierType::DpFloor
        | ModifierType::DontHaveDp
        | ModifierType::ChangePlayCost
        | ModifierType::ChangeDigivolveCost
        | ModifierType::CannotReduceCost
        | ModifierType::CannotBeDestroyed
        | ModifierType::CannotBeDestroyedByBattle
        | ModifierType::CannotBeDestroyedByEffect
        | ModifierType::CannotBeRemoved
        | ModifierType::CannotBeReturnedToDeck
        | ModifierType::CannotBeReturnedToHand
        | ModifierType::CannotBeTrashedByEffect
        | ModifierType::CannotBeDeDigivolved
        | ModifierType::CannotAttack
        | ModifierType::CannotAttackPlayer
        | ModifierType::VortexCanAttackPlayer
        | ModifierType::CanAttackUnsuspended
        | ModifierType::CanAttackActivePlayer
        | ModifierType::CannotAttackTarget
        | ModifierType::CannotSuspend
        | ModifierType::CannotUnsuspend
        | ModifierType::CannotBeSelectedByEffect
        | ModifierType::CannotBeAffected
        | ModifierType::GrantBlocker
        | ModifierType::GrantRush
        | ModifierType::GrantJamming
        | ModifierType::GrantPiercing
        | ModifierType::GrantReboot
        | ModifierType::GrantBlitz
        | ModifierType::GrantAlliance
        | ModifierType::GrantRaid
        | ModifierType::GrantDecoy
        | ModifierType::GrantVortex
        | ModifierType::GrantOverclock
        | ModifierType::MayAttack
        | ModifierType::ForceAttack
        | ModifierType::SecurityAttackChange
        | ModifierType::ImmunityToOpponentEffects
        | ModifierType::DontBattleSecurityDigimon
        | ModifierType::CannotDigivolve
        | ModifierType::ChangeColor
        | ModifierType::AddColor
        | ModifierType::ChangeLevel
        | ModifierType::CannotReturnToHand
        | ModifierType::CannotTrash
        | ModifierType::CannotBlock
        | ModifierType::CannotCounter
        | ModifierType::DrawBlock
        | ModifierType::MemoryBlock
        | ModifierType::CannotPlayFromHand
        | ModifierType::CannotPlayDigimonByEffect
        | ModifierType::CannotPlayTamerByEffect
        | ModifierType::CannotGainMemoryByEffect
        | ModifierType::CannotGainMemoryExceptFromTamers
        | ModifierType::CannotPlayFromTrash
        | ModifierType::CannotReducePlayCost
        | ModifierType::CannotReduceDigivolveCost
        | ModifierType::OpponentCannotReduceDigivolveCost
        | ModifierType::CannotActivateMainEffects
        | ModifierType::CannotActivateWhenDigivolvingEffects
        | ModifierType::CannotActivateWhenAttackingEffects
        | ModifierType::CannotActivateSecurityEffects
        | ModifierType::CannotDigivolveDigimonByEffect
        | ModifierType::CannotDrawByEffect
        | ModifierType::CannotAddSecurityByEffect
        | ModifierType::CannotTrashOpponentSecurity
        | ModifierType::CannotReduceOpponentSecurity
        | ModifierType::IgnoreColorRequirement
        | ModifierType::MayAttackPlayerOnly
        | ModifierType::CannotMove
        | ModifierType::CannotSwitchAttackTarget
        | ModifierType::CanNotSwitchAttackTarget
        | ModifierType::CannotBeRedirectedAsAttackTarget
        | ModifierType::CanAttackTargetDefendingPermanent
        | ModifierType::CannotAddMemory
        | ModifierType::CannotAddSecurity
        | ModifierType::ChangeEndTurnMinMemory
        | ModifierType::ImmuneFromDPMinus
        | ModifierType::ImmuneFromStackTrashing
        | ModifierType::DisableEffect
        | ModifierType::TreatAsDigimon
        | ModifierType::ChangeCardDP
        | ModifierType::ChangeOriginDP
        | ModifierType::ChangeSAttack
        | ModifierType::ChangeLinkCost
        | ModifierType::ChangeLinkMax
        | ModifierType::ChangePermanentLevel
        | ModifierType::ChangeTraits
        | ModifierType::ChangeBaseCardName
        | ModifierType::ChangeBaseCardColor
        | ModifierType::ChangeCardLevelForAssembly
        | ModifierType::ChangeCardNamesForDigiXros => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Enumerate every `ModifierType` variant — paired with
    /// `_modifier_variant_exhaustiveness_check` to keep the test in sync
    /// with the enum.
    fn all_variants() -> &'static [ModifierType] {
        &[
            ModifierType::ChangeDp,
            ModifierType::ChangeBaseDp,
            ModifierType::DpFloor,
            ModifierType::DontHaveDp,
            ModifierType::ChangePlayCost,
            ModifierType::ChangeDigivolveCost,
            ModifierType::CannotReduceCost,
            ModifierType::CannotBeDestroyed,
            ModifierType::CannotBeDestroyedByBattle,
            ModifierType::CannotBeDestroyedByEffect,
            ModifierType::CannotBeRemoved,
            ModifierType::CannotBeReturnedToDeck,
            ModifierType::CannotBeReturnedToHand,
            ModifierType::CannotBeTrashedByEffect,
            ModifierType::CannotBeDeDigivolved,
            ModifierType::CannotAttack,
            ModifierType::CannotAttackPlayer,
            ModifierType::VortexCanAttackPlayer,
            ModifierType::CanAttackUnsuspended,
            ModifierType::CanAttackActivePlayer,
            ModifierType::CannotAttackTarget,
            ModifierType::CannotSuspend,
            ModifierType::CannotUnsuspend,
            ModifierType::CannotBeSelectedByEffect,
            ModifierType::CannotBeAffected,
            ModifierType::GrantBlocker,
            ModifierType::GrantRush,
            ModifierType::GrantJamming,
            ModifierType::GrantPiercing,
            ModifierType::GrantReboot,
            ModifierType::GrantBlitz,
            ModifierType::GrantAlliance,
            ModifierType::GrantRaid,
            ModifierType::GrantDecoy,
            ModifierType::GrantVortex,
            ModifierType::GrantOverclock,
            ModifierType::MayAttack,
            ModifierType::ForceAttack,
            ModifierType::SecurityAttackChange,
            ModifierType::ImmunityToOpponentEffects,
            ModifierType::DontBattleSecurityDigimon,
            ModifierType::CannotDigivolve,
            ModifierType::ChangeColor,
            ModifierType::AddColor,
            ModifierType::ChangeLevel,
            ModifierType::CannotReturnToHand,
            ModifierType::CannotTrash,
            ModifierType::CannotBlock,
            ModifierType::CannotCounter,
            ModifierType::DrawBlock,
            ModifierType::MemoryBlock,
            ModifierType::CannotPlayFromHand,
            ModifierType::CannotPlayDigimonByEffect,
            ModifierType::CannotPlayTamerByEffect,
            ModifierType::CannotGainMemoryByEffect,
            ModifierType::CannotGainMemoryExceptFromTamers,
            ModifierType::CannotPlayFromTrash,
            ModifierType::CannotReducePlayCost,
            ModifierType::CannotReduceDigivolveCost,
            ModifierType::OpponentCannotReduceDigivolveCost,
            ModifierType::CannotActivateMainEffects,
            ModifierType::CannotActivateWhenDigivolvingEffects,
            ModifierType::CannotActivateWhenAttackingEffects,
            ModifierType::CannotActivateSecurityEffects,
            ModifierType::CannotDigivolveDigimonByEffect,
            ModifierType::CannotDrawByEffect,
            ModifierType::CannotAddSecurityByEffect,
            ModifierType::CannotTrashOpponentSecurity,
            ModifierType::CannotReduceOpponentSecurity,
            ModifierType::IgnoreColorRequirement,
            ModifierType::MayAttackPlayerOnly,
            ModifierType::CannotMove,
            ModifierType::CannotSwitchAttackTarget,
            ModifierType::CanNotSwitchAttackTarget,
            ModifierType::CannotBeRedirectedAsAttackTarget,
            ModifierType::CanAttackTargetDefendingPermanent,
            ModifierType::CannotAddMemory,
            ModifierType::CannotAddSecurity,
            ModifierType::ChangeEndTurnMinMemory,
            ModifierType::ImmuneFromDPMinus,
            ModifierType::ImmuneFromStackTrashing,
            ModifierType::DisableEffect,
            ModifierType::TreatAsDigimon,
            ModifierType::ChangeCardDP,
            ModifierType::ChangeOriginDP,
            ModifierType::ChangeSAttack,
            ModifierType::ChangeLinkCost,
            ModifierType::ChangeLinkMax,
            ModifierType::ChangePermanentLevel,
            ModifierType::ChangeTraits,
            ModifierType::ChangeBaseCardName,
            ModifierType::ChangeBaseCardColor,
            ModifierType::ChangeCardLevelForAssembly,
            ModifierType::ChangeCardNamesForDigiXros,
        ]
    }

    /// Every variant must round-trip: the engine's PascalCase string ↔
    /// `ModifierType` enum mapping must cover every variant.
    #[test]
    fn every_variant_round_trips() {
        for &v in all_variants() {
            let name = format!("{v:?}");
            assert_eq!(
                lookup_modifier_type(&name),
                Some(v),
                "ModifierType::{v:?} did not round-trip via lookup_modifier_type"
            );
        }
    }

    /// Bidirectional parity with `digimon_dsl::validator::KNOWN_MODIFIER_KEYS`.
    /// Drift here means either:
    ///  - validator wrongly rejects YAML the engine recognizes, or
    ///  - validator green-lights YAML the engine silently no-ops.
    #[test]
    fn validator_keys_match_engine_table() {
        let validator: BTreeSet<&str> = digimon_dsl::validator::KNOWN_MODIFIER_KEYS
            .iter()
            .copied()
            .collect();
        let engine: BTreeSet<String> = all_variants().iter().map(|v| format!("{v:?}")).collect();
        let engine_strs: BTreeSet<&str> = engine.iter().map(String::as_str).collect();

        let only_validator: Vec<_> = validator.difference(&engine_strs).collect();
        let only_engine: Vec<_> = engine_strs.difference(&validator).collect();

        assert!(
            only_validator.is_empty() && only_engine.is_empty(),
            "modifier-key drift between digimon-dsl validator and digimon-engine modifier_map. \
             Only in validator (would be silently dropped at runtime): {only_validator:?}. \
             Only in engine (validator would reject valid YAML): {only_engine:?}."
        );
    }
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
        "DigiBurst" => Keyword::DigiBurst(value.unwrap_or(1) as u8),
        "Fortitude" => Keyword::Fortitude,
        "Overclock" => Keyword::Overclock,
        "Barrier" => Keyword::Barrier,
        // DSL `keyword: Decoy` with optional `value:` carrying the color
        // bitmask (default 0 = no filter — matches all ally Digimon).
        // Bit layout matches `CardColor as u8`: Red=0..Purple=6. Cards
        // wanting a single-color filter pass `value: 32` (bit 5 = Black),
        // multi-color use OR'd values (e.g. `value: 33` for Red|Black).
        "Decoy" => Keyword::Decoy(value.unwrap_or(0) as u8),
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
