from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_008(CardScript):
    """BT15-008 Muchomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [Your Turn][Once per turn] When one of your red Digimon attacks a player, [Draw 1].
        # C# ref: OnAllyAttack timing with PermanentCondition checking red color,
        # and DefendingPermanent == null (attacks player, not Digimon).
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnAllyAttack)
        effect0.set_effect_name("BT15-008 Draw 1")
        effect0.set_effect_description("[Your Turn][Once per turn] When one of your red Digimon attacks a player, [Draw 1].")
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Draw1_BT15_008")
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check the attacking permanent is a red Digimon on owner's battle area
            attacker = context.get('attacker')
            if attacker is not None:
                top = attacker.top_card
                if not top or CardColor.Red not in (top.card_colors or []):
                    return False
                # Check attacker is on owner's battle area
                if attacker not in (card.owner.battle_area if card.owner else []):
                    return False
            # Check attack target is player (not Digimon)
            # DefendingPermanent == null means player attack
            game = context.get('game')
            if game and game.pending_attack:
                if game.pending_attack.original_target is not None:
                    # If original_target is a Permanent (not a Player), it's a Digimon attack
                    from ....core.permanent import Permanent
                    if isinstance(game.pending_attack.original_target, Permanent):
                        return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
