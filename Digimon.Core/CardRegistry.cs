using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;
using Digimon.Core.Constants;

namespace Digimon.Core
{
    public static class CardRegistry
    {
        private static readonly Dictionary<string, int> _idToInt = [];
        private static readonly Dictionary<int, string> _intToId = [];
        private static readonly Dictionary<string, CardInfo> _cardMetadata = [];
        // 0 is reserved for padding/null
        public static readonly int PaddingId = 0;
        
        // Regex to find text inside < >
        private static readonly Regex _keywordRegex = new(@"<([^>]+)>");

        private class CardInfo 
        {
            public string Name { get; set; } = "";
            public CardKind Kind { get; set; }
            public List<CardColor> Colors { get; set; } = [];
            public int Level { get; set; }
            public int DP { get; set; }
            public int PlayCost { get; set; }
            public int DigivolveCost { get; set; } // Added
            public List<string> Traits { get; set; } = [];
            public HashSet<string> Keywords { get; set; } = [];
            public HashSet<string> InheritedKeywords { get; set; } = [];
        }

        public static void Initialize(string jsonPath)
        {
            if (!File.Exists(jsonPath))
            {
                throw new FileNotFoundException($"cards.json not found at {jsonPath}");
            }

            string json = File.ReadAllText(jsonPath);
            using JsonDocument doc = JsonDocument.Parse(json);
            
            var cardIds = new HashSet<string>();
            _cardMetadata.Clear();

            foreach (var element in doc.RootElement.EnumerateArray())
            {
                if (element.TryGetProperty("card_id", out var idProp))
                {
                   string id = idProp.GetString() ?? string.Empty;
                   if (string.IsNullOrEmpty(id)) continue;

                   cardIds.Add(id);
                   
                   // Parse Metadata
                   var info = new CardInfo();
                   info.Name = element.GetProperty("card_name_eng").GetString() ?? "Unknown";
                   
                   int kindInt = element.GetProperty("card_kind").GetInt32();
                   info.Kind = (CardKind)kindInt; 
                   
                   info.PlayCost = element.TryGetProperty("play_cost", out var pc) ? pc.GetInt32() : 0;
                   info.DigivolveCost = element.TryGetProperty("digivolve_cost", out var dc) ? dc.GetInt32() : 0; // Parse
                   info.DP = element.TryGetProperty("dp", out var dp) ? dp.GetInt32() : 0;
                   info.Level = element.TryGetProperty("level", out var lv) ? lv.GetInt32() : 0;
                   
                   info.Colors = new List<CardColor>();
                   if(element.TryGetProperty("card_colors", out var colors))
                   {
                       foreach(var c in colors.EnumerateArray()) 
                           info.Colors.Add((CardColor)c.GetInt32());
                   }
                   
                   if(element.TryGetProperty("type_eng", out var types))
                   {
                       foreach(var t in types.EnumerateArray())
                           info.Traits.Add(t.GetString() ?? "");
                   }

                   // Keyword Parsing
                   if (element.TryGetProperty("effect_description_eng", out var effect))
                   {
                       string text = effect.GetString() ?? "";
                       foreach(Match match in _keywordRegex.Matches(text))
                       {
                           if (match.Groups.Count > 1)
                               info.Keywords.Add(match.Groups[1].Value);
                       }
                   }

                   if (element.TryGetProperty("inherited_effect_description_eng", out var inherited))
                   {
                       string text = inherited.GetString() ?? "";
                       foreach(Match match in _keywordRegex.Matches(text))
                       {
                           if (match.Groups.Count > 1)
                               info.InheritedKeywords.Add(match.Groups[1].Value);
                       }
                   }

                   _cardMetadata[id] = info;
                }
            }

            // ... (Rest of Init Code) ...
            
            // Sort alphabetically for deterministic IDs
            var sortedIds = cardIds.Where(id => !string.IsNullOrEmpty(id)).OrderBy(id => id).ToList();

            _idToInt.Clear();
            _intToId.Clear();
            
            int currentId = 1;
            foreach (var id in sortedIds)
            {
                _idToInt[id] = currentId;
                _intToId[currentId] = id;
                currentId++;
            }
            
            Console.WriteLine($"[CardRegistry] Initialized with {sortedIds.Count} cards. Max ID: {currentId-1}");
        }
        
        // Helper for testing without file
        public static void InitializeFromList(List<string> cardIds)
        {
             var sortedIds = cardIds.Where(id => !string.IsNullOrEmpty(id)).OrderBy(id => id).Distinct().ToList();

            _idToInt.Clear();
            _intToId.Clear();
            
            int currentId = 1;
            foreach (var id in sortedIds)
            {
                _idToInt[id] = currentId;
                _intToId[currentId] = id;
                currentId++;
            }
        }

        public static int GetId(string cardId)
        {
            if (string.IsNullOrEmpty(cardId)) return PaddingId;
            if (_idToInt.TryGetValue(cardId, out int id)) return id;
            return PaddingId;
        }

        public static string? GetStringId(int internalId)
        {
            if (internalId == PaddingId) return null;
            if (_intToId.TryGetValue(internalId, out string? id)) return id;
            return null;
        }

        public static void Reset() // Clears registry
        {
            _idToInt.Clear();
            _intToId.Clear();
            _cardMetadata.Clear();
        }

        public static Card CreateCard(string id)
        {
            if (_cardMetadata.TryGetValue(id, out var info))
            {
                var c = new Card(id, info.Name, info.Kind, new List<CardColor>(info.Colors), info.Level, info.DP, info.PlayCost, info.DigivolveCost);
                c.Traits.AddRange(info.Traits);
                foreach(var k in info.Keywords) c.Keywords.Add(k);
                foreach(var k in info.InheritedKeywords) c.InheritedKeywords.Add(k);
                return c;
            }
            
            // Fallback for unknown IDs
            Console.WriteLine($"[CardRegistry] Warning: creating dummy card for unknown ID: {id}");
            return new Card(id, $"Dummy {id}", CardKind.Digimon, new List<CardColor>{ CardColor.Red }, 3, 2000, 3, 0);
        }
    }
}
