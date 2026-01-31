using System;
using System.Text.Json;
using System.Collections.Generic;
using System.Linq;

namespace Digimon.Core
{
    public class Game
    {
        public Player Player1 { get; private set; }
        public Player Player2 { get; private set; }
        public Player? CurrentPlayer { get; set; }
        public Player? OpponentPlayer => CurrentPlayer == Player1 ? Player2 : Player1;
        public TurnStateMachine TurnStateMachine { get; private set; }
        public bool IsGameOver { get; private set; }
        public Player? Winner { get; private set; }

        // Shared Memory Gauge: Positive for P1, Negative for P2.
        // P1 turn: 1 to 10 (Max), P2 side is < 0.
        // P2 turn: -1 to -10 (Max), P1 side is > 0.
        public int MemoryGauge { get; private set; }

        public Game()
        {
            Player1 = new Player(1, "Player 1");
            Player2 = new Player(2, "Player 2");
            TurnStateMachine = new TurnStateMachine(this);
        }

        public void StartGame(System.Collections.Generic.List<Card> deck1, System.Collections.Generic.List<Card> deck2)
        {
            Player1.SetupDeck(deck1);
            Player2.SetupDeck(deck2);

            Player1.SetupSecurity(5);
            Player2.SetupSecurity(5);

            // Set Initial Memory
            MemoryGauge = 0;

            // Initial Draw
            for(int i=0; i<5; i++)
            {
                Player1.Draw();
                Player2.Draw();
            }

            // Decide who goes first (random or fixed)
            CurrentPlayer = Player1;
            Player1.IsMyTurn = true;
            Player2.IsMyTurn = false;

            // Start First Turn
            TurnStateMachine.StartTurn();
        }

        public int GetMemory(Player player)
        {
            if (player == Player1) return MemoryGauge;
            return -MemoryGauge;
        }

        public void AddMemory(Player player, int amount)
        {
            if (player == Player1)
            {
                MemoryGauge += amount;
                if (MemoryGauge > 10) MemoryGauge = 10;
            }
            else
            {
                MemoryGauge -= amount;
                if (MemoryGauge < -10) MemoryGauge = -10;
            }
        }

        public void PayCost(Player player, int cost)
        {
            // Paying cost means moving towards opponent's side (subtracting memory from self)
            // If P1 pays, MemoryGauge decreases.
            // If P2 pays, MemoryGauge increases (from negative to positive).
            if (player == Player1)
            {
                MemoryGauge -= cost;
                if (MemoryGauge < -10) MemoryGauge = -10;
            }
            else
            {
                MemoryGauge += cost;
                if (MemoryGauge > 10) MemoryGauge = 10;
            }
        }

        public float[] GetBoardStateTensor(int playerId)
        {
            // Tensor Layout (Total Size: 570 floats):
            // ---------------------------------------------------------
            // [0-9] GLOBAL DATA (10 floats)
            // 0: Turn Count
            // 1: Current Phase (Enum Int)
            // 2: Memory Gauge (Relative to requesting player)
            // 3: Reserved / Padding
            // 4: Reserved / Padding
            // 5: Reserved / Padding
            // 6: Reserved / Padding
            // 7: Reserved / Padding
            // 8: Reserved / Padding (e.g. Pending Action Type)
            // 9: Reserved / Padding
            //
            // [10-249] MY BATTLE AREA (12 Slots * 20 Floats = 240 floats)
            // Each Slot Structure:
            //   Offset + 0: Top Card Internal ID (from CardRegistry)
            //   Offset + 1: Current DP
            //   Offset + 2: Is Suspended (1.0 = Yes, 0.0 = No)
            //   Offset + 3: Source Count
            //   Offset + 4-19: Source Card IDs (Bottom to Top, max 16)
            // Slots 0-11 represent the field positions.
            //
            // [250-489] OPPONENT BATTLE AREA (12 Slots * 20 Floats = 240 floats)
            // Same structure as My Battle Area.
            //
            // [490-509] MY HAND (20 floats)
            // List of Card IDs. Paddings are 0.
            //
            // [510-529] OPPONENT HAND (20 floats)
            // List of Card IDs (Perfect Information assumption for training).
            //
            // [530-539] MY TRASH (10 floats)
            // Top 10 Card IDs in Trash.
            //
            // [540-549] OPPONENT TRASH (10 floats)
            // Top 10 Card IDs in Trash.
            //
            // [550-559] MY SECURITY STACK (10 floats)
            // Top 10 Card IDs (Revealed/Known state assumption or placeholders).
            //
            // [560-569] OPPONENT SECURITY STACK (10 floats)
            // Top 10 Card IDs.
            // ---------------------------------------------------------

            List<float> tensor = [];

            Player me = (playerId == 1) ? Player1 : Player2;
            Player opp = (playerId == 1) ? Player2 : Player1;

            // --- Global Data [0-9] ---
            tensor.Add(TurnStateMachine.TurnCount);
            tensor.Add((float)TurnStateMachine.CurrentPhase);
            // Memory relative to me
            tensor.Add(GetMemory(me));
            // Pad remaining 7
            for(int i=0; i<7; i++) tensor.Add(0);

            // --- My Field [12 slots * 20] ---
            AppendFieldData(tensor, me);

            // --- Opp Field [12 slots * 20] ---
            AppendFieldData(tensor, opp);

            // --- My Hand [20] ---
            AppendListIds(tensor, me.Hand, 20);

            // --- Opp Hand [20] ---
            AppendListIds(tensor, opp.Hand, 20);

            // --- My Trash [10] ---
            AppendListIds(tensor, me.Trash, 10);

            // --- Opp Trash [10] ---
            AppendListIds(tensor, opp.Trash, 10);

            // --- My Security [10] ---
            AppendListIds(tensor, me.Security, 10);

            // --- Opp Security [10] ---
            AppendListIds(tensor, opp.Security, 10);

            return [.. tensor];
        }

        private static void AppendFieldData(List<float> tensor, Player p)
        {
            // 12 Slots
            for (int i = 0; i < 12; i++)
            {
                if (i < p.BattleArea.Count)
                {
                    Card c = p.BattleArea[i];
                    // 1. Internal ID
                    tensor.Add(CardRegistry.GetId(c.Id));
                    // 2. DP
                    tensor.Add(c.GetCurrentDP());
                    // 3. Suspended
                    tensor.Add(0); // Stub
                    // 4. Source Count
                    tensor.Add(0); // Stub
                    // 5-20. Sources (16 slots)
                    for(int j=0; j<16; j++) tensor.Add(0);
                }
                else
                {
                    // Empty Slot (20 floats)
                    for(int j=0; j<20; j++) tensor.Add(0);
                }
            }
        }

        private static void AppendListIds(List<float> tensor, List<Card> cards, int limit)
        {
            for (int i = 0; i < limit; i++)
            {
                if (i < cards.Count)
                    tensor.Add(CardRegistry.GetId(cards[i].Id));
                else
                    tensor.Add(0);
            }
        }

        public void EndGame(Player? winner)
        {
            IsGameOver = true;
            Winner = winner;
        }

        public void SwitchTurn()
        {
            if (CurrentPlayer != null) CurrentPlayer.IsMyTurn = false;
            CurrentPlayer = OpponentPlayer;
            if (CurrentPlayer != null) CurrentPlayer.IsMyTurn = true;

            TurnStateMachine.StartTurn();
        }

        private static readonly JsonSerializerOptions _jsonOptions = new() { WriteIndented = true };

        public string ToJson()
        {
            var state = new
            {
                TurnStateMachine.TurnCount,
                TurnStateMachine.CurrentPhase,
                CurrentPlayer = CurrentPlayer?.Id,
                MemoryGauge,
                IsGameOver,
                Winner = Winner?.Id,
                Player1 = new
                {
                    Player1.Id,
                    Memory = GetMemory(Player1),
                    HandCount = Player1.Hand.Count,
                    HandIds = Player1.Hand.Select(c => c.Id).ToList(),
                    SecurityCount = Player1.Security.Count,
                    DeckCount = Player1.Deck.Count,
                    BattleAreaCount = Player1.BattleArea.Count
                },
                Player2 = new
                {
                    Player2.Id,
                    Memory = GetMemory(Player2),
                    HandCount = Player2.Hand.Count,
                    HandIds = Player2.Hand.Select(c => c.Id).ToList(),
                    SecurityCount = Player2.Security.Count,
                    DeckCount = Player2.Deck.Count,
                    BattleAreaCount = Player2.BattleArea.Count
                }
            };

            return JsonSerializer.Serialize(state, _jsonOptions);
        }
    }
}
