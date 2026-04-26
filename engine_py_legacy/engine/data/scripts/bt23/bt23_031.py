from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_031(CardScript):
    """BT23-031 Angewomon | Lv.5

    Main:
      When this card would be played from the hand, if you have
      [LadyDevimon] or [Mirei Mikagura], reduce the play cost by 3.
      [On Play] [When Digivolving] Add your top security card to the hand.
      Then, if you have 3 or fewer security cards, <Recovery +1 (Deck)>.

    Inherited:
      <Alliance>

    Alt-digi (C# BT23_031.cs PermanentCondition):
      Lv.4 w/ [CS] trait for cost 3.  No color restriction — C# checks only
      HasLevel + IsLevel4 + HasCSTraits.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ──────────────────────────────────────────────────────────────
        # 1. Alternate digivolution requirement (factory, NoTiming)
        # ──────────────────────────────────────────────────────────────
        # C# condition: targetPermanent.TopCard.IsLevel4 && HasCSTraits.
        # No color restriction. Cost 3.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-031 Alternate digivolution requirement")
        effect0.set_effect_description(
            "Alternate digivolution: Lv.4 w/[CS] trait for cost 3")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "CS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ──────────────────────────────────────────────────────────────
        # 2. BeforePayCost — cost reduction by 3
        # ──────────────────────────────────────────────────────────────
        # "When this card would be played from the hand, if you have
        #  [LadyDevimon] or [Mirei Mikagura], reduce the play cost by 3."
        #
        # C# check (EqualsCardName — exact match):
        #   (permanent.IsDigimon && TopCard.EqualsCardName("LadyDevimon"))
        #   || (permanent.IsTamer  && TopCard.EqualsCardName("Mirei Mikagura"))
        #
        # Use EXACT name match (not contains) to avoid matching
        # "LadyDevimon (X Antibody)" which is a different card in DCGO.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT23-031 Play cost reduction -3")
        effect1.set_effect_description(
            "When this card would be played from the hand, if you have "
            "[LadyDevimon] or [Mirei Mikagura], reduce the play cost by 3.")
        effect1.set_hash_string("BT23_031_ReducePlayCost")
        effect1.cost_reduction = 3

        def condition1(context: Dict[str, Any]) -> bool:
            # Leak guard — only reduce cost for THIS card being played.
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if owner is None:
                return False
            # Mirrors C# IsExistOnHand — effect is for "from the hand" play.
            if card not in owner.hand_cards:
                return False
            for perm in owner.battle_area:
                top = perm.top_card
                if top is None:
                    continue
                names = getattr(top, 'card_names', []) or []
                # LadyDevimon (exact) on a Digimon permanent
                if perm.is_digimon and any(n == "LadyDevimon" for n in names):
                    return True
                # Mirei Mikagura (exact) on a Tamer permanent
                if perm.is_tamer and any(n == "Mirei Mikagura" for n in names):
                    return True
            return False

        effect1.set_can_use_condition(condition1)
        # No process callback — `cost_reduction` is applied directly by
        # `calculate_play_cost()` when the condition passes.
        effects.append(effect1)

        # ──────────────────────────────────────────────────────────────
        # 3. [On Play] — Add top security to hand, conditional Recovery +1
        # ──────────────────────────────────────────────────────────────
        # C# SharedActivateCoroutine:
        #   if (SecurityCards.Count > 0) { add top to hand; ReduceSecurity }
        #   if (SecurityCards.Count <= 3) Recovery(1)
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name(
            "BT23-031 [On Play] Add top security, then <Recovery +1> "
            "if 3 or fewer")
        effect2.set_effect_description(
            "[On Play] Add your top security card to the hand. Then, if "
            "you have 3 or fewer security cards, <Recovery +1 (Deck)>")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Must be on the field after being played.
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            _add_sec_and_recovery(ctx)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ──────────────────────────────────────────────────────────────
        # 4. [When Digivolving] — Same effect as [On Play]
        # ──────────────────────────────────────────────────────────────
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name(
            "BT23-031 [When Digivolving] Add top security, then <Recovery +1> "
            "if 3 or fewer")
        effect3.set_effect_description(
            "[When Digivolving] Add your top security card to the hand. "
            "Then, if you have 3 or fewer security cards, "
            "<Recovery +1 (Deck)>")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            _add_sec_and_recovery(ctx)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # ──────────────────────────────────────────────────────────────
        # 5. Inherited <Alliance>
        # ──────────────────────────────────────────────────────────────
        # BT23-031's ＜Alliance＞ is only in the inherited_text slot, so it
        # grants Alliance to Digimon digivolved on top of BT23-031 (not to
        # Angewomon herself when she is the top card).
        effect4 = ICardEffect()
        effect4.set_effect_name("BT23-031 Alliance")
        effect4.set_effect_description("<Alliance>")
        effect4.is_inherited_effect = True
        effect4.is_on_attack = True
        effect4._is_alliance = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects


def _add_sec_and_recovery(ctx: Dict[str, Any]) -> None:
    """Shared resolution for [On Play] / [When Digivolving]:

      1. If there is a top security card, move it to the owner's hand.
      2. Then, if security count is <= 3 (after step 1), Recovery +1.

    Step 2 runs whether or not step 1 added a card — this matches the C#
    SharedActivateCoroutine, which checks `SecurityCards.Count <= 3`
    after the optional add, so an empty security stack (count 0 <= 3)
    still triggers the Recovery.
    """
    player = ctx.get('player')
    if player is None:
        return
    # Step 1: move the top security card into hand, if one exists.
    if player.security_cards:
        top_sec = player.security_cards.pop(0)
        face_up = getattr(player, 'face_up_security', None)
        if face_up is not None:
            face_up.discard(top_sec)
        player.hand_cards.append(top_sec)
    # Step 2: Recovery +1 (Deck) if the security stack is now 3 or fewer.
    if len(player.security_cards) <= 3:
        player.recovery(1)
