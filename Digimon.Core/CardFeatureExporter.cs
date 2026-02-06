using System.Text.Json;
using System.ComponentModel;
using Digimon.Core.Constants;

namespace Digimon.Core
{
    public static class CardFeatureExporter
    {


        private static readonly JsonSerializerOptions _jsonOptions = new() { WriteIndented = true };

        public static string GetDescription(Enum value)
        {
            var field = value.GetType().GetField(value.ToString());
            if (field is null) return value.ToString();

            var attribute = Attribute.GetCustomAttribute(field, typeof(DescriptionAttribute)) as DescriptionAttribute;
            return attribute?.Description ?? value.ToString();
        }

        public static Dictionary<string, int> ExtractFeatures(Card card)
        {
            var features = new Dictionary<string, int>();

            // Combine Name and Traits for text search (mocking "CardText" for now since Card model is slim)
            string combinedText = card.Name + " " + string.Join(" ", card.Traits);

            // Check Keywords
            foreach (CardKeyword kw in Enum.GetValues(typeof(CardKeyword)))
            {
                string searchTerm = GetDescription(kw);
                if (combinedText.Contains(searchTerm, StringComparison.OrdinalIgnoreCase))
                {
                    features[kw.ToString()] = 1;
                }
                else
                {
                    features[kw.ToString()] = 0;
                }
            }

            // Check Timings
            foreach (EffectTiming time in Enum.GetValues(typeof(EffectTiming)))
            {
                string searchTerm = GetDescription(time);
                if (combinedText.Contains(searchTerm, StringComparison.OrdinalIgnoreCase))
                {
                    features[time.ToString()] = 1;
                }
                else
                {
                    features[time.ToString()] = 0;
                }
            }

            // Explicit Properties (Mock)
            features["IsDigimon"] = card.IsDigimon ? 1 : 0;
            features["IsTamer"] = card.IsTamer ? 1 : 0;
            features["IsOption"] = card.IsOption ? 1 : 0;
            features["Level"] = card.Level;
            features["PlayCost"] = card.PlayCost;
            features["DP"] = card.BaseDP;
            
            // Color Features (Static Tensor)
            foreach (CardColor cVal in Enum.GetValues(typeof(CardColor)))
            {
                features[$"Is_{cVal}"] = card.Colors.Contains(cVal) ? 1 : 0;
            }

            return features;
        }

        public static void ExportFeatures(string outputPath, List<Card> allCards)
        {
            var exportData = new Dictionary<int, Dictionary<string, int>>();

            foreach (var card in allCards)
            {
                int id = CardRegistry.GetId(card.Id);
                // If id is 0, it means it wasn't in the registry. 
                // We proceed with 0 or log warning? For now proceed with 0 as it's the "Padding/Unknown" ID.

                exportData[id] = ExtractFeatures(card);
            }

            string json = JsonSerializer.Serialize(exportData, _jsonOptions);
            File.WriteAllText(outputPath, json);
        }
    }
}
