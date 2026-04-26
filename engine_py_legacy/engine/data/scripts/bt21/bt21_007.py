from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_007(CardScript):
    """BT21-007 Agumon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Add To Hand
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT21-007 Return a card from trash")
        effect0.set_effect_description("Add To Hand")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Return 1 Reptile/Dragonkin Digimon from trash to hand"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def trait_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Reptile' in t or 'Dragonkin' in t for t in traits)

            from engine_py_legacy.engine.game.constants import SEL_TRASH_START
            valid = [SEL_TRASH_START + i for i, c in enumerate(player.trash_cards) if trait_filter(c)]
            if not valid:
                return

            def on_trash_selected(action_id: int):
                idx = action_id - SEL_TRASH_START
                if not (0 <= idx < len(player.trash_cards)):
                    return
                chosen = player.trash_cards[idx]
                player.trash_cards.remove(chosen)
                player.hand_cards.append(chosen)

            game.request_selection(
                GamePhase.SelectTrash, player, on_trash_selected, valid,
                is_optional=True,
                prompt="Select 1 Digimon with [Reptile] or [Dragonkin] trait from your trash to add to your hand."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: dp_modifier
        # DP modifier
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-007 DP modifier")
        effect1.set_effect_description("DP modifier")
        effect1.is_inherited_effect = True
        effect1.dp_modifier = 2000

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
