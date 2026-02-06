using System.ComponentModel;

namespace Digimon.Core.Constants
{
    public enum EffectTiming
    {
        [Description("On Play")]
        OnPlay,
        [Description("When Digivolving")]
        WhenDigivolving,
        [Description("On Deletion")]
        OnDeletion,
        [Description("End of Turn")]
        EndOfTurn,
        [Description("Start of Turn")]
        StartOfTurn,
        [Description("Your Turn")]
        YourTurn,
        [Description("Opponent's Turn")]
        OpponentsTurn,
        [Description("All Turns")]
        AllTurns,
        [Description("Security")]
        Security
    }
}
