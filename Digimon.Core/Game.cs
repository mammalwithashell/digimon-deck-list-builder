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

            // Reset Memory to 3 if less than 0?
            // Simple rule: if turn ends, memory is passed.
            // For now, assume fixed memory or memory gauge logic.
            // We'll set memory to 3 for the new turn player as a simplification if we don't track full gauge.
            CurrentPlayer.Memory = 3;

            TurnStateMachine.StartTurn();
        }

        public string ToJson()
        {
            var state = new
            {
                TurnCount = TurnStateMachine.TurnCount,
                CurrentPhase = TurnStateMachine.CurrentPhase.ToString(),
                CurrentPlayer = CurrentPlayer.Id,
                IsGameOver = IsGameOver,
                Winner = Winner?.Id,
                Player1 = new
                {
                    Id = Player1.Id,
                    Memory = Player1.Memory,
                    HandCount = Player1.Hand.Count,
                    HandIds = Player1.Hand.Select(c => c.Id).ToList(),
                    SecurityCount = Player1.Security.Count,
                    DeckCount = Player1.Deck.Count,
                    BattleAreaCount = Player1.BattleArea.Count
                },
                Player2 = new
                {
                    Id = Player2.Id,
                    Memory = Player2.Memory,
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
