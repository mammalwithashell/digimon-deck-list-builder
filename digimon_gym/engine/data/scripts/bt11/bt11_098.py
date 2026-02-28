from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_098(CardScript):
    """BT11-098 Maelstrom"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] You may play 1 blue Digimon card from one of your blue Digimon's digivolution cards without paying the cost. Then, if you have a Digimon with [Seadramon] in its name in play, return 1 of your opponent's level 4 or lower Digimon to the bottom of its owner's deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-098 Play Card, Bounce")
        effect0.set_effect_description("[Main] You may play 1 blue Digimon card from one of your blue Digimon's digivolution cards without paying the cost. Then, if you have a Digimon with [Seadramon] in its name in play, return 1 of your opponent's level 4 or lower Digimon to the bottom of its owner's deck.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Bounce"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not ('Blue' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 4:
                    return False
                if not (p.contains_card_name('Seadramon')):
                    return False
                return True
            def on_bounce(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)
            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-098 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
