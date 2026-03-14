from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_061(CardScript):
    """EX11-061 Mirai Kinosaki"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: gain_memory_tamer
        # [Start of Main] Gain 1 memory if opponent has Digimon
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX11-061 Gain 1 memory")
        effect0.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon in play, gain 1 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory if opponent has Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                opponent = game.opponent_player
                if opponent and any(p.is_digimon for p in opponent.battle_area):
                    game.memory += 1
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When any of your Digimon digivolve into a [Puppet] trait Digimon, by suspending this Tamer, you may play 1 level 3 [Puppet] trait Digimon card from your hand without paying the cost. At turn end, delete the Digimon this effect played.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-061 Suspend tamer, play lvl 3 [Puppet], delete it at end of turns.")
        effect1.set_effect_description("[Your Turn] When any of your Digimon digivolve into a [Puppet] trait Digimon, by suspending this Tamer, you may play 1 level 3 [Puppet] trait Digimon card from your hand without paying the cost. At turn end, delete the Digimon this effect played.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True
        effect1._is_digivolve_observer = True

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check the digivolved permanent has Puppet trait
            trigger_perm = context.get('digivolved_permanent')
            if not trigger_perm:
                return False
            if trigger_perm.owner != card.owner:
                return False
            if not trigger_perm.is_digimon:
                return False
            if trigger_perm.top_card and _is_puppet_trait(trigger_perm.top_card):
                return True
            return False

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, play level 3 Puppet from hand free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            if tamer_perm.is_suspended:
                return
            tamer_perm.suspend()

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'card_level', None) != 3:
                    return False
                return _is_puppet_trait(c)
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("EX11-061 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
