from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_156(CardScript):
    """P-156 Future Potential! (Option)

    You may ignore color requirements to use this card.

    [Main] Choose 1 of your Tamers. You may play 1 Digimon card with the same
        color as that Tamer and with a play cost of 3 or less from your hand or
        trash without paying the cost.

    [Security] You may play 1 Tamer card from your hand without paying the cost.
        Then, add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Ignore Color Requirements ---
        # "While you have a Tamer, you can ignore this card's color requirements."
        # Set unconditionally; the Main effect itself guards on having a Tamer.
        card._match_color_requirement = False

        # --- Effect 1: [Main] Choose Tamer, play matching-color Digimon cost<=3 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("P-156 Play 1 Digimon, 3 cost or less")
        effect1.set_effect_description(
            "[Main] Choose 1 of your Tamers. You may play 1 Digimon card with "
            "the same color as that Tamer and with a play cost of 3 or less "
            "from your hand or trash without paying the cost."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Find all tamers on our field
            tamer_perms = [p for p in player.battle_area if p.is_tamer]
            if not tamer_perms:
                return
            # Collect colors from all our Tamers for filtering
            # In the engine, we pick a Tamer via selection, then filter by its colors.
            # Since the engine doesn't have multi-step selection easily, we collect
            # all Tamer colors and allow playing a Digimon matching ANY of them.
            # This is a simplification — ideally we'd select a specific Tamer first.
            tamer_colors = set()
            for tp in tamer_perms:
                if tp.top_card:
                    colors = getattr(tp.top_card, 'card_colors', []) or []
                    for c in colors:
                        tamer_colors.add(c)

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                cost = getattr(c, 'get_cost_itself', None)
                if cost is None:
                    return False
                if cost > 3:
                    return False
                # Must share a color with one of our Tamers
                card_colors = getattr(c, 'card_colors', []) or []
                return any(cc in tamer_colors for cc in card_colors)

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True,
                prompt="Play 1 Digimon (cost 3 or less) matching a Tamer's color.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play 1 Tamer from hand, then add this card to hand ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("P-156 Play 1 tamer, then add this card to hand")
        effect2.set_effect_description(
            "[Security] You may play 1 tamer card from your hand without paying "
            "the cost. Then, add this card to the hand."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Play 1 Tamer from hand
            def tamer_filter(c):
                return getattr(c, 'is_tamer', False)
            game.effect_play_from_zone(
                player, 'hand', tamer_filter, free=True, is_optional=True,
                prompt="You may play 1 Tamer from hand.")
            # Then add this card (the option) to hand from trash
            if card in player.trash_cards:
                player.trash_cards.remove(card)
                player.hand_cards.append(card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
