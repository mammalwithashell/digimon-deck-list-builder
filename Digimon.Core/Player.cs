using System;
using System.Collections.Generic;
using System.Linq;

namespace Digimon.Core
{
    public class Player
    {
        public int Id { get; set; }
        public string Name { get; set; }
        public int Memory { get; set; }

        public List<Card> Hand { get; set; } = new List<Card>();
        public List<Card> Deck { get; set; } = new List<Card>();
        public List<Card> Security { get; set; } = new List<Card>();
        public List<Card> Trash { get; set; } = new List<Card>();
        public List<Card> BreedingArea { get; set; } = new List<Card>(); // Using Card for now, maybe Permanent later
        public List<Card> BattleArea { get; set; } = new List<Card>();

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
            Deck = new List<Card>(newDeck);
            // Shuffle stub
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
    }
}
