from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_057(CardScript):
    """BT20-057 Gankoomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, if you have a Digimon with [Huckmon], [Jesmon], or [Sistermon] in its name, reduce the play cost by 4.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-057 Reduce the play cost by 4")
        effect0.set_effect_description("When this card would be played, if you have a Digimon with [Huckmon], [Jesmon], or [Sistermon] in its name, reduce the play cost by 4.")
        effect0.set_hash_string("PlayCost-4_BT20_057")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-057 Play Cost -4")
        effect1.set_effect_description("Effect")

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

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-057 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: reboot
        # Reboot
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-057 Reboot")
        effect3.set_effect_description("Reboot")
        effect3._is_reboot = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your Digimon may digivolve into a level 6 or lower Digimon card with [Huckmon] in its name or the [Royal Knight] trait in the hand or trash without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-057 Digivolve into level 6")
        effect4.set_effect_description("[On Play] 1 of your Digimon may digivolve into a level 6 or lower Digimon card with [Huckmon] in its name or the [Royal Knight] trait in the hand or trash without paying the cost.")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not (any('Huckmon' in _n for _n in getattr(c, 'card_names', [])) or any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] 1 of your Digimon may digivolve into a level 6 or lower Digimon card with [Huckmon] in its name or the [Royal Knight] trait in the hand or trash without paying the cost.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-057 Digivolve into level 6")
        effect5.set_effect_description("[When Digivolving] 1 of your Digimon may digivolve into a level 6 or lower Digimon card with [Huckmon] in its name or the [Royal Knight] trait in the hand or trash without paying the cost.")
        effect5.is_when_digivolving = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not (any('Huckmon' in _n for _n in getattr(c, 'card_names', [])) or any('Royal Knight' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
