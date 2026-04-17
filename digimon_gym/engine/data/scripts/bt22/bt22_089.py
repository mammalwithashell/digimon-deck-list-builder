from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


# C# traits use EqualsTraits(...) which is an EXACT case-insensitive,
# whitespace-insensitive match.
_C2_TRAITS = {"Holy Beast", "Angel", "Archangel", "Fallen Angel", "CS"}


def _trait_eq(trait: str, target: str) -> bool:
    """Mirror CardSource.EqualsTraits: exact or case/space-insensitive match."""
    if not trait or not target:
        return False
    if trait == target:
        return True
    return trait.replace(" ", "").lower() == target.replace(" ", "").lower()


def _has_any_trait(card: "CardSource", targets) -> bool:
    traits = getattr(card, "card_traits", []) or []
    return any(_trait_eq(t, target) for t in traits for target in targets)


class BT22_089(CardScript):
    """BT22-089 Mirei Mikagura"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ─────────────────────────────────────────────────────────────
        # C1: [Start of Your Main Phase]
        # By returning this Tamer to the bottom of the deck, you may
        # play 1 play cost 4 or higher [Mirei Mikagura] or play cost 4
        # or higher Tamer card with the [CS] trait from your hand
        # without paying the cost.
        # ─────────────────────────────────────────────────────────────
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name(
            "BT22-089 By bottom decking, Play 1 4 cost or higher [Mirei Mikagura] or [CS] tamer from hand"
        )
        effect0.set_effect_description(
            "[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, "
            "you may play 1 play cost 4 or higher [Mirei Mikagura] or play cost 4 or higher "
            "Tamer card with the [CS] trait from your hand without paying the cost."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Must be on battle area (IsExistOnBattleArea)
            if not card or card.permanent_of_this_card() is None:
                return False
            # Must be owner's turn (IsOwnerTurn)
            if not (card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Return this Tamer to deck bottom, then play 1 qualifying tamer from hand free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return

            # Cost: return this Tamer to the bottom of the deck (use engine API)
            tamer_perm = card.permanent_of_this_card()
            if not tamer_perm:
                return
            player.return_permanent_to_deck_bottom(tamer_perm)

            # If the return was blocked (CANNOT_BE_RETURNED modifier or
            # _is_cannot_return_to_deck keyword), the cost was not paid —
            # do NOT trigger the success branch.
            if tamer_perm in player.battle_area:
                return

            # Success: let player pick a valid card from hand to play free.
            # Per C#: HasPlayCost && BasePlayCostFromEntity >= 4
            #         && (EqualsCardName("Mirei Mikagura") || (IsTamer && HasCSTraits))
            def play_filter(c) -> bool:
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) < 4:
                    return False
                names = getattr(c, 'card_names', []) or []
                is_mirei = any(n == 'Mirei Mikagura' for n in names)
                is_cs_tamer = (
                    getattr(c, 'is_tamer', False)
                    and _has_any_trait(c, {'CS'})
                )
                return is_mirei or is_cs_tamer

            game.effect_play_from_zone(
                player, 'hand', play_filter,
                free=True, is_optional=True,
                prompt="Select 1 [Mirei Mikagura] or cost 4+ [CS] Tamer from hand to play without paying the cost.",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─────────────────────────────────────────────────────────────
        # C2: [On Play]
        # By trashing 1 card with the [Holy Beast], [Angel], [Archangel],
        # [Fallen Angel] or [CS] trait from your hand, <Draw 2>.
        # ─────────────────────────────────────────────────────────────
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name(
            "BT22-089 By trashing 1 [Holy Beast]/[Angel]/[Archangel]/[Fallen Angel]/[CS], Draw 2"
        )
        effect1.set_effect_description(
            "[On Play] By trashing 1 card with the [Holy Beast], [Angel], [Archangel], "
            "[Fallen Angel] or [CS] trait from your hand, <Draw 2> "
            "(Draw 2 cards from your deck.)"
        )
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Must be on battle area
            if not card or card.permanent_of_this_card() is None:
                return False
            owner = card.owner
            if not owner:
                return False
            # Must have at least 1 qualifying card in hand (by-cost must be payable)
            return any(_has_any_trait(c, _C2_TRAITS) for c in owner.hand_cards)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Trash 1 qualifying card from hand, then Draw 2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c) -> bool:
                return _has_any_trait(c, _C2_TRAITS)

            def on_trashed(selected):
                # Use engine helper to move hand → trash
                player.trash_from_hand([selected])
                # Draw 2 (the draw is mandatory once the by-cost is paid)
                player.draw_cards(2)

            # Per C# canNoSelect: true, but the selection is offered only if
            # a valid target exists. Decline means the whole by-cost effect
            # aborts. In the Python engine, this optionality is already exposed
            # via effect1.is_optional at the trigger gate.
            game.effect_select_hand_card(
                player, hand_filter, on_trashed,
                is_optional=False,
                prompt="Select 1 card with [Holy Beast]/[Angel]/[Archangel]/[Fallen Angel]/[CS] trait to trash.",
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ─────────────────────────────────────────────────────────────
        # C3: [Security] Play this card without paying the cost.
        # (Inherited — PlaySelfTamerSecurityEffect factory in C#)
        # ─────────────────────────────────────────────────────────────
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT22-089 Security: Play this card")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this Tamer from security without paying the cost."""
            game = ctx.get('game')
            player = ctx.get('player')
            if not (game and player and card):
                return
            game.effect_play_from_security(player, card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
