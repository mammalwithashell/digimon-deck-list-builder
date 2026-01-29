using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Events;

public abstract class ICardEffect
{
    public CardSource EffectSourceCard { get; set; }
    public Permanent EffectSourcePermanent { get; set; }
    public int MaxCountPerTurn { get; set; }
    public string EffectName { get; set; }
    public string EffectDiscription { get; set; }
    public string HashString { get; set; }
    public UnityAction OnProcessCallbuck { get; set; }
    public ICardEffect RootCardEffect { get; set; }
    public Func<Hashtable, bool> CanUseCondition { get; set; }
    public Func<Hashtable, bool> CanActivateCondition { get; set; }
    public bool IsOptional { get; set; }
    public bool UseOptional { get; set; }
    public bool IsDeclarative { get; set; }
    public bool IsInheritedEffect { get; set; }
    public bool IsLinkedEffect { get; set; }
    public bool IsSecurityEffect { get; set; }
    public bool IsCounterEffect { get; set; }
    public bool IsDigimonEffect { get; set; }
    public bool IsTamerEffect { get; set; }
    public int ChainActivations { get; set; }
    public bool IsBackgroundProcess { get; set; }
    public bool IsNotShowUI { get; set; }
    public virtual bool IsDisabled => false;
    public virtual bool IsOnPlay => false;
    public virtual bool IsWhenDigivolving => false;
    public virtual bool IsOnDeletion => false;
    public virtual bool IsOnAttack => false;

    public void SetUpICardEffect(string effectName, Func<Hashtable, bool> canUseCondition, CardSource card)
    {
        EffectName = effectName;
        CanUseCondition = canUseCondition;
        EffectSourceCard = card;
    }

    public void SetEffectSourceCard(CardSource effectSourceCard) => EffectSourceCard = effectSourceCard;
    public void SetEffectSourcePermanent(Permanent effectSourcePermanent) => EffectSourcePermanent = effectSourcePermanent;
    public void SetMaxCountPerTurn(int maxCountPerTurn) => MaxCountPerTurn = maxCountPerTurn;
    public void SetEffectName(string effectName) => EffectName = effectName;
    public void SetEffectDiscription(string effectDiscription) => EffectDiscription = effectDiscription;
    public void SetHashString(string hashString) => HashString = hashString;
    public void SetOnProcessCallbuck(UnityAction onProcessCallbuck) => OnProcessCallbuck = onProcessCallbuck;
    public void SetRootCardEffect(ICardEffect rootCardEffect) => RootCardEffect = rootCardEffect;
    public void SetCanUseCondition(Func<Hashtable, bool> canUseCondition) => CanUseCondition = canUseCondition;
    public void SetCanActivateCondition(Func<Hashtable, bool> canActivateCondition) => CanActivateCondition = canActivateCondition;
    public virtual bool CanTrigger(Hashtable hashtable) => true;
    public virtual bool CanActivate(Hashtable hashtable) => true;
    public virtual bool CanUse(Hashtable hashtable) => true;
    public void SetIsOptional(bool isOptional) => IsOptional = isOptional;
    public void SetUseOptional(bool useOptional) => UseOptional = useOptional;
    public void SetIsDeclarative(bool isDeclarative) => IsDeclarative = isDeclarative;
    public void SetIsInheritedEffect(bool isInheritedEffect) => IsInheritedEffect = isInheritedEffect;
    public void SetIsLinkedEffect(bool isLinkedEffect) => IsLinkedEffect = isLinkedEffect;
    public void SetIsSecurityEffect(bool isSecurityEffect) => IsSecurityEffect = isSecurityEffect;
    public void SetIsCounterEffect(bool isCounterEffect) => IsCounterEffect = isCounterEffect;
    public void SetIsDigimonEffect(bool isDigimonEffect) => IsDigimonEffect = isDigimonEffect;
    public void SetIsTamerEffect(bool isTamerEffect) => IsTamerEffect = isTamerEffect;
    public void SetChainActivationCount(int chainActivations) => ChainActivations = chainActivations;
    public void SetIsBackgroundProcess(bool isBackgroundProcess) => IsBackgroundProcess = isBackgroundProcess;
    public void SetNotShowUI(bool isNotShowUI) => IsNotShowUI = isNotShowUI;

    public bool IsSameEffect(ICardEffect cardEffect)
    {
        if (cardEffect == null) return false;
        return this.HashString == cardEffect.HashString && this.RootCardEffect == cardEffect.RootCardEffect;
    }
}
