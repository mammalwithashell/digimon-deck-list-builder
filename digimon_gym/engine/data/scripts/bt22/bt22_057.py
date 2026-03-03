from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_057(CardScript):
    """BT22-057 Kurisarimon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-057 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT22-057 Play [Arata Sanada] from hand")
        effect1.set_effect_description(
            "[When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Arata Sanada] from your hand without paying the cost."
        )
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner:
                tamer_count = sum(1 for p in card.owner.battle_area if p.is_tamer)
                if tamer_count > 1:
                    return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                return any('Arata Sanada' in _n for _n in getattr(c, 'card_names', []))

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("BT22-057 Delete 1 of your other [Diaboromon] to prevent this Digimon from leaving")
        effect2.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon with [Diaboromon] in its text would leave the battle area, by deleting 1 of your other [Diaboromon], it doesn't leave."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Substitute_BT22_056")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect2.effect_source_permanent if hasattr(effect2, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if 'Diaboromon' not in text:
                    return False
            else:
                return False
            if not (permanent and permanent.contains_card_name('Diaboromon')):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
