from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_055(CardScript):
    """EX11-055 Chitose Horaiji"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # Draw 1, Gain 1 memory, Trash From Hand
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX11-055 Draw 1, Gain 1 memory, Trash From Hand")
        effect0.set_effect_description("Draw 1, Gain 1 memory, Trash From Hand")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: By trashing 1 Composite/Wicked God trait card from hand, Draw 1 and gain 1 memory."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: trash 1 Composite or Wicked God trait card from hand
            def trait_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('Composite' in t or 'Wicked God' in t for t in traits)

            qualifying = [c for c in player.hand_cards if trait_filter(c)]
            if not qualifying:
                return

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                # Effect: Draw 1 and gain 1 memory
                player.draw_cards(1)
                player.add_memory(1)

            game.effect_select_hand_card(
                player, trait_filter, on_trashed, is_optional=False,
                prompt="Trash 1 [Composite] or [Wicked God] trait card.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Draw 1, Gain 1 memory, Trash From Hand
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-055 Draw 1, Gain 1 memory, Trash From Hand")
        effect1.set_effect_description("Draw 1, Gain 1 memory, Trash From Hand")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: By trashing 1 Composite/Wicked God trait card from hand, Draw 1 and gain 1 memory."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def trait_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('Composite' in t or 'Wicked God' in t for t in traits)

            qualifying = [c for c in player.hand_cards if trait_filter(c)]
            if not qualifying:
                return

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                player.draw_cards(1)
                player.add_memory(1)

            game.effect_select_hand_card(
                player, trait_filter, on_trashed, is_optional=False,
                prompt="Trash 1 [Composite] or [Wicked God] trait card.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # Suspend, Play Card
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("EX11-055 By suspending this tamer, play 1 [Gazimon] or [Gizamon] for free.")
        effect2.set_effect_description("Suspend, Play Card")
        effect2.is_optional = True
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Check this Tamer is unsuspended
            my_perm = card.permanent_of_this_card()
            if my_perm and my_perm.is_suspended:
                return False
            # The deleted Digimon must be ours and have Composite/Wicked God trait
            deleted_perm = context.get('permanent')
            if deleted_perm and card.owner:
                if deleted_perm.owner != card.owner:
                    return False
                if not deleted_perm.is_digimon:
                    return False
                if deleted_perm.top_card:
                    traits = getattr(deleted_perm.top_card, 'card_traits', []) or []
                    if not any('Composite' in t or 'Wicked God' in t for t in traits):
                        return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Suspend this Tamer, play 1 [Gazimon] or [Gizamon] from hand free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm or my_perm.is_suspended:
                return
            # Cost: suspend this Tamer
            my_perm.suspend()
            # Effect: play 1 Gazimon or Gizamon from hand
            def play_filter(c):
                return (c.contains_card_name('Gazimon') or
                        c.contains_card_name('Gizamon'))

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True,
                prompt="Play 1 [Gazimon] or [Gizamon] from hand.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("EX11-055 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
