from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_094(CardScript):
    """BT22-094 Yuugo Kamishiro"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [CS] trait among them to the hand. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT22-094 Reveal top 3, add 1 [CS] card to hand, bottom deck the rest")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [CS] trait among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add 1 CS card to hand, bottom deck the rest"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def reveal_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('CS' in t for t in traits)
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # [Your Turn] When any of your Digimon or Tamers with the [CS] trait would be played, by returning this Tamer to the bottom of the deck, reduce the play cost by 2.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT22-094 Bottom deck this tamer, reduce play cost by 2")
        effect1.set_effect_description("[Your Turn] When any of your Digimon or Tamers with the [CS] trait would be played, by returning this Tamer to the bottom of the deck, reduce the play cost by 2.")
        effect1.is_optional = True
        effect1.cost_reduction = 2

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            # Only applies when playing a CS-trait Digimon or Tamer
            target_card = context.get('card_to_play')
            if target_card:
                traits = getattr(target_card, 'card_traits', []) or []
                if not any('CS' in t for t in traits):
                    return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Return this Tamer to deck bottom, reduce play cost by 2"""
            player = ctx.get('player')
            game = ctx.get('game')
            # Return this Tamer to bottom of deck as cost (use proper engine API)
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and player:
                player.return_permanent_to_deck_bottom(tamer_perm)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-094 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
