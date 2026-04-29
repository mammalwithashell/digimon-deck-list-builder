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
    pub fn tensor_value(&self) -> f32 {
        match self {
            GamePhase::Mulligan => 0.0,
            GamePhase::Unsuspend => 1.0,
            GamePhase::Draw => 2.0,
            GamePhase::Breeding => 3.0,
            GamePhase::Main => 4.0,
            GamePhase::EndTurn => 5.0,
            GamePhase::SelectTarget => 6.0,
            GamePhase::SelectMaterial => 7.0,
            GamePhase::SelectTrash => 8.0,
            GamePhase::SelectSource => 9.0,
            GamePhase::SelectHand => 10.0,
            GamePhase::SelectReveal => 11.0,
            GamePhase::SelectSecurity => 12.0,
            GamePhase::EffectChoice => 13.0,
            GamePhase::BlockTiming => 14.0,
            GamePhase::CounterTiming => 15.0,
            GamePhase::AllianceTiming => 16.0,
            GamePhase::EndOfTurnAction => 17.0,
            GamePhase::GameOver => 18.0,
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
