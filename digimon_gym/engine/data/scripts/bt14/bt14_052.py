from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_052(CardScript):
    """BT14-052 Panjyamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 1 of your opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-052 Suspend 1 Digimon")
        effect1.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return True

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: [Your Turn] While this Digimon has [Leomon] in its name, it gets +2000 DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-052 Inherited +2000 DP with [Leomon] name")
        effect2.set_effect_description("[Your Turn] While this Digimon has [Leomon] in its name, it gets +2000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            name = getattr(perm, 'name', None)
            if not isinstance(name, str):
                return False
            return 'leomon' in name.lower()

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
