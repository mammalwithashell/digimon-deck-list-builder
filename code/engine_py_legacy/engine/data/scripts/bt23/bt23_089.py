from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_089(CardScript):
    """BT23-089 Takumi Aiba"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_cs_trait(c_or_perm):
            """Check if a CardSource or Permanent's top card has [CS] trait."""
            if hasattr(c_or_perm, 'top_card'):
                traits = getattr(c_or_perm.top_card, 'card_traits', []) or []
            else:
                traits = getattr(c_or_perm, 'card_traits', []) or []
            return any('CS' in t for t in traits)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT23-089 Memory +1")
        effect0.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check opponent has a Digimon
            enemy = card.owner.enemy if card.owner else None
            if not enemy or not any(p.is_digimon for p in enemy.battle_area):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your Digimon with the [CS] trait would leave
        # the battle area, by suspending this Tamer and trash 2 same-level
        # cards from 1 of your [CS] trait Digimon's digivolution cards,
        # they don't leave.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenRemoveField)
        effect1.set_effect_name("BT23-089 Prevent CS Digimon from leaving")
        effect1.set_effect_description(
            "[All Turns] When any of your Digimon with the [CS] trait would "
            "leave the battle area, by suspending this Tamer and trash 2 "
            "same-level cards from 1 of your [CS] trait Digimon's digivolution "
            "cards, they don't leave.")
        effect1.is_optional = True
        effect1.set_hash_string("Substitute_BT23_089")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            my_perm = card.permanent_of_this_card()
            # Must not already be suspended (suspend is the cost)
            if my_perm and my_perm.is_suspended:
                return False
            # The leaving permanent must be one of our CS Digimon
            leaving_perm = context.get('event_permanent') or context.get('permanent')
            if not leaving_perm or not leaving_perm.is_digimon:
                return False
            if not _has_cs_trait(leaving_perm):
                return False
            # Check that the leaving perm belongs to us
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            if leaving_perm not in owner.battle_area:
                return False
            # Must have a CS Digimon with at least 2 same-level evo cards
            def _has_trashable_pair(p):
                if not p.is_digimon or not _has_cs_trait(p):
                    return False
                if len(p.card_sources) <= 1:
                    return False
                # Check for 2 same-level cards in digivolution stack
                evo_cards = [cs for cs in p.card_sources
                             if cs is not p.top_card
                             and getattr(cs, 'level', None) is not None]
                levels = {}
                for ec in evo_cards:
                    lv = ec.level
                    levels[lv] = levels.get(lv, 0) + 1
                return any(cnt >= 2 for cnt in levels.values())

            return any(_has_trashable_pair(p) for p in owner.battle_area)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, trash 2 same-level evo cards, prevent leaving"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            leaving_perm = ctx.get('event_permanent') or ctx.get('permanent')
            if not my_perm:
                return

            # Suspend this Tamer as cost
            my_perm.suspend()

            # Select a CS Digimon to trash 2 same-level evo cards from
            def target_filter(p):
                if not p.is_digimon or not _has_cs_trait(p):
                    return False
                if len(p.card_sources) <= 1:
                    return False
                evo_cards = [cs for cs in p.card_sources
                             if cs is not p.top_card
                             and getattr(cs, 'level', None) is not None]
                levels = {}
                for ec in evo_cards:
                    lv = ec.level
                    levels[lv] = levels.get(lv, 0) + 1
                return any(cnt >= 2 for cnt in levels.values())

            def on_target_selected(target_perm):
                # Now select 2 same-level digivolution cards to trash
                # Build list of trashable evo card indices
                evo_cards = [cs for cs in target_perm.card_sources
                             if cs is not target_perm.top_card
                             and getattr(cs, 'level', None) is not None]
                if len(evo_cards) < 2:
                    return

                # Use SelectTrash to let the agent choose the first card
                _SEL_TRASH_START = 130
                valid_indices = []
                evo_card_map = {}
                for i, cs in enumerate(evo_cards):
                    action_id = _SEL_TRASH_START + i
                    valid_indices.append(action_id)
                    evo_card_map[action_id] = cs

                selected_cards = []

                def on_first_selected(action_id):
                    first_card = evo_card_map.get(action_id)
                    if not first_card:
                        return
                    selected_cards.append(first_card)
                    first_level = first_card.level

                    # Select second card with same level
                    valid_second = []
                    for aid, cs in evo_card_map.items():
                        if cs is not first_card and getattr(cs, 'level', None) == first_level:
                            valid_second.append(aid)

                    if not valid_second:
                        return

                    def on_second_selected(action_id2):
                        second_card = evo_card_map.get(action_id2)
                        if not second_card:
                            return
                        selected_cards.append(second_card)

                        # Trash both cards from the digivolution stack
                        for sc in selected_cards:
                            if sc in target_perm.card_sources:
                                target_perm.card_sources.remove(sc)
                                player.trash_cards.append(sc)

                        # Prevent leaving
                        if leaving_perm:
                            leaving_perm._will_not_be_removed = True

                    game.request_selection(
                        GamePhase.SelectTrash, player, on_second_selected,
                        valid_second, is_optional=False,
                        prompt="Select 2nd digivolution card (same level) to trash.")

                game.request_selection(
                    GamePhase.SelectTrash, player, on_first_selected,
                    valid_indices, is_optional=False,
                    prompt="Select 1st digivolution card to trash (will trash 2 same-level).")

            game.effect_select_own_permanent(
                player, on_target_selected, filter_fn=target_filter,
                is_optional=False,
                prompt="Select a [CS] Digimon to trash 2 same-level digivolution cards from.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT23-089 Security: Play this card")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
