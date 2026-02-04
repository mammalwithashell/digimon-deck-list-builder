using System;
using System.Collections.Generic;
using System.Linq;
using Digimon.Core.Loggers;

namespace Digimon.Core
{
    public class Player
    {
        public int Id { get; set; }
        public string Name { get; set; }
        // Memory is managed by Game.MemoryGauge

        public List<Card> Hand { get; set; } = [];
        public List<Card> Deck { get; set; } = [];
        public List<Card> DigitamaDeck { get; set; } = [];
        public List<Card> Security { get; set; } = [];
        public List<Card> Trash { get; set; } = [];
        public List<Permanent> BreedingArea { get; set; } = [];
        public List<Permanent> BattleArea { get; set; } = [];

        public bool IsMyTurn { get; set; }

        public Player(int id, string name)
        {
            Id = id;
            Name = name;
        }

        public void Draw(IGameLogger? logger = null)
        {
            if (Deck.Count > 0)
            {
                Card card = Deck[0];
                Deck.RemoveAt(0);
                Hand.Add(card);
                logger?.LogVerbose($"[Player {Id}] Drew {card.Name} ({card.Id})");
            }
            else
            {
                // Deck out logic handled by Game
                logger?.LogVerbose($"[Player {Id}] Cannot draw - Deck empty.");
            }
        }

        public void SetupDeck(List<Card> newDeck)
        {
            Deck = [];
            DigitamaDeck = [];

            foreach (var card in newDeck)
            {
                if (card.IsDigiEgg)
                {
                    DigitamaDeck.Add(card);
                }
                else
                {
                    Deck.Add(card);
                }
            }
            // Shuffle stub for both decks would go here
        }

        public void SetupSecurity(int count)
        {
            for (int i = 0; i < count; i++)
            {
                if (Deck.Count > 0)
                {
                    Card card = Deck[0];
                    Deck.RemoveAt(0);
                    Security.Add(card);
                }
            }
        }



        public void Hatch(IGameLogger? logger = null)
        {
            if (DigitamaDeck.Count > 0 && BreedingArea.Count == 0)
            {
                Card egg = DigitamaDeck[0];
                DigitamaDeck.RemoveAt(0);
                
                Permanent permanent = new(egg);
                BreedingArea.Add(permanent);
                logger?.Log($"[Player {Id}] Hatched {egg.Name} ({egg.Id})");
            }
        }

        public void MoveBreedingToBattle(IGameLogger? logger = null)
        {
            if (BreedingArea.Count > 0)
            {
                Permanent digimon = BreedingArea[0];
                BreedingArea.Clear();
                BattleArea.Add(digimon);
                logger?.Log($"[Player {Id}] Moved {digimon.TopCard.Name} to Battle Area.");
            }
        }

        public void UnsuspendAll()
        {
            foreach (var p in BreedingArea) 
            {
                p.Unsuspend();
                p.ResetTurnStats();
            }
            foreach (var p in BattleArea) 
            {
                p.Unsuspend();
                p.ResetTurnStats();
            }
        }

        public void PlayCard(int handIndex, Game game)
        {
            if (handIndex < 0 || handIndex >= Hand.Count) return;

            Card card = Hand[handIndex];
            
            // 1. Pay Cost (Standard Play Cost)
            // Check if enough memory? (Game rules allow going negative, but Turn ends)
            // But we should subtract cost.
            game.PayCost(this, card.PlayCost);

            // 2. Place on Board
            Hand.RemoveAt(handIndex);
            
            if (card.IsDigimon || card.IsTamer)
            {
                Permanent perm = new(card);
                BattleArea.Add(perm);
                game.Logger.Log($"[Player {Id}] Played {card.Name}. Memory: {game.MemoryGauge}");
            }
            else if (card.IsOption)
            {
                // Option Resolution Stub
                // Use Main Effect -> Then Trash
                Trash.Add(card);
                game.Logger.Log($"[Player {Id}] Used Option {card.Name}. Memory: {game.MemoryGauge}");
            }
            
            // 3. Trigger OnPlay Effects (Stub)
            
            // 4. Check Turn End via Game Loop (Caller handles this usually, or TurnStateMachine checks)
            game.TurnStateMachine.CheckTurnEnd();
        }

        public void RemovePermanent(Permanent permanent)
        {
            if (BattleArea.Contains(permanent))
            {
                BattleArea.Remove(permanent);
            }
            // Could also be in Breeding, but usually Deletion is from Battle
        }

        public void AddCardsToTrash(List<Card> cards)
        {
            Trash.AddRange(cards);
        }
    }
}
