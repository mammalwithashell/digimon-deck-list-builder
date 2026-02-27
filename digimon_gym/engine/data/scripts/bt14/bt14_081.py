from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_081(CardScript):
    """BT14-081 Fenriloogamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Digivolving] You may play 1 level 4 or lower card with the [Dark Animal] or [SoC] trait from your trash without paying the cost.
        # If [Eiji Nagasumi] is in this Digimon's digivolution cards, add 2 to the number of cards this effect may play.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-081 Play Digimon cards from trash")
        effect0.set_effect_description("[When Digivolving] You may play 1 level 4 or lower card with the [Dark Animal] or [SoC] trait from your trash without paying the cost. If [Eiji Nagasumi] is in this Digimon's digivolution cards, add 2 to the number of cards this effect may play.")
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            return not (card and card.permanent_of_this_card() is None)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            evo_cards = getattr(perm, 'digivolution_cards', None) or []
            has_eiji = any(getattr(dc, 'card_id', '') == 'BT14-087' or getattr(dc, 'name', '') == 'Eiji Nagasumi' for dc in evo_cards)
            play_count = 3 if has_eiji else 1

            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                traits = getattr(c, 'type_eng', None) or getattr(c, 'types', None) or []
                return ('Dark Animal' in traits) or ('SoC' in traits)

            for _ in range(play_count):
                game.effect_play_from_zone(player, 'trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [When Attacking][Once Per Turn] By deleting 1 of your opponent's level 3 or lower Digimon, unsuspend this Digimon.
        # For each of your Digimon, add 1 to the level this effect may choose.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-081 Delete 1 opponent's Digimon to unsuspend this Digimon")
        effect1.set_effect_description("[When Attacking][Once Per Turn] By deleting 1 of your opponent's level 3 or lower Digimon, unsuspend this Digimon. For each of your Digimon, add 1 to the level this effect may choose.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Delete_BT14_081")
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            return not (card and card.permanent_of_this_card() is None)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            my_digimon = getattr(player, 'battle_area', None) or []
            max_level = 3 + len(my_digimon)

            def target_filter(p):
                lvl = getattr(getattr(p, 'card', None), 'level', None)
                return lvl is not None and lvl <= max_level

            def on_delete(target_perm):
                game.delete_digimon(target_perm)
                perm.unsuspend()

            game.effect_select_opponent_permanent(player, on_delete, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Your Turn] Your turn end condition is the opponent having 3 or more memory.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-081 The turn end condition is the opponent having 3 or more memory.")
        effect2.set_effect_description("[Your Turn] Your turn end condition is the opponent having 3 or more memory.")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return bool(card and card.owner and card.owner.is_my_turn)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            game = ctx.get('game')
            if game is None:
                return
            if hasattr(game, 'set_turn_end_memory_threshold'):
                game.set_turn_end_memory_threshold(3)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
