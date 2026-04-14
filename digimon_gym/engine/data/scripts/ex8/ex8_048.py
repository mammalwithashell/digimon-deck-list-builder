from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_048(CardScript):
    """EX8-048 Landramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Close]
        # from your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX8-048 Play 1 [Close] from hand free")
        effect0.set_effect_description(
            "[When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Close] from your hand without paying the cost."
        )
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if owner:
                tamer_count = sum(1 for p in owner.battle_area if p.is_tamer)
                if tamer_count > 1:
                    return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                return any('Close' in n for n in getattr(c, 'card_names', []))

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # Inherited: When effects trash this card from a [Mineral] or [Rock] trait
        # Digimon's digivolution cards, delete 1 of your opponent's Digimon with a
        # play cost of 4 or less.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect1.set_effect_name("EX8-048 Delete opponent Digimon cost 4 or less")
        effect1.set_effect_description(
            "When effects trash this card from a [Mineral] or [Rock] trait Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play cost of 4 or less."
        )
        effect1.is_inherited_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Check this card was the one trashed (not a different digi-card)
            trashed_cards = context.get('trashed_cards', [])
            if card not in trashed_cards:
                return False
            # Check the host permanent has [Mineral] or [Rock] trait
            permanent = context.get('permanent')
            if permanent is None:
                return False
            if not (permanent.has_trait('Mineral') or permanent.has_trait('Rock')):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and p.top_card.get_cost_itself <= 4

            def on_delete(target):
                enemy = player.enemy
                if enemy:
                    enemy.delete_permanent(target)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
