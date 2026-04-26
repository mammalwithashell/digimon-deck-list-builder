from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_081(CardScript):
    """BT22-081 Eater Eve"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-081 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Yuuko Kamishiro] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Yuuko Kamishiro"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Yuuko Kamishiro'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: raid
        # Raid
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-081 Raid")
        effect1.set_effect_description("Raid")
        effect1.is_on_attack = True
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Until your opponent's turn ends, 1 of their Digimon can't suspend. Then, if this Digimon has no digivolution cards, you may place 1 [Yuuko Kamishiro] from your hand or trash as this Digimon's bottom digivolution card.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-081 1 digimon can not suspend, then if no sources, place [Yuuko Kamishiro] from hand or trash")
        effect2.set_effect_description("[On Play] Until your opponent's turn ends, 1 of their Digimon can't suspend. Then, if this Digimon has no digivolution cards, you may place 1 [Yuuko Kamishiro] from your hand or trash as this Digimon's bottom digivolution card.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Until your opponent's turn ends, 1 of their Digimon can't suspend. Then, if this Digimon has no digivolution cards, you may place 1 [Yuuko Kamishiro] from your hand or trash as this Digimon's bottom digivolution card.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-081 1 digimon can not suspend, then if no sources, place [Yuuko Kamishiro] from hand or trash")
        effect3.set_effect_description("[When Digivolving] Until your opponent's turn ends, 1 of their Digimon can't suspend. Then, if this Digimon has no digivolution cards, you may place 1 [Yuuko Kamishiro] from your hand or trash as this Digimon's bottom digivolution card.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area, you may play 1 [Yuuko Kamishiro] from its digivolution cards without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT22-081 Play [Yuuko Kamishiro]")
        effect4.set_effect_description("[All Turns] When this Digimon would leave the battle area, you may play 1 [Yuuko Kamishiro] from its digivolution cards without paying the cost.")
        effect4.is_optional = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
