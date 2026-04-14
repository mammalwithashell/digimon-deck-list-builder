from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST20_10(CardScript):
    """ST20-10 Agumon | Lv.3 Black Digimon | Reptile/ADVENTURE/Hero

    Alt Digivolution: [ADVENTURE] or [Hero] trait Lv.2 for cost 0.
    Warp Digivolution: [Your Turn] If your opponent has a Digimon with
        10000 DP or more, or you have 3 or more colors among your Tamers,
        this Digimon may digivolve into [WarGreymon] in your hand for a
        memory cost of 4, ignoring digivolution requirements.
    Inherited: <Reboot>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0a: Alt Digivolution from [ADVENTURE] trait Lv.2 for cost 0 ---
        effect0a = ICardEffect()
        effect0a.set_effect_name("ST20-10 Alt digi: ADVENTURE Lv.2 cost 0")
        effect0a.set_effect_description(
            "Digivolve: Lv.2 w/[ADVENTURE] trait for cost 0."
        )
        effect0a._alt_digi_cost = 0
        effect0a._alt_digi_level = 2
        effect0a._alt_digi_trait = 'ADVENTURE'

        def condition0a(context: Dict[str, Any]) -> bool:
            return True
        effect0a.set_can_use_condition(condition0a)
        effects.append(effect0a)

        # --- Effect 0b: Alt Digivolution from [Hero] trait Lv.2 for cost 0 ---
        effect0b = ICardEffect()
        effect0b.set_effect_name("ST20-10 Alt digi: Hero Lv.2 cost 0")
        effect0b.set_effect_description(
            "Digivolve: Lv.2 w/[Hero] trait for cost 0."
        )
        effect0b._alt_digi_cost = 0
        effect0b._alt_digi_level = 2
        effect0b._alt_digi_trait = 'Hero'

        def condition0b(context: Dict[str, Any]) -> bool:
            return True
        effect0b.set_can_use_condition(condition0b)
        effects.append(effect0b)

        # --- Effect 1: Warp Digivolution into [WarGreymon] for cost 4 ---
        # [Your Turn] While your opponent has a Digimon with 10000 DP or more,
        # or your Tamers have 3 or more total colors, this Digimon can digivolve
        # into [WarGreymon] in the hand for a digivolution cost of 4, ignoring
        # digivolution requirements.
        #
        # This is a continuous effect that enables a warp digivolve action.
        # Uses effect_digivolve_from_hand to let the agent pick a WarGreymon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("ST20-10 Warp digi: into WarGreymon cost 4")
        effect1.set_effect_description(
            "[Your Turn] If your opponent has a Digimon with 10000 DP or more, "
            "or you have 3 or more colors among your Tamers, this Digimon may "
            "digivolve into [WarGreymon] in your hand for a memory cost of 4, "
            "ignoring digivolution requirements."
        )
        effect1.is_optional = True

        def _count_tamer_colors():
            """Count distinct colors among owner's Tamers."""
            owner = card.owner if card else None
            if not owner:
                return 0
            color_set = set()
            for p in owner.battle_area:
                if p.is_tamer and p.top_card:
                    for col in (getattr(p.top_card, 'card_colors', []) or []):
                        color_set.add(col)
            return len(color_set)

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card.permanent_of_this_card() is None:
                return False
            owner = card.owner
            enemy = owner.enemy if owner else None
            # Check opponent has a Digimon with 10000+ DP
            if enemy:
                for p in enemy.battle_area:
                    if p.is_digimon and p.dp is not None and p.dp >= 10000:
                        return True
            # Check 3+ colors among own Tamers
            if _count_tamer_colors() >= 3:
                return True
            return False
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Warp digivolve this permanent into a WarGreymon from hand."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def wargreymon_filter(c):
                return c.contains_card_name('WarGreymon')

            game.effect_digivolve_from_hand(
                player, perm, wargreymon_filter,
                cost_override=4,
                ignore_requirements=True,
                is_optional=True,
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited <Reboot> ---
        effect2 = ICardEffect()
        effect2.set_effect_name("ST20-10 Inherited: Reboot")
        effect2.set_effect_description("<Reboot>")
        effect2.is_inherited_effect = True
        effect2._is_reboot = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
