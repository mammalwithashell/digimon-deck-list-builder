from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_212(CardScript):
    """P-212 Asuna Shiroki"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: gain_memory_tamer
        # Gain 1 memory (Tamer)
        effect0 = ICardEffect()
        effect0.set_effect_name("P-212 Gain 1 memory (Tamer)")
        effect0.set_effect_description("Gain 1 memory (Tamer)")
        # [Start of Main] Gain 1 memory if opponent has Digimon

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Draw 1 and trash 1 card in your hand. If this effect trashed a card with the [Three Musketeers] or [TS] trait, delete 1 of your opponent's level 3 Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-212 Draw 1 and trash 1. if this trashed a [Three Musketeers]/[TS] card, delete 1 level 3 digimon")
        effect1.set_effect_description("[On Play] Draw 1 and trash 1 card in your hand. If this effect trashed a card with the [Three Musketeers] or [TS] trait, delete 1 of your opponent's level 3 Digimon.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("P-212 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
