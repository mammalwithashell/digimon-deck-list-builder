from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_092(CardScript):
    """BT14-092 Marching Fishes"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Choose 1 of your Digimon. Until the end of your opponent's turn, 3 of your opponent's Digimon with as many or fewer digivolution cards as that Digimon can't attack or block.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-092 Gain Keyword Cannot Attack, Gain Keyword Cannot Block, Grant Cannot Block")
        effect0.set_effect_description("[Main] Choose 1 of your Digimon. Until the end of your opponent's turn, 3 of your opponent's Digimon with as many or fewer digivolution cards as that Digimon can't attack or block.")
        effect0._is_cannot_attack = True
        effect0._is_cannot_block = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Attack, Gain Keyword Cannot Block, Grant Cannot Block"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_attack')
                perm.grant_keyword('_is_cannot_block')
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

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] 1 of your opponent's Digimon can't attack for the turn. Then, add this card to the hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-092 Add To Hand, Gain Keyword Cannot Attack")
        effect1.set_effect_description("[Security] 1 of your opponent's Digimon can't attack for the turn. Then, add this card to the hand.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True
        effect1._is_cannot_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add To Hand, Gain Keyword Cannot Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if perm:
                perm.grant_keyword('_is_cannot_attack')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
