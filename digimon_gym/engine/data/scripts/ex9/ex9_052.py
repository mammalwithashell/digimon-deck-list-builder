from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_052(CardScript):
    """EX9-052 Raremon | Lv.4 | Undead/DM/Ver.5
    [End of Your Turn][Once Per Turn] By placing 3 Digimon cards with [Ver.5]
    trait from trash face down as bottom digi cards, it may digivolve into a
    [Ver.5] trait Digimon from hand or trash.
    Inherited: [On Deletion] De-Digivolve 1 opponent Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Alt digivolve from Lv.3
        effect_alt = ICardEffect()
        effect_alt.set_effect_name("EX9-052 Alternate digivolution requirement")
        effect_alt.set_effect_description("Alternate digivolution requirement")
        effect_alt._alt_digi_cost = 3
        effect_alt._alt_digi_level = 3

        def condition_alt(context: Dict[str, Any]) -> bool:
            return True
        effect_alt.set_can_use_condition(condition_alt)
        effects.append(effect_alt)

        # [End of Your Turn][Once Per Turn] Place 3 Ver.5 from trash, digivolve
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEndTurn)
        effect0.set_effect_name("EX9-052 End of Turn: Place Ver.5 from trash, digivolve")
        effect0.set_effect_description(
            "[End of Your Turn][Once Per Turn] Place 3 Ver.5 Digimon from trash "
            "as bottom digi cards, digivolve into Ver.5 from hand or trash."
        )
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Digivolve_EX9_052")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Place 3 Ver.5 Digimon from trash as bottom digi cards
            placed_count = 0
            for c in list(player.trash_cards):
                if placed_count >= 3:
                    break
                if getattr(c, 'is_digimon', False):
                    traits = getattr(c, 'card_traits', []) or []
                    if any('Ver.5' in t for t in traits):
                        player.trash_cards.remove(c)
                        perm.add_card_source_bottom(c)
                        placed_count += 1

            if placed_count < 3:
                return  # Not enough cards to pay cost

            # Digivolve into Ver.5 from hand or trash
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Ver.5' in t for t in traits)

            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [On Deletion] De-Digivolve 1 opponent
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("EX9-052 Inherited: On Deletion De-Digivolve 1")
        effect1.set_effect_description(
            "Inherited: [On Deletion] De-Digivolve 1 opponent Digimon."
        )
        effect1.is_inherited_effect = True
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon

            def on_dedigivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player, on_dedigivolve, filter_fn=target_filter, is_optional=False)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
