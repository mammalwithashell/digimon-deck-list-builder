from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_159(CardScript):
    """P-159 Rook Device"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("P-159 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Rook Device'))):
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
        # 1 of your Digimon gains <Reboot>, <Blocker> and gets +2000 DP until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-159 Gain Reboot, Blocker, +2000 DP")
        effect1.set_effect_description("1 of your Digimon gains <Reboot>, <Blocker> and gets +2000 DP until the end of your opponent's turn.")
        effect1.is_on_deletion = True
        effect1._is_reboot = True
        effect1._is_blocker = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +2000, Gain Keyword Reboot, Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(2000)
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_reboot')
                target_perm.grant_keyword('_is_blocker')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your Digimon gains <Reboot>, <Blocker> and gets +2000 DP until the end of your opponent's turn. Then, place this card in the battle area.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-159 Gain Reboot, Blocker, +2000 DP, then place in battle area")
        effect2.set_effect_description("[Main] 1 of your Digimon gains <Reboot>, <Blocker> and gets +2000 DP until the end of your opponent's turn. Then, place this card in the battle area.")
        effect2._is_reboot = True
        effect2._is_blocker = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP +2000, Gain Keyword Reboot, Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(2000)
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_reboot')
                target_perm.grant_keyword('_is_blocker')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] <De-Digivolve 2> 1 of your opponent's Digimon. Then, add this card to the hand.
        effect3 = ICardEffect()
        effect3.set_effect_name("P-159 De-Digivolve 2, 1 of your opponents Digimon, then add this to hand")
        effect3.set_effect_description("[Security] <De-Digivolve 2> 1 of your opponent's Digimon. Then, add this card to the hand.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Add To Hand, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
