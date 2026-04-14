from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_182(CardScript):
    """P-182 WarGreymon | Lv.6 Red Digimon | Dragonkin, ADVENTURE

    Security A. +1, Blocker.
    [When Digivolving] Delete 1 of your opponent's Digimon with as much or less
        DP as this Digimon.
    [All Turns] This Digimon gets +1000 DP for each color Digimon and Tamers have.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        effect0 = ICardEffect()
        effect0.set_effect_name("P-182 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "ADVENTURE"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('ADVENTURE' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: security_attack_plus
        effect1 = ICardEffect()
        effect1.set_effect_name("P-182 Security Attack +1")
        effect1.set_effect_description("Security Attack +1")
        effect1._security_attack_modifier = 1

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("P-182 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # [When Digivolving] Delete 1 opponent Digimon with DP <= this Digimon's DP
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("P-182 Delete 1 opponent Digimon with DP <= this Digimon")
        effect3.set_effect_description(
            "[When Digivolving] Delete 1 of your opponent's Digimon with as much "
            "or less DP as this Digimon."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Delete 1 opponent Digimon with DP <= this Digimon's DP."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            my_dp = perm.dp or 0

            def target_filter(p):
                if not p.is_digimon:
                    return False
                opp_dp = p.dp
                if opp_dp is None:
                    return False
                return opp_dp <= my_dp

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # [All Turns] +1000 DP for each color among Digimon and Tamers
        effect4 = ICardEffect()
        effect4.set_effect_name("P-182 DP per color")
        effect4.set_effect_description(
            "[All Turns] This Digimon gets +1000 DP for each color Digimon and "
            "Tamers have."
        )
        # Dynamic DP modifier - can't use static dp_modifier since the value changes
        # We'll compute it in the condition and set the value

        def _count_colors():
            """Count distinct colors among all Digimon and Tamers on both fields."""
            colors = set()
            owner = card.owner if card else None
            if not owner:
                return 0
            # Scan both players' battle areas
            for player in [owner, owner.enemy] if owner.enemy else [owner]:
                for p in player.battle_area:
                    if p.is_digimon or p.is_tamer:
                        if p.top_card:
                            for col in (getattr(p.top_card, 'card_colors', []) or []):
                                colors.add(col)
            return len(colors)

        # Use a dynamic dp_modifier by setting it in condition
        # Default non-zero so get_active_effects() will evaluate the condition
        effect4.dp_modifier = 1000

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            color_count = _count_colors()
            if color_count == 0:
                return False
            effect4.dp_modifier = color_count * 1000
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            pass  # Declarative DP modifier
        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
