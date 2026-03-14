from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource

# Source-selection action-ID offset (mirrors game.py SEL_SOURCE_START logic)
# We reuse SelectSource phase with indices 0..MAX_SOURCES
_SEL_SOURCE_START = 0


class ST9_06(CardScript):
    """ST9-06 Imperialdramon Dragon Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Digivolving] You may play 1 level 4 or lower blue Digimon card and
        # 1 level 4 or lower green Digimon card from this Digimon's digivolution cards
        # without paying their memory costs.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST9-06 Play Blue Lv4 and Green Lv4 from digi-stack")
        effect0.set_effect_description(
            "[When Digivolving] You may play 1 level 4 or lower blue Digimon card and "
            "1 level 4 or lower green Digimon card from this Digimon's digivolution cards "
            "without paying their memory costs."
        )
        effect0.is_when_digivolving = True
        effect0.is_optional = True

        def _is_blue_lv4(cs, top):
            if cs is top:
                return False
            if not getattr(cs, 'is_digimon', False):
                return False
            level = getattr(cs, 'level', None)
            if level is None or level > 4:
                return False
            colors = [
                getattr(col, 'name', str(col))
                for col in (getattr(cs, 'card_colors', None) or [])
            ]
            return 'Blue' in colors

        def _is_green_lv4(cs, top):
            if cs is top:
                return False
            if not getattr(cs, 'is_digimon', False):
                return False
            level = getattr(cs, 'level', None)
            if level is None or level > 4:
                return False
            colors = [
                getattr(col, 'name', str(col))
                for col in (getattr(cs, 'card_colors', None) or [])
            ]
            return 'Green' in colors

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            top = perm.top_card
            has_blue = any(_is_blue_lv4(cs, top) for cs in perm.card_sources)
            has_green = any(_is_green_lv4(cs, top) for cs in perm.card_sources)
            return has_blue or has_green

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            top = perm.top_card
            selected_cards = []

            def _play_selected():
                """Play all selected cards from digi-stack without paying cost."""
                for cs in selected_cards:
                    cur_perm = card.permanent_of_this_card() if card else None
                    if cur_perm is None:
                        break
                    if cs not in cur_perm.card_sources:
                        continue
                    cur_perm.card_sources.remove(cs)
                    played = player.play_card_from_source(cs, pay_cost=False)
                    if played:
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {
                                'played_card': cs,
                                'played_permanent': played,
                                'event_player': player,
                            },
                        )

            def _select_green():
                """Step 2: Select 1 green Lv.4-or-lower Digimon from digi-stack."""
                cur_perm = card.permanent_of_this_card() if card else None
                if cur_perm is None:
                    _play_selected()
                    return
                cur_top = cur_perm.top_card
                valid_indices = []
                for i, cs in enumerate(cur_perm.card_sources):
                    if _is_green_lv4(cs, cur_top) and cs not in selected_cards:
                        valid_indices.append(_SEL_SOURCE_START + i)

                if not valid_indices:
                    _play_selected()
                    return

                def on_green_selected(action_id: int):
                    idx = action_id - _SEL_SOURCE_START
                    inner_perm = card.permanent_of_this_card() if card else None
                    if inner_perm and 0 <= idx < len(inner_perm.card_sources):
                        selected_cards.append(inner_perm.card_sources[idx])
                    _play_selected()

                game.request_selection(
                    GamePhase.SelectSource,
                    player,
                    on_green_selected,
                    valid_indices,
                    is_optional=False,
                    prompt="Select 1 level 4 or lower green Digimon card to play.",
                )

            # Step 1: Select 1 blue Lv.4-or-lower Digimon from digi-stack
            blue_valid = []
            for i, cs in enumerate(perm.card_sources):
                if _is_blue_lv4(cs, top):
                    blue_valid.append(_SEL_SOURCE_START + i)

            if blue_valid:
                def on_blue_selected(action_id: int):
                    idx = action_id - _SEL_SOURCE_START
                    inner_perm = card.permanent_of_this_card() if card else None
                    if inner_perm and 0 <= idx < len(inner_perm.card_sources):
                        selected_cards.append(inner_perm.card_sources[idx])
                    _select_green()

                game.request_selection(
                    GamePhase.SelectSource,
                    player,
                    on_blue_selected,
                    blue_valid,
                    is_optional=False,
                    prompt="Select 1 level 4 or lower blue Digimon card to play.",
                )
            else:
                _select_green()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
