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

        def _has_mineral_rock_in_hand(player) -> bool:
            if not player:
                return False
            return any(
                'Mineral' in (getattr(c, 'card_traits', []) or [])
                or 'Rock' in (getattr(c, 'card_traits', []) or [])
                for c in player.hand_cards
            )

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
            ba = player.breeding_area
            if ba:
                for c in ba.digivolution_cards:
                    if c is ba.top_card:
                        continue
                    traits = getattr(c, 'card_traits', []) or []
                    if 'Mineral' in traits or 'Rock' in traits:
                        return True
            return False

        def _trash_and_draw(ctx: Dict[str, Any]):
            """By trashing 1 [Mineral] or [Rock] card from hand or sources, Draw 1."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Try to trash from hand first (Mineral/Rock trait)
            def hand_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return 'Mineral' in traits or 'Rock' in traits

            hand_candidates = [c for c in player.hand_cards if hand_filter(c)]

            if hand_candidates:
                def on_trashed(selected):
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                        player.trash_cards.append(selected)
                    player.draw_cards(1)

                game.effect_select_hand_card(
                    player, hand_filter, on_trashed, is_optional=True,
                    prompt="Trash 1 [Mineral] or [Rock] card from hand to draw 1.")
            else:
                # Trash 1 Mineral/Rock from any Digimon's digivolution cards
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
                    player.draw_cards(1)

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
