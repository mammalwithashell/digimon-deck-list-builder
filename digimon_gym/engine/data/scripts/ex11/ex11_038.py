from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_038(CardScript):
    """EX11-038 Sunarizamon | Lv.3

    [When Moving] [On Play] By trashing 1 [Mineral] or [Rock] trait card
    from your hand or your Digimon's digivolution cards, Draw 1.

    Inherited: When effects trash this card from a [Mineral] or [Rock]
    trait Digimon's digivolution cards, Draw 1.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_mineral_or_rock(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return 'Mineral' in traits or 'Rock' in traits

        def _has_mineral_rock_in_hand(player) -> bool:
            if not player:
                return False
            return any(_is_mineral_or_rock(c) for c in player.hand_cards)

        def _has_mineral_rock_in_sources(player) -> bool:
            """Check if any Digimon's digivolution cards contain a Mineral/Rock card."""
            if not player:
                return False
            for perm in player.battle_area:
                for c in perm.digivolution_cards:
                    if c is perm.top_card:
                        continue
                    if _is_mineral_or_rock(c):
                        return True
            ba = player.breeding_area
            if ba:
                for c in ba.digivolution_cards:
                    if c is ba.top_card:
                        continue
                    if _is_mineral_or_rock(c):
                        return True
            return False

        def _trash_from_hand(player, game):
            """Let player select a Mineral/Rock card from hand to trash, then draw 1."""
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                player.draw_cards(1)

            game.effect_select_hand_card(
                player, _is_mineral_or_rock, on_trashed, is_optional=True,
                prompt="Trash 1 [Mineral] or [Rock] card from hand to draw 1.")

        def _trash_from_sources(player, game):
            """Let player select a Mineral/Rock card from any Digimon's
            digivolution cards to trash, then draw 1."""
            from ....data.enums import GamePhase
            from ....game.constants import SOURCES_PER_FIELD

            valid = []
            # Map action_id -> (permanent, card_source)
            source_map = {}

            for fi, perm in enumerate(player.battle_area):
                base = 2000 + fi * SOURCES_PER_FIELD
                top = perm.top_card
                for si, cs in enumerate(perm.card_sources):
                    if cs is top:
                        continue
                    action_id = base + si
                    if action_id >= 2168:  # ACTION_SPACE_SIZE
                        continue
                    if _is_mineral_or_rock(cs):
                        valid.append(action_id)
                        source_map[action_id] = (perm, cs)

            if not valid:
                return

            def on_source_selected(action_id):
                entry = source_map.get(action_id)
                if not entry:
                    return
                perm, selected = entry
                # Remove from digivolution cards and fire timing
                if selected in perm.card_sources:
                    perm.card_sources.remove(selected)
                    trashed = [selected]
                    player.trash_cards.append(selected)
                    # Fire OnDigivolutionCardDiscarded so inherited effects trigger
                    perm._fire_timing(EffectTiming.OnDigivolutionCardDiscarded,
                                      {"permanent": perm, "trashed_cards": trashed})
                player.draw_cards(1)

            game.request_selection(
                GamePhase.SelectSource, player, on_source_selected,
                valid, is_optional=True,
                prompt="Trash 1 [Mineral] or [Rock] card from digivolution cards to draw 1.")

        def _trash_and_draw(ctx: Dict[str, Any]):
            """By trashing 1 [Mineral] or [Rock] card from hand or sources, Draw 1.

            DCGO reference: presents zone choice (hand / digi-sources / do not trash)
            when both zones have eligible cards. Single zone skips the choice.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            can_hand = _has_mineral_rock_in_hand(player)
            can_sources = _has_mineral_rock_in_sources(player)

            if can_hand and can_sources:
                # Both zones available: let player choose which zone
                def on_zone_choice(branch: int):
                    if branch == 0:
                        _trash_from_hand(player, game)
                    else:
                        _trash_from_sources(player, game)

                game.effect_choose_branch(
                    player, 2, on_zone_choice,
                    prompt="From which area will you trash a card?",
                    branch_labels=["From hand", "From digivolution cards"])
            elif can_hand:
                _trash_from_hand(player, game)
            elif can_sources:
                _trash_from_sources(player, game)

        # [When Moving]
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnMove)
        effect0.set_effect_name("EX11-038 Trash 1 Mineral/Rock to draw 1")
        effect0.set_effect_description(
            "[When Moving] By trashing 1 [Mineral] or [Rock] trait card from "
            "your hand or your Digimon's digivolution cards, Draw 1."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only fire for the permanent that actually moved (DCGO: CanTriggerOnMove)
            moved_perm = context.get('moved_permanent')
            if moved_perm is not None and moved_perm is not card.permanent_of_this_card():
                return False
            owner = card.owner if card else None
            return _has_mineral_rock_in_hand(owner) or _has_mineral_rock_in_sources(owner)

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_trash_and_draw)
        effects.append(effect0)

        # [On Play]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-038 Trash 1 Mineral/Rock to draw 1")
        effect1.set_effect_description(
            "[On Play] By trashing 1 [Mineral] or [Rock] trait card from "
            "your hand or your Digimon's digivolution cards, Draw 1."
        )
        effect1.is_on_play = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            return _has_mineral_rock_in_hand(owner) or _has_mineral_rock_in_sources(owner)

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_trash_and_draw)
        effects.append(effect1)

        # Inherited: When effects trash this card from a [Mineral] or [Rock]
        # trait Digimon's digivolution cards, Draw 1.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect2.set_effect_name("EX11-038 Draw 1 when trashed from Mineral/Rock sources")
        effect2.set_effect_description(
            "When effects trash this card from a [Mineral] or [Rock] trait "
            "Digimon's digivolution cards, Draw 1."
        )
        effect2.is_inherited_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            trashed_cards = context.get('trashed_cards', [])
            if card not in trashed_cards:
                return False
            permanent = context.get('permanent')
            if permanent is None:
                return False
            # DCGO: checks top card's traits (permanent.TopCard.EqualsTraits)
            if not (permanent.has_trait('Mineral') or permanent.has_trait('Rock')):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
