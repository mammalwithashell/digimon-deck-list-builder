from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_078(CardScript):
    """BT19-078 ADR-01 Jeri"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] For each digivolution card of 1 of your [Mother D-Reaper]s, 1 of your opponent's Digimon gets -1000 for the turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT19-078 -1000DP for each digivolution source")
        effect0.set_effect_description("[On Play] For each digivolution card of 1 of your [Mother D-Reaper]s, 1 of your opponent's Digimon gets -1000 for the turn.")
        effect0.is_on_play = True
        effect0.dp_modifier = -1000

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Mother D-Reaper'))):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: DP -1000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-1000)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDeclaration
        # [Main] Place this Digimon as the bottom digivolution card of 1 of your [Mother D-Reaper] without [ADR-01 Jeri] in its digivolution cards.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1._is_field_main = True
        effect1.set_effect_name("BT19-078 Place 1 this card as 1 of your [Mother D-Reaper] digivolution cards")
        effect1.set_effect_description("[Main] Place this Digimon as the bottom digivolution card of 1 of your [Mother D-Reaper] without [ADR-01 Jeri] in its digivolution cards.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("PlaceUnder_BT19-078")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent:
                if not any(src.contains_card_name('ADR-01 Jeri') for src in permanent.digivolution_cards):
                    return False
            else:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Mother D-Reaper'))):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [Opponent's Turn] When any of your opponent's Digimon attack, you may play 1 [ADR-01 Jeri] from this Digimon's digivolution cards without paying the cost. If you played, you may change the attack target to the Digimon played by this effect.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT19-078 You may play 1 [ADR-01 Jeri] from this Digimon's digivolution cards, then change the attack target to the  Digimon played.")
        effect2.set_effect_description("[Opponent's Turn] When any of your opponent's Digimon attack, you may play 1 [ADR-01 Jeri] from this Digimon's digivolution cards without paying the cost. If you played, you may change the attack target to the Digimon played by this effect.")
        effect2.is_inherited_effect = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Redirect Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Redirect attack target (SwitchDefender) — not yet in engine
            pass  # descriptive-tagged: redirect_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
