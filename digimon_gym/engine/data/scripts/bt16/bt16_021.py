from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_021(CardScript):
    """BT16-021 Togemogumon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-021 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT16-021 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect1._alt_digi_cost = 2
        effect1._alt_digi_name = "Wormmon"

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: armor_purge
        # Armor Purge
        effect2 = ICardEffect()
        effect2.set_effect_name("BT16-021 Armor Purge")
        effect2.set_effect_description("Armor Purge")
        effect2._is_armor_purge = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When an opponent's Digimon becomes suspended, trash the top digivolution card of 1 of their Digimon. Then, 1 of their Digimon with no digivolution cards can't attack or block until the end of their turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnTappedAnyone)
        effect3.set_effect_name("BT16-021 Trash 1 digivolution card & stun 1 Digimon")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When an opponent's Digimon becomes suspended, trash the top digivolution card of 1 of their Digimon. Then, 1 of their Digimon with no digivolution cards can't attack or block until the end of their turn.")
        effect3.set_max_count_per_turn(1)
        effect3._is_cannot_attack = True
        effect3._is_cannot_block = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Gain Keyword Cannot Attack, Gain Keyword Cannot Block, Grant Cannot Block"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
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

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
