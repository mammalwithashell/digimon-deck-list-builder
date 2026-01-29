using System.Collections.Generic;
using UnityEngine;

public class CEntity_EffectController : MonoBehaviour
{
    public CEntity_Effect cEntity_Effect { get; set; }

    public List<ICardEffect> GetCardEffects(EffectTiming timing, CardSource card)
    {
        if (cEntity_Effect != null)
        {
            return cEntity_Effect.GetCardEffects(timing, card);
        }
        return new List<ICardEffect>();
    }
}
