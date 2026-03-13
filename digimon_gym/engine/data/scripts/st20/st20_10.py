from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST20_10(CardScript):
    """ST20-10 Agumon | Lv.3 Red Digimon | Adventure/Hero

    Alt Digivolution: [Adventure] or [Hero] trait Lv.2 for cost 0.
    Warp Digivolution: [Your Turn] If your opponent has a Digimon with
        10000 DP or more, or you have 3 or more colors among your Tamers,
        this Digimon may digivolve into [WarGreymon] in your hand for a
        memory cost of 4, ignoring digivolution requirements.
    Inherited: <Reboot>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt Digivolution from [Adventure]/[Hero] trait Lv.2 for cost 0 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("ST20-10 Alt digi: Adventure/Hero Lv.2 cost 0")
        effect0.set_effect_description(
            "Digivolve: Lv.2 w/[Adventure] or [Hero] trait for cost 0."
        )

        def alt_digi_permanent_condition(permanent) -> bool:
            if not permanent.top_card:
                return False
            tc = permanent.top_card
            traits = getattr(tc, 'card_traits', []) or []
            has_adventure = any('Adventure' in t for t in traits)
            has_hero = any('Hero' in t for t in traits)
            if not (has_adventure or has_hero):
                return False
            level = getattr(permanent, 'level', None)
            return level == 2

        effect0._alt_digi_permanent_condition = alt_digi_permanent_condition
        effect0._alt_digi_cost = 0
        effect0._alt_digi_ignore_requirements = False

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Warp Digivolution into [WarGreymon] for cost 4 ---
        effect1 = ICardEffect()
        effect1.set_effect_name("ST20-10 Warp digi: into WarGreymon cost 4")
        effect1.set_effect_description(
            "[Your Turn] If your opponent has a Digimon with 10000 DP or more, "
            "or you have 3 or more colors among your Tamers, this Digimon may "
            "digivolve into [WarGreymon] in your hand for a memory cost of 4, "
            "ignoring digivolution requirements."
        )

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

        def warp_permanent_condition(permanent) -> bool:
            return permanent is (card.permanent_of_this_card() if card else None)

        def warp_card_condition(card_source) -> bool:
            if card_source is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if card_source not in owner.hand_cards:
                return False
            return card_source.contains_card_name('WarGreymon')

        def warp_condition() -> bool:
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

        effect1._alt_digi_permanent_condition = warp_permanent_condition
        effect1._alt_digi_card_condition = warp_card_condition
        effect1._alt_digi_cost = 4
        effect1._alt_digi_ignore_requirements = True
        effect1._alt_digi_condition = warp_condition

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
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
