from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_091(CardScript):
    """BT20-091 Cool Boy"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_royal_knight_trait(card_source) -> bool:
            """Check if a card source has the Royal Knight trait."""
            traits = getattr(card_source, 'card_traits', []) or []
            return any("Royal Knight" in trait for trait in traits)

        def _draw_and_gain_memory(ctx: Dict[str, Any]):
            """Shared process: suspend this tamer, draw 1, gain 1 memory."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not (player and perm):
                return
            # Pay cost: suspend this tamer
            perm.suspend()
            # Then draw 1 and gain 1 memory
            player.draw_cards(1)
            player.add_memory(1)

        # effect0a: [Your Turn] When any of your Digimon are PLAYED, if they have
        # [Royal Knight] trait, by suspending this Tamer, <Draw 1> and gain 1 memory.
        effect0a = ICardEffect()
        effect0a.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0a.set_effect_name("BT20-091 Draw 1 and gain 1 memory (on play)")
        effect0a.set_effect_description(
            "[Your Turn] When any of your Digimon are played, if any of them have "
            "the [Royal Knight] trait, by suspending this Tamer, <Draw 1> and gain 1 memory."
        )
        effect0a.is_optional = True  # "by suspending" makes this optional (player can decline)

        def condition0a(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # Can't pay cost if already suspended
            if perm.is_suspended:
                return False
            # Check that a Digimon was played (not this tamer itself)
            played_card_src = context.get('played_card')
            if not played_card_src:
                return False
            if not getattr(played_card_src, 'is_digimon', False):
                return False
            # Must be OUR Digimon
            played_perm = context.get('played_permanent') or context.get('event_permanent')
            if played_perm:
                owner = context.get('game')
                if owner and hasattr(owner, '_find_owner'):
                    perm_owner = owner._find_owner(played_perm)
                    if perm_owner is not card.owner:
                        return False
            # Check Royal Knight trait
            if not _has_royal_knight_trait(played_card_src):
                return False
            return True

        effect0a.set_can_use_condition(condition0a)
        effect0a.set_on_process_callback(_draw_and_gain_memory)
        effects.append(effect0a)

        # effect0b: [Your Turn] When any of your Digimon DIGIVOLVE, if they have
        # [Royal Knight] trait, by suspending this Tamer, <Draw 1> and gain 1 memory.
        effect0b = ICardEffect()
        effect0b.set_timing(EffectTiming.WhenDigivolving)
        effect0b.set_effect_name("BT20-091 Draw 1 and gain 1 memory (on digivolve)")
        effect0b.set_effect_description(
            "[Your Turn] When any of your Digimon digivolve, if any of them have "
            "the [Royal Knight] trait, by suspending this Tamer, <Draw 1> and gain 1 memory."
        )
        effect0b.is_optional = True  # "by suspending" makes this optional

        def condition0b(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # Can't pay cost if already suspended
            if perm.is_suspended:
                return False
            # Check that a Digimon digivolved
            digivolved_perm = context.get('digivolved_permanent')
            if not digivolved_perm:
                return False
            # Must be OUR Digimon
            game = context.get('game')
            if game and hasattr(game, '_find_owner'):
                digi_owner = game._find_owner(digivolved_perm)
                if digi_owner is not card.owner:
                    return False
            # Check Royal Knight trait on the digivolved Digimon's top card
            top = digivolved_perm.top_card
            if not top or not _has_royal_knight_trait(top):
                return False
            return True

        effect0b.set_can_use_condition(condition0b)
        effect0b.set_on_process_callback(_draw_and_gain_memory)
        effects.append(effect0b)

        # effect1: [Opponent's Turn][Once Per Turn] When any of your Digimon with
        # the [Royal Knight] trait would leave the battle area, you may play 1
        # [Omekamon] from your hand without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenRemoveField)
        effect1.set_effect_name("BT20-091 Play 1 [Omekamon] from your hand")
        effect1.set_effect_description(
            "[Opponent's Turn] [Once Per Turn] When any of your Digimon with the "
            "[Royal Knight] trait would leave the battle area, you may play 1 "
            "[Omekamon] from your hand without paying the cost."
        )
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("PlayCard_BT20-091")

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner):
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            # Opponent's turn only
            if card.owner.is_my_turn:
                return False
            # Check that the leaving permanent has Royal Knight trait
            event_perm = context.get('event_permanent')
            if not event_perm:
                return False
            top = event_perm.top_card if event_perm.top_card else None
            if not top:
                return False
            if not _has_royal_knight_trait(top):
                return False
            # Check that the leaving permanent belongs to us
            event_player = context.get('event_player')
            if event_player is not card.owner:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play 1 Omekamon from hand"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                card_names = getattr(c, 'card_names', []) or []
                return any('Omekamon' in name for name in card_names)

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-091 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
