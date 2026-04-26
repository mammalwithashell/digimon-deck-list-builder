from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_093(CardScript):
    """BT19-093 Queen Device"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-093 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Queen Device'))):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # When this card is trashed in your battle area, until the end of your opponent's turn, 1 of your opponent's Digimon gets -3000 DP and that Digimon's [When Digivolving] effects don't activate.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-093 Give -3000 DP and Cannot activate When Digivolving effects.")
        effect1.set_effect_description("When this card is trashed in your battle area, until the end of your opponent's turn, 1 of your opponent's Digimon gets -3000 DP and that Digimon's [When Digivolving] effects don't activate.")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -3000, Disable Effect, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-3000)
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OptionSkill
        # [Main] Until the end of your opponent's turn, 1 of your opponent's Digimon gets -3000 DP and that Digimon's [When Digivolving] effects don't activate. Then, place this card in the battle area.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-093 Give -3000 DP and Cannot activate When Digivolving effects.")
        effect2.set_effect_description("[Main] Until the end of your opponent's turn, 1 of your opponent's Digimon gets -3000 DP and that Digimon's [When Digivolving] effects don't activate. Then, place this card in the battle area.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -3000, Disable Effect, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-3000)
            # Disable/invalidate effects on target — not yet in engine
            pass  # descriptive-tagged: disable_effect
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] 2 of your opponent's Digimon gain <Security A. -2> for the turn. Then, add this card to the hand.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-093 Security Attack -2 for 2 of your opponents Digimon, then add this to hand")
        effect3.set_effect_description("[Security] 2 of your opponent's Digimon gain <Security A. -2> for the turn. Then, add this card to the hand.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Add To Hand, Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
