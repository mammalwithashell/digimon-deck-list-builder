from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_101(CardScript):
    """BT18-101 Lucemon: Satan Mode | Lv.7

    [When Digivolving] By playing 1 [Lucemon: Larva] from your trash to your
        empty breeding area without paying the cost, delete 1 of your opponent's
        Digimon or Tamers.
    [End of All Turns] [Once Per Turn] Trash your opponent's top security card.
        If this effect didn't trash, delete 1 of your opponent's Digimon and
        1 of their Tamers.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Digivolving] Play Lucemon: Larva from trash to breeding,
        # then delete 1 opponent Digimon or Tamer
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT18-101 When Digivolving: play Larva, delete opponent")
        effect0.set_effect_description(
            "[When Digivolving] By playing 1 [Lucemon: Larva] from your trash "
            "to your empty breeding area without paying the cost, delete 1 of "
            "your opponent's Digimon or Tamers."
        )
        effect0.is_when_digivolving = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Must have empty breeding area
            if player.breeding_area is not None:
                return False
            # Must have Lucemon: Larva in trash
            has_larva = any(
                getattr(c, 'is_digimon', False) and
                any('Lucemon: Larva' in n or 'Lucemon Larva' in n
                    for n in (getattr(c, 'card_names', []) or []))
                for c in player.trash_cards
            )
            return has_larva
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Find Lucemon: Larva in trash
            def larva_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                names = getattr(c, 'card_names', []) or []
                return any('Lucemon: Larva' in n or 'Lucemon Larva' in n for n in names)

            larva_cards = [c for c in player.trash_cards if larva_filter(c)]
            if not larva_cards or player.breeding_area is not None:
                return

            # Play Larva to breeding area
            larva = larva_cards[0]
            player.trash_cards.remove(larva)
            player.play_card_from_source(larva, pay_cost=False)

            # Delete 1 opponent Digimon or Tamer
            enemy = player.enemy
            if enemy:
                def target_filter(p):
                    return p.is_digimon or p.is_tamer

                def on_delete(target_perm):
                    if target_perm in enemy.battle_area:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [End of All Turns] [Once Per Turn] Trash opponent's top security.
        # If didn't trash, delete 1 opponent Digimon and 1 opponent Tamer.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.EndOfTurn)
        effect1.set_effect_name("BT18-101 End of All Turns: trash security or delete")
        effect1.set_effect_description(
            "[End of All Turns] [Once Per Turn] Trash your opponent's top security "
            "card. If this effect didn't trash, delete 1 of your opponent's Digimon "
            "and 1 of their Tamers."
        )
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("EndAllTurns_BT18_101")

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
            enemy = player.enemy
            if not enemy:
                return

            if enemy.security_cards:
                # Trash opponent's top security
                trashed = enemy.security_cards.pop(0)
                enemy.trash_cards.append(trashed)
            else:
                # Didn't trash -> delete 1 Digimon and 1 Tamer
                opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
                if opp_digimon:
                    def on_delete_digimon(target_perm):
                        if target_perm in enemy.battle_area:
                            enemy.delete_permanent(target_perm)
                        # Then delete 1 Tamer
                        opp_tamers = [p for p in enemy.battle_area if p.is_tamer]
                        if opp_tamers:
                            def on_delete_tamer(tamer_perm):
                                if tamer_perm in enemy.battle_area:
                                    enemy.delete_permanent(tamer_perm)
                            game.effect_select_opponent_permanent(
                                player, on_delete_tamer,
                                filter_fn=lambda p: p.is_tamer,
                                is_optional=False)

                    game.effect_select_opponent_permanent(
                        player, on_delete_digimon,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False)
                else:
                    # No Digimon, still try to delete a Tamer
                    opp_tamers = [p for p in enemy.battle_area if p.is_tamer]
                    if opp_tamers:
                        def on_delete_tamer2(tamer_perm):
                            if tamer_perm in enemy.battle_area:
                                enemy.delete_permanent(tamer_perm)
                        game.effect_select_opponent_permanent(
                            player, on_delete_tamer2,
                            filter_fn=lambda p: p.is_tamer,
                            is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
