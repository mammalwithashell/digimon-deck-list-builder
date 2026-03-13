from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_066(CardScript):
    """EX11-066 Xeno | Tamer

    This card is also treated as [Zenith]. (Engine gap — name aliasing not modeled.)

    [On Play] [Start of Your Main Phase] By trashing 1 card with [Vemmon] in
    its text from your hand, <Draw 1> and gain 1 memory.

    [All Turns] When your Digimon are played or digivolve, if any of them have
    [Vemmon] in their texts, by suspending this Tamer, reveal the top 2 cards
    of your deck. Place all [Vemmon] among them as any of those Digimon's
    bottom digivolution cards. Trash the rest.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_vemmon_text(c) -> bool:
            return 'Vemmon' in getattr(c, 'card_text', '')

        def _is_vemmon_name(c) -> bool:
            return any('Vemmon' in n for n in getattr(c, 'card_names', []))

        # ─── Effect 0: Also treated as [Zenith]
        # Engine gap — name aliasing not modeled. Stub retained for documentation.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-066 Also treated as [Zenith]")
        effect0.set_effect_description("Also Treated As")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Also treated as [Zenith] — name aliasing not modeled in engine."""
            pass  # descriptive-tagged: also_treated_as_name

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Shared process for On Play / Start of Your Main Phase
        # Cost: trash 1 card with [Vemmon] in text from hand.
        # Then: Draw 1, gain 1 memory.
        def _shared_trash_draw_memory(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Must have a card with [Vemmon] in text in hand to pay the cost
            vemmon_in_hand = [c for c in player.hand_cards if _has_vemmon_text(c)]
            if not vemmon_in_hand:
                return

            def hand_filter(c):
                return _has_vemmon_text(c)

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    # After paying trash cost, draw 1 and gain 1 memory
                    player.draw_cards(1)
                    player.add_memory(1)

            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False,
                prompt="Trash 1 card with [Vemmon] in its text from your hand.")

        # ─── Effect 1: [Start of Your Main Phase]
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("EX11-066 Trash Vemmon-text card, draw 1, gain 1 memory")
        effect1.set_effect_description("[Start of Your Main Phase] By trashing 1 card with [Vemmon] in its text from your hand, <Draw 1> and gain 1 memory.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must have a [Vemmon]-text card in hand to pay cost
            owner = card.owner
            if not any(_has_vemmon_text(c) for c in owner.hand_cards):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_shared_trash_draw_memory)
        effects.append(effect1)

        # ─── Effect 2: [On Play]
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-066 Trash Vemmon-text card, draw 1, gain 1 memory")
        effect2.set_effect_description("[On Play] By trashing 1 card with [Vemmon] in its text from your hand, <Draw 1> and gain 1 memory.")
        effect2.is_on_play = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must have a [Vemmon]-text card in hand to pay cost
            owner = card.owner if card else None
            if not owner:
                return False
            if not any(_has_vemmon_text(c) for c in owner.hand_cards):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_trash_draw_memory)
        effects.append(effect2)

        # ─── Effect 3: [All Turns] When your Digimon are played or digivolve
        # NOTE: Engine gap — OnEnterFieldAnyone dispatches only to the permanent
        # that was played/digivolved, not to other permanents like this Tamer.
        # The logic below is correct per card text but won't trigger automatically
        # from the engine's effect dispatch for other Digimon's play/digivolve.
        # Using OnEnterFieldAnyone without is_on_play so it fires in the
        # no-flag timing match path (which still requires perm.top_card == played_card,
        # so effectively this only fires on the Tamer's own play as a side effect).
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX11-066 Suspend, reveal 2, place Vemmon as bottom digi-cards")
        effect3.set_effect_description("[All Turns] When your Digimon are played or digivolve, if any of them have [Vemmon] in their texts, by suspending this Tamer, reveal the top 2 cards of your deck. Place all [Vemmon] among them as any of those Digimon's bottom digivolution cards. Trash the rest.")
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # This Tamer must not already be suspended (suspend is the cost)
            perm = card.permanent_of_this_card()
            if perm and getattr(perm, 'is_suspended', False):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Suspend this Tamer, reveal top 2, place [Vemmon] as bottom digi-cards,
            trash the rest."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Cost: suspend this Tamer
            tamer_perm = card.permanent_of_this_card()
            if not tamer_perm:
                return
            tamer_perm.suspend()

            # Reveal top 2 cards
            revealed = []
            for _ in range(min(2, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))

            if not revealed:
                return

            # Separate Vemmon-named cards from the rest
            vemmon_cards = [c for c in revealed if _is_vemmon_name(c)]
            non_vemmon = [c for c in revealed if not _is_vemmon_name(c)]

            # Place all [Vemmon] as bottom digi-cards of the triggering Digimon.
            # Since the engine gap means we can't identify which specific Digimon
            # triggered this, place on own Digimon with [Vemmon] in text if available.
            # In the context, 'permanent' is the permanent that triggered the effect.
            target_perm = ctx.get('permanent')
            if target_perm and target_perm.is_digimon:
                for vc in vemmon_cards:
                    target_perm.add_card_source_bottom(vc)
            else:
                # Fallback: find a Digimon with Vemmon in text on own field
                own_vemmon_digimon = [
                    p for p in player.battle_area
                    if p.is_digimon and _has_vemmon_text(p.top_card) if p.top_card
                ]
                if own_vemmon_digimon:
                    target = own_vemmon_digimon[0]
                    for vc in vemmon_cards:
                        target.add_card_source_bottom(vc)
                else:
                    # No valid target — all go to trash
                    non_vemmon.extend(vemmon_cards)

            # Trash the rest
            for c in non_vemmon:
                player.trash_cards.append(c)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # ─── Effect 4: Security — play this card without paying the cost
        effect4 = ICardEffect()
        effect4.set_effect_name("EX11-066 Security: Play this card")
        effect4.set_effect_description("Security: Play this card")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
