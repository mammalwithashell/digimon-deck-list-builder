from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_003(CardScript):
    """BT15-003 Nyaromon | Lv.2

    Inherited Effect:
        [When Attacking] [Once Per Turn] By trashing the top or bottom card of
            your security stack, gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited [When Attacking][OPT] By trashing top or bottom security,
        # gain 1 memory. Optional at the activation level (C# canNoSelect=true
        # default). Once the player opts in, the cost must be paid.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnUseAttack)
        effect0.set_effect_name(
            "BT15-003 Trash the top or bottom of your security to gain Memory +1"
        )
        effect0.set_effect_description(
            "[When Attacking][Once Per Turn] By trashing the top or bottom card "
            "of your security stack, gain 1 memory."
        )
        effect0.is_inherited_effect = True
        effect0.is_on_attack = True  # required for engine._matches on OnUseAttack
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("TrashSecuirtyToGainMemory+1_BT15_003")

        def condition0(context: Dict[str, Any]) -> bool:
            # Field presence check (DCGO IsExistOnBattleArea).
            if card and card.permanent_of_this_card() is None:
                return False
            # Cost payability check (DCGO CanActivateCondition:
            # `card.Owner.SecurityCards.Count >= 1`).
            player = card.owner if card else None
            if player is None or not player.security_cards:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Process: player chooses top or bottom of security, trash it
            via the canonical helper so OnDiscardSecurity/OnLoseSecurity fire,
            then gain 1 memory."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game) or not player.security_cards:
                return

            def on_choice(choice: int):
                # Re-check security is non-empty (defensive: another effect
                # could have emptied the stack between the selection prompt
                # and its resolution).
                if not player.security_cards:
                    return
                if choice == 0:
                    target = player.security_cards[0]       # Top of security
                else:
                    target = player.security_cards[-1]      # Bottom of security
                # Use the canonical helper: fires OnDiscardSecurity + OnLoseSecurity,
                # matching DCGO IDestroySecurity -> IReduceSecurity flow.
                player.trash_security_card(target)
                player.add_memory(1)

            game.effect_choose_branch(
                player, 2, on_choice,
                branch_labels=["Trash top security", "Trash bottom security"],
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
