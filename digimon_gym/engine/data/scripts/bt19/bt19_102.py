from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_102(CardScript):
    """BT19-102 Luminamon (Nene Version) | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-102 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Luminamon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Luminamon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Luminamon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-102 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Nene Amano] for cost 3
        effect1._alt_digi_cost = 3
        effect1._alt_digi_name = "Nene Amano"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and any(src.contains_card_name('Shademon') for src in permanent.digivolution_cards)):
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Nene Amano'))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-102 Effect")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Choose 1 other Digimon. By playing level 4 or lower Digimon card from it's digivolution cards, delete the chosen Digimon.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT19-102 Play level 4 from source, delete target Digimon")
        effect3.set_effect_description("[On Play] Choose 1 other Digimon. By playing level 4 or lower Digimon card from it's digivolution cards, delete the chosen Digimon.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Choose opponent Digimon, play Lv.4- from this perm's digi-sources, delete chosen"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Step 1: Choose 1 other Digimon (opponent's)
            def target_filter(p):
                return p.is_digimon

            def on_target_selected(target_perm):
                if target_perm is None:
                    return
                # Step 2: Play a level 4 or lower Digimon card from this
                # permanent's digivolution cards (card_sources)
                candidates = [
                    cs for cs in perm.card_sources
                    if cs is not perm.top_card
                    and getattr(cs, 'is_digimon', False)
                    and getattr(cs, 'level', None) is not None
                    and cs.level <= 4
                ]
                if not candidates:
                    return
                from ....data.enums import GamePhase
                from ....game.constants import SOURCES_PER_FIELD
                # Find field index of this permanent
                field_idx = None
                for fi, fp in enumerate(player.battle_area):
                    if fp is perm:
                        field_idx = fi
                        break
                if field_idx is None:
                    return
                base = 2000 + field_idx * SOURCES_PER_FIELD
                valid = []
                for i, cs in enumerate(perm.card_sources):
                    if cs in candidates and (base + i) < 2168:
                        valid.append(base + i)
                if not valid:
                    return

                def on_source_selected(action_id):
                    idx = action_id - base
                    if not (0 <= idx < len(perm.card_sources)):
                        return
                    chosen = perm.card_sources[idx]
                    perm.card_sources.remove(chosen)
                    player.play_card_from_source(chosen, pay_cost=False)
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.request_selection(
                    GamePhase.SelectSource, player, on_source_selected,
                    valid, is_optional=True,
                    prompt="Select a level 4 or lower Digimon from this permanent's digivolution cards to play.")

            game.effect_select_opponent_permanent(
                player, on_target_selected, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Choose 1 other Digimon. By playing level 4 or lower Digimon card from it's digivolution cards, delete the chosen Digimon.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT19-102 Play level 4 from source, delete target Digimon")
        effect4.set_effect_description("[When Digivolving] Choose 1 other Digimon. By playing level 4 or lower Digimon card from it's digivolution cards, delete the chosen Digimon.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Choose opponent Digimon, play Lv.4- from this perm's digi-sources, delete chosen"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Step 1: Choose 1 other Digimon (opponent's)
            def target_filter(p):
                return p.is_digimon

            def on_target_selected(target_perm):
                if target_perm is None:
                    return
                candidates = [
                    cs for cs in perm.card_sources
                    if cs is not perm.top_card
                    and getattr(cs, 'is_digimon', False)
                    and getattr(cs, 'level', None) is not None
                    and cs.level <= 4
                ]
                if not candidates:
                    return
                from ....data.enums import GamePhase
                from ....game.constants import SOURCES_PER_FIELD
                field_idx = None
                for fi, fp in enumerate(player.battle_area):
                    if fp is perm:
                        field_idx = fi
                        break
                if field_idx is None:
                    return
                base = 2000 + field_idx * SOURCES_PER_FIELD
                valid = []
                for i, cs in enumerate(perm.card_sources):
                    if cs in candidates and (base + i) < 2168:
                        valid.append(base + i)
                if not valid:
                    return

                def on_source_selected(action_id):
                    idx = action_id - base
                    if not (0 <= idx < len(perm.card_sources)):
                        return
                    chosen = perm.card_sources[idx]
                    perm.card_sources.remove(chosen)
                    player.play_card_from_source(chosen, pay_cost=False)
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.request_selection(
                    GamePhase.SelectSource, player, on_source_selected,
                    valid, is_optional=True,
                    prompt="Select a level 4 or lower Digimon from this permanent's digivolution cards to play.")

            game.effect_select_opponent_permanent(
                player, on_target_selected, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 card with a play cost of 5 or less from under any of your Tamers without paying the cost.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnDestroyedAnyone)
        effect5.set_effect_name("BT19-102 Play Card")
        effect5.set_effect_description("[On Deletion] You may play 1 card with a play cost of 5 or less from under any of your Tamers without paying the cost.")
        effect5.is_on_deletion = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play 1 card with play cost 5 or less from under any of your Tamers"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Gather all qualifying cards from under all own Tamers
            from ....data.enums import GamePhase
            from ....game.constants import SOURCES_PER_FIELD
            candidates = []  # (tamer_perm, field_idx, source_index, card_source)
            for fi, tamer in enumerate(player.battle_area):
                if not tamer.is_tamer:
                    continue
                for si, cs in enumerate(tamer.card_sources):
                    if cs is tamer.top_card:
                        continue
                    cost = getattr(cs, 'get_cost_itself', 0)
                    if isinstance(cost, property):
                        cost = 0
                    if cost <= 5:
                        candidates.append((tamer, fi, si, cs))
            if not candidates:
                return
            # Select tamer first, then source from it
            tamer_set = list({t for t, _, _, _ in candidates})

            def _select_source_from_tamer(selected_tamer):
                if not selected_tamer:
                    return
                fi = None
                for idx, p in enumerate(player.battle_area):
                    if p is selected_tamer:
                        fi = idx
                        break
                if fi is None:
                    return
                base = 2000 + fi * SOURCES_PER_FIELD
                valid = []
                for t, _, si, cs in candidates:
                    if t is selected_tamer:
                        aid = base + si
                        if aid < 2168:
                            valid.append(aid)
                if not valid:
                    return

                def on_src_sel(action_id):
                    idx = action_id - base
                    if not (0 <= idx < len(selected_tamer.card_sources)):
                        return
                    chosen = selected_tamer.card_sources[idx]
                    selected_tamer.card_sources.remove(chosen)
                    player.play_card_from_source(chosen, pay_cost=False)

                game.request_selection(
                    GamePhase.SelectSource, player, on_src_sel,
                    valid, is_optional=True,
                    prompt="Select a card to play from under this Tamer.")

            if len(tamer_set) == 1:
                _select_source_from_tamer(tamer_set[0])
            else:
                def tamer_filter(p):
                    return p in tamer_set

                game.effect_select_own_permanent(
                    player, _select_source_from_tamer, filter_fn=tamer_filter,
                    is_optional=True,
                    prompt="Select one of your Tamers to play a card from under.")

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
