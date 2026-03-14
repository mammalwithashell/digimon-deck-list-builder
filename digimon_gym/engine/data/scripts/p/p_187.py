from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_187(CardScript):
    """P-187 Mastemon | Lv.6

    [When Digivolving] Recovery +1 (Deck). Then, if DNA digivolving, by placing
        1 other Digimon or Tamer as the top or bottom security card, trash your
        opponent's top security card.
    [When Digivolving] [When Attacking] [Once Per Turn] By trashing your top
        security card, you may play 1 purple or yellow Digimon card with 6000 DP
        or less from your hand or trash without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("P-187 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [When Digivolving] Recovery +1. Then, if DNA digivolving, by placing
        # 1 other Digimon or Tamer as security, trash opponent's top security.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("P-187 Recovery +1, DNA: place perm to security, trash opp security")
        effect1.set_effect_description(
            "[When Digivolving] Recovery +1 (Deck). Then, if DNA digivolving, "
            "by placing 1 other Digimon or Tamer as the top or bottom security "
            "card, trash your opponent's top security card."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not player:
                return
            # Recovery +1
            player.recovery(1)
            if not (game and ctx.get('is_dna_digivolve')):
                return

            def target_filter(p):
                return p is not perm and (p.is_digimon or p.is_tamer)

            def on_put_security(target_perm):
                if player and target_perm in player.battle_area:
                    player.put_permanent_to_security(target_perm)
                    # Trash opponent's top security card
                    enemy = player.enemy
                    if enemy and enemy.security_cards:
                        enemy.trash_cards.append(enemy.security_cards.pop(0))

            has_target = any(target_filter(p) for p in player.battle_area)
            if has_target:
                game.effect_select_own_permanent(
                    player, on_put_security, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [When Digivolving] [When Attacking] [Once Per Turn]
        # By trashing your top security card, you may play 1 purple or yellow
        # Digimon card with 6000 DP or less from your hand or trash without
        # paying the cost.
        def _build_play_effect(is_when_digivolving=False, is_on_attack=False):
            effect = ICardEffect()
            if is_when_digivolving:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
                effect.is_when_digivolving = True
            else:
                effect.set_timing(EffectTiming.OnUseAttack)
                effect.is_on_attack = True
            effect.set_effect_name("P-187 Trash own security to play Digimon")
            effect.set_effect_description(
                "[Once Per Turn] By trashing your top security card, you may play "
                "1 purple or yellow Digimon card with 6000 DP or less from your "
                "hand or trash without paying the cost."
            )
            effect.is_optional = True
            effect.set_max_count_per_turn(1)
            effect.set_hash_string("PlayDigimon_P_187")

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                player = card.owner if card else None
                if not player or not player.security_cards:
                    return False
                return True
            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return
                if not player.security_cards:
                    return
                # Pay cost: trash own top security card
                trashed = player.security_cards.pop(0)
                player.trash_cards.append(trashed)

                # Play 1 purple or yellow Digimon with 6000 DP or less
                def play_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    colors = [col.name for col in (getattr(c, 'card_colors', []) or [])]
                    if 'Purple' not in colors and 'Yellow' not in colors:
                        return False
                    dp = getattr(c, 'dp', None)
                    if dp is None or dp > 6000:
                        return False
                    return True

                game.effect_play_from_zone(
                    player, 'hand_or_trash', play_filter, free=True, is_optional=True)

            effect.set_on_process_callback(process)
            return effect

        effects.append(_build_play_effect(is_when_digivolving=True))
        effects.append(_build_play_effect(is_on_attack=True))

        return effects
