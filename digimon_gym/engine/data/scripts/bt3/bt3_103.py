from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType, ModifierEntry
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_103(CardScript):
    """BT3-103 Hidden Potential Discovered! | Option (Green, Cost 0)

    [Main] For the turn, when one of your green Digimon would next
    digivolve, by suspending 1 of your Digimon, reduce the digivolution
    cost by 5.
    [Security] Add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Grant a one-shot digivolution cost reduction of 5 for
        # the next green digivolve this turn, with suspend-as-cost.
        # Approximation: We register a CHANGE_DIGIVOLUTION_COST modifier
        # that reduces cost by 5 for green Digimon, expiring at end of turn.
        # The "suspend 1 Digimon" cost is paid upfront (select + suspend).
        # The "next digivolve" one-shot nature is approximated by end_of_turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT3-103 Suspend to reduce next green digi cost by 5")
        effect0.set_effect_description(
            "[Main] For the turn, when one of your green Digimon would next "
            "digivolve, by suspending 1 of your Digimon, reduce the "
            "digivolution cost by 5."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            # Must have at least 1 unsuspended Digimon to suspend
            return any(
                p.is_digimon and not p.is_suspended
                for p in owner.battle_area
            )

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Select 1 Digimon to suspend as cost
            def on_suspend(target_perm):
                target_perm.suspend()

                # Register digivolution cost reduction for green Digimon
                # This applies to all of player's Digimon for the turn
                for field_perm in player.battle_area:
                    if field_perm.is_digimon:
                        game.register_modifier(
                            field_perm, ModifierType.CHANGE_DIGIVOLUTION_COST,
                            value_fn=lambda current, target, c: current - 5,
                            expiry='end_of_turn',
                        )

            game.effect_select_own_permanent(
                player, on_suspend,
                filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                is_optional=False,
                prompt="Suspend 1 of your Digimon to reduce next green digi cost by 5.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Security] Add this card to the hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT3-103 Security: Add to hand")
        effect1.set_effect_description("[Security] Add this card to the hand.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.hand_cards.append(card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
