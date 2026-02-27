from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_092(CardScript):
    """BT15-092 Revelation of Light"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardSecurity
        # When an effect trashes this card from the security stack, activate this card's [Security] effect.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-092 Activate security effect")
        effect0.set_effect_description("When an effect trashes this card from the security stack, activate this card's [Security] effect.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Search your security stack. You may play 1 level 4 or lower yellow Digimon card among it without paying the cost. Then, shuffle your security stack. If you have a Tamer with [Kari Kamiya] in its name, place this card on top of your security stack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-092 Play 1 Digimon from security")
        effect1.set_effect_description("[Main] Search your security stack. You may play 1 level 4 or lower yellow Digimon card among it without paying the cost. Then, shuffle your security stack. If you have a Tamer with [Kari Kamiya] in its name, place this card on top of your security stack.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Add To Security, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if not ('Yellow' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Add top card of deck to security
            if player:
                player.recovery(1)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] All of your opponent's Digimon and all of your opponent's Security Digimon get -5000 DP until the end of your turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT15-092 Effect")
        effect2.set_effect_description("[Security] All of your opponent's Digimon and all of your opponent's Security Digimon get -5000 DP until the end of your turn.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
