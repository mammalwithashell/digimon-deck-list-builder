using UnityEngine;
using System.Collections;
using System.Collections.Generic;

// Headless replacement for ContinuousController to mute audio and misc overhead
public class HeadlessContinuousController : ContinuousController
{
    // Assuming ContinuousController has a Singleton 'instance' field we need to populate
    // or we rely on the base class Awake.
    // If base.Awake() sets 'instance', we are good.
    // If we need to override methods that play sound or update UI:

    public override void PlaySE(AudioClip clip)
    {
        // No Audio in Headless
    }

    // Add other overrides as needed based on ContinuousController API
}

// Headless replacement for Effects to strip animation delays
public class HeadlessEffects : Effects
{
    // Override visual effect coroutines to return immediately

    public override IEnumerator ShowCardEffect(List<CardSource> cards, string name, bool b1, bool b2)
    {
        yield break;
    }

    public override IEnumerator MoveToExecuteCardEffect(CardSource card)
    {
        yield break;
    }

    public override IEnumerator DeleteHandCardEffectCoroutine(CardSource card)
    {
        yield break;
    }

    public override IEnumerator ShowUseHandCardEffect_PlayCard(CardSource card)
    {
        yield break;
    }

    public override IEnumerator ShrinkUpUseHandCard(object obj)
    {
        // Signature guessed from usage: ShrinkUpUseHandCard(GManager.instance.GetComponent<Effects>().ShowUseHandCard)
        yield break;
    }

    public override IEnumerator FailedPlayCardEffect(CardSource card)
    {
        yield break;
    }

    public override IEnumerator RemoveDigivolveRootEffect(CardSource card, Permanent permanent)
    {
        yield break;
    }

    public override IEnumerator CreateFieldPermanentCardEffect(object showingCard, bool HasETB = false, bool isDigiXros = false, CardSource[] jogressEvoRoots = null)
    {
        yield break;
    }

    public override IEnumerator DigivolveFieldPermanentCardEffect(object showingCard, bool burst, bool blast, bool appFusion)
    {
        yield break;
    }

    public override IEnumerator CreateRecoveryEffect(Player player)
    {
        yield break;
    }

    public override IEnumerator DeckBounceEffect(Permanent permanent)
    {
        yield break;
    }

    public override IEnumerator DestroyPermanentEffect(Permanent permanent)
    {
        yield break;
    }

    public override IEnumerator BreakSecurityEffect(Player player)
    {
        yield break;
    }

    public override IEnumerator EnterSecurityCardEffect(CardSource card)
    {
        yield break;
    }

    public override IEnumerator DestroySecurityEffect(CardSource card)
    {
        yield break;
    }

    public override IEnumerator BattleEffect(List<Permanent> winners, List<Permanent> losers, CardSource loserCard)
    {
        yield break;
    }

    public override IEnumerator CreateDebuffEffect(Permanent permanent)
    {
        yield break;
    }

    public override IEnumerator BounceEffect(Permanent permanent, bool toHand)
    {
        yield break;
    }
}
