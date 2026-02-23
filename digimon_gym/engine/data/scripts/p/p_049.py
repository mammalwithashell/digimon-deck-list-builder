from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_049(CardScript):
    """P-049 Phoenixmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have a Tamer in play, this Digimon gains <Security Attack +1> for the turn. (This Digimon checks 1 additional security card.)
        effect0 = ICardEffect()
        effect0.set_effect_name("P-049 This Digimon gains Security Attack +1")
        effect0.set_effect_description("[When Digivolving] If you have a Tamer in play, this Digimon gains <Security Attack +1> for the turn. (This Digimon checks 1 additional security card.)")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnBlockAnyone
        # [Your Turn][Once Per Turn] When this Digimon is blocked, trash the top card of your opponent's security stack.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-049 Trash the top card of opponent's security")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When this Digimon is blocked, trash the top card of your opponent's security stack.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("TrashSecurity_P_049")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
