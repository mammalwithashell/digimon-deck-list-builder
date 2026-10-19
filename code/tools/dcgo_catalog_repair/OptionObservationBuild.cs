using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEngine;

namespace DCGO.CatalogRepair
{
    // Build recorder-only instrumentation from the already isolated/repaired
    // project. Never modify card catalog entries or overwrite an old player.
    public static class OptionObservationBuild
    {
        [Serializable] class Receipt
        {
            public string companyName, productName, expectedPersistentDataPath;
            public bool buildSucceeded;
        }

        public static void Execute()
        {
            try
            {
                string[] args = Environment.GetCommandLineArgs();
                int index = Array.IndexOf(args, "-observationOutput");
                if (index < 0 || index + 1 >= args.Length)
                    throw new InvalidOperationException("Missing -observationOutput");
                string output = Path.GetFullPath(args[index + 1]);
                string project = Path.GetFullPath(Directory.GetCurrentDirectory()).TrimEnd(Path.DirectorySeparatorChar);
                if (Directory.Exists(output) || File.Exists(output) ||
                    output.Equals(project, StringComparison.OrdinalIgnoreCase) ||
                    output.StartsWith(project + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
                    throw new InvalidOperationException("Output must be new and outside the isolated project");
                var scenes = EditorBuildSettings.scenes.Where(s => s.enabled).Select(s => s.path).ToArray();
                if (scenes.Length == 0) throw new InvalidOperationException("No enabled scenes");
                Directory.CreateDirectory(output);
                File.Copy("ProjectSettings/ProjectSettings.asset", Path.Combine(output, "ProjectSettings.asset.before"));
                var receipt = new Receipt {
                    companyName = "DCGO-OptionObservation", productName = "Exam-" + Guid.NewGuid().ToString("N")
                };
                receipt.expectedPersistentDataPath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                    "AppData", "LocalLow", receipt.companyName, receipt.productName);
                string receiptPath = Path.Combine(output, "observation-receipt.json");
                File.WriteAllText(receiptPath, JsonUtility.ToJson(receipt, true));
                string oldCompany = PlayerSettings.companyName, oldProduct = PlayerSettings.productName;
                var oldBackend = PlayerSettings.GetScriptingBackend(BuildTargetGroup.Standalone);
                BuildReport report;
                try
                {
                    PlayerSettings.companyName = receipt.companyName;
                    PlayerSettings.productName = receipt.productName;
                    PlayerSettings.SetScriptingBackend(BuildTargetGroup.Standalone, ScriptingImplementation.Mono2x);
                    report = BuildPipeline.BuildPlayer(new BuildPlayerOptions {
                        scenes = scenes, locationPathName = Path.Combine(output, "DCGO.exe"),
                        target = BuildTarget.StandaloneWindows64, options = BuildOptions.None
                    });
                }
                finally
                {
                    PlayerSettings.companyName = oldCompany;
                    PlayerSettings.productName = oldProduct;
                    PlayerSettings.SetScriptingBackend(BuildTargetGroup.Standalone, oldBackend);
                    AssetDatabase.SaveAssets();
                }
                if (report.summary.result != BuildResult.Succeeded)
                    throw new InvalidOperationException("Build failed: " + report.summary.result);
                receipt.buildSucceeded = true;
                File.WriteAllText(receiptPath, JsonUtility.ToJson(receipt, true));
                Debug.Log("Option observation player built: " + output);
                EditorApplication.Exit(0);
            }
            catch (Exception e)
            {
                Debug.LogError(e.ToString());
                EditorApplication.Exit(1);
            }
        }
    }
}
