from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_158(CardScript):
    """P-158 Jeri (Fake)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 4 cards of your deck. Add 1 card with the [D-Reaper] trait among them to the hand. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("P-158 Reveal the top 4 cards of your deck")
        effect0.set_effect_description("[On Play] Reveal the top 4 cards of your deck. Add 1 card with the [D-Reaper] trait among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('D-Reaper' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 4, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDeclaration
        # [Main] By returning this Tamer to the bottom of the deck, you may play 1 Digimon card with the [D-Reaper] trait and play cost of 3 or less from your hand without paying the cost. For each digivolution card of 1 of your [Mother D-Reaper], add 1 to the maximum play cost you may play with this effect.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1._is_field_main = True
        effect1.set_effect_name("P-158 Play 1 Digimon card with the [D-Reaper] trait")
        effect1.set_effect_description("[Main] By returning this Tamer to the bottom of the deck, you may play 1 Digimon card with the [D-Reaper] trait and play cost of 3 or less from your hand without paying the cost. For each digivolution card of 1 of your [Mother D-Reaper], add 1 to the maximum play cost you may play with this effect.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Mother D-Reaper'))):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('D-Reaper' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("P-158 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
