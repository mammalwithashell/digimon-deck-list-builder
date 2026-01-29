using System.Collections;
using System.Collections.Generic;
using UnityEngine;

public class Effects : MonoBehaviour
{
    public virtual IEnumerator ShowCardEffect(List<CardSource> cards, string name, bool b1, bool b2) { yield break; }
    public virtual IEnumerator MoveToExecuteCardEffect(CardSource card) { yield break; }
    public virtual IEnumerator DeleteHandCardEffectCoroutine(CardSource card) { yield break; }
    public virtual IEnumerator ShowUseHandCardEffect_PlayCard(CardSource card) { yield break; }
    public virtual IEnumerator ShrinkUpUseHandCard(object obj) { yield break; }
    public virtual IEnumerator FailedPlayCardEffect(CardSource card) { yield break; }
    public virtual IEnumerator RemoveDigivolveRootEffect(CardSource card, Permanent permanent) { yield break; }
    public virtual IEnumerator CreateFieldPermanentCardEffect(object showingCard, bool HasETB = false, bool isDigiXros = false, CardSource[] jogressEvoRoots = null) { yield break; }
    public virtual IEnumerator DigivolveFieldPermanentCardEffect(object showingCard, bool burst, bool blast, bool appFusion) { yield break; }
    public virtual IEnumerator CreateRecoveryEffect(Player player) { yield break; }
    public virtual IEnumerator DeckBounceEffect(Permanent permanent) { yield break; }
    public virtual IEnumerator DestroyPermanentEffect(Permanent permanent) { yield break; }
    public virtual IEnumerator BreakSecurityEffect(Player player) { yield break; }
    public virtual IEnumerator EnterSecurityCardEffect(CardSource card) { yield break; }
    public virtual IEnumerator DestroySecurityEffect(CardSource card) { yield break; }
    public virtual IEnumerator BattleEffect(List<Permanent> winners, List<Permanent> losers, CardSource loserCard) { yield break; }
    public virtual IEnumerator CreateDebuffEffect(Permanent permanent) { yield break; }
    public virtual IEnumerator BounceEffect(Permanent permanent, bool toHand) { yield break; }
}
