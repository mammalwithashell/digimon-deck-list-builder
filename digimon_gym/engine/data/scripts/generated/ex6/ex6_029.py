from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_029(CardScript):
    """EX6-029 Mastemon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-029 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 level 5 or lower Digimon card with the [Angel]/[Archangel]/[Fallen Angel] trait from your hand or trash without paying the cost. Then, if DNA digivolving, place 1 other Digimon at the bottom of its owner's security stack, and trash cards from the top of your opponent's security stack until it has 4 left.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-029 Play 1 level 5 or lower Digimon card with the [Angel]/[Archangel]/[Fallen Angel] trait from your hand or trash, then, if DNA digivolving, place 1 other Digimon at the bottom of its owner's security stack, and trash cards from the top of your opponent's security stack until it has 4 left.")
        effect1.set_effect_description("[On Play] You may play 1 level 5 or lower Digimon card with the [Angel]/[Archangel]/[Fallen Angel] trait from your hand or trash without paying the cost. Then, if DNA digivolving, place 1 other Digimon at the bottom of its owner's security stack, and trash cards from the top of your opponent's security stack until it has 4 left.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Put To Security, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Place a permanent into the security stack
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_put_security(target_perm):
                if player:
                    player.put_permanent_to_security(target_perm)
            game.effect_select_own_permanent(
                player, on_put_security, filter_fn=target_filter, is_optional=True)
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 level 5 or lower Digimon card with the [Angel]/[Archangel]/[Fallen Angel] trait from your hand or trash without paying the cost. Then, if DNA digivolving, place 1 other Digimon at the bottom of its owner's security stack, and trash cards from the top of your opponent's security stack until it has 4 left.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-029 Play 1 level 5 or lower Digimon card with the [Angel]/[Archangel]/[Fallen Angel] trait from your hand or trash, then, if DNA digivolving, place 1 other Digimon at the bottom of its owner's security stack, and trash cards from the top of your opponent's security stack until it has 4 left.")
        effect2.set_effect_description("[When Digivolving] You may play 1 level 5 or lower Digimon card with the [Angel]/[Archangel]/[Fallen Angel] trait from your hand or trash without paying the cost. Then, if DNA digivolving, place 1 other Digimon at the bottom of its owner's security stack, and trash cards from the top of your opponent's security stack until it has 4 left.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Put To Security, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
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

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
