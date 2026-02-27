from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_063(CardScript):
    """BT16-063 Shakkoumon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-063 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] This Digimon isn't affected by the effects of your opponent's Digimon until the end of their turn. Then, if DNA digivolving, place 1 of your opponent's Digimon whose level is less than or equal to the number of cards in yours or your opponent's security stack at the bottom of your opponent's security stack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-063 Unaffected by effects of your opponent's Digimon")
        effect1.set_effect_description("[When Digivolving] This Digimon isn't affected by the effects of your opponent's Digimon until the end of their turn. Then, if DNA digivolving, place 1 of your opponent's Digimon whose level is less than or equal to the number of cards in yours or your opponent's security stack at the bottom of your opponent's security stack.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Put To Security, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Place a permanent into the security stack
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_put_security(target_perm):
                if player:
                    player.put_permanent_to_security(target_perm)
            game.effect_select_own_permanent(
                player, on_put_security, filter_fn=target_filter, is_optional=False)
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: partition
        # Partition
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-063 Partition")
        effect2.set_effect_description("Partition")
        effect2._is_partition = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: partition
        # Partition
        effect3 = ICardEffect()
        effect3.set_effect_name("BT16-063 Partition")
        effect3.set_effect_description("Partition")
        effect3.is_inherited_effect = True
        effect3._is_partition = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
