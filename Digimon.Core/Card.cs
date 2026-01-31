using System;
using System.Collections.Generic;

namespace Digimon.Core
{
    public enum CardColor
    {
        Red, Blue, Yellow, Green, Black, Purple, White, Multi
    }

    public enum CardKind
    {
        Digimon, Tamer, Option, DigiEgg
    }

    public class Card
    {
        public string Id { get; set; }
        public string Name { get; set; }
        public int PlayCost { get; set; }
        public int UseCost { get; set; }
        public int BaseDP { get; set; }
        public int Level { get; set; }
        public CardColor Color { get; set; }
        public CardKind Kind { get; set; }
        public List<string> Traits { get; set; } = new List<string>();

        // Logic placeholders
        public bool IsDigimon => Kind == CardKind.Digimon;
        public bool IsTamer => Kind == CardKind.Tamer;
        public bool IsOption => Kind == CardKind.Option;
        public bool IsDigiEgg => Kind == CardKind.DigiEgg;

        public Card(string id, string name, CardKind kind, CardColor color, int level, int dp, int playCost)
        {
            Id = id;
            Name = name;
            Kind = kind;
            Color = color;
            Level = level;
            BaseDP = dp;
            PlayCost = playCost;
        }

        // Additional logic from CardSource can be added here
        public int GetCurrentDP()
        {
            // Placeholder for DP calculation with effects
            return BaseDP;
        }
    }
}
