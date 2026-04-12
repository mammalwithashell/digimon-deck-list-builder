from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_043(CardScript):
    """LM-043 Darkdramon | Lv.6 Black/Purple | Cyborg, D-Brigade, ACCEL | 12000 DP | Cost 7

    [Hand] [Counter] Blast Digivolve (Your Digimon may digivolve into this
        card without paying the cost.)
    Scapegoat (When this Digimon would be deleted other than by your effects,
        by deleting 1 of your other Digimon, prevent that deletion.)
    [On Play] [When Digivolving] De-Digivolve 1 1 of your opponent's Digimon.
        Then, delete all of their Digimon with the lowest play cost.
    Inherited: Ace Overflow -4
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Blast Digivolve ---
        effect0 = ICardEffect()
        effect0.set_effect_name("LM-043 Blast Digivolve")
        effect0.set_effect_description("[Hand] [Counter] <Blast Digivolve>")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Scapegoat ---
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-043 Scapegoat")
        effect1.set_effect_description(
            "<Scapegoat> (When this Digimon would be deleted other than by "
            "your effects, by deleting 1 of your other Digimon, prevent that "
            "deletion.)"
        )
        effect1._is_scapegoat = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared process for On Play / When Digivolving ---
        def _de_digivolve_one_then_delete_lowest_cost(ctx: Dict[str, Any]):
            """De-Digivolve 1 one opponent Digimon, then delete all with lowest play cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Step 1: De-Digivolve 1 one of opponent's Digimon
            def dedigivolve_filter(p):
                return p.is_digimon

            def on_dedigivolve(target_perm):
                if len(target_perm.card_sources) > 1:
                    removed = target_perm.de_digivolve(1)
                    for rc in removed:
                        enemy.trash_cards.append(rc)

                # Step 2: Delete all opponent Digimon with the lowest play cost
                opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
                if not opp_digimon:
                    return

                def get_play_cost(p):
                    if p.top_card and p.top_card.c_entity_base:
                        return p.top_card.c_entity_base.play_cost or 0
                    return 0

                min_cost = min(get_play_cost(p) for p in opp_digimon)
                to_delete = [p for p in opp_digimon if get_play_cost(p) == min_cost]
                for perm in to_delete:
                    if perm in enemy.battle_area:
                        enemy.delete_permanent(perm)

            game.effect_select_opponent_permanent(
                player, on_dedigivolve, filter_fn=dedigivolve_filter,
                is_optional=False)

        # --- Effect 2: [On Play] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("LM-043 On Play: De-Digivolve 1 + delete lowest cost")
        effect2.set_effect_description(
            "[On Play] De-Digivolve 1 1 of your opponent's Digimon. Then, "
            "delete all of their Digimon with the lowest play cost."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_de_digivolve_one_then_delete_lowest_cost)
        effects.append(effect2)

        # --- Effect 3: [When Digivolving] ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("LM-043 When Digivolving: De-Digivolve 1 + delete lowest cost")
        effect3.set_effect_description(
            "[When Digivolving] De-Digivolve 1 1 of your opponent's Digimon. "
            "Then, delete all of their Digimon with the lowest play cost."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_de_digivolve_one_then_delete_lowest_cost)
        effects.append(effect3)

        # --- Effect 4: Inherited Ace Overflow -4 ---
        # Engine handles ACE Overflow automatically via entity_base.is_ace /
        # ace_overflow_cost parsed from inherited_effect_description_eng.
        # This effect tag documents the inherited effect in the script.
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("LM-043 Ace Overflow <-4>")
        effect_ace.set_effect_description("Ace Overflow <-4>")
        effect_ace.is_inherited_effect = True
        effect_ace._is_ace_overflow = True
        effect_ace._overflow_amount = -4

        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
