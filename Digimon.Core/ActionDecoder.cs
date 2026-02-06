using System;
using System.Collections.Generic;
using Digimon.Core.Constants;

namespace Digimon.Core
{
    public static class ActionDecoder
    {
        public static void DecodeAndExecute(Game game, int actionId)
        {
            var phase = game.TurnStateMachine.CurrentPhase;

            // 1. Context: Main Phase (Standard Turn)
            if (phase == GamePhase.Main)
            {
                DecodeMainPhase(game, actionId);
            }
            // 2. Context: Breeding Phase (Limited Actions)
            else if (phase == GamePhase.Breeding)
            {
                DecodeBreedingPhase(game, actionId);
            }
            // 3. Context: Pending Target/Material (Selection)
            else if (phase == GamePhase.SelectTarget || phase == GamePhase.SelectMaterial)
            {
                DecodeSelection(game, actionId);
            }
            // 4. Context: Block Timing (Opponent Turn)
            else if (phase == GamePhase.BlockTiming)
            {
                DecodeBlock(game, actionId);
            }
             // 5. Context: Counter Timing (Opponent Turn)
            else if (phase == GamePhase.CounterTiming)
            {
                DecodeCounter(game, actionId);
            }
             // 6. Context: Trash/Source Selection
            else if (phase == GamePhase.SelectTrash)
            {
                DecodeTrashSelection(game, actionId);
            }
            else if (phase == GamePhase.SelectSource)
            {
                DecodeSourceSelection(game, actionId);
            }
            else
            {
                Console.WriteLine($"[ActionDecoder] Warning: Action received in unhandled phase: {phase}");
            }
        }

        private static void DecodeMainPhase(Game game, int actionId)
        {
             // General (0-99)
             if (actionId >= 0 && actionId <= 99)
             {
                 if (actionId <= 29) // Play Card (0-29)
                 {
                     int handIndex = actionId;
                     // TODO: Check if card needs targets (e.g. Option with Destroy effect)
                     // If so, game.TurnStateMachine.TransitionTo(GamePhase.SelectTarget);
                     // For now, just play it directly using existing logic stub
                     // We need a PlayCard method in Player/Game that handles memory payment
                     // game.CurrentPlayer.PlayCard(handIndex);
                     // Console.WriteLine($"[ActionDecoder] Play Card Index {handIndex}");
                     game.Logger.LogVerbose($"[ActionDecoder] Play Card Index {handIndex}");
                     game.CurrentPlayer.PlayCard(handIndex, game); // Assuming we'll add this
                 }
                 else if (actionId >= 30 && actionId <= 59) // Trash Card (30-59)
                 {
                     int handIndex = actionId - 30;
                     // Only valid if an effect requires it, but maybe we allow voluntary trash?
                     // Usually not allowed rules-wise unless effect triggers it.
                     // But for "General" actions, maybe unused in Main Phase.
                 }
                 else if (actionId == 62) // Pass Turn
                 {
                     game.TurnStateMachine.PassTurn();
                 }
                 else if (actionId == 63) // Unsuspend
                 {
                     // Typically Phase Start only, but some effects might allow?
                 }
             }
             // Attack (100-399)
             else if (actionId >= 100 && actionId <= 399)
             {
                 // Formula: 100 + (AttackerIndex * 15) + TargetIndex
                 int normalized = actionId - 100;
                 int attackerIndex = normalized / 15;
                 int targetIndex = normalized % 15;
                 
                 game.Logger.Log($"[ActionDecoder] Attack: Slot {attackerIndex} -> Target {targetIndex}");
                 game.ExecuteAttack(attackerIndex, targetIndex);
             }
             // Digivolve (400-999)
             else if (actionId >= 400 && actionId <= 999)
             {
                 // Formula: 400 + (HandCardIndex * 15) + TargetFieldIndex
                 int normalized = actionId - 400;
                 int handIndex = normalized / 15;
                 int fieldIndex = normalized % 15;

                 game.Logger.Log($"[ActionDecoder] Digivolve: Hand {handIndex} -> Field {fieldIndex}");
                 game.ExecuteDigivolve(handIndex, fieldIndex);
             }
             // Activate Effect (1000-1999)
             else if (actionId >= 1000 && actionId <= 1999)
             {
                 int normalized = actionId - 1000;
                 int sourceIndex = normalized / 10;
                 int effectIndex = normalized % 10;

                 game.Logger.Log($"[ActionDecoder] Effect: Source {sourceIndex}, Effect {effectIndex}");
                 // game.ExecuteEffect(sourceIndex, effectIndex);
             }
        }

        private static void DecodeBreedingPhase(Game game, int actionId)
        {
            // Hatch (60)
            if (actionId == 60) game.BreedingHatch();
            // Move (61)
            if (actionId == 61) game.BreedingMove();
            // Pass/Skip (62 or 0-29/etc treated as skip?)
            // Usually we treat non-breeding actions as "Do nothing/Pass" or Invalid.
            // If Pass (62) is sent, we skip.
            if (actionId == 62) game.BreedingPass();
        }

        private static void DecodeSelection(Game game, int actionId)
        {
             // Re-interpret indices for selection
             if (actionId >= 0 && actionId <= 29)
             {
                 game.Logger.LogVerbose($"[ActionDecoder] Selected Hand Card {actionId}");
                 // game.ResolveSelection(TargetType.Hand, actionId);
             }
             else if (actionId >= 100 && actionId <= 299)
             {
                 // Attack Range maps to Field Slots
                 // 100-111: My Field
                 // 115-126: Opp Field
                 int normalized = actionId - 100;
                 // Determine slot.. roughly. 
                 // We can simplify and just say 100+ = Field Entity selection.
                  game.Logger.LogVerbose($"[ActionDecoder] Selected Field Entity {actionId}");
             }
             
             // After selection, transition back to Main or Next Step
             game.TurnStateMachine.ClearPendingState(); 
        }


        
        // ... (Main Phase Methods remain same) ...

        private static void DecodeBlock(Game game, int actionId)
        {
             if (actionId == 62) // Pass/Decline
             {
                 game.TurnStateMachine.ClearPendingState(); 
             }
             else if (actionId >= 100 && actionId <= 111)
             {
                 int blockerIndex = actionId - 100;
                  game.Logger.Log($"[ActionDecoder] Block with Slot {blockerIndex}");
                  // game.ExecuteBlock(blockerIndex);
                  game.TurnStateMachine.ClearPendingState();
             }
        }

        private static void DecodeCounter(Game game, int actionId)
        {
            if (actionId == 62) // Pass/Decline
            {
                game.TurnStateMachine.ClearPendingState(); 
            }
            else if (actionId >= 400 && actionId <= 999)
            {
                 // Blast Digivolve (Same range as Digivolve)
                 int normalized = actionId - 400;
                 int handIndex = normalized / 15;
                 int fieldIndex = normalized % 15;
                 game.Logger.Log($"[ActionDecoder] Counter Blast Digivolve: Hand {handIndex} -> Field {fieldIndex}");
                 // game.ExecuteBlastDigivolve(handIndex, fieldIndex);
                 game.TurnStateMachine.ClearPendingState(); 
            }
        }
        
        private static void DecodeTrashSelection(Game game, int actionId)
        {
            // Reuse 0-59 for Trash Indices
            if (actionId >= 0 && actionId <= 59)
            {
                game.Logger.LogVerbose($"[ActionDecoder] Selected Trash Index {actionId}");
                // game.ResolveSelection(TargetType.Trash, actionId);
                game.TurnStateMachine.ClearPendingState();
            }
        }

        private static void DecodeSourceSelection(Game game, int actionId)
        {
            // Range 2000-2119: Source Index
             if (actionId >= 2000 && actionId <= 2119)
             {
                 int normalized = actionId - 2000;
                 int fieldIndex = normalized / 10;
                 int sourceIndex = normalized % 10;
                 game.Logger.LogVerbose($"[ActionDecoder] Selected Source: Field {fieldIndex}, Source {sourceIndex}");
                 // game.ResolveSelection(TargetType.Source, fieldIndex, sourceIndex);
                 game.TurnStateMachine.ClearPendingState();
             }
        }
    }
}
