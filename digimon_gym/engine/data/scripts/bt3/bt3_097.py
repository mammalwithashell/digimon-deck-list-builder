from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_097(CardScript):
    """BT3-097 A Delicate Plan | Option Red (Cost 1)

    [Main] 1 of your Digimon gains "This Digimon doesn't activate the
    [Security] effects of any Option cards it checks" for the turn.
    [Security] Add this card to its owner's hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Grant security-option-immunity to 1 Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT3-097 Grant security option immunity")
        effect0.set_effect_description(
            "[Main] 1 of your Digimon gains \"This Digimon doesn't "
            "activate the [Security] effects of any Option cards it "
            "checks\" for the turn."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def digimon_filter(p):
                return p.is_digimon and p in player.battle_area

            def on_selected(target_perm):
                if target_perm is None:
                    return
                # Grant "ignore security effects of Option cards" for the turn.
                # The engine checks _ignore_security_option_effects on the
                # attacking permanent to skip Option security effects.
                target_perm._ignore_security_option_effects = True

            game.effect_select_own_permanent(
                player, on_selected, filter_fn=digimon_filter,
                is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Add this card to its owner's hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT3-097 Security: Add to hand")
        effect1.set_effect_description(
            "[Security] Add this card to its owner's hand."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                # Move from trash/security discard to hand
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                player.hand_cards.append(card)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
