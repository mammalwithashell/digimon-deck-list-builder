from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_037(CardScript):
    """BT24-037 Silphymon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-037 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 with [TS] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Jogress Condition
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-037 Jogress Condition")
        effect1.set_effect_description("Jogress Condition")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # DP -5000, Force Attack, Change Security Attack
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-037 DP -5000, Force Attack, Change Security Attack")
        effect2.set_effect_description("DP -5000, Force Attack, Change Security Attack")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP -5000, Force Attack, Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-5000)
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # DP -5000, Force Attack, Change Security Attack
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-037 DP -5000, Force Attack, Change Security Attack")
        effect3.set_effect_description("DP -5000, Force Attack, Change Security Attack")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP -5000, Force Attack, Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-5000)
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower yellow, red or [TS] trait Digimon card from its digivolution cards without paying the cost.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenRemoveField)
        effect4.set_effect_name("BT24-037 Play 1 level 4- [Yellow]/[Red]/[TS] trait digimon from digivolution sources")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower yellow, red or [TS] trait Digimon card from its digivolution cards without paying the cost.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_037_AT")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play 1 lv4- yellow/red/[TS] Digimon from digivolution cards."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', []) or [])]
                traits = getattr(c, 'card_traits', []) or []
                # Yellow OR Red OR [TS] trait — three independent conditions
                if 'Yellow' in colors or 'Red' in colors or any('TS' in t for t in traits):
                    return True
                return False
            # Play from this Digimon's digivolution cards (not hand)
            top = perm.top_card
            candidates = [cs for cs in perm.card_sources if cs is not top and play_filter(cs)]
            if not candidates:
                return
            # Auto-select best candidate (first matching) for RL simplicity
            chosen = candidates[0]
            perm.card_sources.remove(chosen)
            played = player.play_card_from_source(chosen, pay_cost=False)
            if played:
                game.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {'played_card': chosen, 'played_permanent': played, 'event_player': player},
                )

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower yellow, red or [TS] trait Digimon card from its digivolution cards without paying the cost.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.WhenRemoveField)
        effect5.set_effect_name("BT24-037 Play 1 level 4- [Yellow]/[Red]/[CS] trait digimon from digivolution sources")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, you may play 1 level 4 or lower yellow, red or [TS] trait Digimon card from its digivolution cards without paying the cost.")
        effect5.is_inherited_effect = True
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT24_037_AT_ESS")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play 1 lv4- yellow/red/[TS] Digimon from digivolution cards (inherited)."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', []) or [])]
                traits = getattr(c, 'card_traits', []) or []
                # Yellow OR Red OR [TS] trait — three independent conditions
                if 'Yellow' in colors or 'Red' in colors or any('TS' in t for t in traits):
                    return True
                return False
            # Play from this Digimon's digivolution cards (not hand)
            top = perm.top_card
            candidates = [cs for cs in perm.card_sources if cs is not top and play_filter(cs)]
            if not candidates:
                return
            chosen = candidates[0]
            perm.card_sources.remove(chosen)
            played = player.play_card_from_source(chosen, pay_cost=False)
            if played:
                game.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {'played_card': chosen, 'played_permanent': played, 'event_player': player},
                )

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
