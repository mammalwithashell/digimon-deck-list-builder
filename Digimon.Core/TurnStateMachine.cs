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

    public class TurnStateMachine
    {
        private Game _game;
        public GamePhase CurrentPhase { get; private set; }
        public int TurnCount { get; private set; }

        public TurnStateMachine(Game game)
        {
            _game = game;
        }

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
            if (TurnCount == 1 && _game.CurrentPlayer.Id == 1)
            {
                // Skip draw
            }
            else
            {
                _game.CurrentPlayer.Draw();
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
            // This is where the agent acts.
            // In a fully automated loop, we might loop here until memory passes 0.
        }

        public void EndTurn()
        {
            CurrentPhase = GamePhase.End;
            // Resolve end of turn effects
            _game.SwitchTurn();
        }
    }
}
