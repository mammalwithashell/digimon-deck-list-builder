from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_079(CardScript):
    """BT24-079 Hadesmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Ultimate Appmon for cost 4
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-079 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: overclock
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-079 Overclock")
        effect1.set_effect_description("Overclock")
        effect1._is_overclock = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ---------------------------------------------------------------
        # When Digivolving:
        # Play 1 Lv.4- Digimon with [System] or [Life] trait from trash free.
        # Then, link 1 [Appmon] Digimon from hand or this Digimon's digi
        # cards to 1 of your Digimon free.
        # NOTE: Link mechanic requires engine support. This implements the
        # play-from-trash portion faithfully. Link portion is noted but the
        # engine link system would need to handle the second half.
        # ---------------------------------------------------------------
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-079 Play a lvl 4- from trash. Add new link to a digimon")
        effect2.set_effect_description("[When Digivolving] You may play 1 level 4 or lower [System] or [Life] trait Digimon card from your trash without paying the cost. Then, you may link 1 [Appmon] trait Digimon card from your hand or this Digimon's digivolution cards to 1 of your Digimon without paying the cost.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play 1 Lv.4- System/Life Digimon from trash free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('System' in tr for tr in traits) or any('Life' in tr for tr in traits)

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

            # NOTE: Link portion (link 1 Appmon from hand or digi sources to a Digimon)
            # requires engine Link support — not yet implemented in engine.

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ---------------------------------------------------------------
        # All Turns OPT: When other Digimon are deleted, re-activate
        # one of this Digimon's [When Digivolving] effects.
        # ---------------------------------------------------------------
        effect3 = ICardEffect()
        # NoTiming + _is_deletion_observer: fires via _fire_deletion_observers()
        # when ANY Digimon is deleted (not just self)
        effect3._is_deletion_observer = True
        effect3.set_effect_name("BT24-079 Activate [When Digivolving]")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When other Digimon are deleted, you may activate 1 of this Digimon's [When Digivolving] effects.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT24_079_AT_Activate_WD")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Re-activate this Digimon's When Digivolving effect."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Re-trigger the When Digivolving effect (effect2) by running its process
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('System' in tr for tr in traits) or any('Life' in tr for tr in traits)

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
