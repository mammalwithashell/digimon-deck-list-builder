from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_019(CardScript):
    """BT8-019 Zhuqiaomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Your opponent chooses 1 of their Digimon. Delete all Digimon other than this Digimon and your opponent's chosen Digimon. For each Digimon deleted by this effect, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-019 Delete Digimons and gain Memory")
        effect0.set_effect_description("[When Digivolving] Your opponent chooses 1 of their Digimon. Delete all Digimon other than this Digimon and your opponent's chosen Digimon. For each Digimon deleted by this effect, gain 1 memory.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete, Effect Immunity"""
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
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [Your Turn] When an opponent's Digimon is deleted, this Digimon gains <Security Attack +1> for the turn for each of your opponent's Digimon deleted.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT8-019 This Digimon gains Security Attack +")
        effect1.set_effect_description("[Your Turn] When an opponent's Digimon is deleted, this Digimon gains <Security Attack +1> for the turn for each of your opponent's Digimon deleted.")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
