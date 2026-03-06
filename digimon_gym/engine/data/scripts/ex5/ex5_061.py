from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_061(CardScript):
    """EX5-061 Cerberusmon (X Antibody) | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-061 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 5
        effect0._alt_digi_name = "Cerberusmon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX5-061 Play 1 purple level 3 Digimon from trash")
        effect1.set_effect_description("[On Play] You may play 1 purple level 3 Digimon card from your trash without paying the cost.")
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) != 3:
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', []) or [])]
                return 'Purple' in colors

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX5-061 Draw 1, trash 1 card from hand and activate [On Play] effect")
        effect2.set_effect_description(
            "[When Digivolving] <Draw 1> (Draw 1 card from your deck). Then, trash 1 card in your hand. If a Digimon card with [Cerberusmon] in its name or [X Antibody] is in this Digimon's digivolution cards, activate this Digimon's [On Play] effects."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if not (player and game):
                return

            def hand_filter(c):
                return True

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)

                sources = perm.card_sources[:-1] if perm else []
                should_trigger = any(
                    any('Cerberusmon' in name for name in getattr(source, 'card_names', []))
                    or 'X Antibody' in getattr(source, 'card_traits', [])
                    or 'X-Antibody' in getattr(source, 'card_traits', [])
                    for source in sources
                )
                if should_trigger:
                    process1(ctx)

            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnAllyAttack)
        effect3.set_effect_name("EX5-061 Delete your another Digimon to unsuspend this Digimon")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] By deleting 1 of your other Digimon, unsuspend this Digimon.")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Unsuspend_EX5_061")
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return True

            def on_unsuspend(target_perm):
                target_perm.unsuspend()

            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
