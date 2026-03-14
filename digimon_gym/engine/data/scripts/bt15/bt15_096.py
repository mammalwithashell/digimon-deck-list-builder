from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_096(CardScript):
    """BT15-096 Supreme Connection!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 5 cards of your deck. Among them, add 1 card with the [Machine] or [Cyborg] trait to the hand and trash 1 such card. Return the rest to the top of the deck. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT15-096 Trash From Hand, Add To Hand, Reveal And Select")
        effect0.set_effect_description("[Main] Reveal the top 5 cards of your deck. Among them, add 1 card with the [Machine] or [Cyborg] trait to the hand and trash 1 such card. Return the rest to the top of the deck. Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Machine' in _t or 'Cyborg' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('Machine' in _t or 'Cyborg' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 5, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-096 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main][Delay] You may play 1 level 5 or higher Digimon card with the [Machine] or [Cyborg] trait from your hand with the play cost reduced by 3.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("BT15-096 Play 1 Digimon from hand")
        effect2.set_effect_description("[Main][Delay] You may play 1 level 5 or higher Digimon card with the [Machine] or [Cyborg] trait from your hand with the play cost reduced by 3.")
        effect2.cost_reduction = 3

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Cost -3, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level < 5:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Cost reduction by 3 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Reveal the top 5 cards of your deck. Among them, add 1 card with the [Machine] or [Cyborg] trait to the hand and trash 1 such card. Return the rest to the top of the deck. Then, place this card in the battle area.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT15-096 Trash From Hand, Add To Hand, Reveal And Select")
        effect3.set_effect_description("[Security] Reveal the top 5 cards of your deck. Among them, add 1 card with the [Machine] or [Cyborg] trait to the hand and trash 1 such card. Return the rest to the top of the deck. Then, place this card in the battle area.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Machine' in _t or 'Cyborg' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('Machine' in _t or 'Cyborg' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 5, reveal_filter, on_revealed, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
