from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_070(CardScript):
    """BT24-070 Growlmon | Lv.4

    [On Play] [When Digivolving] If you have 4 or fewer cards in your hand,
        you may play 1 purple Tamer card with a play cost of 4 or less from
        your trash without paying the cost.
    Inherited: [When Attacking] [Once Per Turn] Delete 1 of your opponent's
        level 3 Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Shared On Play / When Digivolving logic ---
        def _shared_process(ctx: Dict[str, Any]):
            """If 4 or fewer hand cards, play 1 purple Tamer (cost 4 or less)
            from trash without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Check hand size condition
            if len(player.hand_cards) > 4:
                return

            def play_filter(c):
                if not c.is_tamer:
                    return False
                if c.get_cost_itself > 4:
                    return False
                return CardColor.Purple in c.card_colors

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True,
                prompt="Play 1 purple Tamer (cost 4 or less) from trash.")

        # --- Effect 0: [On Play] ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT24-070 Play purple Tamer from trash")
        effect0.set_effect_description(
            "[On Play] If you have 4 or fewer cards in your hand, you may play "
            "1 purple Tamer card with a play cost of 4 or less from your trash "
            "without paying the cost.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_shared_process)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-070 Play purple Tamer from trash")
        effect1.set_effect_description(
            "[When Digivolving] If you have 4 or fewer cards in your hand, you may "
            "play 1 purple Tamer card with a play cost of 4 or less from your trash "
            "without paying the cost.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_shared_process)
        effects.append(effect1)

        # --- Effect 2: Inherited [When Attacking] [Once Per Turn]
        #     Delete 1 opponent's level 3 Digimon ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT24-070 Delete opponent's level 3 Digimon")
        effect2.set_effect_description(
            "[When Attacking] [Once Per Turn] Delete 1 of your opponent's level 3 Digimon.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT24_070_Inherited")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete 1 of opponent's level 3 Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon and p.level is not None and p.level == 3

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            if any(target_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter,
                    is_optional=False,
                    prompt="Delete 1 of your opponent's level 3 Digimon.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
