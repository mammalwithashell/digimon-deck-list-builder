from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any

from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_017(CardScript):
    """BT20-017 Jesmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT20-017 play Atho, Rene & Por Token")
        effect0.set_effect_description(
            "[On Play] Play 1 [Atho, Rene & Por] Token. (Digimon/White/6000 DP/<Reboot>/<Blocker>/<Decoy>)"
        )
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            return bool(card and card.permanent_of_this_card() is not None)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get("player")
            game = ctx.get("game")
            if player and game:
                game.effect_play_token(player, "atho_rene_por")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT20-017 play Atho, Rene & Por Token")
        effect1.set_effect_description(
            "[When Digivolving] Play 1 [Atho, Rene & Por] Token. (Digimon/White/6000 DP/<Reboot>/<Blocker>/<Decoy>)"
        )
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            return bool(card and card.permanent_of_this_card() is not None)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get("player")
            game = ctx.get("game")
            if player and game:
                game.effect_play_token(player, "atho_rene_por")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT20-017 delete 8000 DP or less, then 1 Digimon may attack")
        effect2.set_effect_description(
            "[Your Turn][Once Per Turn] When any of your other Digimon are played, delete 1 of your opponent's Digimon with 8000 DP or less. Then, 1 of your Digimon may attack."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("PlayLevel6_BT20_017")

        def condition2(context: Dict[str, Any]) -> bool:
            if card is None or card.permanent_of_this_card() is None:
                return False
            if not (card.owner and card.owner.is_my_turn):
                return False
            played_perm = context.get("played_permanent")
            owner_perm = card.permanent_of_this_card()
            return played_perm is not None and played_perm is not owner_perm and played_perm.top_card.owner is card.owner

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get("player")
            game = ctx.get("game")
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and p.dp is not None and p.dp <= 8000

            def on_attack(selected_perm):
                pass  # descriptive-tagged: force_attack

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
                # Then, 1 of your Digimon may attack
                game.effect_select_own_permanent(
                    player, on_attack,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=True,
                    prompt="Select a Digimon to attack."
                )

            game.effect_select_opponent_permanent(player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
