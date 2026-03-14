from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT9_069(CardScript):
    """BT9-069 Baihumon | Lv.6 Mega (Black, Data, Holy Beast/Four Sovereigns)

    [When Digivolving] Unsuspend up to 2 Digimon and/or Tamers. Then, gain
        1 memory for each of your opponent's unsuspended Digimon and Tamers.
    [End of Your Turn] [Once Per Turn] For every 2 unsuspended Digimon
        and/or Tamers your opponent has in play, trash 1 card from the top
        of your opponent's security stack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 1: [When Digivolving] Unsuspend up to 2, gain memory ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT9-069 When Digivolving: Unsuspend 2, gain memory")
        effect1.set_effect_description(
            "[When Digivolving] Unsuspend up to 2 Digimon and/or Tamers. Then, "
            "gain 1 memory for each of your opponent's unsuspended Digimon and "
            "Tamers."
        )
        effect1.is_when_digivolving = True

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

            # Unsuspend up to 2 Digimon and/or Tamers (own side)
            suspended = [
                p for p in player.battle_area
                if (p.is_digimon or p.is_tamer) and p.is_suspended
            ]
            for p in suspended[:2]:
                p.unsuspend()

            # Then gain 1 memory per opponent's unsuspended Digimon and Tamers
            enemy = player.enemy
            if not enemy:
                return
            opp_unsuspended = sum(
                1 for p in enemy.battle_area
                if (p.is_digimon or p.is_tamer) and not p.is_suspended
            )
            if opp_unsuspended > 0:
                player.add_memory(opp_unsuspended)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [End of Your Turn] [Once Per Turn] Trash opponent security ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT9-069 EOT: Trash opponent security")
        effect2.set_effect_description(
            "[End of Your Turn] [Once Per Turn] For every 2 unsuspended Digimon "
            "and/or Tamers your opponent has in play, trash 1 card from the top "
            "of your opponent's security stack."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT9_069_EOYT")

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

            # Count opponent's unsuspended Digimon and Tamers
            opp_unsuspended = sum(
                1 for p in enemy.battle_area
                if (p.is_digimon or p.is_tamer) and not p.is_suspended
            )

            # For every 2 unsuspended, trash 1 security
            cards_to_trash = opp_unsuspended // 2
            for _ in range(cards_to_trash):
                if enemy.security_cards:
                    trashed = enemy.security_cards.pop(0)
                    enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
