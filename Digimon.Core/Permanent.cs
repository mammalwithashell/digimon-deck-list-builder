
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
    }
}
