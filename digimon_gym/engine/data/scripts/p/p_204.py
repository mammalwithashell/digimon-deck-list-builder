from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_204(CardScript):
    """P-204 Release of the Sealed Knight!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] By trashing 1 card with the [X Antibody] or [Chronicle] trait from your hand, <Draw 2> (Draw 2 cards from your deck). Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-204 Trash 1 [X Antibody]/[Chronicle] from hand, draw 2")
        effect0.set_effect_description("[Main] By trashing 1 card with the [X Antibody] or [Chronicle] trait from your hand, <Draw 2> (Draw 2 cards from your deck). Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 2, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)
            if player:
                player.draw_cards(2)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("P-204 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Grademon'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [All Turns] When Digimon attack players, <Delay>. 1 of your [Grademon] or Digimon with the [Chronicle] trait may digivolve into [Alphamon] or a level 6 or lower Digimon card with the [Chronicle] trait in the hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-204 1 [Grademon]/[Chronicle] digimon digivoles into [Alphamon]/ level 6 or lower [Chronicle] digimon")
        effect2.set_effect_description("[All Turns] When Digimon attack players, <Delay>. 1 of your [Grademon] or Digimon with the [Chronicle] trait may digivolve into [Alphamon] or a level 6 or lower Digimon card with the [Chronicle] trait in the hand without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Grademon'))):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                if not (any('Alphamon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
