using System;

namespace Digimon.Core
{
    public enum GamePhase
    {
        Start,
        Draw,
        Breeding,
        Main,
        End
    }

    public class TurnStateMachine(Game game)
    {
        public GamePhase CurrentPhase { get; private set; }
        public int TurnCount { get; private set; }

        public void StartTurn()
        {
            TurnCount++;
            CurrentPhase = GamePhase.Start;
            // Auto advance through start phases for headless
            UnsuspendPhase();
            DrawPhase();
            BreedingPhase();
            MainPhase();
        }

        private void UnsuspendPhase()
        {
            // Unsuspend all permanents logic here
            // _game.CurrentPlayer.UnsuspendAll();
        }

        private void DrawPhase()
        {
            CurrentPhase = GamePhase.Draw;
            // Draw 1 unless first player first turn (Digimon rules: First player draws? No, usually not on turn 1 in some games, but Digimon TCG: First player does NOT draw on first turn).
            // Logic:
            if (TurnCount == 1 && game.CurrentPlayer!.Id == 1)
            {
                // Skip draw
            }
            else
            {
                game.CurrentPlayer!.Draw();
            }
        }

        private void BreedingPhase()
        {
            CurrentPhase = GamePhase.Breeding;
            // In headless, maybe skip or random hatch?
            // For now, pass.
        }

        private void MainPhase()
        {
            CurrentPhase = GamePhase.Main;
            // The Main Phase continues until the player passes or memory crosses 0.
            // Logic handled by Game loop / Agent inputs.
        }

        // Called by Game Loop when an action finishes
        public void CheckTurnEnd()
        {
            if (CurrentPhase != GamePhase.Main) return;

            // Check memory condition
            int currentMemory = game.GetMemory(game.CurrentPlayer!);
            if (currentMemory < 0)
            {
                EndTurn();
            }
        }

        public void PassTurn()
        {
            // Passing turn means giving opponent 3 memory.
            // Move memory to opponent's side (3).
            // Current Player is P1: MemoryGauge -> -3
            // Current Player is P2: MemoryGauge -> 3

            // Only if we are currently positive or 0?
            // Rule: You can pass any time. If you have >0 memory, opponent starts at 3.
            // Wait, standard rule: Pass Turn sets memory to 3 on opponent side.
            // Specifically: It consumes all your memory + gives opponent 3.
            // Actually, the rule is "Set memory gauge to 3 on opponent's side."

            if (game.CurrentPlayer == game.Player1)
                game.PayCost(game.CurrentPlayer!, game.GetMemory(game.CurrentPlayer!) + 3); // Naive math
            else
                game.PayCost(game.CurrentPlayer!, game.GetMemory(game.CurrentPlayer!) + 3);

            // Easier: Just set it directly.
             if (game.CurrentPlayer == game.Player1)
             {
                 // Set to -3
                 // _game.MemoryGauge = -3; // Can't set private.
                 // Use a dedicated method or PayCost until -3.
                 int cost = game.MemoryGauge - (-3);
                 game.PayCost(game.CurrentPlayer!, cost);
             }
             else
             {
                 // Set to 3
                 // _game.MemoryGauge = 3;
                 // Current is negative (e.g. -2). Target is 3.
                 // Pay cost? P2 paying cost ADDS to gauge.
                 // Cost = Target - Current = 3 - (-2) = 5.
                 int cost = 3 - game.MemoryGauge;
                 game.PayCost(game.CurrentPlayer!, cost);
             }

             EndTurn();
        }

        private void EndTurn()
        {
            CurrentPhase = GamePhase.End;
            // Resolve end of turn effects
            game.SwitchTurn();
        }
    }
}
