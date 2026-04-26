from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_070(CardScript):
    """EX8-070 Zofr Kabus"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] By trashing any 1 digivolution card of 1 of your [Mineral] or [Rock] trait Digimon, until the end of your opponent's turn, it gains <Collision>, <Piercing> and <Reboot>, gets +3000 DP, and can't be returned to the hand or deck by your opponent's effects.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-070 By trashing 1 source, gain collision, piercing, reboot, +3000 DP, and can't be returned to hand or deck by opponent")
        effect0.set_effect_description("[Main] By trashing any 1 digivolution card of 1 of your [Mineral] or [Rock] trait Digimon, until the end of your opponent's turn, it gains <Collision>, <Piercing> and <Reboot>, gets +3000 DP, and can't be returned to the hand or deck by your opponent's effects.")
        effect0._is_piercing = True
        effect0._is_reboot = True
        effect0._is_cannot_return_to_hand = True
        effect0._is_cannot_return_to_deck = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Gain Keyword Piercing, Gain Keyword Reboot, Gain Keyword Cannot Return To Hand, Gain Keyword Cannot Return To Deck, Grant Skill, Grant Bounce Immunity, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if perm:
                perm.grant_keyword('_is_piercing')
                perm.grant_keyword('_is_reboot')
                perm.grant_keyword('_is_cannot_return_to_hand')
                perm.grant_keyword('_is_cannot_return_to_deck')
            # Grant keyword to other permanents (AddSkillClass) — not yet in engine
            pass  # descriptive-tagged: grant_skill
            # Prevent return to hand/deck via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_RETURNED, perm,
                    value_fn=lambda: True, expiry='end_of_turn')
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Delete 1 of your opponent's Digimon with the lowest play cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX8-070 Delete")
        effect1.set_effect_description("[Security] Delete 1 of your opponent's Digimon with the lowest play cost.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
