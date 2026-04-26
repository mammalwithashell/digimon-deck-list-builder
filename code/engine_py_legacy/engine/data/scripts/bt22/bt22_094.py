from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_094(CardScript):
    """BT22-094 Yuugo Kamishiro | Tamer | White | Cost 3 | CS

    [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [CS]
    trait among them to the hand. Return the rest to the bottom of the deck.

    [Your Turn] When any of your Digimon or Tamers with the [CS] trait would
    be played, by returning this Tamer to the bottom of the deck, reduce the
    play cost by 2.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Effect 0: [On Play] reveal top 3, pick 1 CS to hand, rest to
        # bottom of deck. Matches C# SimplifiedRevealDeckTopCardsAndSelect
        # with CanSelectCardCondition = HasCSTraits and
        # RemainingCardsPlace.DeckBottom.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name(
            "BT22-094 Reveal top 3, add 1 [CS] card to hand, bottom deck rest"
        )
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 card with "
            "the [CS] trait among them to the hand. Return the rest to the "
            "bottom of the deck."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Must be on field when the on-play effect fires
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 3 cards; agent picks a [CS] card to hand; rest to bottom."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def is_cs_card(c) -> bool:
                traits = getattr(c, 'card_traits', []) or []
                return 'CS' in traits

            game.effect_reveal_and_select_multi(
                player, 3,
                passes=[(is_cs_card, 'hand')],
                remaining_placement='deck_bottom',
                is_optional=False,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ── Effect 1: [Your Turn] BeforePayCost — when an owned CS Digimon
        # or Tamer would be played, by returning this Tamer to the bottom of
        # the deck, reduce the play cost by 2.
        #
        # NOTE: The engine's calculate_play_cost auto-applies BeforePayCost
        # effects whose condition passes (no per-play prompt). is_optional
        # is set as an intent marker. Engine gap: agent cannot decline the
        # reduction. See qa/archetype-qa/engine-gaps.md (BeforePayCost
        # optionality).
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name(
            "BT22-094 Bottom deck this Tamer, reduce CS play cost by 2"
        )
        effect1.set_effect_description(
            "[Your Turn] When any of your Digimon or Tamers with the [CS] "
            "trait would be played, by returning this Tamer to the bottom "
            "of the deck, reduce the play cost by 2."
        )
        effect1.is_optional = True
        effect1.cost_reduction = 2

        def condition1(context: Dict[str, Any]) -> bool:
            # LEAK GUARD: never reduce this Tamer's own play cost.
            cs = context.get('card_source')
            if cs is card:
                return False

            # Must be on field
            own_perm = card.permanent_of_this_card() if card else None
            if own_perm is None:
                return False

            # Must be on owner's turn (C#: IsOwnerTurn(card))
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False

            # The card being played must exist
            if cs is None:
                return False

            # Same owner (C#: cardSource.Owner == card.Owner)
            if getattr(cs, 'owner', None) is not owner:
                return False

            # Card being played must be a Digimon or Tamer
            if not (getattr(cs, 'is_digimon', False)
                    or getattr(cs, 'is_tamer', False)):
                return False

            # Card being played must have the [CS] trait (exact match)
            traits = getattr(cs, 'card_traits', []) or []
            if 'CS' not in traits:
                return False

            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Return this Tamer to the bottom of the deck as the cost."""
            player = ctx.get('player')
            if not player:
                return
            own_perm = card.permanent_of_this_card() if card else None
            if own_perm is not None and own_perm in player.battle_area:
                player.return_permanent_to_deck_bottom(own_perm)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── Effect 2: [Security] Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT22-094 Security: Play this card")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this card from security without paying the cost."""
            game = ctx.get('game')
            player = ctx.get('player')
            if game and player and card is not None:
                game.effect_play_from_security(player, card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
