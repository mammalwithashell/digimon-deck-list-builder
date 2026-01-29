using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using UnityEngine;

public class HeadlessGameManager : MonoBehaviour
{
    public static HeadlessGameManager Instance { get; private set; }

    [Header("Configuration")]
    public string Deck1Path;
    public string Deck2Path;
    public string OutputPath = "match_result.json";

    private void Awake()
    {
        if (Instance == null) Instance = this;
        else Destroy(gameObject);

        // Ensure we run in background
        Application.runInBackground = true;

        // If in batch mode, initialize headless logic
        if (Application.isBatchMode)
        {
            Debug.Log("[Headless] Starting Headless Mode...");
            StartCoroutine(InitializeHeadless());
        }
    }

    private IEnumerator InitializeHeadless()
    {
        // 1. Parse Command Line Arguments
        ParseArgs();

        // 2. Setup Environment (Strip UI, Fast Forward)
        SetupEnvironment();

        // 3. Load Decks
        List<string> deck1 = LoadDeck(Deck1Path);
        List<string> deck2 = LoadDeck(Deck2Path);

        if (deck1 == null || deck2 == null)
        {
            Debug.LogError("[Headless] Failed to load decks. Aborting.");
            Application.Quit();
            yield break;
        }

        Debug.Log($"[Headless] Decks loaded. Player 1: {deck1.Count} cards, Player 2: {deck2.Count} cards.");

        // 4. Setup Game
        // Ensure GManager exists and is initialized
        if (GManager.Instance == null)
        {
            InitializeGManager();
        }

        // Wait for GManager to be fully ready if needed
        while (GManager.Instance == null)
        {
            yield return null;
        }

        // Setup Players and Agent
        SetupGame(deck1, deck2);

        // 5. Run Game
        Debug.Log("[Headless] Starting Game Loop...");
        yield return StartCoroutine(GameLoop());
    }

    private void ParseArgs()
    {
        string[] args = System.Environment.GetCommandLineArgs();
        for (int i = 0; i < args.Length; i++)
        {
            if (args[i] == "--deck1" && i + 1 < args.Length) Deck1Path = args[i + 1];
            if (args[i] == "--deck2" && i + 1 < args.Length) Deck2Path = args[i + 1];
            if (args[i] == "--output" && i + 1 < args.Length) OutputPath = args[i + 1];
        }

        Debug.Log($"[Headless] Args Parsed: Deck1={Deck1Path}, Deck2={Deck2Path}, Output={OutputPath}");
    }

    private void SetupEnvironment()
    {
        // Maximize simulation speed
        Time.timeScale = 100.0f;
        Application.targetFrameRate = -1; // Unlimited FPS

        // Replace ContinuousController if possible
        if (ContinuousController.instance != null)
        {
            Destroy(ContinuousController.instance);
        }

        // Add Headless Controller
        GameObject controllerObj = new GameObject("HeadlessContinuousController");
        controllerObj.AddComponent<HeadlessContinuousController>();

        SetupHeadlessGManager();

        // Disable Audio
        AudioListener.pause = true;
        AudioListener.volume = 0;
    }

    private void InitializeGManager()
    {
        GameObject gManagerObj = new GameObject("GManager");
        gManagerObj.AddComponent<GManager>();
        // Also need TurnStateMachine
        gManagerObj.AddComponent<TurnStateMachine>();

        // Initialize basics
        GManager.Instance.TurnStateMachine = gManagerObj.GetComponent<TurnStateMachine>();
        GManager.Instance.TurnStateMachine.gameContext = new GameContext();
        GManager.Instance.TurnStateMachine.gameContext.Players = new Player[2];

        // Create Players
        GameObject player1Obj = new GameObject("Player1");
        Player player1 = player1Obj.AddComponent<Player>();
        player1.PlayerID = 0;

        GameObject player2Obj = new GameObject("Player2");
        Player player2 = player2Obj.AddComponent<Player>();
        player2.PlayerID = 1;

        GManager.Instance.You = player1;
        GManager.Instance.Opponent = player2;
        GManager.Instance.TurnStateMachine.gameContext.Players[0] = player1;
        GManager.Instance.TurnStateMachine.gameContext.Players[1] = player2;
    }

    private void SetupHeadlessGManager()
    {
        if (GManager.Instance != null)
        {
            var effects = GManager.Instance.GetComponent<Effects>();
            if (effects != null) Destroy(effects);
            if (GManager.Instance.GetComponent<HeadlessEffects>() == null)
            {
                GManager.Instance.gameObject.AddComponent<HeadlessEffects>();
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
        // Initialize MCTS Agent for Player 2 (or 1?)
        // Assuming Player 1 is the "Evolving Agent" (MCTS) and Player 2 is the Meta Deck (Script/Random)
        // Or vice-versa.

        // Hook into GManager to set up the game
        // This part relies on specific GManager API which we don't have fully.
        // We will invoke a setup method if available, or manually inject.

        // Example injection:
        // GManager.instance.LoadDecks(deck1, deck2);
        // GManager.instance.StartGame();

        // For the purpose of this task, we assume the Agent is attached here.
        MCTSAgent agent = gameObject.AddComponent<MCTSAgent>();
        agent.Initialize(GManager.Instance);
    }

    private IEnumerator GameLoop()
    {
        // Wait for game end
        while (GManager.Instance.TurnStateMachine != null && !GManager.Instance.TurnStateMachine.endGame)
        {
            yield return null;
        }

        // Report Results
        ReportResult();

        Application.Quit();
    }

    private void ReportResult()
    {
        // Collect stats
        bool player1Won = false; // Retrieve from GManager
        int turnCount = GManager.Instance.TurnStateMachine.TurnCount;

        // Find winner
        foreach(var player in GManager.Instance.TurnStateMachine.gameContext.Players)
        {
            if (!player.IsLose)
            {
                // If this player didn't lose, and the game ended, they likely won.
                // Or check IsWin if available.
                if (player.PlayerID == 0) player1Won = true; // Assuming ID 0 is Player 1
            }
        }

        string resultJson = $"{{\"win\": {player1Won.ToString().ToLower()}, \"turns\": {turnCount}}}";

        if (!string.IsNullOrEmpty(OutputPath))
        {
            File.WriteAllText(OutputPath, resultJson);
        }

        Console.WriteLine($"[Headless] Result: {resultJson}");
    }
}
