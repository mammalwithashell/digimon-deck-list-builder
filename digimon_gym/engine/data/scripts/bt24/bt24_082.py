from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_082(CardScript):
    """BT24-082 Owen Dreadnought"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Owen Dreadnought] from your hand without paying the cost. Then, if you don't have a Digimon, you may play 1 [Elizamon] from your trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT24-082 By bottom decking, Play 1 [Owen Dreadnought] from hand. Then if you have no digimon, Play 1 [Elizamon] from trash")
        effect0.set_effect_description("[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Owen Dreadnought] from your hand without paying the cost. Then, if you don't have a Digimon, you may play 1 [Elizamon] from your trash without paying the cost.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Return self to deck bottom, play Owen from hand, then Elizamon from trash"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: return this Tamer to bottom of deck
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm:
                player.return_permanent_to_deck_bottom(tamer_perm)
            # Play 1 [Owen Dreadnought] from hand without paying cost
            def owen_filter(c):
                c_names = getattr(c, 'card_names', []) or []
                return any('Owen Dreadnought' in n for n in c_names)
            game.effect_play_from_zone(
                player, 'hand', owen_filter, free=True, is_optional=True)
            # Then, if you don't have a Digimon, play 1 [Elizamon] from trash
            if not any(p.is_digimon for p in player.battle_area):
                def eliza_filter(c):
                    c_names = getattr(c, 'card_names', []) or []
                    return any('Elizamon' in n for n in c_names)
                game.effect_play_from_zone(
                    player, 'trash', eliza_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] trait Digimon,
        # by suspending this Tamer, that Digimon gets +3000 DP for the turn. Then, it may attack.
        # Uses _is_digivolve_observer pattern so the engine fires this via _fire_digivolve_observers.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-082 +3k and may attack")
        effect1.set_effect_description("[Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] trait Digimon, by suspending this Tamer, that Digimon gets +3000 DP for the turn. Then, it may attack.")
        effect1.is_optional = True
        effect1._is_digivolve_observer = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Cost: tamer must not already be suspended
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and getattr(tamer_perm, 'is_suspended', False):
                return False
            # Must be our turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check the digivolved permanent has Reptile or Dragonkin trait
            digivolved = context.get('digivolved_permanent')
            if not digivolved:
                return False
            traits = getattr(digivolved.top_card, 'card_traits', []) or [] if digivolved.top_card else []
            if not any('Reptile' in t or 'Dragonkin' in t for t in traits):
                return False
            # Must be our own Digimon
            player = card.owner if card else None
            if player and digivolved not in player.battle_area:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, DP +3000 to digivolved Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: suspend this Tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm:
                tamer_perm.suspend()
            # Target: the digivolved Digimon gets +3000 DP
            digivolved = ctx.get('digivolved_permanent')
            if digivolved:
                digivolved.change_dp(3000)
                # "Then, that Digimon may attack" — unsuspend so it can attack this turn
                if digivolved.is_suspended:
                    digivolved.unsuspend()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-082 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
