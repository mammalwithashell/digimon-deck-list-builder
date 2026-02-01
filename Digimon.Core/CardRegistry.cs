using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;

namespace Digimon.Core
{
    public static class CardRegistry
    {
        private static readonly Dictionary<string, int> _idToInt = [];
        private static readonly Dictionary<int, string> _intToId = [];
        // 0 is reserved for padding/null
        public static readonly int PaddingId = 0;

        public static void Initialize(string jsonPath)
        {
            if (!File.Exists(jsonPath))
            {
                throw new FileNotFoundException($"cards.json not found at {jsonPath}");
            }

            string json = File.ReadAllText(jsonPath);
            using JsonDocument doc = JsonDocument.Parse(json);
            
            var cardIds = new HashSet<string>();
            foreach (var element in doc.RootElement.EnumerateArray())
            {
                if (element.TryGetProperty("card_id", out var idProp))
                {
                   cardIds.Add(idProp.GetString() ?? string.Empty);
                }
            }

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
            
            // Allow dynamic temporary registration for debugging if not found? 
            // Better to return PaddingId (0) or throw? 
            // Request said "Reserve 0 for Empty/Padding". 
            // If unknown, let's treat as padding/unknown (0)
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
        }
    }
}
