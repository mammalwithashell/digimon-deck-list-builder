use crate::enums::GamePhase;

impl GamePhase {
    /// Whether this phase is a selection/interrupt phase (requires player input to resolve).
    pub fn is_selection_phase(&self) -> bool {
        matches!(
            self,
            GamePhase::SelectTarget
                | GamePhase::SelectMaterial
                | GamePhase::SelectTrash
                | GamePhase::SelectSource
                | GamePhase::SelectHand
                | GamePhase::SelectReveal
                | GamePhase::SelectSecurity
                | GamePhase::EffectChoice
                // Phase 4 kinds (Tasks 2-5 wire full dispatch)
                | GamePhase::SelectUnion
                | GamePhase::SelectPermutation
                | GamePhase::SelectBudgeted
                | GamePhase::SelectBreedingPermanent
        )
    }

    /// Whether this phase is a combat sub-phase.
    pub fn is_combat_phase(&self) -> bool {
        matches!(
            self,
            GamePhase::BlockTiming | GamePhase::CounterTiming | GamePhase::AllianceTiming
        )
    }

    /// Whether the game is still in progress.
    pub fn is_active(&self) -> bool {
        !matches!(self, GamePhase::GameOver)
    }

    /// Numeric value for tensor encoding.
    ///
    /// These values intentionally mirror the legacy Python `GamePhase` enum
    /// because RL observation tensors are a cross-backend contract.
    pub fn tensor_value(&self) -> f32 {
        match self {
            GamePhase::Mulligan => 17.0,
            GamePhase::Unsuspend => 0.0,
            GamePhase::Draw => 1.0,
            GamePhase::Breeding => 2.0,
            GamePhase::Main => 3.0,
            GamePhase::EndTurn => 4.0,
            GamePhase::SelectTarget => 5.0,
            GamePhase::SelectMaterial => 6.0,
            GamePhase::SelectTrash => 9.0,
            GamePhase::SelectSource => 10.0,
            GamePhase::SelectHand => 11.0,
            GamePhase::SelectReveal => 12.0,
            GamePhase::SelectSecurity => 14.0,
            GamePhase::EffectChoice => 13.0,
            GamePhase::BlockTiming => 7.0,
            GamePhase::CounterTiming => 8.0,
            GamePhase::AllianceTiming => 16.0,
            GamePhase::EndOfTurnAction => 15.0,
            GamePhase::GameOver => 4.0,
            // Phase 4 variants — tensor encoding TBD (Tasks 2-5); reuse
            // the nearest existing selection bucket as a placeholder so
            // training tensors stay in-range.
            GamePhase::SelectUnion => 6.0,
            GamePhase::SelectPermutation => 6.0,
            GamePhase::SelectBudgeted => 6.0,
            GamePhase::SelectBreedingPermanent => 6.0,
        }
    }
}
