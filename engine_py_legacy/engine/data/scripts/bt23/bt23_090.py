from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_090(CardScript):
    """BT23-090 Keisuke Amasawa"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # [Start of Your Turn] Set memory to 3 if <= 2
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT23-090 Set memory to 3")
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

        # [All Turns] All of your Digimon with the [Hudie] trait get +1000 DP.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-090 [Hudie] trait Digimon DP modifier")
        effect1.set_effect_description("[All Turns] All of your Digimon with the [Hudie] trait get +1000 DP.")
        effect1.dp_modifier = 1000
        effect1._applies_to_all_own_digimon = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only apply to [Hudie] trait Digimon
            perm = context.get('permanent')
            if perm:
                traits = getattr(perm.top_card, 'card_traits', []) or []
                if not any('Hudie' in t for t in traits):
                    return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] By suspending this Tamer and returning 1 of your Digimon with the [Hudie] trait to the hand, you may play 1 Tamer card with the [CS] trait from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT23-090 By suspending this tamer & bouncing 1 [Hudie] digimon to hand, play 1 [CS] tamer in hand")
        effect2.set_effect_description("[End of Your Turn] By suspending this Tamer and returning 1 of your Digimon with the [Hudie] trait to the hand, you may play 1 Tamer card with the [CS] trait from your hand without paying the cost.")
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
            """Action: Suspend self, bounce 1 own [Hudie] Digimon to hand, play 1 [CS] Tamer free"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            # Suspend this Tamer
            perm.suspend()
            # Select 1 of your Digimon with [Hudie] trait to return to hand
            def hudie_filter(p):
                if p is perm:
                    return False  # Don't target self
                if not p.is_digimon:
                    return False
                traits = getattr(p.top_card, 'card_traits', []) or []
                return any('Hudie' in t for t in traits)

            def on_bounce(target_perm):
                # Return the Digimon to hand via engine API (top card to hand, sources to trash)
                player.bounce_permanent_to_hand(target_perm)
                # Then play 1 CS Tamer from hand free
                def play_filter(c):
                    if not getattr(c, 'is_tamer', False):
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    return any('CS' in t for t in traits)
                game.effect_play_from_zone(
                    player, 'hand', play_filter, free=True, is_optional=True)

            game.effect_select_own_permanent(
                player, on_bounce, filter_fn=hudie_filter, is_optional=True,
                prompt="Select 1 of your Digimon with [Hudie] trait to return to hand.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-090 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
