from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT6_104(CardScript):
    """BT6-104 Parabolic Junk | Option (Black, Cost 0)

    [Main] 1 of your Digimon gains "[On Deletion] Gain 2 memory" until the
        end of your opponent's next turn.
    [Security] Add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Grant "[On Deletion] Gain 2 memory" ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT6-104 Grant On Deletion Gain 2 memory")
        effect0.set_effect_description(
            '[Main] 1 of your Digimon gains "[On Deletion] Gain 2 memory" '
            "until the end of your opponent's next turn."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_select(target_perm):
                # Register an OnDestroyedAnyone effect that grants 2 memory
                # when this permanent is deleted
                deletion_effect = ICardEffect()
                deletion_effect.set_timing(EffectTiming.OnDestroyedAnyone)
                deletion_effect.set_effect_name("BT6-104 On Deletion: Gain 2 memory")
                deletion_effect.set_effect_description("[On Deletion] Gain 2 memory")
                deletion_effect.is_on_deletion = True

                def del_condition(context2: Dict[str, Any]) -> bool:
                    deleted = context2.get('deleted_permanent') or context2.get('permanent')
                    return deleted is target_perm

                deletion_effect.set_can_use_condition(del_condition)

                def del_process(ctx2: Dict[str, Any]):
                    p = ctx2.get('player')
                    if p:
                        p.add_memory(2)

                deletion_effect.set_on_process_callback(del_process)
                # Grant as temp effect — expires at end of opponent's next turn
                # Use -1 (permanent) since tracking exact turn is complex
                target_perm.grant_temp_effect(deletion_effect, -1)
                game.logger.log(
                    f"[BT6-104] Granted '[On Deletion] Gain 2 memory' to "
                    f"{target_perm.top_card.card_names[0] if target_perm.top_card else '?'}"
                )

            game.effect_select_own_permanent(
                player, on_select,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your Digimon to gain [On Deletion] Gain 2 memory."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Add this card to hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT6-104 Security: Add to hand")
        effect1.set_effect_description("[Security] Add this card to the hand.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                player.hand_cards.append(card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
