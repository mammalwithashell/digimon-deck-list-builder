from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_056(CardScript):
    """EX7-056 Orochimon | Lv.5 Purple Dark Dragon

    <Blocker>
    [On Deletion] Trash 1 card in your hand. Then, delete 1 of your
        opponent's level 3 Digimon and level 4 Digimon.

    Inherited: <Retaliation>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Blocker ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX7-056 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Deletion] Trash 1 hand card, then delete opp lv3 + lv4 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("EX7-056 On Deletion: trash hand, delete lv3 and lv4")
        effect1.set_effect_description(
            "[On Deletion] Trash 1 card in your hand. Then, delete 1 of your "
            "opponent's level 3 Digimon and level 4 Digimon."
        )
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Trash 1 card from hand (mandatory)
            def on_hand_trashed(hand_card):
                if hand_card and hand_card in player.hand_cards:
                    player.trash_from_hand([hand_card])

                # Step 2: Delete 1 opponent's level 3 Digimon
                enemy = player.enemy
                if not enemy:
                    return

                def lv3_filter(p):
                    return p.is_digimon and getattr(p, 'level', None) == 3

                def on_delete_lv3(target_perm):
                    enemy.delete_permanent(target_perm)

                    # Step 3: Delete 1 opponent's level 4 Digimon
                    def lv4_filter(p):
                        return p.is_digimon and getattr(p, 'level', None) == 4

                    def on_delete_lv4(target_perm2):
                        enemy.delete_permanent(target_perm2)

                    opp_lv4 = [p for p in enemy.battle_area if lv4_filter(p)]
                    if opp_lv4:
                        game.effect_select_opponent_permanent(
                            player, on_delete_lv4, filter_fn=lv4_filter,
                            is_optional=False,
                            prompt="Delete 1 of your opponent's level 4 Digimon."
                        )

                opp_lv3 = [p for p in enemy.battle_area if lv3_filter(p)]
                if opp_lv3:
                    game.effect_select_opponent_permanent(
                        player, on_delete_lv3, filter_fn=lv3_filter,
                        is_optional=False,
                        prompt="Delete 1 of your opponent's level 3 Digimon."
                    )
                else:
                    # No lv3 targets, still try lv4
                    def lv4_filter(p):
                        return p.is_digimon and getattr(p, 'level', None) == 4

                    def on_delete_lv4(target_perm2):
                        enemy.delete_permanent(target_perm2)

                    opp_lv4 = [p for p in enemy.battle_area if lv4_filter(p)]
                    if opp_lv4:
                        game.effect_select_opponent_permanent(
                            player, on_delete_lv4, filter_fn=lv4_filter,
                            is_optional=False,
                            prompt="Delete 1 of your opponent's level 4 Digimon."
                        )

            if player.hand_cards:
                game.effect_select_hand_card(
                    player,
                    filter_fn=lambda c: True,
                    callback=on_hand_trashed,
                    is_optional=False,
                    prompt="Trash 1 card from your hand."
                )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited <Retaliation> ---
        effect2 = ICardEffect()
        effect2.set_effect_name("EX7-056 Retaliation")
        effect2.set_effect_description("Retaliation")
        effect2.is_inherited_effect = True
        effect2._is_retaliation = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
