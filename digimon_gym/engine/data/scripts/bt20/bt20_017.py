from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_017(CardScript):
    """BT20-017 Jesmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Play 1 [Atho, René & Por] Token. (Digimon/White/6000 DP/<Reboot>/<Blocker>/<Decoy Red/Black>)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-017 Play a token")
        effect0.set_effect_description("[On Play] Play 1 [Atho, René & Por] Token. (Digimon/White/6000 DP/<Reboot>/<Blocker>/<Decoy Red/Black>)")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Play 1 [Atho, René & Por] Token. (Digimon/White/6000 DP/<Reboot> <Blocker> <Decoy Red/Black>)
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-017 Play token")
        effect1.set_effect_description("[When Digivolving] Play 1 [Atho, René & Por] Token. (Digimon/White/6000 DP/<Reboot> <Blocker> <Decoy Red/Black>)")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] [Once Per Turn] When any of your other Digimon are played, delete 1 of your opponent's Digimon with 8000 DP or less. Then, 1 of you Digimon may attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-017 Delete 8k DP or less, Then 1 Digimon may attack.")
        effect2.set_effect_description("[Your Turn] [Once Per Turn] When any of your other Digimon are played, delete 1 of your opponent's Digimon with 8000 DP or less. Then, 1 of you Digimon may attack.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("PlayLevel6_BT20_017")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.dp is None or p.dp > 8000:
                    return False
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
