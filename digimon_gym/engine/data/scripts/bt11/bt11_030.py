from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_030(CardScript):
    """BT11-030 MetalGreymon + Cyber Launcher | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-030 Also treated as [MetalGreymon]/[Cyberdramon]")
        effect0.set_effect_description("Also Treated As")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: armor_purge
        # Armor Purge
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-030 Armor Purge")
        effect1.set_effect_description("Armor Purge")
        effect1._is_armor_purge = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may place 1 Digimon card with [Blue Flare] in its traits from your hand, or from under your Tamers, under this Digimon as its bottom digivolution card. Then, return 1 of your opponent's level 3 Digimon to the bottom of its owner's deck. If [Cyberdramon] is in this Digimon's digivolution cards, return 1 of your opponent's level 4 or lower Digimon to the bottom of its owner's deck.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT11-030 Place 1 card to digivolution cards and return Digimons to the bottom of deck")
        effect2.set_effect_description("[On Play] You may place 1 Digimon card with [Blue Flare] in its traits from your hand, or from under your Tamers, under this Digimon as its bottom digivolution card. Then, return 1 of your opponent's level 3 Digimon to the bottom of its owner's deck. If [Cyberdramon] is in this Digimon's digivolution cards, return 1 of your opponent's level 4 or lower Digimon to the bottom of its owner's deck.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 4:
                    return False
                if not (any('Blue Flare' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('BlueFlare' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 Digimon card with [Blue Flare] in its traits from your hand, or from under your Tamers, under this Digimon as its bottom digivolution card. Then, return 1 of your opponent's level 3 Digimon to the bottom of its owner's deck. If [Cyberdramon] is in this Digimon's digivolution cards, return 1 of your opponent's level 4 or lower Digimon to the bottom of its owner's deck.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT11-030 Place 1 card to digivolution cards and return Digimons to the bottom of deck")
        effect3.set_effect_description("[When Digivolving] You may place 1 Digimon card with [Blue Flare] in its traits from your hand, or from under your Tamers, under this Digimon as its bottom digivolution card. Then, return 1 of your opponent's level 3 Digimon to the bottom of its owner's deck. If [Cyberdramon] is in this Digimon's digivolution cards, return 1 of your opponent's level 4 or lower Digimon to the bottom of its owner's deck.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 4:
                    return False
                if not (any('Blue Flare' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('BlueFlare' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.None
        # Effect
        effect4 = ICardEffect()
        effect4.set_effect_name("BT11-030 Effect")
        effect4.set_effect_description("Effect")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
