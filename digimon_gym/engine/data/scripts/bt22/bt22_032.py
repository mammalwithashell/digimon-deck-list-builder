from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_032(CardScript):
    """BT22-032 ShoeShoemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        # --- Effect 0: [On Deletion] Play 1 level 3 [Puppet] Digimon from hand ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("BT22-032 Play 1 level 3 [Puppet] digimon")
        effect0.set_effect_description("[On Deletion] You may play 1 level 3 Digimon card with the [Puppet] trait from your hand without paying the cost.")
        effect0.is_optional = True
        effect0.is_on_deletion = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level is None or level != 3:
                    return False
                return _is_puppet_trait(c)

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True,
                prompt="You may play 1 level 3 [Puppet] Digimon from hand.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1 (Inherited): [When Attacking][Once Per Turn] -2000 DP ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT22-032 -2K DP")
        effect1.set_effect_description("[When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT22_032_WA")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
