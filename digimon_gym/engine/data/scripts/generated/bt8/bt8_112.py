from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT8_112(CardScript):
    """BT8-112 Imperialdramon Paladin Mode | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When one of your Digimon would digivolve into this card in your hand, you may return 1 white level 7 Digimon card from your trash to the bottom of your deck to reduce the digivolution cost by 4.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT8-112 Digivolution Cost -4")
        effect0.set_effect_description("When one of your Digimon would digivolve into this card in your hand, you may return 1 white level 7 Digimon card from your trash to the bottom of your deck to reduce the digivolution cost by 4.")
        effect0.is_optional = True
        effect0.set_hash_string("DigivolutionCost-4_BT8_112")
        effect0.cost_reduction = 4

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -4, Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 4 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may return 1 2-color card from this Digimon's digivolution cards to the bottom of its owner's deck to trash all of the digivolution cards of 1 of your opponent's Digimon. Then, return all of your opponent's Digimon with no digivolution cards to the bottom of their owners' decks in any order.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT8-112 Trash digivolution cards and return Digimons to the bottom of deck")
        effect1.set_effect_description("[When Digivolving] You may return 1 2-color card from this Digimon's digivolution cards to the bottom of its owner's deck to trash all of the digivolution cards of 1 of your opponent's Digimon. Then, return all of your opponent's Digimon with no digivolution cards to the bottom of their owners' decks in any order.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] You may return 1 2-color card from this Digimon's digivolution cards to the bottom of its owner's deck to trash all of the digivolution cards of 1 of your opponent's Digimon. Then, return all of your opponent's Digimon with no digivolution cards to the bottom of their owners' decks in any order.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT8-112 Trash digivolution cards and return Digimons to the bottom of deck")
        effect2.set_effect_description("[When Attacking] You may return 1 2-color card from this Digimon's digivolution cards to the bottom of its owner's deck to trash all of the digivolution cards of 1 of your opponent's Digimon. Then, return all of your opponent's Digimon with no digivolution cards to the bottom of their owners' decks in any order.")
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
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
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
