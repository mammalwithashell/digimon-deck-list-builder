using Digimon.Core.Constants;

namespace Digimon.Core
{
    public class Card
    {
        public string Id { get; set; }
        public string Name { get; set; }
        public int PlayCost { get; set; }
        public int UseCost { get; set; }
        public int BaseDP { get; set; }
        public int DigivolveCost { get; set; } // MVP: Primary Evo Cost
        public int Level { get; set; }
        public List<CardColor> Colors { get; set; }
        public CardKind Kind { get; set; }
        public List<string> Traits { get; set; } = new List<string>();
        public HashSet<string> Keywords { get; set; } = new HashSet<string>();
        public HashSet<string> InheritedKeywords { get; set; } = new HashSet<string>();

        // Logic placeholders
        public bool IsDigimon => Kind == CardKind.Digimon;
        public bool IsTamer => Kind == CardKind.Tamer;
        public bool IsOption => Kind == CardKind.Option;
        public bool IsDigiEgg => Kind == CardKind.DigiEgg;

        public Card(string id, string name, CardKind kind, List<CardColor> colors, int level, int dp, int playCost, int digivolveCost = 0)
        {
            Id = id;
            Name = name;
            Kind = kind;
            Colors = colors;
            Level = level;
            BaseDP = dp;
            PlayCost = playCost;
            DigivolveCost = digivolveCost;
        }

        // Additional logic from CardSource can be added here
        public int GetCurrentDP()
        {
            // Placeholder for DP calculation with effects
            return BaseDP;
        }
    }
}
