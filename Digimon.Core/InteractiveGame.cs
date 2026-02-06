using Digimon.Core.Loggers;

namespace Digimon.Core
{
    public class InteractiveGame : BaseGameRunner
    {
        public PlayerType Player1Type { get; private set; }
        public PlayerType Player2Type { get; private set; }

        public InteractiveGame(List<string> deck1Ids, List<string> deck2Ids, PlayerType player1Type, PlayerType player2Type)
            : base(deck1Ids, deck2Ids, new VerboseLogger())
        {
            Player1Type = player1Type;
            Player2Type = player2Type;
        }

        public string RunStep()
        {
            if (GameInstance.IsGameOver) return GameInstance.ToJson();

             // Check win condition (Deck out)
            if (GameInstance.CurrentPlayer.Deck.Count == 0)
            {
                GameInstance.EndGame(GameInstance.OpponentPlayer);
                return GameInstance.ToJson();
            }

            if (IsCurrentPlayerHuman())
            {
                // Do nothing, wait for external Step(actionId)
                // Return current state so UI can render
                return GameInstance.ToJson();
            }
            else
            {
                ExecuteAgentTurn();
                return GameInstance.ToJson();
            }
        }

        private bool IsCurrentPlayerHuman()
        {
            if (GameInstance.CurrentPlayer == GameInstance.Player1)
                return Player1Type == PlayerType.Human;
            else
                return Player2Type == PlayerType.Human;
        }

        public void Step(int actionId)
        {
            // Execute Action using Decoder (which logs to VerboseLogger)
            ActionDecoder.DecodeAndExecute(GameInstance, actionId);
        }
        
        public List<string> GetLastLog()
        {
            return GameInstance.Logger.GetLogs();
        }

        public void ClearLog()
        {
           GameInstance.Logger.Clear();
        }
    }
}
