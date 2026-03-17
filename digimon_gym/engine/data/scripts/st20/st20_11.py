from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST20_11(CardScript):
    """ST20-11 WarGreymon | Lv.6 Black Digimon | Adventure/Hero | 12000 DP

    Alt Digivolution: [Adventure] or [Hero] trait Lv.5 for cost 3.
    Ace: <Blast Digivolve>
    [On Play] [When Digivolving] Until your opponent's turn ends, for every
        2 colors your Tamers have, their Digimon's effects don't affect 1 of
        your Digimon.
    [When Digivolving] [When Attacking] Delete 1 of your opponent's lowest
        DP Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt Digivolution from [Adventure]/[Hero] trait Lv.5 for cost 3 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("ST20-11 Alt digi: Adventure/Hero Lv.5 cost 3")
        effect0.set_effect_description(
            "Digivolve: Lv.5 w/[Adventure] or [Hero] trait for cost 3."
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
            return permanent.level == 5

        effect0._alt_digi_permanent_condition = alt_digi_permanent_condition
        effect0._alt_digi_cost = 3
        effect0._alt_digi_ignore_requirements = False

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Ace <Blast Digivolve> ---
        effect1 = ICardEffect()
        effect1.set_effect_name("ST20-11 Blast Digivolve")
        effect1.set_effect_description("[Hand] [Counter] <Blast Digivolve>")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Helper: count tamer colors / 2 ---
        def _tamer_two_color_count():
            owner = card.owner if card else None
            if not owner:
                return 0
            color_set = set()
            for p in owner.battle_area:
                if p.is_tamer and p.top_card:
                    for col in (getattr(p.top_card, 'card_colors', []) or []):
                        color_set.add(col)
            return len(color_set) // 2

        # --- Effect 2: [On Play] Immunity from opponent Digimon effects ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("ST20-11 On Play: Immune to opponent Digimon effects")
        effect2.set_effect_description(
            "[On Play] Until your opponent's turn ends, for every 2 colors "
            "your Tamers have, their Digimon's effects don't affect 1 of "
            "your Digimon."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _tamer_two_color_count() > 0
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Grant immunity to N own Digimon from opponent Digimon effects."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            count = _tamer_two_color_count()
            if count <= 0:
                return
            own_digimon = [p for p in player.battle_area if p.is_digimon]
            for perm in own_digimon[:count]:
                perm.grant_keyword('_immune_to_opponent_digimon_effects',
                                   expiry='end_of_opponent_turn')
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [When Digivolving] Same immunity ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("ST20-11 When Digivolving: Immune to opponent Digimon effects")
        effect3.set_effect_description(
            "[When Digivolving] Until your opponent's turn ends, for every "
            "2 colors your Tamers have, their Digimon's effects don't affect "
            "1 of your Digimon."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return _tamer_two_color_count() > 0
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Grant immunity to N own Digimon from opponent Digimon effects."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            count = _tamer_two_color_count()
            if count <= 0:
                return
            own_digimon = [p for p in player.battle_area if p.is_digimon]
            for perm in own_digimon[:count]:
                perm.grant_keyword('_immune_to_opponent_digimon_effects',
                                   expiry='end_of_opponent_turn')
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [When Digivolving] Delete opponent's lowest DP Digimon ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("ST20-11 When Digivolving: Delete lowest DP")
        effect4.set_effect_description(
            "[When Digivolving] Delete 1 of your opponent's lowest DP Digimon."
        )
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            return any(p.is_digimon for p in enemy.battle_area)
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Delete 1 opponent's lowest DP Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def lowest_dp_filter(p):
                if not p.is_digimon:
                    return False
                opp_digimon = [q for q in enemy.battle_area if q.is_digimon and q.dp is not None]
                if not opp_digimon:
                    return False
                min_dp = min(q.dp for q in opp_digimon)
                return p.dp is not None and p.dp == min_dp

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=lowest_dp_filter,
                is_optional=False)
        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # --- Effect 5: [When Attacking] Delete opponent's lowest DP Digimon ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnUseAttack)
        effect5.set_effect_name("ST20-11 When Attacking: Delete lowest DP")
        effect5.set_effect_description(
            "[When Attacking] Delete 1 of your opponent's lowest DP Digimon."
        )

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            ctx_perm = context.get('attacking_permanent') or context.get('permanent')
            if perm and ctx_perm and perm != ctx_perm:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            return any(p.is_digimon for p in enemy.battle_area)
        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Delete 1 opponent's lowest DP Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def lowest_dp_filter(p):
                if not p.is_digimon:
                    return False
                opp_digimon = [q for q in enemy.battle_area if q.is_digimon and q.dp is not None]
                if not opp_digimon:
                    return False
                min_dp = min(q.dp for q in opp_digimon)
                return p.dp is not None and p.dp == min_dp

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=lowest_dp_filter,
                is_optional=False)
        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
