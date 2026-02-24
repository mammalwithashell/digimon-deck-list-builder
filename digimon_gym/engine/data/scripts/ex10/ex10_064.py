from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_064(CardScript):
    """EX10-064 Yuu Amano & Nene Amano"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-064 Also treated as [Yuu Amano]")
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

        # Timing: EffectTiming.None
        # Also Treated As
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-064 Also treated as [Nene Amano]")
        effect1.set_effect_description("Also Treated As")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By placing 1 [Bagra Army] or [Twilight] trait Digimon card from your hand or trash under this Tamer, <Draw 1>.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-064 Place 1 card under this Tamer from hand or trash to Draw 1")
        effect2.set_effect_description("[Start of Your Main Phase] By placing 1 [Bagra Army] or [Twilight] trait Digimon card from your hand or trash under this Tamer, <Draw 1>.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.BeforePayCost
        # [All Turns] When any of your [Bagra Army] or [Twilight] trait Digimon cards with DigiXros requirements would be played, by suspending this Tamer, 1 card from under your Tamers and 1 card in your trash can also be placed for their DigiXros.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-064 Can select DigiXros cards from Tamer's digivolution cards from trash")
        effect3.set_effect_description("[All Turns] When any of your [Bagra Army] or [Twilight] trait Digimon cards with DigiXros requirements would be played, by suspending this Tamer, 1 card from under your Tamers and 1 card in your trash can also be placed for their DigiXros.")
        effect3.is_optional = True
        effect3.set_hash_string("CanSelectDigiXrosFromTamer_EX10_064")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: security_play
        # Security: Play this card
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-064 Security: Play this card")
        effect4.set_effect_description("Security: Play this card")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
