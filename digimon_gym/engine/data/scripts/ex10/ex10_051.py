from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_051(CardScript):
    """EX10-051 Mummymon | Lv.5

    [On Play] By trashing 1 card in your hand, De-Digivolve 1 one of your
        opponent's Digimon.
    [On Deletion] You may play 1 Tamer card with [Myotismon] in its text
        from your trash without paying the cost. This effect can't play
        cards with the same name as any of your Tamers.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [On Play] By trashing 1 hand card, De-Digivolve 1 opponent Digimon
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX10-051 On Play: trash hand, De-Digivolve 1")
        effect0.set_effect_description(
            "[On Play] By trashing 1 card in your hand, De-Digivolve 1 one of "
            "your opponent's Digimon."
        )
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player or not player.hand_cards:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c):
                return True

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)

                # De-Digivolve 1 opponent Digimon
                def on_de_digivolve(target_perm):
                    removed = target_perm.de_digivolve(1)
                    enemy = player.enemy
                    if enemy:
                        enemy.trash_cards.extend(removed)

                enemy = player.enemy
                if enemy and any(p.is_digimon for p in enemy.battle_area):
                    game.effect_select_opponent_permanent(
                        player, on_de_digivolve,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False)

            if player.hand_cards:
                game.effect_select_hand_card(
                    player, hand_filter, on_trashed, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [On Deletion] Play 1 Tamer with [Myotismon] in text from trash
        # (can't play same name as existing Tamers)
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("EX10-051 On Deletion: play Myotismon-text Tamer from trash")
        effect1.set_effect_description(
            "[On Deletion] You may play 1 Tamer card with [Myotismon] in its "
            "text from your trash without paying the cost. This effect can't "
            "play cards with the same name as any of your Tamers."
        )
        effect1.is_optional = True
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Get names of existing Tamers
            existing_tamer_names = set()
            for p in player.battle_area:
                if p.is_tamer and p.top_card:
                    for n in (getattr(p.top_card, 'card_names', []) or []):
                        existing_tamer_names.add(n)

            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                # Must have [Myotismon] in text
                text = getattr(c, 'card_text', '') or ''
                if 'Myotismon' not in text:
                    return False
                # Can't play same name as existing Tamers
                names = getattr(c, 'card_names', []) or []
                if any(n in existing_tamer_names for n in names):
                    return False
                return True

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
