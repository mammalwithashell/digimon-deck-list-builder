using System;
using System.Collections.Generic;
using Digimon.Core.Constants;

namespace Digimon.Core
{
    public class InteractiveGame : BaseGameRunner
    {
        public PlayerType Player1Type { get; private set; }
        public PlayerType Player2Type { get; private set; }

        public InteractiveGame(List<string> deck1Ids, List<string> deck2Ids, PlayerType player1Type, PlayerType player2Type)
            : base(deck1Ids, deck2Ids)
        {
            Player1Type = player1Type;
            Player2Type = player2Type;
        }

        public string RunStep()
        {
            if (GameInstance.IsGameOver) return GameInstance.ToJson();

             // Check win condition (Deck out)
            if (GameInstance.CurrentPlayer.Deck.Count == 0)
            {
                GameInstance.EndGame(GameInstance.OpponentPlayer);
                return GameInstance.ToJson();
            }

            if (IsCurrentPlayerHuman())
            {
                // Do nothing, wait for external Step(actionId)
                // Return current state so UI can render
                return GameInstance.ToJson();
            }
            else
            {
                ExecuteAgentTurn();
                return GameInstance.ToJson();
            }
        }

        private bool IsCurrentPlayerHuman()
        {
            if (GameInstance.CurrentPlayer == GameInstance.Player1)
                return Player1Type == PlayerType.Human;
            else
                return Player2Type == PlayerType.Human;
        }

        public void Step(int actionId)
        {
            // Stub for human interaction
            // In reality, this would decode the actionId and apply it to GameInstance
            // Console.WriteLine($"[InteractiveGame] Human Execute Action: {actionId}");
            
            // Intelligence: Handle Phase
            if (GameInstance.TurnStateMachine.CurrentPhase == GamePhase.Breeding)
            {
                // For this stub, assume actionId 0 means "Proceed" or "Skip"
                GameInstance.BreedingPass(); 
                
                if (GameInstance.TurnStateMachine.CurrentPhase == GamePhase.Main)
                {
                    // Check if we should auto-pass main, but usually human wants to do things.
                    // For now, let's keep the existing stub logic:
                    GameInstance.TurnStateMachine.PassTurn();
                }
            }
            else if (GameInstance.TurnStateMachine.CurrentPhase == GamePhase.Main)
            {
                GameInstance.TurnStateMachine.PassTurn();
            }
        }
    }
}
