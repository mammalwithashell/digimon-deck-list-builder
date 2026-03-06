from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_019(CardScript):
    """BT13-019 Gankoomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-019 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 Digimon card with [Sistermon] in its name from your trash
        # or 1 Digimon card with the [Royal Knight] trait from the digivolution cards of your
        # Digimon in the breeding area without paying its cost. You can't play [Gankoomon] or [Omnimon].
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT13-019 Play Sistermon from trash or Royal Knight from breeding sources")
        effect1.set_effect_description("[On Play] You may play 1 Digimon card with [Sistermon] in its name from your trash or 1 Digimon card with the [Royal Knight] trait from the digivolution cards of your Digimon in the breeding area without paying its cost. You can't play [Gankoomon] or [Omnimon] with this effect.")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Sistermon from trash (or Royal Knight from breeding sources)"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def sistermon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                names = getattr(c, 'card_names', [])
                if not any('Sistermon' in n for n in names):
                    return False
                # Can't play Gankoomon or Omnimon
                if any('Gankoomon' in n or 'Omnimon' in n for n in names):
                    return False
                return True

            # TODO: also allow playing Royal Knight from breeding area digivolution cards
            game.effect_play_from_zone(
                player, 'trash', sistermon_filter, free=True, is_optional=True,
                prompt="Play 1 Sistermon from trash (or Royal Knight from breeding sources).")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Same effect as On Play
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT13-019 Play Sistermon from trash or Royal Knight from breeding sources")
        effect2.set_effect_description("[When Digivolving] You may play 1 Digimon card with [Sistermon] in its name from your trash or 1 Digimon card with the [Royal Knight] trait from the digivolution cards of your Digimon in the breeding area without paying its cost. You can't play [Gankoomon] or [Omnimon] with this effect.")
        effect2.is_optional = True
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Sistermon from trash (or Royal Knight from breeding sources)"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def sistermon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                names = getattr(c, 'card_names', [])
                if not any('Sistermon' in n for n in names):
                    return False
                # Can't play Gankoomon or Omnimon
                if any('Gankoomon' in n or 'Omnimon' in n for n in names):
                    return False
                return True

            # TODO: also allow playing Royal Knight from breeding area digivolution cards
            game.effect_play_from_zone(
                player, 'trash', sistermon_filter, free=True, is_optional=True,
                prompt="Play 1 Sistermon from trash (or Royal Knight from breeding sources).")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
