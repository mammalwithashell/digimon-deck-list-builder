// Copy into Assets/Editor only in an approved DCGO project. Unity 2021.3.
// Default Execute performs validation only. -repairApply explicitly enables writes/build.
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace DCGO.CatalogRepair
{
    public static class EX13CatalogRepair
    {
        const string ScenePath = "Assets/Scenes/ContinuousControllerScene.unity";
        static readonly string[] IDs = {"EX13-005","EX13-008","EX13-018","EX13-021","EX13-024","EX13-039","EX13-041","EX13-044","EX13-045"};
        static readonly HashSet<string> Excluded = new HashSet<string> {"P-147","BT17-085_P3","BT15-082_P2","BT15-084_P2","BT14-085_P2","BT13-100_P2","BT11-086_P2","BT10-042_P5"};
        [Serializable] public class Payload { public int schemaVersion; public string evidenceManifestSha256; public Card[] cards; }
        [Serializable] public class Card
        {
            public string CardID, CardSpriteName, CardEffectClassName, CardName_ENG;
            public List<CardColor> cardColors;
            public List<CardKind> cardKind;
            public int PlayCost, DP, Level, MaxCountInDeck, OverflowMemory, LinkDP;
            public Rarity rarity;
            public List<string> Form_ENG, Attribute_ENG, Type_ENG;
            public List<EvoCost> EvoCosts;
            public string EffectDiscription_ENG, InheritedEffectDiscription_ENG, SecurityEffectDiscription_ENG;
            public string LinkEffect, LinkRequirement, dualEffect, OptionEffect;
            public List<CardColor> OptionCardColorRequirements;
        }
        [Serializable] class Receipt
        {
            public string companyName, productName, expectedPersistentDataPath, evidenceManifestSha256;
            public string[] addedAssets;
            public int[] addedIndices;
            public bool buildSucceeded;
        }
        static string Arg(string key)
        {
            var args = Environment.GetCommandLineArgs();
            int i = Array.IndexOf(args, key);
            return i >= 0 && i + 1 < args.Length ? args[i + 1] : null;
        }
        static void Require(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        public static void Execute()
        {
            try { Run(); EditorApplication.Exit(0); }
            catch (Exception error) { Debug.LogException(error); EditorApplication.Exit(1); }
        }
        static void Run()
        {
            Require(Application.unityVersion.StartsWith("2021.3."), "Use Unity 2021.3");
            string payloadPath = Arg("-repairPayload");
            Require(!string.IsNullOrEmpty(payloadPath), "Missing -repairPayload");
            var payload = JsonUtility.FromJson<Payload>(File.ReadAllText(payloadPath));
            Require(payload.schemaVersion == 1 && payload.cards != null && payload.cards.Length == IDs.Length, "Invalid payload schema/count");
            Require(payload.cards.Select(c => c.CardID).OrderBy(x => x).SequenceEqual(IDs), "Exact nine-card pool required");
            Require(payload.evidenceManifestSha256 != null && payload.evidenceManifestSha256.Length == 64, "Missing evidence binding");
            var all = AssetDatabase.FindAssets("t:CEntity_Base", new[]{"Assets/CardBaseEntity"})
                .Select(g => AssetDatabase.LoadAssetAtPath<CEntity_Base>(AssetDatabase.GUIDToAssetPath(g))).ToList();
            Require(all.Count > 0 && all.All(c => c != null), "Existing catalog missing");
            Require(all.All(c => c.CardIndex >= 0) && all.Select(c => c.CardIndex).Distinct().Count() == all.Count, "Existing duplicate/negative indices need separate repair");
            int nextIndex = checked(all.Max(c => c.CardIndex) + 1);
            Require(nextIndex + IDs.Length <= 65536, "Deck-code index capacity exhausted");
            foreach (var c in payload.cards)
            {
                Require(!all.Any(a => a.CardSpriteName == c.CardID), "Target already exists; refusing overwrite: " + c.CardID);
                Require(c.CardSpriteName == c.CardID && c.CardEffectClassName == c.CardID.Replace('-', '_'), "Identity/class mismatch");
                Require(c.cardColors != null && c.cardColors.Count > 0 && c.cardKind != null && c.cardKind.Count == 1 && !string.IsNullOrEmpty(c.CardName_ENG), "Incomplete card metadata");
                Require(AppDomain.CurrentDomain.GetAssemblies().Any(a => {
                    var t = a.GetType("DCGO.CardEffects.EX13." + c.CardEffectClassName);
                    return t != null && typeof(CEntity_Effect).IsAssignableFrom(t);
                }), "Missing compiled card effect: " + c.CardEffectClassName);
                Require(!File.Exists("Assets/CardBaseEntity/EX13/" + c.CardEffectClassName + ".asset"), "Target path occupied");
            }
            string[] scenes = EditorBuildSettings.scenes.Where(s => s.enabled).Select(s => s.path).ToArray();
            Require(scenes.Contains(ScenePath), "ContinuousControllerScene is not enabled for build");
            var scene = EditorSceneManager.OpenScene(ScenePath, OpenSceneMode.Single);
            var controllers = scene.GetRootGameObjects().SelectMany(g => g.GetComponentsInChildren<ContinuousController>(true)).ToArray();
            Require(controllers.Length == 1, "Expected one ContinuousController in catalog scene");
            Debug.Log("EX13 catalog dry-run OK: append " + nextIndex + ".." + (nextIndex+8));
            if (!Environment.GetCommandLineArgs().Contains("-repairApply")) return;
            string output = Arg("-repairOutput");
            Require(!string.IsNullOrEmpty(output) && Path.IsPathRooted(output), "Provide absolute -repairOutput NEW directory");
            output = Path.GetFullPath(output);
            string project = Path.GetFullPath(Path.Combine(Application.dataPath, "..")) + Path.DirectorySeparatorChar;
            Require(!output.StartsWith(project, StringComparison.OrdinalIgnoreCase), "Build output must be outside project");
            Require(!Directory.Exists(output) && !File.Exists(output), "Output must not already exist");
            Directory.CreateDirectory(output);
            File.Copy(ScenePath, Path.Combine(output, "ContinuousControllerScene.unity.before"));
            File.Copy("ProjectSettings/ProjectSettings.asset", Path.Combine(output, "ProjectSettings.asset.before"));
            var receipt = new Receipt {
                companyName = "DCGO-CatalogRepair", productName = "EX13-" + Guid.NewGuid().ToString("N"),
                evidenceManifestSha256 = payload.evidenceManifestSha256,
                addedAssets = payload.cards.Select(c => "Assets/CardBaseEntity/EX13/" + c.CardEffectClassName + ".asset").ToArray(),
                addedIndices = Enumerable.Range(nextIndex, IDs.Length).ToArray()
            };
            receipt.expectedPersistentDataPath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), "AppData", "LocalLow", receipt.companyName, receipt.productName);
            string receiptPath = Path.Combine(output,"repair-receipt.json");
            File.WriteAllText(receiptPath, JsonUtility.ToJson(receipt,true));
            if (!AssetDatabase.IsValidFolder("Assets/CardBaseEntity/EX13")) AssetDatabase.CreateFolder("Assets/CardBaseEntity", "EX13");
            for (int i=0; i<payload.cards.Length; i++)
            {
                var entity = ScriptableObject.CreateInstance<CEntity_Base>();
                JsonUtility.FromJsonOverwrite(JsonUtility.ToJson(payload.cards[i]), entity);
                entity.CardIndex = nextIndex+i;
                entity.name = payload.cards[i].CardEffectClassName;
                AssetDatabase.CreateAsset(entity, receipt.addedAssets[i]);
                all.Add(entity);
            }
            AssetDatabase.SaveAssets();
            var visible = all.Where(c => !string.IsNullOrEmpty(c.CardSpriteName) && !Excluded.Contains(c.CardSpriteName)).ToList();
            controllers[0].CardList = DeckData.SortedCardPoolList(visible).ToArray();
            controllers[0].SortedCardList = visible.OrderBy(c => c.CardIndex).ToArray();
            EditorUtility.SetDirty(controllers[0]);
            EditorSceneManager.MarkSceneDirty(scene);
            Require(EditorSceneManager.SaveScene(scene), "Scene save failed");
            AssetDatabase.SaveAssets();
            string oldCompany = PlayerSettings.companyName, oldProduct = PlayerSettings.productName;
            var oldBackend = PlayerSettings.GetScriptingBackend(BuildTargetGroup.Standalone);
            BuildReport report;
            try
            {
                PlayerSettings.companyName = receipt.companyName;
                PlayerSettings.productName = receipt.productName;
                PlayerSettings.SetScriptingBackend(BuildTargetGroup.Standalone, ScriptingImplementation.Mono2x);
                report = BuildPipeline.BuildPlayer(new BuildPlayerOptions {
                    scenes=scenes, locationPathName=Path.Combine(output,"DCGO.exe"),
                    target=BuildTarget.StandaloneWindows64, options=BuildOptions.None
                });
            }
            finally
            {
                PlayerSettings.companyName = oldCompany;
                PlayerSettings.productName = oldProduct;
                PlayerSettings.SetScriptingBackend(BuildTargetGroup.Standalone,oldBackend);
                AssetDatabase.SaveAssets();
            }
            Require(report.summary.result == BuildResult.Succeeded, "Build failed; inspect logs and backup/receipt; no player was launched");
            receipt.buildSucceeded=true;
            File.WriteAllText(receiptPath,JsonUtility.ToJson(receipt,true));
            Debug.Log("EX13 isolated build complete: " + output + "; persistentDataPath=" + receipt.expectedPersistentDataPath);
        }
    }
}
