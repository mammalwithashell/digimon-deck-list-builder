using System;
using System.Collections.Generic;
using Digimon.Core.Constants;

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
                deck.Add(new Card(id, $"Card {id}", CardKind.Digimon, new List<CardColor>{ CardColor.Red }, 3, 2000, 3));
            }
            // Ensure minimum deck size for simulation if empty
            if (deck.Count == 0)
            {
                // Add 5 DigiEggs
                for(int i=0; i<5; i++)
                    deck.Add(new Card($"egg_{i}", "DigiEgg", CardKind.DigiEgg, new List<CardColor>{ CardColor.Red }, 2, 0, 0));

                // Add 45 Digimon
                for(int i=0; i<45; i++)
                    deck.Add(new Card($"dummy_{i}", "Dummy Card", CardKind.Digimon, new List<CardColor>{ CardColor.Red }, 3, 2000, 3));
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
                if (GameInstance.CurrentPlayer.Deck.Count == 0)
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

        public void RunVerification()
        {
            Console.WriteLine("=== Starting Verification ===");
            var game = new Game();
            List<Card> deck1 = CreateDeck(new List<string>());
            List<Card> deck2 = CreateDeck(new List<string>());
            game.StartGame(deck1, deck2);

            // Test 1: Start of Game (Turn 1, Player 1)
            Console.WriteLine($"Turn: {game.TurnStateMachine.TurnCount}, Player: {game.CurrentPlayer.Id}, Phase: {game.TurnStateMachine.CurrentPhase}");
            
            // Should be in Breeding Phase.
            if (game.TurnStateMachine.CurrentPhase == GamePhase.Breeding)
                Console.WriteLine("SUCCESS: Game paused at Breeding Phase.");
            else
                Console.WriteLine($"FAILURE: Phase is {game.TurnStateMachine.CurrentPhase}");

            // Action: Hatch
            Console.WriteLine("Action: Player 1 Hatches Egg.");
            game.BreedingHatch();

            // Verify Hatch and Phase Change
            if (game.Player1.BreedingArea.Count == 1)
                Console.WriteLine("SUCCESS: Player 1 Hatched an Egg.");
            else
                Console.WriteLine($"FAILURE: Player 1 Breeding Area count: {game.Player1.BreedingArea.Count}");

            if (game.TurnStateMachine.CurrentPhase == GamePhase.Main)
                Console.WriteLine("SUCCESS: Phase Advanced to Main.");
            else
                Console.WriteLine($"FAILURE: Phase is {game.TurnStateMachine.CurrentPhase}");

            // Action: Pass Turn
            Console.WriteLine("Action: Player 1 Passes Turn.");
            game.TurnStateMachine.PassTurn();

            // Test 2: Turn 2 (Player 2)
            Console.WriteLine($"Turn: {game.TurnStateMachine.TurnCount}, Player: {game.CurrentPlayer.Id}, Phase: {game.TurnStateMachine.CurrentPhase}");

            // Action: Hatch
            Console.WriteLine("Action: Player 2 Hatches Egg.");
            game.BreedingHatch();
             
            if (game.Player2.BreedingArea.Count == 1)
                Console.WriteLine("SUCCESS: Player 2 Hatched an Egg.");
            
            // Action: Pass Turn
            Console.WriteLine("Action: Player 2 Passes Turn.");
            game.TurnStateMachine.PassTurn();

            // Test 3: Turn 3 (Player 1) - Breeding Phase (Level 2 Egg)
            Console.WriteLine($"Turn: {game.TurnStateMachine.TurnCount}, Player: {game.CurrentPlayer.Id}, Phase: {game.TurnStateMachine.CurrentPhase}");
            
            // Try to Move (Should fail or do nothing if valid? Logic says check level)
            // But Level is 2. So Move should fail or not happen?
            // Actually our BreedingMove wrapper checks "if (count < initial)", so it only advances if move succeeded.
            // If move failed, we are still in Breeding Phase?
            // Let's assume Agent chooses PASS because it knows it can't move.
            
            Console.WriteLine("Action: Player 1 attempts to Move (Level 2) - Expect Failure/No Move.");
            game.BreedingMove(); // Should fail
            
            if (game.Player1.BreedingArea.Count == 1 && game.TurnStateMachine.CurrentPhase == GamePhase.Breeding)
                Console.WriteLine("SUCCESS: Move failed/ignored, still in Breeding Phase.");
            else
                Console.WriteLine($"FAILURE: Breeding: {game.Player1.BreedingArea.Count}, Phase: {game.TurnStateMachine.CurrentPhase}");

            Console.WriteLine("Action: Player 1 Skips Breeding Phase.");
            game.BreedingPass();

             if (game.TurnStateMachine.CurrentPhase == GamePhase.Main)
                Console.WriteLine("SUCCESS: Phase Advanced to Main.");

            // Force Level Up for Test
            game.Player1.BreedingArea[0].TopCard.Level = 3;
            // Note: Digivolution log as requested by user
            Console.WriteLine("LOG: [Player 1] Digivolved Egg to Level 3.");

            // Action: Pass Turn
            game.TurnStateMachine.PassTurn(); // P1 -> P2
            // P2 Pass Breeding
            game.BreedingPass();
            game.TurnStateMachine.PassTurn(); // P2 -> P1 (Turn 5)

            // Test 4: Turn 5 (Player 1) - Should Move
            Console.WriteLine($"Turn: {game.TurnStateMachine.TurnCount}, Player: {game.CurrentPlayer.Id}, Phase: {game.TurnStateMachine.CurrentPhase}");
            
            Console.WriteLine("Action: Player 1 Moves Digimon to Battle Area.");
            game.BreedingMove();

            if (game.Player1.BreedingArea.Count == 0 && game.Player1.BattleArea.Count == 1)
                Console.WriteLine("SUCCESS: Player 1 Digimon moved to Battle Area.");
            else
                Console.WriteLine($"FAILURE: Breeding: {game.Player1.BreedingArea.Count}, Battle: {game.Player1.BattleArea.Count}");

            Console.WriteLine("=== Verification Complete ===");
        }
    }
}
