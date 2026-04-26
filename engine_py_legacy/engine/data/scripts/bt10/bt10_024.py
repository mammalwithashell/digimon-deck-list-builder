from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_024(CardScript):
    """BT10-024 MetalGreymon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: save
        # Save
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-024 Save")
        effect0.set_effect_description("Save")
        effect0._is_save = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: material_save
        # Material Save
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-024 Material Save")
        effect1.set_effect_description("Material Save")
        effect1._is_material_save = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] This Digimon gains <Rush> for the turn. Then, if DigiXrosing, 3 of your opponent's Digimon with digivolution cards less than or equal to this Digimon's digivolution cards can't attack or block until the end of your opponent's turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT10-024 This Digimon gains Rush and opponent's Digimons can't attack and block")
        effect2.set_effect_description("[On Play] This Digimon gains <Rush> for the turn. Then, if DigiXrosing, 3 of your opponent's Digimon with digivolution cards less than or equal to this Digimon's digivolution cards can't attack or block until the end of your opponent's turn.")
        effect2.is_on_play = True
        effect2._is_cannot_attack = True
        effect2._is_cannot_block = True
        effect2._is_rush = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Attack, Gain Keyword Cannot Block, Grant Cannot Block, Gain Keyword Rush"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_attack')
                perm.grant_keyword('_is_cannot_block')
                perm.grant_keyword('_is_rush')
            # Prevent target from blocking
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def on_restrict(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_BLOCK, target_perm,
                    value_fn=lambda: True, expiry='end_of_turn')
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT10-024 Effect")
        effect3.set_effect_description("Effect")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
