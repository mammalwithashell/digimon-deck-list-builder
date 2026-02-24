from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_003(CardScript):
    """EX10-003 Tumblemon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, by trashing 3 [Mineral] or [Rock] trait cards from this Digimon's digivolution cards, end that attack.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-003 by trashing 3 digivolution sources, end an attack")
        effect0.set_effect_description("[Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, by trashing 3 [Mineral] or [Rock] trait cards from this Digimon's digivolution cards, end that attack.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("ESS_EX10-003")
        effect0.is_on_attack = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
