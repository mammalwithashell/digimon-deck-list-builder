from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_037(CardScript):
    """BT15-037 Gatomon | Lv.4 Yellow Holy Beast

    When an effect trashes this card from the security stack, you may play it
        without paying the cost.
    <Barrier>
    [All Turns] [Once Per Turn] When a card is removed from your security stack,
        gain 1 memory.

    Inherited: <Barrier>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # -----------------------------------------------------------------
        # Factory effect 0: Barrier (non-inherited, main text)
        # -----------------------------------------------------------------
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-037 Barrier")
        effect0.set_effect_description("Barrier")
        effect0._is_barrier = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # -----------------------------------------------------------------
        # Factory effect 1: Barrier (inherited — listed below the line)
        # -----------------------------------------------------------------
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-037 Barrier (inherited)")
        effect1.set_effect_description("Barrier")
        effect1.is_inherited_effect = True
        effect1._is_barrier = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # -----------------------------------------------------------------
        # Effect 2: [All Turns] [Once Per Turn] OnLoseSecurity → +1 memory
        # -----------------------------------------------------------------
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnLoseSecurity)
        effect2.set_effect_name("BT15-037 Gain 1 memory on security lost")
        effect2.set_effect_description(
            "[All Turns] [Once Per Turn] When a card is removed from your "
            "security stack, gain 1 memory."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT15_037_OnLoseSecurity_Memory")

        def condition2(context: Dict[str, Any]) -> bool:
            # Must be on the battle area
            if card and card.permanent_of_this_card() is None:
                return False
            # Owner gate: only fires when OUR OWN security is lost.
            #
            # _fire_timing(OnLoseSecurity, {"player": <owner whose sec was lost>})
            # is merged into context as event_player. For field-zone effects,
            # context['player'] is the effect host owner (Gatomon's owner), so
            # we require event_player to match that.
            owner = card.owner if card else None
            if owner is None:
                return False
            event_player = context.get('event_player')
            if event_player is None:
                # If missing (shouldn't happen from player._fire_timing), be safe
                return False
            if event_player is not owner:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory (for effect owner)."""
            # Use the card owner directly — ctx['player'] is the effect host owner
            # for field-zone effects, which matches card.owner. Use player.add_memory
            # so the direction (turn player vs opponent) is handled correctly.
            player = ctx.get('player')
            if player is None and card is not None:
                player = card.owner
            if player is not None:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # -----------------------------------------------------------------
        # Effect 3: OnDiscardSecurity — when THIS card is trashed from the
        # security stack by an effect, you MAY play it without paying the cost.
        # -----------------------------------------------------------------
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDiscardSecurity)
        effect3.set_effect_name("BT15-037 Play self free when trashed from security")
        effect3.set_effect_description(
            "When an effect trashes this card from the security stack, you "
            "may play it without paying the cost."
        )
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            # Guard: only fire when THIS specific card is the one being trashed
            # from security. The engine fires OnDiscardSecurity once per trashed
            # card, but the trash-zone scan iterates every OnDiscardSecurity
            # effect owner in trash. Without this guard, a previously-trashed
            # Gatomon would re-trigger whenever any other security card is
            # subsequently trashed.
            discarded = context.get('discarded_card')
            if discarded is not card:
                return False
            # The card must actually be in the trash to be played from trash.
            owner = card.owner if card else None
            if owner is None or card not in owner.trash_cards:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play THIS specific card from trash without paying cost.

            The card text only offers one target — this specific Gatomon.
            The only choice is yes/no, which is encoded via `is_optional=True`
            on the effect (the engine exposes that as an auto-resolve branch).
            We do not expose a target selection, because there is nothing to
            choose between.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Defensive: the engine already validates the card is still in trash
            # via _triggered_effect_still_valid, but re-check for safety.
            if card not in player.trash_cards:
                return
            # Remove from trash and play for free
            player.trash_cards.remove(card)
            played_perm = player.play_card_from_source(card, pay_cost=False)
            if played_perm:
                game.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {
                        "played_card": card,
                        "played_permanent": played_perm,
                        "event_player": player,
                        "is_effect_play": True,
                    },
                )
                # Fire post-play observers (e.g. cost reduction triggers clearing)
                if hasattr(game, '_fire_play_observers'):
                    game._fire_play_observers(played_perm, player)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
