from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_057(CardScript):
    """EX6-057 Lilithmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Until the end of your opponent's turn, 1 of your opponent's Digimon gains [End of Your Turn] Delete this Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-057 Give 1 opponent Digimon [End of Your Turn] Delete this Digimon.")
        effect0.set_effect_description("[On Play] Until the end of your opponent's turn, 1 of your opponent's Digimon gains [End of Your Turn] Delete this Digimon.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [End of Your Turn] Delete this Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-057 Delete this Digimon")
        effect1.set_effect_description("[End of Your Turn] Delete this Digimon.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Until the end of your opponent's turn, 1 of your opponent's Digimon gains [End of Your Turn] Delete this Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-057 Give 1 opponent Digimon [End of Your Turn] Delete this Digimon.")
        effect2.set_effect_description("[When Digivolving] Until the end of your opponent's turn, 1 of your opponent's Digimon gains [End of Your Turn] Delete this Digimon.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [End of Your Turn] Delete this Digimon.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX6-057 Delete this Digimon")
        effect3.set_effect_description("[End of Your Turn] Delete this Digimon.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
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

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area other than in battle, by deleting 1 level 5 or lower Digimon, prevent it from leaving.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX6-057 Delete 1 level 5 or lower Digimon to prevent leaving the battle area")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than in battle, by deleting 1 level 5 or lower Digimon, prevent it from leaving.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Protection_EX6_057")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [Opponent's Turn] [Once Per Turn] When another Digimon is deleted, trash the top card of your opponent's security stack.
        effect5 = ICardEffect()
        effect5.set_effect_name("EX6-057 Trash the top card of your opponent's security")
        effect5.set_effect_description("[Opponent's Turn] [Once Per Turn] When another Digimon is deleted, trash the top card of your opponent's security stack.")
        effect5.set_max_count_per_turn(1)
        effect5.is_on_deletion = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
