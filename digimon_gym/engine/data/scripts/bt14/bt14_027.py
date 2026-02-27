from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_027(CardScript):
    """BT14-027 MarineDevimon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Return all level 3 Digimon to their owner's hands.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-027 Return all level 3 Digimons to hand")
        effect0.set_effect_description("[On Play] Return all level 3 Digimon to their owner's hands.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            game = ctx.get('game')
            if not game:
                return

            all_permanents = []
            for p in game.get_all_battle_area_permanents():
                if p is None:
                    continue
                if not p.is_digimon():
                    continue
                if p.get_level() != 3:
                    continue
                all_permanents.append(p)

            for target in all_permanents:
                owner = target.get_owner()
                if owner:
                    owner.bounce_permanent_to_hand(target)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Return all level 3 Digimon to their owner's hands.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-027 Return all level 3 Digimons to hand")
        effect1.set_effect_description("[When Digivolving] Return all level 3 Digimon to their owner's hands.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            game = ctx.get('game')
            if not game:
                return

            all_permanents = []
            for p in game.get_all_battle_area_permanents():
                if p is None:
                    continue
                if not p.is_digimon():
                    continue
                if p.get_level() != 3:
                    continue
                all_permanents.append(p)

            for target in all_permanents:
                owner = target.get_owner()
                if owner:
                    owner.bounce_permanent_to_hand(target)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
