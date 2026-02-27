from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_097(CardScript):
    """BT15-097 Ultimate Slicer"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] By trashing 1 Digimon card with the [Machine], [Cyborg] or [SoC] trait in your hand, delete 1 of your opponent's Digimon or Tamers with the lowest play cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-097 Delete 1 Digimon or Tamer")
        effect0.set_effect_description("[Main] By trashing 1 Digimon card with the [Machine], [Cyborg] or [SoC] trait in your hand, delete 1 of your opponent's Digimon or Tamers with the lowest play cost.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Machine' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('Cyborg' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('SoC' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Machine' in _t or 'Cyborg' in _t or 'SoC' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
