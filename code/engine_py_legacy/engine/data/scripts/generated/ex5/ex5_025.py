from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_025(CardScript):
    """EX5-025 Dianamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-025 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 4
        effect0._alt_digi_cost = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-025 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn] For each of this Digimon's digivolution cards, trash any 1 digivolution card from 1 of your opponent's Digimon. Then, until the end of your opponent's turn, all of their Digimon with no digivolution cards can't suspend.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX5-025 Trash digivolution cards and opponent's Digimon can't suspend")
        effect2.set_effect_description("[When Digivolving] [Once Per Turn] For each of this Digimon's digivolution cards, trash any 1 digivolution card from 1 of your opponent's Digimon. Then, until the end of your opponent's turn, all of their Digimon with no digivolution cards can't suspend.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("TrashDigivolutionCards_EX5_025")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] For each of this Digimon's digivolution cards, trash any 1 digivolution card from 1 of your opponent's Digimon. Then, until the end of your opponent's turn, all of their Digimon with no digivolution cards can't suspend.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX5-025 Trash digivolution cards and opponent's Digimon can't suspend")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] For each of this Digimon's digivolution cards, trash any 1 digivolution card from 1 of your opponent's Digimon. Then, until the end of your opponent's turn, all of their Digimon with no digivolution cards can't suspend.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("TrashDigivolutionCards_EX5_025")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # [All Turns] [Once Per Turn] When an opponent's Digimon's digivolution card is trashed, unsuspend this Digimon.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX5-025 Unsuspend this Digimon")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When an opponent's Digimon's digivolution card is trashed, unsuspend this Digimon.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Unsuspend_EX5_025")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
