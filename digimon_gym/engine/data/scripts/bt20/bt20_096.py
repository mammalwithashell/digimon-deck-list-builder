from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_096(CardScript):
    """BT20-096 Black Sabbath | Option

    [Trash] [Main] If you have 4 or fewer cards in your hand, by paying 6
        cost, return this card to the bottom of the deck and delete 1 of your
        opponent's unsuspended Digimon.
    [Main] Trash 1 card in your hand. Then, delete 1 of your opponent's
        level 4 or lower Digimon.
    [Security] Delete 1 of your opponent's level 6 or lower Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Trash] [Main] If 4 or fewer hand cards, pay 6, return to deck bottom,
        # delete 1 opponent unsuspended Digimon
        # Note: Trash trigger effects are complex; simplified as descriptive
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-096 Trash trigger: delete unsuspended Digimon")
        effect0.set_effect_description(
            "[Trash] [Main] If you have 4 or fewer cards in your hand, by paying "
            "6 cost, return this card to the bottom of the deck and delete 1 of "
            "your opponent's unsuspended Digimon."
        )
        effect0._is_trash_trigger = True

        def condition0(context: Dict[str, Any]) -> bool:
            return False  # Trash triggers not fully supported; main effect covers use case
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [Main] Trash 1 card in hand. Then, delete 1 opponent Lv4 or lower Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT20-096 Main: trash hand card, delete Lv4- Digimon")
        effect1.set_effect_description(
            "[Main] Trash 1 card in your hand. Then, delete 1 of your opponent's "
            "level 4 or lower Digimon."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # First: trash 1 card in hand
            def hand_filter(c):
                return True

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)

                # Then: delete 1 opponent Lv4 or lower Digimon
                enemy = player.enemy
                if not enemy:
                    return

                def target_filter(p):
                    return p.is_digimon and p.level is not None and p.level <= 4

                def on_delete(target_perm):
                    if target_perm in enemy.battle_area:
                        enemy.delete_permanent(target_perm)

                if any(target_filter(p) for p in enemy.battle_area):
                    game.effect_select_opponent_permanent(
                        player, on_delete, filter_fn=target_filter, is_optional=False)

            if player.hand_cards:
                game.effect_select_hand_card(
                    player, hand_filter, on_trashed, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Security] Delete 1 opponent Lv6 or lower Digimon
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT20-096 Security: delete Lv6 or lower Digimon")
        effect2.set_effect_description(
            "[Security] Delete 1 of your opponent's level 6 or lower Digimon."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon and p.level is not None and p.level <= 6

            def on_delete(target_perm):
                if target_perm in enemy.battle_area:
                    enemy.delete_permanent(target_perm)

            if any(target_filter(p) for p in enemy.battle_area):
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
