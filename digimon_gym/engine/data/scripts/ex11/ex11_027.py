from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_027(CardScript):
    """EX11-027 Maquinamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-027 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.2 for cost 0
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Maquinamon' in text):
                    return False
            else:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Add To Hand, Reveal And Select
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-027 Reveal top 3, Add [Maquinamon] & 1 [Maquinamon] in text, then may link")
        effect1.set_effect_description("Add To Hand, Reveal And Select")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Maquinamon' in text):
                    return False
            else:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add 1 card with [Maquinamon] in name or 1 card with [Maquinamon] in text."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def reveal_filter(c):
                # Card must have [Maquinamon] in its name OR [Maquinamon] in its text
                names = getattr(c, 'card_names', []) or []
                if any('Maquinamon' in n for n in names):
                    return True
                text = getattr(c, 'card_text', '') or ''
                if 'Maquinamon' in text:
                    return True
                return False
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area, by placing 1 of its link cards as its bottom digivolution card, it doesn't leave.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("EX11-027 Place a linked card into digivolution cards to prevent this digimon from leaving")
        effect2.set_effect_description("[All Turns] When this Digimon would leave the battle area, by placing 1 of its link cards as its bottom digivolution card, it doesn't leave.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only trigger when THIS permanent would leave
            leaving_perm = context.get('permanent')
            my_perm = card.permanent_of_this_card() if card else None
            if leaving_perm is not my_perm:
                return False
            # Must have linked cards to sacrifice
            if not my_perm or not my_perm.linked_cards:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Place 1 linked card as bottom digivolution card to prevent leaving."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm):
                return
            my_perm = card.permanent_of_this_card() if card else perm
            if my_perm and my_perm.linked_cards:
                linked = my_perm.linked_cards.pop(0)
                my_perm.add_card_source_bottom(linked)
                # Re-add to battle area if already removed
                if my_perm not in player.battle_area:
                    # Restore from trash
                    for cs in list(my_perm.card_sources):
                        if cs in player.trash_cards:
                            player.trash_cards.remove(cs)
                    player.battle_area.append(my_perm)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
