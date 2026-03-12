from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_097(CardScript):
    """BT8-097 Crimson Blaze"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Variable cost reduction: -1 per opponent Digimon
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT8-097 Cost reduction")
        effect0.set_effect_description("Reduce the memory cost by 1 for each Digimon your opponent has in play.")
        effect0.cost_reduction = 0

        def condition0(context: Dict[str, Any]) -> bool:
            player = card.owner if card else None
            if player:
                enemy = player.enemy if player else None
                if enemy:
                    opp_digi = len([p for p in enemy.battle_area if p.is_digimon])
                    effect0.cost_reduction = opp_digi
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Your opponent can't play Digimon by effects until the end of their turn. Delete all of your opponent's Digimon with 6000 DP or less.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT8-097 Can't play Digimon by effect")
        effect1.set_effect_description("[Main] Your opponent can't play Digimon by effects until the end of their turn. Delete all of your opponent's Digimon with 6000 DP or less.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Opponent can't play Digimon by effects, delete ALL opp Digimon ≤6000 DP"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            # Play restriction: opponent can't play Digimon by effects until end of their turn
            if enemy and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                # Note: CANNOT_PUT_ON_FIELD is over-broad (known engine gap #6)
                # but arg order must be (target_permanent, modifier_type, ...)
                for opp_perm in list(enemy.battle_area):
                    game.register_modifier(
                        opp_perm, ModifierType.CANNOT_PUT_ON_FIELD,
                        value_fn=lambda: True, expiry='end_of_opponent_turn')
            # Delete ALL opponent Digimon with 6000 DP or less
            if enemy:
                to_delete = [p for p in list(enemy.battle_area)
                             if p.is_digimon and p.dp is not None and p.dp <= 6000]
                for target in to_delete:
                    enemy.delete_permanent(target)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT8-097 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
