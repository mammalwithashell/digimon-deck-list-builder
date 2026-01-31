using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Digimon.Core
{
    public static class CardFeatureExporter
    {
        // Vocabulary
        private static readonly List<string> Keywords = new List<string>
        {
            "Blocker", "Rush", "Piercing", "Recovery", "Security Attack", "Draw", "Memory",
            "De-Digivolve", "Digi-Burst", "Delay", "Decoy", "Retaliation", "Armro Purge"
        };

        private static readonly List<string> Timings = new List<string>
        {
            "On Play", "When Digivolving", "On Deletion", "End of Turn", "Start of Turn",
            "Your Turn", "Opponent's Turn", "All Turns", "Security"
        };

        public static Dictionary<string, int> ExtractFeatures(Card card)
        {
            var features = new Dictionary<string, int>();

            // Combine Name and Traits for text search (mocking "CardText" for now since Card model is slim)
            // In a real scenario, Card would have 'EffectText' property.
            // Using Name + Traits as proxy + some explicit logic if we had properties
            string combinedText = (card.Name + " " + string.Join(" ", card.Traits)).ToLower();

            // Check Keywords
            foreach (var kw in Keywords)
            {
                // Simple regex or string check
                if (combinedText.Contains(kw.ToLower()))
                {
                    features[kw] = 1;
                }
                else
                {
                    features[kw] = 0;
                }
            }

            // Check Timings
            foreach (var time in Timings)
            {
                if (combinedText.Contains(time.ToLower()))
                {
                    features[time] = 1;
                }
                else
                {
                    features[time] = 0;
                }
            }

            // Explicit Properties (Mock)
            features["IsDigimon"] = card.IsDigimon ? 1 : 0;
            features["IsTamer"] = card.IsTamer ? 1 : 0;
            features["IsOption"] = card.IsOption ? 1 : 0;
            features["Level"] = card.Level;
            features["PlayCost"] = card.PlayCost;
            features["DP"] = card.BaseDP;

            return features;
        }

        public static void ExportFeatures(string outputPath, List<Card> allCards)
        {
            var exportData = new Dictionary<int, Dictionary<string, int>>();

            foreach (var card in allCards)
            {
                int id = CardRegistry.GetId(card.Id);
                if (id == 0)
                {
                    CardRegistry.Register(card.Id);
                    id = CardRegistry.GetId(card.Id);
                }

                exportData[id] = ExtractFeatures(card);
            }

            string json = JsonSerializer.Serialize(exportData, new JsonSerializerOptions { WriteIndented = true });
            File.WriteAllText(outputPath, json);
        }
    }
}
