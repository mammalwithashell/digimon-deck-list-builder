from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_082(CardScript):
    """BT18-082 Lucemon: Chaos Mode | Lv.5 Purple/Yellow Digimon

    [On Play] [When Digivolving] Your opponent may delete 1 of their Digimon
        or Tamers. If this effect didn't delete, Recovery +1 (Deck) and trash
        the top card of your opponent's security stack.
    [All Turns] [Once Per Turn] When this Digimon would leave the battle area,
        by trashing the bottom card of your security stack, it doesn't leave.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Opponent deletes or recovery+trash ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT18-082 On Play: opponent deletes or recovery+trash")
        effect0.set_effect_description(
            "[On Play] Your opponent may delete 1 of their Digimon or Tamers. "
            "If this effect didn't delete, Recovery +1 (Deck) and trash the "
            "top card of your opponent's security stack."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Opponent may delete their own, or we get recovery+trash."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Check if opponent has any Digimon or Tamers to delete
            deletable = [
                p for p in enemy.battle_area
                if p.is_digimon or p.is_tamer
            ]
            if not deletable:
                # No targets — auto recovery + trash
                player.recovery(1)
                if enemy.security_cards:
                    trashed = enemy.security_cards.pop(0)
                    enemy.trash_cards.append(trashed)
                return

            # Simplified: opponent auto-declines deletion (AI can't make
            # opponent decisions), so we get the recovery + trash benefit.
            # In a full implementation, this would prompt the opponent.
            player.recovery(1)
            if enemy.security_cards:
                trashed = enemy.security_cards.pop(0)
                enemy.trash_cards.append(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] Same effect ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT18-082 When Digivolving: opponent deletes or recovery+trash")
        effect1.set_effect_description(
            "[When Digivolving] Your opponent may delete 1 of their Digimon "
            "or Tamers. If this effect didn't delete, Recovery +1 (Deck) and "
            "trash the top card of your opponent's security stack."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Same as on play — opponent deletes or recovery+trash."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            deletable = [
                p for p in enemy.battle_area
                if p.is_digimon or p.is_tamer
            ]
            if not deletable:
                player.recovery(1)
                if enemy.security_cards:
                    trashed = enemy.security_cards.pop(0)
                    enemy.trash_cards.append(trashed)
                return

            # Simplified: auto-grant recovery + trash (opponent declines)
            player.recovery(1)
            if enemy.security_cards:
                trashed = enemy.security_cards.pop(0)
                enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [All Turns] [Once Per Turn] Leave protection ---
        # When this Digimon would leave the battle area, trash bottom security
        # to prevent leaving. This is a "would be removed" interrupt.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect2.set_effect_name("BT18-082 Leave protection")
        effect2.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon would leave the "
            "battle area, by trashing the bottom card of your security stack, "
            "it doesn't leave."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("AllTurns_BT18-082_LeaveProtection")
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Check that the permanent being deleted is this one
            perm = card.permanent_of_this_card() if card else None
            ctx_perm = context.get('permanent')
            if perm and ctx_perm and perm != ctx_perm:
                return False
            # Need security to trash
            player = card.owner if card else None
            if player and not player.security_cards:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Trash bottom security card to prevent leaving."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if player.security_cards:
                # Trash bottom card of security
                trashed = player.security_cards.pop(-1)
                player.trash_cards.append(trashed)
                # Mark permanent as surviving deletion
                perm = card.permanent_of_this_card() if card else None
                if perm:
                    perm._survived_deletion = True
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
