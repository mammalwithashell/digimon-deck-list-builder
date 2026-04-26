from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_084(CardScript):
    """BT10-084 Tactimon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] Play 1 Lv.4 or lower Digimon card with [Bagra Army] in its
        # traits from your trash without paying its memory cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT10-084 Play Bagra Army from trash")
        effect0.set_effect_description(
            "[On Play] Play 1 Lv.4 or lower Digimon card with [Bagra Army] in its "
            "traits from your trash without paying its memory cost."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level is None or level > 4:
                    return False
                traits = getattr(c, 'card_traits', [])
                return 'Bagra Army' in traits or 'BagraArmy' in traits

            if not any(play_filter(c) for c in player.trash_cards):
                return

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Opponent's Turn] When an effect would trash one of your other Digimon's
        # digivolution cards, you may trash this Digimon's digivolution cards instead.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenWouldDigivolutionCardDiscarded)
        effect1.set_effect_name("BT10-084 Redirect digivolution card trash")
        effect1.set_effect_description(
            "[Opponent's Turn] When an effect would trash one of your other Digimon's "
            "digivolution cards, you may trash this Digimon's digivolution cards instead."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Must be on the field as a Digimon
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            # Must be opponent's turn
            if card.owner and card.owner.is_my_turn:
                return False
            # The targeted permanent must be a DIFFERENT one of our Digimon
            event_perm = context.get('event_permanent') or context.get('permanent')
            if event_perm is None or event_perm is perm:
                return False
            # Check the targeted permanent is one of our Digimon
            if card.owner:
                if event_perm not in card.owner.battle_area:
                    return False
            # This permanent must have digivolution cards to offer
            if len(perm.card_sources) <= 1:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Redirect: cancel original trash, select own digi cards to trash instead."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return

            # Get the original cards that would be trashed
            trashed_cards = ctx.get('trashed_cards', [])
            if not trashed_cards:
                return

            redirect_count = len(trashed_cards)

            # Check we have enough digivolution cards
            digi_cards = perm.card_sources[:-1]  # exclude top card
            if len(digi_cards) < redirect_count:
                return

            # Cancel the original trash (DCGO: willBeRemoveSources = false)
            for cs in trashed_cards:
                cs._will_be_removed = False

            # If exactly enough cards, auto-select all
            if len(digi_cards) == redirect_count:
                for cs in list(digi_cards):
                    if cs in perm.card_sources:
                        perm.card_sources.remove(cs)
                        player.trash_cards.append(cs)
                return

            # Otherwise, let player select which digi cards to trash
            from ....game.constants import SOURCES_PER_FIELD
            from ....data.enums import GamePhase

            field_idx = None
            for fi, fp in enumerate(player.battle_area):
                if fp is perm:
                    field_idx = fi
                    break
            if field_idx is None:
                return

            remaining = [redirect_count]  # mutable counter

            def _select_next():
                if remaining[0] <= 0:
                    return
                base = 2000 + field_idx * SOURCES_PER_FIELD
                valid = []
                for i, cs in enumerate(perm.card_sources[:-1]):
                    action_id = base + i
                    if action_id < 2168:
                        valid.append(action_id)
                if not valid:
                    return

                def on_selected(action_id):
                    idx = action_id - base
                    if not (0 <= idx < len(perm.card_sources)):
                        return
                    chosen = perm.card_sources[idx]
                    if chosen is perm.top_card:
                        return
                    perm.card_sources.remove(chosen)
                    player.trash_cards.append(chosen)
                    remaining[0] -= 1
                    _select_next()

                game.request_selection(
                    GamePhase.SelectSource, player, on_selected,
                    valid, is_optional=False,
                    prompt=f"Select digivolution card to trash instead ({redirect_count - remaining[0] + 1}/{redirect_count}).")

            _select_next()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
