from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_064(CardScript):
    """EX5-064 Koh & Sayo"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # [Start of Your Turn] Set memory to 3 if <= 2
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX5-064 Set memory to 3")
        effect0.set_effect_description("[Start of Your Turn] If your memory is at 2 or less, it becomes 3.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Set memory to 3 if <= 2"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By suspending this Tamer and placing the top top card of one of your [Light Fang]/[Night Claw] trait Digimon as that Digimon's bottom digivolution card, 1 of your Digimon may digivolve into a Digimon card in your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX5-064 Your Digimon digivolves")
        effect1.set_effect_description("[On Play] By suspending this Tamer and placing the top top card of one of your [Light Fang]/[Night Claw] trait Digimon as that Digimon's bottom digivolution card, 1 of your Digimon may digivolve into a Digimon card in your hand without paying the cost.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 1):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Light Fang' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('LightFung' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Night Claw' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('NightClaw' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] By suspending this Tamer and placing the top top card of one of your [Light Fang]/[Night Claw] trait Digimon as that Digimon's bottom digivolution card, 1 of your Digimon may digivolve into a Digimon card in your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("EX5-064 Your Digimon digivolves")
        effect2.set_effect_description("[Main] By suspending this Tamer and placing the top top card of one of your [Light Fang]/[Night Claw] trait Digimon as that Digimon's bottom digivolution card, 1 of your Digimon may digivolve into a Digimon card in your hand without paying the cost.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and len(permanent.digivolution_cards) >= 1):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend, Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Light Fang' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('LightFung' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Night Claw' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('NightClaw' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("EX5-064 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
