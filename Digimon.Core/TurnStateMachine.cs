using System;

using Digimon.Core.Constants;

namespace Digimon.Core
{
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
            CurrentPhase = GamePhase.Start;
            // Only increment global turn count on Player 1? Or both?
            // Usually "Turn" is "Player 1's Turn", then "Player 2's Turn".
            // TurnCount typically counts Total Turns or Pairs. 
            // For now, simple increment.
            TurnCount++; 

            UnsuspendPhase();
            DrawPhase();
            BreedingPhase();
            // MainPhase(); // Removed: Handled by Breeding Actions or Pass
        }

        private void UnsuspendPhase()
        {
            _game.CurrentPlayer.UnsuspendAll();
        }

        private void DrawPhase()
        {
            CurrentPhase = GamePhase.Draw;

            // Player 1, Turn 1: No Draw
            // Note: If using a "First Turn" flag or checking Player ID + Game Turn.
            // Assuming Game.TurnCount starts at 0 or 1.
            // If TurnCount is incremented at start of every turn, then TurnCount == 1 is Player 1 Turn 1.
            
            if (TurnCount == 1 && _game.CurrentPlayer.Id == 1)
            {
                // Skip draw 
                return;
            }

            if (_game.CurrentPlayer.Deck.Count == 0)
            {
                _game.EndGame(_game.OpponentPlayer);
                return;
            }

            _game.CurrentPlayer.Draw();
        }

        private void BreedingPhase()
        {
            CurrentPhase = GamePhase.Breeding;
            // Waits for Agent Action: Hatch, Move, or Pass
        }

        public void OnBreedingActionCompleted()
        {
            // Transition to Main Phase after Hatch or Move
            MainPhase();
        }

        public void SkipBreedingPhase()
        {
            // Transition to Main Phase if Agent chooses to do nothing
            MainPhase();
        }

        private void MainPhase()
        {
            CurrentPhase = GamePhase.Main;
        }

        // Called by Game Loop when an action finishes
        public void CheckTurnEnd()
        {
            if (CurrentPhase != GamePhase.Main) return;

            // Turn ends if Memory is on opponent's side (< 0 relative to current player)
            // AND (implied) no pending effects.
            int mem = _game.GetMemory(_game.CurrentPlayer);
            if (mem < 0)
            {
                EndTurn();
            }
        }

        public void PassTurn()
        {
            // Set memory gauge to 3 on opponent's side.
            // Opponent side 3 means Memory for Current Player is -3.
            
            int targetMemory = -3;
            int currentMemory = _game.GetMemory(_game.CurrentPlayer);

            if (currentMemory > targetMemory)
            {
                int cost = currentMemory - targetMemory;
                _game.PayCost(_game.CurrentPlayer, cost);
            }
            
            // CheckTurnEnd will handle the transition
            CheckTurnEnd();
        }

        private void EndTurn()
        {
            CurrentPhase = GamePhase.End;
            
            // 1. Resolve [End of Turn] effects
            ResolveEndOfTurnEffects();

            // 2. Check Memory again. 
            // If effects changed memory back to >= 0, turn resumes.
            int mem = _game.GetMemory(_game.CurrentPlayer);
            if (mem >= 0)
            {
                // Resume Main Phase
                CurrentPhase = GamePhase.Main;
                return;
            }

            // 3. Switch Turn
            _game.SwitchTurn();
        }

        private void ResolveEndOfTurnEffects()
        {
            // TODO: Iterate over all permanents and active effects to find "End of Turn" triggers.
            // For now, this is a stub.
        }
    }
}
