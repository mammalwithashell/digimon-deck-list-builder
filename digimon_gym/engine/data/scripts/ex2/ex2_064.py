from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX2_064(CardScript):
    """EX2-064 Alice McCoy | Tamer | White | Cost 2

    [Your Turn][Once Per Turn] When one of your Digimon would digivolve
        from level 5 to level 6, you may delete 1 of your Digimon to
        reduce the digivolution cost by 3.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Your Turn][Once Per Turn] Delete 1 Digimon to reduce
        #     digivolution cost by 3 when digivolving Lv.5 -> Lv.6 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("EX2-064 Delete Digimon for evo cost -3")
        effect0.set_effect_description(
            "[Your Turn][Once Per Turn] When one of your Digimon would "
            "digivolve from level 5 to level 6, you may delete 1 of "
            "your Digimon to reduce the digivolution cost by 3."
        )
        effect0.is_optional = True
        effect0.cost_reduction = 3
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("DeleteDigimon_EX2_064")

        def condition0(context: Dict[str, Any]) -> bool:
            # Leak guard
            if context.get('card_source') is not card:
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # [Your Turn] only
            if not owner.is_my_turn:
                return False
            # Must be a digivolution from Lv.5 to Lv.6
            digivolve_target = context.get('digivolve_target')
            digivolve_card = context.get('digivolve_card')
            if digivolve_target is not None:
                target_level = getattr(digivolve_target, 'level', None)
                if target_level != 5:
                    return False
            if digivolve_card is not None:
                card_level = getattr(digivolve_card, 'level', None)
                if card_level != 6:
                    return False
            # Need at least 1 own Digimon on field to delete
            has_digimon = any(
                p.is_digimon for p in owner.battle_area
            )
            return has_digimon
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Delete 1 of your Digimon to reduce cost by 3."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def digimon_filter(p):
                return p.is_digimon and p in player.battle_area

            def on_delete(target_perm):
                if target_perm is not None:
                    player.delete_permanent(target_perm)

            game.effect_select_own_permanent(
                player, on_delete, filter_fn=digimon_filter,
                is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Play this card without paying the cost ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("EX2-064 Security: Play free")
        effect1.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
