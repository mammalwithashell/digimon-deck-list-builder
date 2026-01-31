using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.ComponentModel;

namespace Digimon.Core
{
    public enum CardKeyword
    {
        Blocker,
        Rush,
        Piercing,
        Recovery,
        [Description("Security Attack")]
        SecurityAttack,
        Draw,
        Memory,
        [Description("De-Digivolve")]
        DeDigivolve,
        [Description("Digi-Burst")]
        DigiBurst,
        Delay,
        Decoy,
        Retaliation,
        [Description("Armor Purge")]
        ArmorPurge
    }

    public static class CardFeatureExporter
    {
        private static readonly List<string> Timings = new List<string>
        {
            "On Play", "When Digivolving", "On Deletion", "End of Turn", "Start of Turn",
            "Your Turn", "Opponent's Turn", "All Turns", "Security"
        };

        public static string GetDescription(Enum value)
        {
            var field = value.GetType().GetField(value.ToString());
            var attribute = (DescriptionAttribute)Attribute.GetCustomAttribute(field, typeof(DescriptionAttribute));
            return attribute == null ? value.ToString() : attribute.Description;
        }

        public static Dictionary<string, int> ExtractFeatures(Card card)
        {
            var features = new Dictionary<string, int>();

            // Combine Name and Traits for text search (mocking "CardText" for now since Card model is slim)
            string combinedText = (card.Name + " " + string.Join(" ", card.Traits)).ToLower();

            // Check Keywords
            foreach (CardKeyword kw in Enum.GetValues(typeof(CardKeyword)))
            {
                string searchTerm = GetDescription(kw).ToLower();
                if (combinedText.Contains(searchTerm))
                {
                    features[kw.ToString()] = 1;
                }
                else
                {
                    features[kw.ToString()] = 0;
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
