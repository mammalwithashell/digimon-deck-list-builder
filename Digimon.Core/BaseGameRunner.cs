using System;
using System.Collections.Generic;
using Digimon.Core.Constants;

namespace Digimon.Core
{
    public abstract class BaseGameRunner
    {
        public Game GameInstance { get; protected set; }

        public BaseGameRunner(List<string> deck1Ids, List<string> deck2Ids)
        {
            GameInstance = new Game();
            var deck1 = CreateDeck(deck1Ids);
            var deck2 = CreateDeck(deck2Ids);
            GameInstance.StartGame(deck1, deck2);
        }

        protected static List<Card> CreateDeck(List<string> ids)
        {
            var deck = new List<Card>();
             foreach (var id in ids)
            {
                // Look up card data from registry
                deck.Add(CardRegistry.CreateCard(id));
            }
            // Ensure minimum deck size for simulation if empty
            if (deck.Count == 0)
            {
                // Add 5 DigiEggs
                for(int i=0; i<5; i++)
                    deck.Add(CardRegistry.CreateCard("ST1-01")); // Koromon

                // Add 45 Digimon
                for(int i=0; i<45; i++)
                     deck.Add(CardRegistry.CreateCard("ST1-03")); // Agumon
            }
            return deck;
        }

        protected void ExecuteAgentTurn()
        {
             // Simulate Agent Action: Pass Turn
             // Real logic: Get Observation -> Model -> Action
             
             // 1. Get State Tensor
             float[] stateTensor = GameInstance.GetBoardStateTensor(GameInstance.CurrentPlayer.Id);
             
             // 2. (Stub) Pass to Model to get Action
             // int action = AgentModel.Predict(stateTensor); 
             
             // 3. Execute Action (Stub based on phase)
            
            if (GameInstance.TurnStateMachine.CurrentPhase == GamePhase.Breeding)
            {
                // Agent Decision: For now, always Pass (Skip) Breeding
                GameInstance.BreedingPass();
            }
            
            if (GameInstance.TurnStateMachine.CurrentPhase == GamePhase.Main)
            {
                GameInstance.TurnStateMachine.PassTurn();
            }
        }
    }
}
