
using System.Collections.Generic;

namespace Digimon.Core
{
    public class Permanent
    {
        public int Id { get; set; } // Unique Per-Game ID?
        public Card TopCard { get; set; }
        public List<Card> Sources { get; set; } = [];
        public bool IsSuspended { get; set; }
        
        // Stats Cache
        public int CurrentDP { get; set; }
        // public List<ValidationState> Effects...

        public Permanent(Card card)
        {
            TopCard = card;
            Sources = [];
            IsSuspended = false;
            CurrentDP = card.BaseDP;
        }

        public void Suspend()
        {
            IsSuspended = true;
        }

        public void Unsuspend()
        {
            IsSuspended = false;
        }

        public bool HasUsedOpt { get; set; } = false;

        public void ResetTurnStats()
        {
            // Reset Once Per Turn flags at start of turn
            HasUsedOpt = false;
        }

        public bool HasKeyword(string keyword)
        {
            if (TopCard.Keywords.Contains(keyword)) return true;
            
            foreach(var source in Sources)
            {
                if (source.InheritedKeywords.Contains(keyword)) return true;
            }
            
            return false;
        }

        public void Digivolve(Card newCard)
        {
            Sources.Add(TopCard);
            TopCard = newCard;
            CurrentDP = newCard.BaseDP;
            // Note: Suspension status is inherited automatically as we don't change IsSuspended.
            // Temporary DP buffs (e.g. from Options this turn) might be lost if we reset CurrentDP?
            // Rules: "Effects applied to the Digimon continue to apply."
            // But base DP changes.
            // If we had +1000 DP buff, we should re-calculate.
            // Since we don't track buffs separately yet, setting to BaseDP is correct for MVP 
            // but effectively clears buffs (which is technically wrong but acceptable for this stage).
        }
    }
}
