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
        public Player CurrentPlayer { get; set; }
        public Player OpponentPlayer => CurrentPlayer == Player1 ? Player2 : Player1;
        public TurnStateMachine TurnStateMachine { get; private set; }
        public bool IsGameOver { get; private set; }
        public Player Winner { get; private set; }

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
            // Tensor Layout (Total Size: 680 floats):
            // ---------------------------------------------------------
            // [0-9] GLOBAL DATA (10 floats)
            // 0: Turn Count
            // 1: Current Phase (Enum Int)
            // 2: Memory Gauge (Relative to requesting player)
            // 3-9: Reserved / Padding (Index 8: PendingAction)
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
            // [530-574] MY TRASH (45 floats)
            // Top 45 Card IDs in Trash.
            //
            // [575-619] OPPONENT TRASH (45 floats)
            // Top 45 Card IDs in Trash.
            //
            // [620-629] MY SECURITY STACK (10 floats)
            // Top 10 Card IDs (Revealed/Known state assumption or placeholders).
            //
            // [630-639] OPPONENT SECURITY STACK (10 floats)
            // Top 10 Card IDs.
            //
            // [640-659] MY BREEDING AREA (1 Slot * 20 Floats = 20 floats)
            // Standard slot structure.
            //
            // [660-679] OPPONENT BREEDING AREA (1 Slot * 20 Floats = 20 floats)
            // Standard slot structure.
            // ---------------------------------------------------------

            List<float> tensor = new List<float>();

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
            AppendFieldData(tensor, me, 12);

            // --- Opp Field [12 slots * 20] ---
            AppendFieldData(tensor, opp, 12);

            // --- My Hand [20] ---
            AppendListIds(tensor, me.Hand, 20);

            // --- Opp Hand [20] ---
            AppendListIds(tensor, opp.Hand, 20);

            // --- My Trash [45] ---
            AppendListIds(tensor, me.Trash, 45);

            // --- Opp Trash [45] ---
            AppendListIds(tensor, opp.Trash, 45);

            // --- My Security [10] ---
            AppendListIds(tensor, me.Security, 10);

            // --- Opp Security [10] ---
            AppendListIds(tensor, opp.Security, 10);

            // --- My Breeding [1 slot * 20] ---
            AppendFieldData(tensor, me, 1, true);

            // --- Opp Breeding [1 slot * 20] ---
            AppendFieldData(tensor, opp, 1, true);

            return tensor.ToArray();
        }

        private void AppendFieldData(List<float> tensor, Player p, int slots, bool isBreeding = false)
        {
            var sourceList = isBreeding ? p.BreedingArea : p.BattleArea;

            for (int i = 0; i < slots; i++)
            {
                if (i < sourceList.Count)
                {
                    Card c = sourceList[i];
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

        private void AppendListIds(List<float> tensor, List<Card> cards, int limit)
        {
            for (int i = 0; i < limit; i++)
            {
                if (i < cards.Count)
                    tensor.Add(CardRegistry.GetId(cards[i].Id));
                else
                    tensor.Add(0);
            }
        }

        public void EndGame(Player winner)
        {
            IsGameOver = true;
            Winner = winner;
        }

        public void SwitchTurn()
        {
            CurrentPlayer.IsMyTurn = false;
            CurrentPlayer = OpponentPlayer;
            CurrentPlayer.IsMyTurn = true;

            TurnStateMachine.StartTurn();
        }

        public string ToJson()
        {
            var state = new
            {
                TurnCount = TurnStateMachine.TurnCount,
                CurrentPhase = TurnStateMachine.CurrentPhase.ToString(),
                CurrentPlayer = CurrentPlayer.Id,
                MemoryGauge = MemoryGauge,
                IsGameOver = IsGameOver,
                Winner = Winner?.Id,
                Player1 = new
                {
                    Id = Player1.Id,
                    Memory = GetMemory(Player1),
                    HandCount = Player1.Hand.Count,
                    HandIds = Player1.Hand.Select(c => c.Id).ToList(),
                    SecurityCount = Player1.Security.Count,
                    DeckCount = Player1.Deck.Count,
                    BattleAreaCount = Player1.BattleArea.Count
                },
                Player2 = new
                {
                    Id = Player2.Id,
                    Memory = GetMemory(Player2),
                    HandCount = Player2.Hand.Count,
                    HandIds = Player2.Hand.Select(c => c.Id).ToList(),
                    SecurityCount = Player2.Security.Count,
                    DeckCount = Player2.Deck.Count,
                    BattleAreaCount = Player2.BattleArea.Count
                }
            };

            return JsonSerializer.Serialize(state, new JsonSerializerOptions { WriteIndented = true });
        }
    }
}
