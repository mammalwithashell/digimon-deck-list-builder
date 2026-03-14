from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT2_047(CardScript):
    """BT2-047 Argomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Digisorption -3> (When one of your Digimon would digivolve into this
        # card from your hand, you may suspend 1 of your Digimon to reduce the
        # memory cost of the digivolution by 3.)
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT2-047 Digisorption -3")
        effect0.set_effect_description(
            "<Digisorption -3> (When one of your Digimon would digivolve into "
            "this card from your hand, you may suspend 1 of your Digimon to "
            "reduce the memory cost of the digivolution by 3.)"
        )
        effect0.is_optional = True
        effect0.set_hash_string("Digisorption-3_BT2_047")
        effect0.cost_reduction = 3

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Suspend 1 of your Digimon to apply digisorption cost reduction."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def digimon_filter(p):
                return p.is_digimon and not p.is_suspended

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_own_permanent(
                player, on_suspend,
                filter_fn=digimon_filter,
                is_optional=False,
                prompt="Suspend 1 of your Digimon for Digisorption -3."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [When Attacking] You may play 1 green level 3 Digimon card
        # from your hand suspended without paying its memory cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1.is_on_attack = True
        effect1.is_inherited_effect = True
        effect1.set_effect_name("BT2-047 Inherited: Play green Lv.3 from hand suspended")
        effect1.set_effect_description(
            "[When Attacking] You may play 1 green level 3 Digimon card from "
            "your hand suspended without paying its memory cost."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            from ....data.enums import CardColor

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if (getattr(c, 'level', None) or 0) != 3:
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Green in colors

            def on_played(played_perm):
                if played_perm:
                    played_perm.suspend()

            # Play from hand, free, optional
            valid_cards = [c for c in player.hand_cards if play_filter(c)]
            if valid_cards:
                game.effect_play_from_zone(
                    player, 'hand', play_filter,
                    free=True, is_optional=True,
                    prompt="You may play 1 green Lv.3 Digimon from hand suspended (free)."
                )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
