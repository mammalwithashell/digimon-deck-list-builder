from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT9_082(CardScript):
    """BT9-082 Ordinemon | Lv.7 Purple/Yellow Digimon

    [When Digivolving] When DNA digivolving, delete 1 of your opponent's
        level 6 or higher Digimon and all of their level 5 or lower Digimon.
        For each Digimon deleted by this effect, Recovery +1 (Deck).
    [On Deletion] You may trash the top card of your security stack to play
        this card without paying its memory cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [When Digivolving] DNA digivolve mass deletion + recovery ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT9-082 When Digivolving: mass delete + recovery")
        effect0.set_effect_description(
            "[When Digivolving] When DNA digivolving, delete 1 of your "
            "opponent's level 6 or higher Digimon and all of their level 5 "
            "or lower Digimon. For each Digimon deleted, Recovery +1 (Deck)."
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only activates when DNA digivolving
            if not context.get('is_dna_digivolve'):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Delete 1 opponent Lv6+ and all Lv5-, then recovery for each."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            deleted_count = [0]  # Use list for closure mutation

            def _delete_low_and_recover():
                """Delete all level 5 or lower Digimon, then recovery."""
                to_delete_low = [
                    p for p in enemy.battle_area
                    if p.is_digimon and p.level is not None and p.level <= 5
                ]
                for perm in to_delete_low:
                    if perm in enemy.battle_area:
                        enemy.delete_permanent(perm)
                        deleted_count[0] += 1
                # Recovery +1 for each deleted
                if deleted_count[0] > 0:
                    player.recovery(deleted_count[0])

            # Delete 1 level 6 or higher Digimon (player selects)
            high_targets = [
                p for p in enemy.battle_area
                if p.is_digimon and p.level is not None and p.level >= 6
            ]
            if high_targets:
                def on_delete_high(target_perm):
                    if target_perm and target_perm in enemy.battle_area:
                        enemy.delete_permanent(target_perm)
                        deleted_count[0] += 1
                    _delete_low_and_recover()

                game.effect_select_opponent_permanent(
                    player, on_delete_high,
                    filter_fn=lambda p: p.is_digimon and p.level is not None and p.level >= 6,
                    is_optional=False)
            else:
                _delete_low_and_recover()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [On Deletion] Trash top security to play this card free ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT9-082 On Deletion: revive from trash")
        effect1.set_effect_description(
            "[On Deletion] You may trash the top card of your security stack "
            "to play this card without paying its memory cost."
        )
        effect1.is_on_deletion = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Check if this card is the one being deleted
            destroyed = context.get('permanent')
            if card and destroyed:
                if card.permanent_of_this_card() is not None:
                    return False  # Still on field, not deleted
            player = card.owner if card else None
            if player and not player.security_cards:
                return False  # No security to trash
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Trash top security card, then play this card from trash free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.security_cards:
                return
            # Trash top card of security
            trashed = player.security_cards.pop(0)
            player.trash_cards.append(trashed)
            # Play this card from trash without paying cost
            if card and card in player.trash_cards:
                player.play_card_from_source(card, pay_cost=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
