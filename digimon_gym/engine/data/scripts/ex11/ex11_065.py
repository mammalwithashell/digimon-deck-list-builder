from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_065(CardScript):
    """EX11-065 Close | Tamer

    [Start of Your Main Phase] By trashing 1 [Mineral] or [Rock] trait
    card from your hand or your Digimon's digivolution cards, gain 1 memory.

    [All Turns] When your Digimon are played or digivolve, if any of them
    have the [Mineral] or [Rock] trait, by suspending this Tamer, you may
    place 1 [Mineral] or [Rock] trait card from your hand or trash as any
    of those Digimon's bottom digivolution card.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Helper: check if player has Mineral/Rock in hand
        def _has_mineral_rock_in_hand(player) -> bool:
            if not player:
                return False
            return any(
                'Mineral' in (getattr(c, 'card_traits', []) or [])
                or 'Rock' in (getattr(c, 'card_traits', []) or [])
                for c in player.hand_cards
            )

        # Helper: check if player has Mineral/Rock in any sources
        def _has_mineral_rock_in_sources(player) -> bool:
            if not player:
                return False
            for perm in player.battle_area:
                for c in perm.digivolution_cards:
                    if c is perm.top_card:
                        continue
                    traits = getattr(c, 'card_traits', []) or []
                    if 'Mineral' in traits or 'Rock' in traits:
                        return True
            return False

        # [Start of Your Main Phase] By trashing 1 [Mineral] or [Rock] from
        # hand or sources, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX11-065 Trash 1 Mineral/Rock to gain 1 memory")
        effect0.set_effect_description(
            "[Start of Your Main Phase] By trashing 1 [Mineral] or [Rock] "
            "trait card from your hand or your Digimon's digivolution cards, "
            "gain 1 memory."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            return _has_mineral_rock_in_hand(owner) or _has_mineral_rock_in_sources(owner)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return 'Mineral' in traits or 'Rock' in traits

            hand_candidates = [c for c in player.hand_cards if hand_filter(c)]

            if hand_candidates:
                def on_trashed(selected):
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                        player.trash_cards.append(selected)
                    player.add_memory(1)

                game.effect_select_hand_card(
                    player, hand_filter, on_trashed, is_optional=True,
                    prompt="Trash 1 [Mineral]/[Rock] card from hand to gain 1 memory.")
            else:
                # Trash from sources
                trashed = False
                for field_perm in player.battle_area:
                    if trashed:
                        break
                    for c in list(field_perm.digivolution_cards):
                        if c is field_perm.top_card:
                            continue
                        traits = getattr(c, 'card_traits', []) or []
                        if 'Mineral' in traits or 'Rock' in traits:
                            field_perm.card_sources.remove(c)
                            player.trash_cards.append(c)
                            trashed = True
                            break
                if trashed:
                    player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [All Turns] When your Digimon are played or digivolve with Mineral/Rock,
        # suspend this tamer to place 1 Mineral/Rock from hand/trash as bottom source.
        # Uses OnEnterFieldAnyone (fires for both play and digivolve).
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-065 Suspend to place Mineral/Rock under Digimon")
        effect1.set_effect_description(
            "[All Turns] When your Digimon are played or digivolve, if any of "
            "them have the [Mineral] or [Rock] trait, by suspending this Tamer, "
            "you may place 1 [Mineral] or [Rock] trait card from your hand or "
            "trash as any of those Digimon's bottom digivolution card."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Tamer must not be suspended
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # The played/digivolved Digimon must have Mineral or Rock trait
            event_perm = context.get('played_permanent') or context.get('digivolved_permanent')
            if event_perm is None:
                return False
            if not (event_perm.has_trait('Mineral') or event_perm.has_trait('Rock')):
                return False
            # Must own the event permanent
            owner = card.owner if card else None
            if not owner or event_perm not in owner.battle_area:
                return False
            # Must have Mineral/Rock in hand or trash
            return any(
                'Mineral' in (getattr(c, 'card_traits', []) or [])
                or 'Rock' in (getattr(c, 'card_traits', []) or [])
                for c in list(owner.hand_cards) + list(owner.trash_cards)
            )

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Suspend this tamer as cost
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()

            # Get the event permanent
            event_perm = ctx.get('played_permanent') or ctx.get('digivolved_permanent')
            if not event_perm:
                return

            # Place 1 Mineral/Rock from hand or trash as bottom source
            def hand_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return 'Mineral' in traits or 'Rock' in traits

            hand_candidates = [c for c in player.hand_cards if hand_filter(c)]

            if hand_candidates:
                def on_selected(selected):
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                        event_perm.add_card_source_bottom(selected)

                game.effect_select_hand_card(
                    player, hand_filter, on_selected, is_optional=True,
                    prompt="Place 1 [Mineral]/[Rock] card from hand under the Digimon.")
            else:
                # Place from trash
                placed = False
                for c in list(player.trash_cards):
                    if placed:
                        break
                    traits = getattr(c, 'card_traits', []) or []
                    if 'Mineral' in traits or 'Rock' in traits:
                        player.trash_cards.remove(c)
                        event_perm.add_card_source_bottom(c)
                        placed = True

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Also fire on WhenDigivolving for digivolve events
        effect1b = ICardEffect()
        effect1b.set_timing(EffectTiming.WhenDigivolving)
        effect1b.set_effect_name("EX11-065 Suspend to place Mineral/Rock under digivolved Digimon")
        effect1b.set_effect_description(
            "[All Turns] When your Digimon digivolve into [Mineral]/[Rock] trait, "
            "by suspending this Tamer, place 1 from hand/trash as bottom source."
        )
        effect1b.is_optional = True

        def condition1b(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            digivolved_perm = context.get('digivolved_permanent')
            if digivolved_perm is None:
                return False
            if not (digivolved_perm.has_trait('Mineral') or digivolved_perm.has_trait('Rock')):
                return False
            owner = card.owner if card else None
            if not owner or digivolved_perm not in owner.battle_area:
                return False
            return any(
                'Mineral' in (getattr(c, 'card_traits', []) or [])
                or 'Rock' in (getattr(c, 'card_traits', []) or [])
                for c in list(owner.hand_cards) + list(owner.trash_cards)
            )

        effect1b.set_can_use_condition(condition1b)

        def process1b(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()

            digivolved_perm = ctx.get('digivolved_permanent')
            if not digivolved_perm:
                return

            def hand_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return 'Mineral' in traits or 'Rock' in traits

            hand_candidates = [c for c in player.hand_cards if hand_filter(c)]
            if hand_candidates:
                def on_selected(selected):
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                        digivolved_perm.add_card_source_bottom(selected)

                game.effect_select_hand_card(
                    player, hand_filter, on_selected, is_optional=True,
                    prompt="Place 1 [Mineral]/[Rock] card from hand under the Digimon.")
            else:
                placed = False
                for c in list(player.trash_cards):
                    if placed:
                        break
                    traits = getattr(c, 'card_traits', []) or []
                    if 'Mineral' in traits or 'Rock' in traits:
                        player.trash_cards.remove(c)
                        digivolved_perm.add_card_source_bottom(c)
                        placed = True

        effect1b.set_on_process_callback(process1b)
        effects.append(effect1b)

        # Security: Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX11-065 Security: Play this card")
        effect2.set_effect_description("Security: Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
