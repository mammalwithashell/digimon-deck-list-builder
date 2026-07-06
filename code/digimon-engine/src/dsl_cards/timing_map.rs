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
        CompiledTiming::OnBlock => EffectTiming::OnBlock,
        CompiledTiming::OnAllyAttack => EffectTiming::OnAllyAttack,
        CompiledTiming::OnOpponentAttack => EffectTiming::OnOpponentAttack,
        CompiledTiming::OnDeletion => EffectTiming::OnDeletion,
        CompiledTiming::OnAnyDeletion => EffectTiming::OnAnyDeletion,
        // Board-wide battle-winner observer rides the `EndOfBattle` dispatch
        // (fired via `TriggerSource::BattleResolved`, carrying the winner). No
        // forced self-filter — scope is gated by `active_when:` (`event_winner_*`).
        // G-DSL-BATTLE-WINNER-BOARDWIDE.
        CompiledTiming::OnAllyWonBattle => EffectTiming::EndOfBattle,
        // Hand-discard observer maps 1:1 to the new engine timing.
        // G-ENGINE-ON-DISCARD-HAND.
        CompiledTiming::OnDiscardHand => EffectTiming::OnDiscardHand,
        CompiledTiming::OnEnterFieldAnyone => EffectTiming::OnEnterFieldAnyone,
        CompiledTiming::OnAnyDigimonPlayed => EffectTiming::OnEnterFieldAnyone,
        CompiledTiming::OnAllyPlayed => EffectTiming::OnAllyPlayed,
        CompiledTiming::OnLeaveField => EffectTiming::OnLeaveField,
        CompiledTiming::OnSuspend => EffectTiming::OnSuspend,
        CompiledTiming::OnUnsuspend => EffectTiming::OnUnsuspend,
        CompiledTiming::OnAddToHand => EffectTiming::OnAddToHand,
        CompiledTiming::OnHatch => EffectTiming::OnHatch,
        CompiledTiming::OnMove => EffectTiming::OnMove,
        CompiledTiming::OnDigivolve => EffectTiming::OnDigivolve,
        CompiledTiming::OnDnaDigivolve => EffectTiming::OnDnaDigivolve,
        CompiledTiming::OnDigixros => EffectTiming::OnDigiXros,
        CompiledTiming::OnOpponentSecurityRemoved => EffectTiming::OnOpponentSecurityRemoved,
        CompiledTiming::OnOwnSecurityRemoved => EffectTiming::OnOwnSecurityRemoved,
        CompiledTiming::OnDigivolutionCardTrashed => EffectTiming::OnDigivolutionCardTrashed,
        CompiledTiming::OnDigivolutionCardReturnedToDeckBottom => {
            EffectTiming::OnDigivolutionCardReturnedToDeckBottom
        }
        CompiledTiming::OnSecurityCheck => EffectTiming::OnSecurityCheck,
        CompiledTiming::OnCheckFaceUpSecurity => EffectTiming::OnCheckFaceUpSecurity,
        CompiledTiming::OnLoseSecurity => EffectTiming::OnLoseSecurity,
        CompiledTiming::OnDiscardSecurity => EffectTiming::OnDiscardSecurity,
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
        CompiledTiming::BeforePayCostObserve => EffectTiming::BeforePayCostObserve,
        CompiledTiming::OnOptionPlaced => EffectTiming::OnOptionPlaced,
        CompiledTiming::OnOptionTrashed => EffectTiming::OnOptionTrashed,
        CompiledTiming::OnPlaceSecurity => EffectTiming::OnPlaceSecurity,
        CompiledTiming::OnAddedToSecurity => EffectTiming::OnPlaceSecurity,
        CompiledTiming::Main => EffectTiming::OptionMain,
        // DigiLink Shape-B: `when: when_linked` rides the `OnLink` dispatch;
        // lower_triggered forces `.linked()` and a self-filter so it fires
        // only for the just-linked card (design D6).
        CompiledTiming::WhenLinked => EffectTiming::OnLink,
        // DigiLink host-side: `when: when_card_linked_to_this` also rides the
        // `OnLink` dispatch; lower_triggered forces a host self-filter
        // (`event_permanent == source_permanent`) instead of `.linked()`.
        CompiledTiming::WhenCardLinkedToThis => EffectTiming::OnLink,
        // DigiLink host-side pre-link replacement: `when: when_would_link_to_this`
        // lowers to the `WhenWouldLink` REPLACEMENT timing; lower_triggered
        // forces a host self-filter (`pending_link_host() == source_permanent`)
        // and routes the body through `replacement_process` (Gap 5).
        CompiledTiming::WhenWouldLinkToThis => EffectTiming::WhenWouldLink,
        // DigiLink board-wide observer: `when: on_any_link` lowers to `OnLink`
        // with NO forced self/host filter — scope is gated entirely by
        // `active_when:` predicates (G-DSL-WHEN-ANY-OWN-DIGIMON-LINKED).
        CompiledTiming::OnAnyLink => EffectTiming::OnLink,
        // Phase 2a non-targets — skip emission.
        CompiledTiming::Delayed => return None,
    })
}
