using System;
using System.Collections.Generic;
using Digimon.Core.Constants;

namespace Digimon.Core
{
    public class HeadlessGame : BaseGameRunner
    {
        public HeadlessGame(List<string> deck1Ids, List<string> deck2Ids) 
            : base(deck1Ids, deck2Ids)
        {
        }

        public int RunUntilConclusion(int maxTurns = 200)
        {
            int currentTurn = 0;

            while (!GameInstance.IsGameOver && currentTurn < maxTurns)
            {
                currentTurn++;
                ExecuteAgentTurn();
                
                if (GameInstance.IsGameOver) break;
            }

            if (!GameInstance.IsGameOver && currentTurn >= maxTurns)
            {
                // Force end
                GameInstance.EndGame(GameInstance.Player1); // Draw/Tie breaker
            }

            return GameInstance.Winner?.Id ?? 0;
        }

        // For training loop where Python drives step-by-step
        public void RunAgentStep()
        {
             if (!GameInstance.IsGameOver)
             {
                 ExecuteAgentTurn();
             }
        }

        public void Step(int actionId)
        {
             // Use ActionDecoder to handle logic based on current state
             ActionDecoder.DecodeAndExecute(GameInstance, actionId);
        }
    }
}
