using System;
using System.Collections.Generic;

namespace Digimon.Core
{
    public class Verification
    {
        public static void Run()
        {
            Console.WriteLine("=== Starting Verification ===");
            
            // 1. Agent vs Agent (Classic)
            Console.WriteLine("--- Test 1: Agent vs Agent (Auto Run via HeadlessGame) ---");
            var gameAA = new HeadlessGame(new List<string>(), new List<string>());
            gameAA.RunUntilConclusion();
            if (gameAA.GameInstance.IsGameOver)
                Console.WriteLine("SUCCESS: Agent vs Agent finished.");
            else
                Console.WriteLine("FAILURE: Agent vs Agent did not finish.");

            // 2. Human vs Agent
            Console.WriteLine("\n--- Test 2: Human (P1) vs Agent (P2) via InteractiveGame ---");
            var gameHA = new InteractiveGame(new List<string>(), new List<string>(), PlayerType.Human, PlayerType.Agent);
            
            // Start
            Console.WriteLine($"Turn 1 (Human): P{gameHA.GameInstance.CurrentPlayer.Id}");
            string stateJson = gameHA.RunStep(); 
            Console.WriteLine($"State JSON Length: {stateJson.Length}");

            // Should verify that nothing happened (waiting for input)
            if (gameHA.GameInstance.TurnStateMachine.TurnCount == 1 && gameHA.GameInstance.CurrentPlayer.Id == 1)
                 Console.WriteLine("SUCCESS: Paused for Human P1.");
            
            // Human Action (Combined Breeding Skip + Main Pass due to Step logic update)
            Console.WriteLine("Human performs Action (Pass).");
            gameHA.Step(0); // Pass

            // Now Agent Turn (P2)
            Console.WriteLine($"After Human Step: P{gameHA.GameInstance.CurrentPlayer.Id}, Turn {gameHA.GameInstance.TurnStateMachine.TurnCount}");
            
            // Agent Turn Execution
            stateJson = gameHA.RunStep(); 
            
            // Agent should have auto-played
             if (gameHA.GameInstance.TurnStateMachine.TurnCount >= 3 || (gameHA.GameInstance.TurnStateMachine.TurnCount == 2 && gameHA.GameInstance.CurrentPlayer.Id == 1)) 
                 Console.WriteLine($"SUCCESS: Agent P2 acted. Current Turn: {gameHA.GameInstance.TurnStateMachine.TurnCount}, P{gameHA.GameInstance.CurrentPlayer.Id}");
            else
                 Console.WriteLine($"FAILURE: Agent did not act. Turn: {gameHA.GameInstance.TurnStateMachine.TurnCount}, P{gameHA.GameInstance.CurrentPlayer.Id}");


            // 3. Human vs Human
            Console.WriteLine("\n--- Test 3: Human vs Human ---");
            var gameHH = new InteractiveGame(new List<string>(), new List<string>(), PlayerType.Human, PlayerType.Human);
            
            // P1
            gameHH.RunStep();
            Console.WriteLine($"Turn {gameHH.GameInstance.TurnStateMachine.TurnCount} (P1 Waiting)");
            gameHH.Step(0); // P1 Pass

            // P2
            Console.WriteLine($"Turn {gameHH.GameInstance.TurnStateMachine.TurnCount} (P2 Waiting)");
             gameHH.RunStep(); // Should wait
             if (gameHH.GameInstance.CurrentPlayer.Id == 2)
                Console.WriteLine("SUCCESS: Paused for Human P2.");
            
             gameHH.Step(0); // P2 Pass
             
             // Back to P1
             Console.WriteLine($"Turn {gameHH.GameInstance.TurnStateMachine.TurnCount} (P1 Waiting)");
             if (gameHH.GameInstance.CurrentPlayer.Id == 1)
                Console.WriteLine("SUCCESS: Back to P1.");

            Console.WriteLine("=== Verification Complete ===");
        }
    }
}
