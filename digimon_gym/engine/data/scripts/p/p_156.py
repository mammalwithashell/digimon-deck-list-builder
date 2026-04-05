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
        # Dynamic check: only bypass color requirement when player has a Tamer.
        def _check_color_req():
            owner = card.owner if card else None
            if not owner:
                return True  # enforce
            has_tamer = any(getattr(p, 'is_tamer', False) for p in owner.battle_area)
            if has_tamer:
                return False  # bypass color requirement
            return True  # enforce color requirement
        card._match_color_requirement_fn = _check_color_req

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

            def tamer_filter(p):
                return p.is_tamer

            def on_tamer_selected(selected_tamer):
                # Get colors of the chosen Tamer
                tamer_colors = set()
                if selected_tamer.top_card:
                    colors = getattr(selected_tamer.top_card, 'card_colors', []) or []
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
                    # Must share a color with the chosen Tamer
                    card_colors = getattr(c, 'card_colors', []) or []
                    return any(cc in tamer_colors for cc in card_colors)

                game.effect_play_from_zone(
                    player, 'hand_or_trash', play_filter, free=True, is_optional=True,
                    prompt="Play 1 Digimon (cost 3 or less) matching the chosen Tamer's color.")

            game.effect_select_own_permanent(
                player, on_tamer_selected,
                filter_fn=tamer_filter,
                is_optional=False,
                prompt="Choose 1 of your Tamers."
            )

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
