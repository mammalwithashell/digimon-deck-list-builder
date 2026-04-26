from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_043(CardScript):
    """BT19-043 Lucemon (X Antibody) | Lv.6

    [All Turns] [Once Per Turn] When this Digimon would leave the battle area,
        if a card with [Lucemon] in its name is in this Digimon's digivolution
        cards, by trashing both players' top security cards, it doesn't leave.
    [End of Your Turn] [Once Per Turn] Your opponent may trash their top
        security card. If this effect didn't trash, Recovery +1 (Deck) and
        delete 1 of your opponent's Digimon or Tamers.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Alt digi from Lucemon for cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-043 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_name = "Lucemon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [All Turns] [Once Per Turn] When this Digimon would leave,
        # if [Lucemon] in digi cards, trash both players' top security, don't leave
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenRemoveField)
        effect1.set_effect_name("BT19-043 Trash both security to prevent leaving")
        effect1.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon would leave the battle "
            "area, if a card with [Lucemon] in its name is in this Digimon's "
            "digivolution cards, by trashing both players' top security cards, "
            "it doesn't leave."
        )
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("TrashSecurityToStay_LucemonXAntibody_BT19_043")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            # Check for [Lucemon] in digivolution cards
            has_lucemon = any(
                any('Lucemon' in n for n in (getattr(cs, 'card_names', []) or []))
                for cs in perm.card_sources[:-1]
            )
            if not has_lucemon:
                return False
            # Must have security to trash
            player = card.owner if card else None
            if not player or not player.security_cards:
                return False
            enemy = player.enemy
            if not enemy or not enemy.security_cards:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Trash both players' top security
            if player.security_cards:
                trashed = player.security_cards.pop(0)
                player.trash_cards.append(trashed)
            enemy = player.enemy
            if enemy and enemy.security_cards:
                trashed = enemy.security_cards.pop(0)
                enemy.trash_cards.append(trashed)
            # Prevent leaving
            ctx['prevent_leave'] = True

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [End of Your Turn] [Once Per Turn]
        # Opponent may trash their top security. If didn't trash,
        # Recovery +1 and delete 1 opponent Digimon or Tamer.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.EndOfTurn)
        effect2.set_effect_name("BT19-043 End of Turn: opponent may trash or recovery+delete")
        effect2.set_effect_description(
            "[End of Your Turn] [Once Per Turn] Your opponent may trash their "
            "top security card. If this effect didn't trash, Recovery +1 (Deck) "
            "and delete 1 of your opponent's Digimon or Tamers."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("EndTurn_LucemonXAntibody_BT19_043")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
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

            # Opponent may trash their top security
            if enemy.security_cards:
                # Simplified: opponent auto-trashes (AI decision)
                trashed = enemy.security_cards.pop(0)
                enemy.trash_cards.append(trashed)
            else:
                # Didn't trash -> Recovery +1 and delete
                player.recovery(1)

                def target_filter(p):
                    return p.is_digimon or p.is_tamer

                def on_delete(target_perm):
                    if target_perm in enemy.battle_area:
                        enemy.delete_permanent(target_perm)

                if any(target_filter(p) for p in enemy.battle_area):
                    game.effect_select_opponent_permanent(
                        player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
