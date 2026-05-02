from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_100(CardScript):
    """BT20-100 The Last Guardian"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 3 cards of your deck. Add 1 [Cool Boy] and 1 card with the [Royal Knight] or [X Antibody] trait among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT20-100 Reveal the top 3 cards, add 1 [Cool Boy] and 1 [Royal Knight]/[X Antibody] trait")
        effect0.set_effect_description("[Main] Reveal the top 3 cards of your deck. Add 1 [Cool Boy] and 1 card with the [Royal Knight] or [X Antibody] trait among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add 1 Cool Boy and 1 Royal Knight/X Antibody trait card to hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def cool_boy_filter(c):
                return any('Cool Boy' in n for n in getattr(c, 'card_names', []))
            def rk_xantibody_filter(c):
                return any('Royal Knight' in t or 'X Antibody' in t for t in getattr(c, 'card_traits', []))
            passes = [
                (cool_boy_filter, 'hand'),
                (rk_xantibody_filter, 'hand'),
            ]
            game.effect_reveal_and_select_multi(
                player, 3, passes, remaining_placement='deck_bottom')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay — marks this option to stay on the battle area
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-100 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When any of your Digimon with [Omnimon] in its name would leave the battle area, <Delay>.
        # (By trashing this card after the placing turn, activate the effect below.)
        # - 1 of those Digimon doesn't leave.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("BT20-100 Prevent Omnimon Removal")
        effect2.set_effect_description("[All Turns] When any of your Digimon with [Omnimon] in its name would leave the battle area, 1 of those Digimon doesn't leave.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The Delay can only be used after the placing turn
            own_perm = card.permanent_of_this_card()
            game_ctx = context.get('game')
            if own_perm and game_ctx:
                turn_played = getattr(own_perm, 'turn_played', None)
                if turn_played is not None and game_ctx.turn_count <= turn_played:
                    return False
            # event_permanent is the permanent that is leaving the field
            # (context['permanent'] is this effect's own permanent — BT20-100 itself)
            leaving_perm = context.get('event_permanent')
            event_player = context.get('event_player')
            if not leaving_perm or not event_player:
                return False
            # Must be our own Digimon
            owner = card.owner if card else None
            if not owner or event_player is not owner:
                return False
            if not getattr(leaving_perm, 'is_digimon', False):
                return False
            # contains_card_name matches 'Omnimon' and 'Omnimon (X Antibody)' — correct
            if not leaving_perm.contains_card_name('Omnimon'):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delay activation cost: trash this option card from the battle area.
            The engine handles keeping the Omnimon on field when a WhenRemoveField
            optional effect resolves its process callback."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not player:
                return
            # Trash this option card's permanent from the battle area
            own_perm = card.permanent_of_this_card() if card else None
            if own_perm and own_perm in player.battle_area:
                player.battle_area.remove(own_perm)
                for cs in list(own_perm.card_sources):
                    player.trash_cards.append(cs)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Omekamon] or [Cool Boy] from your hand or trash without paying the cost. Then, place this card in the battle area.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT20-100 Play Card")
        effect3.set_effect_description("[Security] You may play 1 [Omekamon] or [Cool Boy] from your hand or trash without paying the cost. Then, place this card in the battle area.")
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play 1 Omekamon or Cool Boy from hand or trash"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return any('Omekamon' in n or 'Cool Boy' in n for n in getattr(c, 'card_names', []))
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
