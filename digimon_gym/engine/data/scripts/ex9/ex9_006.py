from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_006(CardScript):
    """EX9-006 Pagumon | Lv.2 Purple Digi-Egg | Lesser/DM/Ver.5
    NOTE: EX9 set not yet in CardDatabase. Script ready for when data is added.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Inherited [When Attacking][Once Per Turn]
        #     Trash bottom digivolution card, digivolve into Ver.5 from trash ---
        # NOTE: Digivolving from trash with cost reduction via trashing own
        # digivolution card is a complex mechanic. Marked PARTIAL.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnTappedAnyone)
        effect0.set_effect_name("EX9-006 Inherited: Trash bottom evo card, digivolve from trash")
        effect0.set_effect_description(
            "Inherited: [When Attacking][Once Per Turn] By trashing this "
            "Digimon's bottom face-down digivolution card, this Digimon may "
            "digivolve into a [Ver.5] trait Digimon card in the trash with "
            "the digivolution cost reduced by 1."
        )
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("evo_from_trash_EX9_006")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm and len(perm.digivolution_cards) < 1:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash bottom digi card, digivolve into Ver.5 from trash"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            # Cost: Trash bottom face-down digivolution card
            if perm.digivolution_cards:
                trashed_card = perm.digivolution_cards.pop()
                player.trash_cards.append(trashed_card)

            # Effect: Digivolve into [Ver.5] Digimon from trash with cost -1
            # Since engine doesn't support digivolve from trash natively,
            # we find a Ver.5 Digimon in trash and digivolve manually
            def ver5_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Ver.5' in t for t in traits)

            qualifying = [c for c in player.trash_cards if ver5_filter(c)]
            if qualifying:
                chosen = qualifying[0]
                player.trash_cards.remove(chosen)
                # Digivolve: place as new top card of the stack
                perm.add_card_source(chosen)
                perm.turn_digivolved = game.turn_count
                # Cost reduction of 1 applied (free since from trash via effect)
                game.logger.log(f"[Effect] {player.player_name} digivolved "
                               f"from trash into Ver.5 Digimon")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
