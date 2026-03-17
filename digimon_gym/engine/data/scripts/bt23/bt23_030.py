from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_030(CardScript):
    """BT23-030 Etemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-030 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alliance
        # Alliance
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-030 Alliance")
        effect1.set_effect_description("Alliance")
        effect1.is_on_attack = True
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] [Once Per Turn] By paying 1 cost, you may play 1 play cost 3 or lower card with [Chuumon] or [Sukamon] in its name or the [CS] trait from your hand without paying the cost. Then, 1 of your level 3 or higher Digimon gains <Reboot> and <Blocker> until your opponent's turn ends.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("BT23-030 By paying 1 cost, play 3 cost or lower [Chuumon]/[Sukamon] in name /[CS] trait from your hand, then 1 level 3+ digimon gains <Reboot> and <Blocker>")
        effect2.set_effect_description("[Main] [Once Per Turn] By paying 1 cost, you may play 1 play cost 3 or lower card with [Chuumon] or [Sukamon] in its name or the [CS] trait from your hand without paying the cost. Then, 1 of your level 3 or higher Digimon gains <Reboot> and <Blocker> until your opponent's turn ends.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT23_030_Main")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: By paying 1 cost, play 1 card (cost 3 or less, Chuumon/Sukamon name or CS trait) from hand free. Then 1 Lv.3+ Digimon gains Reboot+Blocker until opponent's turn ends."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # "By paying 1 cost" = cost that happens BEFORE the effect
            player.lose_memory(1)
            def play_filter(c):
                pc = getattr(c, 'play_cost', 0) or 0
                if pc > 3:
                    return False
                names = getattr(c, 'card_names', []) or []
                has_name = any('Chuumon' in _n or 'Sukamon' in _n for _n in names)
                traits = getattr(c, 'card_traits', []) or []
                has_cs = any('CS' in _t for _t in traits)
                return has_name or has_cs
            def after_play():
                # "Then, 1 of your level 3 or higher Digimon gains <Reboot> and <Blocker> until your opponent's turn ends."
                def lv3_filter(p):
                    return p.is_digimon and (getattr(p, 'level', 0) or 0) >= 3
                def on_grant(target_perm):
                    # Lasts through current turn + opponent's next turn
                    expiry_turn = game.turn_count + 1 if game else -1
                    target_perm.grant_keyword('_is_reboot', expiry_turn)
                    target_perm.grant_keyword('_is_blocker', expiry_turn)
                game.effect_select_own_permanent(
                    player, on_grant, filter_fn=lv3_filter, is_optional=False,
                    prompt="Select 1 of your Lv.3+ Digimon to gain Reboot and Blocker.")
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            after_play()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: alliance
        # Alliance
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-030 Alliance")
        effect3.set_effect_description("Alliance")
        effect3.is_inherited_effect = True
        effect3.is_on_attack = True
        effect3._is_alliance = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
