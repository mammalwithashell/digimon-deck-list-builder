from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_040(CardScript):
    """BT13-040 Magnamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-040 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Veemon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-040 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area, Draw 1. Then, you may play 1 [Veemon]
        # from your hand or from this Digimon's digivolution cards without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("BT13-040 Draw 1 and play 1 [Veemon] from hand or digi sources")
        effect2.set_effect_description("[All Turns] When this Digimon would leave the battle area, <Draw 1>. Then, you may play 1 [Veemon] from your hand or this Digimon's digivolution cards without paying the cost.")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Trigger only when THIS Magnamon leaves
            leaving_perm = context.get('permanent')
            my_perm = card.permanent_of_this_card() if card else None
            if leaving_perm is not my_perm:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Draw 1, then play 1 Veemon from hand or this Digimon's digi sources"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if not (player and game):
                return

            def veemon_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('Veemon' in n for n in names)

            # Try to play from hand first
            hand_candidates = [c for c in player.hand_cards if veemon_filter(c)]

            # Try to play from digi sources (snapshot the perm before it's gone)
            my_perm = card.permanent_of_this_card() if card else perm
            digi_source_candidates = []
            if my_perm:
                for cs in list(my_perm.card_sources):
                    if cs is my_perm.top_card:
                        continue
                    if veemon_filter(cs):
                        digi_source_candidates.append(cs)

            if hand_candidates or digi_source_candidates:
                # Prefer hand if available, then digi sources
                if hand_candidates:
                    game.effect_play_from_zone(
                        player, 'hand', veemon_filter, free=True, is_optional=True,
                        prompt="You may play 1 [Veemon] from your hand without paying the cost.")
                elif digi_source_candidates:
                    # Play from digi source — use Pattern 11
                    for cs in digi_source_candidates:
                        if my_perm and cs in my_perm.card_sources:
                            my_perm.card_sources.remove(cs)
                        played = player.play_card_from_source(cs, pay_cost=False)
                        if played and game:
                            game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
                                'played_card': cs,
                                'played_permanent': played,
                                'event_permanent': played,
                                'event_player': player,
                            })
                        break

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
