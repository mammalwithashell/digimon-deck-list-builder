//! Map `digimon_dsl::compiled::CompiledTiming` → engine `EffectTiming`.
//! Returns None for DSL-only virtual timings that don't map to a single
//! engine timing (e.g. `Delayed`) — callers skip emission.

use digimon_dsl::compiled::CompiledTiming;

use crate::enums::EffectTiming;

pub fn compiled_timing_to_engine(t: CompiledTiming) -> Option<EffectTiming> {
    Some(match t {
        CompiledTiming::OnPlay => EffectTiming::OnPlay,
        CompiledTiming::WhenDigivolving => EffectTiming::WhenDigivolving,
        CompiledTiming::WhenAttacking => EffectTiming::WhenAttacking,
        CompiledTiming::EndOfAttack => EffectTiming::EndOfAttack,
        CompiledTiming::EndOfBattle => EffectTiming::EndOfBattle,
        CompiledTiming::OnAttack => EffectTiming::OnAttack,
        CompiledTiming::OnAllyAttack => EffectTiming::OnAllyAttack,
        CompiledTiming::OnOpponentAttack => EffectTiming::OnOpponentAttack,
        CompiledTiming::OnDeletion => EffectTiming::OnDeletion,
        CompiledTiming::OnAnyDeletion => EffectTiming::OnAnyDeletion,
        CompiledTiming::OnEnterFieldAnyone => EffectTiming::OnEnterFieldAnyone,
        CompiledTiming::OnAllyPlayed => EffectTiming::OnAllyPlayed,
        CompiledTiming::OnLeaveField => EffectTiming::OnLeaveField,
        CompiledTiming::OnSuspend => EffectTiming::OnSuspend,
        CompiledTiming::OnUnsuspend => EffectTiming::OnUnsuspend,
        CompiledTiming::OnHatch => EffectTiming::OnHatch,
        CompiledTiming::OnMove => EffectTiming::OnMove,
        CompiledTiming::OnDigivolve => EffectTiming::OnDigivolve,
        CompiledTiming::OnDnaDigivolve => EffectTiming::OnDnaDigivolve,
        CompiledTiming::OnDigixros => EffectTiming::OnDigiXros,
        CompiledTiming::OnOpponentSecurityRemoved => EffectTiming::OnOpponentSecurityRemoved,
        CompiledTiming::OnOwnSecurityRemoved => EffectTiming::OnOwnSecurityRemoved,
        CompiledTiming::OnDigivolutionCardTrashed => EffectTiming::OnDigivolutionCardTrashed,
        CompiledTiming::OnSecurityCheck => EffectTiming::OnSecurityCheck,
        CompiledTiming::OnLoseSecurity => EffectTiming::OnLoseSecurity,
        CompiledTiming::OnSecurity => EffectTiming::SecuritySkill,
        CompiledTiming::StartOfYourTurn => EffectTiming::StartOfYourTurn,
        CompiledTiming::StartOfOpponentsTurn => EffectTiming::StartOfOpponentsTurn,
        CompiledTiming::StartOfYourMainPhase => EffectTiming::StartOfYourMainPhase,
        CompiledTiming::EndOfYourTurn => EffectTiming::EndOfYourTurn,
        CompiledTiming::EndOfOpponentsTurn => EffectTiming::EndOfOpponentsTurn,
        CompiledTiming::EndOfYourNextTurn => EffectTiming::EndOfYourNextTurn,
        CompiledTiming::EndOfOpponentsNextTurn => EffectTiming::EndOfOpponentsNextTurn,
        CompiledTiming::UntilNextUnsuspend => EffectTiming::UntilNextUnsuspend,
        CompiledTiming::OnAttackTargetChange => EffectTiming::OnAttackTargetChange,
        CompiledTiming::MainFromHand => EffectTiming::MainFromHand,
        CompiledTiming::MainOnField => EffectTiming::MainOnField,
        CompiledTiming::MainFromTrash => EffectTiming::MainFromTrash,
        CompiledTiming::Counter => EffectTiming::CounterEffect,
        CompiledTiming::BeforePayCost => EffectTiming::BeforePayCost,
        CompiledTiming::OnOptionPlaced => EffectTiming::OnOptionPlaced,
        CompiledTiming::Main => EffectTiming::OptionMain,
        // Phase 2a non-targets — skip emission.
        CompiledTiming::Delayed => return None,
    })
}
