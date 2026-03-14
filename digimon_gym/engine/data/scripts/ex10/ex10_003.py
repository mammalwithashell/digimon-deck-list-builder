from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_003(CardScript):
    """EX10-003 Tumblemon | Lv.2

    Inherited Effect [Opponent's Turn] [Once Per Turn] When one of your
    opponent's Digimon attacks, by trashing 3 [Mineral] or [Rock] trait
    cards from this Digimon's digivolution cards, end that attack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited: [Opponent's Turn] [Once Per Turn] When opponent attacks,
        # by trashing 3 [Mineral]/[Rock] sources, end that attack.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnUseAttack)
        effect0.set_effect_name("EX10-003 By trashing 3 Mineral/Rock sources, end attack")
        effect0.set_effect_description(
            "[Opponent's Turn] [Once Per Turn] When one of your opponent's "
            "Digimon attacks, by trashing 3 [Mineral] or [Rock] trait cards "
            "from this Digimon's digivolution cards, end that attack."
        )
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("ESS_EX10-003")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only on opponent's turn
            owner = card.owner if card else None
            if not owner or owner.is_my_turn:
                return False
            # Check this Digimon has at least 3 Mineral/Rock digivolution cards
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            count = 0
            for c in perm.digivolution_cards:
                traits = getattr(c, 'card_traits', []) or []
                if 'Mineral' in traits or 'Rock' in traits:
                    count += 1
                    if count >= 3:
                        return True
            return False

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Trash 3 Mineral/Rock digivolution cards and end the attack."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Trash 3 Mineral/Rock digivolution cards as cost
            trashed = 0
            for c in list(perm.digivolution_cards):
                if trashed >= 3:
                    break
                if c is perm.top_card:
                    continue
                traits = getattr(c, 'card_traits', []) or []
                if 'Mineral' in traits or 'Rock' in traits:
                    if c in perm.card_sources:
                        perm.card_sources.remove(c)
                        player.trash_cards.append(c)
                        trashed += 1

            if trashed >= 3:
                game.force_end_attack()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
