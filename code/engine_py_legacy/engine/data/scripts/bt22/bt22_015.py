from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_015(CardScript):
    """BT22-015 Omnimon | Lv.7

    Alt digivolve: from Lv.6 w/ [CS] trait for 5.
    DNA: Lv.6 w/ [Greymon] + Lv.6 w/ [Garurumon] for 0.
    Blocker. Decode (Red/Black Lv.3). Decode (Blue/Yellow Lv.3).
    [On Play] [When Attacking] Delete 1 of your opponent's Digimon with the lowest DP.
    [When Digivolving] For every 2 same-level cards this Digimon's stack has,
        return 1 of your opponent's Digimon to the bottom of the deck.
        Then, this Digimon may attack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.6 w/ CS trait for 5 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-015 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 6
        effect0._alt_digi_trait = "CS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Jogress Condition ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-015 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Blocker ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-015 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: Decode (Red/Black) ---
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-015 Decode")
        effect3.set_effect_description("Decode")
        effect3._is_decode = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # --- Effect 4: Decode (Blue/Yellow) ---
        effect4 = ICardEffect()
        effect4.set_effect_name("BT22-015 Decode")
        effect4.set_effect_description("Decode")
        effect4._is_decode = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # --- Helper: delete lowest DP opponent Digimon ---
        def _delete_lowest_dp(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            enemy_digimon = [p for p in enemy.battle_area
                             if p.is_digimon and p.dp is not None]
            if not enemy_digimon:
                return
            min_dp = min(p.dp for p in enemy_digimon)

            def lowest_dp_filter(p):
                return p.is_digimon and p.dp is not None and p.dp == min_dp

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete,
                filter_fn=lowest_dp_filter,
                is_optional=False)

        # --- Effect 5: [On Play] Delete 1 lowest DP ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("BT22-015 Delete 1 Digimon")
        effect5.set_effect_description(
            "[On Play] Delete 1 of your opponent's Digimon with the lowest DP."
        )
        effect5.is_on_play = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effect5.set_on_process_callback(_delete_lowest_dp)
        effects.append(effect5)

        # --- Effect 6: [When Attacking] Delete 1 lowest DP ---
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnUseAttack)
        effect6.set_effect_name("BT22-015 Delete 1 Digimon")
        effect6.set_effect_description(
            "[When Attacking] Delete 1 of your opponent's Digimon with the lowest DP."
        )
        effect6.is_on_attack = True

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect6.set_can_use_condition(condition6)
        effect6.set_on_process_callback(_delete_lowest_dp)
        effects.append(effect6)

        # --- Effect 7: [When Digivolving] Bottom-deck per 2 same-level, then attack ---
        effect7 = ICardEffect()
        effect7.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect7.set_effect_name("BT22-015 Return digimon, then attack")
        effect7.set_effect_description(
            "[When Digivolving] For every 2 same-level cards this Digimon's "
            "stack has, return 1 of your opponent's Digimon to the bottom of "
            "the deck. Then, this Digimon may attack."
        )
        effect7.is_when_digivolving = True

        def condition7(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect7.set_can_use_condition(condition7)

        def process7(ctx: Dict[str, Any]):
            """Count same-level pairs, bottom-deck that many, then may attack."""
            from ....interfaces.modifiers import ModifierType

            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Count same-level pairs in digi-stack
            level_counts = {}
            for cs in perm.card_sources:
                lvl = getattr(cs, 'level', None)
                if lvl is not None:
                    level_counts[lvl] = level_counts.get(lvl, 0) + 1
            target_count = sum(count // 2 for count in level_counts.values())

            def _grant_may_attack():
                """Then, this Digimon may attack."""
                p = card.permanent_of_this_card() if card else None
                if p and game:
                    game.register_modifier(
                        p, ModifierType.MAY_ATTACK,
                        expiry='end_of_turn',
                    )

            if target_count > 0:
                enemy = player.enemy
                if enemy:
                    def _bottom_deck_loop(remaining):
                        if remaining <= 0:
                            _grant_may_attack()
                            return
                        opp_digimon = [p for p in enemy.battle_area
                                       if p.is_digimon]
                        if not opp_digimon:
                            _grant_may_attack()
                            return

                        def opp_digimon_filter(p):
                            return p.is_digimon

                        def on_bottom_deck(target_perm):
                            enemy.return_permanent_to_deck_bottom(target_perm)
                            _bottom_deck_loop(remaining - 1)

                        game.effect_select_opponent_permanent(
                            player, on_bottom_deck,
                            filter_fn=opp_digimon_filter,
                            is_optional=False)

                    _bottom_deck_loop(target_count)
                else:
                    _grant_may_attack()
            else:
                _grant_may_attack()

        effect7.set_on_process_callback(process7)
        effects.append(effect7)

        return effects
