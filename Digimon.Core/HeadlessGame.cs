using System;
using System.Collections.Generic;

namespace Digimon.Core
{
    public class HeadlessGame
    {
        public Game GameInstance { get; private set; }

        public HeadlessGame(List<string> deck1Ids, List<string> deck2Ids)
        {
            GameInstance = new Game();
            var deck1 = CreateDeck(deck1Ids);
            var deck2 = CreateDeck(deck2Ids);
            GameInstance.StartGame(deck1, deck2);
        }

        private static List<Card> CreateDeck(List<string> ids)
        {
            var deck = new List<Card>();
            foreach (var id in ids)
            {
                // In a real scenario, look up card by ID in CardDatabase
                // Here we create a dummy card
                deck.Add(new Card(id, $"Card {id}", CardKind.Digimon, CardColor.Red, 3, 2000, 3));
            }
            // Ensure minimum deck size for simulation if empty
            if (deck.Count == 0)
            {
                for(int i=0; i<50; i++)
                    deck.Add(new Card($"dummy_{i}", "Dummy Card", CardKind.Digimon, CardColor.Red, 3, 2000, 3));
            }
            return deck;
        }

        public int RunUntilConclusion()
        {
            int maxTurns = 100;
            int currentTurn = 0;

            while (!GameInstance.IsGameOver && currentTurn < maxTurns)
            {
                currentTurn++;
                // Simulate Main Phase Action: Pass Turn
                // In a real game, agents would act here.
                // For this headless test, we just pass the turn.

                // Check win condition (Deck out)
                if (GameInstance.CurrentPlayer!.Deck.Count == 0)
                {
                    GameInstance.EndGame(GameInstance.OpponentPlayer);
                    break;
                }

                // Simulate Agent Action: Pass Turn
                GameInstance.TurnStateMachine.PassTurn();
            }

            if (!GameInstance.IsGameOver)
            {
                // Force end
                GameInstance.EndGame(GameInstance.Player1); // Draw/Tie breaker
            }

            return GameInstance.Winner?.Id ?? 0;
        }

        public void Step(int actionId)
        {
            // Stub for human interaction
            // In reality, this would decode the actionId and apply it to GameInstance
            Console.WriteLine($"[HeadlessGame] Execute Action: {actionId}");

            // For now, just advance phase to simulate progress (Pass Turn)
            GameInstance.TurnStateMachine.PassTurn();
        }
    }
}
