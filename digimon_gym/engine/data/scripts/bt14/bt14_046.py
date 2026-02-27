from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_046(CardScript):
    """BT14-046 Togemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Your Turn] [Once Per Turn] When you would play a green Tamer from your hand,
        # by suspending 1 of your green Digimon, reduce the play cost by 3.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-046 Play Green Tamer Cost -3")
        effect0.set_effect_description("When playing a green Tamer from hand, suspend 1 of your green Digimon to reduce play cost by 3.")
        effect0.cost_reduction = 3
        effect0.is_once_per_turn = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            action = context.get('action') or context.get('event') or context.get('timing')
            if action not in ('play_tamer', 'would_play_tamer', 'play_from_hand', 'would_play_from_hand'):
                return False

            target = context.get('target_card') or context.get('card_to_play') or context.get('play_card')
            if target is None:
                return False

            colors = getattr(target, 'card_colors', None) or getattr(target, 'colors', None) or []
            if 3 not in colors:
                return False

            owner = card.owner
            board = getattr(owner, 'digimon', None) or getattr(owner, 'digimon_zone', None) or []
            for d in board:
                if d is None:
                    continue
                if getattr(d, 'is_suspended', False):
                    continue
                d_colors = getattr(d, 'card_colors', None) or getattr(d, 'colors', None) or []
                if 3 in d_colors:
                    return True
            return False

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            # Cost reduction is handled by `cost_reduction`.
            # This marks intent to pay the suspend requirement in engines that consume this flag.
            ctx['requires_suspend_my_green_digimon'] = 1

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [Your Turn] [Once Per Turn] When this Digimon would digivolve,
        # if you have a green Tamer, reduce the digivolution cost by 1.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-046 Inherited Digivolution Cost -1")
        effect1.set_effect_description("When this Digimon would digivolve, if you have a green Tamer, reduce digivolution cost by 1.")
        effect1.is_inherited_effect = True
        effect1.cost_reduction = 1
        effect1.is_once_per_turn = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            action = context.get('action') or context.get('event') or context.get('timing')
            if action not in ('digivolve', 'would_digivolve', 'digivolution'):
                return False

            source_perm = context.get('source_permanent') or context.get('permanent')
            my_perm = card.permanent_of_this_card() if card else None
            if my_perm is not None and source_perm is not None and source_perm is not my_perm:
                return False

            owner = card.owner
            tamers = getattr(owner, 'tamers', None) or getattr(owner, 'tamer_zone', None) or []
            for t in tamers:
                if t is None:
                    continue
                t_colors = getattr(t, 'card_colors', None) or getattr(t, 'colors', None) or []
                if 3 in t_colors:
                    return True
            return False

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            # Cost reduction by 1 is handled via `cost_reduction`.
            return None

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
