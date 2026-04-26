from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT4_072(CardScript):
    """BT4-072 Gogmamon | Lv.5

    [Main] Digi-Burst 1 (You may trash 1 of this Digimon's digivolution
    cards to activate the effect below.)
    - 1 of your Digimon gets +2000 DP until the end of your opponent's
      next turn.

    Inherited: [All Turns] This Digimon gets +1000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Digi-Burst 1: trash 1 source, give 1 own Digimon +2000 DP
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.NoTiming)
        effect0._is_field_main = True
        effect0.set_effect_name("BT4-072 Digi-Burst 1: +2000 DP to 1 Digimon")
        effect0.set_effect_description(
            "[Main] Digi-Burst 1: 1 of your Digimon gets +2000 DP until "
            "the end of your opponent's next turn."
        )
        effect0.is_optional = True
        effect0._is_digi_burst = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            # Must have at least 1 digivolution card to trash
            return len(perm.card_sources) > 1

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Trash 1 digivolution card as cost
            if len(perm.card_sources) <= 1:
                return
            trashed = perm.trash_digivolution_cards(1)
            player.trash_cards.extend(trashed)

            # Give 1 own Digimon +2000 DP until end of opponent's next turn
            def on_target(target_perm):
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_DP,
                    value_fn=lambda current, target, c: current + 2000,
                    expiry='end_of_opponent_turn',
                )

            game.effect_select_own_permanent(
                player, on_target,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your Digimon to give +2000 DP.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [All Turns] This Digimon gets +1000 DP.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT4-072 +1000 DP")
        effect1.set_effect_description("[All Turns] This Digimon gets +1000 DP.")
        effect1.is_inherited_effect = True
        effect1._dp_modifier = 1000

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
