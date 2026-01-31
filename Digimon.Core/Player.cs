using System;
using System.Collections.Generic;
using System.Linq;

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

        public void Draw()
        {
            if (Deck.Count > 0)
            {
                Card card = Deck[0];
                Deck.RemoveAt(0);
                Hand.Add(card);
                // Console.WriteLine($"[Player {Id}] Drew {card.Name} ({card.Id})");
            }
            else
            {
                // Deck out logic handled by Game
                // Console.WriteLine($"[Player {Id}] Cannot draw - Deck empty.");
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



        public void Hatch()
        {
            if (DigitamaDeck.Count > 0 && BreedingArea.Count == 0)
            {
                Card egg = DigitamaDeck[0];
                DigitamaDeck.RemoveAt(0);
                
                Permanent permanent = new Permanent(egg);
                BreedingArea.Add(permanent);
                // Console.WriteLine($"[Player {Id}] Hatched {egg.Name} ({egg.Id})");
            }
        }

        public void MoveBreedingToBattle()
        {
            if (BreedingArea.Count > 0)
            {
                Permanent digimon = BreedingArea[0];
                BreedingArea.Clear();
                BattleArea.Add(digimon);
                // Console.WriteLine($"[Player {Id}] Moved {digimon.TopCard.Name} to Battle Area.");
            }
        }

        public void UnsuspendAll()
        {
            foreach (var p in BreedingArea) p.Unsuspend();
            foreach (var p in BattleArea) p.Unsuspend();
        }
    }
}
