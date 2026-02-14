from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_098(CardScript):
    """BT20-098 Apparition Legion"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] By returning 9 levels' total worth of Digimon cards from your opponent's trash to the bottom of the deck, you may play 1 [Ghost] trait Digimon card of each returned card's level from your trash without paying the costs. Then, the Digimon this effect played gain <Rush> and <Blocker> until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-098 Play Card, Return To Deck")
        effect0.set_effect_description("[Main] By returning 9 levels' total worth of Digimon cards from your opponent's trash to the bottom of the deck, you may play 1 [Ghost] trait Digimon card of each returned card's level from your trash without paying the costs. Then, the Digimon this effect played gain <Rush> and <Blocker> until the end of your opponent's turn.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 level 5 or lower Digimon card with the [Ghost] trait from your trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-098 Play Card")
        effect1.set_effect_description("[Security] You may play 1 level 5 or lower Digimon card with the [Ghost] trait from your trash without paying the cost.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
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
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
