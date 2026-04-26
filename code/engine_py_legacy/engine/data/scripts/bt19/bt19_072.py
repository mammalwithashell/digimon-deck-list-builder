from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_072(CardScript):
    """BT19-072 LordKnightmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 level 4 or lower Digimon card from your trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT19-072 You may play 1 level 4 or lower Digimon card from your trash.")
        effect0.set_effect_description("[On Play] You may play 1 level 4 or lower Digimon card from your trash without paying the cost.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card from trash"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return c.is_digimon and getattr(c, 'level', None) is not None and c.level <= 4
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 level 4 or lower Digimon card from your trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT19-072 You may play 1 level 4 or lower Digimon card from your trash.")
        effect1.set_effect_description("[When Digivolving] You may play 1 level 4 or lower Digimon card from your trash without paying the cost.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card from trash"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return c.is_digimon and getattr(c, 'level', None) is not None and c.level <= 4
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Opponent's Turn] [Once Per Turn] When any of your opponent's Digimon attack, you may
        # change the attack target to 1 of your Digimon with the [Royal Knight] trait.
        # Uses _is_when_attacked_observer pattern + game.switch_attack_target()
        effect2 = ICardEffect()
        effect2.set_effect_name("BT19-072 You may change the attack target to 1 of your Digimon with the [Royal Knight] trait.")
        effect2.set_effect_description("[Opponent's Turn] [Once Per Turn] When any of your opponent's Digimon attack, you may change the attack target to 1 of your Digimon with the [Royal Knight] trait.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Redirect_BT19-072")
        effect2._is_when_attacked_observer = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # [Opponent's Turn] — only active when it's not our turn
            owner = card.owner if card else None
            if not owner or owner.is_my_turn:
                return False
            # Must have at least one Royal Knight Digimon on our side to redirect to
            has_royal_knight = any(
                p.is_digimon and p.top_card
                and any('Royal Knight' in t for t in (getattr(p.top_card, 'card_traits', []) or []))
                for p in owner.battle_area
            )
            return has_royal_knight

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Redirect attack to a Royal Knight Digimon (SwitchDefender)"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def royal_knight_filter(p):
                return (p.is_digimon and p.top_card
                        and any('Royal Knight' in t
                                for t in (getattr(p.top_card, 'card_traits', []) or [])))

            def on_select_redirect(target_perm):
                game.switch_attack_target(target_perm)

            game.effect_select_own_permanent(
                player, on_select_redirect,
                filter_fn=royal_knight_filter,
                is_optional=False,
                prompt="Select 1 of your Digimon with the [Royal Knight] trait to redirect the attack to.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
