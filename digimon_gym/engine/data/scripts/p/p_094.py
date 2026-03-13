from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_094(CardScript):
    """P-094 Destromon | Lv.5

    DigiXros -1: [Snatchmon] x 1 + [Vemmon] x 4. When this would be played,
    you may place specified cards from your hand/battle area under it. Each
    reduces the play cost.

    [On Play] [When Digivolving] Delete your opponent's Digimon and Tamers with
    a total play cost of 3. For every [Vemmon] in this Digimon's digivolution
    cards, increase the maximum play cost you can choose by this effect by 1.

    --- Inherited ---
    [Opponent's Turn] [Once Per Turn] When an opponent's Digimon attacks, by
    placing 2 [Vemmon] from 1 of your [Galacticmon]'s digivolution cards at
    the bottom of their owners' decks, switch the target of attack to this
    Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _count_vemmon_in_digi_cards(perm) -> int:
            """Count [Vemmon] in digivolution cards (exclude top card)."""
            if perm is None or len(perm.card_sources) <= 1:
                return 0
            return sum(
                1 for cs in perm.card_sources[:-1]
                if any('Vemmon' in n for n in getattr(cs, 'card_names', []))
            )

        def _max_budget(perm) -> int:
            """Base budget 3 + 1 per [Vemmon] in digi-cards."""
            return 3 + _count_vemmon_in_digi_cards(perm)

        # ─── Shared On Play / When Digivolving: budget-based multi-delete
        def _shared_budget_delete(ctx: Dict[str, Any]):
            """Delete opponent's Digimon and Tamers with total play cost up to budget."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            budget = _max_budget(perm)

            # Find all eligible opponent Digimon and Tamers within budget
            eligible = [
                p for p in enemy.battle_area
                if (p.is_digimon or getattr(p, 'is_tamer', False))
                and p.top_card
                and p.top_card.get_cost_itself <= budget
            ]
            if not eligible:
                return

            # Greedy budget-based deletion: repeatedly pick targets whose cost
            # fits within remaining budget. The engine's select-opponent-permanent
            # is single-select, so we loop.
            remaining_budget = [budget]  # mutable container for closure

            def _do_delete_pass():
                """Run one selection pass; recurse if budget remains."""
                current_budget = remaining_budget[0]

                def target_filter(p):
                    return (
                        (p.is_digimon or getattr(p, 'is_tamer', False))
                        and p.top_card
                        and p.top_card.get_cost_itself <= current_budget
                    )

                candidates = [p for p in enemy.battle_area if target_filter(p)]
                if not candidates:
                    return

                def on_delete(target_perm):
                    cost = target_perm.top_card.get_cost_itself if target_perm.top_card else 0
                    remaining_budget[0] -= cost
                    enemy.delete_permanent(target_perm)
                    # Continue selecting if budget remains
                    if remaining_budget[0] > 0:
                        _do_delete_pass()

                # If only 1 candidate, auto-select it
                if len(candidates) == 1:
                    on_delete(candidates[0])
                else:
                    game.effect_select_opponent_permanent(
                        player, on_delete, filter_fn=target_filter,
                        is_optional=True)

            _do_delete_pass()

        # ─── Effect 0: [On Play]
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("P-094 Budget delete Digimon/Tamers")
        effect0.set_effect_description("[On Play] Delete your opponent's Digimon and Tamers with a total play cost of 3. For every [Vemmon] in this Digimon's digivolution cards, increase the maximum play cost you can choose by this effect by 1.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_shared_budget_delete)
        effects.append(effect0)

        # ─── Effect 1: [When Digivolving]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("P-094 Budget delete Digimon/Tamers")
        effect1.set_effect_description("[When Digivolving] Delete your opponent's Digimon and Tamers with a total play cost of 3. For every [Vemmon] in this Digimon's digivolution cards, increase the maximum play cost you can choose by this effect by 1.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_shared_budget_delete)
        effects.append(effect1)

        # ─── Effect 2 (Inherited): Redirect attack
        # BLOCKED: redirect_attack / SwitchDefender is not implemented in the engine.
        # The inherited effect requires switching the attack target to this Digimon
        # after placing 2 [Vemmon] from a [Galacticmon]'s digi-cards to deck bottom.
        # Engine gap: no redirect_attack / SwitchDefender mechanic.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("P-094 Redirect attack (BLOCKED — engine gap)")
        effect2.set_effect_description("[Opponent's Turn] [Once Per Turn] When an opponent's Digimon attacks, by placing 2 [Vemmon] from 1 of your [Galacticmon]'s digivolution cards at the bottom of their owners' decks, switch the target of attack to this Digimon.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("SwitchAttackTarget_P_094")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Redirect Attack — BLOCKED: redirect_attack not in engine."""
            pass  # descriptive-tagged: redirect_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
