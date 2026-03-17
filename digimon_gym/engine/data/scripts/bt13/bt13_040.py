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

            # Check both zones for Veemon candidates
            hand_candidates = [c for c in player.hand_cards if veemon_filter(c)]

            my_perm = card.permanent_of_this_card() if card else perm
            digi_source_candidates = []
            if my_perm:
                for cs in list(my_perm.card_sources):
                    if cs is my_perm.top_card:
                        continue
                    if veemon_filter(cs):
                        digi_source_candidates.append(cs)

            can_hand = len(hand_candidates) > 0
            can_digi = len(digi_source_candidates) > 0

            if not (can_hand or can_digi):
                return

            def _play_from_hand():
                game.effect_play_from_zone(
                    player, 'hand', veemon_filter, free=True, is_optional=True,
                    prompt="Select 1 [Veemon] from your hand to play without paying the cost.")

            def _play_from_digi():
                from ....data.enums import GamePhase
                from ....game.constants import SEL_HAND_START
                # Build selection indices for digi-source Veemon cards
                valid = []
                for i, cs in enumerate(my_perm.card_sources):
                    if cs is my_perm.top_card:
                        continue
                    if veemon_filter(cs):
                        valid.append(SEL_HAND_START + i)
                if not valid:
                    return

                def on_digi_select(action_id: int):
                    idx = action_id - SEL_HAND_START
                    if not (0 <= idx < len(my_perm.card_sources)):
                        return
                    chosen = my_perm.card_sources[idx]
                    my_perm.card_sources.remove(chosen)
                    played = player.play_card_from_source(chosen, pay_cost=False)
                    if played and game:
                        game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
                            'played_card': chosen,
                            'played_permanent': played,
                            'event_permanent': played,
                            'event_player': player,
                        })

                game.request_selection(
                    GamePhase.SelectSource, player, on_digi_select, valid,
                    is_optional=True,
                    prompt="Select 1 [Veemon] from this Digimon's digivolution cards to play.")

            if can_hand and can_digi:
                # C# ref: branch choice between hand and digi-source
                def on_branch(branch_index: int):
                    if branch_index == 0:
                        _play_from_hand()
                    elif branch_index == 1:
                        _play_from_digi()

                game.effect_choose_branch(
                    player, 2, on_branch,
                    prompt="From which area do you play a [Veemon]?",
                    branch_labels=["From hand", "From digivolution cards"],
                    is_optional=True,
                )
            elif can_hand:
                _play_from_hand()
            elif can_digi:
                _play_from_digi()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
