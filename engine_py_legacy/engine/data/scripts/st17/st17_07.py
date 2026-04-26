from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST17_07(CardScript):
    """ST17-07 Rapidmon | Lv.5 Green/Black Cyborg

    [On Play] [When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon. Then,
        if you have a green Tamer, until the end of your opponent's turn, your opponent's
        effects can't delete this Digimon or return it to the hand or deck.
    Inherited [All Turns] [Once Per Turn] When this Digimon deletes an opponent's
        Digimon in battle, trash the top card of their security stack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] De-Digivolve 1, then if green Tamer grant protection ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name(
            "ST17-07 On Play: De-Digivolve 1 opponent, if green Tamer gain delete/bounce immunity"
        )
        effect0.set_effect_description(
            "[On Play] <De-Digivolve 1> 1 of your opponent's Digimon. Then, if you "
            "have a green Tamer, until the end of your opponent's turn, your opponent's "
            "effects can't delete this Digimon or return it to the hand or deck."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: De-Digivolve 1, then if green Tamer grant destruction/bounce immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # De-Digivolve 1 one opponent Digimon
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon, is_optional=False)
            # Check for green Tamer
            from digimon_gym.engine.data.enums import CardColor
            has_green_tamer = any(
                p.is_tamer and CardColor.Green in getattr(p.top_card, 'card_colors', [])
                for p in player.battle_area
            )
            if has_green_tamer and perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_DESTROYED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
                game.register_modifier(
                    ModifierType.CANNOT_BE_RETURNED, perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] De-Digivolve 1, then if green Tamer grant protection ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name(
            "ST17-07 When Digivolving: De-Digivolve 1 opponent, if green Tamer gain delete/bounce immunity"
        )
        effect1.set_effect_description(
            "[When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon. Then, "
            "if you have a green Tamer, until the end of your opponent's turn, your "
            "opponent's effects can't delete this Digimon or return it to the hand or deck."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: De-Digivolve 1, then if green Tamer grant destruction/bounce immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # De-Digivolve 1 one opponent Digimon
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon, is_optional=False)
            # Check for green Tamer
            from digimon_gym.engine.data.enums import CardColor
            has_green_tamer = any(
                p.is_tamer and CardColor.Green in getattr(p.top_card, 'card_colors', [])
                for p in player.battle_area
            )
            if has_green_tamer and perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_DESTROYED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
                game.register_modifier(
                    ModifierType.CANNOT_BE_RETURNED, perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited [All Turns] [Once Per Turn] When deletes opponent in battle, trash top security ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndBattle)
        effect2.set_effect_name(
            "ST17-07 When deletes opponent Digimon in battle, trash top security"
        )
        effect2.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon deletes an opponent's "
            "Digimon in battle, trash the top card of their security stack."
        )
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("ST17_07_KillTrashSecurity")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash top card of opponent's security stack"""
            player = ctx.get('player')
            if not player:
                return
            enemy = player.enemy
            if enemy:
                if enemy.security_cards:
                    trashed = enemy.security_cards.pop(0)
                    enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
