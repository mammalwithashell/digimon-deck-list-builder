from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_091(CardScript):
    """BT19-091 Trinity Burst!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-091 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('WarGrowlmon') or permanent.contains_card_name('Taomon') or permanent.contains_card_name('Rapidmon'))):
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

        # Timing: EffectTiming.OptionSkill
        # [Main] Play 1 [WarGrowlmon] Token (Digimon/Red/6000 DP), [Taomon] Token (Digimon/Yellow/6000 DP), and 1 [Rapidmon] Token (Digimon/Green/6000 DP). This effect can't play tokens with the same names as your Digimon. Then, 1 of your level 5 Digimon gains <Alliance> twice for the turn and attacks.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-091 Play tokens, give alliance and attack")
        effect1.set_effect_description("[Main] Play 1 [WarGrowlmon] Token (Digimon/Red/6000 DP), [Taomon] Token (Digimon/Yellow/6000 DP), and 1 [Rapidmon] Token (Digimon/Green/6000 DP). This effect can't play tokens with the same names as your Digimon. Then, 1 of your level 5 Digimon gains <Alliance> twice for the turn and attacks.")
        effect1._is_alliance = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Alliance, Play Token, Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_alliance')
            # Play WarGrowlmon Token — token play not yet supported in engine
            pass  # descriptive-tagged: play_token
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 level 5 [WarGrowlmon]/[Taomon]/[Rapidmon] from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-091 Play Card")
        effect2.set_effect_description("[Security] You may play 1 level 5 [WarGrowlmon]/[Taomon]/[Rapidmon] from your hand without paying the cost.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('WarGrowlmon' in _n or 'Taomon' in _n or 'Rapidmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
