from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_068(CardScript):
    """EX6-068 Descent of the Three Great Angels"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] You may place 1 Digimon card with the [Angel]/[Archangel]/[Three Great Angels] trait from your hand at the bottom of your security stack. Then, place this card in your battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-068 Add To Security")
        effect0.set_effect_description("[Main] You may place 1 Digimon card with the [Angel]/[Archangel]/[Three Great Angels] trait from your hand at the bottom of your security stack. Then, place this card in your battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-068 Delay")
        effect1.set_effect_description("Delay")
        effect1.is_on_deletion = True
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [All Turns] When one of your Digimon with the [Angel] or [Archangel] trait is deleted, <Delay>.\r\n• Search your security stack. You may play 1 Digimon card with the [Three Great Angels] trait among it without paying the cost. Shuffle your security stack.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-068 Play 1 card with the [Three Great Angels] trait from your security stack without paying the cost")
        effect2.set_effect_description("[All Turns] When one of your Digimon with the [Angel] or [Archangel] trait is deleted, <Delay>.\r\n• Search your security stack. You may play 1 Digimon card with the [Three Great Angels] trait among it without paying the cost. Shuffle your security stack.")
        effect2.is_optional = True
        effect2.set_hash_string("PlaySecurityCard_EX6_068")
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
