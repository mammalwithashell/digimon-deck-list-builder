from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_073(CardScript):
    """BT20-073 MetalPhantomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-073 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: [Phantomon] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Phantomon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-073 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ---------------------------------------------------------------
        # Shared On Play / When Digivolving:
        # By deleting 1 of your Digimon, delete 1 of your opponent's
        # level 5 or lower Digimon.
        # ---------------------------------------------------------------

        def _delete_own_then_opp_process(ctx: Dict[str, Any]):
            """Delete own Digimon as cost, then delete opponent's Lv.5- Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def own_digi_filter(p):
                return p.is_digimon

            def on_own_selected(own_perm):
                # Delete own Digimon as cost
                player.delete_permanent(own_perm)

                # Then, delete 1 opponent's Lv.5 or lower Digimon
                def opp_filter(p):
                    if not p.is_digimon:
                        return False
                    if getattr(p, 'level', None) is None:
                        return False
                    return p.level <= 5

                def on_opp_delete(target_perm):
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_opp_delete, filter_fn=opp_filter, is_optional=False,
                    prompt="Select 1 of your opponent's level 5 or lower Digimon to delete.")

            # Select 1 of your own Digimon to delete (optional — "By deleting" = can choose not to)
            game.effect_select_own_permanent(
                player, on_own_selected, filter_fn=own_digi_filter, is_optional=True,
                prompt="Select 1 of your Digimon to delete.")

        # On Play
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT20-073 Delete 1 of your Digimon to delete your opponent's Digimon")
        effect2.set_effect_description("[On Play] By deleting 1 of your Digimon, delete 1 of your opponent's level 5 or lower Digimon.")
        effect2.is_optional = True
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_delete_own_then_opp_process)
        effects.append(effect2)

        # When Digivolving
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT20-073 Delete 1 of your Digimon to delete your opponent's Digimon")
        effect3.set_effect_description("[When Digivolving] By deleting 1 of your Digimon, delete 1 of your opponent's level 5 or lower Digimon.")
        effect3.is_optional = True
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_delete_own_then_opp_process)
        effects.append(effect3)

        # ---------------------------------------------------------------
        # On Deletion ESS: De-Digivolve 1 opponent's Digimon
        # ---------------------------------------------------------------
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("BT20-073 De-Digivolve 1 on 1 Digimon")
        effect4.set_effect_description("[On Deletion] [De-Digivolve] 1 of your opponent's Digimon (Trash up to 1 card from the top of one of your opponent's Digimon. If it has no digivolution cards, or becomes a level 3 Digimon, you can't trash any more cards).")
        effect4.is_inherited_effect = True
        effect4.is_on_deletion = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: De-Digivolve 1 opponent's Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False,
                prompt="Select 1 of your opponent's Digimon to De-Digivolve.")

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
