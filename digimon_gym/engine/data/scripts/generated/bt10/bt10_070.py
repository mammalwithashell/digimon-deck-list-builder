from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_070(CardScript):
    """BT10-070 Blastmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: rush
        # Rush
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-070 Rush")
        effect0.set_effect_description("Rush")
        effect0._is_rush = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blitz
        # Blitz
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-070 Blitz")
        effect1.set_effect_description("Blitz")
        effect1._is_blitz = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and len(permanent.digivolution_cards) >= 3):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by trashing 1 of this Digimon's digivolution cards, delete 1 of your opponent's level 4 or lower Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT10-070 Trash 1 digivolution card to delete 1 level4 or lower Digimon")
        effect2.set_effect_description("[Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by trashing 1 of this Digimon's digivolution cards, delete 1 of your opponent's level 4 or lower Digimon.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete, Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 4:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True)
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
