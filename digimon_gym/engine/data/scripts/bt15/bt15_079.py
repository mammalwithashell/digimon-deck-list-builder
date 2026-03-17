from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_079(CardScript):
    """BT15-079 Piedmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Shared delete logic for On Play / When Attacking ---
        # Per C#: Delete 1 of opponent's UNSUSPENDED Digimon
        def _delete_unsuspended_process(ctx: Dict[str, Any]):
            """Delete 1 of opponent's unsuspended Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and not p.is_suspended

            enemy = player.enemy
            if not enemy:
                return
            has_target = any(target_filter(p) for p in enemy.battle_area)
            if not has_target:
                return

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        # [On Play] Delete 1 of opponent's unsuspended Digimon
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT15-079 Delete 1 unsuspended Digimon")
        effect0.set_effect_description("[On Play] Delete 1 of your opponent's unsuspended Digimon.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_delete_unsuspended_process)
        effects.append(effect0)

        # [When Attacking] Delete 1 of opponent's unsuspended Digimon
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT15-079 Delete 1 unsuspended Digimon")
        effect1.set_effect_description("[When Attacking] Delete 1 of your opponent's unsuspended Digimon.")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_delete_unsuspended_process)
        effects.append(effect1)

        # [End of Opponent's Turn] Delete THIS Digimon. Then, may play 1 Digimon
        # with [Dark Masters] trait (not [Piedmon]) from hand without paying cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT15-079 Delete this Digimon and play 1 Digimon from hand")
        effect2.set_effect_description("[End of Opponent's Turn] Delete this Digimon. Then, you may play 1 Digimon card with the [Dark Masters] trait, other than [Piedmon] from your hand without paying the cost.")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only fires on opponent's turn
            if not (card and card.owner and not card.owner.is_my_turn):
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete self, then optionally play Dark Masters Digimon (not Piedmon) from hand free."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            # Delete THIS Digimon (self)
            player.delete_permanent(perm)

            # Then, may play 1 Digimon with [Dark Masters] trait (not Piedmon) from hand
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                card_names = getattr(c, 'card_names', []) or []
                if any('Piedmon' in n for n in card_names):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Dark Masters' in t or 'DarkMasters' in t for t in traits)

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Inherited: Retaliation
        effect3 = ICardEffect()
        effect3.set_effect_name("BT15-079 Retaliation")
        effect3.set_effect_description("Retaliation")
        effect3.is_inherited_effect = True
        effect3.is_on_deletion = True
        effect3._is_retaliation = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
