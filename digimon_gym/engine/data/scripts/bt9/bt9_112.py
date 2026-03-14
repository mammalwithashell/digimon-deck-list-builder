from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT9_112(CardScript):
    """BT9-112 DeathXmon | Lv.7 Purple/Black | Unanalyzable, X Program | 15000 DP | Cost 20

    When you would play this card, reduce its memory cost by 3 for each
        Digimon and Tamer your opponent has in play.
    [On Play] [When Digivolving] De-Digivolve 1 all of your opponent's Digimon.
        Then, delete all of your opponent's level 4 or lower Digimon.
    [End of Opponent's Turn] [Once Per Turn] Delete all of your opponent's
        Digimon with the lowest play cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: BeforePayCost - reduce cost by 3 per opponent's Digimon/Tamer ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT9-112 Cost reduction per opponent Digimon/Tamer")
        effect0.set_effect_description(
            "When you would play this card, reduce its memory cost by 3 for "
            "each Digimon and Tamer your opponent has in play."
        )

        def _cost_reduction_fn(context):
            player = card.owner if card else None
            if not player or not player.enemy:
                return 0
            enemy = player.enemy
            count = sum(1 for p in enemy.battle_area if p.is_digimon or p.is_tamer)
            return 3 * count

        effect0._cost_reduction_value_fn = _cost_reduction_fn

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reduce play cost by 3 per opponent's Digimon and Tamer in play."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            count = sum(
                1 for p in enemy.battle_area
                if p.is_digimon or p.is_tamer
            )
            reduction = 3 * count
            if reduction > 0:
                if hasattr(player, '_temp_play_cost_reduction'):
                    player._temp_play_cost_reduction += reduction
                else:
                    player._temp_play_cost_reduction = reduction
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Shared process for On Play / When Digivolving ---
        def _de_digivolve_all_then_delete_lv4(ctx: Dict[str, Any]):
            """De-Digivolve 1 all opponent Digimon, then delete all Lv.4 or lower."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Step 1: De-Digivolve 1 all opponent Digimon
            for perm in list(enemy.battle_area):
                if perm.is_digimon and len(perm.card_sources) > 1:
                    removed = perm.de_digivolve(1)
                    for rc in removed:
                        enemy.trash_cards.append(rc)

            # Step 2: Delete all opponent's Lv.4 or lower Digimon
            to_delete = [
                p for p in enemy.battle_area
                if p.is_digimon and p.level is not None and p.level <= 4
            ]
            for perm in to_delete:
                if perm in enemy.battle_area:
                    enemy.delete_permanent(perm)

        # --- Effect 1: [On Play] ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT9-112 On Play: De-Digivolve all + delete Lv.4-")
        effect1.set_effect_description(
            "[On Play] De-Digivolve 1 all of your opponent's Digimon. Then, "
            "delete all of your opponent's level 4 or lower Digimon."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_de_digivolve_all_then_delete_lv4)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT9-112 When Digivolving: De-Digivolve all + delete Lv.4-")
        effect2.set_effect_description(
            "[When Digivolving] De-Digivolve 1 all of your opponent's Digimon. "
            "Then, delete all of your opponent's level 4 or lower Digimon."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_de_digivolve_all_then_delete_lv4)
        effects.append(effect2)

        # --- Effect 3: [End of Opponent's Turn] [Once Per Turn] ---
        # Delete all opponent's Digimon with the lowest play cost.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndTurn)
        effect3.set_effect_name("BT9-112 End of Opponent's Turn: Delete lowest cost Digimon")
        effect3.set_effect_description(
            "[End of Opponent's Turn] [Once Per Turn] Delete all of your "
            "opponent's Digimon with the lowest play cost."
        )
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EndOppTurn_BT9_112")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only on opponent's turn
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Delete all opponent Digimon with the lowest play cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            opp_digimon = [
                p for p in enemy.battle_area if p.is_digimon
            ]
            if not opp_digimon:
                return

            # Find the lowest play cost among opponent's Digimon
            def get_play_cost(p):
                if p.top_card and p.top_card.c_entity_base:
                    return p.top_card.c_entity_base.play_cost or 0
                return 0

            min_cost = min(get_play_cost(p) for p in opp_digimon)
            to_delete = [p for p in opp_digimon if get_play_cost(p) == min_cost]
            for perm in to_delete:
                if perm in enemy.battle_area:
                    enemy.delete_permanent(perm)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
