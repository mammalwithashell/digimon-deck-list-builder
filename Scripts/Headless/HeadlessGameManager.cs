using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using UnityEngine;
using UnityEngine.SceneManagement;

public class HeadlessGameManager : MonoBehaviour
{
    public static HeadlessGameManager Instance { get; private set; }

    [Header("Configuration")]
    public string Deck1Path;
    public string Deck2Path;
    public string OutputPath = "match_result.json";
    public int Matches = 1;
    public int HeadedMatches = 0;
    public string ScreenshotDir = "Screenshots";

    private void Awake()
    {
        if (Instance == null)
        {
            Instance = this;
            DontDestroyOnLoad(gameObject);

            // Ensure we run in background
            Application.runInBackground = true;

            // If in batch mode or arguments suggest headless run, start session
            // We check args here because we need to know if we should take control
            ParseArgs();

            if (Application.isBatchMode || Matches > 1 || HeadedMatches > 0)
            {
                Debug.Log("[Headless] Starting Session...");
                StartCoroutine(SessionLoop());
            }
        }
        else
        {
            Destroy(gameObject);
        }
    }

    private IEnumerator SessionLoop()
    {
        // Ensure Screenshot dir exists if needed
        if (HeadedMatches > 0 && !Directory.Exists(ScreenshotDir))
        {
            Directory.CreateDirectory(ScreenshotDir);
        }

        // Load Decks once (assuming they don't change between matches)
        List<string> deck1 = LoadDeck(Deck1Path);
        List<string> deck2 = LoadDeck(Deck2Path);

        if (deck1 == null || deck2 == null)
        {
            Debug.LogError("[Headless] Failed to load decks. Aborting.");
            Application.Quit();
            yield break;
        }

        Debug.Log($"[Headless] Decks loaded. Player 1: {deck1.Count} cards, Player 2: {deck2.Count} cards.");

        for (int i = 0; i < Matches; i++)
        {
            bool isHeaded = i < HeadedMatches;
            Debug.Log($"[Headless] Starting Match {i + 1}/{Matches} (Headed: {isHeaded})");

            // Wait for GManager
            while (GManager.instance == null) yield return null;

            // Setup Environment
            SetupEnvironment(isHeaded);

            // Setup Game
            SetupGame(deck1, deck2);

            // Wait for Game End
            while (GManager.instance.turnStateMachine != null && !GManager.instance.turnStateMachine.endGame)
            {
                yield return null;
            }

            // Screenshot if headed
            if (isHeaded)
            {
                // Wait end of frame to ensure UI is rendered?
                yield return new WaitForEndOfFrame();
                string path = Path.Combine(ScreenshotDir, $"Match_{i + 1}_{DateTime.Now:yyyyMMdd_HHmmss}.png");
                ScreenCapture.CaptureScreenshot(path);
                Debug.Log($"[Headless] Screenshot saved to {path}");
            }

            // Report Results
            ReportResult(i);

            // If not the last match, reload scene
            if (i < Matches - 1)
            {
                Debug.Log("[Headless] Reloading Scene...");
                SceneManager.LoadScene(SceneManager.GetActiveScene().name);
                yield return null; // Wait for scene load to initiate
            }
        }

        Debug.Log("[Headless] Session Complete.");
        Application.Quit();
    }

    private void ParseArgs()
    {
        string[] args = System.Environment.GetCommandLineArgs();
        for (int i = 0; i < args.Length; i++)
        {
            if (args[i] == "--deck1" && i + 1 < args.Length) Deck1Path = args[i + 1];
            if (args[i] == "--deck2" && i + 1 < args.Length) Deck2Path = args[i + 1];
            if (args[i] == "--output" && i + 1 < args.Length) OutputPath = args[i + 1];
            if (args[i] == "--matches" && i + 1 < args.Length) int.TryParse(args[i + 1], out Matches);
            if (args[i] == "--headed-matches" && i + 1 < args.Length) int.TryParse(args[i + 1], out HeadedMatches);
            if (args[i] == "--screenshot-dir" && i + 1 < args.Length) ScreenshotDir = args[i + 1];
        }

        Debug.Log($"[Headless] Args Parsed: Deck1={Deck1Path}, Deck2={Deck2Path}, Output={OutputPath}, Matches={Matches}, HeadedMatches={HeadedMatches}");
    }

    private void SetupEnvironment(bool isHeaded)
    {
        if (!isHeaded)
        {
            // Maximize simulation speed
            Time.timeScale = 100.0f;
            Application.targetFrameRate = -1; // Unlimited FPS

            // Replace ContinuousController if possible
            if (ContinuousController.instance != null)
            {
                Destroy(ContinuousController.instance);
            }

            // Add Headless Controller if not present (Scene reload destroys it, so this adds it back)
            // But if we are persistent, we might already have it?
            // ContinuousController is usually a scene object.
            // If we created a NEW GameObject for HeadlessContinuousController, it was destroyed on scene load.
            GameObject controllerObj = new GameObject("HeadlessContinuousController");
            controllerObj.AddComponent<HeadlessContinuousController>();

            // Disable Audio
            AudioListener.pause = true;
            AudioListener.volume = 0;

            // Replace Effects with HeadlessEffects
            SetupHeadlessGManager();
        }
        else
        {
            // Headed Mode
            Time.timeScale = 2.0f; // Fast but visible
            Application.targetFrameRate = 60;

            // Do NOT destroy ContinuousController or Effects

            // Enable Audio
            AudioListener.pause = false;
            AudioListener.volume = 1;
        }
    }

    private void SetupHeadlessGManager()
    {
        if (GManager.instance != null)
        {
            var effects = GManager.instance.GetComponent<Effects>();
            if (effects != null) Destroy(effects);

            // Only add HeadlessEffects if not already there (though Destroy above should handle replacement)
            if (GManager.instance.GetComponent<HeadlessEffects>() == null)
            {
                GManager.instance.gameObject.AddComponent<HeadlessEffects>();
            }
        }
    }

    private List<string> LoadDeck(string path)
    {
        if (string.IsNullOrEmpty(path)) return new List<string>();

        if (!File.Exists(path))
        {
            // If path is not a file, maybe it's the raw JSON string?
            if (path.StartsWith("["))
            {
                return ParseDeckJson(path);
            }
            Debug.LogError($"[Headless] Deck file not found: {path}");
            return null;
        }

        try
        {
            string json = File.ReadAllText(path);
            return ParseDeckJson(json);
        }
        catch (Exception e)
        {
            Debug.LogError($"[Headless] Error reading deck file: {e.Message}");
            return null;
        }
    }

    private List<string> ParseDeckJson(string json)
    {
        // Simple parser for ["Name","ID","ID"...]
        // Remove brackets
        json = json.Trim().Trim('[', ']');

        // Split by comma
        string[] parts = json.Split(',');

        List<string> deck = new List<string>();
        foreach (string part in parts)
        {
            // Remove quotes
            string clean = part.Trim().Trim('"');
            // Skip empty or metadata
            if (!string.IsNullOrWhiteSpace(clean) && !clean.StartsWith("Exported from"))
            {
                deck.Add(clean);
            }
        }
        return deck;
    }

    private void SetupGame(List<string> deck1, List<string> deck2)
    {
        // Hook into GManager to set up the game
        // This part relies on specific GManager API which we don't have fully.
        // We will invoke a setup method if available, or manually inject.

        // For the purpose of this task, we assume the Agent is attached here.

        // Clean up previous agent if any
        var existingAgent = GetComponent<MCTSAgent>();
        if (existingAgent != null) Destroy(existingAgent);

        MCTSAgent agent = gameObject.AddComponent<MCTSAgent>();
        agent.Initialize(GManager.instance);
    }

    // Removed old InitializeHeadless as it's replaced by SessionLoop

    // Removed old GameLoop as it's integrated into SessionLoop logic

    private void ReportResult(int matchIndex)
    {
        // Collect stats
        bool player1Won = false; // Retrieve from GManager
        int turnCount = GManager.instance.turnStateMachine.TurnCount;

        // Find winner
        foreach(var player in GManager.instance.turnStateMachine.gameContext.Players)
        {
            if (!player.IsLose)
            {
                // If this player didn't lose, and the game ended, they likely won.
                // Or check IsWin if available.
                if (player.PlayerID == 0) player1Won = true; // Assuming ID 0 is Player 1
            }
        }

        string resultJson = $"{{\"match\": {matchIndex}, \"win\": {player1Won.ToString().ToLower()}, \"turns\": {turnCount}}}";

        // Append to output file or write separate files?
        // Current requirement: "match_result.json".
        // Maybe we append lines?
        if (!string.IsNullOrEmpty(OutputPath))
        {
            File.AppendAllText(OutputPath, resultJson + "\n");
        }

        Console.WriteLine($"[Headless] Result: {resultJson}");
    }
}
