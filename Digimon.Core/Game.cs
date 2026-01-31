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

            // Standard Rule:
            // If new turn player has <= 0 memory (meaning opponent passed big or just barely passed),
            // reset to 3.
            // From perspective of new player:
            int startMemory = GetMemory(CurrentPlayer);
            if (startMemory <= 0)
            {
                // Set to 3
                if (CurrentPlayer == Player1) MemoryGauge = 3;
                else MemoryGauge = -3;
            }

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
