from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_090(CardScript):
    """BT24-090 Abyss Sanctuary: Throne Room"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-090 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
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

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-090 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Neptunemon') or permanent.contains_card_name('Venusmon'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: alliance
        # Alliance
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-090 Alliance")
        effect2.set_effect_description("Alliance")
        effect2._is_alliance = True

        def condition2(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Neptunemon') or permanent.contains_card_name('Venusmon'))):
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Grant Skill
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-090 Your Digimon gain <Alliance>")
        effect3.set_effect_description("Grant Skill")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Neptunemon') or permanent.contains_card_name('Venusmon'))):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Grant Skill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant keyword to other permanents (AddSkillClass) — not yet in engine
            pass  # descriptive-tagged: grant_skill

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OptionSkill
        # [Main] Add your bottom security card to the hand and place this card face up as the bottom security card. Then, you may play 1 blue or yellow [TS] trait Digimon card from your hand with the play cost reduced by 3.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OptionSkill)
        effect4.set_effect_name("BT24-090 Replace your bottom sec with this face-up card, play a [TS] Digimon for -3")
        effect4.set_effect_description("[Main] Add your bottom security card to the hand and place this card face up as the bottom security card. Then, you may play 1 blue or yellow [TS] trait Digimon card from your hand with the play cost reduced by 3.")
        effect4.cost_reduction = 3

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Swap bottom security with this card, then optionally play TS Digimon at -3."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Add bottom security card to hand
            if player.security_cards:
                bottom_sec = player.security_cards.pop(-1)
                player.hand_cards.append(bottom_sec)

            # Step 2: Place this option as bottom security card
            # The option card is the card being resolved
            if card:
                player.security_cards.insert(0, card)

            # Step 3: Optionally play 1 blue/yellow [TS] Digimon at -3 cost
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
                if 'Blue' not in colors and 'Yellow' not in colors:
                    return False
                if not any('TS' in _t for _t in (getattr(c, 'card_traits', []) or [])):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=False,
                manual_reduction=3, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.SecuritySkill
        # Play Card
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.SecuritySkill)
        effect5.set_effect_name("BT24-090 Play Card")
        effect5.set_effect_description("Play Card")
        effect5.is_security_effect = True
        effect5.is_security_effect = True

        def condition5(context: Dict[str, Any]) -> bool:
            return False  # Security effects handled by engine

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not ('Blue' in [col.name for col in getattr(c, 'card_colors', [])] or 'Yellow' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                if not (any('TS' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
