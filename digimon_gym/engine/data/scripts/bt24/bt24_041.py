from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_041(CardScript):
    """BT24-041 Minervamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-041 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Beastkin] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_trait = "Beastkin"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Beastkin' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])) or any('Dark Dragon' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if you have an [Iliad] trait Digimon or Tamer, reduce the play cost by 5.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-041 Reduce play cost (5)")
        effect1.set_effect_description("When this card would be played, if you have an [Iliad] trait Digimon or Tamer, reduce the play cost by 5.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-041 Play Cost -5")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            pass

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-041 Effect")
        effect3.set_effect_description("Effect")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-041 Effect")
        effect4.set_effect_description("Effect")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnDestroyedAnyone
        # Effect
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-041 Effect")
        effect5.set_effect_description("Effect")
        effect5.is_on_deletion = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        # Factory effect: blocker
        # Blocker
        effect6 = ICardEffect()
        effect6.set_effect_name("BT24-041 Blocker")
        effect6.set_effect_description("Blocker")
        effect6._is_blocker = True

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Iliad' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        # Factory effect: reboot
        # Reboot
        effect7 = ICardEffect()
        effect7.set_effect_name("BT24-041 Reboot")
        effect7.set_effect_description("Reboot")
        effect7._is_reboot = True

        def condition7(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Iliad' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect7.set_can_use_condition(condition7)
        effects.append(effect7)

        return effects
