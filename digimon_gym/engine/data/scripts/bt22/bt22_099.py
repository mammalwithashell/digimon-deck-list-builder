from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_099(CardScript):
    """BT22-099 Kuremi Detective Agency"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Dynamic color bypass: ignore color requirements while a CS trait Digimon/Tamer is on field
        def _has_cs_on_field():
            owner = card.owner if card else None
            if not owner:
                return False
            for p in owner.battle_area:
                if (p.is_tamer or p.is_digimon) and p.top_card:
                    traits = getattr(p.top_card, 'card_traits', []) or []
                    if any('CS' in t for t in traits):
                        return True
            return False

        def _check_cs_color_req():
            return not _has_cs_on_field()  # False = bypass, True = enforce
        card._match_color_requirement_fn = _check_cs_color_req

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 3 cards of your deck. Add 1 [CS] trait card
        # among them to the hand. Return the rest to the bottom of the deck.
        # Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT22-099 Reveal top 3, add 1 [CS] card to hand, bottom deck the rest")
        effect1.set_effect_description("[Main] Reveal the top 3 cards of your deck. Add 1 [CS] trait card among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add 1 CS card to hand"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('CS' in t for t in traits)

            game.effect_reveal_and_select_multi(
                player, 3,
                passes=[
                    (reveal_filter, 'hand'),
                ],
                remaining_placement='deck_bottom',
                is_optional=True
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: delay
        # Delay
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-099 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay> Gain 2 memory.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDeclaration)
        effect3._is_field_main = True
        effect3.set_effect_name("BT22-099 Delay: Gain 2 memory")
        effect3.set_effect_description("[Main] <Delay> Gain 2 memory.")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash this card from battle area (Delay cost), then gain 2 memory"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not player:
                return
            # Delay cost: trash this Option card from the battle area
            perm = card.permanent_of_this_card() if card else None
            if perm and perm in player.battle_area:
                player.battle_area.remove(perm)
                for cs in perm.card_sources:
                    player.trash_cards.append(cs)
            # Effect: gain 2 memory
            player.add_memory(2)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Security Effect: Place this card in the battle area
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("BT22-099 Security: Place in battle area")
        effect4.set_effect_description("[Security] Place this card in the battle area.")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
