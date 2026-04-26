from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_029(CardScript):
    """BT22-029 Shoemon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        def _puppet_digimon_filter(p):
            """Filter for own Digimon with [Puppet] trait."""
            if not p.is_digimon:
                return False
            if p.top_card and _is_puppet_trait(p.top_card):
                return True
            return False

        # --- Shared: Grant Blocker to 1 Puppet Digimon until opponent's turn ends ---
        def _grant_blocker(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_grant(target_perm):
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    target_perm, ModifierType.GRANT_BLOCKER,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')

            game.effect_select_own_permanent(
                player, on_grant, filter_fn=_puppet_digimon_filter,
                is_optional=False,
                prompt="Select 1 of your [Puppet] trait Digimon to gain <Blocker>.")

        # --- Effect 0: [On Play] 1 Puppet Digimon gains Blocker ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT22-029 1 [Puppet] digimon gain <Blocker>")
        effect0.set_effect_description("[On Play] 1 of your Digimon with the [Puppet] trait gains <Blocker> until your opponent's turn ends.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_grant_blocker)
        effects.append(effect0)

        # --- Effect 1: [On Deletion] 1 Puppet Digimon gains Blocker ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT22-029 1 [Puppet] digimon gain <Blocker>")
        effect1.set_effect_description("[On Deletion] 1 of your Digimon with the [Puppet] trait gains <Blocker> until your opponent's turn ends.")
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_grant_blocker)
        effects.append(effect1)

        # --- Effect 2 (Inherited): [When Attacking][Once Per Turn] -2000 DP ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAllyAttack)
        effect2.set_effect_name("BT22-029 -2K DP")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT22_029_WA")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return

            def target_filter(p):
                return p.is_digimon

            def on_select(target_perm):
                target_perm.change_dp(-2000)

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False,
                prompt="Select 1 opponent's Digimon to give -2000 DP."
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
