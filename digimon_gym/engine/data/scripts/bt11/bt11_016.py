from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_016(CardScript):
    """BT11-016 Phoenixmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnLoseSecurity
        # [Your Turn][Once Per Turn] When a card is removed from your opponent's security stack, you may activate 1 of this Digimon's [On Deletion] effects.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnLoseSecurity)
        effect0.set_effect_name("BT11-016 Activate 1 [On Deletion] effect")
        effect0.set_effect_description("[Your Turn][Once Per Turn] When a card is removed from your opponent's security stack, you may activate 1 of this Digimon's [On Deletion] effects.")
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Activate_BT11_016")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 red Digimon with [Avian], [Bird], [Beast], [Animal], or [Sovereign] in one of its traits and 3000 DP or less from your hand without paying the cost. For each red Tamer you have in play, add 2000 to the maximum DP of the card you can play by this effect.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT11-016 Play 1 red Digimon from hand")
        effect1.set_effect_description("[On Deletion] You may play 1 red Digimon with [Avian], [Bird], [Beast], [Animal], or [Sovereign] in one of its traits and 3000 DP or less from your hand without paying the cost. For each red Tamer you have in play, add 2000 to the maximum DP of the card you can play by this effect.")
        effect1.is_optional = True
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not ('Red' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
