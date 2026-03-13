from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_025(CardScript):
    """EX7-025 ShoeShoemon | Lv.4, Yellow, Puppet/LIBERATOR, DP 4000, Cost 4

    [When Digivolving] If you have 1 or fewer Tamers, you may play 1
        [Arisa Kinosaki] from your hand without paying the cost.
    [Inherited][Your Turn] All of your opponent's Security Digimon get -3000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [When Digivolving] Play 1 [Arisa Kinosaki] from hand free ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX7-025 Play [Arisa Kinosaki] from hand")
        effect0.set_effect_description(
            "[When Digivolving] If you have 1 or fewer Tamers, you may play 1 "
            "[Arisa Kinosaki] from your hand without paying the cost."
        )
        effect0.is_when_digivolving = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if owner is None:
                return False
            tamer_count = sum(1 for p in owner.battle_area if p.is_tamer)
            if tamer_count > 1:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Play [Arisa Kinosaki] from hand free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                names = getattr(c, 'card_names', []) or []
                return (getattr(c, 'is_tamer', False) and
                        any('Arisa Kinosaki' in n for n in names))

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Inherited][Your Turn] Opponent's Security Digimon -3000 DP ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX7-025 Inherited: Opponent's Security Digimon -3000 DP")
        effect1.set_effect_description(
            "[Inherited][Your Turn] All of your opponent's Security Digimon get -3000 DP."
        )
        effect1.is_inherited_effect = True
        effect1.security_dp_modifier = -3000

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            target_card = context.get('card_source')
            if target_card:
                owner = card.owner
                if owner and target_card.owner is not owner.enemy:
                    return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
