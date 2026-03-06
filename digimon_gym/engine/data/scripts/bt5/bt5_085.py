from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_085(CardScript):
    """BT5-085 Armageddemon | Lv.7 Mega | White | Unidentified

    When you would play this card from your hand, you may delete 1 of your
        [Diaboromon] to reduce this card's play cost by 12.
    <Rush>
    [All Turns] The [When Digivolving] effects on level 7 Digimon don't activate.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Cost reduction by deleting own Diaboromon ---
        # When playing from hand, delete 1 own [Diaboromon] to reduce cost by 12.
        # This is a play-cost reduction mechanic modeled as a BeforePayCost effect.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT5-085 Cost reduction: delete Diaboromon")
        effect0.set_effect_description(
            "When you would play this card from your hand, you may delete 1 "
            "of your [Diaboromon] to reduce this card's play cost by 12."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Delete own Diaboromon to reduce play cost by 12."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Find a Diaboromon on our field
            diaboromon = None
            for perm in player.battle_area:
                if perm.top_card and perm.top_card.c_entity_base:
                    if 'Diaboromon' in (perm.top_card.c_entity_base.card_name_eng or ''):
                        diaboromon = perm
                        break
            if diaboromon:
                player.delete_permanent(diaboromon)
                if hasattr(player, '_temp_play_cost_reduction'):
                    player._temp_play_cost_reduction += 12
                else:
                    player._temp_play_cost_reduction = 12
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Rush keyword ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT5-085 Rush")
        effect1.set_effect_description("<Rush>")
        effect1._is_rush = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [All Turns] Lv.7 When Digivolving effects don't activate ---
        # This is a suppression effect: When Digivolving effects on Lv.7 Digimon
        # are prevented. We model this as a declarative effect that the engine
        # can check when processing digivolution triggers.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT5-085 Suppress Lv.7 When Digivolving effects")
        effect2.set_effect_description(
            "[All Turns] The [When Digivolving] effects on level 7 Digimon "
            "don't activate."
        )
        effect2.is_declarative = True
        effect2._suppresses_when_digivolving_lv7 = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
