from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_089(CardScript):
    """BT22-089 Mirei Mikagura"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 play cost 4 or higher [Mirei Mikagura] or play cost 4 or higher Tamer card with the [CS] trait from your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-089 By bottom decking, Play 1 4 cost or higher [Mirei Mikagura] or [CS] tamer from hand")
        effect0.set_effect_description("[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 play cost 4 or higher [Mirei Mikagura] or play cost 4 or higher Tamer card with the [CS] trait from your hand without paying the cost.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) < 4:
                    return False
                if not (any('Mirei Mikagura' in _n for _n in getattr(c, 'card_names', [])) or any('CS' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing 1 card with the [Holy Beast], [Angel], [Archangel], [Fallen Angel] or [CS] trait from your hand, <Draw 2> (Draw 2 cards from your deck.)
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-089 By trashing 1 [Holy Beast]/[Angel]/[Archangel]/[Fallen Angel]/[CS], Draw 2")
        effect1.set_effect_description("[On Play] By trashing 1 card with the [Holy Beast], [Angel], [Archangel], [Fallen Angel] or [CS] trait from your hand, <Draw 2> (Draw 2 cards from your deck.)")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 2"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(2)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-089 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
