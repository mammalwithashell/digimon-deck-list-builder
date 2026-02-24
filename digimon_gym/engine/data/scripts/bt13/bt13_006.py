from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_006(CardScript):
    """BT13-006 Kapurimon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] By trashing 1 card in your hand, delete 1 of your opponent's level 3 Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-006 Trash 1 card from hand to delete 1 level 3 Digimon")
        effect0.set_effect_description("[On Deletion] By trashing 1 card in your hand, delete 1 of your opponent's level 3 Digimon.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.is_on_deletion = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash 1 from hand (mandatory) to delete 1 opponent level 3 Digimon (mandatory, if possible)"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Only offer effect if player has card in hand and opponent has level 3 Digimon
            enemy = player.enemy if player else None
            if not enemy:
                return
            level3_targets = [p for p in enemy.permanents if getattr(p, 'is_digimon', False) and getattr(p, 'get_level', lambda: 0)() == 3]
            if not player.hand_cards or not level3_targets:
                # If either mandatory component is missing, do nothing
                return

            # Ask player if they want to activate the effect (optional effect)
            # If user proceeds, require trash, then deletion
            def on_trash(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    # Now select enemy's level 3 Digimon to delete (mandatory)
                    def target_filter(p):
                        return getattr(p, 'is_digimon', False) and getattr(p, 'get_level', lambda: 0)() == 3
                    def on_delete(target_perm):
                        if target_perm in enemy.permanents:
                            enemy.delete_permanent(target_perm)
                    game.effect_select_opponent_permanent(
                        player, on_delete, filter_fn=target_filter, is_optional=False
                    )
            # Trash a hand card—mandatory if effect is activated
            hand_filter = lambda c: True  # Can choose any card
            # Present only one selection, mandatory if effect is used
            game.effect_select_hand_card(player, hand_filter, on_trash, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
